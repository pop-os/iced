use sctk::{
    data_device_manager::{
        data_device::DataDeviceHandler, data_offer::DragOffer,
    },
    reexports::client::{
        protocol::{wl_data_device, wl_surface::WlSurface},
        Connection, QueueHandle,
    },
};

use crate::{
    event_loop::state::{SctkDragOffer, SctkState},
    sctk_event::{DndOfferEvent, SctkEvent},
};

impl<T> DataDeviceHandler for SctkState<T> {
    fn enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        wl_data_device: &wl_data_device::WlDataDevice,
        x: f64,
        y: f64,
        s: &WlSurface,
    ) {
        let data_device = if let Some(seat) = self
            .seats
            .iter()
            .find(|s| s.data_device.inner() == wl_data_device)
        {
            &seat.data_device
        } else {
            return;
        };

        let drag_offer = data_device.data().drag_offer();
        let mime_types = drag_offer
            .as_ref()
            .map(|offer| offer.with_mime_types(|types| types.to_vec()))
            .unwrap_or_default();
        self.dnd_offer = Some(SctkDragOffer {
            dropped: false,
            offer: drag_offer.clone(),
            cur_read: None,
            surface: s.clone(),
        });
        self.sctk_events.push(SctkEvent::DndOffer {
            event: DndOfferEvent::Enter { mime_types, x, y },
            surface: s.clone(),
        });
    }

    fn leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _wl_data_device: &wl_data_device::WlDataDevice,
    ) {
        // ASHLEY TODO the dnd_offer should be removed when the leave event is received
        // but for now it is not if the offer was previously dropped.
        // It seems that leave events are received even for offers which have
        // been accepted and need to be read.
        if let Some(dnd_offer) = self.dnd_offer.take() {
            if dnd_offer.dropped {
                self.dnd_offer = Some(dnd_offer);
                return;
            }

            self.sctk_events.push(SctkEvent::DndOffer {
                event: DndOfferEvent::Leave,
                surface: dnd_offer.surface.clone(),
            });
        }
    }

    fn motion(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        wl_data_device: &wl_data_device::WlDataDevice,
        x: f64,
        y: f64,
    ) {
        let data_device = if let Some(seat) = self
            .seats
            .iter()
            .find(|s| s.data_device.inner() == wl_data_device)
        {
            &seat.data_device
        } else {
            return;
        };

        let offer = data_device.data().drag_offer();
        // if the offer is not the same as the current one, ignore the leave event
        if offer.as_ref()
            != self.dnd_offer.as_ref().and_then(|o| o.offer.as_ref())
        {
            return;
        }

        let Some(surface) = offer.as_ref().map(|o| o.surface.clone()) else {
            return;
        };

        self.sctk_events.push(SctkEvent::DndOffer {
            event: DndOfferEvent::Motion { x, y },
            surface,
        });
    }

    fn selection(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _wl_data_device: &wl_data_device::WlDataDevice,
    ) {
        // not handled here
    }

    fn drop_performed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        wl_data_device: &wl_data_device::WlDataDevice,
    ) {
        let data_device = if let Some(seat) = self
            .seats
            .iter()
            .find(|s| s.data_device.inner() == wl_data_device)
        {
            &seat.data_device
        } else {
            return;
        };

        if let Some(dnd_offer) = self.dnd_offer.as_mut() {
            if data_device.data().drag_offer() != dnd_offer.offer {
                return;
            }
            self.sctk_events.push(SctkEvent::DndOffer {
                event: DndOfferEvent::DropPerformed,
                surface: dnd_offer.surface.clone(),
            });
            dnd_offer.dropped = true;
        }
    }
}
