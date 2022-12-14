mod layer;
mod output;
mod popup;
mod window;
mod seat;

use crate::window::Id;
use sctk::reexports::client::protocol::{wl_output::WlOutput, wl_surface::WlSurface, wl_seat::WlSeat};

pub use layer::*;
pub use output::*;
pub use popup::*;
pub use window::*;
pub use seat::*;

/// wayland events
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// layer surface event
    Layer(LayerEvent, WlSurface, Id),
    /// popup event
    Popup(PopupEvent, WlSurface, Id),
    /// output event
    Output(OutputEvent, WlOutput),
    /// window event
    Window(WindowEvent, WlSurface, Id),
    /// Seat Event
    Seat(SeatEvent, WlSeat),
}
