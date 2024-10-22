//! Sliders let users set a value by moving an indicator.
//!
//! # Example
//! ```no_run
//! # mod iced { pub mod widget { pub use iced_widget::*; } pub use iced_widget::Renderer; pub use iced_widget::core::*; }
//! # pub type Element<'a, Message> = iced_widget::core::Element<'a, Message, iced_widget::Theme, iced_widget::Renderer>;
//! #
//! use iced::widget::slider;
//!
//! struct State {
//!    value: f32,
//! }
//!
//! #[derive(Debug, Clone)]
//! enum Message {
//!     ValueChanged(f32),
//! }
//!
//! fn view(state: &State) -> Element<'_, Message> {
//!     slider(0.0..=100.0, state.value, Message::ValueChanged).into()
//! }
//!
//! fn update(state: &mut State, message: Message) {
//!     match message {
//!         Message::ValueChanged(value) => {
//!             state.value = value;
//!         }
//!     }
//! }
//! ```
use crate::core::border::{self, Border};
use crate::core::event::{self, Event};
use crate::core::keyboard;
use crate::core::keyboard::key::{self, Key};
use crate::core::layout;
use crate::core::mouse;
use crate::core::renderer;
use crate::core::touch;
use crate::core::widget::tree::{self, Tree};
use crate::core::widget::Id;
use crate::core::{
    self, Background, Clipboard, Color, Element, Layout, Length, Pixels, Point,
    Rectangle, Shell, Size, Theme, Widget,
};

use std::ops::RangeInclusive;

use iced_renderer::core::border::Radius;
use iced_runtime::core::gradient::Linear;

#[cfg(feature = "a11y")]
use std::borrow::Cow;

/// An horizontal bar and a handle that selects a single value from a range of
/// values.
///
/// A [`Slider`] will try to fill the horizontal space of its container.
///
/// The [`Slider`] range of numeric values is generic and its step size defaults
/// to 1 unit.
///
/// # Example
/// ```no_run
/// # mod iced { pub mod widget { pub use iced_widget::*; } pub use iced_widget::Renderer; pub use iced_widget::core::*; }
/// # pub type Element<'a, Message> = iced_widget::core::Element<'a, Message, iced_widget::Theme, iced_widget::Renderer>;
/// #
/// use iced::widget::slider;
///
/// struct State {
///    value: f32,
/// }
///
/// #[derive(Debug, Clone)]
/// enum Message {
///     ValueChanged(f32),
/// }
///
/// fn view(state: &State) -> Element<'_, Message> {
///     slider(0.0..=100.0, state.value, Message::ValueChanged).into()
/// }
///
/// fn update(state: &mut State, message: Message) {
///     match message {
///         Message::ValueChanged(value) => {
///             state.value = value;
///         }
///     }
/// }
/// ```
#[allow(missing_debug_implementations)]
pub struct Slider<'a, T, Message, Theme = crate::Theme>
where
    Theme: Catalog,
{
    id: Id,
    #[cfg(feature = "a11y")]
    name: Option<Cow<'a, str>>,
    #[cfg(feature = "a11y")]
    description: Option<iced_accessibility::Description<'a>>,
    #[cfg(feature = "a11y")]
    label: Option<Vec<iced_accessibility::accesskit::NodeId>>,
    range: RangeInclusive<T>,
    step: T,
    shift_step: Option<T>,
    value: T,
    default: Option<T>,
    breakpoints: &'a [T],
    on_change: Box<dyn Fn(T) -> Message + 'a>,
    on_release: Option<Message>,
    width: Length,
    height: f32,
    class: Theme::Class<'a>,
}

