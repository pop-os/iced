use sctk::{
    data_device_manager::data_offer::{DataOfferHandler, DragOffer},
    reexports::client::{
        protocol::wl_data_device_manager::DndAction, Connection, QueueHandle,
    },
};

use crate::event_loop::state::SctkState;

impl<T> DataOfferHandler for SctkState<T> {
    fn source_actions(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        offer: &mut DragOffer,
        actions: DndAction,
    ) {
        if self
            .dnd_offer
            .as_ref()
            .map(|o| o.offer.inner() == offer.inner())
            .unwrap_or(false)
        {
            self.sctk_events
                .push(crate::sctk_event::SctkEvent::DndOffer {
                    event: crate::sctk_event::DndOfferEvent::SourceActions(
                        actions,
                    ),
                    surface: offer.surface.clone(),
                });
        }
    }

    fn selected_action(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        offer: &mut DragOffer,
        actions: DndAction,
    ) {
        if self
            .dnd_offer
            .as_ref()
            .map(|o| o.offer.inner() == offer.inner())
            .unwrap_or(false)
        {
            self.sctk_events
                .push(crate::sctk_event::SctkEvent::DndOffer {
                    event: crate::sctk_event::DndOfferEvent::SelectedAction(
                        actions,
                    ),
                    surface: offer.surface.clone(),
                });
        }
    }
}
