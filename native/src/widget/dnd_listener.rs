//! A container for capturing mouse events.

use sctk::reexports::client::protocol::wl_data_device_manager::DndAction;

use crate::event::wayland::{DndOfferEvent, ReadData};
use crate::event::{self, Event, PlatformSpecific};
use crate::layout;
use crate::mouse;
use crate::overlay;
use crate::renderer;
use crate::widget::{tree, Operation, Tree};
use crate::{
    Clipboard, Element, Layout, Length, Point, Rectangle, Shell, Widget,
};

use std::u32;

/// Emit messages on mouse events.
#[allow(missing_debug_implementations)]
pub struct DndListener<'a, Message, Renderer> {
    content: Element<'a, Message, Renderer>,

    /// Sets the message to emit on a drag enter.
    on_enter: Option<Box<dyn Fn(DndAction, Vec<String>, (f32, f32)) -> Message>>,

    /// Sets the message to emit on a drag motion.
    /// x and y are the coordinates of the pointer relative to the widget in the range (0.0, 1.0)
    on_motion: Option<Box<dyn Fn(f32, f32) -> Message>>,

    /// Sets the message to emit on a drag exit.
    on_exit: Option<Message>,

    /// Sets the message to emit on a drag drop.
    on_drop: Option<Message>,

    /// Sets the message to emit on a drag mime type event.
    on_mime_type: Option<Box<dyn Fn(String) -> Message>>,

    /// Sets the message to emit on a drag action event.
    on_source_actions: Option<Box<dyn Fn(DndAction) -> Message>>,

    /// Sets the message to emit on a drag action event.
    on_selected_action: Option<Box<dyn Fn(DndAction) -> Message>>,

    /// Sets the message to emit on a Read Data event.
    on_read_data: Option<Box<dyn Fn(ReadData) -> Message>>,
}

impl<'a, Message, Renderer> DndListener<'a, Message, Renderer> {
    /// The message to emit on a drag enter.
    #[must_use]
    pub fn on_enter(mut self, message: Box<dyn Fn(DndAction, Vec<String>, (f32, f32)) -> Message>) -> Self {
        self.on_enter = Some(message);
        self
    }

    /// The message to emit on a drag exit.
    #[must_use]
    pub fn on_exit(mut self, message: Message) -> Self {
        self.on_exit = Some(message);
        self
    }

    /// The message to emit on a drag drop.
    #[must_use]
    pub fn on_drop(mut self, message: Message) -> Self {
        self.on_drop = Some(message);
        self
    }

    /// The message to emit on a drag mime type event.
    #[must_use]
    pub fn on_mime_type(mut self, message: Box<dyn Fn(String) -> Message>) -> Self {
        self.on_mime_type = Some(message);
        self
    }

    /// The message to emit on a drag action event.
    #[must_use]
    pub fn on_action(mut self, message: Box<dyn Fn(DndAction) -> Message>) -> Self {
        self.on_source_actions = Some(message);
        self
    }
}

/// Local state of the [`DndListener`].
#[derive(Default)]
struct State {
    dnd: Option<(DndAction, Vec<String>)>,
    hovered: bool,
}

impl<'a, Message, Renderer> DndListener<'a, Message, Renderer> {
    /// Creates an empty [`DndListener`].
    pub fn new(content: impl Into<Element<'a, Message, Renderer>>) -> Self {
        DndListener {
            content: content.into(),
            on_enter: None,
            on_motion: None,
            on_exit: None,
            on_drop: None,
            on_mime_type: None,
            on_source_actions: None,
            on_selected_action: None,
            on_read_data: None,
        }
    }
}

impl<'a, Message, Renderer> Widget<Message, Renderer>
    for DndListener<'a, Message, Renderer>
where
    Renderer: crate::Renderer,
    Message: Clone,
{
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn width(&self) -> Length {
        self.content.as_widget().width()
    }

    fn height(&self) -> Length {
        self.content.as_widget().height()
    }

    fn layout(
        &self,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout(
            renderer,
            limits,
            Widget::<Message, Renderer>::width(self),
            Widget::<Message, Renderer>::height(self),
            u32::MAX,
            u32::MAX,
            |renderer, limits| {
                self.content.as_widget().layout(renderer, limits)
            },
        )
    }

    fn operate(
        &self,
        tree: &mut Tree,
        layout: Layout<'_>,
        operation: &mut dyn Operation<Message>,
    ) {
        operation.container(None, &mut |operation| {
            self.content.as_widget().operate(
                &mut tree.children[0],
                layout.children().next().unwrap(),
                operation,
            );
        });
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
            self,
            &event,
            layout,
            shell,
            tree.state.downcast_mut::<State>(),
        )
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor_position: Point,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout.children().next().unwrap(),
            cursor_position,
            viewport,
            renderer,
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
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            renderer_style,
            layout.children().next().unwrap(),
            cursor_position,
            viewport,
        );
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

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }
}

impl<'a, Message, Renderer> From<DndListener<'a, Message, Renderer>>
    for Element<'a, Message, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a + crate::Renderer,
{
    fn from(
        listener: DndListener<'a, Message, Renderer>,
    ) -> Element<'a, Message, Renderer> {
        Element::new(listener)
    }
}

