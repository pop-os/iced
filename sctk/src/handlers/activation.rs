use std::sync::Mutex;

use sctk::{
    activation::{ActivationHandler, RequestData, RequestDataExt},
    delegate_activation,
    reexports::client::protocol::{wl_seat::WlSeat, wl_surface::WlSurface},
};

use crate::event_loop::state::SctkState;

pub struct IcedRequestData<T> {
    data: RequestData,
    message: Mutex<
        Option<Box<dyn FnOnce(Option<String>) -> T + Send + Sync + 'static>>,
    >,
}

impl<T> IcedRequestData<T> {
    pub fn new(
        data: RequestData,
        message: Box<dyn FnOnce(Option<String>) -> T + Send + Sync + 'static>,
    ) -> IcedRequestData<T> {
        IcedRequestData {
            data,
            message: Mutex::new(Some(message)),
        }
    }
}

impl<T> RequestDataExt for IcedRequestData<T> {
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

impl<T> ActivationHandler for SctkState<T> {
    type RequestData = IcedRequestData<T>;

    fn new_token(&mut self, token: String, data: &Self::RequestData) {
        if let Some(message) = data.message.lock().unwrap().take() {
            self.pending_user_events.push(
                crate::application::Event::SctkEvent(
                    crate::sctk_event::IcedSctkEvent::UserEvent(message(Some(
                        token,
                    ))),
                ),
            );
        } // else the compositor send two tokens???
    }
}

delegate_activation!(@<T> SctkState<T>, IcedRequestData<T>);
