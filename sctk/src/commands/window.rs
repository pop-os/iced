//! Interact with the window of your application.
use std::marker::PhantomData;

use iced_runtime::{
    self,
    core::window::Mode,
    platform_specific::{
        self,
        wayland::{self, window::SctkWindowSettings},
    },
    window, Action, Task,
};

pub fn get_window<Message>(builder: SctkWindowSettings) -> Task<Message> {
    Task::effect(Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::Window(
            wayland::window::Action::Window { builder },
        )),
    ))
}

// TODO Ashley refactor to use regular window events maybe...
/// close the window
pub fn close_window<Message>(id: iced_core::window::Id) -> Task<Message> {
    Task::effect(Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::Window(
            wayland::window::Action::Destroy(id),
        )),
    ))
}

/// Resizes the window to the given logical dimensions.
pub fn resize_window<Message>(
    id: iced_core::window::Id,
    width: u32,
    height: u32,
) -> Task<Message> {
    Task::effect(Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::Window(
            wayland::window::Action::Size { id, width, height },
        )),
    ))
}

pub fn start_drag_window<Message>(id: iced_core::window::Id) -> Task<Message> {
    Task::effect(Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::Window(
            wayland::window::Action::InteractiveMove { id },
        )),
    ))
}

pub fn maximize<Message>(
    id: iced_core::window::Id,
    maximized: bool,
) -> Task<Message> {
    Task::effect(Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::Window(
            if maximized {
                wayland::window::Action::Maximize { id }
            } else {
                wayland::window::Action::UnsetMaximize { id }
            },
        )),
    ))
}

pub fn toggle_maximize<Message>(id: iced_core::window::Id) -> Task<Message> {
    Task::effect(Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::Window(
            wayland::window::Action::ToggleMaximized { id },
        )),
    ))
}

pub fn set_app_id_window<Message>(
    id: iced_core::window::Id,
    app_id: String,
) -> Task<Message> {
    Task::effect(Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::Window(
            wayland::window::Action::AppId { id, app_id },
        )),
    ))
}

/// Sets the [`Mode`] of the window.
pub fn set_mode_window<Message>(
    id: iced_core::window::Id,
    mode: Mode,
) -> Task<Message> {
    Task::effect(Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::Window(
            wayland::window::Action::Mode(id, mode),
        )),
    ))
}