/// Processes the given [`Event`] and updates the [`State`] of an [`DndListener`]
/// accordingly.
fn update<Message: Clone, Renderer>(
    widget: &mut DndListener<'_, Message, Renderer>,
    event: &Event,
    layout: Layout<'_>,
    shell: &mut Shell<'_, Message>,
    state: &mut State,
) -> event::Status {
    match event {
        Event::PlatformSpecific(PlatformSpecific::Wayland(event::wayland::Event::DndOffer(DndOfferEvent::Enter { x, y, mime_types }))) => {
            state.dnd = Some((DndAction::empty(), mime_types.clone()));
            let bounds = layout.bounds();
            let p = Point { x: *x as f32, y: *y as f32 };
            if layout.bounds().contains(p) {
                state.hovered = true;
                if let Some(message) = widget.on_enter.as_ref() {
                    let normalized_x: f32 = (p.x - bounds.x) / bounds.width;
                    let normalized_y: f32 = (p.y - bounds.y) / bounds.height;
                    shell.publish(message(DndAction::empty(), mime_types.clone(), (normalized_x, normalized_y)));
                    return event::Status::Captured;
                }
            }
    
        }
        Event::PlatformSpecific(PlatformSpecific::Wayland(event::wayland::Event::DndOffer(DndOfferEvent::Motion { x, y }))) => {
            let bounds = layout.bounds();
            let p = Point { x: *x as f32, y: *y as f32 };
            // motion can trigger an enter, motion or leave event on the widget
            if state.hovered && !layout.bounds().contains(p) {
                state.hovered = false;
                if let Some(message) = widget.on_exit.clone() {
                    shell.publish(message);
                    return event::Status::Captured;
                }
            } else if state.hovered && layout.bounds().contains(p) {
                if let Some(message) = widget.on_motion.as_ref() {
                    let normalized_x: f32 = (p.x - bounds.x) / bounds.width;
                    let normalized_y: f32 = (p.y - bounds.y) / bounds.height;
                    shell.publish(message(normalized_x, normalized_y));
                    return event::Status::Captured;
                }
            } else if !state.hovered && layout.bounds().contains(p) {
                let (action, mime_types) = match state.dnd.as_ref() {
                    Some((action, mime_types)) => (action, mime_types),
                    None => return event::Status::Ignored,
                };
                state.hovered = true;
                if let Some(message) = widget.on_enter.as_ref() {
                    let normalized_x: f32 = (p.x - bounds.x) / bounds.width;
                    let normalized_y: f32 = (p.y - bounds.y) / bounds.height;
                    shell.publish(message(*action, mime_types.clone(), (normalized_x, normalized_y)));
                    return event::Status::Captured;
                }
            } else {
                state.hovered = false;
            }
        }
        Event::PlatformSpecific(PlatformSpecific::Wayland(event::wayland::Event::DndOffer(DndOfferEvent::Leave))) => {
            state.hovered = false;
            state.dnd = None;
            if let Some(message) = widget.on_exit.clone() {
                shell.publish(message);
                return event::Status::Captured;
            }
        }
        Event::PlatformSpecific(PlatformSpecific::Wayland(event::wayland::Event::DndOffer(DndOfferEvent::DropPerformed))) => {
            state.hovered = false;
            state.dnd = None;
            if let Some(message) = widget.on_drop.clone() {
                shell.publish(message);
                return event::Status::Captured;
            }
        }
        Event::PlatformSpecific(PlatformSpecific::Wayland(event::wayland::Event::DndOffer(DndOfferEvent::ReadData(read_data)))) => {
            if let Some(message) = widget.on_read_data.as_ref() {
                shell.publish(message(read_data.clone()));
                return event::Status::Captured;
            }
        }
        Event::PlatformSpecific(PlatformSpecific::Wayland(event::wayland::Event::DndOffer(DndOfferEvent::SourceActions(actions)))) => {
            match state.dnd.as_mut() {
                Some((action, _)) => *action = *actions,
                None => state.dnd = Some((*actions, vec![])),
            };
            if let Some(message) = widget.on_source_actions.as_ref() {
                shell.publish(message(*actions));
                return event::Status::Captured;
            }
        }
        Event::PlatformSpecific(PlatformSpecific::Wayland(event::wayland::Event::DndOffer(DndOfferEvent::SelectedAction(action)))) => {
            match state.dnd.as_mut() {
                Some((dnd_action, _)) => *dnd_action = *action,
                None => state.dnd = Some((*action, vec![])),
            };
            if let Some(message) = widget.on_selected_action.as_ref() {
                shell.publish(message(*action));
                return event::Status::Captured;
            }
        }
        _ => {} 
    };
    event::Status::Ignored
}

/// Computes the layout of a [`DndListener`].
pub fn layout<Renderer>(
    renderer: &Renderer,
    limits: &layout::Limits,
    width: Length,
    height: Length,
    max_height: u32,
    max_width: u32,
    layout_content: impl FnOnce(&Renderer, &layout::Limits) -> layout::Node,
) -> layout::Node {
    let limits = limits
        .loose()
        .max_height(max_height)
        .max_width(max_width)
        .width(width)
        .height(height);

    let content = layout_content(renderer, &limits);
    let size = limits.resolve(content.size());

    layout::Node::with_children(size, vec![content])
}
