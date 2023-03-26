//! Interact with the data device objects of your application.

use iced_native::{
    command::{
        self,
        platform_specific::{
            self,
            wayland::{self, data_device::DndIcon},
        },
    },
    widget, window, Command,
};
use sctk::reexports::client::protocol::wl_data_device_manager::DndAction;

/// Set the selection. When a client asks for the selection, an event will be delivered to the
/// client with the fd to write the data to.
pub fn set_selection<Message>(mime_types: Vec<String>) -> Command<Message> {
    Command::single(command::Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::DataDevice(
            wayland::data_device::Action::SetSelection {
                mime_types,
                _phantom: std::marker::PhantomData,
            },
        )),
    ))
}

/// unset the selection
pub fn unset_selection<Message>() -> Command<Message> {
    Command::single(command::Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::DataDevice(
            wayland::data_device::Action::UnsetSelection,
        )),
    ))
}

/// request the selection
/// This will trigger an event with a read pipe to read the data from.
pub fn request_selection<Message>(mime_type: String) -> Command<Message> {
    Command::single(command::Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::DataDevice(
            wayland::data_device::Action::RequestSelectionData { mime_type },
        )),
    ))
}

/// start an internal drag and drop operation. Events will only be delivered to the same client.
/// The client is responsible for data transfer.
pub fn start_internal_drag<Message>(
    origin_id: window::Id,
    icon_id: Option<window::Id>,
) -> Command<Message> {
    Command::single(command::Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::DataDevice(
            wayland::data_device::Action::StartInternalDnd {
                origin_id,
                icon_id,
            },
        )),
    ))
}

/// Start a drag and drop operation. When a client asks for the selection, an event will be delivered
/// to the client with the fd to write the data to.
pub fn start_drag<Message>(
    mime_types: Vec<String>,
    actions: DndAction,
    origin_id: window::Id,
    icon_id: Option<DndIcon>,
) -> Command<Message> {
    Command::single(command::Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::DataDevice(
            wayland::data_device::Action::StartDnd {
                mime_types,
                actions,
                origin_id,
                icon_id,
            },
        )),
    ))
}

/// Set accepted and preferred drag and drop actions.
pub fn set_actions<Message>(
    preferred: DndAction,
    accepted: DndAction,
) -> Command<Message> {
    Command::single(command::Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::DataDevice(
            wayland::data_device::Action::SetActions {
                preferred,
                accepted,
            },
        )),
    ))
}

/// Read drag and drop data. This will trigger an event with a read pipe to read the data from.
pub fn request_dnd_data<Message>(
    mime_type: String,
    action: DndAction,
    widget_id: Option<widget::Id>,
) -> Command<Message> {
    Command::single(command::Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::DataDevice(
            wayland::data_device::Action::RequestDndData {
                id: widget_id,
                mime_type,
                action,
            },
        )),
    ))
}

/// Finished the drag and drop operation.
pub fn finish_dnd<Message>() -> Command<Message> {
    Command::single(command::Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::DataDevice(
            wayland::data_device::Action::DndFinished,
        )),
    ))
}

/// Cancel the drag and drop operation.
pub fn cancel_dnd<Message>() -> Command<Message> {
    Command::single(command::Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::DataDevice(
            wayland::data_device::Action::DndCancelled,
        )),
    ))
}
