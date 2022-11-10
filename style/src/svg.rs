//! Change the appearance of svg.

use iced_core::Color;

/// appearance
#[derive(Debug, Default, Clone, Copy)]
pub struct Appearance {
    /// fill
    pub fill: Option<Color>,
}

/// svf stylesheet
pub trait StyleSheet {
    /// style
    type Style: Default + Copy;

    /// appearance
    fn appearance(&self, style: Self::Style) -> Appearance;
}
