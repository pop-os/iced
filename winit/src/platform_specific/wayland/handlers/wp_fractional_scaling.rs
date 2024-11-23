// From: https://github.com/rust-windowing/winit/blob/master/src/platform_impl/linux/wayland/types/wp_fractional_scaling.rs
//! Handling of the fractional scaling.

use cctk::sctk::reexports::client::globals::{BindError, GlobalList};
use cctk::sctk::reexports::client::protocol::wl_surface::WlSurface;
use cctk::sctk::reexports::client::Dispatch;
use cctk::sctk::reexports::client::{delegate_dispatch, Connection, Proxy, QueueHandle};
use cctk::sctk::reexports::protocols::wp::fractional_scale::v1::client::wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1;
use cctk::sctk::reexports::protocols::wp::fractional_scale::v1::client::wp_fractional_scale_v1::Event as FractionalScalingEvent;
use cctk::sctk::reexports::protocols::wp::fractional_scale::v1::client::wp_fractional_scale_v1::WpFractionalScaleV1;

use cctk::sctk::globals::GlobalData;

use crate::platform_specific::wayland::event_loop::state::SctkState;

/// The scaling factor denominator.
const SCALE_DENOMINATOR: f64 = 120.;

/// Fractional scaling manager.
#[derive(Debug)]
pub struct FractionalScalingManager {
    manager: WpFractionalScaleManagerV1,
}

pub struct FractionalScaling {
    /// The surface used for scaling.
    surface: WlSurface,
}

impl FractionalScalingManager {
    /// Create new viewporter.
    pub fn new(
        globals: &GlobalList,
        queue_handle: &QueueHandle<SctkState>,
    ) -> Result<Self, BindError> {
        let manager = globals.bind(queue_handle, 1..=1, GlobalData)?;
        Ok(Self { manager })
    }

    pub fn fractional_scaling(
        &self,
        surface: &WlSurface,
        queue_handle: &QueueHandle<SctkState>,
    ) -> WpFractionalScaleV1 {
        let data = FractionalScaling {
            surface: surface.clone(),
        };
        self.manager
            .get_fractional_scale(surface, queue_handle, data)
    }
}

impl Dispatch<WpFractionalScaleManagerV1, GlobalData, SctkState>
    for FractionalScalingManager
{
    fn event(
        _: &mut SctkState,
        _: &WpFractionalScaleManagerV1,
        _: <WpFractionalScaleManagerV1 as Proxy>::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<SctkState>,
    ) {
        // No events.
    }
}

impl Dispatch<WpFractionalScaleV1, FractionalScaling, SctkState>
    for FractionalScalingManager
{
    fn event(
        state: &mut SctkState,
        _: &WpFractionalScaleV1,
        event: <WpFractionalScaleV1 as Proxy>::Event,
        data: &FractionalScaling,
        _: &Connection,
        _: &QueueHandle<SctkState>,
    ) {
        if let FractionalScalingEvent::PreferredScale { scale } = event {
            state.scale_factor_changed(
                &data.surface,
                scale as f64 / SCALE_DENOMINATOR,
                false,
            );
        }
    }
}

delegate_dispatch!(SctkState: [WpFractionalScaleManagerV1: GlobalData] => FractionalScalingManager);
delegate_dispatch!(SctkState: [WpFractionalScaleV1: FractionalScaling] => FractionalScalingManager);
