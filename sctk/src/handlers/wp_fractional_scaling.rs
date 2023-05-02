// From: https://github.com/rust-windowing/winit/blob/master/src/platform_impl/linux/wayland/types/wp_fractional_scaling.rs
//! Handling of the fractional scaling.

use std::marker::PhantomData;

use sctk::reexports::client::globals::{BindError, GlobalList};
use sctk::reexports::client::protocol::wl_surface::WlSurface;
use sctk::reexports::client::Dispatch;
use sctk::reexports::client::{delegate_dispatch, Connection, Proxy, QueueHandle};
use sctk::reexports::protocols::wp::fractional_scale::v1::client::wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1;
use sctk::reexports::protocols::wp::fractional_scale::v1::client::wp_fractional_scale_v1::Event as FractionalScalingEvent;
use sctk::reexports::protocols::wp::fractional_scale::v1::client::wp_fractional_scale_v1::WpFractionalScaleV1;

use sctk::globals::GlobalData;

use crate::event_loop::state::SctkState;

/// The scaling factor denominator.
const SCALE_DENOMINATOR: f64 = 120.;

/// Fractional scaling manager.
#[derive(Debug)]
pub struct FractionalScalingManager<T> {
    manager: WpFractionalScaleManagerV1,

    _phantom: PhantomData<T>,
}

pub struct FractionalScaling {
    /// The surface used for scaling.
    surface: WlSurface,
}

impl<T: 'static> FractionalScalingManager<T> {
    /// Create new viewporter.
    pub fn new(
        globals: &GlobalList,
        queue_handle: &QueueHandle<SctkState<T>>,
    ) -> Result<Self, BindError> {
        let manager = globals.bind(queue_handle, 1..=1, GlobalData)?;
        Ok(Self {
            manager,
            _phantom: PhantomData,
        })
    }

    pub fn fractional_scaling(
        &self,
        surface: &WlSurface,
        queue_handle: &QueueHandle<SctkState<T>>,
    ) -> WpFractionalScaleV1 {
        let data = FractionalScaling {
            surface: surface.clone(),
        };
        self.manager
            .get_fractional_scale(surface, queue_handle, data)
    }
}

impl<T: 'static> Dispatch<WpFractionalScaleManagerV1, GlobalData, SctkState<T>>
    for FractionalScalingManager<T>
{
    fn event(
        _: &mut SctkState<T>,
        _: &WpFractionalScaleManagerV1,
        _: <WpFractionalScaleManagerV1 as Proxy>::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<SctkState<T>>,
    ) {
        // No events.
    }
}

impl<T: 'static> Dispatch<WpFractionalScaleV1, FractionalScaling, SctkState<T>>
    for FractionalScalingManager<T>
{
    fn event(
        state: &mut SctkState<T>,
        _: &WpFractionalScaleV1,
        event: <WpFractionalScaleV1 as Proxy>::Event,
        data: &FractionalScaling,
        _: &Connection,
        _: &QueueHandle<SctkState<T>>,
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

delegate_dispatch!(@<T: 'static> SctkState<T>: [WpFractionalScaleManagerV1: GlobalData] => FractionalScalingManager<T>);
delegate_dispatch!(@<T: 'static> SctkState<T>: [WpFractionalScaleV1: FractionalScaling] => FractionalScalingManager<T>);
