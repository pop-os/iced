//! Display fields that can be filled with text.
//!
//! A [`TextInput`] has some local [`State`].
pub(crate) mod editor;
pub(crate) mod value;

pub mod cursor;

mod text_input;
pub use text_input::*;
