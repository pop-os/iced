use sctk::reexports::client::protocol::wl_seat::WlSeat;

/// seat events
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeatEvent {
    /// A new seat is interacting with the application
    Enter(WlSeat),
    /// A seat is not interacting with the application anymore
    Leave(WlSeat),
}