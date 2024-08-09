//! Show toggle controls using togglers.
#[cfg(feature = "a11y")]
use std::borrow::Cow;

use iced_runtime::core::border::Radius;

use crate::core::alignment;
use crate::core::event;
use crate::core::layout;
use crate::core::mouse;
use crate::core::renderer;
use crate::core::text;
use crate::core::touch;
use crate::core::widget::tree::{self, Tree};
use crate::core::widget::{self, Id};
use crate::core::{
    id, Border, Clipboard, Color, Element, Event, Layout, Length, Pixels,
    Rectangle, Shell, Size, Theme, Widget,
};

/// A toggler widget.
///
/// # Example
///
/// ```no_run
/// # type Toggler<'a, Message> = iced_widget::Toggler<'a, Message>;
/// #
/// pub enum Message {
///     TogglerToggled(bool),
/// }
///
/// let is_toggled = true;
///
/// Toggler::new(String::from("Toggle me!"), is_toggled, |b| Message::TogglerToggled(b));
/// ```
#[allow(missing_debug_implementations)]
pub struct Toggler<
    'a,
    Message,
    Theme = crate::Theme,
    Renderer = crate::Renderer,
> where
    Theme: Catalog,
    Renderer: text::Renderer,
{
    id: Id,
    label_id: Option<Id>,
    #[cfg(feature = "a11y")]
    name: Option<Cow<'a, str>>,
    #[cfg(feature = "a11y")]
    description: Option<iced_accessibility::Description<'a>>,
    #[cfg(feature = "a11y")]
    labeled_by_widget: Option<Vec<iced_accessibility::accesskit::NodeId>>,
    is_toggled: bool,
    on_toggle: Box<dyn Fn(bool) -> Message + 'a>,
    label: Option<String>,
    width: Length,
    size: f32,
    text_size: Option<Pixels>,
    text_line_height: text::LineHeight,
    text_alignment: alignment::Horizontal,
    text_shaping: text::Shaping,
    text_wrap: text::Wrap,
    spacing: f32,
    font: Option<Renderer::Font>,
    class: Theme::Class<'a>,
}

impl<'a, Message, Theme, Renderer> Toggler<'a, Message, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: text::Renderer,
{
    /// The default size of a [`Toggler`].
    pub const DEFAULT_SIZE: f32 = 16.0;

