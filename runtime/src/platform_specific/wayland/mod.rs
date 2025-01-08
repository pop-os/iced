//! Wayland specific actions

use std::fmt::Debug;

use cctk::{sctk::output::OutputState, wayland_client::protocol::wl_output::WlOutput};
use iced_core::window::Id;

use crate::oneshot;

/// activation Actions
pub mod activation;

/// layer surface actions
pub mod layer_surface;
/// popup actions
pub mod popup;
/// session locks
pub mod session_lock;

/// Platform specific actions defined for wayland
pub enum Action {
    /// LayerSurface Actions
    LayerSurface(layer_surface::Action),
    /// popup
    Popup(popup::Action),
    /// activation
    Activation(activation::Action),
    /// session lock
    SessionLock(session_lock::Action),
    /// Overlap Notify
    OverlapNotify(Id, bool),
    // WlOutput getter
    GetOutput {
        f: Box<dyn Fn(&OutputState) -> Option<WlOutput> + Send + Sync>,
        channel: oneshot::Sender<Option<WlOutput>>,
    },
}

impl Debug for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Action::LayerSurface(arg0) => {
                f.debug_tuple("LayerSurface").field(arg0).finish()
            }
            Action::Popup(arg0) => f.debug_tuple("Popup").field(arg0).finish(),
            Action::Activation(arg0) => {
                f.debug_tuple("Activation").field(arg0).finish()
            }
            Action::SessionLock(arg0) => {
                f.debug_tuple("SessionLock").field(arg0).finish()
            }
            Action::OverlapNotify(id, _) => {
                f.debug_tuple("OverlapNotify").field(id).finish()
            }
            Action::GetOutput {..} => {
                f.debug_tuple("GetOutput").finish()
            }
        }
    }
}
