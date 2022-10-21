
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::{collections::hash_map::DefaultHasher, fmt};

use iced_futures::MaybeSend;
use sctk::{
    reexports::client::backend::ObjectId,
};

/// TODO
#[derive(Debug)]
pub struct IcedPopup;
/// Window Action
pub enum Action<T> {
    /// create a window and receive a message with its Id
    Popup {
        /// popup
        popup: IcedPopup,
        /// phantom
        _phantom: PhantomData<T>,
    },
}

impl<T> Action<T> {
    /// Maps the output of a window [`Action`] using the provided closure.
    pub fn map<A>(
        self,
        f: impl Fn(T) -> A + 'static + MaybeSend + Sync,
    ) -> Action<A>
    where
        T: 'static,
    {
        match self {
            Action::Popup { popup, .. } => Action::Popup {
                popup,
                _phantom: PhantomData::default(),
            },
        }
    }
}

impl<T> fmt::Debug for Action<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Action::Popup { popup, .. } => write!(
                f,
                "Action::LayerSurfaceAction::LayerSurface {{ popup: {:?} }}",
                popup
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
/// TODO(derezzedex)
pub struct Id(u64);

impl Id {
    /// TODO(derezzedex)
    pub fn new(id: impl Hash) -> Id {
        let mut hasher = DefaultHasher::new();
        id.hash(&mut hasher);

        Id(hasher.finish())
    }
}
