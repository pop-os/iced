//! Allow your users to perform actions by pressing a button.
//!
//! A [`Button`] has some local [`State`].
use std::borrow::Cow;

use crate::event::{self, Event};
use crate::keyboard;
use crate::layout;
use crate::mouse;
use crate::overlay;
use crate::renderer;
use crate::touch;
use crate::widget;
use crate::widget::operation::{self, Operation};
use crate::widget::tree::{self, Tree};
use crate::{
    Background, Clipboard, Color, Command, Element, Layout, Length, Padding,
    Point, Rectangle, Shell, Vector, Widget,
};

pub use iced_style::button::{Appearance, StyleSheet};

use super::operation::OperationOutputWrapper;

/// A generic widget that produces a message when pressed.
///
/// ```
/// # type Button<'a, Message> =
/// #     iced_native::widget::Button<'a, Message, iced_native::renderer::Null>;
/// #
/// #[derive(Clone)]
/// enum Message {
///     ButtonPressed,
/// }
///
/// let button = Button::new("Press me!").on_press(Message::ButtonPressed);
/// ```
///
/// If a [`Button::on_press`] handler is not set, the resulting [`Button`] will
/// be disabled:
///
/// ```
/// # type Button<'a, Message> =
/// #     iced_native::widget::Button<'a, Message, iced_native::renderer::Null>;
/// #
/// #[derive(Clone)]
/// enum Message {
///     ButtonPressed,
/// }
///
/// fn disabled_button<'a>() -> Button<'a, Message> {
///     Button::new("I'm disabled!")
/// }
///
/// fn enabled_button<'a>() -> Button<'a, Message> {
///     disabled_button().on_press(Message::ButtonPressed)
/// }
/// ```
#[allow(missing_debug_implementations)]
pub struct Button<'a, Message, Renderer>
where
    Renderer: crate::Renderer,
    Renderer::Theme: StyleSheet,
{
    id: Id,
    name: Option<Cow<'a, str>>,
    description: Option<Cow<'a, str>>,
    content: Element<'a, Message, Renderer>,
    on_press: Option<Message>,
    width: Length,
    height: Length,
    padding: Padding,
    style: <Renderer::Theme as StyleSheet>::Style,
}

impl<'a, Message, Renderer> Button<'a, Message, Renderer>
where
    Renderer: crate::Renderer,
    Renderer::Theme: StyleSheet,
{
    /// Creates a new [`Button`] with the given content.
    pub fn new(content: impl Into<Element<'a, Message, Renderer>>) -> Self {
        Button {
            id: Id::unique(),
            name: None,
            description: None,
            content: content.into(),
            on_press: None,
            width: Length::Shrink,
            height: Length::Shrink,
            padding: Padding::new(5),
            style: <Renderer::Theme as StyleSheet>::Style::default(),
        }
    }

    /// Sets the width of the [`Button`].
    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    /// Sets the height of the [`Button`].
    pub fn height(mut self, height: Length) -> Self {
        self.height = height;
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
    pub fn on_press(mut self, msg: Message) -> Self {
        self.on_press = Some(msg);
        self
    }

    /// Sets the [`Id`] of the [`Button`].
    pub fn id(mut self, id: Id) -> Self {
        self.id = id;
        self
    }

    /// Sets the style variant of this [`Button`].
    pub fn style(
        mut self,
        style: <Renderer::Theme as StyleSheet>::Style,
    ) -> Self {
        self.style = style;
        self
    }

    /// Sets the name of the [`Button`].
    pub fn name(mut self, name: impl Into<Cow<'a, str>>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the description of the [`Button`].
    pub fn description(mut self, description: impl Into<Cow<'a, str>>) -> Self {
        self.description = Some(description.into());
        self
    }
}

impl<'a, Message, Renderer> Widget<Message, Renderer>
    for Button<'a, Message, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a + crate::Renderer,
    Renderer::Theme: StyleSheet,
{
    fn name(&self) -> String {
        self.id.0.to_string()
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::new())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content))
    }

    fn width(&self) -> Length {
        self.width
    }

    fn height(&self) -> Length {
        self.height
    }

    #[cfg(feature = "a11y")]
    /// get the a11y nodes for the widget
    fn a11y_nodes(&self, layout: Layout<'_>, state: &Tree) -> iced_accessibility::A11yTree {
        use iced_accessibility::{
            accesskit::{Action, DefaultActionVerb, NodeBuilder, Rect, Role},
            A11yNode, A11yTree,
        };

        let child_layout = layout.children().next().unwrap();
        let child_tree = &state.children[0];
        let child_tree = self.content.as_widget().a11y_nodes(child_layout, &child_tree);

        let Rectangle {
            x,
            y,
            width,
            height,
        } = layout.bounds();
        let bounds = Rect::new(
            x as f64,
            y as f64,
            (x + width) as f64,
            (y + height) as f64,
        );
        let is_hovered = state.state.downcast_ref::<State>().is_hovered;

        let mut node = NodeBuilder::new(Role::Button);
        node.add_action(Action::Focus);
        node.add_action(Action::Default);
        node.set_bounds(bounds);
        node.set_name(self.name());
        if self.on_press.is_none() {
            node.set_disabled()
        }
        if is_hovered {
            node.set_hovered()
        }
        node.set_default_action_verb(DefaultActionVerb::Click);

        A11yTree::node_with_child_tree(
            A11yNode::new(node, self.id.0.clone()),
            child_tree,
        )
    }

    fn layout(
        &self,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout(
            renderer,
            limits,
            self.width,
            self.height,
            self.padding,
            |renderer, limits| {
                self.content.as_widget().layout(renderer, limits)
            },
        )
    }

    fn operate(
        &self,
        tree: &mut Tree,
        layout: Layout<'_>,
        operation: &mut dyn Operation<OperationOutputWrapper<Message>>,
    ) {
        operation.container(None, &mut |operation| {
            self.content.as_widget().operate(
                &mut tree.children[0],
                layout.children().next().unwrap(),
                operation,
            );
        });

        let state = tree.state.downcast_mut::<State>();
        operation.focusable(state, Some(&self.id.0));
    }

    fn on_event(
        &mut self,
        tree: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor_position: Point,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) -> event::Status {
        if let event::Status::Captured = self.content.as_widget_mut().on_event(
            &mut tree.children[0],
            event.clone(),
            layout.children().next().unwrap(),
            cursor_position,
            renderer,
            clipboard,
            shell,
        ) {
            return event::Status::Captured;
        }

        update(
            self.id.clone(),
            event,
            layout,
            cursor_position,
            shell,
            &self.on_press,
            || tree.state.downcast_mut::<State>(),
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Renderer::Theme,
        renderer_style: &renderer::Style,
        layout: Layout<'_>,
        cursor_position: Point,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let content_layout = layout.children().next().unwrap();

        let styling = draw(
            renderer,
            bounds,
            cursor_position,
            self.on_press.is_some(),
            theme,
            &self.style,
            || tree.state.downcast_ref::<State>(),
        );

        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            &renderer::Style {
                text_color: styling.text_color,
                scale_factor: renderer_style.scale_factor,
            },
            content_layout,
            cursor_position,
            &bounds,
        );
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor_position: Point,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        mouse_interaction(layout, cursor_position, self.on_press.is_some())
    }

    fn overlay<'b>(
        &'b self,
        tree: &'b mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
    ) -> Option<overlay::Element<'b, Message, Renderer>> {
        self.content.as_widget().overlay(
            &mut tree.children[0],
            layout.children().next().unwrap(),
            renderer,
        )
    }

    // TODO each accessible widget really should always have Id, so maybe this doesn't need to be an option
    fn id(&self) -> Option<widget::Id> {
        Some(self.id.0.clone())
    }
}

