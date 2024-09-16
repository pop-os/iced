use crate::platform_specific::{wayland::Action};
use sctk::reexports::{
        calloop::channel,
        client::{
            protocol::{wl_display::WlDisplay, wl_surface::WlSurface},
            Proxy,
        },
    };
use std::sync::{Arc, Mutex};
use winit::{dpi::LogicalSize, window::WindowButtons};

use crate::platform_specific::SurfaceIdWrapper;

use super::event_loop::state::{Common, SctkLayerSurface, SctkLockSurface, SctkPopup};

#[derive(Debug)]
pub(crate) enum Surface {
    Popup(SctkPopup),
    Layer(SctkLayerSurface),
    Lock(SctkLockSurface),
}

impl Surface {}

pub struct SctkWinitWindow {
    tx: channel::Sender<Action>,
    id: SurfaceIdWrapper,
    surface: WlSurface,
    common: Arc<Mutex<Common>>,
    display: WlDisplay,
}

impl SctkWinitWindow {
    pub(crate) fn new(
        tx: channel::Sender<Action>,
        common: Arc<Mutex<Common>>,
        id: SurfaceIdWrapper,
        surface: WlSurface,
        display: WlDisplay,
    ) -> Arc<dyn winit::window::Window> {
        Arc::new(Self {
            tx,
            common,
            id,
            surface,
            display,
        })
    }
}

impl winit::window::Window for SctkWinitWindow {
    fn id(&self) -> winit::window::WindowId {
        winit::window::WindowId::from(self.surface.id().as_ptr() as u64)
    }

    fn scale_factor(&self) -> f64 {
        let guard = self.common.lock().unwrap();
        guard.fractional_scale.unwrap_or(1.)
    }

    fn request_redraw(&self) {
        _ = self.tx.send(Action::RequestRedraw(self.surface.id()));
    }

    fn pre_present_notify(&self) {
        _ = self.tx.send(Action::PrePresentNotify(self.surface.id()));
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

    fn set_cursor_position(
        &self,
        position: winit::dpi::Position,
    ) -> Result<(), winit::error::ExternalError> {
        // TODO
        Ok(())
    }

    fn set_cursor_grab(
        &self,
        mode: winit::window::CursorGrabMode,
    ) -> Result<(), winit::error::ExternalError> {
        // TODO
        Ok(())
    }

    fn set_cursor_visible(&self, visible: bool) {
        // TODO
    }

    fn inner_size(&self) -> winit::dpi::PhysicalSize<u32> {
        let guard = self.common.lock().unwrap();
        let size = guard.size;
        size.to_physical(guard.fractional_scale.unwrap_or(1.))
    }

    fn request_inner_size(
        &self,
        size: winit::dpi::Size,
    ) -> Option<winit::dpi::PhysicalSize<u32>> {
        let guard = self.common.lock().unwrap();

        let size: LogicalSize<u32> =
            size.to_logical(guard.fractional_scale.unwrap_or(1.));
        let action = match &self.id {
            SurfaceIdWrapper::LayerSurface(id) => {
                iced_runtime::platform_specific::wayland::Action::LayerSurface(iced_runtime::platform_specific::wayland::layer_surface::Action::Size { id: id.clone(), width: Some(size.width as u32), height: Some(size.height as u32) })
            }
            SurfaceIdWrapper::Window(_) => unimplemented!(),
            SurfaceIdWrapper::Popup(id) => {
                {
                    iced_runtime::platform_specific::wayland::Action::Popup(iced_runtime::platform_specific::wayland::popup::Action::Size { id: id.clone(), width: size.width as u32, height: size.height as u32 })
                }
            },
            SurfaceIdWrapper::SessionLock(_) => return None,
        };
        _ = self.tx.send(Action::Action(action));
        None
    }

    fn reset_dead_keys(&self) {
        // TODO refer to winit for implementation
    }

    fn inner_position(
        &self,
    ) -> Result<
        winit::dpi::PhysicalPosition<i32>,
        winit::error::NotSupportedError,
    > {
        Err(winit::error::NotSupportedError::default())
    }

    fn outer_position(
        &self,
    ) -> Result<
        winit::dpi::PhysicalPosition<i32>,
        winit::error::NotSupportedError,
    > {
        Ok(Default::default())
    }

    fn set_outer_position(&self, position: winit::dpi::Position) {}

    fn outer_size(&self) -> winit::dpi::PhysicalSize<u32> {
        // XXX not applicable to wrapped surfaces
        Default::default()
    }

    fn set_min_inner_size(&self, min_size: Option<winit::dpi::Size>) {
        // XXX not applicable to wrapped surfaces
    }

    fn set_max_inner_size(&self, max_size: Option<winit::dpi::Size>) {
        // XXX not applicable to wrapped surfaces
    }

    fn resize_increments(&self) -> Option<winit::dpi::PhysicalSize<u32>> {
        None
    }

    fn set_resize_increments(&self, increments: Option<winit::dpi::Size>) {
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

    fn set_cursor_hittest(
        &self,
        hittest: bool,
    ) -> Result<(), winit::error::ExternalError> {
        todo!()
    }

    fn current_monitor(&self) -> Option<winit::monitor::MonitorHandle> {
        todo!()
    }

    fn available_monitors(
        &self,
    ) -> Box<dyn Iterator<Item = winit::monitor::MonitorHandle>> {
        todo!()
    }

    fn has_focus(&self) -> bool {
        todo!()
    }

    fn set_ime_cursor_area(
        &self,
        position: winit::dpi::Position,
        size: winit::dpi::Size,
    ) {
        todo!()
    }

    fn set_ime_allowed(&self, allowed: bool) {
        todo!()
    }

    fn set_ime_purpose(&self, purpose: winit::window::ImePurpose) {
        todo!()
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

    fn drag_window(&self) -> Result<(), winit::error::ExternalError> {
        // XXX can't drag the wrapped surfaces
        Ok(())
    }

    fn drag_resize_window(
        &self,
        direction: winit::window::ResizeDirection,
    ) -> Result<(), winit::error::ExternalError> {
        // XXX can't drag resize the wrapped surfaces
        Ok(())
    }

    fn show_window_menu(&self, position: winit::dpi::Position) {
        // XXX can't show window menu on wrapped surfaces
    }

    fn primary_monitor(&self) -> Option<winit::monitor::MonitorHandle> {
        None
    }
}

impl raw_window_handle::HasWindowHandle for SctkWinitWindow {
    fn window_handle(
        &self,
    ) -> Result<raw_window_handle::WindowHandle, raw_window_handle::HandleError>
    {
        let raw = raw_window_handle::WaylandWindowHandle::new({
            let ptr = self.surface.id().as_ptr();
            std::ptr::NonNull::new(ptr as *mut _)
                .expect("wl_surface will never be null")
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
