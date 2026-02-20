use crate::platform_specific::wayland::Action;
use cctk::sctk::reexports::{
    calloop::{channel, LoopHandle},
    client::{protocol::wl_display::WlDisplay, Proxy, QueueHandle},
};
use raw_window_handle::HandleError;
use std::sync::{Arc, Mutex};
use winit::{
    dpi::LogicalSize,
    error::{NotSupportedError, RequestError},
    window::WindowButtons,
};

use crate::platform_specific::SurfaceIdWrapper;

use super::event_loop::state::{Common, CommonSurface, SctkState, TOKEN_CTR};

pub struct SctkWinitWindow {
    tx: channel::Sender<Action>,
    id: SurfaceIdWrapper,
    surface: CommonSurface,
    common: Arc<Mutex<Common>>,
    display: WlDisplay,
    pub(crate) queue_handle: QueueHandle<SctkState>,
}

impl Drop for SctkWinitWindow {
    fn drop(&mut self) {
        self.tx.send(Action::Dropped(self.id)).unwrap();
    }
}

impl SctkWinitWindow {
    pub(crate) fn new(
        tx: channel::Sender<Action>,
        common: Arc<Mutex<Common>>,
        id: SurfaceIdWrapper,
        surface: CommonSurface,
        display: WlDisplay,
        queue_handle: QueueHandle<SctkState>,
    ) -> Arc<dyn winit::window::Window> {
        Arc::new(Self {
            tx,
            common,
            id,
            surface,
            display,
            queue_handle,
        })
    }
}

impl winit::window::Window for SctkWinitWindow {
    fn id(&self) -> winit::window::WindowId {
        winit::window::WindowId::from(
            self.surface.wl_surface().id().as_ptr() as u64
        )
    }

    fn scale_factor(&self) -> f64 {
        let guard = self.common.lock().unwrap();
        guard.fractional_scale.unwrap_or(1.)
    }

    fn request_redraw(&self) {
        let surface = self.surface.wl_surface();
        _ = self.tx.send(Action::RequestRedraw(surface.id()));
    }

    fn pre_present_notify(&self) {
        let surface = self.surface.wl_surface();
        _ = surface.frame(&self.queue_handle, surface.clone());
    }

    fn set_cursor(&self, cursor: winit::window::Cursor) {
        match cursor {
            winit::window::Cursor::Icon(icon) => {
                _ = self.tx.send(Action::SetCursor(icon));
            }
            winit::window::Cursor::Custom(_) => {
                // TODO
            }
        }
    }

    fn set_cursor_visible(&self, visible: bool) {
        // TODO
    }

    fn surface_size(&self) -> winit::dpi::PhysicalSize<u32> {
        let guard = self.common.lock().unwrap();
        let size = guard.size;
        size.to_physical(guard.fractional_scale.unwrap_or(1.))
    }

    fn request_surface_size(
        &self,
        size: winit::dpi::Size,
    ) -> Option<winit::dpi::PhysicalSize<u32>> {
        let mut guard = self.common.lock().unwrap();
        self.request_redraw();
        let logical_size: LogicalSize<u32> =
            size.to_logical(guard.fractional_scale.unwrap_or(1.));
        match &self.surface {
            CommonSurface::Popup(popup, positioner) => {
                if logical_size.width == 0 || logical_size.height == 0 {
                    return None;
                }
                if guard.size.width == logical_size.width
                    && guard.size.height == logical_size.height
                {
                    return None;
                }

                guard.size = logical_size;
                guard.requested_size.0 = Some(guard.size.width);
                guard.requested_size.1 = Some(guard.size.height);
                positioner.set_size(
                    guard.size.width as i32,
                    guard.size.height as i32,
                );
                popup.xdg_surface().set_window_geometry(
                    0,
                    0,
                    guard.size.width as i32,
                    guard.size.height as i32,
                );
                popup.xdg_popup().reposition(
                    positioner,
                    TOKEN_CTR
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                );
                if let Some(viewport) = guard.wp_viewport.as_ref() {
                    // Set inner size without the borders.
                    viewport.set_destination(
                        guard.size.width as i32,
                        guard.size.height as i32,
                    );
                }
            }
            CommonSurface::Layer(layer_surface) => {
                guard.requested_size = (
                    (logical_size.width > 0).then_some(logical_size.width),
                    (logical_size.height > 0).then_some(logical_size.height),
                );
                if guard.size.width == logical_size.width
                    && guard.size.height == logical_size.height
                {
                    return None;
                }
                if logical_size.width > 0 {
                    guard.size.width = logical_size.width;
                }
                if logical_size.height > 0 {
                    guard.size.height = logical_size.height;
                }

                layer_surface.set_size(logical_size.width, logical_size.height);
                if let Some(viewport) = guard.wp_viewport.as_ref() {
                    // Set inner size without the borders.
                    viewport.set_destination(
                        guard.size.width as i32,
                        guard.size.height as i32,
                    );
                }
            }
            CommonSurface::Lock(_) => {}
            CommonSurface::Subsurface { .. } => {
                if guard.size.width == logical_size.width
                    && guard.size.height == logical_size.height
                {
                    return None;
                }
                guard.requested_size = (
                    (logical_size.width > 0).then_some(logical_size.width),
                    (logical_size.height > 0).then_some(logical_size.height),
                );
                guard.size = logical_size;
                if let Some(viewport) = guard.wp_viewport.as_ref() {
                    // Set inner size without the borders.
                    viewport.set_destination(
                        guard.size.width as i32,
                        guard.size.height as i32,
                    );
                }
                _ = self
                    .tx
                    .send(Action::SubsurfaceResize(self.id.inner(), size));
            }
        }
        None
    }

