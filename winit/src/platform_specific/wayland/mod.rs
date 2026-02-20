pub mod commands;
pub mod conversion;
pub(crate) mod event_loop;
pub(crate) mod handlers;
pub mod keymap;
pub mod sctk_event;
pub mod subsurface_widget;
pub mod winit_window;

use super::{PlatformSpecific, SurfaceIdWrapper};
use crate::program::{Control, Program, WindowManager};

use cctk::sctk::reexports::calloop;
use cctk::sctk::reexports::client::protocol::wl_surface::WlSurface;
use cctk::sctk::seat::keyboard::Modifiers;
use iced_futures::futures::channel::mpsc;
use iced_graphics::Compositor;
use iced_runtime::core::{window, Vector};
use iced_runtime::Debug;
use raw_window_handle::{DisplayHandle, HasDisplayHandle, HasWindowHandle};
use raw_window_handle::{HasRawDisplayHandle, RawWindowHandle};
use sctk_event::SctkEvent;
use std::{collections::HashMap, sync::Arc};
use subsurface_widget::{SubsurfaceInstance, SubsurfaceState};
use wayland_backend::client::ObjectId;
use wayland_client::{Connection, Proxy};
use winit::dpi::Size;
use winit::event_loop::OwnedDisplayHandle;
use winit::window::{CursorIcon, ImePurpose};

pub(crate) enum Action {
    Action(iced_runtime::platform_specific::wayland::Action),
    SetCursor(CursorIcon),
    RequestRedraw(ObjectId),
    TrackWindow(Arc<dyn winit::window::Window>, window::Id),
    ResizeWindow(window::Id),
    RemoveWindow(window::Id),
    Dropped(SurfaceIdWrapper),
    SubsurfaceResize(window::Id, Size),
    SetImeAllowed(bool),
    SetImeCursorArea(i32, i32, i32, i32),
    SetImePurpose(ImePurpose),
}

impl std::fmt::Debug for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Action(arg0) => f.debug_tuple("Action").field(arg0).finish(),
            Self::SetCursor(arg0) => {
                f.debug_tuple("SetCursor").field(arg0).finish()
            }
            Self::RequestRedraw(arg0) => {
                f.debug_tuple("RequestRedraw").field(arg0).finish()
            }
            Self::TrackWindow(_arg0, arg1) => {
                f.debug_tuple("TrackWindow").field(arg1).finish()
            }
            Self::RemoveWindow(arg0) => {
                f.debug_tuple("RemoveWindow").field(arg0).finish()
            }
            Self::Dropped(_surface_id_wrapper) => write!(f, "Dropped"),
            Self::SubsurfaceResize(id, size) => f
                .debug_tuple("SubsurfaceResize")
                .field(id)
                .field(size)
                .finish(),
            Self::ResizeWindow(arg0) => {
                f.debug_tuple("ResizeWindow").field(arg0).finish()
            }
            Self::SetImeAllowed(allowed) => {
                f.debug_tuple("SetImeAllowed").field(allowed).finish()
            }
            Self::SetImeCursorArea(x, y, width, height) => f
                .debug_tuple("SetImeCursorArea")
                .field(x)
                .field(y)
                .field(width)
                .field(height)
                .finish(),
            Self::SetImePurpose(purpose) => {
                f.debug_tuple("SetImePurpose").field(purpose).finish()
            }
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct WaylandSpecific {
    winit_event_sender: Option<mpsc::UnboundedSender<Control>>,
    proxy: Option<winit::event_loop::EventLoopProxy>,
    sender: Option<calloop::channel::Sender<Action>>,
    display_handle: Option<OwnedDisplayHandle>,
    conn: Option<Connection>,
    modifiers: Modifiers,
    surface_ids: HashMap<ObjectId, SurfaceIdWrapper>,
    subsurface_state: Option<SubsurfaceState>,
    surface_subsurfaces: HashMap<window::Id, Vec<SubsurfaceInstance>>,
}

impl PlatformSpecific {
    pub(crate) fn with_wayland(
        mut self,
        tx: mpsc::UnboundedSender<Control>,
        raw: winit::event_loop::EventLoopProxy,
        display: OwnedDisplayHandle,
    ) -> Self {
        self.wayland.winit_event_sender = Some(tx);
        self.wayland.conn = match display.raw_display_handle() {
            Ok(raw_window_handle::RawDisplayHandle::Wayland(
                wayland_display_handle,
            )) => {
                let backend = unsafe {
                    wayland_backend::client::Backend::from_foreign_display(
                        wayland_display_handle.display.as_ptr().cast(),
                    )
                };
                Some(Connection::from_backend(backend))
            }
            Ok(_) => {
                log::warn!("Non-Wayland display handle");
                None
            }
            Err(_) => {
                log::error!("No display handle");
                None
            }
        };
        self.wayland.display_handle = Some(display);
        self.wayland.proxy = Some(raw);
        // TODO remove this
        self.wayland.sender =
            crate::platform_specific::event_loop::SctkEventLoop::new(
                self.wayland.winit_event_sender.clone().unwrap(),
                self.wayland.proxy.clone().unwrap(),
                self.wayland.display_handle.clone().unwrap(),
            )
            .ok();
        self
    }

