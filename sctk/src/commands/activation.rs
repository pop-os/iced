use iced_core::window::Id as SurfaceId;
use iced_runtime::{
    self,
    platform_specific::{self, wayland},
    Action, Task,
};

pub fn request_token<Message>(
    app_id: Option<String>,
    window: Option<SurfaceId>,
    to_message: impl FnOnce(Option<String>) -> Message + Send + Sync + 'static,
) -> Task<Option<String>> {
    Task::oneshot(|channel| {
        Action::PlatformSpecific(platform_specific::Action::Wayland(
            wayland::Action::Activation(
                wayland::activation::Action::RequestToken {
                    app_id,
                    window,
                    channel,
                },
            ),
        ))
    })
}

pub fn activate<Message>(window: SurfaceId, token: String) -> Task<Message> {
    Task::effect(Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::Activation(
            wayland::activation::Action::Activate { window, token },
        )),
    ))
}
