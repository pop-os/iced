use crate::{widget, window};
use core::fmt;
use iced_futures::MaybeSend;
use sctk::reexports::client::protocol::wl_data_device_manager::DndAction;
use std::{any::Any, marker::PhantomData};

/// DataDevice Action
pub enum Action<T> {
    /// Indicate that you are setting the selection and will respond to events
    /// with data of the advertised mime types.
    SetSelection {
        /// The mime types that the selection can be converted to.
        mime_types: Vec<String>,
        /// Phantom data to allow the user to pass in a custom type.
        _phantom: PhantomData<T>,
    },
    /// Unset the selection.
    UnsetSelection,
    /// Send the selection data.
    SendSelectionData {
        /// The data to send.
        data: Vec<u8>,
    },
    /// Request the selection data from the clipboard.
    RequestSelectionData {
        /// The mime type that the selection should be converted to.
        mime_type: String,
    },
    /// Start a drag and drop operation. When a client asks for the selection, an event will be delivered
    /// This is used for internal drags, where the client is the source of the drag.
    /// The client will be resposible for data transfer.
    StartInternalDnd {
        /// The window id of the window that is the source of the drag.
        origin_id: window::Id,
        /// An optional window id for the cursor icon surface.
        icon_id: Option<window::Id>,
    },
    /// Start a drag and drop operation. When a client asks for the selection, an event will be delivered
    StartDnd {
        /// The mime types that the dnd data can be converted to.
        mime_types: Vec<String>,
        /// The actions that the client supports.
        actions: DndAction,
        /// The window id of the window that is the source of the drag.
        origin_id: window::Id,
        /// An optional window id for the cursor icon surface.
        icon_id: Option<DndIcon>,
    },
    /// Set accepted and preferred drag and drop actions.
    SetActions {
        /// The preferred action.
        preferred: DndAction,
        /// The accepted actions.
        accepted: DndAction,
    },
    /// Read the Drag and Drop data. An event will be delivered with a pipe to read the data from.
    RequestDndData {
        /// id of the widget which is requesting the drag
        id: Option<widget::Id>,
        /// The mime type that the selection should be converted to.
        mime_type: String,
        /// The action that the client supports.
        action: DndAction,
    },
    /// Send the drag and drop data.
    SendDndData {
        /// The data to send.
        data: Vec<u8>,
    },
    /// The drag and drop operation has finished.
    DndFinished,
    /// The drag and drop operation has been cancelled.
    DndCancelled,
}

/// DndIcon
#[derive(Debug)]
pub enum DndIcon {
    /// The id of the widget which will draw the dnd icon.
    Widget(window::Id, Box<dyn Send + Any>),
    /// A custom icon.
    Custom(window::Id),
}

impl<T> Action<T> {
    /// Maps the output of a window [`Action`] using the provided closure.
    pub fn map<A>(
        self,
        _: impl Fn(T) -> A + 'static + MaybeSend + Sync,
    ) -> Action<A>
    where
        T: 'static,
    {
        match self {
            Action::UnsetSelection => Action::UnsetSelection,
            Action::SetSelection {
                mime_types,
                _phantom,
            } => Action::SetSelection {
                mime_types,
                _phantom: PhantomData,
            },
            Action::RequestSelectionData { mime_type } => {
                Action::RequestSelectionData { mime_type }
            }
            Action::SendSelectionData { data } => {
                Action::SendSelectionData { data }
            }
            Action::StartInternalDnd { origin_id, icon_id } => {
                Action::StartInternalDnd { origin_id, icon_id }
            }
            Action::StartDnd {
                mime_types,
                actions,
                origin_id,
                icon_id,
            } => Action::StartDnd {
                mime_types,
                actions,
                origin_id,
                icon_id,
            },
            Action::SetActions {
                preferred,
                accepted,
            } => Action::SetActions {
                preferred,
                accepted,
            },
            Action::RequestDndData {
                id,
                mime_type,
                action,
            } => Action::RequestDndData {
                id,
                mime_type,
                action,
            },
            Action::SendDndData { data } => Action::SendDndData { data },
            Action::DndFinished => Action::DndFinished,
            Action::DndCancelled => Action::DndCancelled,
        }
    }
}

impl<T> fmt::Debug for Action<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SetSelection { mime_types, .. } => {
                f.debug_tuple("SetSelection").field(mime_types).finish()
            }
            Self::UnsetSelection => f.debug_tuple("UnsetSelection").finish(),
            Self::RequestSelectionData { mime_type } => {
                f.debug_tuple("RequestSelection").field(mime_type).finish()
            }
            Self::SendSelectionData { data } => {
                f.debug_tuple("SendSelectionData").field(data).finish()
            }
            Self::StartInternalDnd { origin_id, icon_id } => f
                .debug_tuple("StartInternalDnd")
                .field(origin_id)
                .field(icon_id)
                .finish(),
            Self::StartDnd {
                mime_types,
                actions,
                origin_id,
                icon_id,
            } => f
                .debug_tuple("StartDnd")
                .field(mime_types)
                .field(actions)
                .field(origin_id)
                .field(icon_id)
                .finish(),
            Self::SetActions {
                preferred,
                accepted,
            } => f
                .debug_tuple("SetActions")
                .field(preferred)
                .field(accepted)
                .finish(),
            Self::RequestDndData {
                mime_type,
                action,
                id,
            } => f
                .debug_tuple("RequestDndData")
                .field(mime_type)
                .field(action)
                .field(id)
                .finish(),
            Self::SendDndData { data } => {
                f.debug_tuple("SendDndData").field(data).finish()
            }
            Self::DndFinished => f.debug_tuple("DndFinished").finish(),
            Self::DndCancelled => f.debug_tuple("DndCancelled").finish(),
        }
    }
}
