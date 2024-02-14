// TODO z-order option?

use crate::core::{
    layout::{self, Layout},
    mouse, renderer,
    widget::{self, Widget},
    ContentFit, Element, Length, Rectangle, Size,
};
use std::{
    cell::RefCell,
    future::Future,
    mem,
    os::unix::io::{AsFd, OwnedFd},
    pin::Pin,
    sync::Arc,
    task,
};

use futures::channel::oneshot;
use sctk::{
    compositor::SurfaceData,
    globals::GlobalData,
    reexports::client::{
        protocol::{
            wl_buffer::{self, WlBuffer},
            wl_compositor::WlCompositor,
            wl_shm::{self, WlShm},
            wl_shm_pool::{self, WlShmPool},
            wl_subcompositor::WlSubcompositor,
            wl_subsurface::WlSubsurface,
            wl_surface::WlSurface,
        },
        Connection, Dispatch, Proxy, QueueHandle,
    },
};
use wayland_protocols::wp::{
    linux_dmabuf::zv1::client::{
        zwp_linux_buffer_params_v1::{self, ZwpLinuxBufferParamsV1},
        zwp_linux_dmabuf_v1::{self, ZwpLinuxDmabufV1},
    },
    viewporter::client::{
        wp_viewport::WpViewport, wp_viewporter::WpViewporter,
    },
};

use crate::event_loop::state::SctkState;

#[derive(Debug)]
pub struct Plane {
    pub fd: OwnedFd,
    pub plane_idx: u32,
    pub offset: u32,
    pub stride: u32,
}

#[derive(Debug)]
pub struct Dmabuf {
    pub width: i32,
    pub height: i32,
    pub planes: Vec<Plane>,
    pub format: u32,
    pub modifier: u64,
}

#[derive(Debug)]
pub struct Shmbuf {
    pub fd: OwnedFd,
    pub offset: i32,
    pub width: i32,
    pub height: i32,
    pub stride: i32,
    pub format: wl_shm::Format,
}

#[derive(Debug)]
pub enum BufferSource {
    Shm(Shmbuf),
    Dma(Dmabuf),
}

impl From<Shmbuf> for BufferSource {
    fn from(buf: Shmbuf) -> Self {
        Self::Shm(buf)
    }
}

impl From<Dmabuf> for BufferSource {
    fn from(buf: Dmabuf) -> Self {
        Self::Dma(buf)
    }
}

#[derive(Debug)]
struct SubsurfaceBufferInner {
    source: Arc<BufferSource>,
    _sender: oneshot::Sender<()>,
}

/// Refcounted type containing a `BufferSource` with a sender that is signaled
/// all references  are dropped and `wl_buffer`s created from the source are
/// released.
#[derive(Clone, Debug)]
pub struct SubsurfaceBuffer(Arc<SubsurfaceBufferInner>);

pub struct BufferData {
    source: SubsurfaceBuffer,
}

/// Future signalled when subsurface buffer is released
pub struct SubsurfaceBufferRelease(oneshot::Receiver<()>);

impl SubsurfaceBufferRelease {
    /// Non-blocking check if buffer is released yet, without awaiting
    pub fn released(&mut self) -> bool {
        self.0.try_recv() == Ok(None)
    }
}

impl Future for SubsurfaceBufferRelease {
    type Output = ();

    fn poll(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context,
    ) -> task::Poll<()> {
        Pin::new(&mut self.0).poll(cx).map(|_| ())
    }
}

impl SubsurfaceBuffer {
    pub fn new(source: Arc<BufferSource>) -> (Self, SubsurfaceBufferRelease) {
        let (_sender, receiver) = oneshot::channel();
        let subsurface_buffer =
            SubsurfaceBuffer(Arc::new(SubsurfaceBufferInner {
                source,
                _sender,
            }));
        (subsurface_buffer, SubsurfaceBufferRelease(receiver))
    }

    // Behavior of `wl_buffer::released` is undefined if attached to multiple surfaces. To allow
    // things like that, create a new `wl_buffer` each time.
    fn create_buffer<T: 'static>(
        &self,
        shm: &WlShm,
        dmabuf: Option<&ZwpLinuxDmabufV1>,
        qh: &QueueHandle<SctkState<T>>,
    ) -> Option<WlBuffer> {
        // create reference to source, that is dropped on release
        match self.0.source.as_ref() {
            BufferSource::Shm(buf) => {
                let pool = shm.create_pool(
                    buf.fd.as_fd(),
                    buf.offset + buf.height * buf.stride,
                    qh,
                    GlobalData,
                );
                let buffer = pool.create_buffer(
                    buf.offset,
                    buf.width,
                    buf.height,
                    buf.stride,
                    buf.format,
                    qh,
                    BufferData {
                        source: self.clone(),
                    },
                );
                pool.destroy();
                Some(buffer)
            }
            BufferSource::Dma(buf) => {
                if let Some(dmabuf) = dmabuf {
                    let params = dmabuf.create_params(qh, GlobalData);
                    for plane in &buf.planes {
                        let modifier_hi = (buf.modifier >> 32) as u32;
                        let modifier_lo = (buf.modifier & 0xffffffff) as u32;
                        params.add(
                            plane.fd.as_fd(),
                            plane.plane_idx,
                            plane.offset,
                            plane.stride,
                            modifier_hi,
                            modifier_lo,
                        );
                    }
                    // Will cause protocol error if format is not supported
                    Some(params.create_immed(
                        buf.width,
                        buf.height,
                        buf.format,
                        zwp_linux_buffer_params_v1::Flags::empty(),
                        qh,
                        BufferData {
                            source: self.clone(),
                        },
                    ))
                } else {
                    None
                }
            }
        }
    }

    fn for_buffer(buffer: &WlBuffer) -> Option<&Self> {
        Some(&buffer.data::<BufferData>()?.source)
    }
}

