//! Display fields that can be filled with text.
//!
//! A [`TextInput`] has some local [`State`].
mod editor;
mod value;

pub mod cursor;

use std::fs::File;
use std::io::Write;

pub use cursor::Cursor;
#[cfg(feature = "wayland")]
use sctk::reexports::client::protocol::wl_data_device_manager::DndAction;
pub use value::Value;

use editor::Editor;

use crate::alignment;
use crate::event::{self, Event};
#[cfg(feature = "wayland")]
use crate::event::{wayland, PlatformSpecific};
use crate::keyboard;
use crate::layout;
use crate::mouse::{self, click};
use crate::renderer;
use crate::text::{self, Text};
use crate::touch;
use crate::widget;
use crate::widget::operation::{self, Operation};
use crate::widget::tree::{self, Tree};
use crate::{
    Clipboard, Color, Command, Element, Layout, Length, Padding, Point,
    Rectangle, Shell, Size, Vector, Widget,
};

pub use iced_style::text_input::{Appearance, StyleSheet};

const SUPPORTED_MIME_TYPES: &'static [&'static str; 6] = &[
    "text/plain;charset=utf-8",
    "text/plain;charset=UTF-8",
    "UTF8_STRING",
    "STRING",
    "text/plain",
    "TEXT",
];

/// A field that can be filled with text.
///
/// # Example
/// ```
/// # pub type TextInput<'a, Message> = iced_native::widget::TextInput<'a, Message, iced_native::renderer::Null>;
/// #[derive(Debug, Clone)]
/// enum Message {
///     TextInputChanged(String),
/// }
///
/// let value = "Some text";
///
/// let input = TextInput::new(
///     "This is the placeholder...",
///     value,
///     Message::TextInputChanged,
/// )
/// .padding(10);
/// ```
/// ![Text input drawn by `iced_wgpu`](https://github.com/iced-rs/iced/blob/7760618fb112074bc40b148944521f312152012a/docs/images/text_input.png?raw=true)
#[allow(missing_debug_implementations)]
pub struct TextInput<'a, Message, Renderer>
where
    Renderer: text::Renderer,
    Renderer::Theme: StyleSheet,
{
    id: Option<Id>,
    placeholder: String,
    value: Value,
    is_secure: bool,
    font: Renderer::Font,
    width: Length,
    padding: Padding,
    size: Option<u16>,
    on_change: Box<dyn Fn(String) -> Message + 'a>,
    // (text_input::State, mime_type, dnd_action) -> Message
    #[cfg(feature = "wayland")]
    on_start_dnd:
        Option<Box<dyn Fn(State, Vec<String>, DndAction) -> Message + 'a>>,
    dnd_icon: bool,
    on_submit: Option<Message>,
    style: <Renderer::Theme as StyleSheet>::Style,
}