    fn reset_dead_keys(&self) {
        // TODO refer to winit for implementation
    }

    fn set_outer_position(&self, position: winit::dpi::Position) {}

    fn outer_size(&self) -> winit::dpi::PhysicalSize<u32> {
        // XXX not applicable to wrapped surfaces
        Default::default()
    }

    fn set_min_surface_size(&self, min_size: Option<winit::dpi::Size>) {
        // XXX not applicable to wrapped surfaces
    }

    fn set_max_surface_size(&self, max_size: Option<winit::dpi::Size>) {
        // XXX not applicable to wrapped surfaces
    }

    fn set_surface_resize_increments(
        &self,
        increments: Option<winit::dpi::Size>,
    ) {
        log::warn!(
            "`set_surface_resize_increments` is not implemented for Wayland"
        )
    }

    fn set_title(&self, title: &str) {
        // XXX not applicable to wrapped surfaces
    }

    fn set_transparent(&self, transparent: bool) {
        todo!()
    }

    fn rwh_06_display_handle(
        &self,
    ) -> &dyn raw_window_handle::HasDisplayHandle {
        self
    }

    fn rwh_06_window_handle(&self) -> &dyn raw_window_handle::HasWindowHandle {
        self
    }

    fn current_monitor(&self) -> Option<winit::monitor::MonitorHandle> {
        tracing::warn!(
            "current_monitor is not implemented for wayland windows."
        );
        None
    }

    fn available_monitors(
        &self,
    ) -> Box<dyn Iterator<Item = winit::monitor::MonitorHandle>> {
        Box::new(None.into_iter())
    }

    fn has_focus(&self) -> bool {
        tracing::warn!("has_focus is not implemented for wayland windows.");
        false
    }

    fn set_ime_cursor_area(
        &self,
        position: winit::dpi::Position,
        size: winit::dpi::Size,
    ) {
        let guard = self.common.lock().unwrap();
        let scale_factor = guard.fractional_scale.unwrap_or(1.);
        let position = position.to_logical(scale_factor);
        let size = size.to_logical(scale_factor);
        _ = self.tx.send(Action::SetImeCursorArea(
            position.x,
            position.y,
            size.width,
            size.height,
        ));
    }

    fn set_ime_allowed(&self, allowed: bool) {
        _ = self.tx.send(Action::SetImeAllowed(allowed));
    }

    fn set_ime_purpose(&self, purpose: winit::window::ImePurpose) {
        _ = self.tx.send(Action::SetImePurpose(purpose));
    }

    fn set_blur(&self, blur: bool) {
        // TODO
    }

    fn set_visible(&self, visible: bool) {}

    fn is_visible(&self) -> Option<bool> {
        None
    }

    fn set_resizable(&self, resizable: bool) {}

    fn is_resizable(&self) -> bool {
        false
    }

    fn set_enabled_buttons(&self, buttons: winit::window::WindowButtons) {
        // TODO v5 of xdg_shell.
    }

    fn enabled_buttons(&self) -> winit::window::WindowButtons {
        WindowButtons::all()
    }

