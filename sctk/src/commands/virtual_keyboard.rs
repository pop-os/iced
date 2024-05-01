//! Interact with the virtual keyboard from your application.
use iced_runtime::command::platform_specific::wayland::virtual_keyboard::ActionInner;
use iced_runtime::command::Command;
use iced_runtime::command::{
    self,
    platform_specific::{self, wayland},
};

pub fn virtual_keyboard_action<Message>(
    action_inner: ActionInner,
) -> Command<Message> {
    Command::single(command::Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::VirtualKeyboard(
            action_inner.into(),
        )),
    ))
}
