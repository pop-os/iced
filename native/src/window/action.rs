use crate::window;

use iced_futures::MaybeSend;
use std::fmt;

/// An operation to be performed on some window.
pub enum Action<T> {
    /// Moves the window with the left mouse button until the button is
    /// released.
    ///
    /// There’s no guarantee that this will work unless the left mouse
    /// button was pressed immediately before this function is called.
    Drag,
    /// TODO(derezzedex)
    Spawn {
        /// TODO(derezzedex)
        settings: window::Settings,
    },
    /// TODO(derezzedex)
    Close,
    /// Resize the window.
    Resize {
        /// The new logical width of the window
        width: u32,
        /// The new logical height of the window
        height: u32,
    },
    /// Sets the window to maximized or back
    Maximize(bool),
    /// Set the window to minimized or back
    Minimize(bool),
    /// Move the window.
    Move {
        /// The new logical x location of the window
        x: i32,
        /// The new logical y location of the window
        y: i32,
    },
    /// Sets the window to maximized or back
    ToggleMaximize,
    /// Set the [`Mode`] of the window.
    SetMode(window::Mode),
    /// Fetch the current [`Mode`] of the window.
    FetchMode(Box<dyn FnOnce(window::Mode) -> T + 'static>),
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
            Self::Spawn { settings } => Action::Spawn { settings },
            Self::Close => Action::Close,
            Self::Resize { width, height } => Action::Resize { width, height },
            Self::Maximize(bool) => Action::Maximize(bool),
            Self::Minimize(bool) => Action::Minimize(bool),
            Self::Move { x, y } => Action::Move { x, y },
            Self::SetMode(mode) => Action::SetMode(mode),
            Self::ToggleMaximize => Action::ToggleMaximize,
            Self::FetchMode(o) => Action::FetchMode(Box::new(move |s| f(o(s)))),
        }
    }
}

impl<T> fmt::Debug for Action<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Drag => write!(f, "Action::Drag"),
            Self::Spawn { settings } => {
                write!(f, "Action::Spawn {{ settings: {:?} }}", settings)
            }
            Self::Close => write!(f, "Action::Close"),
            Self::Resize { width, height } => write!(
                f,
                "Action::Resize {{ widget: {}, height: {} }}",
                width, height
            ),
            Self::Maximize(value) => write!(f, "Action::Maximize({})", value),
            Self::Minimize(value) => write!(f, "Action::Minimize({}", value),
            Self::Move { x, y } => {
                write!(f, "Action::Move {{ x: {}, y: {} }}", x, y)
            }
            Self::SetMode(mode) => write!(f, "Action::SetMode({:?})", mode),
            Self::ToggleMaximize => write!(f, "Action::ToggleMaximize"),
            Self::FetchMode(_) => write!(f, "Action::FetchMode"),
        }
    }
}