impl<'a, Message, Renderer> From<Button<'a, Message, Renderer>>
    for Element<'a, Message, Renderer>
where
    Message: Clone + 'a,
    Renderer: crate::Renderer + 'a,
    Renderer::Theme: StyleSheet,
{
    fn from(button: Button<'a, Message, Renderer>) -> Self {
        Self::new(button)
    }
}

/// The local state of a [`Button`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct State {
    is_hovered: bool,
    is_pressed: bool,
    is_focused: bool,
}

impl State {
    /// Creates a new [`State`].
    pub fn new() -> State {
        State::default()
    }

    /// Returns whether the [`Button`] is currently focused or not.
    pub fn is_focused(&self) -> bool {
        self.is_focused
    }

    /// Returns whether the [`Button`] is currently hovered or not.
    pub fn is_hovered(&self) -> bool {
        self.is_hovered
    }

    /// Focuses the [`Button`].
    pub fn focus(&mut self) {
        self.is_focused = true;
    }

    /// Unfocuses the [`Button`].
    pub fn unfocus(&mut self) {
        self.is_focused = false;
    }
}

/// Processes the given [`Event`] and updates the [`State`] of a [`Button`]
/// accordingly.
pub fn update<'a, Message: Clone>(
    id: Id,
    event: Event,
    layout: Layout<'_>,
    cursor_position: Point,
    shell: &mut Shell<'_, Message>,
    on_press: &Option<Message>,
    state: impl FnOnce() -> &'a mut State,
) -> event::Status {
    match event {
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
        | Event::Touch(touch::Event::FingerPressed { .. }) => {
            if on_press.is_some() {
                let bounds = layout.bounds();

                if bounds.contains(cursor_position) {
                    let state = state();
                    
                    state.is_pressed = true;

                    return event::Status::Captured;
                }
            }
        }
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
        | Event::Touch(touch::Event::FingerLifted { .. }) => {
            if let Some(on_press) = on_press.clone() {
                let state = state();

                if state.is_pressed {
                    state.is_pressed = false;

                    let bounds = layout.bounds();

                    if bounds.contains(cursor_position) {
                        shell.publish(on_press);
                    }

                    return event::Status::Captured;
                }
            }
        }

        Event::Keyboard(keyboard::Event::KeyPressed { key_code, .. }) => {
            if let Some(on_press) = on_press.clone() {
                let state = state();
                if state.is_focused && key_code == keyboard::KeyCode::Enter {
                    state.is_pressed = true;
                    shell.publish(on_press);
                    return event::Status::Captured;
                }
            }
        }
        Event::Touch(touch::Event::FingerLost { .. }) => {
            let state = state();
            state.is_hovered = false;
            state.is_pressed = false;
        }
        #[cfg(feature = "a11y")]
        Event::A11y(
            event_id,
            iced_accessibility::accesskit::ActionRequest { action, .. },
        ) => {
            let state = state();
            if let Some(Some(on_press)) = (id.0 == event_id
                && matches!(
                    action,
                    iced_accessibility::accesskit::Action::Default
                ))
            .then(|| on_press.clone())
            {
                state.is_pressed = false;
                shell.publish(on_press);
            }
            return event::Status::Captured;
        }
        Event::Mouse(mouse::Event::CursorEntered | mouse::Event::CursorMoved {
            ..
        }) => {
            let state = state();
            if layout.bounds().contains(cursor_position) {
                state.is_hovered = true;
            } else {
                state.is_hovered = false;
            }
        }
        _ => {}
    }

    event::Status::Ignored
}

