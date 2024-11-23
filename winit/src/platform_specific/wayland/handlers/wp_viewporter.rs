//! Handling of the wp-viewporter.

use cctk::sctk::reexports::client::globals::{BindError, GlobalList};
use cctk::sctk::reexports::client::protocol::wl_surface::WlSurface;
use cctk::sctk::reexports::client::Dispatch;
use cctk::sctk::reexports::client::{
    delegate_dispatch, Connection, Proxy, QueueHandle,
};
use cctk::sctk::reexports::protocols::wp::viewporter::client::wp_viewport::WpViewport;
use cctk::sctk::reexports::protocols::wp::viewporter::client::wp_viewporter::WpViewporter;

use cctk::sctk::globals::GlobalData;

use crate::platform_specific::wayland::event_loop::state::SctkState;

/// Viewporter.
#[derive(Debug)]
pub struct ViewporterState {
    viewporter: WpViewporter,
}

impl ViewporterState {
    /// Create new viewporter.
    pub fn new(
        globals: &GlobalList,
        queue_handle: &QueueHandle<SctkState>,
    ) -> Result<Self, BindError> {
        let viewporter = globals.bind(queue_handle, 1..=1, GlobalData)?;
        Ok(Self { viewporter })
    }

    /// Get the viewport for the given object.
    pub fn get_viewport(
        &self,
        surface: &WlSurface,
        queue_handle: &QueueHandle<SctkState>,
    ) -> WpViewport {
        self.viewporter
            .get_viewport(surface, queue_handle, GlobalData)
    }
}

impl Dispatch<WpViewporter, GlobalData, SctkState> for ViewporterState {
    fn event(
        _: &mut SctkState,
        _: &WpViewporter,
        _: <WpViewporter as Proxy>::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<SctkState>,
    ) {
        // No events.
    }
}

impl Dispatch<WpViewport, GlobalData, SctkState> for ViewporterState {
    fn event(
        _: &mut SctkState,
        _: &WpViewport,
        _: <WpViewport as Proxy>::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<SctkState>,
    ) {
        // No events.
    }
}

delegate_dispatch!(SctkState: [WpViewporter: GlobalData] => ViewporterState);
delegate_dispatch!(SctkState: [WpViewport: GlobalData] => ViewporterState);
