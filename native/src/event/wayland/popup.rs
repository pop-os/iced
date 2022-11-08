use crate::window;

/// popup events
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PopupEvent {
    /// Done
    Done(window::Id),
    /// repositioned,
    Configured {
        /// x position
        x: i32,
        /// y position
        y: i32,
        /// width
        width: u32,
        /// height
        height: u32,
    },
}
