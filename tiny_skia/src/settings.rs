use crate::core::{Font, Pixels};

/// The settings of a [`Backend`].
///
/// [`Backend`]: crate::Backend
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    /// The default [`Font`] to use.
    pub default_font: Font,

    /// The default size of text.
    ///
    /// By default, it will be set to `16.0`.
    pub default_text_size: Pixels,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            default_font: Font::default(),
            default_text_size: Pixels(14.0),
        }
    }
}
