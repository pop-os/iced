// TODO z-order option?

use crate::core::{
    ContentFit, Element, Length, Rectangle, Size, Vector,
    layout::{self, Layout},
    mouse, renderer,
    widget::{self, Widget},
};
use std::{
    borrow::BorrowMut,
    cell::RefCell,
    collections::HashMap,
    fmt::Debug,
    future::Future,
    hash::{Hash, Hasher},
    mem,
    os::unix::io::{AsFd, OwnedFd},
    pin::Pin,
    ptr,
    sync::{Arc, Mutex, Weak},
    task,
};

use crate::futures::futures::channel::oneshot;
use cctk::sctk::{
    compositor::SurfaceData,
    error::GlobalError,
    globals::{GlobalData, ProvidesBoundGlobal},
    reexports::client::{
        Connection, Dispatch, Proxy, QueueHandle, delegate_noop,
        protocol::{
            wl_buffer::{self, WlBuffer},
            wl_compositor::WlCompositor,
            wl_output,
            wl_shm::{self, WlShm},
            wl_shm_pool::{self, WlShmPool},
            wl_subcompositor::WlSubcompositor,
            wl_subsurface::WlSubsurface,
            wl_surface::WlSurface,
        },
    },
    shm::slot::SlotPool,
};
use iced_futures::core::window;
use wayland_backend::client::ObjectId;
use wayland_protocols::wp::{
    alpha_modifier::v1::client::{
        wp_alpha_modifier_surface_v1::WpAlphaModifierSurfaceV1,
        wp_alpha_modifier_v1::WpAlphaModifierV1,
    },
    fractional_scale::v1::client::wp_fractional_scale_v1::WpFractionalScaleV1,
    linux_dmabuf::zv1::client::{
        zwp_linux_buffer_params_v1::{self, ZwpLinuxBufferParamsV1},
        zwp_linux_dmabuf_v1::{self, ZwpLinuxDmabufV1},
    },
    viewporter::client::{
        wp_viewport::WpViewport, wp_viewporter::WpViewporter,
    },
};
use winit::window::WindowId;

use crate::platform_specific::{
    SurfaceIdWrapper, event_loop::state::SctkState,
};

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
    source: WeakBufferSource,
    // This reference is held until the surface `release`s the buffer
    subsurface_buffer: Mutex<Option<SubsurfaceBuffer>>,
}

impl BufferData {
    fn for_buffer(buffer: &WlBuffer) -> Option<&Self> {
        buffer.data::<BufferData>()
    }
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
        cx: &mut task::Context<'_>,
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

    pub fn width(&self) -> i32 {
        match &*self.0.source {
            BufferSource::Dma(dma) => dma.width,
            BufferSource::Shm(shm) => shm.width,
        }
    }

    pub fn height(&self) -> i32 {
        match &*self.0.source {
            BufferSource::Dma(dma) => dma.height,
            BufferSource::Shm(shm) => shm.height,
        }
    }

    // Behavior of `wl_buffer::released` is undefined if attached to multiple surfaces. To allow
    // things like that, create a new `wl_buffer` each time.
    fn create_buffer(
        &self,
        shm: &WlShm,
        dmabuf: Option<&ZwpLinuxDmabufV1>,
        qh: &QueueHandle<SctkState>,
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
                        source: WeakBufferSource(Arc::downgrade(
                            &self.0.source,
                        )),
                        subsurface_buffer: Mutex::new(Some(self.clone())),
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
                            source: WeakBufferSource(Arc::downgrade(
                                &self.0.source,
                            )),
                            subsurface_buffer: Mutex::new(Some(self.clone())),
                        },
                    ))
                } else {
                    None
                }
            }
        }
    }
}

impl PartialEq for SubsurfaceBuffer {
    fn eq(&self, rhs: &Self) -> bool {
        Arc::ptr_eq(&self.0, &rhs.0)
    }
}

impl Dispatch<WlShmPool, GlobalData> for SctkState {
    fn event(
        _: &mut SctkState,
        _: &WlShmPool,
        _: wl_shm_pool::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<SctkState>,
    ) {
        unreachable!()
    }
}

impl Dispatch<ZwpLinuxDmabufV1, GlobalData> for SctkState {
    fn event(
        _: &mut SctkState,
        _: &ZwpLinuxDmabufV1,
        _: zwp_linux_dmabuf_v1::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<SctkState>,
    ) {
    }
}