impl<'a, Message, Renderer> TextInput<'a, Message, Renderer>
where
    Message: Clone,
    Renderer: text::Renderer,
    Renderer::Theme: StyleSheet,
{
    /// Creates a new [`TextInput`].
    ///
    /// It expects:
    /// - a placeholder,
    /// - the current value, and
    /// - a function that produces a message when the [`TextInput`] changes.
    pub fn new<F>(placeholder: &str, value: &str, on_change: F) -> Self
    where
        F: 'a + Fn(String) -> Message,
    {
        TextInput {
            id: None,
            placeholder: String::from(placeholder),
            value: Value::new(value),
            is_secure: false,
            font: Default::default(),
            width: Length::Fill,
            padding: Padding::new(5),
            size: None,
            dnd_icon: false,
            on_change: Box::new(on_change),
            #[cfg(feature = "wayland")]
            on_start_dnd: None,
            on_submit: None,
            style: Default::default(),
        }
    }

    /// Sets the [`Id`] of the [`TextInput`].
    pub fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }

    /// Converts the [`TextInput`] into a secure password input.
    pub fn password(mut self) -> Self {
        self.is_secure = true;
        self
    }

    #[cfg(feature = "wayland")]
    /// Sets the on_start_dnd handler of the [`TextInput`].
    pub fn on_start_dnd(
        mut self,
        on_start_dnd: impl Fn(State, Vec<String>, DndAction) -> Message + 'a,
    ) -> Self {
        self.on_start_dnd = Some(Box::new(on_start_dnd));
        self
    }

    /// Sets the [`Font`] of the [`TextInput`].
    ///
    /// [`Font`]: text::Renderer::Font
    pub fn font(mut self, font: Renderer::Font) -> Self {
        self.font = font;
        self
    }
    /// Sets the width of the [`TextInput`].
    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    /// Sets the [`Padding`] of the [`TextInput`].
    pub fn padding<P: Into<Padding>>(mut self, padding: P) -> Self {
        self.padding = padding.into();
        self
    }

    /// Sets the text size of the [`TextInput`].
    pub fn size(mut self, size: u16) -> Self {
        self.size = Some(size);
        self
    }

    /// Sets the message that should be produced when the [`TextInput`] is
    /// focused and the enter key is pressed.
    pub fn on_submit(mut self, message: Message) -> Self {
        self.on_submit = Some(message);
        self
    }

    /// Sets the style of the [`TextInput`].
    pub fn style(
        mut self,
        style: impl Into<<Renderer::Theme as StyleSheet>::Style>,
    ) -> Self {
        self.style = style.into();
        self
    }

    #[cfg(feature = "wayland")]
    /// Sets the mode of this [`TextInput`] to be a drag and drop icon.
    pub fn dnd_icon(mut self, dnd_icon: bool) -> Self {
        self.dnd_icon = dnd_icon;
        self
    }

    /// Draws the [`TextInput`] with the given [`Renderer`], overriding its
    /// [`Value`] if provided.
    ///
    /// [`Renderer`]: text::Renderer
    pub fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Renderer::Theme,
        layout: Layout<'_>,
        cursor_position: Point,
        value: Option<&Value>,
    ) {
        draw(
            renderer,
            theme,
            layout,
            cursor_position,
            tree.state.downcast_ref::<State>(),
            value.unwrap_or(&self.value),
            &self.placeholder,
            self.size,
            &self.font,
            self.is_secure,
            self.dnd_icon,
            &self.style,
        )
    }
}

impl<'a, Message, Renderer> Widget<Message, Renderer>
    for TextInput<'a, Message, Renderer>
where
    Message: Clone,
    Renderer: text::Renderer,
    Renderer::Theme: StyleSheet,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::new())
    }

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
        if self.dnd_icon {
            let limits = limits.width(Length::Shrink).height(Length::Shrink);

            let size = self.size.unwrap_or_else(|| renderer.default_size());

            let bounds = limits.max();

            let (width, height) = renderer.measure(
                &self.value.to_string(),
                size,
                self.font.clone(),
                bounds,
            );

            let size = limits.resolve(Size::new(width, height));
            layout::Node::with_children(size, vec![layout::Node::new(size)])
        } else {
            layout(renderer, limits, self.width, self.padding, self.size)
        }
    }

    fn operate(
        &self,
        tree: &mut Tree,
        _layout: Layout<'_>,
        operation: &mut dyn Operation<Message>,
    ) {
        let state = tree.state.downcast_mut::<State>();

        operation.focusable(state, self.id.as_ref().map(|id| &id.0));
        operation.text_input(state, self.id.as_ref().map(|id| &id.0));
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
        update(
            event,
            layout,
            cursor_position,
            renderer,
            clipboard,
            shell,
            &mut self.value,
            self.size,
            &self.font,
            self.is_secure,
            self.on_change.as_ref(),
            #[cfg(feature = "wayland")]
            self.on_start_dnd.as_deref(),
            #[cfg(feature = "wayland")]
            self.dnd_icon,
            &self.on_submit,
            || tree.state.downcast_mut::<State>(),
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Renderer::Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        cursor_position: Point,
        _viewport: &Rectangle,
    ) {
        draw(
            renderer,
            theme,
            layout,
            cursor_position,
            tree.state.downcast_ref::<State>(),
            &self.value,
            &self.placeholder,
            self.size,
            &self.font,
            self.is_secure,
            self.dnd_icon,
            &self.style,
        );
    }

    fn mouse_interaction(
        &self,
        _state: &Tree,
        layout: Layout<'_>,
        cursor_position: Point,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        mouse_interaction(layout, cursor_position)
    }
}

impl<'a, Message, Renderer> From<TextInput<'a, Message, Renderer>>
    for Element<'a, Message, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a + text::Renderer,
    Renderer::Theme: StyleSheet,
{
    fn from(
        text_input: TextInput<'a, Message, Renderer>,
    ) -> Element<'a, Message, Renderer> {
        Element::new(text_input)
    }
}

/// The identifier of a [`TextInput`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

/// Produces a [`Command`] that focuses the [`TextInput`] with the given [`Id`].
pub fn focus<Message: 'static>(id: Id) -> Command<Message> {
    Command::widget(operation::focusable::focus(id.0))
}

