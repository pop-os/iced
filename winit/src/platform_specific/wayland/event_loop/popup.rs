use std::sync::Mutex;
use std::sync::Arc;
use cctk::sctk::shell::xdg::popup::PopupConfigure;
use cctk::sctk::shell::xdg::popup::Popup;
use iced_runtime::{platform_specific, core::window::Id};
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_protocols::wp::fractional_scale::v1::client::wp_fractional_scale_v1::WpFractionalScaleV1;

use crate::event_loop::state::Common;
use crate::event_loop::state::SctkPopupData;

#[derive(Debug, Default)]
pub struct PopupManager {
    chains: Vec<Vec<SctkPopup>>,
}

impl PopupManager {
    pub(crate) fn popups(&self) -> impl Iterator<Item = &SctkPopup> {
        self.chains.iter().map(|c| c.iter()).flatten()
    }

    pub(crate) fn chain_for_popup_mut(
        &mut self,
        id: &WlSurface,
    ) -> Option<(&mut Vec<SctkPopup>, usize)> {
        let pos = self.chains.iter().enumerate().find_map(move |(pos, c)| {
            if let Some(pop_pos) =
                c.into_iter().position(|p| p.popup.wl_surface() == id)
            {
                Some((pos, pop_pos))
            } else {
                None
            }
        });

        pos.map(|pos| (&mut self.chains[pos.0], pos.1))
    }

    pub(crate) fn chain_for_popup(
        &self,
        id: &WlSurface,
    ) -> Option<(&Vec<SctkPopup>, usize)> {
        let pos = self.chains.iter().enumerate().find_map(move |(pos, c)| {
            if let Some(pop_pos) =
                c.into_iter().position(|p| p.popup.wl_surface() == id)
            {
                Some((pos, pop_pos))
            } else {
                None
            }
        });

        pos.map(|pos| (&self.chains[pos.0], pos.1))
    }

    pub(crate) fn popup(&self, id: &WlSurface) -> Option<&SctkPopup> {
        self.chains.iter().find_map(move |c| {
            c.into_iter().find(|p| p.popup.wl_surface() == id)
        })
    }

    pub(crate) fn popup_mut(
        &mut self,
        id: &WlSurface,
    ) -> Option<&mut SctkPopup> {
        self.chains.iter_mut().find_map(move |c| {
            c.into_iter().find(|p| p.popup.wl_surface() == id)
        })
    }

    pub(crate) fn popup_id(&self, id: Id) -> Option<&SctkPopup> {
        self.chains
            .iter()
            .find_map(move |c| c.into_iter().find(|p| p.data.id == id))
    }

    pub(crate) fn popup_id_mut(&mut self, id: Id) -> Option<&mut SctkPopup> {
        self.chains
            .iter_mut()
            .find_map(move |c| c.into_iter().find(|p| p.data.id == id))
    }

    pub(crate) fn active_grab(&self) -> Option<&SctkPopup> {
        self.chains
            .iter()
            .find_map(|c| c.into_iter().rev().find(|p| p.data.grab))
    }

    pub(crate) fn root_grab(&self) -> Option<&SctkPopup> {
        self.chains
            .iter()
            .find_map(|c| c.into_iter().find(|p| p.data.grab))
    }

    pub(crate) fn push(&mut self, popup: SctkPopup) {
        if let Some((chain, _pos)) =
            self.chain_for_popup_mut(popup.data.parent.wl_surface())
        {
            // TODO should we return an error if a popup is attempted to be added to a non-leaf popup of a chain?
            chain.push(popup);
        } else {
            if let Some(empty) = self.chains.iter_mut().find(|c| c.is_empty()) {
                empty.push(popup);
            } else {
                self.chains.push(vec![popup]);
            }
        }
    }

    pub(crate) fn remove(
        &mut self,
        popup: &WlSurface,
    ) -> Option<impl Iterator<Item = SctkPopup>> {
        // must perform cleanup so that the popups are dropped in the correct order
        let ret = if let Some((chain, mut pos)) =
            self.chain_for_popup_mut(popup)
        {
            // TODO should we return an error if a popup is attempted to be added to a non-leaf popup of a chain?
            while let Some(p) = pos.checked_sub(1).and_then(|p| chain.get(p)) {
                if p.close_with_children {
                    pos -= 1;
                } else {
                    break;
                }
            }

            Some(chain.drain(pos..).rev())
        } else {
            None
        };

        return ret;
    }

    pub(crate) fn remove_ignore_children(
        &mut self,
        popup: &WlSurface,
    ) -> Option<impl Iterator<Item = SctkPopup>> {
        // must perform cleanup so that the popups are dropped in the correct order
        let ret = if let Some((chain, pos)) = self.chain_for_popup_mut(popup) {
            Some(chain.drain(pos..).rev())
        } else {
            None
        };

        return ret;
    }
}

#[derive(Debug)]
pub struct SctkPopup {
    pub(crate) popup: Popup,
    pub(crate) last_configure: Option<PopupConfigure>,
    pub(crate) _pending_requests:
        Vec<platform_specific::wayland::popup::Action>,
    pub(crate) data: SctkPopupData,
    pub(crate) common: Arc<Mutex<Common>>,
    pub(crate) wp_fractional_scale: Option<WpFractionalScaleV1>,
    pub(crate) close_with_children: bool,
}

impl SctkPopup {
    pub(crate) fn set_size(&mut self, w: u32, h: u32, token: u32) {
        let guard = self.common.lock().unwrap();
        if guard.size.width == w && guard.size.height == h {
            return;
        }
        drop(guard);
        // update geometry
        self.popup
            .xdg_surface()
            .set_window_geometry(0, 0, w as i32, h as i32);
        self.update_viewport(w, h);
        // update positioner
        self.data.positioner.set_size(w as i32, h as i32);
        self.popup.reposition(&self.data.positioner, token);
    }

    pub(crate) fn update_viewport(&mut self, w: u32, h: u32) {
        let common = self.common.lock().unwrap();
        if common.size.width == w && common.size.height == h {
            return;
        }
        if let Some(viewport) = common.wp_viewport.as_ref() {
            // Set inner size without the borders.
            viewport.set_destination(w as i32, h as i32);
        }
    }
}
