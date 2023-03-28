//! Interact with the data device objects of your application.

use iced_native::{
    command::{
        self,
        platform_specific::{
            self,
            wayland::{
                self,
                data_device::{ActionInner, DndIcon},
            },
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
            wayland::data_device::ActionInner::SetSelection { mime_types }
                .into(),
        )),
    ))
}

/// unset the selection
pub fn unset_selection<Message>() -> Command<Message> {
    Command::single(command::Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::DataDevice(
            wayland::data_device::ActionInner::UnsetSelection.into(),
        )),
    ))
}

/// request the selection
/// This will trigger an event with a read pipe to read the data from.
pub fn request_selection<Message>(mime_type: String) -> Command<Message> {
    Command::single(command::Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::DataDevice(
            wayland::data_device::ActionInner::RequestSelectionData {
                mime_type,
            }
            .into(),
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
            wayland::data_device::ActionInner::StartInternalDnd {
                origin_id,
                icon_id,
            }
            .into(),
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
            wayland::data_device::ActionInner::StartDnd {
                mime_types,
                actions,
                origin_id,
                icon_id,
            }
            .into(),
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
            wayland::data_device::ActionInner::SetActions {
                preferred,
                accepted,
            }
            .into(),
        )),
    ))
}

/// Read drag and drop data. This will trigger an event with the data.
pub fn request_dnd_data<Message>(mime_type: String) -> Command<Message> {
    Command::single(command::Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::DataDevice(
            wayland::data_device::ActionInner::RequestDndData(mime_type).into(),
        )),
    ))
}

/// Set the DnD data in response to a request event.
pub fn send_dnd_data<Message>(data: Vec<u8>) -> Command<Message> {
    Command::single(command::Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::DataDevice(
            wayland::data_device::ActionInner::SendDndData { data }.into(),
        )),
    ))
}

/// Finished the drag and drop operation.
pub fn finish_dnd<Message>() -> Command<Message> {
    Command::single(command::Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::DataDevice(
            wayland::data_device::ActionInner::DndFinished.into(),
        )),
    ))
}

/// Cancel the drag and drop operation.
pub fn cancel_dnd<Message>() -> Command<Message> {
    Command::single(command::Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::DataDevice(
            wayland::data_device::ActionInner::DndCancelled.into(),
        )),
    ))
}

/// Run a generic drag action
pub fn action<Message>(action: ActionInner) -> Command<Message> {
    Command::single(command::Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::DataDevice(
            action.into(),
        )),
    ))
}