impl<'a, T, Message, Theme> Slider<'a, T, Message, Theme>
where
    T: Copy + From<u8> + PartialOrd,
    Message: Clone,
    Theme: Catalog,
{
    /// The default height of a [`Slider`].
    pub const DEFAULT_HEIGHT: f32 = 16.0;

    /// Creates a new [`Slider`].
    ///
    /// It expects:
    ///   * an inclusive range of possible values
    ///   * the current value of the [`Slider`]
    ///   * a function that will be called when the [`Slider`] is dragged.
    ///     It receives the new value of the [`Slider`] and must produce a
    ///     `Message`.
    pub fn new<F>(range: RangeInclusive<T>, value: T, on_change: F) -> Self
    where
        F: 'a + Fn(T) -> Message,
    {
        let value = if value >= *range.start() {
            value
        } else {
            *range.start()
        };

        let value = if value <= *range.end() {
            value
        } else {
            *range.end()
        };

        Slider {
            id: Id::unique(),
            #[cfg(feature = "a11y")]
            name: None,
            #[cfg(feature = "a11y")]
            description: None,
            #[cfg(feature = "a11y")]
            label: None,
            value,
            default: None,
            range,
            step: T::from(1),
            shift_step: None,
            breakpoints: &[],
            on_change: Box::new(on_change),
            on_release: None,
            width: Length::Fill,
            height: Self::DEFAULT_HEIGHT,
            class: Theme::default(),
        }
    }

    /// Sets the optional default value for the [`Slider`].
    ///
    /// If set, the [`Slider`] will reset to this value when ctrl-clicked or command-clicked.
    pub fn default(mut self, default: impl Into<T>) -> Self {
        self.default = Some(default.into());
        self
    }

    /// Defines breakpoints to visibly mark on the slider.
    ///
    /// The slider will gravitate towards a breakpoint when near it.
    pub fn breakpoints(mut self, breakpoints: &'a [T]) -> Self {
        self.breakpoints = breakpoints;
        self
    }

    /// Sets the release message of the [`Slider`].
    /// This is called when the mouse is released from the slider.
    ///
    /// Typically, the user's interaction with the slider is finished when this message is produced.
    /// This is useful if you need to spawn a long-running task from the slider's result, where
    /// the default `on_change` message could create too many events.
    pub fn on_release(mut self, on_release: Message) -> Self {
        self.on_release = Some(on_release);
        self
    }

    /// Sets the width of the [`Slider`].
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the height of the [`Slider`].
    pub fn height(mut self, height: impl Into<Pixels>) -> Self {
        self.height = height.into().0;
        self
    }

    /// Sets the step size of the [`Slider`].
    pub fn step(mut self, step: impl Into<T>) -> Self {
        self.step = step.into();
        self
    }

    /// Sets the optional "shift" step for the [`Slider`].
    ///
    /// If set, this value is used as the step while the shift key is pressed.
    pub fn shift_step(mut self, shift_step: impl Into<T>) -> Self {
        self.shift_step = Some(shift_step.into());
        self
    }

    /// Sets the style of the [`Slider`].
    #[must_use]
    pub fn style(mut self, style: impl Fn(&Theme, Status) -> Style + 'a) -> Self
    where
        Theme::Class<'a>: From<StyleFn<'a, Theme>>,
    {
        self.class = (Box::new(style) as StyleFn<'a, Theme>).into();
        self
    }

    /// Sets the style class of the [`Slider`].
    #[cfg(feature = "advanced")]
    #[must_use]
    pub fn class(mut self, class: impl Into<Theme::Class<'a>>) -> Self {
        self.class = class.into();
        self
    }

    #[cfg(feature = "a11y")]
    /// Sets the name of the [`Slider`].
    pub fn name(mut self, name: impl Into<Cow<'a, str>>) -> Self {
        self.name = Some(name.into());
        self
    }

    #[cfg(feature = "a11y")]
    /// Sets the description of the [`Slider`].
    pub fn description_widget(
        mut self,
        description: &impl iced_accessibility::Describes,
    ) -> Self {
        self.description = Some(iced_accessibility::Description::Id(
            description.description(),
        ));
        self
    }

    #[cfg(feature = "a11y")]
    /// Sets the description of the [`Slider`].
    pub fn description(mut self, description: impl Into<Cow<'a, str>>) -> Self {
        self.description =
            Some(iced_accessibility::Description::Text(description.into()));
        self
    }

    #[cfg(feature = "a11y")]
    /// Sets the label of the [`Slider`].
    pub fn label(mut self, label: &dyn iced_accessibility::Labels) -> Self {
        self.label =
            Some(label.label().into_iter().map(|l| l.into()).collect());
        self
    }
}

impl<'a, T, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Slider<'a, T, Message, Theme>
where
    T: Copy + Into<f64> + num_traits::FromPrimitive,
    Message: Clone,
    Theme: Catalog,
    Renderer: core::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: Length::Shrink,
        }
    }

    fn layout(
        &self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::atomic(limits, self.width, self.height)
    }

    fn on_event(
        &mut self,
        tree: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) -> event::Status {
        let state = tree.state.downcast_mut::<State>();

        let is_dragging = state.is_dragging;
        let current_value = self.value;

        let locate = |cursor_position: Point| -> Option<T> {
            let bounds = layout.bounds();
            let new_value = if cursor_position.x <= bounds.x {
                Some(*self.range.start())
            } else if cursor_position.x >= bounds.x + bounds.width {
                Some(*self.range.end())
            } else {
                let step = if state.keyboard_modifiers.shift() {
                    self.shift_step.unwrap_or(self.step)
                } else {
                    self.step
                }
                .into();

                let start = (*self.range.start()).into();
                let end = (*self.range.end()).into();

                let percent = f64::from(cursor_position.x - bounds.x)
                    / f64::from(bounds.width);

                let steps = (percent * (end - start) / step).round();
                let value = steps * step + start;

                T::from_f64(value.min(end))
            };

            new_value
        };

        let increment = |value: T| -> Option<T> {
            let step = if state.keyboard_modifiers.shift() {
                self.shift_step.unwrap_or(self.step)
            } else {
                self.step
            }
            .into();

            let steps = (value.into() / step).round();
            let new_value = step * (steps + 1.0);

            if new_value > (*self.range.end()).into() {
                return Some(*self.range.end());
            }

            T::from_f64(new_value)
        };

        let decrement = |value: T| -> Option<T> {
            let step = if state.keyboard_modifiers.shift() {
                self.shift_step.unwrap_or(self.step)
            } else {
                self.step
            }
            .into();

            let steps = (value.into() / step).round();
            let new_value = step * (steps - 1.0);

            if new_value < (*self.range.start()).into() {
                return Some(*self.range.start());
            }

            T::from_f64(new_value)
        };

        let change = |new_value: T| {
            if (self.value.into() - new_value.into()).abs() > f64::EPSILON {
                shell.publish((self.on_change)(new_value));

                self.value = new_value;
            }
        };

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerPressed { .. }) => {
                if let Some(cursor_position) =
                    cursor.position_over(layout.bounds())
                {
                    if state.keyboard_modifiers.command() {
                        let _ = self.default.map(change);
                        state.is_dragging = false;
                    } else {
                        let _ = locate(cursor_position).map(change);
                        state.is_dragging = true;
                    }

                    return event::Status::Captured;
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerLifted { .. })
            | Event::Touch(touch::Event::FingerLost { .. }) => {
                if is_dragging {
                    if let Some(on_release) = self.on_release.clone() {
                        shell.publish(on_release);
                    }
                    state.is_dragging = false;

                    return event::Status::Captured;
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. })
            | Event::Touch(touch::Event::FingerMoved { .. }) => {
                if is_dragging {
                    let _ = cursor.position().and_then(locate).map(change);

                    return event::Status::Captured;
                }
            }
            Event::Mouse(mouse::Event::WheelScrolled { delta })
                if state.keyboard_modifiers.control() =>
            {
                if cursor.is_over(layout.bounds()) {
                    let delta = match delta {
                        mouse::ScrollDelta::Lines { x: _, y } => y,
                        mouse::ScrollDelta::Pixels { x: _, y } => y,
                    };

                    if delta < 0.0 {
                        let _ = decrement(current_value).map(change);
                    } else {
                        let _ = increment(current_value).map(change);
                    }

                    return event::Status::Captured;
                }
            }
            Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) => {
                if cursor.is_over(layout.bounds()) {
                    match key {
                        Key::Named(key::Named::ArrowUp) => {
                            let _ = increment(current_value).map(change);
                        }
                        Key::Named(key::Named::ArrowDown) => {
                            let _ = decrement(current_value).map(change);
                        }
                        _ => (),
                    }

                    return event::Status::Captured;
                }
            }
            Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                state.keyboard_modifiers = modifiers;
            }
            _ => {}
        }

        event::Status::Ignored
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();
        let bounds = layout.bounds();
        let is_mouse_over = cursor.is_over(bounds);

        let style = theme.style(
            &self.class,
            if state.is_dragging {
                Status::Dragged
            } else if is_mouse_over {
                Status::Hovered
            } else {
                Status::Active
            },
        );

        let border_width = style
            .handle
            .border_width
            .min(bounds.height / 2.0)
            .min(bounds.width / 2.0);

        let (handle_width, handle_height, handle_border_radius) =
            match style.handle.shape {
                HandleShape::Circle { radius } => {
                    let radius = (radius)
                        .max(2.0 * border_width)
                        .min(bounds.height / 2.0)
                        .min(bounds.width / 2.0 + 2.0 * border_width);
                    (radius * 2.0, radius * 2.0, Radius::from(radius))
                }
                HandleShape::Rectangle {
                    height,
                    width,
                    border_radius,
                } => {
                    let width = (f32::from(width)).max(2.0 * border_width);
                    let height = (f32::from(height)).max(2.0 * border_width);
                    let mut border_radius: [f32; 4] = border_radius.into();
                    for r in &mut border_radius {
                        *r = (*r)
                            .min(height / 2.0)
                            .min(width / 2.0)
                            .max(*r * (width + border_width * 2.0) / width);
                    }

                    (
                        width,
                        height,
                        Radius {
                            top_left: border_radius[0],
                            top_right: border_radius[1],
                            bottom_right: border_radius[2],
                            bottom_left: border_radius[3],
                        },
                    )
                }
            };

        let value = self.value.into() as f32;
        let (range_start, range_end) = {
            let (start, end) = self.range.clone().into_inner();

            (start.into() as f32, end.into() as f32)
        };

        let offset = if range_start >= range_end {
            0.0
        } else {
            (bounds.width - handle_width) * (value - range_start)
                / (range_end - range_start)
        };

        let rail_y = bounds.y + bounds.height / 2.0;

        // Draw the breakpoint indicators beneath the slider.
        const BREAKPOINT_WIDTH: f32 = 2.0;
        for &value in self.breakpoints {
            let value: f64 = value.into();
            let offset = if range_start >= range_end {
                0.0
            } else {
                (bounds.width - BREAKPOINT_WIDTH) * (value as f32 - range_start)
                    / (range_end - range_start)
            };

            renderer.fill_quad(
                renderer::Quad {
                    bounds: Rectangle {
                        x: bounds.x + offset,
                        y: rail_y + 6.0,
                        width: BREAKPOINT_WIDTH,
                        height: 8.0,
                    },
                    border: Border {
                        radius: 0.0.into(),
                        width: 0.0,
                        color: Color::TRANSPARENT,
                    },
                    ..renderer::Quad::default()
                },
                crate::core::Background::Color(style.breakpoint.color),
            );
        }

        renderer.fill_quad(
            renderer::Quad {
                bounds: Rectangle {
                    x: bounds.x,
                    y: rail_y - style.rail.width / 2.0,
                    width: offset + handle_width / 2.0,
                    height: style.rail.width,
                },
                border: style.rail.border,
                ..renderer::Quad::default()
            },
            style.rail.backgrounds.0,
        );

        // TODO align the angle of the gradient for the slider?

        renderer.fill_quad(
            renderer::Quad {
                bounds: Rectangle {
                    x: bounds.x + offset + handle_width / 2.0,
                    y: rail_y - style.rail.width / 2.0,
                    width: bounds.width - offset - handle_width / 2.0,
                    height: style.rail.width,
                },
                border: style.rail.border,
                ..renderer::Quad::default()
            },
            style.rail.backgrounds.1,
        );

        // handle
        renderer.fill_quad(
            renderer::Quad {
                bounds: Rectangle {
                    x: bounds.x + offset,
                    y: rail_y - (handle_height / 2.0),
                    width: handle_width,
                    height: handle_height,
                },
                border: Border {
                    radius: handle_border_radius,
                    width: style.handle.border_width,
                    color: style.handle.border_color,
                },
                ..renderer::Quad::default()
            },
            style.handle.background,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<State>();
        let bounds = layout.bounds();
        let is_mouse_over = cursor.is_over(bounds);

        if state.is_dragging {
            mouse::Interaction::Grabbing
        } else if is_mouse_over {
            mouse::Interaction::Grab
        } else {
            mouse::Interaction::default()
        }
    }

    #[cfg(feature = "a11y")]
    fn a11y_nodes(
        &self,
        layout: Layout<'_>,
        _state: &Tree,
        cursor: mouse::Cursor,
    ) -> iced_accessibility::A11yTree {
        use iced_accessibility::{
            accesskit::{NodeBuilder, NodeId, Rect, Role},
            A11yTree,
        };

        let bounds = layout.bounds();
        let is_hovered = cursor.is_over(bounds);
        let Rectangle {
            x,
            y,
            width,
            height,
        } = bounds;
        let bounds = Rect::new(
            x as f64,
            y as f64,
            (x + width) as f64,
            (y + height) as f64,
        );
        let mut node = NodeBuilder::new(Role::Slider);
        node.set_bounds(bounds);
        if let Some(name) = self.name.as_ref() {
            node.set_name(name.clone());
        }
        match self.description.as_ref() {
            Some(iced_accessibility::Description::Id(id)) => {
                node.set_described_by(
                    id.iter()
                        .cloned()
                        .map(|id| NodeId::from(id))
                        .collect::<Vec<_>>(),
                );
            }
            Some(iced_accessibility::Description::Text(text)) => {
                node.set_description(text.clone());
            }
            None => {}
        }

        if is_hovered {
            node.set_hovered();
        }

        if let Some(label) = self.label.as_ref() {
            node.set_labelled_by(label.clone());
        }

        if let Ok(min) = self.range.start().clone().try_into() {
            node.set_min_numeric_value(min);
        }
        if let Ok(max) = self.range.end().clone().try_into() {
            node.set_max_numeric_value(max);
        }
        if let Ok(value) = self.value.clone().try_into() {
            node.set_numeric_value(value);
        }
        if let Ok(step) = self.step.clone().try_into() {
            node.set_numeric_value_step(step);
        }

        // TODO: This could be a setting on the slider
        node.set_live(iced_accessibility::accesskit::Live::Polite);

        A11yTree::leaf(node, self.id.clone())
    }

    fn id(&self) -> Option<Id> {
        Some(self.id.clone())
    }

    fn set_id(&mut self, id: Id) {
        self.id = id;
    }
}

