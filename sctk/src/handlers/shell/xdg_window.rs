use crate::{
    dpi::LogicalSize,
    event_loop::state::SctkState,
    sctk_event::{SctkEvent, WindowEventVariant},
};
use sctk::{
    delegate_xdg_shell, delegate_xdg_window,
    shell::{xdg::window::WindowHandler, WaylandSurface},
};
use std::{fmt::Debug, num::NonZeroU32};

impl<T: Debug> WindowHandler for SctkState<T> {
    fn request_close(
        &mut self,
        _conn: &sctk::reexports::client::Connection,
        _qh: &sctk::reexports::client::QueueHandle<Self>,
        window: &sctk::shell::xdg::window::Window,
    ) {
        let window = match self
            .windows
            .iter()
            .find(|s| s.window.wl_surface() == window.wl_surface())
        {
            Some(w) => w,
            None => return,
        };

        self.sctk_events.push(SctkEvent::WindowEvent {
            variant: WindowEventVariant::Close,
            id: window.window.wl_surface().clone(),
        })
        // TODO popup cleanup
    }

    fn configure(
        &mut self,
        _conn: &sctk::reexports::client::Connection,
        _qh: &sctk::reexports::client::QueueHandle<Self>,
        window: &sctk::shell::xdg::window::Window,
        configure: sctk::shell::xdg::window::WindowConfigure,
        _serial: u32,
    ) {
        let window = match self
            .windows
            .iter_mut()
            .find(|w| w.window.wl_surface() == window.wl_surface())
        {
            Some(w) => w,
            None => return,
        };

        if window.last_configure.as_ref().map(|c| c.state)
            != Some(configure.state)
        {
            self.sctk_events.push(SctkEvent::WindowEvent {
                variant: WindowEventVariant::StateChanged(configure.state),
                id: window.window.wl_surface().clone(),
            });
        }
        if window.last_configure.as_ref().map(|c| c.capabilities)
            != Some(configure.capabilities)
        {
            self.sctk_events.push(SctkEvent::WindowEvent {
                variant: WindowEventVariant::WmCapabilities(
                    configure.capabilities,
                ),
                id: window.window.wl_surface().clone(),
            });
        }

        window.update_size(configure.new_size);

        let wl_surface = window.window.wl_surface();
        let id = wl_surface.clone();
        let first = window.last_configure.is_none();
        window.last_configure.replace(configure.clone());

        self.sctk_events.push(SctkEvent::WindowEvent {
            variant: WindowEventVariant::Configure(
                window.current_size,
                configure,
                wl_surface.clone(),
                first,
            ),
            id,
        });
        self.frame_events.push((wl_surface.clone(), 0));
    }
}

delegate_xdg_window!(@<T: 'static + Debug> SctkState<T>);
delegate_xdg_shell!(@<T: 'static + Debug> SctkState<T>);