/// Produces a [`Command`] that moves the cursor of the [`TextInput`] with the given [`Id`] to the
/// end.
pub fn move_cursor_to_end<Message: 'static>(id: Id) -> Command<Message> {
    Command::widget(operation::text_input::move_cursor_to_end(id.0))
}

/// Produces a [`Command`] that moves the cursor of the [`TextInput`] with the given [`Id`] to the
/// front.
pub fn move_cursor_to_front<Message: 'static>(id: Id) -> Command<Message> {
    Command::widget(operation::text_input::move_cursor_to_front(id.0))
}

/// Produces a [`Command`] that moves the cursor of the [`TextInput`] with the given [`Id`] to the
/// provided position.
pub fn move_cursor_to<Message: 'static>(
    id: Id,
    position: usize,
) -> Command<Message> {
    Command::widget(operation::text_input::move_cursor_to(id.0, position))
}

/// Produces a [`Command`] that selects all the content of the [`TextInput`] with the given [`Id`].
pub fn select_all<Message: 'static>(id: Id) -> Command<Message> {
    Command::widget(operation::text_input::select_all(id.0))
}

/// Computes the layout of a [`TextInput`].
pub fn layout<Renderer>(
    renderer: &Renderer,
    limits: &layout::Limits,
    width: Length,
    padding: Padding,
    size: Option<u16>,
) -> layout::Node
where
    Renderer: text::Renderer,
{
    let text_size = size.unwrap_or_else(|| renderer.default_size());

    let padding = padding.fit(Size::ZERO, limits.max());

    let limits = limits
        .pad(padding)
        .width(width)
        .height(Length::Units(text_size));

    let mut text = layout::Node::new(limits.resolve(Size::ZERO));
    text.move_to(Point::new(padding.left.into(), padding.top.into()));

    layout::Node::with_children(text.size().pad(padding), vec![text])
}

