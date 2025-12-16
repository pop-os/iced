//! Change the apperance of a button.
use iced_core::{Background, Border, Color};
use iced_style::Theme;

/// The appearance of a button.
#[derive(Debug, Clone, Copy)]
pub struct Appearance {
    /// The [`Background`] of the button.
    pub background: Option<Background>,
    /// The border of the button.
    pub border: Border,
    /// The icon [`Color`] of the button.
    pub icon_color: Option<Color>,
    /// The text [`Color`] of the button.
    pub text_color: Color,
}

impl std::default::Default for Appearance {
    fn default() -> Self {
        Self {
            background: None,
            border: Border::default(),
            icon_color: None,
            text_color: Color::WHITE,
        }
    }
}

/// A set of rules that dictate the style of a button.
pub trait StyleSheet {
    /// The supported style of the [`StyleSheet`].
    type Style: Default;

    /// Produces the active [`Appearance`] of a button.
    fn default(&self, style: &Self::Style) -> Appearance;

    /// Produces the selected [`Appearance`] of a button.
    fn selected(&self, style: &Self::Style) -> Appearance;
}

/// The style of a button.
#[derive(Default)]
pub enum SelectionField {
    /// The primary style.
    #[default]
    Default,
    /// A custom style.
    Custom(Box<dyn StyleSheet<Style = Theme>>),
}

impl SelectionField {
    /// Creates a custom [`Button`] style variant.
    pub fn custom(
        style_sheet: impl StyleSheet<Style = Theme> + 'static,
    ) -> Self {
        Self::Custom(Box::new(style_sheet))
    }
}

impl StyleSheet for Theme {
    type Style = SelectionField;

    fn default(&self, _style: &Self::Style) -> Appearance {
        Appearance::default()
    }

    fn selected(&self, _style: &Self::Style) -> Appearance {
        Appearance {
            background: Some(Background::Color(Color::from_rgba(
                0.0, 0.07, 0.42, 1.0,
            ))),
            border: Border {
                color: Color::WHITE,
                width: 1.0,
                radius: 5.5.into(),
            },
            icon_color: None,
            text_color: Color::WHITE,
        }
    }
}
