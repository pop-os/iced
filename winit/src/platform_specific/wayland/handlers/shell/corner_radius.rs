use cctk::{sctk, cosmic_protocols::{
    corner_radius::v1::client::{
        cosmic_corner_radius_manager_v1::CosmicCornerRadiusManagerV1,
        cosmic_corner_radius_toplevel_v1::CosmicCornerRadiusToplevelV1,
    },
    overlap_notify::v1::client::zcosmic_overlap_notification_v1::ZcosmicOverlapNotificationV1,
}};
use sctk::reexports::{
    client::{Connection, Dispatch, Proxy},

};

use crate::event_loop::state::SctkState;
use crate::platform_specific::wayland::SctkEvent;

impl Dispatch<CosmicCornerRadiusManagerV1, ()> for SctkState {
    fn event(
        _state: &mut Self,
        _proxy: &CosmicCornerRadiusManagerV1,
        _event: <CosmicCornerRadiusManagerV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &sctk::reexports::client::QueueHandle<Self>,
    ) {}
}

impl
    Dispatch<
        CosmicCornerRadiusToplevelV1,
        (),
    > for SctkState
{
    fn event(
        state: &mut Self,
        _proxy: &CosmicCornerRadiusToplevelV1,
        event: <CosmicCornerRadiusToplevelV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &sctk::reexports::client::QueueHandle<Self>,
    ) {
        match event {
            _ => unimplemented!()
        }
    }
}
