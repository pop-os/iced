#![allow(missing_docs)]

use cctk::sctk::reexports::csd_frame::WindowState;

/// window events
#[derive(Debug, PartialEq, Clone)]
pub enum WindowEvent {
    /// Window suggested bounds.
    SuggestedBounds(Option<crate::Size>),
    /// Window state
    WindowState(WindowState),
}