impl PartialEq for SubsurfaceBuffer {
    fn eq(&self, rhs: &Self) -> bool {
        Arc::ptr_eq(&self.0, &rhs.0)
    }
}

impl<T> Dispatch<WlShmPool, GlobalData> for SctkState<T> {
    fn event(
        _: &mut SctkState<T>,
        _: &WlShmPool,
        _: wl_shm_pool::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<SctkState<T>>,
    ) {
        unreachable!()
    }
}

impl<T> Dispatch<ZwpLinuxDmabufV1, GlobalData> for SctkState<T> {
    fn event(
        _: &mut SctkState<T>,
        _: &ZwpLinuxDmabufV1,
        _: zwp_linux_dmabuf_v1::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<SctkState<T>>,
    ) {
    }
}

impl<T> Dispatch<ZwpLinuxBufferParamsV1, GlobalData> for SctkState<T> {
    fn event(
        _: &mut SctkState<T>,
        _: &ZwpLinuxBufferParamsV1,
        _: zwp_linux_buffer_params_v1::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<SctkState<T>>,
    ) {
    }
}

impl<T> Dispatch<WlBuffer, BufferData> for SctkState<T> {
    fn event(
        _: &mut SctkState<T>,
        _: &WlBuffer,
        event: wl_buffer::Event,
        _: &BufferData,
        _: &Connection,
        _: &QueueHandle<SctkState<T>>,
    ) {
        match event {
            wl_buffer::Event::Release => {}
            _ => unreachable!(),
        }
    }
}

// create wl_buffer from BufferSource (avoid create_immed?)
// release
#[doc(hidden)]
pub struct SubsurfaceState<T> {
    pub wl_compositor: WlCompositor,
    pub wl_subcompositor: WlSubcompositor,
    pub wp_viewporter: WpViewporter,
    pub wl_shm: WlShm,
    pub wp_dmabuf: Option<ZwpLinuxDmabufV1>,
    pub qh: QueueHandle<SctkState<T>>,
}

impl<T: std::fmt::Debug + 'static> SubsurfaceState<T> {
    fn create_subsurface(&self, parent: &WlSurface) -> SubsurfaceInstance {
        let wl_surface = self
            .wl_compositor
            .create_surface(&self.qh, SurfaceData::new(None, 1));
        let wl_subsurface = self.wl_subcompositor.get_subsurface(
            &wl_surface,
            parent,
            &self.qh,
            (),
        );
        let wp_viewport = self.wp_viewporter.get_viewport(
            &wl_surface,
            &self.qh,
            sctk::globals::GlobalData,
        );
        SubsurfaceInstance {
            wl_surface,
            wl_subsurface,
            wp_viewport,
            wl_buffer: None,
            bounds: None,
        }
    }

    // Update `subsurfaces` from `view_subsurfaces`
    pub(crate) fn update_subsurfaces(
        &self,
        parent: &WlSurface,
        subsurfaces: &mut Vec<SubsurfaceInstance>,
        view_subsurfaces: &[SubsurfaceInfo],
    ) {
        // If view requested fewer subsurfaces than there currently are,
        // destroy excess.
        if view_subsurfaces.len() < subsurfaces.len() {
            subsurfaces.truncate(view_subsurfaces.len());
        }
        // Create new subsurfaces if there aren't enough.
        while subsurfaces.len() < view_subsurfaces.len() {
            subsurfaces.push(self.create_subsurface(parent));
        }
        // Attach buffers to subsurfaces, set viewports, and commit.
        for (subsurface_data, subsurface) in
            view_subsurfaces.iter().zip(subsurfaces.iter_mut())
        {
            subsurface.attach_and_commit(
                subsurface_data,
                &self.wl_shm,
                self.wp_dmabuf.as_ref(),
                &self.qh,
            );
        }
    }
}

pub(crate) struct SubsurfaceInstance {
    wl_surface: WlSurface,
    wl_subsurface: WlSubsurface,
    wp_viewport: WpViewport,
    wl_buffer: Option<WlBuffer>,
    bounds: Option<Rectangle<f32>>,
}