/// Processes an [`Event`] and updates the [`State`] of a [`TextInput`]
/// accordingly.
pub fn update<'a, Message, Renderer>(
    event: Event,
    layout: Layout<'_>,
    cursor_position: Point,
    renderer: &Renderer,
    clipboard: &mut dyn Clipboard,
    shell: &mut Shell<'_, Message>,
    value: &mut Value,
    size: Option<u16>,
    font: &Renderer::Font,
    is_secure: bool,
    on_change: &dyn Fn(String) -> Message,
    #[cfg(feature = "wayland")] on_start_dnd: Option<
        &dyn Fn(State, Vec<String>, DndAction) -> Message,
    >,
    #[cfg(feature = "wayland")] dnd_icon: bool,
    on_submit: &Option<Message>,
    state: impl FnOnce() -> &'a mut State,
) -> event::Status
where
    Message: Clone,
    Renderer: text::Renderer,
{
    // Dnd Icon version of the widget does not handle any events.
    // TODO maybe the TextInputDndIcon could be a separate widget altogether?
    if dnd_icon {
        return event::Status::Ignored;
    }
    match event {
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
        | Event::Touch(touch::Event::FingerPressed { .. }) => {
            let state = state();
            let bounds = layout.bounds();
            let is_clicked = bounds.contains(cursor_position);

            state.is_focused = is_clicked;

            if is_clicked {
                let text_layout = layout.children().next().unwrap();
                let target = cursor_position.x - text_layout.bounds().x;

                let click =
                    mouse::Click::new(cursor_position, state.last_click);
                match (
                    &state.dragging_state,
                    click.kind(),
                    state.cursor().state(value),
                ) {
                    (
                        None,
                        click::Kind::Single,
                        cursor::State::Selection { start, end },
                    ) => {
                        // if something is already selected, we can start a drag and drop for a
                        // single click that is on top of the selected text

                        // is the click on selected text?
                        #[cfg(feature = "wayland")]
                        if let Some(on_start_dnd) = on_start_dnd {
                            let secure_value =
                                is_secure.then(|| value.secure());
                            let value = secure_value.as_ref().unwrap_or(value);
                            let text_bounds =
                                layout.children().next().unwrap().bounds();
                            let actual_size =
                                size.unwrap_or_else(|| renderer.default_size());

                            let left = start.min(end);
                            let right = end.max(start);

                            let (left_position, _left_offset) =
                                measure_cursor_and_scroll_offset(
                                    renderer,
                                    text_bounds,
                                    value,
                                    actual_size,
                                    left,
                                    font.clone(),
                                );

                            let (right_position, _right_offset) =
                                measure_cursor_and_scroll_offset(
                                    renderer,
                                    text_bounds,
                                    value,
                                    actual_size,
                                    right,
                                    font.clone(),
                                );

                            let width = right_position - left_position;
                            let selection_bounds = Rectangle {
                                x: text_bounds.x + left_position,
                                y: text_bounds.y,
                                width,
                                height: text_bounds.height,
                            };

                            if selection_bounds.contains(cursor_position) {
                                state.dragging_state = Some(
                                    DraggingState::Dnd(DndAction::empty()),
                                );
                                shell.publish(on_start_dnd(
                                    state.clone(),
                                    SUPPORTED_MIME_TYPES
                                        .iter()
                                        .map(|t| t.to_string())
                                        .collect(),
                                    DndAction::Move.union(DndAction::Copy),
                                ));
                            } else {
                                // existing logic for setting the selection
                                let position = if target > 0.0 {
                                    let value = if is_secure {
                                        value.secure()
                                    } else {
                                        value.clone()
                                    };

                                    find_cursor_position(
                                        renderer,
                                        text_layout.bounds(),
                                        font.clone(),
                                        size,
                                        &value,
                                        state,
                                        target,
                                    )
                                } else {
                                    None
                                };

                                state.cursor.move_to(position.unwrap_or(0));
                                state.dragging_state =
                                    Some(DraggingState::Selection);
                            }
                        } else {
                            state.dragging_state = None;
                        }
                    }
                    (Some(DraggingState::Dnd(_)), _, _) => {
                        // TODO: should we cancel if this happens?
                        state.dragging_state = None;
                    }
                    (None, click::Kind::Single, _) => {
                        // existing logic for setting the selection
                        let position = if target > 0.0 {
                            let value = if is_secure {
                                value.secure()
                            } else {
                                value.clone()
                            };

                            find_cursor_position(
                                renderer,
                                text_layout.bounds(),
                                font.clone(),
                                size,
                                &value,
                                state,
                                target,
                            )
                        } else {
                            None
                        };

                        state.cursor.move_to(position.unwrap_or(0));
                        state.dragging_state = Some(DraggingState::Selection);
                    }
                    (None, click::Kind::Double, _)
                    | (
                        Some(DraggingState::Selection),
                        click::Kind::Double,
                        _,
                    ) => {
                        if is_secure {
                            state.cursor.select_all(value);
                        } else {
                            let position = find_cursor_position(
                                renderer,
                                text_layout.bounds(),
                                font.clone(),
                                size,
                                value,
                                state,
                                target,
                            )
                            .unwrap_or(0);

                            state.cursor.select_range(
                                value.previous_start_of_word(position),
                                value.next_end_of_word(position),
                            );
                        }
                        state.dragging_state = Some(DraggingState::Selection);
                    }
                    (None, click::Kind::Triple, _)
                    | (
                        Some(DraggingState::Selection),
                        click::Kind::Triple,
                        _,
                    ) => {
                        state.cursor.select_all(value);
                        state.dragging_state = Some(DraggingState::Selection);
                    }
                    _ => {
                        state.dragging_state = None;
                    }
                }

                state.last_click = Some(click);

                return event::Status::Captured;
            }
        }
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
        | Event::Touch(touch::Event::FingerLifted { .. })
        | Event::Touch(touch::Event::FingerLost { .. }) => {
            let state = state();
            state.dragging_state = None;
        }
        Event::Mouse(mouse::Event::CursorMoved { position })
        | Event::Touch(touch::Event::FingerMoved { position, .. }) => {
            let state = state();
            if matches!(state.dragging_state, Some(DraggingState::Selection)) {
                let text_layout = layout.children().next().unwrap();
                let target = position.x - text_layout.bounds().x;

                let value = if is_secure {
                    value.secure()
                } else {
                    value.clone()
                };

                let position = find_cursor_position(
                    renderer,
                    text_layout.bounds(),
                    font.clone(),
                    size,
                    &value,
                    state,
                    target,
                )
                .unwrap_or(0);

                state
                    .cursor
                    .select_range(state.cursor.start(&value), position);

                return event::Status::Captured;
            }
        }
        Event::Keyboard(keyboard::Event::CharacterReceived(c)) => {
            let state = state();

            if state.is_focused
                && state.is_pasting.is_none()
                && !state.keyboard_modifiers.command()
                && !c.is_control()
            {
                let mut editor = Editor::new(value, &mut state.cursor);

                editor.insert(c);

                let message = (on_change)(editor.contents());
                shell.publish(message);

                return event::Status::Captured;
            }
        }
        Event::Keyboard(keyboard::Event::KeyPressed { key_code, .. }) => {
            let state = state();

            if state.is_focused {
                let modifiers = state.keyboard_modifiers;

                match key_code {
                    keyboard::KeyCode::Enter
                    | keyboard::KeyCode::NumpadEnter => {
                        if let Some(on_submit) = on_submit.clone() {
                            shell.publish(on_submit);
                        }
                    }
                    keyboard::KeyCode::Backspace => {
                        if platform::is_jump_modifier_pressed(modifiers)
                            && state.cursor.selection(value).is_none()
                        {
                            if is_secure {
                                let cursor_pos = state.cursor.end(value);
                                state.cursor.select_range(0, cursor_pos);
                            } else {
                                state.cursor.select_left_by_words(value);
                            }
                        }

                        let mut editor = Editor::new(value, &mut state.cursor);
                        editor.backspace();

                        let message = (on_change)(editor.contents());
                        shell.publish(message);
                    }
                    keyboard::KeyCode::Delete => {
                        if platform::is_jump_modifier_pressed(modifiers)
                            && state.cursor.selection(value).is_none()
                        {
                            if is_secure {
                                let cursor_pos = state.cursor.end(value);
                                state
                                    .cursor
                                    .select_range(cursor_pos, value.len());
                            } else {
                                state.cursor.select_right_by_words(value);
                            }
                        }

                        let mut editor = Editor::new(value, &mut state.cursor);
                        editor.delete();

                        let message = (on_change)(editor.contents());
                        shell.publish(message);
                    }
                    keyboard::KeyCode::Left => {
                        if platform::is_jump_modifier_pressed(modifiers)
                            && !is_secure
                        {
                            if modifiers.shift() {
                                state.cursor.select_left_by_words(value);
                            } else {
                                state.cursor.move_left_by_words(value);
                            }
                        } else if modifiers.shift() {
                            state.cursor.select_left(value)
                        } else {
                            state.cursor.move_left(value);
                        }
                    }
                    keyboard::KeyCode::Right => {
                        if platform::is_jump_modifier_pressed(modifiers)
                            && !is_secure
                        {
                            if modifiers.shift() {
                                state.cursor.select_right_by_words(value);
                            } else {
                                state.cursor.move_right_by_words(value);
                            }
                        } else if modifiers.shift() {
                            state.cursor.select_right(value)
                        } else {
                            state.cursor.move_right(value);
                        }
                    }
                    keyboard::KeyCode::Home => {
                        if modifiers.shift() {
                            state
                                .cursor
                                .select_range(state.cursor.start(value), 0);
                        } else {
                            state.cursor.move_to(0);
                        }
                    }
                    keyboard::KeyCode::End => {
                        if modifiers.shift() {
                            state.cursor.select_range(
                                state.cursor.start(value),
                                value.len(),
                            );
                        } else {
                            state.cursor.move_to(value.len());
                        }
                    }
                    keyboard::KeyCode::C
                        if state.keyboard_modifiers.command() =>
                    {
                        if let Some((start, end)) =
                            state.cursor.selection(value)
                        {
                            clipboard
                                .write(value.select(start, end).to_string());
                        }
                    }
                    keyboard::KeyCode::X
                        if state.keyboard_modifiers.command() =>
                    {
                        if let Some((start, end)) =
                            state.cursor.selection(value)
                        {
                            clipboard
                                .write(value.select(start, end).to_string());
                        }

                        let mut editor = Editor::new(value, &mut state.cursor);
                        editor.delete();

                        let message = (on_change)(editor.contents());
                        shell.publish(message);
                    }
                    keyboard::KeyCode::V => {
                        if state.keyboard_modifiers.command() {
                            let content = match state.is_pasting.take() {
                                Some(content) => content,
                                None => {
                                    let content: String = clipboard
                                        .read()
                                        .unwrap_or_default()
                                        .chars()
                                        .filter(|c| !c.is_control())
                                        .collect();

                                    Value::new(&content)
                                }
                            };

                            let mut editor =
                                Editor::new(value, &mut state.cursor);

                            editor.paste(content.clone());

                            let message = (on_change)(editor.contents());
                            shell.publish(message);

                            state.is_pasting = Some(content);
                        } else {
                            state.is_pasting = None;
                        }
                    }
                    keyboard::KeyCode::A
                        if state.keyboard_modifiers.command() =>
                    {
                        state.cursor.select_all(value);
                    }
                    keyboard::KeyCode::Escape => {
                        state.is_focused = false;
                        state.dragging_state = None;
                        match state.dragging_state.as_ref() {
                            Some(DraggingState::Dnd(_)) => {
                                // TODO should Dnd be canceled? And should it be canceled in
                                // iced-sctk instead?
                            }
                            Some(DraggingState::Selection) => {
                                state.dragging_state = None;
                            }
                            None => {}
                        }
                        state.is_pasting = None;

                        state.keyboard_modifiers =
                            keyboard::Modifiers::default();
                    }
                    keyboard::KeyCode::Tab
                    | keyboard::KeyCode::Up
                    | keyboard::KeyCode::Down => {
                        return event::Status::Ignored;
                    }
                    _ => {}
                }

                return event::Status::Captured;
            }
        }
        Event::Keyboard(keyboard::Event::KeyReleased { key_code, .. }) => {
            let state = state();

            if state.is_focused {
                match key_code {
                    keyboard::KeyCode::V => {
                        state.is_pasting = None;
                    }
                    keyboard::KeyCode::Tab
                    | keyboard::KeyCode::Up
                    | keyboard::KeyCode::Down => {
                        return event::Status::Ignored;
                    }
                    _ => {}
                }

                return event::Status::Captured;
            } else {
                state.is_pasting = None;
            }
        }
        Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
            let state = state();

            state.keyboard_modifiers = modifiers;
        }
        Event::PlatformSpecific(PlatformSpecific::Wayland(
            wayland::Event::DataSource(wayland::DataSourceEvent::SendDndData(
                mime_type,
            )),
        )) => {
            let state = state();
            // if let Some(DraggingState::Dnd(action)) = state.dragging_state {
            //     dbg!(action);
            //     if !action.is_empty()
            //         && SUPPORTED_MIME_TYPES.contains(&mime_type.as_str())
            //     {
            //         if let Some(fd) =
            //             fd.lock().ok().and_then(|fd| fd.try_clone().ok())
            //         {
            //             let mut f = File::from(fd);
            //             let _ = f.write_all(
            //                 state
            //                     .selected_text(value.to_string().as_str())
            //                     .unwrap_or_default()
            //                     .as_bytes(),
            //             );
            //             if action.contains(DndAction::Move) {
            //                 let mut editor =
            //                     Editor::new(value, &mut state.cursor);
            //                 editor.delete();
            //
            //                 let message = (on_change)(editor.contents());
            //                 shell.publish(message);
            //             }
            //         }
            //     }
            // }
        }
        Event::PlatformSpecific(PlatformSpecific::Wayland(
            wayland::Event::DataSource(wayland::DataSourceEvent::DndFinished),
        )) => {
            let state = state();
            if matches!(state.dragging_state, Some(DraggingState::Dnd(_))) {
                state.dragging_state = None;
            }
        }
        Event::PlatformSpecific(PlatformSpecific::Wayland(
            wayland::Event::DataSource(wayland::DataSourceEvent::Cancelled),
        )) => {
            let state = state();
            if matches!(state.dragging_state, Some(DraggingState::Dnd(_))) {
                state.dragging_state = None;
            }
        }
        Event::PlatformSpecific(PlatformSpecific::Wayland(
            wayland::Event::DataSource(
                wayland::DataSourceEvent::DndActionAccepted(action),
            ),
        )) => {
            let state = state();
            if let Some(DraggingState::Dnd(_)) = state.dragging_state {
                state.dragging_state = Some(DraggingState::Dnd(action));
            }
        }
        _ => {}
    }
    event::Status::Ignored
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DraggingState {
    Selection,
    Dnd(DndAction),
}

