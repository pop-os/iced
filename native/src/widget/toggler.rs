//! Show toggle controls using togglers.
use std::borrow::Cow;

use crate::alignment;
use crate::event;
use crate::layout;
use crate::mouse;
use crate::renderer;
use crate::text;
use crate::widget::{self, Row, Text, Tree};
use crate::{
    Alignment, Clipboard, Element, Event, Layout, Length, Point, Rectangle,
    Shell, Widget,
};

use iced_core::Id;
use iced_core::Internal;
pub use iced_style::toggler::{Appearance, StyleSheet};

/// A toggler widget.
///
/// # Example
///
/// ```
/// # type Toggler<'a, Message> = iced_native::widget::Toggler<'a, Message, iced_native::renderer::Null>;
/// #
/// pub enum Message {
///     TogglerToggled(bool),
/// }
///
/// let is_active = true;
///
/// Toggler::new(is_active, String::from("Toggle me!"), |b| Message::TogglerToggled(b));
/// ```
#[allow(missing_debug_implementations)]
pub struct Toggler<'a, Message, Renderer>
where
    Renderer: text::Renderer,
    Renderer::Theme: StyleSheet,
{
    id: Id,
    label_id: Option<Id>,
    #[cfg(feature = "a11y")]
    name: Option<Cow<'a, str>>,
    #[cfg(feature = "a11y")]
    description: Option<iced_accessibility::Description<'a>>,
    #[cfg(feature = "a11y")]
    labeled_by_widget: Option<Vec<iced_accessibility::accesskit::NodeId>>,
    is_active: bool,
    on_toggle: Box<dyn Fn(bool) -> Message + 'a>,
    label: Option<String>,
    width: Length,
    size: u16,
    text_size: Option<u16>,
    text_alignment: alignment::Horizontal,
    spacing: u16,
    font: Renderer::Font,
    style: <Renderer::Theme as StyleSheet>::Style,
}

