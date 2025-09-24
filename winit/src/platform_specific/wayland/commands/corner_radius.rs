use iced_futures::core::{border::Radius, window};
use iced_runtime::{
    self,
    platform_specific::{
        self,
        wayland::{self, CornerRadius},
    },
    task, Action, Task,
};

pub fn corner_radius(
    id: window::Id,
    corners: Option<CornerRadius>,
) -> Task<()> {
    task::oneshot(|_| {
        Action::PlatformSpecific(platform_specific::Action::Wayland(
            wayland::Action::RoundedCorners(id, corners),
        ))
    })
}