/// Draws the [`TextInput`] with the given [`Renderer`], overriding its
/// [`Value`] if provided.
///
/// [`Renderer`]: text::Renderer
pub fn draw<Renderer>(
    renderer: &mut Renderer,
    theme: &Renderer::Theme,
    layout: Layout<'_>,
    cursor_position: Point,
    state: &State,
    value: &Value,
    placeholder: &str,
    size: Option<u16>,
    font: &Renderer::Font,
    is_secure: bool,
    dnd_icon: bool,
    style: &<Renderer::Theme as StyleSheet>::Style,
) where
    Renderer: text::Renderer,
    Renderer::Theme: StyleSheet,
{
    let secure_value = is_secure.then(|| value.secure());
    let value = secure_value.as_ref().unwrap_or(value);

    let bounds = layout.bounds();
    let text_bounds = layout.children().next().unwrap().bounds();

    let is_mouse_over = bounds.contains(cursor_position);

    let appearance = if state.is_focused() {
        theme.focused(style)
    } else if is_mouse_over {
        theme.hovered(style)
    } else {
        theme.active(style)
    };

    renderer.fill_quad(
        renderer::Quad {
            bounds,
            border_radius: appearance.border_radius.into(),
            border_width: appearance.border_width,
            border_color: appearance.border_color,
        },
        appearance.background,
    );

    let mut text = value.to_string();
    let size = size.unwrap_or_else(|| renderer.default_size());

    let (cursor, offset) = if state.is_focused() {
        match state.cursor.state(value) {
            cursor::State::Index(position) => {
                let (text_value_width, offset) =
                    measure_cursor_and_scroll_offset(
                        renderer,
                        text_bounds,
                        value,
                        size,
                        position,
                        font.clone(),
                    );
                if !dnd_icon {
                    (
                        Some((
                            renderer::Quad {
                                bounds: Rectangle {
                                    x: text_bounds.x + text_value_width,
                                    y: text_bounds.y,
                                    width: 1.0,
                                    height: text_bounds.height,
                                },
                                border_radius: 0.0.into(),
                                border_width: 0.0,
                                border_color: Color::TRANSPARENT,
                            },
                            theme.value_color(style),
                        )),
                        offset,
                    )
                } else {
                    (None, 0.0)
                }
            }
            cursor::State::Selection { start, end } => {
                let left = start.min(end);
                let right = end.max(start);

                let (left_position, left_offset) =
                    measure_cursor_and_scroll_offset(
                        renderer,
                        text_bounds,
                        value,
                        size,
                        left,
                        font.clone(),
                    );

                let (right_position, right_offset) =
                    measure_cursor_and_scroll_offset(
                        renderer,
                        text_bounds,
                        value,
                        size,
                        right,
                        font.clone(),
                    );

                let width = right_position - left_position;
                if !dnd_icon {
                    (
                        Some((
                            renderer::Quad {
                                bounds: Rectangle {
                                    x: text_bounds.x + left_position,
                                    y: text_bounds.y,
                                    width,
                                    height: text_bounds.height,
                                },
                                border_radius: 0.0.into(),
                                border_width: 0.0,
                                border_color: Color::TRANSPARENT,
                            },
                            theme.selection_color(style),
                        )),
                        if end == right {
                            right_offset
                        } else {
                            left_offset
                        },
                    )
                } else {
                    (None, 0.0)
                }
            }
        }
    } else {
        (None, 0.0)
    };

    let text_width = renderer.measure_width(
        if text.is_empty() { placeholder } else { &text },
        size,
        font.clone(),
    );

    let render = |renderer: &mut Renderer| {
        if let Some((cursor, color)) = cursor {
            if !dnd_icon {
                renderer.fill_quad(cursor, color);
            }
        }
        renderer.fill_text(Text {
            content: if text.is_empty() { placeholder } else { &text },
            color: if text.is_empty() {
                theme.placeholder_color(style)
            } else {
                let mut c = theme.value_color(style);
                c.a = 0.5;
                c
            },
            font: font.clone(),
            bounds: Rectangle {
                y: text_bounds.center_y(),
                width: f32::INFINITY,
                ..text_bounds
            },
            size: f32::from(size),
            horizontal_alignment: alignment::Horizontal::Left,
            vertical_alignment: alignment::Vertical::Center,
        });
    };

    if text_width > text_bounds.width {
        renderer.with_layer(text_bounds, |renderer| {
            renderer.with_translation(Vector::new(-offset, 0.0), render)
        });
    } else {
        render(renderer);
    }
}

