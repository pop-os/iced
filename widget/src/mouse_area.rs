//! A container for capturing mouse events.

use iced_renderer::core::mouse::Click;
use iced_renderer::core::widget::OperationOutputWrapper;
use iced_renderer::core::Point;

use crate::core::event::{self, Event};
use crate::core::layout;
use crate::core::mouse;
use crate::core::overlay;
use crate::core::renderer;
use crate::core::touch;
use crate::core::widget::{tree, Operation, Tree};
use crate::core::{
    Clipboard, Element, Layout, Length, Rectangle, Shell, Size, Widget,
};

/// Emit messages on mouse events.
#[allow(missing_debug_implementations)]
pub struct MouseArea<
    'a,
    Message,
    Theme = crate::Theme,
    Renderer = crate::Renderer,
> {
    content: Element<'a, Message, Theme, Renderer>,
    on_drag: Option<Message>,
    on_press: Option<Message>,
    on_double_press: Option<Message>,
    on_release: Option<Message>,
    on_right_press: Option<Message>,
    on_right_release: Option<Message>,
    on_middle_press: Option<Message>,
    on_middle_release: Option<Message>,
    on_mouse_enter: Option<Message>,
    on_mouse_exit: Option<Message>,
}

impl<'a, Message, Theme, Renderer> MouseArea<'a, Message, Theme, Renderer> {
    /// The message to emit when a drag is initiated.
    #[must_use]
    pub fn on_drag(mut self, message: Message) -> Self {
        self.on_drag = Some(message);
        self
    }

    /// The message to emit on a left button press.
    #[must_use]
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }
    /// The message to emit on a left double button press.
    #[must_use]
    pub fn on_double_press(mut self, message: Message) -> Self {
        self.on_double_press = Some(message);
        self
    }

    /// The message to emit on a left button release.
    #[must_use]
    pub fn on_release(mut self, message: Message) -> Self {
        self.on_release = Some(message);
        self
    }

    /// The message to emit on a right button press.
    #[must_use]
    pub fn on_right_press(mut self, message: Message) -> Self {
        self.on_right_press = Some(message);
        self
    }

    /// The message to emit on a right button release.
    #[must_use]
    pub fn on_right_release(mut self, message: Message) -> Self {
        self.on_right_release = Some(message);
        self
    }

    /// The message to emit on a middle button press.
    #[must_use]
    pub fn on_middle_press(mut self, message: Message) -> Self {
        self.on_middle_press = Some(message);
        self
    }

    /// The message to emit on a middle button release.
    #[must_use]
    pub fn on_middle_release(mut self, message: Message) -> Self {
        self.on_middle_release = Some(message);
        self
    }
    #[must_use]
    /// The message to emit on mouse enter.
    pub fn on_mouse_enter(mut self, message: Message) -> Self {
        self.on_mouse_enter = Some(message);
        self
    }
    #[must_use]
    /// The message to emit on mouse exit.
    pub fn on_mouse_exit(mut self, message: Message) -> Self {
        self.on_mouse_exit = Some(message);
        self
    }
}

/// Local state of the [`MouseArea`].
struct State {
    // TODO: Support on_mouse_enter and on_mouse_exit
    drag_initiated: Option<Point>,
    is_out_of_bounds: bool,
    last_click: Option<Click>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            drag_initiated: Default::default(),
            is_out_of_bounds: true,
            last_click: Default::default(),
        }
    }
}

impl<'a, Message, Theme, Renderer> MouseArea<'a, Message, Theme, Renderer> {
    /// Creates a [`MouseArea`] with the given content.
    pub fn new(
        content: impl Into<Element<'a, Message, Theme, Renderer>>,
    ) -> Self {
        MouseArea {
            content: content.into(),
            on_drag: None,
            on_press: None,
            on_double_press: None,
            on_release: None,
            on_right_press: None,
            on_right_release: None,
            on_middle_press: None,
            on_middle_release: None,
            on_mouse_enter: None,
            on_mouse_exit: None,
        }
    }
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for MouseArea<'a, Message, Theme, Renderer>
where
    Renderer: renderer::Renderer,
    Message: Clone,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&mut self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_mut(&mut self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation<OperationOutputWrapper<Message>>,
    ) {
        self.content.as_widget().operate(
            &mut tree.children[0],
            layout,
            renderer,
            operation,
        );
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
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        ) {
            return event::Status::Captured;
        }

