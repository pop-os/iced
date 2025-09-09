use iced_runtime::{
    self,
    platform_specific::{self, wayland},
    task, Action, Task,
};

pub fn inhibit_shortcuts(inhibit: bool) -> Task<()> {
    task::oneshot(|_| {
        Action::PlatformSpecific(platform_specific::Action::Wayland(
            wayland::Action::InhibitShortcuts(inhibit),
        ))
    })
}
