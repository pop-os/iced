use drm_fourcc::{DrmFourcc, DrmModifier};
use gst::glib::{self, translate::IntoGlib};
use gst::prelude::*;
use iced::futures::{executor::block_on, SinkExt};
use iced_sctk::subsurface_widget::{
    BufferSource, Dmabuf, Plane, SubsurfaceBuffer,
};
use std::{ffi::c_void, os::unix::io::BorrowedFd, sync::Arc, thread};

// Store a reference to the `BufferSource` in the data assocaited with the `BufferRef`.
// So the `BufferSource` can be re-used, instead of dupping fds and creating a new
// `wl_buffer` each buffer swap.
//
// See https://gitlab.freedesktop.org/gstreamer/gstreamer/-/blob/main/subprojects/gst-plugins-bad/gst-libs/gst/wayland/gstwlbuffer.c
// for information about how `waylandsink` does this.
fn get_buffer_source(buffer: &gst::BufferRef) -> Option<Arc<BufferSource>> {
    let buffer_source_quark = glib::Quark::from_str("SctkBufferSource");
    unsafe {
        let data = gst::ffi::gst_mini_object_get_qdata(
            buffer.upcast_ref().as_mut_ptr(),
            buffer_source_quark.into_glib(),
        );
        if data.is_null() {
            None
        } else {
            Arc::increment_strong_count(data as *const BufferSource);
            Some(Arc::from_raw(data as *const BufferSource))
        }
    }
}

fn set_buffer_source(buffer: &gst::BufferRef, source: Arc<BufferSource>) {
    let buffer_source_quark = glib::Quark::from_str("SctkBufferSource");
    unsafe extern "C" fn destroy_buffer_source(data: *mut c_void) {
        Arc::from_raw(data);
    }
    unsafe {
        gst::ffi::gst_mini_object_set_qdata(
            buffer.upcast_ref().as_mut_ptr(),
            buffer_source_quark.into_glib(),
            Arc::into_raw(source) as *mut c_void,
            Some(destroy_buffer_source),
        );
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    Frame(SubsurfaceBuffer),
}

pub fn subscription(path: &str) -> iced::Subscription<Event> {
    let path = path.to_string();
    iced::subscription::channel("pw", 16, |sender| async {
        thread::spawn(move || pipewire_thread(&path, sender));
        std::future::pending().await
    })
}

fn pipewire_thread(
    path: &str,
    mut sender: futures_channel::mpsc::Sender<Event>,
) {
    gst::init().unwrap();

    // `vapostproc` can be added to convert color format
    // TODO had issue on smithay using NV12?
    let pipeline = gst::parse_launch(&format!(
        "filesrc location={path} !
         qtdemux !
         h264parse !
         vah264dec !
         vapostproc !
         video/x-raw(memory:DMABuf),format=BGRA !
         appsink name=sink",
    ))
    .unwrap()
    .dynamic_cast::<gst::Pipeline>()
    .unwrap();

    let appsink = pipeline
        .by_name("sink")
        .unwrap()
        .dynamic_cast::<gst_app::AppSink>()
        .unwrap();

    let mut subsurface_release = None;

    appsink.set_callbacks(
        gst_app::AppSinkCallbacks::builder()
            .new_sample(move |appsink| {
                let sample =
                    appsink.pull_sample().map_err(|_| gst::FlowError::Eos)?;

                let buffer = sample.buffer().unwrap();
                let meta = buffer.meta::<gst_video::VideoMeta>().unwrap();

                let buffer_source = if let Some(buffer_source) = get_buffer_source(buffer) {
                    buffer_source
                } else {
                    let planes = (0..meta.n_planes())
                        .map(|plane_idx| {
                            let memory = buffer
                                .memory(plane_idx)
                                .unwrap()
                                .downcast_memory::<gst_allocators::DmaBufMemory>()
                                .unwrap();

                            // TODO avoid dup?
                            let fd = unsafe { BorrowedFd::borrow_raw(memory.fd()) }
                                .try_clone_to_owned()
                                .unwrap();

                            Plane {
                                fd,
                                plane_idx,
                                offset: meta.offset()[plane_idx as usize] as u32,
                                stride: meta.stride()[plane_idx as usize] as u32,
                            }
                        })
                        .collect();

                    let dmabuf = Dmabuf {
                        width: meta.width() as i32,
                        height: meta.height() as i32,
                        planes,
                        // TODO should use dmabuf protocol to get supported formats,
                        // convert if needed.
                        format: DrmFourcc::Argb8888 as u32,
                        //format: DrmFourcc::Nv12 as u32,
                        // TODO modifier negotiation
                        modifier: DrmModifier::Linear.into(),
                    };

                    let buffer_source = Arc::new(BufferSource::from(dmabuf));
                    set_buffer_source(buffer, buffer_source.clone());
                    buffer_source
                };

                let (buffer, new_subsurface_release) =
                    SubsurfaceBuffer::new(buffer_source);
                block_on(sender.send(Event::Frame(buffer))).unwrap();

                // Wait for server to release other buffer
                // TODO is gstreamer using triple buffering?
                if let Some(release) = subsurface_release.take() {
                    block_on(release);
                }
                subsurface_release = Some(new_subsurface_release);

                Ok(gst::FlowSuccess::Ok)
            })
            .build(),
    );

    pipeline.set_state(gst::State::Playing).unwrap();
    let bus = pipeline.bus().unwrap();
    for _msg in bus.iter_timed(gst::ClockTime::NONE) {}
}