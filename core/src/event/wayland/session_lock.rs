use crate::window::Id;
use sctk::reexports::client::protocol::wl_surface::WlSurface;

/// session lock events
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionLockEvent {
    /// Compositor has activated lock
    Locked,
    /// Lock rejected / canceled by compositor
    Finished,
    /// Session lock protocol not supported
    NotSupported,
    /// Session lock surface focused
    Focused(WlSurface, Id),
    /// Session lock surface unfocused
    Unfocused(WlSurface, Id),
    /// Session unlock has been processed by server
    Unlocked,
}
