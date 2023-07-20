use std::fmt::Debug;

use sctk::{
    data_device_manager::{
        data_device::{DataDevice, DataDeviceHandler},
        data_offer::DragOffer,
    },
    reexports::client::{protocol::wl_data_device, Connection, QueueHandle},
};

use crate::{
    event_loop::state::{SctkDragOffer, SctkSelectionOffer, SctkState},
    sctk_event::{DndOfferEvent, SctkEvent},
};

impl<T> DataDeviceHandler for SctkState<T> {
    fn enter(
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

        let drag_offer = data_device.data().drag_offer().unwrap();
        let mime_types = drag_offer.with_mime_types(|types| types.to_vec());
        self.dnd_offer = Some(SctkDragOffer {
            dropped: false,
            offer: drag_offer.clone(),
            cur_read: None,
        });
        self.sctk_events.push(SctkEvent::DndOffer {
            event: DndOfferEvent::Enter {
                mime_types,
                x: drag_offer.x,
                y: drag_offer.y,
            },
            surface: drag_offer.surface.clone(),
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
                surface: dnd_offer.offer.surface.clone(),
            });
        }
    }

    fn motion(
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

        let offer = data_device.data().drag_offer();
        // if the offer is not the same as the current one, ignore the leave event
        if offer.as_ref() != self.dnd_offer.as_ref().map(|o| &o.offer) {
            return;
        }
        let DragOffer { x, y, surface, .. } =
            data_device.data().drag_offer().unwrap();
        self.sctk_events.push(SctkEvent::DndOffer {
            event: DndOfferEvent::Motion { x, y },
            surface: surface.clone(),
        });
    }

    fn selection(
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

        if let Some(offer) = data_device.data().selection_offer() {
            let mime_types = offer.with_mime_types(|types| types.to_vec());
            self.sctk_events.push(SctkEvent::SelectionOffer(
                crate::sctk_event::SelectionOfferEvent::Offer(mime_types),
            ));
            self.selection_offer = Some(SctkSelectionOffer {
                offer: offer.clone(),
                cur_read: None,
            });
        }
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

        if let Some(offer) = data_device.data().drag_offer() {
            if let Some(dnd_offer) = self.dnd_offer.as_mut() {
                if offer != dnd_offer.offer {
                    return;
                }
                dnd_offer.dropped = true;
            }
            self.sctk_events.push(SctkEvent::DndOffer {
                event: DndOfferEvent::DropPerformed,
                surface: offer.surface.clone(),
            });
        }
    }
}