/// Computes the current [`mouse::Interaction`] of the [`TextInput`].
pub fn mouse_interaction(
    layout: Layout<'_>,
    cursor_position: Point,
) -> mouse::Interaction {
    if layout.bounds().contains(cursor_position) {
        mouse::Interaction::Text
    } else {
        mouse::Interaction::default()
    }
}

/// The state of a [`TextInput`].
#[derive(Debug, Default, Clone)]
pub struct State {
    is_focused: bool,
    dragging_state: Option<DraggingState>,
    is_pasting: Option<Value>,
    last_click: Option<mouse::Click>,
    cursor: Cursor,
    keyboard_modifiers: keyboard::Modifiers,
    // TODO: Add stateful horizontal scrolling offset
}

impl State {
    /// Creates a new [`State`], representing an unfocused [`TextInput`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the current value of the selected text in the [`TextInput`].
    pub fn selected_text(&self, text: &str) -> Option<String> {
        let value = Value::new(text);
        match self.cursor.state(&value) {
            cursor::State::Index(_) => None,
            cursor::State::Selection { start, end } => {
                let left = start.min(end);
                let right = end.max(start);
                Some(text[left..right].to_string())
            }
        }
    }

    /// Creates a new [`State`], representing a focused [`TextInput`].
    pub fn focused() -> Self {
        Self {
            is_focused: true,
            dragging_state: None,
            is_pasting: None,
            last_click: None,
            cursor: Cursor::default(),
            keyboard_modifiers: keyboard::Modifiers::default(),
        }
    }