impl Dispatch<ZwpLinuxBufferParamsV1, GlobalData> for SctkState {
    fn event(
        _: &mut SctkState,
        _: &ZwpLinuxBufferParamsV1,
        _: zwp_linux_buffer_params_v1::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<SctkState>,
    ) {
    }
}

impl Dispatch<WlBuffer, GlobalData> for SctkState {
    fn event(
        _: &mut SctkState,
        _: &WlBuffer,
        event: wl_buffer::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<SctkState>,
    ) {
        match event {
            wl_buffer::Event::Release => {}
            _ => unreachable!(),
        }
    }
}

impl Dispatch<WlBuffer, BufferData> for SctkState {
    fn event(
        _: &mut SctkState,
        _: &WlBuffer,
        event: wl_buffer::Event,
        data: &BufferData,
        _: &Connection,
        _: &QueueHandle<SctkState>,
    ) {
        match event {
            wl_buffer::Event::Release => {
                // Release reference to `SubsurfaceBuffer`
                _ = data.subsurface_buffer.lock().unwrap().take();
            }
            _ => unreachable!(),
        }
    }
}

#[doc(hidden)]
#[derive(Clone, Debug)]
pub(crate) struct WeakBufferSource(Weak<BufferSource>);

impl PartialEq for WeakBufferSource {
    fn eq(&self, rhs: &Self) -> bool {
        Weak::ptr_eq(&self.0, &rhs.0)
    }
}

impl Eq for WeakBufferSource {}

impl Hash for WeakBufferSource {
    fn hash<H: Hasher>(&self, state: &mut H) {
        ptr::hash::<BufferSource, _>(self.0.as_ptr(), state)
    }
}

// Implement `ProvidesBoundGlobal` to use `SlotPool`
struct ShmGlobal<'a>(&'a WlShm);

impl<'a> ProvidesBoundGlobal<WlShm, 1> for ShmGlobal<'a> {
    fn bound_global(&self) -> Result<WlShm, GlobalError> {
        Ok(self.0.clone())
    }
}

// create wl_buffer from BufferSource (avoid create_immed?)
// release
#[derive(Debug, Clone)]
#[doc(hidden)]
pub struct SubsurfaceState {
    pub wl_compositor: WlCompositor,
    pub wl_subcompositor: WlSubcompositor,
    pub wp_viewporter: WpViewporter,
    pub wl_shm: WlShm,
    pub wp_dmabuf: Option<ZwpLinuxDmabufV1>,
    pub wp_alpha_modifier: Option<WpAlphaModifierV1>,
    pub qh: QueueHandle<SctkState>,
    pub(crate) buffers: HashMap<WeakBufferSource, Vec<WlBuffer>>,
    pub(crate) unmapped_subsurfaces: Vec<SubsurfaceInstance>,
    pub new_iced_subsurfaces: Vec<(
        window::Id,
        ObjectId,
        window::Id,
        WlSubsurface,
        WlSurface,
        u32,
    )>,
}

impl SubsurfaceState {
    pub fn create_surface(&self) -> WlSurface {
        self.wl_compositor
            .create_surface(&self.qh, SurfaceData::new(None, 1))
    }

    pub fn update_surface_shm(
        &self,
        surface: &WlSurface,
        width: u32,
        height: u32,
        scale: f64,
        data: &[u8],
        offset: Vector,
    ) {
        let wp_viewport = self.wp_viewporter.get_viewport(
            &surface,
            &self.qh,
            cctk::sctk::globals::GlobalData,
        );
        let shm = ShmGlobal(&self.wl_shm);
        let mut pool =
            SlotPool::new(width as usize * height as usize * 4, &shm).unwrap();
        let (buffer, canvas) = pool
            .create_buffer(
                width as i32,
                height as i32,
                width as i32 * 4,
                wl_shm::Format::Argb8888,
            )
            .unwrap();
        canvas[0..width as usize * height as usize * 4].copy_from_slice(data);
        surface.damage_buffer(0, 0, width as i32, height as i32);
        buffer.attach_to(&surface);
        surface.offset(offset.x as i32, offset.y as i32);
        wp_viewport.set_destination(
            (width as f64 / scale) as i32,
            (height as f64 / scale) as i32,
        );
        surface.commit();
        wp_viewport.destroy();
    }

