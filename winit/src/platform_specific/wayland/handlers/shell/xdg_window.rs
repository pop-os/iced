use crate::platform_specific::wayland::event_loop::state::SctkState;
use cctk::sctk::{
    shell::xdg::window::WindowHandler,
};

impl WindowHandler for SctkState {
    fn request_close(
        &mut self,
        _conn: &cctk::sctk::reexports::client::Connection,
        _qh: &cctk::sctk::reexports::client::QueueHandle<Self>,
        _window: &cctk::sctk::shell::xdg::window::Window,
    ) {
    }

    fn configure(
        &mut self,
        _conn: &cctk::sctk::reexports::client::Connection,
        _qh: &cctk::sctk::reexports::client::QueueHandle<Self>,
        _window: &cctk::sctk::shell::xdg::window::Window,
        _configure: cctk::sctk::shell::xdg::window::WindowConfigure,
        _serial: u32,
    ) {
    }
}
