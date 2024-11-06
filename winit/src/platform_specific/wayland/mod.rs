pub mod commands;
pub mod conversion;
pub(crate) mod event_loop;
pub(crate) mod handlers;
pub mod keymap;
pub mod sctk_event;
pub mod subsurface_widget;
pub mod winit_window;

use super::{PlatformSpecific, SurfaceIdWrapper};
use crate::{Control, Program, WindowManager};

use crate::platform_specific::UserInterfaces;
use cctk::sctk::reexports::calloop;
use cctk::sctk::reexports::client::protocol::wl_surface::WlSurface;
use cctk::sctk::seat::keyboard::Modifiers;
use cursor_icon::CursorIcon;
use iced_futures::futures::channel::mpsc;
use iced_graphics::{Compositor, compositor};
use iced_runtime::core::window;
use sctk_event::SctkEvent;
use std::{collections::HashMap, sync::Arc};
use subsurface_widget::{SubsurfaceInstance, SubsurfaceState};
use wayland_backend::client::ObjectId;
use winit::event_loop::OwnedDisplayHandle;

pub(crate) enum Action {
    Action(iced_runtime::platform_specific::wayland::Action),
    SetCursor(CursorIcon),
    RequestRedraw(ObjectId),
    TrackWindow(Arc<dyn winit::window::Window>, window::Id),
    RemoveWindow(window::Id),
    Dropped(SurfaceIdWrapper),
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
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct WaylandSpecific {
    winit_event_sender: Option<mpsc::UnboundedSender<Control>>,
    proxy: Option<winit::event_loop::EventLoopProxy>,
    sender: Option<calloop::channel::Sender<Action>>,
    display_handle: Option<OwnedDisplayHandle>,
    modifiers: Modifiers,
    surface_ids: HashMap<ObjectId, SurfaceIdWrapper>,
    destroyed_surface_ids: HashMap<ObjectId, SurfaceIdWrapper>,
    subsurface_ids: HashMap<ObjectId, (i32, i32, window::Id)>,
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
            log::error!("Failed to process wayland Action.");
        }
    }
}

impl WaylandSpecific {
    pub(crate) fn handle_event<'a, P>(
        &mut self,
        e: SctkEvent,
        events: &mut Vec<(Option<window::Id>, iced_runtime::core::Event)>,
        program: &'a crate::program::Instance<P>,
        compositor: &mut <<P as Program>::Renderer as compositor::Default>::Compositor,
        window_manager: &mut WindowManager<
            P,
            <<P as Program>::Renderer as compositor::Default>::Compositor,
        >,
        user_interfaces: &mut UserInterfaces<'a, P>,
        clipboard: &mut crate::Clipboard,
        #[cfg(feature = "a11y")] adapters: &mut HashMap<
            window::Id,
            (u64, iced_accessibility::accesskit_winit::Adapter),
        >,
    ) where
        P: Program,
    {
        let Self {
            winit_event_sender,
            proxy,
            sender,
            display_handle,
            surface_ids,
            destroyed_surface_ids,
            subsurface_ids,
            modifiers,
            subsurface_state,
            surface_subsurfaces,
        } = self;

        match e {
            sctk_event => {
                let Some(sender) = sender.as_ref() else {
                    log::error!("Missing calloop sender");
                    return Default::default();
                };
                let Some(event_sender) = winit_event_sender.as_ref() else {
                    log::error!("Missing control sender");
                    return Default::default();
                };
                let Some(proxy) = proxy.as_ref() else {
                    log::error!("Missing event loop proxy");
                    return Default::default();
                };

                sctk_event.process(
                    modifiers,
                    program,
                    compositor,
                    window_manager,
                    surface_ids,
                    subsurface_ids,
                    sender,
                    event_sender,
                    proxy,
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
            id,
            &mut self.subsurface_ids,
            wl_surface,
            surface_subsurfaces,
            &subsurfaces,
        );
    }
}
