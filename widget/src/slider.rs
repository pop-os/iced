//! Display an interactive selector of a single value from a range of values.
//!
//! A [`Slider`] has some local [`State`].
use crate::core::event::{self, Event};
use crate::core::layout;
use crate::core::mouse;
use crate::core::renderer;
use crate::core::touch;
use crate::core::widget::tree::{self, Tree};
use crate::core::widget::Id;
use crate::core::{
    Border, Clipboard, Color, Element, Layout, Length, Pixels, Point,
    Rectangle, Shell, Size, Widget,
};

use std::ops::RangeInclusive;

use iced_renderer::core::{border::Radius, Degrees, Radians};
pub use iced_style::slider::{
    Appearance, Handle, HandleShape, Rail, RailBackground, StyleSheet,
};

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
/// # type Slider<'a, T, Message> =
/// #     iced_widget::Slider<'a, Message, T, iced_widget::style::Theme>;
/// #
/// #[derive(Clone)]
/// pub enum Message {
///     SliderChanged(f32),
/// }
///
/// let value = 50.0;
///
/// Slider::new(0.0..=100.0, value, Message::SliderChanged);
/// ```
///
/// ![Slider drawn by Coffee's renderer](https://github.com/hecrj/coffee/blob/bda9818f823dfcb8a7ad0ff4940b4d4b387b5208/images/ui/slider.png?raw=true)
#[allow(missing_debug_implementations)]
#[must_use]
pub struct Slider<'a, T, Message, Theme = crate::Theme>
where
    Theme: StyleSheet,
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
    value: T,
    breakpoints: &'a [T],
    on_change: Box<dyn Fn(T) -> Message + 'a>,
    on_release: Option<Message>,
    width: Length,
    height: f32,
    style: Theme::Style,
}

impl<'a, T, Message, Theme> Slider<'a, T, Message, Theme>
where
    T: Copy + From<u8> + std::cmp::PartialOrd,
    Message: Clone,
    Theme: StyleSheet,
{
    /// The default height of a [`Slider`].
    pub const DEFAULT_HEIGHT: f32 = 22.0;

    /// Creates a new [`Slider`].
    ///
    /// It expects:
    ///   * an inclusive range of possible values
    ///   * the current value of the [`Slider`]
    ///   * a function that will be called when the [`Slider`] is dragged.
    ///   It receives the new value of the [`Slider`] and must produce a
    ///   `Message`.
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
            range,
            step: T::from(1),
            breakpoints: &[],
            on_change: Box::new(on_change),
            on_release: None,
            width: Length::Fill,
            height: Self::DEFAULT_HEIGHT,
            style: Default::default(),
        }
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

    /// Sets the style of the [`Slider`].
    pub fn style(mut self, style: impl Into<Theme::Style>) -> Self {
        self.style = style.into();
        self
    }

    /// Sets the step size of the [`Slider`].
    pub fn step(mut self, step: impl Into<T>) -> Self {
        self.step = step.into();
        self
    }

    #[cfg(feature = "a11y")]
    /// Sets the name of the [`Button`].
    pub fn name(mut self, name: impl Into<Cow<'a, str>>) -> Self {
        self.name = Some(name.into());
        self
    }

    #[cfg(feature = "a11y")]
    /// Sets the description of the [`Button`].
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
    /// Sets the description of the [`Button`].
    pub fn description(mut self, description: impl Into<Cow<'a, str>>) -> Self {
        self.description =
            Some(iced_accessibility::Description::Text(description.into()));
        self
    }

    #[cfg(feature = "a11y")]
    /// Sets the label of the [`Button`].
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
    Theme: StyleSheet,
    Renderer: crate::core::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::new())
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
        update(
            event,
            layout,
            cursor,
            shell,
            tree.state.downcast_mut::<State>(),
            &mut self.value,
            &self.range,
            self.step,
            self.on_change.as_ref(),
            &self.on_release,
        )
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
        draw(
            renderer,
            layout,
            cursor,
            tree.state.downcast_ref::<State>(),
            self.value,
            &self.range,
            self.breakpoints,
            theme,
            &self.style,
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
        mouse_interaction(layout, cursor, tree.state.downcast_ref::<State>())
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
    Theme: StyleSheet + 'a,
    Renderer: crate::core::Renderer + 'a,
{
    fn from(
        slider: Slider<'a, T, Message, Theme>,
    ) -> Element<'a, Message, Theme, Renderer> {
        Element::new(slider)
    }
}

