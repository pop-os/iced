use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::{collections::hash_map::DefaultHasher, fmt};

use iced_futures::MaybeSend;

use crate::window;

/// window settings
#[derive(Debug, Clone)]
pub struct SctkWindowSettings {
    /// vanilla window settings
    pub iced_settings: window::Settings,
    /// window id
    pub window_id: window::Id,
    /// optional app id
    pub app_id: Option<String>,
    /// optional window title
    pub title: Option<String>,
    /// optional window parent
    pub parent: Option<window::Id>,
}

impl Default for SctkWindowSettings {
    fn default() -> Self {
        Self { iced_settings: Default::default(), window_id: window::Id::new(0), app_id: Default::default(), title: Default::default(), parent: Default::default() }
    }
}

#[derive(Clone)]
/// Window Action
pub enum Action<T> {
    /// create a window and receive a message with its Id
    Window {
        /// window builder
        builder: SctkWindowSettings,
        /// phanton
        _phantom: PhantomData<T>
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
            Action::Window { builder, .. } => Action::Window {
                builder,
                _phantom: PhantomData::default()
            },
        }
    }
}

impl<T> fmt::Debug for Action<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Action::Window { builder, .. } => write!(
                f,
                "Action::LayerSurfaceAction::LayerSurface {{ builder: {:?} }}",
                builder
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
