use cctk::{
    cosmic_protocols::toplevel_management::v1::client::zcosmic_toplevel_manager_v1,
    toplevel_info::{ToplevelInfoHandler, ToplevelInfoState},
    toplevel_management::ToplevelManagerHandler,
    wayland_client::{self, WEnum},
};
use wayland_client::{Connection, QueueHandle};
use wayland_protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_handle_v1;

use crate::event_loop::state::SctkState;

impl ToplevelManagerHandler for SctkState {
    fn toplevel_manager_state(
        &mut self,
    ) -> &mut cctk::toplevel_management::ToplevelManagerState {
        self.toplevel_manager.as_mut().unwrap()
    }

    fn capabilities(
        &mut self,
        _conn: &Connection,
        _: &QueueHandle<Self>,
        _capabilities: Vec<
            WEnum<zcosmic_toplevel_manager_v1::ZcosmicToplelevelManagementCapabilitiesV1>,
        >,
    ) {
        // TODO
    }
}

impl ToplevelInfoHandler for SctkState {
    fn toplevel_info_state(&mut self) -> &mut ToplevelInfoState {
        self.toplevel_info.as_mut().unwrap()
    }

    fn new_toplevel(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _toplevel: &ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1,
    ) {
        // TODO
    }

    fn update_toplevel(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _toplevel: &ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1,
    ) {
        // TODO
    }

    fn toplevel_closed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _toplevel: &ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1,
    ) {
        // TODO
    }
}

cctk::delegate_toplevel_info!(SctkState);
cctk::delegate_toplevel_manager!(SctkState);
