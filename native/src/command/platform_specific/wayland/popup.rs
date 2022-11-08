use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::{collections::hash_map::DefaultHasher, fmt};

use iced_core::Rectangle;
use iced_futures::MaybeSend;
use sctk::reexports::protocols::xdg::shell::client::xdg_positioner::{
    Anchor, Gravity,
};
use sctk::shell::xdg::XdgPositioner;

use crate::window;
/// TODO
#[derive(Debug, Clone)]
pub struct IcedPopup {
    /// XXX must be unique, id of the parent
    parent: window::Id,
    /// XXX must be unique, id of the popup
    id: window::Id,
    /// size of the popup
    size: (u32, u32),
    /// the rectangle which the popup will be anchored to
    anchor_rect: Rectangle<u32>,
    /// the anchor location on the popup
    anchor: Anchor,
    /// the gravity of the popup
    gravity: Gravity,
    /// the constraint adjustment
    constraint_adjustment: u32,
    /// offset of the popup
    offset: (u32, u32),
    /// whether the popup is reactive
    reactive: bool,
    /// optional parent size, must be correct or the behavior is undefined
    parent_size: Option<(u32, u32)>,
    /// whether a grab should be requested for the popup after creation
    grab: bool,
}

#[derive(Clone)]
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
