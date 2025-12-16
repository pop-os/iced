//! Allow your users to perform actions by selecting a field.

use super::style::StyleSheet;
use iced_runtime::core::{
    event::{self, Event},
    layout, mouse, overlay, renderer, touch,
    widget::{
        tree::{self, Tree},
        Id,
    },
    Background, Clipboard, Color, Element, Layout, Length, Padding, Point,
    Rectangle, Shell, Widget,
};

/// A generic widget that produces a message when pressed.
#[allow(missing_debug_implementations)]
pub struct SelectionField<'a, Message, Theme, Renderer = iced::Renderer>
where
    Renderer: iced_core::Renderer,
    Theme: StyleSheet,
{
    id: Id,
    content: Element<'a, Message, Theme, Renderer>,
    on_press: Option<Message>,
    on_select: Option<Message>,
    index: usize,
    is_selected: bool,
    width: Length,
    height: Length,
    padding: Padding,
    style: <Theme as StyleSheet>::Style,
}

impl<'a, Message, Theme, Renderer> SelectionField<'a, Message, Theme, Renderer>
where
    Renderer: iced_core::Renderer,
    Theme: StyleSheet,
{
    /// Creates a new [`Button`] with the given content.
    pub fn new(
        content: impl Into<Element<'a, Message, Theme, Renderer>>,
    ) -> Self {
        SelectionField {
            id: Id::unique(),
            content: content.into(),
            on_press: None,
            on_select: None,
            index: 0,
            is_selected: false,
            width: Length::Shrink,
            height: Length::Shrink,
            padding: Padding::new(2.0),
            style: <Theme as StyleSheet>::Style::default(),
        }
    }

    /// Sets the width of the [`Button`].
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the height of the [`Button`].
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Sets the [`Padding`] of the [`Button`].
    pub fn padding<P: Into<Padding>>(mut self, padding: P) -> Self {
        self.padding = padding.into();
        self
    }

    /// Sets the message that will be produced when the [`Button`] is pressed.
    ///
    /// Unless `on_press` is called, the [`Button`] will be disabled.
    pub fn on_press(mut self, on_press: Message) -> Self {
        self.on_press = Some(on_press);
        self
    }

    /// Sets the message that will be produced when the [`SelectionField`] is selected
    pub fn on_select(mut self, on_select: Message) -> Self {
        self.on_select = Some(on_select);
        self
    }

    /// Sets the index values
    pub fn set_indexes(mut self, index: usize) -> Self {
        self.index = index;
        self
    }

    /// Selects the [`SelectionField`] at current page and index
    pub fn selected(mut self, index: usize) -> Self {
        self.is_selected = index == self.index;
        self
    }

    /// Sets the style variant of this [`Button`].
    pub fn style(mut self, style: <Theme as StyleSheet>::Style) -> Self {
        self.style = style;
        self
    }

    /// Sets the [`Id`] of the [`Button`].
    pub fn id(mut self, id: Id) -> Self {
        self.id = id;
        self
    }
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for SelectionField<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a + iced_core::Renderer,
    Theme: StyleSheet,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::new())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&mut self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_mut(&mut self.content))
    }

    fn size(&self) -> iced_core::Size<Length> {
        iced_core::Size::new(self.width, self.height)
    }

    fn layout(
        &self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let limits = limits.width(self.width).height(self.height);

        let mut content = self.content.as_widget().layout(
            &mut tree.children[0],
            renderer,
            &limits,
        );
        let padding = self.padding.fit(content.size(), limits.max());
        let limits = limits.shrink(self.padding);
        let size = limits
            .resolve(self.width, self.height, content.size())
            .expand(self.padding);

        let content = content.move_to(Point::new(padding.left, padding.top));

        layout::Node::with_children(size, vec![content])
    }

    fn on_event(
        &mut self,
        tree: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) -> event::Status {
        if let event::Status::Captured = self.content.as_widget_mut().on_event(
            &mut tree.children[0],
            event.clone(),
            layout.children().next().unwrap(),
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        ) {
            return event::Status::Captured;
        }
        let state = tree.state.downcast_mut::<State>();
        match event {
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let Some(_cursor_position) =
                    cursor.position_in(layout.bounds())
                {
                    state.is_hovered = true;
                    if let Some(on_select) = self.on_select.clone() {
                        shell.publish(on_select);
                    }
                    return event::Status::Captured;
                }
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerPressed { .. }) => {
                if self.on_press.is_some() && cursor.is_over(layout.bounds()) {
                    state.is_pressed = true;
                    return event::Status::Captured;
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerLifted { .. }) => {
                if let Some(on_press) = self.on_press.clone() {
                    if state.is_pressed {
                        state.is_pressed = false;
                        if cursor.is_over(layout.bounds()) {
                            shell.publish(on_press);
                        }
                        return event::Status::Captured;
                    }
                }
            }
            Event::Touch(touch::Event::FingerLost { .. })
            | Event::Mouse(mouse::Event::CursorLeft) => {
                state.is_hovered = false;
                state.is_pressed = false;
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
        renderer_style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let content_layout = layout.children().next().unwrap();

        let styling = if self.is_selected {
            theme.selected(&self.style)
        } else {
            theme.default(&self.style)
        };

        if styling.background.is_some() || styling.border.width > 0.0 {
            renderer.fill_quad(
                renderer::Quad {
                    bounds: layout.bounds(),
                    border: styling.border,
                    shadow: Default::default(),
                },
                styling
                    .background
                    .unwrap_or(Background::Color(Color::TRANSPARENT)),
            );
        }

        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            &renderer::Style {
                icon_color: styling
                    .icon_color
                    .unwrap_or(renderer_style.icon_color),
                text_color: styling.text_color,
                scale_factor: renderer_style.scale_factor,
            },
            content_layout,
            cursor,
            &layout.bounds(),
        );
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        let is_mouse_over = cursor.is_over(layout.bounds());
        if is_mouse_over {
            mouse::Interaction::Pointer
        } else {
            <mouse::Interaction as Default>::default()
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout.children().next().unwrap(),
            renderer,
        )
    }

    fn id(&self) -> Option<Id> {
        Some(self.id.clone())
    }

    fn set_id(&mut self, id: Id) {
        self.id = id;
    }
}

impl<'a, Message, Theme, Renderer>
    From<SelectionField<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: Clone + 'a,
    Renderer: iced_core::Renderer + 'a,
    Theme: StyleSheet + 'a,
{
    fn from(
        selection_field: SelectionField<'a, Message, Theme, Renderer>,
    ) -> Self {
        Self::new(selection_field)
    }
}

/// The local state of a [`Button`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct State {
    is_hovered: bool,
    is_pressed: bool,
}

impl State {
    /// Creates a new [`State`].
    pub fn new() -> State {
        State::default()
    }
}

pub fn selection_field<'a, Message, Theme, Renderer>(
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
) -> SelectionField<'a, Message, Theme, Renderer>
where
    Renderer: iced_core::Renderer,
    Theme: StyleSheet,
    <Theme as StyleSheet>::Style: Default,
{
    SelectionField::new(content)
}
