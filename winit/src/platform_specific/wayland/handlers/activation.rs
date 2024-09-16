
use sctk::{
    activation::{ActivationHandler, RequestData},
    delegate_activation,
};

use crate::platform_specific::wayland::event_loop::state::SctkState;

// pub struct IcedRequestData {
//     data: RequestData,
// }

// impl<T> IcedRequestData<T> {
//     pub fn new(
//         data: RequestData,
//         message: Box<dyn FnOnce(Option<String>) -> T + Send + Sync + 'static>,
//     ) -> IcedRequestData<T> {
//         IcedRequestData { data }
//     }
// }

// impl<T> RequestDataExt for IcedRequestData<T> {
//     fn app_id(&self) -> Option<&str> {
//         self.data.app_id()
//     }

//     fn seat_and_serial(&self) -> Option<(&WlSeat, u32)> {
//         self.data.seat_and_serial()
//     }

//     fn surface(&self) -> Option<&WlSurface> {
//         self.data.surface()
//     }
// }

impl ActivationHandler for SctkState {
    type RequestData = RequestData;

    fn new_token(&mut self, token: String, data: &Self::RequestData) {
        // TODO cleanup
        // self.pending_events.push(
        //         Event::SctkEvent(
        //             crate::platform_specific::wayland::sctk_event::IcedSctkEvent::SctkEvent(Some(
        //                 token,
        //             )),
        //         ),
        //     );
    }
}

delegate_activation!(SctkState, RequestData);
