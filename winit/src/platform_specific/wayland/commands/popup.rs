//! Interact with the popups of your application.
use crate::core::window::Id as SurfaceId;
use iced_runtime::{
    self,
    platform_specific::{
        self,
        wayland::{self, popup::SctkPopupSettings},
    },
    task, Action, Task,
};

/// <https://wayland.app/protocols/wlr-layer-shell-unstable-v1#zwlr_layer_surface_v1:request:get_popup>
/// <https://wayland.app/protocols/xdg-shell#xdg_surface:request:get_popup>
pub fn get_popup<Message>(popup: SctkPopupSettings) -> Task<Message> {
    task::effect(Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::Popup(
            wayland::popup::Action::Popup { popup },
        )),
    ))
}

/// <https://wayland.app/protocols/xdg-shell#xdg_popup:request:reposition>
pub fn set_size<Message>(
    id: SurfaceId,
    width: u32,
    height: u32,
) -> Task<Message> {
    task::effect(Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::Popup(
            wayland::popup::Action::Size { id, width, height },
        )),
    ))
}

/// <https://wayland.app/protocols/xdg-shell#xdg_popup:request:destroy>
pub fn destroy_popup<Message>(id: SurfaceId) -> Task<Message> {
    task::effect(Action::PlatformSpecific(
        platform_specific::Action::Wayland(wayland::Action::Popup(
            wayland::popup::Action::Destroy { id },
        )),
    ))
}