impl<'a, Message, Renderer> Toggler<'a, Message, Renderer>
where
    Renderer: text::Renderer,
    Renderer::Theme: StyleSheet,
{
    /// The default size of a [`Toggler`].
    pub const DEFAULT_SIZE: u16 = 20;

    /// Creates a new [`Toggler`].
    ///
    /// It expects:
    ///   * a boolean describing whether the [`Toggler`] is checked or not
    ///   * An optional label for the [`Toggler`]
    ///   * a function that will be called when the [`Toggler`] is toggled. It
    ///     will receive the new state of the [`Toggler`] and must produce a
    ///     `Message`.
    pub fn new<F>(
        is_active: bool,
        label: impl Into<Option<String>>,
        f: F,
    ) -> Self
    where
        F: 'a + Fn(bool) -> Message,
    {
        let label = label.into();

        Toggler {
            id: Id::unique(),
            label_id: label.as_ref().map(|_| Id::unique()),
            #[cfg(feature = "a11y")]
            name: None,
            #[cfg(feature = "a11y")]
            description: None,
            #[cfg(feature = "a11y")]
            labeled_by_widget: None,
            is_active,
            on_toggle: Box::new(f),
            label: label.into(),
            width: Length::Fill,
            size: Self::DEFAULT_SIZE,
            text_size: None,
            text_alignment: alignment::Horizontal::Left,
            spacing: 0,
            font: Renderer::Font::default(),
            style: Default::default(),
        }
    }

    #[cfg(feature = "a11y")]
    /// Sets the name of the [`Button`].
    pub fn name(mut self, name: impl Into<Cow<'a, str>>) -> Self {
        self.name = Some(name.into());
        self
    }

    #[cfg(feature = "a11y")]
    /// Sets the description of the [`Button`].
    pub fn description_widget<T: iced_accessibility::Describes>(
        mut self,
        description: &T,
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
    /// Sets the label of the [`Button`] using another widget.
    pub fn label(mut self, label: &dyn iced_accessibility::Labels) -> Self {
        self.labeled_by_widget =
            Some(label.label().into_iter().map(|l| l.into()).collect());
        self
    }

    /// Sets the size of the [`Toggler`].
    pub fn size(mut self, size: u16) -> Self {
        self.size = size;
        self
    }

    /// Sets the width of the [`Toggler`].
    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    /// Sets the text size o the [`Toggler`].
    pub fn text_size(mut self, text_size: u16) -> Self {
        self.text_size = Some(text_size);
        self
    }

    /// Sets the horizontal alignment of the text of the [`Toggler`]
    pub fn text_alignment(mut self, alignment: alignment::Horizontal) -> Self {
        self.text_alignment = alignment;
        self
    }

    /// Sets the spacing between the [`Toggler`] and the text.
    pub fn spacing(mut self, spacing: u16) -> Self {
        self.spacing = spacing;
        self
    }

    /// Sets the [`Font`] of the text of the [`Toggler`]
    ///
    /// [`Font`]: crate::text::Renderer::Font
    pub fn font(mut self, font: Renderer::Font) -> Self {
        self.font = font;
        self
    }

    /// Sets the style of the [`Toggler`].
    pub fn style(
        mut self,
        style: impl Into<<Renderer::Theme as StyleSheet>::Style>,
    ) -> Self {
        self.style = style.into();
        self
    }
}

impl<'a, Message, Renderer> Widget<Message, Renderer>
    for Toggler<'a, Message, Renderer>
where
    Renderer: text::Renderer,
    Renderer::Theme: StyleSheet + widget::text::StyleSheet,
{
    fn width(&self) -> Length {
        self.width
    }

    fn height(&self) -> Length {
        Length::Shrink
    }

    fn layout(
        &self,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let mut row = Row::<(), Renderer>::new()
            .width(self.width)
            .spacing(self.spacing)
            .align_items(Alignment::Center);

        if let Some(label) = &self.label {
            row = row.push(
                Text::new(label)
                    .horizontal_alignment(self.text_alignment)
                    .font(self.font.clone())
                    .width(self.width)
                    .size(
                        self.text_size
                            .unwrap_or_else(|| renderer.default_size()),
                    ),
            );
        }

        row = row.push(
            Row::new()
                .width(Length::Units(2 * self.size))
                .height(Length::Units(self.size)),
        );

        row.layout(renderer, limits)
    }

    fn on_event(
        &mut self,
        _state: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor_position: Point,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) -> event::Status {
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let mouse_over = layout.bounds().contains(cursor_position);

                if mouse_over {
                    shell.publish((self.on_toggle)(!self.is_active));

                    event::Status::Captured
                } else {
                    event::Status::Ignored
                }
            }
            _ => event::Status::Ignored,
        }
    }

    fn mouse_interaction(
        &self,
        _state: &Tree,
        layout: Layout<'_>,
        cursor_position: Point,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if layout.bounds().contains(cursor_position) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }

    fn draw(
        &self,
        _state: &Tree,
        renderer: &mut Renderer,
        theme: &Renderer::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor_position: Point,
        _viewport: &Rectangle,
    ) {
        /// Makes sure that the border radius of the toggler looks good at every size.
        const BORDER_RADIUS_RATIO: f32 = 32.0 / 13.0;

        /// The space ratio between the background Quad and the Toggler bounds, and
        /// between the background Quad and foreground Quad.
        const SPACE_RATIO: f32 = 0.05;

        let mut children = layout.children();

        if let Some(label) = &self.label {
            let label_layout = children.next().unwrap();

            crate::widget::text::draw(
                renderer,
                style,
                label_layout,
                label,
                self.text_size,
                self.font.clone(),
                Default::default(),
                self.text_alignment,
                alignment::Vertical::Center,
            );
        }

        let toggler_layout = children.next().unwrap();
        let bounds = toggler_layout.bounds();

        let is_mouse_over = bounds.contains(cursor_position);

        let style = if is_mouse_over {
            theme.hovered(&self.style, self.is_active)
        } else {
            theme.active(&self.style, self.is_active)
        };

        let border_radius = bounds.height as f32 / BORDER_RADIUS_RATIO;
        let space = SPACE_RATIO * bounds.height as f32;

        let toggler_background_bounds = Rectangle {
            x: bounds.x + space,
            y: bounds.y + space,
            width: bounds.width - (2.0 * space),
            height: bounds.height - (2.0 * space),
        };

        renderer.fill_quad(
            renderer::Quad {
                bounds: toggler_background_bounds,
                border_radius: border_radius.into(),
                border_width: 1.0,
                border_color: style
                    .background_border
                    .unwrap_or(style.background),
            },
            style.background,
        );

        let toggler_foreground_bounds = Rectangle {
            x: bounds.x
                + if self.is_active {
                    bounds.width - 2.0 * space - (bounds.height - (4.0 * space))
                } else {
                    2.0 * space
                },
            y: bounds.y + (2.0 * space),
            width: bounds.height - (4.0 * space),
            height: bounds.height - (4.0 * space),
        };

        renderer.fill_quad(
            renderer::Quad {
                bounds: toggler_foreground_bounds,
                border_radius: border_radius.into(),
                border_width: 1.0,
                border_color: style
                    .foreground_border
                    .unwrap_or(style.foreground),
            },
            style.foreground,
        );
    }

    #[cfg(feature = "a11y")]
    /// get the a11y nodes for the widget
    fn a11y_nodes(
        &self,
        layout: Layout<'_>,
        _state: &Tree,
        cursor_position: Point,
    ) -> iced_accessibility::A11yTree {
        use iced_accessibility::{
            accesskit::{
                Action, CheckedState, NodeBuilder, NodeId, Rect, Role,
            },
            A11yNode, A11yTree,
        };

        let bounds = layout.bounds();
        let is_hovered = bounds.contains(cursor_position);
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

        let mut node = NodeBuilder::new(Role::Switch);
        node.add_action(Action::Focus);
        node.add_action(Action::Default);
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
        node.set_checked_state(if self.is_active {
            CheckedState::True
        } else {
            CheckedState::False
        });
        if is_hovered {
            node.set_hovered();
        }
        node.add_action(Action::Default);
        if let Some(label) = self.label.as_ref() {
            let mut label_node = NodeBuilder::new(Role::StaticText);

            label_node.set_name(label.clone());
            // TODO proper label bounds for the label
            label_node.set_bounds(bounds);

            A11yTree::node_with_child_tree(
                A11yNode::new(node, self.id.clone()),
                A11yTree::leaf(label_node, self.label_id.clone().unwrap()),
            )
        } else {
            if let Some(labeled_by_widget) = self.labeled_by_widget.as_ref() {
                node.set_labelled_by(labeled_by_widget.clone());
            }
            A11yTree::leaf(node, self.id.clone())
        }
    }

    fn id(&self) -> Option<Id> {
        if self.label.is_some() {
            Some(Id(Internal::Set(vec![
                self.id.0.clone(),
                self.label_id.clone().unwrap().0,
            ])))
        } else {
            Some(self.id.clone())
        }
    }
}

impl<'a, Message, Renderer> From<Toggler<'a, Message, Renderer>>
    for Element<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: 'a + text::Renderer,
    Renderer::Theme: StyleSheet + widget::text::StyleSheet,
{
    fn from(
        toggler: Toggler<'a, Message, Renderer>,
    ) -> Element<'a, Message, Renderer> {
        Element::new(toggler)
    }
}
