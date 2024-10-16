#![allow(missing_docs)]

/// window events
#[derive(Debug, PartialEq, Clone)]
pub enum WindowEvent {
    /// Window suggested bounds.
    SuggestedBounds(Option<crate::Size>),
}