impl<'a, T, Message, Theme, Renderer> From<Slider<'a, T, Message, Theme>>
    for Element<'a, Message, Theme, Renderer>
where
    T: Copy + Into<f64> + num_traits::FromPrimitive + 'a,
    Message: Clone + 'a,
    Theme: Catalog + 'a,
    Renderer: core::Renderer + 'a,
{
    fn from(
        slider: Slider<'a, T, Message, Theme>,
    ) -> Element<'a, Message, Theme, Renderer> {
        Element::new(slider)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct State {
    is_dragging: bool,
    keyboard_modifiers: keyboard::Modifiers,
}

/// The possible status of a [`Slider`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// The [`Slider`] can be interacted with.
    Active,
    /// The [`Slider`] is being hovered.
    Hovered,
    /// The [`Slider`] is being dragged.
    Dragged,
}

/// The appearance of a slider.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Style {
    /// The colors of the rail of the slider.
    pub rail: Rail,
    /// The appearance of the [`Handle`] of the slider.
    pub handle: Handle,
    /// The appearance of breakpoints.
    pub breakpoint: Breakpoint,
}

/// The appearance of slider breakpoints.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Breakpoint {
    /// The color of the slider breakpoint.
    pub color: Color,
}

impl Style {
    /// Changes the [`HandleShape`] of the [`Style`] to a circle
    /// with the given radius.
    pub fn with_circular_handle(mut self, radius: impl Into<Pixels>) -> Self {
        self.handle.shape = HandleShape::Circle {
            radius: radius.into().0,
        };
        self
    }
}

