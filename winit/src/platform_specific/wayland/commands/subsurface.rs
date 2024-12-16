use crate::core::window::Id as SurfaceId;
pub use cctk::sctk::shell::wlr_layer::{Anchor, KeyboardInteractivity, Layer};
use iced_runtime::{
    self,
    platform_specific::{
        self,
        wayland::{self, subsurface::SctkSubsurfaceSettings},
    },
    task, Action, Task,
};

pub fn get_subsurface<Message>(
    subsurface: SctkSubsurfaceSettings,
) -> Task<Message> {
    task::effect(Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::Subsurface(
            wayland::subsurface::Action::Subsurface { subsurface },
        )),
    ))
}

pub fn destroy_subsurface<Message>(id: SurfaceId) -> Task<Message> {
    task::effect(Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::Subsurface(
            wayland::subsurface::Action::Destroy { id },
        )),
    ))
}
