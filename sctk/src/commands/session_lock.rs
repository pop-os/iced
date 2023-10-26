use iced_runtime::command::Command;
use iced_runtime::command::{
    self,
    platform_specific::{self, wayland},
};
use iced_runtime::window::Id as SurfaceId;
use sctk::reexports::client::protocol::wl_output::WlOutput;

use std::marker::PhantomData;

pub fn lock<Message>() -> Command<Message> {
    Command::single(command::Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::SessionLock(
            wayland::session_lock::Action::Lock,
        )),
    ))
}

pub fn unlock<Message>() -> Command<Message> {
    Command::single(command::Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::SessionLock(
            wayland::session_lock::Action::Unlock,
        )),
    ))
}

pub fn get_lock_surface<Message>(
    id: SurfaceId,
    output: WlOutput,
) -> Command<Message> {
    Command::single(command::Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::SessionLock(
            wayland::session_lock::Action::LockSurface {
                id,
                output,
                _phantom: PhantomData,
            },
        )),
    ))
}

pub fn destroy_lock_surface<Message>(id: SurfaceId) -> Command<Message> {
    Command::single(command::Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::SessionLock(
            wayland::session_lock::Action::DestroyLockSurface { id },
        )),
    ))
}
