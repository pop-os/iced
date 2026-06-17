use cctk::sctk::{
    activation::{ActivationHandler, RequestData},
    reexports::client::protocol::{wl_seat::WlSeat, wl_surface::WlSurface},
};
use iced_futures::futures::channel::oneshot::Sender;

use crate::platform_specific::wayland::event_loop::state::SctkState;

pub struct IcedRequestData {
    id: u32,
}

impl IcedRequestData {
    pub fn new(id: u32) -> IcedRequestData {
        IcedRequestData { id }
    }
}

impl ActivationHandler for SctkState {
    type RequestUdata = IcedRequestData;

    fn new_token(&mut self, token: String, data: &RequestData<Self::RequestUdata>) {
        if let Some(tx) = self.token_senders.remove(&data.udata.id) {
            _ = tx.send(Some(token));
        } else {
            log::error!("Missing activation request Id.");
        }
    }
}