/// Processes an [`Event`] and updates the [`State`] of a [`Slider`]
/// accordingly.
pub fn update<Message, T>(
    event: Event,
    layout: Layout<'_>,
    cursor: mouse::Cursor,
    shell: &mut Shell<'_, Message>,
    state: &mut State,
    value: &mut T,
    range: &RangeInclusive<T>,
    step: T,
    on_change: &dyn Fn(T) -> Message,
    on_release: &Option<Message>,
) -> event::Status
where
    T: Copy + Into<f64> + num_traits::FromPrimitive,
    Message: Clone,
{
    let is_dragging = state.is_dragging;

    let mut change = |cursor_position: Point| {
        let bounds = layout.bounds();
        let new_value = if cursor_position.x <= bounds.x {
            *range.start()
        } else if cursor_position.x >= bounds.x + bounds.width {
            *range.end()
        } else {
            let step = step.into();
            let start = (*range.start()).into();
            let end = (*range.end()).into();

            let percent = f64::from(cursor_position.x - bounds.x)
                / f64::from(bounds.width);

            let steps = (percent * (end - start) / step).round();
            let value = steps * step + start;

            if let Some(value) = T::from_f64(value) {
                value
            } else {
                return;
            }
        };

        if ((*value).into() - new_value.into()).abs() > f64::EPSILON {
            shell.publish((on_change)(new_value));

            *value = new_value;
        }
    };

    match event {
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
        | Event::Touch(touch::Event::FingerPressed { .. }) => {
            if let Some(cursor_position) = cursor.position_over(layout.bounds())
            {
                change(cursor_position);
                state.is_dragging = true;

                return event::Status::Captured;
            }
        }
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
        | Event::Touch(touch::Event::FingerLifted { .. })
        | Event::Touch(touch::Event::FingerLost { .. }) => {
            if is_dragging {
                if let Some(on_release) = on_release.clone() {
                    shell.publish(on_release);
                }
                state.is_dragging = false;

                return event::Status::Captured;
            }
        }
        Event::Mouse(mouse::Event::CursorMoved { .. })
        | Event::Touch(touch::Event::FingerMoved { .. }) => {
            if is_dragging {
                let _ = cursor.position().map(change);

                return event::Status::Captured;
            }
        }
        _ => {}
    }

    event::Status::Ignored
}

/// Draws a [`Slider`].
pub fn draw<T, Theme, Renderer>(
    renderer: &mut Renderer,
    layout: Layout<'_>,
    cursor: mouse::Cursor,
    state: &State,
    value: T,
    range: &RangeInclusive<T>,
    breakpoints: &[T],
    theme: &Theme,
    style: &Theme::Style,
) where
    T: Into<f64> + Copy,
    Theme: StyleSheet,
    Renderer: crate::core::Renderer,
{
    let bounds = layout.bounds();
    let is_mouse_over = cursor.is_over(bounds);

    let style = if state.is_dragging {
        theme.dragging(style)
    } else if is_mouse_over {
        theme.hovered(style)
    } else {
        theme.active(style)
    };

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
                    .min(bounds.width / 2.0);
                (radius * 2.0, radius * 2.0, Radius::from(radius))
            }
            HandleShape::Rectangle {
                height,
                width,
                border_radius,
            } => {
                let width = (f32::from(width))
                    .max(2.0 * border_width)
                    .min(bounds.width);
                let height = (f32::from(height))
                    .max(2.0 * border_width)
                    .min(bounds.height);
                let mut border_radius: [f32; 4] = border_radius.into();
                for r in &mut border_radius {
                    *r = (*r)
                        .min(height / 2.0)
                        .min(width / 2.0)
                        .max(*r * (width + border_width * 2.0) / width);
                }

                (width, height, border_radius.into())
            }
        };

    let value = value.into() as f32;
    let (range_start, range_end) = {
        let (start, end) = range.clone().into_inner();

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
    for &value in breakpoints {
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

    match style.rail.colors {
        RailBackground::Pair(l, r) => {
            // rail
            renderer.fill_quad(
                renderer::Quad {
                    bounds: Rectangle {
                        x: bounds.x,
                        y: rail_y - style.rail.width / 2.0,
                        width: offset + handle_width / 2.0,
                        height: style.rail.width,
                    },
                    border: Border::with_radius(style.rail.border_radius),
                    ..renderer::Quad::default()
                },
                l,
            );

            // right rail
            renderer.fill_quad(
                renderer::Quad {
                    bounds: Rectangle {
                        x: bounds.x + offset + handle_width / 2.0,
                        y: rail_y - style.rail.width / 2.0,
                        width: bounds.width - offset - handle_width / 2.0,
                        height: style.rail.width,
                    },
                    border: Border::with_radius(style.rail.border_radius),
                    ..renderer::Quad::default()
                },
                r,
            );
        }
        RailBackground::Gradient {
            mut gradient,
            auto_angle,
        } => renderer.fill_quad(
            renderer::Quad {
                bounds: Rectangle {
                    x: bounds.x,
                    y: rail_y - style.rail.width / 2.0,
                    width: bounds.width,
                    height: style.rail.width,
                },
                border: Border::with_radius(style.rail.border_radius),
                ..renderer::Quad::default()
            },
            if auto_angle {
                gradient.angle = Radians::from(Degrees(90.0));
                gradient
            } else {
                gradient
            },
        ),
    }

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
        style.handle.color,
    );
}

/// Computes the current [`mouse::Interaction`] of a [`Slider`].
pub fn mouse_interaction(
    layout: Layout<'_>,
    cursor: mouse::Cursor,
    state: &State,
) -> mouse::Interaction {
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

/// The local state of a [`Slider`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct State {
    is_dragging: bool,
}

impl State {
    /// Creates a new [`State`].
    pub fn new() -> State {
        State::default()
    }
}