    /// Returns whether the [`TextInput`] is currently focused or not.
    pub fn is_focused(&self) -> bool {
        self.is_focused
    }

    /// Returns the [`Cursor`] of the [`TextInput`].
    pub fn cursor(&self) -> Cursor {
        self.cursor
    }

    /// Focuses the [`TextInput`].
    pub fn focus(&mut self) {
        self.is_focused = true;
        self.move_cursor_to_end();
    }

    /// Unfocuses the [`TextInput`].
    pub fn unfocus(&mut self) {
        self.is_focused = false;
    }

    /// Moves the [`Cursor`] of the [`TextInput`] to the front of the input text.
    pub fn move_cursor_to_front(&mut self) {
        self.cursor.move_to(0);
    }

    /// Moves the [`Cursor`] of the [`TextInput`] to the end of the input text.
    pub fn move_cursor_to_end(&mut self) {
        self.cursor.move_to(usize::MAX);
    }

    /// Moves the [`Cursor`] of the [`TextInput`] to an arbitrary location.
    pub fn move_cursor_to(&mut self, position: usize) {
        self.cursor.move_to(position);
    }

    /// Selects all the content of the [`TextInput`].
    pub fn select_all(&mut self) {
        self.cursor.select_range(0, usize::MAX);
    }
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

impl operation::TextInput for State {
    fn move_cursor_to_front(&mut self) {
        State::move_cursor_to_front(self)
    }

