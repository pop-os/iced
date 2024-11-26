mod layer;
mod output;
mod overlap_notify;
mod popup;
mod seat;
mod session_lock;
mod window;

use crate::{time::Instant, window::Id};
use cctk::sctk::reexports::client::protocol::{
    wl_output::WlOutput, wl_seat::WlSeat, wl_surface::WlSurface,
};

pub use layer::*;
pub use output::*;
pub use overlap_notify::*;
pub use popup::*;
pub use seat::*;
pub use session_lock::*;
pub use window::*;

/// wayland events
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// layer surface event
    Layer(LayerEvent, WlSurface, Id),
    /// popup event
    Popup(PopupEvent, WlSurface, Id),
    /// output event
    Output(OutputEvent, WlOutput),
    /// Overlap notify event
    OverlapNotify(overlap_notify::OverlapNotifyEvent),
    /// window event
    Window(WindowEvent),
    /// Seat Event
    Seat(SeatEvent, WlSeat),
    /// Session lock events
    SessionLock(SessionLockEvent),
    /// Frame events
    Frame(Instant, WlSurface, Id),
    /// Request Resize
    RequestResize,
}
