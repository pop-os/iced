// SPDX-License-Identifier: MPL-2.0-only
use cctk::sctk::{
    compositor::CompositorHandler,
    delegate_compositor,
    reexports::client::{
        Connection, QueueHandle,
        protocol::{wl_output, wl_surface},
    },
    shell::WaylandSurface,
};

use crate::{
    event_loop::state::receive_frame,
    platform_specific::wayland::{
        event_loop::state::SctkState,
        sctk_event::{LayerSurfaceEventVariant, SctkEvent},
    },
};

impl CompositorHandler for SctkState {
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
        _ = receive_frame(&mut self.frame_status, surface);
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
        surface: &wl_surface::WlSurface,
        output: &wl_output::WlOutput,
    ) {
        // Only layer surfaces report it for now: they are the ones the
        // compositor may place on an output the client didn't choose.
        if !self
            .layer_surfaces
            .iter()
            .any(|layer| layer.surface.wl_surface() == surface)
        {
            return;
        }

        // The compositor sends `enter` once per wl_output object the client
        // bound, and other code in the same process (e.g. libcosmic) binds
        // its own. Report only the one this event loop tracks, so the
        // output and its name match what `OutputEvent`s carry.
        if let Some(info) = self.output_state.info(output) {
            self.sctk_events.push(SctkEvent::LayerSurfaceEvent {
                variant: LayerSurfaceEventVariant::OutputEnter(
                    output.clone(),
                    info.name,
                ),
                id: surface.clone(),
            });
        }
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

delegate_compositor!(SctkState);
