use std::hash::{Hash, Hasher};
use std::{collections::hash_map::DefaultHasher, fmt};

use iced_futures::MaybeSend;
use sctk::{
    reexports::client::backend::ObjectId,
    shell::layer::{Layer, KeyboardInteractivity, Anchor, },
};

/// output for layer surface
#[derive(Debug, Clone)]
pub enum IcedOutput {
    /// show on all outputs
    All,
    /// show on active output
    Active,
    /// show on a specific output
    Output {
        /// make
        make: String,
        /// model
        model: String
    },
}

impl Default for IcedOutput {
    fn default() -> Self {
        Self::Active
    }
}

/// margins of the layer surface
#[derive(Debug, Clone, Copy, Default)]
pub struct IcedMargin {
    /// top
    pub top: i32,
    /// right
    pub right: i32,
    /// bottom
    pub bottom: i32,
    /// left
    pub left: i32,
}

/// layer surface
#[derive(Debug, Clone)]
pub struct IcedLayerSurface {
    /// layer
    pub layer: Layer,
    /// interactivity
    pub keyboard_interactivity: KeyboardInteractivity,
    /// anchor
    pub anchor: Anchor,
    /// output
    pub output: IcedOutput,
    /// namespace
    pub namespace: String,
    /// margin
    pub margin: IcedMargin,
    /// size
    pub size: (u32, u32),
    /// exclusive zone
    pub exclusive_zone: i32,
}

impl Default for IcedLayerSurface{
    fn default() -> Self {
        Self { layer: Layer::Top, keyboard_interactivity: Default::default(), anchor: Anchor::empty(), output: Default::default(), namespace: Default::default(), margin: Default::default(), size: (100, 100), exclusive_zone: Default::default() }
    }
}

/// LayerSurface Action
pub enum Action<T> {
    /// create a layer surface and receive a message with its Id
    LayerSurface {
        /// surface builder
        builder: IcedLayerSurface,
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
                o: output,
            } => Action::LayerSurface {
                builder,
                o: Box::new(move |s| f(output(s))),
            },
            Action::Size { width, height } => Action::Size { width, height },
        }
    }
}

impl<T> fmt::Debug for Action<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Action::LayerSurface { builder, .. } => write!(
                f,
                "Action::LayerSurfaceAction::LayerSurface {{ builder: {:?} }}",
                builder
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
