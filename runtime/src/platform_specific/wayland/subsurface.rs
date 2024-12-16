use std::any::Any;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use cctk::sctk::reexports::protocols::xdg::shell::client::xdg_positioner::{
    Anchor, Gravity,
};
use iced_core::layout::Limits;
use iced_core::window::Id;
use iced_core::{Element, Point, Rectangle, Size};

/// Subsurface creation details
#[derive(Debug, Clone)]
pub struct SctkSubsurfaceSettings {
    /// XXX must be unique, id of the parent
    pub parent: Id,
    /// XXX must be unique, id of the subsurface
    pub id: Id,
    /// position of the subsurface
    pub loc: Point,
    /// size of the subsurface
    pub size: Option<Size>,
    // pub subsurface_view: Option<Arc<dyn Any + Send + Sync>>,
    /// Z
    pub z: u32,
}

impl Hash for SctkSubsurfaceSettings {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl SctkSubsurfaceSettings {
    // /// Create a subsurface with a self defined view
    // pub fn with_view<M: 'static, T: 'static, R: 'static>(
    //     mut self,
    //     view: Box<
    //         dyn Fn() -> Option<Element<'static, M, T, R>>
    //             + Send
    //             + Sync
    //             + 'static,
    //     >,
    // ) -> Self {
    //     self.subsurface_view = Some(Arc::new(view));
    //     self
    // }
}

#[derive(Clone)]
/// Window Action
pub enum Action {
    /// create a window and receive a message with its Id
    Subsurface {
        /// subsurface
        subsurface: SctkSubsurfaceSettings,
    },
    /// destroy the subsurface
    Destroy {
        /// id of the subsurface
        id: Id,
    },
}

impl fmt::Debug for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Action::Subsurface { subsurface, .. } => write!(
                f,
                "Action::SubsurfaceAction::Subsurface {{ subsurface: {:?} }}",
                subsurface
            ),
            Action::Destroy { id } => write!(
                f,
                "Action::SubsurfaceAction::Destroy {{ id: {:?} }}",
                id
            ),
        }
    }
}
