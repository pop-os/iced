use iced_core::window::Id;
use iced_futures::MaybeSend;

use std::fmt;

use crate::oneshot;

/// xdg-activation Actions
pub enum Action {
    /// request an activation token
    RequestToken {
        /// application id
        app_id: Option<String>,
        /// window, if provided
        window: Option<Id>,
        /// message generation
        channel: oneshot::Sender<Option<String>>,
    },
    /// request a window to be activated
    Activate {
        /// window to activate
        window: Id,
        /// activation token
        token: String,
    },
}

// impl<T> Action<T> {
//     /// Maps the output of a window [`Action`] using the provided closure.
//     pub fn map<A>(
//         self,
//         mapper: impl Fn(T) -> A + 'static + MaybeSend + Sync,
//     ) -> Action<A>
//     where
//         T: 'static,
//     {
//         match self {
//             Action::RequestToken {
//                 app_id,
//                 window,
//                 message,
//             } => Action::RequestToken {
//                 app_id,
//                 window,
//                 message: Box::new(move |token| mapper(message(token))),
//             },
//             Action::Activate { window, token } => {
//                 Action::Activate { window, token }
//             }
//         }
//     }
// }

impl fmt::Debug for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Action::RequestToken { app_id, window, .. } => write!(
                f,
                "Action::ActivationAction::RequestToken {{ app_id: {:?}, window: {:?} }}",
                app_id, window,
            ),
            Action::Activate { window, token } => write!(
                f,
                "Action::ActivationAction::Activate {{ window: {:?}, token: {:?} }}",
                window, token,
            )
        }
    }
}
