use std::collections::HashMap;

use cctk::sctk;
use iced_runtime::core::Rectangle;
use iced_runtime::platform_specific::wayland::Action;
use sctk::globals::GlobalData;
use sctk::reexports::client::globals::{BindError, GlobalList};
use sctk::reexports::client::protocol::wl_surface::WlSurface;
use sctk::reexports::client::{Connection, Dispatch, Proxy, QueueHandle, delegate_dispatch};
use wayland_protocols::ext::background_effect::v1::client::ext_background_effect_manager_v1::{Capability, Event, ExtBackgroundEffectManagerV1};
use wayland_protocols::ext::background_effect::v1::client::ext_background_effect_surface_v1::ExtBackgroundEffectSurfaceV1;

use crate::event_loop::state::SctkState;
use crate::window;

#[derive(Debug, Clone)]
pub struct ExtBackgroundEffectManager {
    manager: ExtBackgroundEffectManagerV1,
    capabilities: Capability,
    queued_blur_actions: HashMap<window::Id, Option<Vec<Rectangle>>>,
}

impl ExtBackgroundEffectManager {
    pub fn new(
        globals: &GlobalList,
        queue_handle: &QueueHandle<SctkState>,
    ) -> Result<Self, BindError> {
        let manager = globals.bind(queue_handle, 1..=1, GlobalData)?;
        Ok(Self {
            manager,
            capabilities: Capability::empty(),
            queued_blur_actions: HashMap::new(),
        })
    }

    pub fn blur(
        &mut self,
        surface: &WlSurface,
        queue_handle: &QueueHandle<SctkState>,
    ) -> ExtBackgroundEffectSurfaceV1 {
        self.manager
            .get_background_effect(surface, queue_handle, ())
    }

    pub fn enqueue(&mut self, id: window::Id, rects: Option<Vec<Rectangle>>) {
        _ = self.queued_blur_actions.insert(id, rects);
    }

    pub fn capabilities(&self) -> Capability {
        self.capabilities
    }
}

impl Dispatch<ExtBackgroundEffectManagerV1, GlobalData, SctkState>
    for ExtBackgroundEffectManager
{
    fn event(
        state: &mut SctkState,
        _: &ExtBackgroundEffectManagerV1,
        event: <ExtBackgroundEffectManagerV1 as Proxy>::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<SctkState>,
    ) {
        match event {
            Event::Capabilities { flags } => match flags {
                wayland_client::WEnum::Value(capability) => {
                    let mut queued_actions = Vec::new();
                    if let Some(bg_effect_mgr) =
                        state.ext_background_effect_manager.as_mut()
                    {
                        bg_effect_mgr.capabilities = capability;
                        queued_actions =
                            bg_effect_mgr.queued_blur_actions.drain().collect();
                    }
                    for (id, rects) in queued_actions {
                        _ = state.handle_action(Action::BlurSurface(id, rects));
                    }
                }
                wayland_client::WEnum::Unknown(u) => {
                    log::warn!("Unknown value: {u:?}");
                }
            },
            e => {
                log::warn!("Ignored event {e:?}");
            }
        }
    }
}

impl Dispatch<ExtBackgroundEffectSurfaceV1, (), SctkState>
    for ExtBackgroundEffectManager
{
    fn event(
        _: &mut SctkState,
        _: &ExtBackgroundEffectSurfaceV1,
        _: <ExtBackgroundEffectSurfaceV1 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<SctkState>,
    ) {
        // There is no event
    }
}

delegate_dispatch!(SctkState: [ExtBackgroundEffectManagerV1: GlobalData] => ExtBackgroundEffectManager);
delegate_dispatch!(SctkState: [ExtBackgroundEffectSurfaceV1: ()] => ExtBackgroundEffectManager);
