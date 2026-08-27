use crate::event_loop::state::receive_frame;
use crate::platform_specific::wayland::{
    event_loop::state::{PopupParent, SctkState},
    sctk_event::{PopupEventVariant, SctkEvent},
};
use cctk::sctk::{
    delegate_xdg_popup, reexports::client::Proxy,
    shell::xdg::popup::PopupHandler,
};
use winit::dpi::LogicalSize;

impl PopupHandler for SctkState {
    fn configure(
        &mut self,
        _conn: &cctk::sctk::reexports::client::Connection,
        _qh: &cctk::sctk::reexports::client::QueueHandle<Self>,
        popup: &cctk::sctk::shell::xdg::popup::Popup,
        configure: cctk::sctk::shell::xdg::popup::PopupConfigure,
    ) {
        self.request_redraw(popup.wl_surface());
        let sctk_popup = match self.popmgr.popup_mut(popup.wl_surface()) {
            Some(p) => p,
            None => {
                return;
            }
        };
        let first = sctk_popup.last_configure.is_none();
        _ = sctk_popup.last_configure.replace(configure.clone());
        let mut guard = sctk_popup.common.lock().unwrap();
        guard.size =
            LogicalSize::new(configure.width as u32, configure.height as u32);
        receive_frame(&mut self.frame_status, popup.wl_surface());

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
        let Some(to_destroy) = self.popmgr.remove(popup.wl_surface()) else {
            return;
        };

        for popup in to_destroy {
            if let Some(id) = self.id_map.remove(&popup.popup.wl_surface().id())
            {
                if let Some(blurred) = self.blur_surfaces.remove(&id) {
                    blurred.destroy();
                }
                _ = self.corner_radii.remove(&id);

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
