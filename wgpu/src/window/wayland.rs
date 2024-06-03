use crate::graphics::compositor::Window;
use raw_window_handle::{RawDisplayHandle, WaylandDisplayHandle};
use rustix::fs::{major, minor};
use sctk::{
    dmabuf::{DmabufFeedback, DmabufHandler, DmabufState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
};
use std::{fs::File, io::Read, path::PathBuf};
use wayland_client::{
    backend::Backend, globals::registry_queue_init, protocol::wl_buffer,
    Connection, QueueHandle,
};
use wayland_protocols::wp::linux_dmabuf::zv1::client::{
    zwp_linux_buffer_params_v1, zwp_linux_dmabuf_feedback_v1,
};

struct AppData {
    registry_state: RegistryState,
    dmabuf_state: DmabufState,
    feedback: Option<DmabufFeedback>,
}

impl DmabufHandler for AppData {
    fn dmabuf_state(&mut self) -> &mut DmabufState {
        &mut self.dmabuf_state
    }

    fn dmabuf_feedback(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _proxy: &zwp_linux_dmabuf_feedback_v1::ZwpLinuxDmabufFeedbackV1,
        feedback: DmabufFeedback,
    ) {
        self.feedback = Some(feedback);
    }

    fn created(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _params: &zwp_linux_buffer_params_v1::ZwpLinuxBufferParamsV1,
        _buffer: wl_buffer::WlBuffer,
    ) {
    }

    fn failed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _params: &zwp_linux_buffer_params_v1::ZwpLinuxBufferParamsV1,
    ) {
    }

    fn released(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _buffer: &wl_buffer::WlBuffer,
    ) {
    }
}

impl ProvidesRegistryState for AppData {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![,];
}

pub fn get_wayland_device_ids<W: Window>(window: &W) -> Option<(u16, u16)> {
    let conn = match window.display_handle().map(|handle| handle.as_raw()) {
        #[allow(unsafe_code)]
        Ok(RawDisplayHandle::Wayland(WaylandDisplayHandle {
            display, ..
        })) => Connection::from_backend(unsafe {
            Backend::from_foreign_display(display.as_ptr() as *mut _)
        }),
        _ => {
            return None;
        }
    };

    let (globals, mut event_queue) = registry_queue_init(&conn).unwrap();
    let qh = event_queue.handle();

    let mut app_data = AppData {
        registry_state: RegistryState::new(&globals),
        dmabuf_state: DmabufState::new(&globals, &qh),
        feedback: None,
    };

    match app_data.dmabuf_state.version() {
        Some(4..) => {
            let _ = app_data.dmabuf_state.get_default_feedback(&qh).unwrap();

            let feedback = loop {
                let _ = event_queue.blocking_dispatch(&mut app_data).ok()?;
                if let Some(feedback) = app_data.feedback.as_ref() {
                    break feedback;
                }
            };

            let dev = feedback.main_device();
            let path = PathBuf::from(format!(
                "/sys/dev/char/{}:{}/device",
                major(dev),
                minor(dev)
            ));
            let vendor = {
                let path = path.join("vendor");
                let mut file = File::open(&path).ok()?;
                let mut contents = String::new();
                let _ = file.read_to_string(&mut contents).ok()?;
                u16::from_str_radix(
                    contents.trim().trim_start_matches("0x"),
                    16,
                )
                .ok()?
            };
            let device = {
                let path = path.join("device");
                let mut file = File::open(&path).ok()?;
                let mut contents = String::new();
                let _ = file.read_to_string(&mut contents).ok()?;
                u16::from_str_radix(
                    contents.trim().trim_start_matches("0x"),
                    16,
                )
                .ok()?
            };

            Some((vendor, device))
        }
        _ => None,
    }
}

sctk::delegate_dmabuf!(AppData);
sctk::delegate_registry!(AppData);