impl SubsurfaceInstance {
    // TODO correct damage? no damage/commit if unchanged?
    fn attach_and_commit<T: 'static>(
        &mut self,
        info: &SubsurfaceInfo,
        shm: &WlShm,
        dmabuf: Option<&ZwpLinuxDmabufV1>,
        qh: &QueueHandle<SctkState<T>>,
    ) {
        let buffer_changed;
        let buffer = match self.wl_buffer.take() {
            Some(buffer)
                if SubsurfaceBuffer::for_buffer(&buffer)
                    == Some(&info.buffer) =>
            {
                // Same buffer is already attached to this subsurface. Don't create new `wl_buffer`.
                buffer_changed = false;
                buffer
            }
            buffer => {
                if let Some(buffer) = buffer {
                    buffer.destroy();
                }
                if let Some(buffer) = info.buffer.create_buffer(shm, dmabuf, qh)
                {
                    buffer_changed = true;
                    buffer
                } else {
                    // TODO log error
                    self.wl_surface.attach(None, 0, 0);
                    return;
                }
            }
        };

        // XXX scale factor?
        let bounds_changed = self.bounds != Some(info.bounds);
        // wlroots seems to have issues changing buffer without running this
        if bounds_changed || buffer_changed {
            self.wl_subsurface
                .set_position(info.bounds.x as i32, info.bounds.y as i32);
            self.wp_viewport.set_destination(
                info.bounds.width as i32,
                info.bounds.height as i32,
            );
        }
        if buffer_changed {
            self.wl_surface.attach(Some(&buffer), 0, 0);
            self.wl_surface.damage(0, 0, i32::MAX, i32::MAX);
        }
        if buffer_changed || bounds_changed {
            self.wl_surface.commit();
        }

        self.wl_buffer = Some(buffer);
        self.bounds = Some(info.bounds);
    }
}

impl Drop for SubsurfaceInstance {
    fn drop(&mut self) {
        self.wp_viewport.destroy();
        self.wl_subsurface.destroy();
        self.wl_surface.destroy();
        if let Some(wl_buffer) = self.wl_buffer.as_ref() {
            wl_buffer.destroy();
        }
    }
}

pub(crate) struct SubsurfaceInfo {
    pub buffer: SubsurfaceBuffer,
    pub bounds: Rectangle<f32>,
}

thread_local! {
    static SUBSURFACES: RefCell<Vec<SubsurfaceInfo>> = RefCell::new(Vec::new());
}

pub(crate) fn take_subsurfaces() -> Vec<SubsurfaceInfo> {
    SUBSURFACES.with(|subsurfaces| mem::take(&mut *subsurfaces.borrow_mut()))
}

#[must_use]
pub struct Subsurface<'a> {
    buffer_size: Size<f32>,
    buffer: &'a SubsurfaceBuffer,
    width: Length,
    height: Length,
    content_fit: ContentFit,
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Subsurface<'a>
where
    Renderer: renderer::Renderer,
{
    fn size(&self) -> Size<Length> {
        Size::new(self.width, self.height)
    }

    // Based on image widget
    fn layout(
        &self,
        _tree: &mut widget::Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let raw_size =
            limits.resolve(self.width, self.height, self.buffer_size);

        let full_size = self.content_fit.fit(self.buffer_size, raw_size);

        let final_size = Size {
            width: match self.width {
                Length::Shrink => f32::min(raw_size.width, full_size.width),
                _ => raw_size.width,
            },
            height: match self.height {
                Length::Shrink => f32::min(raw_size.height, full_size.height),
                _ => raw_size.height,
            },
        };

        layout::Node::new(final_size)
    }

    fn draw(
        &self,
        _state: &widget::Tree,
        _renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        // Instead of using renderer, we need to add surface to a list that is
        // read by the iced-sctk shell.
        SUBSURFACES.with(|subsurfaces| {
            subsurfaces.borrow_mut().push(SubsurfaceInfo {
                buffer: self.buffer.clone(),
                bounds: layout.bounds(),
            })
        });
    }
}

impl<'a> Subsurface<'a> {
    pub fn new(
        buffer_width: u32,
        buffer_height: u32,
        buffer: &'a SubsurfaceBuffer,
    ) -> Self {
        Self {
            buffer_size: Size::new(buffer_width as f32, buffer_height as f32),
            buffer,
            // Matches defaults of image widget
            width: Length::Shrink,
            height: Length::Shrink,
            content_fit: ContentFit::Contain,
        }
    }

    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    pub fn height(mut self, height: Length) -> Self {
        self.height = height;
        self
    }

    pub fn content_fit(mut self, content_fit: ContentFit) -> Self {
        self.content_fit = content_fit;
        self
    }
}

impl<'a, Message, Theme, Renderer> From<Subsurface<'a>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: Clone + 'a,
    Renderer: renderer::Renderer,
{
    fn from(subsurface: Subsurface<'a>) -> Self {
        Self::new(subsurface)
    }
}
