//! Access the clipboard.
#[cfg(any(feature = "winit"))]
// TODO support in wayland
pub use crate::runtime::clipboard::{read, write};
