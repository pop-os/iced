use std::{fmt, marker::PhantomData};

use iced_core::window::Id;
use iced_futures::MaybeSend;

use sctk::reexports::client::protocol::wl_output::WlOutput;

/// Session lock action
#[derive(Clone)]
pub enum Action<T> {
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
        /// phantom
        _phantom: PhantomData<T>,
    },
    /// Destroy lock surface
    DestroyLockSurface {
        /// unique id for surface
        id: Id,
    },
}

impl<T> Action<T> {
    /// Maps the output of a window [`Action`] using the provided closure.
    pub fn map<A>(
        self,
        _: impl Fn(T) -> A + 'static + MaybeSend + Sync,
    ) -> Action<A>
    where
        T: 'static,
    {
        match self {
            Action::Lock => Action::Lock,
            Action::Unlock => Action::Unlock,
            Action::LockSurface {
                id,
                output,
                _phantom,
            } => Action::LockSurface {
                id,
                output,
                _phantom: PhantomData,
            },
            Action::DestroyLockSurface { id } => {
                Action::DestroyLockSurface { id }
            }
        }
    }
}

impl<T> fmt::Debug for Action<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Action::Lock => write!(f, "Action::SessionLock::Lock"),
            Action::Unlock => write!(f, "Action::SessionLock::Unlock"),
            Action::LockSurface {
                id,
                output,
                _phantom,
            } => write!(
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
