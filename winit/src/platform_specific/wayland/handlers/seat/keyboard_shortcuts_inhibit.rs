use cctk::sctk;
use sctk::reexports::{
    client::{Connection, Dispatch, Proxy},
    protocols::wp::keyboard_shortcuts_inhibit::{
        self, zv1::client::zwp_keyboard_shortcuts_inhibitor_v1,
    },
};

use crate::event_loop::state::SctkState;
use crate::platform_specific::wayland::SctkEvent;

impl Dispatch<keyboard_shortcuts_inhibit::zv1::client::zwp_keyboard_shortcuts_inhibit_manager_v1::ZwpKeyboardShortcutsInhibitManagerV1, SctkState> for () {
    fn event(
        &self,
        _state: &mut SctkState,
        _proxy: &keyboard_shortcuts_inhibit::zv1::client::zwp_keyboard_shortcuts_inhibit_manager_v1::ZwpKeyboardShortcutsInhibitManagerV1,
        _event: <keyboard_shortcuts_inhibit::zv1::client::zwp_keyboard_shortcuts_inhibit_manager_v1::ZwpKeyboardShortcutsInhibitManagerV1 as Proxy>::Event,
        _conn: &Connection,
        _qhandle: &sctk::reexports::client::QueueHandle<SctkState>,
    ) {}
}

impl
    Dispatch<
        zwp_keyboard_shortcuts_inhibitor_v1::ZwpKeyboardShortcutsInhibitorV1,
        SctkState,
    > for ()
{
    fn event(
        &self,
        state: &mut SctkState,
        _proxy: &zwp_keyboard_shortcuts_inhibitor_v1::ZwpKeyboardShortcutsInhibitorV1,
        event: <zwp_keyboard_shortcuts_inhibitor_v1::ZwpKeyboardShortcutsInhibitorV1 as Proxy>::Event,
        _conn: &Connection,
        _qhandle: &sctk::reexports::client::QueueHandle<SctkState>,
    ) {
        match event {
            zwp_keyboard_shortcuts_inhibitor_v1::Event::Active => {
                state.sctk_events.push(SctkEvent::ShortcutsInhibited(true));
                state.inhibited = true;
            }
            zwp_keyboard_shortcuts_inhibitor_v1::Event::Inactive => {
                state.sctk_events.push(SctkEvent::ShortcutsInhibited(false));
                state.inhibited = false;
                if let Some(inhibitor) = state.inhibitor.take() {
                    inhibitor.destroy();
                }
            }
            _ => unimplemented!(),
        }
    }
}
