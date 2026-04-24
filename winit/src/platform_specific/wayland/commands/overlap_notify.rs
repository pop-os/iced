use iced_futures::core::window::Id;
use iced_runtime::{
    Action, Task,
    platform_specific::{self, wayland},
    task,
};

/// Request subscription for overlap notification events on the surface
pub fn overlap_notify<Message>(id: Id, enable: bool) -> Task<Message> {
    task::effect(Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::OverlapNotify(id, enable)),
    ))
}
