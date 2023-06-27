#![allow(missing_docs)]

use sctk::shell::xdg::window::{WindowManagerCapabilities, WindowState};

/// window events
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowEvent {
    /// window manager capabilities
    WmCapabilities(WindowManagerCapabilities),
    /// window state
    State(WindowState),
}
