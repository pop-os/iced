#![allow(missing_docs)]

use sctk::{
    reexports::csd_frame::{WindowManagerCapabilities, WindowState},
    shell::xdg::window::WindowConfigure,
};

/// window events
#[derive(Debug, Clone)]
pub enum WindowEvent {
    /// Window manager capabilities.
    WmCapabilities(WindowManagerCapabilities),
    /// Window state.
    State(WindowState),
    /// Window configure event.
    Configure(WindowConfigure),
}

impl PartialEq for WindowEvent {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::WmCapabilities(a), Self::WmCapabilities(b)) => a == b,
            (Self::State(a), Self::State(b)) => a == b,
            (Self::Configure(a), Self::Configure(b)) => {
                a.capabilities == b.capabilities
                    && a.state == b.state
                    && a.decoration_mode == b.decoration_mode
                    && a.new_size == b.new_size
                    && a.suggested_bounds == b.suggested_bounds
            }
            _ => false,
        }
    }
}
