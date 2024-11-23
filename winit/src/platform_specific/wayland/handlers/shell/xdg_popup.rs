use crate::platform_specific::wayland::{
    event_loop::state::{self, PopupParent, SctkState},
    sctk_event::{PopupEventVariant, SctkEvent},
};
use cctk::sctk::{
    delegate_xdg_popup, reexports::client::Proxy,
    shell::xdg::popup::PopupHandler,
};

impl PopupHandler for SctkState {
    fn configure(
        &mut self,
        _conn: &cctk::sctk::reexports::client::Connection,
        _qh: &cctk::sctk::reexports::client::QueueHandle<Self>,
        popup: &cctk::sctk::shell::xdg::popup::Popup,
        configure: cctk::sctk::shell::xdg::popup::PopupConfigure,
    ) {
        self.request_redraw(popup.wl_surface());
        let sctk_popup = match self.popups.iter_mut().find(|s| {
            s.popup.wl_surface().clone() == popup.wl_surface().clone()
        }) {
            Some(p) => p,
            None => return,
        };
        let first = sctk_popup.last_configure.is_none();
        _ = sctk_popup.last_configure.replace(configure.clone());

        self.sctk_events.push(SctkEvent::PopupEvent {
            variant: PopupEventVariant::Configure(
                configure,
                popup.wl_surface().clone(),
                first,
            ),
            id: popup.wl_surface().clone(),
            toplevel_id: sctk_popup.data.toplevel.clone(),
            parent_id: match &sctk_popup.data.parent {
                PopupParent::LayerSurface(s) => s.clone(),
                PopupParent::Window(s) => s.clone(),
                PopupParent::Popup(s) => s.clone(),
            },
        });
    }

    fn done(
        &mut self,
        _conn: &cctk::sctk::reexports::client::Connection,
        _qh: &cctk::sctk::reexports::client::QueueHandle<Self>,
        popup: &cctk::sctk::shell::xdg::popup::Popup,
    ) {
        let sctk_popup = match self.popups.iter().position(|s| {
            s.popup.wl_surface().clone() == popup.wl_surface().clone()
        }) {
            Some(p) => self.popups.remove(p),
            None => return,
        };
        let mut to_destroy = vec![sctk_popup];
        while let Some(popup_to_destroy) = to_destroy.last() {
            match popup_to_destroy.data.parent.clone() {
                state::PopupParent::LayerSurface(_)
                | state::PopupParent::Window(_) => {
                    break;
                }
                state::PopupParent::Popup(popup_to_destroy_first) => {
                    let popup_to_destroy_first = self
                        .popups
                        .iter()
                        .position(|p| {
                            p.popup.wl_surface() == &popup_to_destroy_first
                        })
                        .unwrap();
                    let popup_to_destroy_first =
                        self.popups.remove(popup_to_destroy_first);
                    to_destroy.push(popup_to_destroy_first);
                }
            }
        }
        for popup in to_destroy.into_iter().rev() {
            if let Some(id) = self.id_map.remove(&popup.popup.wl_surface().id())
            {
                _ = self.destroyed.insert(id);
            }

            self.sctk_events.push(SctkEvent::PopupEvent {
                variant: PopupEventVariant::Done,
                toplevel_id: popup.data.toplevel.clone(),
                parent_id: popup.data.parent.wl_surface().clone(),
                id: popup.popup.wl_surface().clone(),
            });
        }
    }
}
delegate_xdg_popup!(SctkState);
