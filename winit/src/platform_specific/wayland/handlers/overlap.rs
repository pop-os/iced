use cctk::{
    cosmic_protocols::overlap_notify::v1::client::{
        zcosmic_overlap_notification_v1::{self, ZcosmicOverlapNotificationV1},
        zcosmic_overlap_notify_v1::ZcosmicOverlapNotifyV1,
    }, sctk::shell::wlr_layer::Layer, wayland_client::{
        self, event_created_child,
        globals::{BindError, GlobalList},
        protocol::wl_surface::WlSurface,
        Connection, Dispatch, Proxy, QueueHandle,
    }, wayland_protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1
};
use cctk::sctk::globals::GlobalData;
use iced_futures::core::Rectangle;

use crate::{event_loop::state::SctkState, sctk_event::SctkEvent};

#[derive(Debug, Clone)]
pub struct OverlapNotifyV1 {
    pub(crate) notify: ZcosmicOverlapNotifyV1,
}

impl OverlapNotifyV1 {
    pub fn bind(
        globals: &GlobalList,
        qh: &QueueHandle<SctkState>,
    ) -> Result<OverlapNotifyV1, BindError> {
        let notify = globals.bind_singleton(qh, 1..=1, GlobalData)?;
        Ok(OverlapNotifyV1 { notify })
    }
}

impl Dispatch<ZcosmicOverlapNotifyV1, SctkState>
    for GlobalData
{
    fn event(
        &self,
        _: &mut SctkState,
        _: &ZcosmicOverlapNotifyV1,
        _: <ZcosmicOverlapNotifyV1 as Proxy>::Event,
        _: &Connection,
        _: &QueueHandle<SctkState>,
    ) {
    }
}

pub struct OverlapNotificationV1 {
    pub surface: WlSurface,
}

impl Dispatch<ZcosmicOverlapNotificationV1, SctkState>
    for OverlapNotificationV1
{
    fn event(
        &self,
        state: &mut SctkState,
        _: &ZcosmicOverlapNotificationV1,
        event: <ZcosmicOverlapNotificationV1 as Proxy>::Event,
        _: &Connection,
        _: &QueueHandle<SctkState>,
    ) {
        let surface = self.surface.clone();

        state.sctk_events.push(match event {
            zcosmic_overlap_notification_v1::Event::ToplevelEnter {
                toplevel,
                x,
                y,
                width,
                height,
            } => SctkEvent::OverlapToplevelAdd {
                surface,
                toplevel,
                logical_rect: Rectangle::new(
                    (x as f32, y as f32).into(),
                    (width as f32, height as f32).into(),
                ),
            },
            zcosmic_overlap_notification_v1::Event::ToplevelLeave {
                toplevel,
            } => {
                SctkEvent::OverlapToplevelRemove { surface, toplevel }
            }
            zcosmic_overlap_notification_v1::Event::LayerEnter {
                identifier,
                namespace,
                exclusive,
                layer,
                x,
                y,
                width,
                height,
            } => SctkEvent::OverlapLayerAdd { surface, namespace, identifier, exclusive, layer: match layer {
                cctk::sctk::reexports::protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1::Layer::Background => Some(Layer::Background),
                cctk::sctk::reexports::protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1::Layer::Bottom => Some(Layer::Bottom),
                cctk::sctk::reexports::protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1::Layer::Top => Some(Layer::Top),
                cctk::sctk::reexports::protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1::Layer::Overlay => Some(Layer::Overlay),
                _ => Default::default(),
            }, logical_rect: Rectangle::new(
                (x as f32, y as f32).into(),
                (width as f32, height as f32).into(),
            ), },
            zcosmic_overlap_notification_v1::Event::LayerLeave {
                identifier,
            } => SctkEvent::OverlapLayerRemove { identifier, surface },
            _ => unimplemented!(),
        });
    }
}