    fn set_minimized(&self, minimized: bool) {
        // XXX not applicable to the wrapped surfaces
    }

    fn is_minimized(&self) -> Option<bool> {
        // XXX clients don't know whether they are minimized or not.
        None
    }

    fn set_maximized(&self, maximized: bool) {
        // XXX can't minimize the wrapped surfaces
    }

    fn is_maximized(&self) -> bool {
        // XXX can't maximize the wrapped surfaces
        false
    }

    fn set_fullscreen(&self, fullscreen: Option<winit::window::Fullscreen>) {
        // XXX can't fullscreen the wrapped surfaces
    }

    fn fullscreen(&self) -> Option<winit::window::Fullscreen> {
        // XXX can't fullscreen the wrapped surfaces
        None
    }

    fn set_decorations(&self, decorations: bool) {
        // XXX no decorations supported for the wrapped surfaces
    }

    fn is_decorated(&self) -> bool {
        false
    }

    fn set_window_level(&self, level: winit::window::WindowLevel) {}

    fn set_window_icon(&self, window_icon: Option<winit::window::Icon>) {}

    fn focus_window(&self) {}

    fn request_user_attention(
        &self,
        request_type: Option<winit::window::UserAttentionType>,
    ) {
        // XXX can't request attention on wrapped surfaces
    }

    fn set_theme(&self, theme: Option<winit::window::Theme>) {}

    fn theme(&self) -> Option<winit::window::Theme> {
        None
    }

    fn set_content_protected(&self, protected: bool) {}

    fn title(&self) -> String {
        String::new()
    }

    fn show_window_menu(&self, _position: winit::dpi::Position) {
        // XXX can't show window menu on wrapped surfaces
    }

    fn primary_monitor(&self) -> Option<winit::monitor::MonitorHandle> {
        None
    }

    fn surface_resize_increments(
        &self,
    ) -> Option<winit::dpi::PhysicalSize<u32>> {
        None
    }

    fn drag_window(&self) -> Result<(), winit::error::RequestError> {
        Ok(())
    }

    fn drag_resize_window(
        &self,
        _direction: winit::window::ResizeDirection,
    ) -> Result<(), winit::error::RequestError> {
        Ok(())
    }

    fn set_cursor_hittest(
        &self,
        _hittest: bool,
    ) -> Result<(), winit::error::RequestError> {
        todo!()
    }

    fn inner_position(
        &self,
    ) -> Result<winit::dpi::PhysicalPosition<i32>, winit::error::RequestError>
    {
        Err(RequestError::NotSupported(NotSupportedError::new(
            "Not supported on wayland.",
        )))
    }

    fn outer_position(
        &self,
    ) -> Result<winit::dpi::PhysicalPosition<i32>, winit::error::RequestError>
    {
        Err(RequestError::NotSupported(NotSupportedError::new(
            "Not supported on wayland.",
        )))
    }

    fn set_cursor_position(
        &self,
        position: winit::dpi::Position,
    ) -> Result<(), winit::error::RequestError> {
        todo!()
    }

    fn set_cursor_grab(
        &self,
        mode: winit::window::CursorGrabMode,
    ) -> Result<(), winit::error::RequestError> {
        todo!()
    }
}

impl raw_window_handle::HasWindowHandle for SctkWinitWindow {
    fn window_handle(
        &self,
    ) -> Result<raw_window_handle::WindowHandle, raw_window_handle::HandleError>
    {
        let raw = raw_window_handle::WaylandWindowHandle::new({
            let ptr = self.surface.wl_surface().id().as_ptr();
            let Some(ptr) = std::ptr::NonNull::new(ptr as *mut _) else {
                return Err(HandleError::Unavailable);
            };
            ptr
        });

        unsafe { Ok(raw_window_handle::WindowHandle::borrow_raw(raw.into())) }
    }
}

impl raw_window_handle::HasDisplayHandle for SctkWinitWindow {
    fn display_handle(
        &self,
    ) -> Result<
        raw_window_handle::DisplayHandle<'_>,
        raw_window_handle::HandleError,
    > {
        let raw = raw_window_handle::WaylandDisplayHandle::new({
            let ptr = self.display.id().as_ptr();
            std::ptr::NonNull::new(ptr as *mut _)
                .expect("wl_proxy should never be null")
        });

        unsafe { Ok(raw_window_handle::DisplayHandle::borrow_raw(raw.into())) }
    }
}
