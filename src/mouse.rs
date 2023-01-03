//! Listen and react to mouse events.
#[cfg(any(feature = "winit", feature = "wayland"))]
pub use crate::runtime::mouse::{Button, Event, Interaction, ScrollDelta};
