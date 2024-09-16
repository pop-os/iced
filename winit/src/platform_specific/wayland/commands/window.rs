//! Interact with the window of your application.

use iced_runtime::{
    self,
    core::{self},
    platform_specific::{
        self,
        wayland::{self, window::SctkWindowSettings},
    },
    task, Action, Task,
};

pub fn get_window<Message>(builder: SctkWindowSettings) -> Task<Message> {
    task::effect(Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::Window(
            wayland::window::Action::Window { builder },
        )),
    ))
}

// TODO Ashley refactor to use regular window events maybe...
/// close the window
pub fn close_window<Message>(id: core::window::Id) -> Task<Message> {
    task::effect(Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::Window(
            wayland::window::Action::Destroy(id),
        )),
    ))
}

pub fn set_app_id_window<Message>(
    id: core::window::Id,
    app_id: String,
) -> Task<Message> {
    task::effect(Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::Window(
            wayland::window::Action::AppId { id, app_id },
        )),
    ))
}
