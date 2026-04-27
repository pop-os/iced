use iced_futures::core::window;
use iced_runtime::{
    self, Action, Task,
    platform_specific::{
        self,
        wayland::{self, CornerRadius},
    },
    task,
};

pub fn corner_radius(id: window::Id, corners: Option<CornerRadius>) -> Task<()> {
    task::oneshot(|_| {
        Action::PlatformSpecific(platform_specific::Action::Wayland(
            wayland::Action::RoundedCorners(id, corners),
        ))
    })
}
