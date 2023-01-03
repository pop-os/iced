//! Listen and react to touch events.
#[cfg(any(feature = "winit", feature = "wayland"))]
pub use crate::runtime::touch::{Event, Finger};
