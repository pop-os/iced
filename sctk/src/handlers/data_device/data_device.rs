use std::{io::Read, fmt::Debug, fs::File};

use sctk::{delegate_data_device, reexports::client::{Connection, protocol::wl_data_device_manager::DndAction, QueueHandle}, data_device_manager::{data_device::{DataDevice, DataDeviceDataExt, DataDeviceHandler}, data_offer::DragOffer}};

use crate::event_loop::state::SctkState;


impl<T> DataDeviceHandler for SctkState<T> {
    fn enter(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, data_device: DataDevice) {
        let mut drag_offer = data_device.drag_offer().unwrap();
        dbg!(drag_offer.x, drag_offer.y);
    }

    fn leave(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _data_device: DataDevice) {
        println!("data offer left");
    }

    fn motion(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, data_device: DataDevice) {
        let DragOffer { x, y, time, .. } = data_device.drag_offer().unwrap();

        dbg!((time, x, y));
    }

    fn selection(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, data_device: DataDevice) {
        if let Some(offer) = data_device.selection_offer() {
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
            dbg!(&offer);
            
            // self.accept_counter += 1;
            // cur_offer.0.accept_mime_type(self.accept_counter, Some(mime_type.clone()));
            // cur_offer.0.set_actions(DndAction::Copy, DndAction::Copy);
            // if let Ok(read_pipe) = cur_offer.0.receive(mime_type.clone()) {
            //     let offer_clone = cur_offer.0.clone();
            //     match self.loop_handle.insert_source(read_pipe, move |_, f: &mut File, state: &mut SctkState<T>| {
            //         let (offer, mut contents, token) = state
            //             .dnd_offer
            //             .take()
            //             .unwrap();
            //
            //         if offer != offer_clone {
            //             return;
            //         }
            //
            //         f.read_to_string(&mut contents).unwrap();
            //         println!("TEXT FROM drop: {contents}");
            //         state.loop_handle.remove(token.unwrap());
            //
            //         offer.finish();
            //     }) {
            //         Ok(token) => {
            //             cur_offer.2.replace(token);
            //         }
            //         Err(err) => {
            //             eprintln!("{:?}", err);
            //         }
            //     }
            // }
        }
    }
}

delegate_data_device!(@<T: 'static + Debug> SctkState<T>);
