//! Interact with the virtual keyboard from your application.
use std::marker::PhantomData;

use iced_runtime::command::platform_specific::wayland::input_method::ActionInner;
use iced_runtime::command::platform_specific::wayland::input_method_popup::InputMethodPopupSettings;
use iced_runtime::command::Command;
use iced_runtime::command::{
    self,
    platform_specific::{self, wayland},
};

pub fn input_method_action<Message>(
    action_inner: ActionInner,
) -> Command<Message> {
    Command::single(command::Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::InputMethod(
            action_inner.into(),
        )),
    ))
}

pub fn get_input_method_popup<Message>(
    builder: InputMethodPopupSettings,
) -> Command<Message> {
    Command::single(command::Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::InputMethodPopup(
            wayland::input_method_popup::Action::Popup {
                settings: builder,
                _phantom: PhantomData,
            },
        )),
    ))
}

pub fn show_input_method_popup<Message>() -> Command<Message> {
    Command::single(command::Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::InputMethodPopup(
            wayland::input_method_popup::Action::ShowPopup,
        )),
    ))
}

pub fn hide_input_method_popup<Message>() -> Command<Message> {
    Command::single(command::Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::InputMethodPopup(
            wayland::input_method_popup::Action::HidePopup,
        )),
    ))
}
