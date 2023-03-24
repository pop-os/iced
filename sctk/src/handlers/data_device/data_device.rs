use std::{fmt::Debug, fs::File, io::Read};

use sctk::{
    data_device_manager::{
        data_device::{DataDevice, DataDeviceDataExt, DataDeviceHandler},
        data_offer::DragOffer,
    },
    delegate_data_device,
    reexports::client::{
        protocol::wl_data_device_manager::DndAction, Connection, QueueHandle,
    },
};

use crate::{
    event_loop::state::SctkState,
    sctk_event::{DndOfferEvent, SctkEvent},
};

impl<T> DataDeviceHandler for SctkState<T> {
    fn enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        data_device: DataDevice,
    ) {
        let mime_types = data_device.drag_mime_types();
        let drag_offer = data_device.drag_offer().unwrap();
        self.dnd_offer = Some(drag_offer.clone());
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
        _data_device: DataDevice,
    ) {
        let surface = self.dnd_offer.take().unwrap().surface.clone();
        self.sctk_events.push(SctkEvent::DndOffer {
            event: DndOfferEvent::Leave,
            surface
        });
    }

    fn motion(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        data_device: DataDevice,
    ) {
        let DragOffer {
            x,
            y,
            time,
            surface,
            ..
        } = data_device.drag_offer().unwrap();
        self.sctk_events.push(SctkEvent::DndOffer {
            event: DndOfferEvent::Motion { x, y },
            surface: surface.clone(),
        });
    }

    fn selection(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        data_device: DataDevice,
    ) {
        if let Some(offer) = data_device.selection_offer() {
            self.sctk_events.push(SctkEvent::SelectionOffer(
                crate::sctk_event::SelectionOfferEvent::Offer(
                    data_device.selection_mime_types(),
                ),
            ));
            self.selection_offer = Some(offer);
        }
    }

    fn drop_performed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        data_device: DataDevice,
    ) {
        if let Some(offer) = data_device.drag_offer() {
            self.sctk_events.push(SctkEvent::DndOffer {
                event: DndOfferEvent::DropPerformed,
                surface: offer.surface.clone(),
            });
        }
    }
}

delegate_data_device!(@<T: 'static + Debug> SctkState<T>);
