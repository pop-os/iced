#![allow(missing_docs)]

use sctk::reexports::csd_frame::{WindowManagerCapabilities, WindowState};

/// window events
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowEvent {
    /// window manager capabilities
    WmCapabilities(WindowManagerCapabilities),
    /// window state
    State(WindowState),
}