/// Draws a [`Button`].
pub fn draw<'a, Renderer: crate::Renderer>(
    renderer: &mut Renderer,
    bounds: Rectangle,
    cursor_position: Point,
    is_enabled: bool,
    style_sheet: &dyn StyleSheet<
        Style = <Renderer::Theme as StyleSheet>::Style,
    >,
    style: &<Renderer::Theme as StyleSheet>::Style,
    state: impl FnOnce() -> &'a State,
) -> Appearance
where
    Renderer::Theme: StyleSheet,
{
    let is_mouse_over = bounds.contains(cursor_position);
    let state = state();

    let styling = if !is_enabled {
        style_sheet.disabled(style)
    } else if is_mouse_over {
        if state.is_pressed {
            style_sheet.pressed(style)
        } else {
            style_sheet.hovered(style)
        }
    } else if state.is_focused {
        style_sheet.focused(style)
    } else {
        style_sheet.active(style)
    };

    if styling.background.is_some() || styling.border_width > 0.0 {
        if styling.shadow_offset != Vector::default() {
            // TODO: Implement proper shadow support
            renderer.fill_quad(
                renderer::Quad {
                    bounds: Rectangle {
                        x: bounds.x + styling.shadow_offset.x,
                        y: bounds.y + styling.shadow_offset.y,
                        ..bounds
                    },
                    border_radius: styling.border_radius,
                    border_width: 0.0,
                    border_color: Color::TRANSPARENT,
                },
                Background::Color([0.0, 0.0, 0.0, 0.5].into()),
            );
        }

        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border_radius: styling.border_radius,
                border_width: styling.border_width,
                border_color: styling.border_color,
            },
            styling
                .background
                .unwrap_or(Background::Color(Color::TRANSPARENT)),
        );
    }

    styling
}

/// Computes the layout of a [`Button`].
pub fn layout<Renderer>(
    renderer: &Renderer,
    limits: &layout::Limits,
    width: Length,
    height: Length,
    padding: Padding,
    layout_content: impl FnOnce(&Renderer, &layout::Limits) -> layout::Node,
) -> layout::Node {
    let limits = limits.width(width).height(height);

    let mut content = layout_content(renderer, &limits.pad(padding));
    let padding = padding.fit(content.size(), limits.max());
    let size = limits.pad(padding).resolve(content.size()).pad(padding);

    content.move_to(Point::new(padding.left.into(), padding.top.into()));

    layout::Node::with_children(size, vec![content])
}

/// Returns the [`mouse::Interaction`] of a [`Button`].
pub fn mouse_interaction(
    layout: Layout<'_>,
    cursor_position: Point,
    is_enabled: bool,
) -> mouse::Interaction {
    let is_mouse_over = layout.bounds().contains(cursor_position);

    if is_mouse_over && is_enabled {
        mouse::Interaction::Pointer
    } else {
        mouse::Interaction::default()
    }
}

/// The identifier of a [`Button`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Id(widget::Id);

impl Id {
    /// Creates a custom [`Id`].
    pub fn new(id: impl Into<std::borrow::Cow<'static, str>>) -> Self {
        Self(widget::Id::new(id))
    }

    /// Creates a unique [`Id`].
    ///
    /// This function produces a different [`Id`] every time it is called.
    pub fn unique() -> Self {
        Self(widget::Id::unique())
    }
}

impl From<Id> for widget::Id {
    fn from(id: Id) -> Self {
        id.0
    }
}

/// Produces a [`Command`] that focuses the [`Button`] with the given [`Id`].
pub fn focus<Message: 'static>(id: Id) -> Command<Message> {
    Command::widget(operation::focusable::focus(id.0))
}

impl operation::Focusable for State {
    fn is_focused(&self) -> bool {
        State::is_focused(self)
    }

    fn focus(&mut self) {
        State::focus(self)
    }

    fn unfocus(&mut self) {
        State::unfocus(self)
    }
}
