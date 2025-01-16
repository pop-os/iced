//! Access the clipboard.

use std::any::Any;

use dnd::{DndDestinationRectangle, DndSurface};
use iced_core::clipboard::DndSource;
use window_clipboard::mime::{AllowedMimeTypes, AsMimeTypes};

use crate::{oneshot, task, Action, Task};

/// An action to be performed on the system.
pub enum DndAction {
    /// Register a Dnd destination.
    RegisterDndDestination {
        /// The surface to register.
        surface: DndSurface,
        /// The rectangles to register.
        rectangles: Vec<DndDestinationRectangle>,
    },
    /// End a Dnd operation.
    EndDnd,
    /// Peek the current Dnd operation.
    PeekDnd(
        String,
        oneshot::Sender<Option<(Vec<u8>, String)>>,
        // Box<dyn Fn(Option<(Vec<u8>, String)>) -> T + Send + 'static>,
    ),
    /// Set the action of the Dnd operation.
    SetAction(dnd::DndAction),
}

impl std::fmt::Debug for DndAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RegisterDndDestination {
                surface,
                rectangles,
            } => f
                .debug_struct("RegisterDndDestination")
                .field("surface", surface)
                .field("rectangles", rectangles)
                .finish(),
            Self::EndDnd => f.write_str("EndDnd"),
            Self::PeekDnd(mime, _) => {
                f.debug_struct("PeekDnd").field("mime", mime).finish()
            }
            Self::SetAction(a) => f.debug_tuple("SetAction").field(a).finish(),
        }
    }
}

/// Read the current contents of the Dnd operation.
pub fn peek_dnd<T: AllowedMimeTypes>() -> Task<Option<T>> {
    task::oneshot(|tx| {
        Action::Dnd(DndAction::PeekDnd(
            T::allowed()
                .first()
                .map_or_else(String::new, std::string::ToString::to_string),
            tx,
        ))
    })
    .map(|data| data.and_then(|data| T::try_from(data).ok()))
}

/// Register a Dnd destination.
pub fn register_dnd_destination<Message>(
    surface: DndSurface,
    rectangles: Vec<DndDestinationRectangle>,
) -> Task<Message> {
    task::effect(Action::Dnd(DndAction::RegisterDndDestination {
        surface,
        rectangles,
    }))
}

/// End a Dnd operation.
pub fn end_dnd<Message>() -> Task<Message> {
    task::effect(Action::Dnd(DndAction::EndDnd))
}

/// Set the action of the Dnd operation.
pub fn set_action<Message>(a: dnd::DndAction) -> Task<Message> {
    task::effect(Action::Dnd(DndAction::SetAction(a)))
}
