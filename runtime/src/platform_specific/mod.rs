//! Platform specific actions defined for wayland

use std::fmt;

#[cfg(wayland_platform)]
/// Platform specific actions defined for wayland
pub mod wayland;

/// Platform specific actions defined for wayland
pub enum Action {
    /// Wayland Specific Actions
    #[cfg(wayland_platform)]
    Wayland(wayland::Action),
}

impl fmt::Debug for Action {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(wayland_platform)]
            Action::Wayland(action) => action.fmt(_f),
            #[cfg(not(wayland_platform))]
            _ => Ok(()),
        }
    }
}
