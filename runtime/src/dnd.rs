//! Access the clipboard.

use std::any::Any;

use dnd::{DndDestinationRectangle, DndSurface};
use iced_core::clipboard::DndSource;
use iced_futures::MaybeSend;
use window_clipboard::mime::{AllowedMimeTypes, AsMimeTypes};

use crate::{command, Command};

/// An action to be performed on the system.
pub enum DndAction<T> {
    /// Register a Dnd destination.
    RegisterDndDestination {
        /// The surface to register.
        surface: DndSurface,
        /// The rectangles to register.
        rectangles: Vec<DndDestinationRectangle>,
    },
    /// Start a Dnd operation.
    StartDnd {
        /// Whether the Dnd operation is internal.
        internal: bool,
        /// The source surface of the Dnd operation.
        source_surface: Option<DndSource>,
        /// The icon surface of the Dnd operation.
        icon_surface: Option<Box<dyn Any>>,
        /// The content of the Dnd operation.
        content: Box<dyn AsMimeTypes + Send + 'static>,
        /// The actions of the Dnd operation.
        actions: dnd::DndAction,
    },
    /// End a Dnd operation.
    EndDnd,
    /// Peek the current Dnd operation.
    PeekDnd(String, Box<dyn Fn(Option<(Vec<u8>, String)>) -> T>),
    /// Set the action of the Dnd operation.
    SetAction(dnd::DndAction),
}

impl<T> std::fmt::Debug for DndAction<T> {
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
            Self::StartDnd {
                internal,
                source_surface,
                icon_surface,
                content: _,
                actions,
            } => f
                .debug_struct("StartDnd")
                .field("internal", internal)
                .field("source_surface", source_surface)
                .field("icon_surface", icon_surface)
                .field("actions", actions)
                .finish(),
            Self::EndDnd => f.write_str("EndDnd"),
            Self::PeekDnd(mime, _) => {
                f.debug_struct("PeekDnd").field("mime", mime).finish()
            }
            Self::SetAction(a) => f.debug_tuple("SetAction").field(a).finish(),
        }
    }
}

impl<T> DndAction<T> {
    /// Maps the output of a [`DndAction`] using the provided closure.
    pub fn map<A>(
        self,
        f: impl Fn(T) -> A + 'static + MaybeSend + Sync,
    ) -> DndAction<A>
    where
        T: 'static,
    {
        match self {
            Self::PeekDnd(m, o) => {
                DndAction::PeekDnd(m, Box::new(move |d| f(o(d))))
            }
            Self::EndDnd => DndAction::EndDnd,
            Self::SetAction(a) => DndAction::SetAction(a),
            Self::StartDnd {
                internal,
                source_surface,
                icon_surface,
                content,
                actions,
            } => DndAction::StartDnd {
                internal,
                source_surface,
                icon_surface,
                content,
                actions,
            },
            Self::RegisterDndDestination {
                surface,
                rectangles,
            } => DndAction::RegisterDndDestination {
                surface,
                rectangles,
            },
        }
    }
}

/// Read the current contents of the Dnd operation.
pub fn peek_dnd<T: AllowedMimeTypes + Send + Sync + 'static, Message>(
    f: impl Fn(Option<T>) -> Message + 'static,
) -> Command<Message> {
    Command::single(command::Action::Dnd(DndAction::PeekDnd(
        T::allowed()
            .get(0)
            .map_or_else(|| String::new(), |s| s.to_string()),
        Box::new(move |d| f(d.and_then(|d| T::try_from(d).ok()))),
    )))
}

/// Register a Dnd destination.
pub fn register_dnd_destination<Message>(
    surface: DndSurface,
    rectangles: Vec<DndDestinationRectangle>,
) -> Command<Message> {
    Command::single(command::Action::Dnd(DndAction::RegisterDndDestination {
        surface,
        rectangles,
    }))
}

/// Start a Dnd operation.
pub fn start_dnd<Message>(
    internal: bool,
    source_surface: Option<DndSource>,
    icon_surface: Option<Box<dyn Any>>,
    content: Box<dyn AsMimeTypes + Send + 'static>,
    actions: dnd::DndAction,
) -> Command<Message> {
    Command::single(command::Action::Dnd(DndAction::StartDnd {
        internal,
        source_surface,
        icon_surface,
        content,
        actions,
    }))
}

/// End a Dnd operation.
pub fn end_dnd<Message>() -> Command<Message> {
    Command::single(command::Action::Dnd(DndAction::EndDnd))
}

/// Set the action of the Dnd operation.
pub fn set_action<Message>(a: dnd::DndAction) -> Command<Message> {
    Command::single(command::Action::Dnd(DndAction::SetAction(a)))
}
