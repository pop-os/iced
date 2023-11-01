use iced_runtime::command::Command;
use iced_runtime::command::{
    self,
    platform_specific::{self, wayland},
};
use iced_runtime::window::Id as SurfaceId;

pub fn request_token<Message>(
    app_id: Option<String>,
    window: Option<SurfaceId>,
    to_message: impl FnOnce(Option<String>) -> Message + Send + Sync + 'static,
) -> Command<Message> {
    Command::single(command::Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::Activation(
            wayland::activation::Action::RequestToken {
                app_id,
                window,
                message: Box::new(to_message),
            },
        )),
    ))
}

pub fn activate<Message>(window: SurfaceId, token: String) -> Command<Message> {
    Command::single(command::Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::Activation(
            wayland::activation::Action::Activate { window, token },
        )),
    ))
}
