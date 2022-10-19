use crate::window::Mode;

use iced_futures::MaybeSend;
use std::fmt;

/// An operation to be performed on some window.
pub enum Action<T> {
    /// Starts a window drag while mouse button is held.
    Drag,
    /// Toggle the maximization of a window
    Maximize,
    /// Minimize the window
    Minimize,
    /// Move the window.
    ///
    /// Unsupported on Wayland.
    Move {
        /// The new logical x location of the window
        x: i32,
        /// The new logical y location of the window
        y: i32,
    },
    /// Resize the window.
    Resize {
        /// The new logical width of the window
        width: u32,
        /// The new logical height of the window
        height: u32,
    },
    /// Resize a window with the mouse
    ResizeDrag,
    /// Set the [`Mode`] of the window.
    SetMode(Mode),
    /// Fetch the current [`Mode`] of the window.
    FetchMode(Box<dyn FnOnce(Mode) -> T + 'static>),
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
            Self::Drag => Action::Drag,
            Self::Resize { width, height } => Action::Resize { width, height },
            Self::ResizeDrag => Action::ResizeDrag,
            Self::Maximize => Action::Maximize,
            Self::Minimize => Action::Minimize,
            Self::Move { x, y } => Action::Move { x, y },
            Self::SetMode(mode) => Action::SetMode(mode),
            Self::FetchMode(o) => Action::FetchMode(Box::new(move |s| f(o(s)))),
        }
    }
}

impl<T> fmt::Debug for Action<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Drag => write!(f, "Action::Drag"),
            Self::Resize { width, height } => write!(
                f,
                "Action::Resize {{ widget: {}, height: {} }}",
                width, height
            ),
            Self::ResizeDrag => write!(f, "Action::ResizeDrag"),
            Self::Maximize => write!(f, "Action::Maximize"),
            Self::Minimize => write!(f, "Action::Minimize"),
            Self::Move { x, y } => {
                write!(f, "Action::Move {{ x: {}, y: {} }}", x, y)
            }
            Self::SetMode(mode) => write!(f, "Action::SetMode({:?})", mode),
            Self::FetchMode(_) => write!(f, "Action::FetchMode"),
        }
    }
}