    fn create_subsurface(&self, parent: &WlSurface) -> SubsurfaceInstance {
        let wl_surface = self
            .wl_compositor
            .create_surface(&self.qh, SurfaceData::new(None, 1));

        // Use empty input region so parent surface gets pointer events
        let region = self.wl_compositor.create_region(&self.qh, ());
        wl_surface.set_input_region(Some(&region));
        region.destroy();

        let wl_subsurface = self.wl_subcompositor.get_subsurface(
            &wl_surface,
            parent,
            &self.qh,
            (),
        );

        let wp_viewport = self.wp_viewporter.get_viewport(
            &wl_surface,
            &self.qh,
            cctk::sctk::globals::GlobalData,
        );

        let wp_alpha_modifier_surface =
            self.wp_alpha_modifier.as_ref().map(|wp_alpha_modifier| {
                wp_alpha_modifier.get_surface(&wl_surface, &self.qh, ())
            });

        SubsurfaceInstance {
            wl_surface,
            wl_subsurface,
            wp_viewport,
            wp_alpha_modifier_surface,
            wl_buffer: None,
            bounds: None,
            wp_fractional_scale: None,
            transform: wl_output::Transform::Normal,
            z: 0,
            parent: parent.id(),
        }
    }

    // Update `subsurfaces` from `view_subsurfaces`
    pub(crate) fn update_subsurfaces(
        &mut self,
        parent: &WlSurface,
        subsurfaces: &mut Vec<SubsurfaceInstance>,
        view_subsurfaces: &[SubsurfaceInfo],
    ) {
        // Subsurfaces aren't destroyed immediately to sync removal with parent
        // surface commit. Since `destroy` is immediate.
        //
        // They should be safe to destroy by the next time `update_subsurfaces`
        // is run.
        ICED_SUBSURFACES.with_borrow_mut(|surfaces| {
            surfaces.retain(|s| {
                !self
                    .unmapped_subsurfaces
                    .iter()
                    .any(|unmapped| unmapped.wl_surface == s.4)
            })
        });
        self.unmapped_subsurfaces.clear();

        // Remove cached `wl_buffers` for any `BufferSource`s that no longer exist.
        self.buffers.retain(|k, v| {
            let retain = k.0.strong_count() > 0;
            if !retain {
                v.iter().for_each(|b| b.destroy());
            }
            retain
        });

        // If view requested fewer subsurfaces than there currently are,
        // unmap excess.
        while view_subsurfaces.len() < subsurfaces.len() {
            let subsurface = subsurfaces.pop().unwrap();
            subsurface.unmap();
            self.unmapped_subsurfaces.push(subsurface);
        }
        let needs_sorting = subsurfaces.len() < view_subsurfaces.len()
            || !self.new_iced_subsurfaces.is_empty();

        // Create new subsurfaces if there aren't enough.
        while subsurfaces.len() < view_subsurfaces.len() {
            subsurfaces.push(self.create_subsurface(parent));
        }
        if needs_sorting {
            let mut sorted_subsurfaces: Vec<_> = view_subsurfaces
                .iter()
                .zip(subsurfaces.iter_mut())
                .map(|(_, instance)| {
                    (
                        instance.parent.clone(),
                        instance.wl_subsurface.clone(),
                        instance.wl_surface.clone(),
                        instance.z,
                    )
                })
                .chain(self.new_iced_subsurfaces.clone().into_iter().map(
                    |(_, parent, _, wl_subsurface, wl_surface, z)| {
                        (parent.clone(), wl_subsurface, wl_surface, z)
                    },
                ))
                .chain(ICED_SUBSURFACES.with(|surfaces| {
                    let b = surfaces.borrow();
                    let v: Vec<_> = b
                        .iter()
                        .map(move |s| {
                            (s.1.clone(), s.3.clone(), s.4.clone(), s.5)
                        })
                        .collect();
                    v.into_iter()
                }))
                .collect();

            sorted_subsurfaces.sort_by(|a, b| a.3.cmp(&b.3));

            // Attach buffers to subsurfaces, set viewports, and commit.
            for i in 1..sorted_subsurfaces.len() {
                let prev = &sorted_subsurfaces[0..i];
                let subsurface = &sorted_subsurfaces[i];
                for prev in prev.iter().rev() {
                    if prev.0 != subsurface.0 {
                        continue;
                    }
                    subsurface.1.place_above(&prev.2);
                }
            }
        }
        if !self.new_iced_subsurfaces.is_empty() {
            ICED_SUBSURFACES.with(|surfaces| {
                surfaces.borrow_mut().append(&mut self.new_iced_subsurfaces);
            })
        };
        for (subsurface_data, subsurface) in
            view_subsurfaces.iter().zip(subsurfaces.iter_mut())
        {
            subsurface.attach_and_commit(subsurface_data, self);
        }
    }

