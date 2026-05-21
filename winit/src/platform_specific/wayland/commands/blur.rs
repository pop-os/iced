use iced_futures::core::window;
use iced_runtime::{
    self, Action, Task,
    platform_specific::{self, wayland},
    task,
};

pub fn blur(
    id: window::Id,
    blur: Option<Vec<iced_runtime::core::Rectangle>>,
) -> Task<()> {
    task::oneshot(|_| {
        Action::PlatformSpecific(platform_specific::Action::Wayland(
            wayland::Action::BlurSurface(id, blur),
        ))
    })
}
