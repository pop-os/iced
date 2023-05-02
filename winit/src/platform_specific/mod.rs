//! Wayland specific shell
//!

use std::collections::HashMap;

use iced_graphics::{Compositor, compositor};
use iced_runtime::{core::window, platform_specific, user_interface};
#[cfg(all(feature = "wayland", target_os = "linux"))]
use sctk::reexports::client::Connection;

#[cfg(all(feature = "wayland", target_os = "linux"))]
pub mod wayland;

#[cfg(all(feature = "wayland", target_os = "linux"))]
pub use wayland::*;
#[cfg(all(feature = "wayland", target_os = "linux"))]
use wayland_backend::client::Backend;

use crate::{Program, WindowManager};

pub type UserInterfaces<'a, P> = HashMap<
    window::Id,
    user_interface::UserInterface<
        'a,
        <P as Program>::Message,
        <P as Program>::Theme,
        <P as Program>::Renderer,
    >,
    rustc_hash::FxBuildHasher,
>;

#[derive(Debug)]
pub enum Event {
    #[cfg(all(feature = "wayland", target_os = "linux"))]
    Wayland(sctk_event::SctkEvent),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SurfaceIdWrapper {
    LayerSurface(window::Id),
    Window(window::Id),
    Popup(window::Id),
    SessionLock(window::Id),
}
impl SurfaceIdWrapper {
    pub fn inner(&self) -> window::Id {
        match self {
            SurfaceIdWrapper::LayerSurface(id) => *id,
            SurfaceIdWrapper::Window(id) => *id,
            SurfaceIdWrapper::Popup(id) => *id,
            SurfaceIdWrapper::SessionLock(id) => *id,
        }
    }
}

#[derive(Debug, Default)]
pub struct PlatformSpecific {
    #[cfg(all(feature = "wayland", target_os = "linux"))]
    wayland: WaylandSpecific,
}

impl PlatformSpecific {
    pub(crate) fn send_action(
        &mut self,
        action: iced_runtime::platform_specific::Action,
    ) {
        match action {
            #[cfg(all(feature = "wayland", target_os = "linux"))]
            platform_specific::Action::Wayland(a) => {
                self.send_wayland(wayland::Action::Action(a));
            }
        }
    }

    pub(crate) fn send_ready(&mut self) {
        #[cfg(all(feature = "wayland", target_os = "linux"))]
        {
            self.send_wayland(wayland::Action::Ready);
        }
    }

    pub(crate) fn update_subsurfaces(
        &mut self,
        id: window::Id,
        window: &dyn winit::window::Window,
    ) {
        #[cfg(all(feature = "wayland", target_os = "linux"))]
        {
            use sctk::reexports::client::{
                Proxy, protocol::wl_surface::WlSurface,
            };
            use wayland_backend::client::ObjectId;

            let Ok(backend) = window.rwh_06_display_handle().display_handle()
            else {
                log::error!("No display handle");
                return;
            };

            let conn = match backend.as_raw() {
                raw_window_handle::RawDisplayHandle::Wayland(
                    wayland_display_handle,
                ) => {
                    let backend = unsafe {
                        Backend::from_foreign_display(
                            wayland_display_handle.display.as_ptr().cast(),
                        )
                    };
                    Connection::from_backend(backend)
                }
                _ => {
                    return;
                }
            };

            let Ok(raw) = window.rwh_06_window_handle().window_handle() else {
                log::error!("Invalid window handle {id:?}");
                return;
            };
            let wl_surface = match raw.as_raw() {
                raw_window_handle::RawWindowHandle::Wayland(
                    wayland_window_handle,
                ) => {
                    let res = unsafe {
                        ObjectId::from_ptr(
                            WlSurface::interface(),
                            wayland_window_handle.surface.as_ptr().cast(),
                        )
                    };
                    let Ok(id) = res else {
                        log::error!(
                            "Could not create WlSurface Id from window"
                        );
                        return;
                    };
                    let Ok(surface) = WlSurface::from_id(&conn, id) else {
                        log::error!("Could not create WlSurface from Id");
                        return;
                    };
                    surface
                }

                _ => {
                    log::error!("Unexpected window handle type");
                    return;
                }
            };
            self.wayland.update_subsurfaces(id, &wl_surface);
        }
    }
}

pub(crate) fn handle_event<'a, P>(
    e: Event,
    events: &mut Vec<(Option<window::Id>, iced_runtime::core::Event)>,
    platform_specific: &mut PlatformSpecific,
    program: &'a crate::program::Instance<P>,
    compositor: &mut <<P as Program>::Renderer as compositor::Default>::Compositor,
    window_manager: &mut WindowManager<
        P,
        <<P as Program>::Renderer as compositor::Default>::Compositor,
    >,
    user_interfaces: &mut UserInterfaces<'a, P>,
    clipboard: &mut crate::Clipboard,
    #[cfg(feature = "a11y")] adapters: &mut std::collections::HashMap<
        window::Id,
        (u64, iced_accessibility::accesskit_winit::Adapter),
    >,
) where
    P: Program,
{
    match e {
        #[cfg(all(feature = "wayland", target_os = "linux"))]
        Event::Wayland(e) => {
            platform_specific.wayland.handle_event(
                e,
                events,
                program,
                compositor,
                window_manager,
                user_interfaces,
                clipboard,
                #[cfg(feature = "a11y")]
                adapters,
            );
        }
    }
}