    pub(crate) fn send_wayland(&mut self, action: Action) {
        if self.wayland.sender.is_none()
            && self.wayland.winit_event_sender.is_some()
            && self.wayland.display_handle.is_some()
            && self.wayland.proxy.is_some()
        {
            self.wayland.sender =
                crate::platform_specific::event_loop::SctkEventLoop::new(
                    self.wayland.winit_event_sender.clone().unwrap(),
                    self.wayland.proxy.clone().unwrap(),
                    self.wayland.display_handle.clone().unwrap(),
                )
                .ok();
        }

        if let Some(tx) = self.wayland.sender.as_ref() {
            _ = tx.send(action);
        } else {
            log::info!("Did not process wayland Action.");
        }
    }
}

impl WaylandSpecific {
    pub(crate) fn conn(&self) -> Option<&Connection> {
        self.conn.as_ref()
    }

    pub(crate) fn handle_event<'a, P, C>(
        &mut self,
        e: SctkEvent,
        events: &mut Vec<(Option<window::Id>, iced_runtime::core::Event)>,
        program: &'a P,
        compositor: &mut C,
        window_manager: &mut WindowManager<P, C>,
        debug: &mut Debug,
        user_interfaces: &mut super::UserInterfaces<'a, P>,
        clipboard: &mut crate::Clipboard,
        #[cfg(feature = "a11y")] adapters: &mut HashMap<
            window::Id,
            (u64, iced_accessibility::accesskit_winit::Adapter),
        >,
    ) where
        P: 'static + Program,
        C: Compositor<Renderer = P::Renderer>,
        P::Message: 'static,
    {
        let Self {
            winit_event_sender,
            proxy,
            sender,
            display_handle,
            conn,
            surface_ids,
            modifiers,
            subsurface_state,
            surface_subsurfaces,
        } = self;

        match e {
            sctk_event => {
                let Some(sender) = sender.as_ref() else {
                    log::warn!("Missing calloop sender");
                    return Default::default();
                };
                let Some(event_sender) = winit_event_sender.as_ref() else {
                    log::warn!("Missing control sender");
                    return Default::default();
                };
                let Some(proxy) = proxy.as_ref() else {
                    log::warn!("Missing event loop proxy");
                    return Default::default();
                };

                sctk_event.process(
                    modifiers,
                    program,
                    compositor,
                    window_manager,
                    surface_ids,
                    sender,
                    event_sender,
                    proxy,
                    debug,
                    user_interfaces,
                    events,
                    clipboard,
                    subsurface_state,
                    #[cfg(feature = "a11y")]
                    adapters,
                );
            }
        };
    }

    pub(crate) fn retain_subsurfaces<F: Fn(window::Id) -> bool>(
        &mut self,
        keep: F,
    ) {
        self.surface_subsurfaces.retain(|k, v| keep(*k))
    }

    pub(crate) fn clear_subsurface_list(&mut self) {
        let _ = crate::subsurface_widget::take_subsurfaces();
    }

    pub(crate) fn update_subsurfaces(
        &mut self,
        id: window::Id,
        wl_surface: &WlSurface,
    ) {
        let subsurfaces = crate::subsurface_widget::take_subsurfaces();
        let mut entry = self.surface_subsurfaces.entry(id);
        let surface_subsurfaces = entry.or_default();
        let Some(subsurface_state) = self.subsurface_state.as_mut() else {
            return;
        };

        subsurface_state.update_subsurfaces(
            wl_surface,
            surface_subsurfaces,
            &subsurfaces,
        );
    }

    pub(crate) fn create_surface(
        &mut self,
    ) -> Option<Box<dyn HasWindowHandle + Send + Sync + 'static>> {
        if let Some(subsurface_state) = self.subsurface_state.as_mut() {
            let wl_surface = subsurface_state.create_surface();
            Some(Box::new(Window(wl_surface)))
        } else {
            None
        }
    }

    pub(crate) fn update_surface_shm(
        &mut self,
        window: &dyn HasWindowHandle,
        width: u32,
        height: u32,
        scale: f64,
        data: &[u8],
        offset: Vector,
    ) {
        if let Some(subsurface_state) = self.subsurface_state.as_mut() {
            if let RawWindowHandle::Wayland(window) =
                window.window_handle().unwrap().as_raw()
            {
                let id = unsafe {
                    ObjectId::from_ptr(
                        WlSurface::interface(),
                        window.surface.as_ptr().cast(),
                    )
                    .unwrap()
                };
                let surface =
                    WlSurface::from_id(self.conn.as_ref().unwrap(), id)
                        .unwrap();
                subsurface_state.update_surface_shm(
                    &surface, width, height, scale, data, offset,
                );
            }
        }
    }
}

struct Window(WlSurface);

impl HasWindowHandle for Window {
    fn window_handle(
        &self,
    ) -> Result<
        raw_window_handle::WindowHandle<'_>,
        raw_window_handle::HandleError,
    > {
        Ok(unsafe {
            raw_window_handle::WindowHandle::borrow_raw(
                raw_window_handle::RawWindowHandle::Wayland(
                    raw_window_handle::WaylandWindowHandle::new(
                        std::ptr::NonNull::new(self.0.id().as_ptr() as *mut _)
                            .unwrap(),
                    ),
                ),
            )
        })
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        self.0.destroy();
    }
}
