use std::fmt::Debug;

use sctk::{
    data_device_manager::data_offer::{DataDeviceOffer, DataOfferHandler},
    delegate_data_offer,
    reexports::client::{
        protocol::wl_data_device_manager::DndAction, Connection, QueueHandle,
    },
};

use crate::event_loop::state::SctkState;

impl<T> DataOfferHandler for SctkState<T> {
    fn offer(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        offer: &mut DataDeviceOffer,
        mime_type: String,
    ) {
        // println!("Received offer with mime type: {mime_type}");
        // let serial = self.accept_counter;
        // self.accept_counter += 1;
        // if &mime_type == "UTF8_STRING"
        //     || &mime_type == "text/plain;charset=UTF-8"
        // {
        //     offer.accept_mime_type(serial, Some(mime_type.clone()));
        // }
    }

    fn source_actions(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        offer: &mut DataDeviceOffer,
        actions: DndAction,
    ) {
        // dbg!(actions);
        // offer.set_actions(DndAction::Copy, DndAction::Copy);
    }

    fn actions(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _offer: &mut DataDeviceOffer,
        actions: DndAction,
    ) {
        // dbg!(actions);
        // TODO ?
    }
}

delegate_data_offer!(@<T: 'static + Debug> SctkState<T>);
