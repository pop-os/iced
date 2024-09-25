use std::{fmt, marker::PhantomData};

use iced_core::window::Id;
use iced_futures::MaybeSend;

use sctk::reexports::client::protocol::wl_output::WlOutput;

/// Session lock action
#[derive(Clone)]
pub enum Action {
    /// Request a session lock
    Lock,
    /// Destroy lock
    Unlock,
    /// Create lock surface for output
    LockSurface {
        /// unique id for surface
        id: Id,
        /// output
        output: WlOutput,
    },
    /// Destroy lock surface
    DestroyLockSurface {
        /// unique id for surface
        id: Id,
    },
}

impl fmt::Debug for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Action::Lock => write!(f, "Action::SessionLock::Lock"),
            Action::Unlock => write!(f, "Action::SessionLock::Unlock"),
            Action::LockSurface { id, output } => write!(
                f,
                "Action::SessionLock::LockSurface {{ id: {:?}, output: {:?} }}",
                id, output
            ),
            Action::DestroyLockSurface { id } => write!(
                f,
                "Action::SessionLock::DestroyLockSurface {{ id: {:?} }}",
                id
            ),
        }
    }
}
