//! Platform specific actions defined for wayland

use std::{fmt, marker::PhantomData};

use iced_futures::MaybeSend;

#[cfg(feature = "wayland")]
/// Platform specific actions defined for wayland
pub mod wayland;

/// Platform specific actions defined for wayland
pub enum Action {
    /// Wayland Specific Actions
    #[cfg(feature = "wayland")]
    Wayland(wayland::Action),
}

impl fmt::Debug for Action {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(feature = "wayland")]
            Action::Wayland(action) => action.fmt(_f),
            _ => Ok(()),
        }
    }
}
