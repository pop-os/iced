// SPDX-License-Identifier: MPL-2.0-only
use sctk::{
    compositor::CompositorHandler,
    delegate_compositor,
    reexports::client::{
        protocol::{wl_output, wl_surface},
        Connection, QueueHandle,
    },
};
use std::fmt::Debug;

use crate::event_loop::state::SctkState;

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
        // TODO time; map subsurface to parent:w
        self.frame_events.push((surface.clone(), 0));
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {
        // TODO
        // this is not required
    }

    fn surface_enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }
}

delegate_compositor!(@<T: 'static + Debug> SctkState<T>);
