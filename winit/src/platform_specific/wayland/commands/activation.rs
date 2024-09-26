use crate::core::window::Id as SurfaceId;
use iced_runtime::{
    self,
    platform_specific::{self, wayland},
    task, Action, Task,
};

pub fn request_token(
    app_id: Option<String>,
    window: Option<SurfaceId>,
) -> Task<Option<String>> {
    task::oneshot(|channel| {
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
    task::effect(Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::Activation(
            wayland::activation::Action::Activate { window, token },
        )),
    ))
}
