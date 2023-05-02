use iced_core::window::Id as SurfaceId;
use iced_runtime::{
    self,
    platform_specific::{self, wayland},
    Action, Task,
};
use sctk::reexports::client::protocol::wl_output::WlOutput;

use std::marker::PhantomData;

pub fn lock<Message>() -> Task<Message> {
    Task::effect(Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::SessionLock(
            wayland::session_lock::Action::Lock,
        )),
    ))
}

pub fn unlock<Message>() -> Task<Message> {
    Task::effect(Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::SessionLock(
            wayland::session_lock::Action::Unlock,
        )),
    ))
}

pub fn get_lock_surface<Message>(
    id: SurfaceId,
    output: WlOutput,
) -> Task<Message> {
    Task::effect(Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::SessionLock(
            wayland::session_lock::Action::LockSurface { id, output },
        )),
    ))
}

pub fn destroy_lock_surface<Message>(id: SurfaceId) -> Task<Message> {
    Task::effect(Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::SessionLock(
            wayland::session_lock::Action::DestroyLockSurface { id },
        )),
    ))
}
