//! Change the apperance of a slider.
use iced_core::{gradient::Linear, BorderRadius, Color};

/// The appearance of a slider.
#[derive(Debug, Clone, Copy)]
pub struct Appearance {
    /// The colors of the rail of the slider.
    pub rail: Rail,
    /// The appearance of the [`Handle`] of the slider.
    pub handle: Handle,
}

/// The appearance of a slider rail
#[derive(Debug, Clone, Copy)]
pub struct Rail {
    /// The colors of the rail of the slider.
    pub colors: RailBackground,
    /// The width of the stroke of a slider rail.
    pub width: f32,
    /// The border radius of the corners of the rail.
    pub border_radius: BorderRadius,
}

/// The background color of the rail
#[derive(Debug, Clone, Copy)]
pub enum RailBackground {
    /// Start and end colors of the rail
    Pair(Color, Color),
    /// Linear gradient for the background of the rail
    /// includes an option for auto-selecting the angle
    Gradient {
        /// the linear gradient of the slider
        gradient: Linear,
        /// Let the widget determin the angle of the gradient
        auto_angle: bool,
    },
}

/// The appearance of the handle of a slider.
#[derive(Debug, Clone, Copy)]
pub struct Handle {
    /// The shape of the handle.
    pub shape: HandleShape,
    /// The [`Color`] of the handle.
    pub color: Color,
    /// The border width of the handle.
    pub border_width: f32,
    /// The border [`Color`] of the handle.
    pub border_color: Color,
}

/// The shape of the handle of a slider.
#[derive(Debug, Clone, Copy)]
pub enum HandleShape {
    /// A circular handle.
    Circle {
        /// The radius of the circle.
        radius: f32,
    },
    /// A rectangular shape.
    Rectangle {
        /// The width of the rectangle.
        width: u16,
        /// The border radius of the corners of the rectangle.
        border_radius: BorderRadius,
    },
}

/// A set of rules that dictate the style of a slider.
pub trait StyleSheet {
    /// The supported style of the [`StyleSheet`].
    type Style: Default;

    /// Produces the style of an active slider.
    fn active(&self, style: &Self::Style) -> Appearance;

    /// Produces the style of an hovered slider.
    fn hovered(&self, style: &Self::Style) -> Appearance;

    /// Produces the style of a slider that is being dragged.
    fn dragging(&self, style: &Self::Style) -> Appearance;
}
