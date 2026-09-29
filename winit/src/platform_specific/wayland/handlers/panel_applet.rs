use cctk::{
    cosmic_protocols::panel_applet::v1::client::{
        cosmic_panel_applet_manager_v1::CosmicPanelAppletManagerV1,
        cosmic_panel_applet_v1::{self, CosmicPanelAppletV1},
    },
    sctk::globals::GlobalData,
    wayland_client::{
        self, Connection, Dispatch, Proxy, QueueHandle, WEnum,
        globals::{BindError, GlobalList},
    },
};
use iced_futures::core::event::wayland::{
    AppletSize, PanelAnchor, PanelAppletSettings, PanelBackground,
};

use crate::{event_loop::state::SctkState, sctk_event::SctkEvent};

/// Settings of the panel this applet is embedded in.
#[derive(Debug, Clone)]
pub struct PanelAppletV1 {
    pub(crate) settings: CosmicPanelAppletV1,
    committed: PanelAppletSettings,
    pending: PanelAppletSettings,
    /// Whether any batch was completed yet.
    initialized: bool,
}

impl PanelAppletV1 {
    pub fn bind(
        globals: &GlobalList,
        qh: &QueueHandle<SctkState>,
    ) -> Result<PanelAppletV1, BindError> {
        let manager: CosmicPanelAppletManagerV1 =
            globals.bind(qh, 1..=1, GlobalData)?;
        let settings = manager.get_panel_applet(qh, ());
        Ok(PanelAppletV1 {
            settings,
            committed: PanelAppletSettings::default(),
            pending: PanelAppletSettings::default(),
            initialized: false,
        })
    }

    fn apply(
        &mut self,
        event: cosmic_panel_applet_v1::Event,
    ) -> Option<PanelAppletSettings> {
        match event {
            cosmic_panel_applet_v1::Event::PanelName { name } => {
                self.pending.panel_name = name;
            }
            cosmic_panel_applet_v1::Event::Output { output } => {
                self.pending.output = output;
            }
            cosmic_panel_applet_v1::Event::Anchor { anchor: value } => {
                self.pending.anchor = anchor(value)?;
            }
            cosmic_panel_applet_v1::Event::AppletSize { size, custom } => {
                self.pending.applet_size = applet_size(size, custom)?;
            }
            cosmic_panel_applet_v1::Event::Spacing { spacing } => {
                self.pending.spacing = spacing;
            }
            cosmic_panel_applet_v1::Event::Background {
                background: value,
                red,
                green,
                blue,
            } => {
                self.pending.background = background(value, red, green, blue)?;
            }
            cosmic_panel_applet_v1::Event::PaddingOverlap {
                padding_overlap,
            } => {
                self.pending.padding_overlap = padding_overlap as f32;
            }
            cosmic_panel_applet_v1::Event::Done => {
                if self.initialized && self.pending == self.committed {
                    return None;
                }
                self.initialized = true;
                self.committed = self.pending.clone();
                return Some(self.committed.clone());
            }
            _ => return None,
        }

        None
    }
}

fn applet_size(
    size: WEnum<cosmic_panel_applet_v1::AppletSize>,
    custom: u32,
) -> Option<AppletSize> {
    Some(match size {
        WEnum::Value(cosmic_panel_applet_v1::AppletSize::ExtraSmall) => {
            AppletSize::ExtraSmall
        }
        WEnum::Value(cosmic_panel_applet_v1::AppletSize::Small) => {
            AppletSize::Small
        }
        WEnum::Value(cosmic_panel_applet_v1::AppletSize::Medium) => {
            AppletSize::Medium
        }
        WEnum::Value(cosmic_panel_applet_v1::AppletSize::Large) => {
            AppletSize::Large
        }
        WEnum::Value(cosmic_panel_applet_v1::AppletSize::ExtraLarge) => {
            AppletSize::ExtraLarge
        }
        WEnum::Value(cosmic_panel_applet_v1::AppletSize::Custom) => {
            AppletSize::Custom(custom)
        }
        _ => return None,
    })
}

fn anchor(
    anchor: WEnum<cosmic_panel_applet_v1::Anchor>,
) -> Option<PanelAnchor> {
    Some(match anchor {
        WEnum::Value(cosmic_panel_applet_v1::Anchor::Left) => PanelAnchor::Left,
        WEnum::Value(cosmic_panel_applet_v1::Anchor::Right) => {
            PanelAnchor::Right
        }
        WEnum::Value(cosmic_panel_applet_v1::Anchor::Top) => PanelAnchor::Top,
        WEnum::Value(cosmic_panel_applet_v1::Anchor::Bottom) => {
            PanelAnchor::Bottom
        }
        _ => return None,
    })
}

fn background(
    background: WEnum<cosmic_panel_applet_v1::Background>,
    red: f64,
    green: f64,
    blue: f64,
) -> Option<PanelBackground> {
    Some(match background {
        WEnum::Value(cosmic_panel_applet_v1::Background::ThemeDefault) => {
            PanelBackground::ThemeDefault
        }
        WEnum::Value(cosmic_panel_applet_v1::Background::Dark) => {
            PanelBackground::Dark
        }
        WEnum::Value(cosmic_panel_applet_v1::Background::Light) => {
            PanelBackground::Light
        }
        WEnum::Value(cosmic_panel_applet_v1::Background::Color) => {
            PanelBackground::Color([red as f32, green as f32, blue as f32])
        }
        _ => return None,
    })
}

impl Dispatch<CosmicPanelAppletManagerV1, GlobalData, SctkState>
    for PanelAppletV1
{
    fn event(
        _: &mut SctkState,
        _: &CosmicPanelAppletManagerV1,
        _: <CosmicPanelAppletManagerV1 as Proxy>::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<SctkState>,
    ) {
        // The manager object has no events.
    }
}

impl Dispatch<CosmicPanelAppletV1, (), SctkState> for PanelAppletV1 {
    fn event(
        state: &mut SctkState,
        _: &CosmicPanelAppletV1,
        event: <CosmicPanelAppletV1 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<SctkState>,
    ) {
        let settings = state
            .panel_applet
            .as_mut()
            .and_then(|panel_applet| panel_applet.apply(event));

        if let Some(settings) = settings {
            state.sctk_events.push(SctkEvent::PanelApplet(settings));
        }
    }
}

wayland_client::delegate_dispatch!(SctkState: [CosmicPanelAppletManagerV1: GlobalData] => PanelAppletV1);
wayland_client::delegate_dispatch!(SctkState: [CosmicPanelAppletV1: ()] => PanelAppletV1);