        update(
            self,
            &event,
            layout,
            cursor,
            shell,
            tree.state.downcast_mut::<State>(),
        )
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        renderer_style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            renderer_style,
            layout,
            cursor,
            viewport,
        );
    }
    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
        )
    }
    fn drag_destinations(
        &self,
        state: &Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        dnd_rectangles: &mut iced_style::core::clipboard::DndDestinationRectangles,
    ) {
        if let Some(state) = state.children.iter().next() {
            self.content.as_widget().drag_destinations(
                state,
                layout,
                renderer,
                dnd_rectangles,
            );
        }
    }
}

impl<'a, Message, Theme, Renderer> From<MouseArea<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Theme: 'a,
    Renderer: 'a + renderer::Renderer,
{
    fn from(
        area: MouseArea<'a, Message, Theme, Renderer>,
    ) -> Element<'a, Message, Theme, Renderer> {
        Element::new(area)
    }
}

/// Processes the given [`Event`] and updates the [`State`] of an [`MouseArea`]
/// accordingly.
fn update<Message: Clone, Theme, Renderer>(
    widget: &mut MouseArea<'_, Message, Theme, Renderer>,
    event: &Event,
    layout: Layout<'_>,
    cursor: mouse::Cursor,
    shell: &mut Shell<'_, Message>,
    state: &mut State,
) -> event::Status {
    if !cursor.is_over(layout.bounds()) {
        if !state.is_out_of_bounds {
            if widget
                .on_mouse_enter
                .as_ref()
                .or(widget.on_mouse_exit.as_ref())
                .is_some()
            {
                if let Event::Mouse(mouse::Event::CursorMoved { .. }) = event {
                    state.is_out_of_bounds = true;
                    if let Some(message) = widget.on_mouse_exit.as_ref() {
                        shell.publish(message.clone());
                    }
                    return event::Status::Captured;
                }
            }
        }

        return event::Status::Ignored;
    }

    if let Some(message) = widget.on_double_press.as_ref() {
        if let Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) =
            event
        {
            if let Some(cursor_position) = cursor.position() {
                let click =
                    mouse::Click::new(cursor_position, state.last_click);
                state.last_click = Some(click);
                if let mouse::click::Kind::Double = click.kind() {
                    state.drag_initiated = None;
                    shell.publish(message.clone());
                    return event::Status::Captured;
                }
            }
        }
    }

    if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
    | Event::Touch(touch::Event::FingerPressed { .. }) = event
    {
        state.drag_initiated = cursor.position();

        if let Some(message) = widget.on_press.as_ref() {
            shell.publish(message.clone());
            return event::Status::Captured;
        }
    }

    if let Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
    | Event::Touch(touch::Event::FingerLifted { .. }) = event
    {
        state.drag_initiated = None;

        if let Some(message) = widget.on_release.as_ref() {
            shell.publish(message.clone());
            return event::Status::Captured;
        }
    }

    if let Some(message) = widget.on_right_press.as_ref() {
        if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) =
            event
        {
            shell.publish(message.clone());

            return event::Status::Captured;
        }
    }

    if let Some(message) = widget.on_right_release.as_ref() {
        if let Event::Mouse(mouse::Event::ButtonReleased(
            mouse::Button::Right,
        )) = event
        {
            shell.publish(message.clone());

            return event::Status::Captured;
        }
    }

    if let Some(message) = widget.on_middle_press.as_ref() {
        if let Event::Mouse(mouse::Event::ButtonPressed(
            mouse::Button::Middle,
        )) = event
        {
            shell.publish(message.clone());

            return event::Status::Captured;
        }
    }

    if let Some(message) = widget.on_middle_release.as_ref() {
        if let Event::Mouse(mouse::Event::ButtonReleased(
            mouse::Button::Middle,
        )) = event
        {
            shell.publish(message.clone());

            return event::Status::Captured;
        }
    }

    if let Some(message) = widget
        .on_mouse_enter
        .as_ref()
        .or(widget.on_mouse_exit.as_ref())
    {
        if let Event::Mouse(mouse::Event::CursorMoved { .. }) = event {
            if state.is_out_of_bounds {
                state.is_out_of_bounds = false;
                if widget.on_mouse_enter.is_some() {
                    shell.publish(message.clone());
                }
                return event::Status::Captured;
            }
        }
    }

    if let Some((message, drag_source)) =
        widget.on_drag.as_ref().zip(state.drag_initiated)
    {
        if let Some(position) = cursor.position() {
            if position.distance(drag_source) > 1.0 {
                state.drag_initiated = None;
                shell.publish(message.clone());

                return event::Status::Captured;
            }
        }
    }

    event::Status::Ignored
}
