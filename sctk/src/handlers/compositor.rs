// SPDX-License-Identifier: MPL-2.0-only
use sctk::{
    compositor::CompositorHandler,
    delegate_compositor,
    reexports::client::{protocol::wl_surface, Connection, Proxy, QueueHandle},
    shell::WaylandSurface,
};
use std::fmt::Debug;

use crate::{event_loop::state::SctkState, sctk_event::SctkEvent};

impl<T: Debug> CompositorHandler for SctkState<T> {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        surface: &wl_surface::WlSurface,
        new_factor: i32,
    ) {
        self.scale_factor_changed(surface, new_factor as f64, true);
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
        self.frame_events.push(surface.clone());
    }
}

delegate_compositor!(@<T: 'static + Debug> SctkState<T>);