    /// Creates a new [`Toggler`].
    ///
    /// It expects:
    ///   * a boolean describing whether the [`Toggler`] is checked or not
    ///   * An optional label for the [`Toggler`]
    ///   * a function that will be called when the [`Toggler`] is toggled. It
    ///     will receive the new state of the [`Toggler`] and must produce a
    ///     `Message`.
    pub fn new<F>(
        label: impl Into<Option<String>>,
        is_toggled: bool,
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
            is_toggled,
            on_toggle: Box::new(f),
            label,
            width: Length::Fill,
            size: Self::DEFAULT_SIZE,
            text_size: None,
            text_line_height: text::LineHeight::default(),
            text_alignment: alignment::Horizontal::Left,
            text_shaping: text::Shaping::Advanced,
            text_wrap: text::Wrap::default(),
            spacing: 0.0,
            font: None,
            class: Theme::default(),
        }
    }

    /// Sets the size of the [`Toggler`].
    pub fn size(mut self, size: impl Into<Pixels>) -> Self {
        self.size = size.into().0;
        self
    }

    /// Sets the width of the [`Toggler`].
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the text size o the [`Toggler`].
    pub fn text_size(mut self, text_size: impl Into<Pixels>) -> Self {
        self.text_size = Some(text_size.into());
        self
    }

    /// Sets the text [`text::LineHeight`] of the [`Toggler`].
    pub fn text_line_height(
        mut self,
        line_height: impl Into<text::LineHeight>,
    ) -> Self {
        self.text_line_height = line_height.into();
        self
    }

    /// Sets the horizontal alignment of the text of the [`Toggler`]
    pub fn text_alignment(mut self, alignment: alignment::Horizontal) -> Self {
        self.text_alignment = alignment;
        self
    }

    /// Sets the [`text::Shaping`] strategy of the [`Toggler`].
    pub fn text_shaping(mut self, shaping: text::Shaping) -> Self {
        self.text_shaping = shaping;
        self
    }

    /// Sets the [`text::Wrap`] mode of the [`Toggler`].
    pub fn text_wrap(mut self, wrap: text::Wrap) -> Self {
        self.text_wrap = wrap;
        self
    }

    /// Sets the spacing between the [`Toggler`] and the text.
    pub fn spacing(mut self, spacing: impl Into<Pixels>) -> Self {
        self.spacing = spacing.into().0;
        self
    }

    /// Sets the [`Renderer::Font`] of the text of the [`Toggler`]
    ///
    /// [`Renderer::Font`]: crate::core::text::Renderer
    pub fn font(mut self, font: impl Into<Renderer::Font>) -> Self {
        self.font = Some(font.into());
        self
    }

    /// Sets the style of the [`Toggler`].
    #[must_use]
    pub fn style(mut self, style: impl Fn(&Theme, Status) -> Style + 'a) -> Self
    where
        Theme::Class<'a>: From<StyleFn<'a, Theme>>,
    {
        self.class = (Box::new(style) as StyleFn<'a, Theme>).into();
        self
    }

    /// Sets the style class of the [`Toggler`].
    #[cfg(feature = "advanced")]
    #[must_use]
    pub fn class(mut self, class: impl Into<Theme::Class<'a>>) -> Self {
        self.class = class.into();
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
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Toggler<'a, Message, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: text::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<widget::text::State<Renderer::Paragraph>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(widget::text::State::<Renderer::Paragraph>::default())
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: Length::Shrink,
        }
    }

    fn layout(
        &self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let limits = limits.width(self.width);

        layout::next_to_each_other(
            &limits,
            self.spacing,
            |_| layout::Node::new(crate::core::Size::new(48., 24.)),
            |limits| {
                if let Some(label) = self.label.as_deref() {
                    let state = tree
                    .state
                    .downcast_mut::<widget::text::State<Renderer::Paragraph>>();

                    widget::text::layout(
                        state,
                        renderer,
                        limits,
                        self.width,
                        Length::Shrink,
                        label,
                        self.text_line_height,
                        self.text_size,
                        self.font,
                        self.text_alignment,
                        alignment::Vertical::Top,
                        self.text_shaping,
                        self.text_wrap,
                    )
                } else {
                    layout::Node::new(crate::core::Size::ZERO)
                }
            },
        )
    }

    fn on_event(
        &mut self,
        _state: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) -> event::Status {
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerPressed { .. }) => {
                let mouse_over = cursor.is_over(layout.bounds());

                if mouse_over {
                    shell.publish((self.on_toggle)(!self.is_toggled));

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
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if cursor.is_over(layout.bounds()) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let mut children = layout.children();
        let toggler_layout = children.next().unwrap();

        if self.label.is_some() {
            let label_layout = children.next().unwrap();

            crate::text::draw(
                renderer,
                style,
                label_layout,
                tree.state.downcast_ref(),
                crate::text::Style::default(),
                viewport,
            );
        }

        let bounds = toggler_layout.bounds();
        let is_mouse_over = cursor.is_over(layout.bounds());

        let status = if is_mouse_over {
            Status::Hovered {
                is_toggled: self.is_toggled,
            }
        } else {
            Status::Active {
                is_toggled: self.is_toggled,
            }
        };

        let style = theme.style(&self.class, status);

        let space = style.handle_margin;

        let toggler_background_bounds = Rectangle {
            x: bounds.x,
            y: bounds.y,
            width: bounds.width,
            height: bounds.height,
        };

        renderer.fill_quad(
            renderer::Quad {
                bounds: toggler_background_bounds,
                border: Border {
                    radius: style.border_radius,
                    width: style.background_border_width,
                    color: style.background_border_color,
                },
                ..renderer::Quad::default()
            },
            style.background,
        );

        let toggler_foreground_bounds = Rectangle {
            x: bounds.x
                + if self.is_toggled {
                    bounds.width - space - (bounds.height - (2.0 * space))
                } else {
                    space
                },
            y: bounds.y + space,
            width: bounds.height - (2.0 * space),
            height: bounds.height - (2.0 * space),
        };

        renderer.fill_quad(
            renderer::Quad {
                bounds: toggler_foreground_bounds,
                border: Border {
                    radius: style.handle_radius,
                    width: style.foreground_border_width,
                    color: style.foreground_border_color,
                },
                ..renderer::Quad::default()
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
        cursor: mouse::Cursor,
    ) -> iced_accessibility::A11yTree {
        use iced_accessibility::{
            accesskit::{Action, NodeBuilder, NodeId, Rect, Role},
            A11yNode, A11yTree,
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
        node.set_selected(self.is_toggled);
        if is_hovered {
            node.set_hovered();
        }
        node.add_action(Action::Default);
        if let Some(label) = self.label.as_ref() {
            let mut label_node = NodeBuilder::new(Role::Label);

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
            Some(Id(iced_runtime::core::id::Internal::Set(vec![
                self.id.0.clone(),
                self.label_id.clone().unwrap().0,
            ])))
        } else {
            Some(self.id.clone())
        }
    }

    fn set_id(&mut self, id: Id) {
        if let Id(id::Internal::Set(list)) = id {
            if list.len() == 2 && self.label.is_some() {
                self.id.0 = list[0].clone();
                self.label_id = Some(Id(list[1].clone()));
            }
        } else if self.label.is_none() {
            self.id = id;
        }
    }
}

impl<'a, Message, Theme, Renderer> From<Toggler<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: Catalog + 'a,
    Renderer: text::Renderer + 'a,
{
    fn from(
        toggler: Toggler<'a, Message, Theme, Renderer>,
    ) -> Element<'a, Message, Theme, Renderer> {
        Element::new(toggler)
    }
}

/// The possible status of a [`Toggler`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// The [`Toggler`] can be interacted with.
    Active {
        /// Indicates whether the [`Toggler`] is toggled.
        is_toggled: bool,
    },
    /// The [`Toggler`] is being hovered.
    Hovered {
        /// Indicates whether the [`Toggler`] is toggled.
        is_toggled: bool,
    },
}

/// The appearance of a toggler.
#[derive(Debug, Clone, Copy)]
pub struct Style {
    /// The background [`Color`] of the toggler.
    pub background: Color,
    /// The width of the background border of the toggler.
    pub background_border_width: f32,
    /// The [`Color`] of the background border of the toggler.
    pub background_border_color: Color,
    /// The foreground [`Color`] of the toggler.
    pub foreground: Color,
    /// The width of the foreground border of the toggler.
    pub foreground_border_width: f32,
    /// The [`Color`] of the foreground border of the toggler.
    pub foreground_border_color: Color,
    /// The border radius of the toggler.
    pub border_radius: Radius,
    /// the radius of the handle of the toggler
    pub handle_radius: Radius,
    /// the space between the handle and the border of the toggler
    pub handle_margin: f32,
}

/// The theme catalog of a [`Toggler`].
pub trait Catalog: Sized {
    /// The item class of the [`Catalog`].
    type Class<'a>;

    /// The default class produced by the [`Catalog`].
    fn default<'a>() -> Self::Class<'a>;

    /// The [`Style`] of a class with the given status.
    fn style(&self, class: &Self::Class<'_>, status: Status) -> Style;
}

/// A styling function for a [`Toggler`].
///
/// This is just a boxed closure: `Fn(&Theme, Status) -> Style`.
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

/// The default style of a [`Toggler`].
pub fn default(theme: &Theme, status: Status) -> Style {
    let palette = theme.extended_palette();

    let background = match status {
        Status::Active { is_toggled } | Status::Hovered { is_toggled } => {
            if is_toggled {
                palette.primary.strong.color
            } else {
                palette.background.strong.color
            }
        }
    };

    let foreground = match status {
        Status::Active { is_toggled } => {
            if is_toggled {
                palette.primary.strong.text
            } else {
                palette.background.base.color
            }
        }
        Status::Hovered { is_toggled } => {
            if is_toggled {
                Color {
                    a: 0.5,
                    ..palette.primary.strong.text
                }
            } else {
                palette.background.weak.color
            }
        }
    };

    Style {
        background,
        foreground,
        foreground_border_width: 0.0,
        foreground_border_color: Color::TRANSPARENT,
        background_border_width: 0.0,
        background_border_color: Color::TRANSPARENT,
        border_radius: Radius::from(8.0),
        handle_radius: Radius::from(8.0),
        handle_margin: 2.0,
    }
}