    // Cache `wl_buffer` for use when `BufferSource` is used in future
    // (Avoid creating wl_buffer each buffer swap)
    fn insert_cached_wl_buffer(&mut self, buffer: WlBuffer) {
        let source = BufferData::for_buffer(&buffer).unwrap().source.clone();
        self.buffers.entry(source).or_default().push(buffer);
    }

    // Gets a cached `wl_buffer` for the `SubsurfaceBuffer`, if any. And stores `SubsurfaceBuffer`
    // reference to be releated on `wl_buffer` release.
    //
    // If `wl_buffer` isn't released, it is destroyed instead.
    fn get_cached_wl_buffer(
        &mut self,
        subsurface_buffer: &SubsurfaceBuffer,
    ) -> Option<WlBuffer> {
        let buffers = self.buffers.get_mut(&WeakBufferSource(
            Arc::downgrade(&subsurface_buffer.0.source),
        ))?;
        while let Some(buffer) = buffers.pop() {
            let mut subsurface_buffer_ref = buffer
                .data::<BufferData>()
                .unwrap()
                .subsurface_buffer
                .lock()
                .unwrap();
            if subsurface_buffer_ref.is_none() {
                *subsurface_buffer_ref = Some(subsurface_buffer.clone());
                drop(subsurface_buffer_ref);
                return Some(buffer);
            } else {
                buffer.destroy();
            }
        }
        None
    }
}

impl Drop for SubsurfaceState {
    fn drop(&mut self) {
        self.buffers
            .values()
            .flatten()
            .for_each(|buffer| buffer.destroy());
    }
}

#[derive(Clone, Debug)]
pub(crate) struct SubsurfaceInstance {
    pub(crate) wl_surface: WlSurface,
    pub(crate) wl_subsurface: WlSubsurface,
    pub(crate) wp_viewport: WpViewport,
    pub(crate) wp_fractional_scale: Option<WpFractionalScaleV1>,
    pub(crate) wp_alpha_modifier_surface: Option<WpAlphaModifierSurfaceV1>,
    pub(crate) wl_buffer: Option<WlBuffer>,
    pub(crate) bounds: Option<Rectangle<f32>>,
    pub(crate) transform: wl_output::Transform,
    pub(crate) z: u32,
    pub parent: ObjectId,
}