    fn move_cursor_to_end(&mut self) {
        State::move_cursor_to_end(self)
    }

    fn move_cursor_to(&mut self, position: usize) {
        State::move_cursor_to(self, position)
    }

    fn select_all(&mut self) {
        State::select_all(self)
    }
}

mod platform {
    use crate::keyboard;

    pub fn is_jump_modifier_pressed(modifiers: keyboard::Modifiers) -> bool {
        if cfg!(target_os = "macos") {
            modifiers.alt()
        } else {
            modifiers.control()
        }
    }
}

fn offset<Renderer>(
    renderer: &Renderer,
    text_bounds: Rectangle,
    font: Renderer::Font,
    size: u16,
    value: &Value,
    state: &State,
) -> f32
where
    Renderer: text::Renderer,
{
    if state.is_focused() {
        let cursor = state.cursor();

        let focus_position = match cursor.state(value) {
            cursor::State::Index(i) => i,
            cursor::State::Selection { end, .. } => end,
        };

        let (_, offset) = measure_cursor_and_scroll_offset(
            renderer,
            text_bounds,
            value,
            size,
            focus_position,
            font,
        );

        offset
    } else {
        0.0
    }
}

fn measure_cursor_and_scroll_offset<Renderer>(
    renderer: &Renderer,
    text_bounds: Rectangle,
    value: &Value,
    size: u16,
    cursor_index: usize,
    font: Renderer::Font,
) -> (f32, f32)
where
    Renderer: text::Renderer,
{
    let text_before_cursor = value.until(cursor_index).to_string();

    let text_value_width =
        renderer.measure_width(&text_before_cursor, size, font);

    let offset = ((text_value_width + 5.0) - text_bounds.width).max(0.0);

    (text_value_width, offset)
}

/// Computes the position of the text cursor at the given X coordinate of
/// a [`TextInput`].
fn find_cursor_position<Renderer>(
    renderer: &Renderer,
    text_bounds: Rectangle,
    font: Renderer::Font,
    size: Option<u16>,
    value: &Value,
    state: &State,
    x: f32,
) -> Option<usize>
where
    Renderer: text::Renderer,
{
    let size = size.unwrap_or_else(|| renderer.default_size());

    let offset =
        offset(renderer, text_bounds, font.clone(), size, value, state);

    renderer
        .hit_test(
            &value.to_string(),
            size.into(),
            font,
            Size::INFINITY,
            Point::new(x + offset, text_bounds.height / 2.0),
            true,
        )
        .map(text::Hit::cursor)
}
