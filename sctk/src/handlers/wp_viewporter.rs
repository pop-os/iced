//! Handling of the wp-viewporter.

use std::marker::PhantomData;

use sctk::reexports::client::globals::{BindError, GlobalList};
use sctk::reexports::client::protocol::wl_surface::WlSurface;
use sctk::reexports::client::Dispatch;
use sctk::reexports::client::{
    delegate_dispatch, Connection, Proxy, QueueHandle,
};
use sctk::reexports::protocols::wp::viewporter::client::wp_viewport::WpViewport;
use sctk::reexports::protocols::wp::viewporter::client::wp_viewporter::WpViewporter;

use sctk::globals::GlobalData;

use crate::event_loop::state::SctkState;

/// Viewporter.
#[derive(Debug)]
pub struct ViewporterState<T> {
    viewporter: WpViewporter,
    _phantom: PhantomData<T>,
}

impl<T: 'static> ViewporterState<T> {
    /// Create new viewporter.
    pub fn new(
        globals: &GlobalList,
        queue_handle: &QueueHandle<SctkState<T>>,
    ) -> Result<Self, BindError> {
        let viewporter = globals.bind(queue_handle, 1..=1, GlobalData)?;
        Ok(Self {
            viewporter,
            _phantom: PhantomData,
        })
    }

    /// Get the viewport for the given object.
    pub fn get_viewport(
        &self,
        surface: &WlSurface,
        queue_handle: &QueueHandle<SctkState<T>>,
    ) -> WpViewport {
        self.viewporter
            .get_viewport(surface, queue_handle, GlobalData)
    }
}

impl<T: 'static> Dispatch<WpViewporter, GlobalData, SctkState<T>>
    for ViewporterState<T>
{
    fn event(
        _: &mut SctkState<T>,
        _: &WpViewporter,
        _: <WpViewporter as Proxy>::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<SctkState<T>>,
    ) {
        // No events.
    }
}

impl<T: 'static> Dispatch<WpViewport, GlobalData, SctkState<T>>
    for ViewporterState<T>
{
    fn event(
        _: &mut SctkState<T>,
        _: &WpViewport,
        _: <WpViewport as Proxy>::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<SctkState<T>>,
    ) {
        // No events.
    }
}

delegate_dispatch!(@<T: 'static> SctkState<T>: [WpViewporter: GlobalData] => ViewporterState<T>);
delegate_dispatch!(@<T: 'static> SctkState<T>: [WpViewport: GlobalData] => ViewporterState<T>);
