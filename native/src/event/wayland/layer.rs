use crate::window;

/// layer surface events
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayerEvent {
    /// layer surface Done
    Done(window::Id),
    /// layer surface focused
    Focused(window::Id),
    /// layer_surface focused
    Unfocused(window::Id),
}
