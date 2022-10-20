use std::hash::{Hash, Hasher};
use std::{collections::hash_map::DefaultHasher, fmt};

use iced_futures::MaybeSend;
use sctk::{
    reexports::client::backend::ObjectId,
    shell::layer::{Layer, LayerSurfaceBuilder},
};

/// LayerSurface Action
pub enum Action<T> {
    /// create a layer surface and receive a message with its Id
    LayerSurface {
        /// surface builder
        builder: LayerSurfaceBuilder,
        /// layer of the surface
        layer: Layer,
        /// the returned object id from sctk
        o: Box<dyn FnOnce(ObjectId) -> T + 'static>,
    },
    /// Set size of the layer surface.
    Size {
        /// The new logical width of the window
        width: u32,
        /// The new logical height of the window
        height: u32,
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
            Action::LayerSurface {
                builder,
                layer,
                o: output,
            } => Action::LayerSurface {
                builder,
                layer,
                o: Box::new(move |s| f(output(s))),
            },
            Action::Size { width, height } => Action::Size { width, height },
        }
    }
}

impl<T> fmt::Debug for Action<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Action::LayerSurface { builder, layer, .. } => write!(
                f,
                "Action::LayerSurfaceAction::LayerSurface {{ builder: {:?} layer: {:?} }}",
                builder, layer
            ),
            Action::Size { width, height } => write!(
                f,
                "Action::LayerSurfaceAction::Size {{ width: {width}, height: {height} }}",
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
