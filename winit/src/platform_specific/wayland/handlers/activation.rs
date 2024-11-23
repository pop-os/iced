use iced_futures::futures::channel::oneshot::Sender;
use cctk::sctk::{
    activation::{ActivationHandler, RequestData, RequestDataExt},
    delegate_activation,
    reexports::client::protocol::{wl_seat::WlSeat, wl_surface::WlSurface},
};

use crate::platform_specific::wayland::event_loop::state::SctkState;

pub struct IcedRequestData {
    id: u32,
    data: RequestData,
}

impl IcedRequestData {
    pub fn new(data: RequestData, id: u32) -> IcedRequestData {
        IcedRequestData { data, id }
    }
}

impl RequestDataExt for IcedRequestData {
    fn app_id(&self) -> Option<&str> {
        self.data.app_id()
    }

    fn seat_and_serial(&self) -> Option<(&WlSeat, u32)> {
        self.data.seat_and_serial()
    }

    fn surface(&self) -> Option<&WlSurface> {
        self.data.surface()
    }
}

impl ActivationHandler for SctkState {
    type RequestData = IcedRequestData;

    fn new_token(&mut self, token: String, data: &Self::RequestData) {
        if let Some(tx) = self.token_senders.remove(&data.id) {
            _ = tx.send(Some(token));
        } else {
            log::error!("Missing activation request Id.");
        }
    }
}

delegate_activation!(SctkState, IcedRequestData);