impl SubsurfaceInstance {
    // TODO correct damage? no damage/commit if unchanged?
    fn attach_and_commit(
        &mut self,
        info: &SubsurfaceInfo,
        state: &mut SubsurfaceState,
    ) {
        let buffer_changed;

        let old_buffer = self.wl_buffer.take();
        let old_buffer_data =
            old_buffer.as_ref().and_then(|b| BufferData::for_buffer(&b));
        let buffer = if old_buffer_data.is_some_and(|b| {
            b.subsurface_buffer.lock().unwrap().as_ref() == Some(&info.buffer)
        }) {
            // Same "BufferSource" is already attached to this subsurface. Don't create new `wl_buffer`.
            buffer_changed = false;
            old_buffer.unwrap()
        } else {
            if let Some(old_buffer) = old_buffer {
                state.insert_cached_wl_buffer(old_buffer);
            }

            buffer_changed = true;

            if let Some(buffer) = state.get_cached_wl_buffer(&info.buffer) {
                buffer
            } else if let Some(buffer) = info.buffer.create_buffer(
                &state.wl_shm,
                state.wp_dmabuf.as_ref(),
                &state.qh,
            ) {
                buffer
            } else {
                // TODO log error
                self.wl_surface.attach(None, 0, 0);
                return;
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
        let transform_changed = self.transform != info.transform;
        if transform_changed {
            self.wl_surface.set_buffer_transform(info.transform);
        }
        if buffer_changed {
            self.wl_surface.attach(Some(&buffer), 0, 0);
            self.wl_surface.damage(0, 0, i32::MAX, i32::MAX);
        }
        if buffer_changed || bounds_changed || transform_changed {
            _ = self.wl_surface.frame(&state.qh, self.wl_surface.clone());
            self.wl_surface.commit();
        }

        if let Some(wp_alpha_modifier_surface) = &self.wp_alpha_modifier_surface
        {
            let alpha = (info.alpha.clamp(0.0, 1.0) * u32::MAX as f32) as u32;
            wp_alpha_modifier_surface.set_multiplier(alpha);
        }

        self.wl_buffer = Some(buffer);
        self.bounds = Some(info.bounds);
        self.transform = info.transform;
        self.z = info.z;
    }

    pub fn unmap(&self) {
        self.wl_surface.attach(None, 0, 0);
        self.wl_surface.commit();
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

#[derive(Debug)]
pub(crate) struct SubsurfaceInfo {
    pub buffer: SubsurfaceBuffer,
    pub bounds: Rectangle<f32>,
    pub alpha: f32,
    pub transform: wl_output::Transform,
    pub z: u32,
}

thread_local! {
    static SUBSURFACES: RefCell<Vec<SubsurfaceInfo>> = RefCell::new(Vec::new());
    static ICED_SUBSURFACES: RefCell<Vec<(window::Id, ObjectId, window::Id, WlSubsurface, WlSurface, u32)>> = RefCell::new(Vec::new());
}

pub(crate) fn take_subsurfaces() -> Vec<SubsurfaceInfo> {
    SUBSURFACES.with(|subsurfaces| mem::take(&mut *subsurfaces.borrow_mut()))
}

pub(crate) fn subsurface_ids(parent: WindowId) -> Vec<WindowId> {
    ICED_SUBSURFACES.with(|subsurfaces| {
        subsurfaces
            .borrow_mut()
            .iter()
            .filter_map(|s| {
                if winit::window::WindowId::from_raw(s.1.as_ptr() as usize)
                    == parent
                {
                    Some(winit::window::WindowId::from_raw(
                        s.4.id().as_ptr() as usize
                    ))
                } else {
                    None
                }
            })
            .collect()
    })
}

pub(crate) fn remove_iced_subsurface(surface: &WlSurface) {
    ICED_SUBSURFACES.with(|surfaces| {
        surfaces
            .borrow_mut()
            .retain(|(_, _, _, _, s, _)| s != surface)
    })
}

#[must_use]
pub struct Subsurface {
    buffer: SubsurfaceBuffer,
    width: Length,
    height: Length,
    content_fit: ContentFit,
    alpha: f32,
    transform: wl_output::Transform,
    pub z: u32,
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for Subsurface
where
    Renderer: renderer::Renderer,
{
    fn size(&self) -> Size<Length> {
        Size::new(self.width, self.height)
    }

    // Based on image widget
    fn layout(
        &mut self,
        _tree: &mut widget::Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let (width, height) = match self.transform {
            wl_output::Transform::_90
            | wl_output::Transform::_270
            | wl_output::Transform::Flipped90
            | wl_output::Transform::Flipped270 => {
                (self.buffer.height(), self.buffer.width())
            }
            _ => (self.buffer.width(), self.buffer.height()),
        };
        let buffer_size = Size::new(width as f32, height as f32);

        // TODO apply transform
        let raw_size = limits.resolve(self.width, self.height, buffer_size);

        let full_size = self.content_fit.fit(buffer_size, raw_size);

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
                alpha: self.alpha,
                transform: self.transform,
                z: self.z,
            })
        });
    }
}

impl Subsurface {
    pub fn new(buffer: SubsurfaceBuffer) -> Self {
        Self {
            buffer,
            // Matches defaults of image widget
            width: Length::Shrink,
            height: Length::Shrink,
            content_fit: ContentFit::Contain,
            alpha: 1.,
            transform: wl_output::Transform::Normal,
            z: 0,
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

    pub fn alpha(mut self, alpha: f32) -> Self {
        self.alpha = alpha;
        self
    }

    pub fn z(mut self, z: u32) -> Self {
        self.z = z;
        self
    }

    pub fn transform(mut self, transform: wl_output::Transform) -> Self {
        self.transform = transform;
        self
    }
}

impl<Message, Theme, Renderer> From<Subsurface>
    for Element<'static, Message, Theme, Renderer>
where
    Message: Clone,
    Renderer: renderer::Renderer,
{
    fn from(subsurface: Subsurface) -> Self {
        Self::new(subsurface)
    }
}

delegate_noop!(SctkState: ignore WpAlphaModifierV1);
delegate_noop!(SctkState: ignore WpAlphaModifierSurfaceV1);
