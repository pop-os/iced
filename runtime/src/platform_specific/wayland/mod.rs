//! Wayland specific actions

use std::fmt::Debug;

use iced_core::MaybeSend;

/// activation Actions
pub mod activation;
/// data device Actions
pub mod data_device;
/// layer surface actions
pub mod layer_surface;
/// popup actions
pub mod popup;
/// session locks
pub mod session_lock;
/// window actions
pub mod window;

/// Platform specific actions defined for wayland
pub enum Action {
    /// LayerSurface Actions
    LayerSurface(layer_surface::Action),
    /// Window Actions
    Window(window::Action),
    /// popup
    Popup(popup::Action),
    /// data device
    DataDevice(data_device::Action),
    /// activation
    Activation(activation::Action),
    /// session lock
    SessionLock(session_lock::Action),
}

impl Debug for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LayerSurface(arg0) => {
                f.debug_tuple("LayerSurface").field(arg0).finish()
            }
            Self::Window(arg0) => f.debug_tuple("Window").field(arg0).finish(),
            Self::Popup(arg0) => f.debug_tuple("Popup").field(arg0).finish(),
            Self::DataDevice(arg0) => {
                f.debug_tuple("DataDevice").field(arg0).finish()
            }
            Self::Activation(arg0) => {
                f.debug_tuple("Activation").field(arg0).finish()
            }
            Self::SessionLock(arg0) => {
                f.debug_tuple("SessionLock").field(arg0).finish()
            }
        }
    }
}
