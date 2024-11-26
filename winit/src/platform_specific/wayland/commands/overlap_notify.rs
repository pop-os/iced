use iced_futures::core::window::Id;
use iced_runtime::{
    platform_specific::{self, wayland},
    task, Action, Task,
};

/// Request subscription for overlap notification events on the surface
pub fn overlap_notify<Message>(id: Id, enable: bool) -> Task<Message> {
    task::effect(Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::OverlapNotify(
            id, enable,
        )),
    ))
}