/// The appearance of a slider rail
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rail {
    /// The backgrounds of the rail of the slider.
    pub backgrounds: (Background, Background),
    /// The width of the stroke of a slider rail.
    pub width: f32,
    /// The border of the rail.
    pub border: Border,
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
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Handle {
    /// The shape of the handle.
    pub shape: HandleShape,
    /// The [`Background`] of the handle.
    pub background: Background,
    /// The border width of the handle.
    pub border_width: f32,
    /// The border [`Color`] of the handle.
    pub border_color: Color,
}

/// The shape of the handle of a slider.
#[derive(Debug, Clone, Copy, PartialEq)]
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
        /// The height of the rectangle.
        height: u16,
        /// The border radius of the corners of the rectangle.
        border_radius: border::Radius,
    },
}

/// The theme catalog of a [`Slider`].
pub trait Catalog: Sized {
    /// The item class of the [`Catalog`].
    type Class<'a>;

    /// The default class produced by the [`Catalog`].
    fn default<'a>() -> Self::Class<'a>;

    /// The [`Style`] of a class with the given status.
    fn style(&self, class: &Self::Class<'_>, status: Status) -> Style;
}

/// A styling function for a [`Slider`].
pub type StyleFn<'a, Theme> = Box<dyn Fn(&Theme, Status) -> Style + 'a>;

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(default)
    }

    fn style(&self, class: &Self::Class<'_>, status: Status) -> Style {
        class(self, status)
    }
}

/// The default style of a [`Slider`].
pub fn default(theme: &Theme, status: Status) -> Style {
    let palette = theme.extended_palette();

    let color = match status {
        Status::Active => palette.primary.strong.color,
        Status::Hovered => palette.primary.base.color,
        Status::Dragged => palette.primary.strong.color,
    };

    Style {
        rail: Rail {
            backgrounds: (color.into(), palette.secondary.base.color.into()),
            width: 4.0,
            border: Border {
                radius: 2.0.into(),
                width: 0.0,
                color: Color::TRANSPARENT,
            },
        },
        handle: Handle {
            shape: HandleShape::Circle { radius: 7.0 },
            background: color.into(),
            border_color: Color::TRANSPARENT,
            border_width: 0.0,
        },
        breakpoint: Breakpoint {
            color: palette.background.weak.text,
        },
    }
}
