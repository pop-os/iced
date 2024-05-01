use iced_core::layout::Limits;
use iced_core::window::Id;
use iced_futures::MaybeSend;
use std::hash::{Hash, Hasher};
use std::{fmt, marker::PhantomData};

/// Popup creation details
#[derive(Debug, Clone)]
pub struct InputMethodPopupSettings {
    /// Id of the popup
    pub id: Id,
    /// Limits of the window size
    pub size_limits: Limits,
    /// The initial size of the window.
    pub size: (u32, u32),
}

impl Default for InputMethodPopupSettings {
    fn default() -> Self {
        Self {
            id: Id::MAIN,
            size_limits: Limits::NONE
                .min_height(1.0)
                .min_width(1.0)
                .max_width(1920.0)
                .max_height(1080.0),
            size: (256, 256),
        }
    }
}

impl Hash for InputMethodPopupSettings {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

/// Input Method Actions
pub enum Action<T> {
    /// Create input method popup
    Popup {
        /// settings
        settings: InputMethodPopupSettings,
        /// phantom
        _phantom: PhantomData<T>,
    },
    /// show input method popup
    ShowPopup,
    /// hide input method popup
    HidePopup,
    /// Set size of the input method popup
    Size {
        /// id of the popup
        id: Id,
        /// width
        width: u32,
        /// height
        height: u32,
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
            Action::Popup { settings, _phantom } => Action::Popup {
                settings,
                _phantom: PhantomData,
            },
            Action::ShowPopup => Action::ShowPopup,
            Action::HidePopup => Action::HidePopup,
            Action::Size { id, width, height } => {
                Action::Size { id, width, height }
            }
        }
    }
}

impl<T> fmt::Debug for Action<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Popup { settings, _phantom } => f
                .debug_tuple("Show Input Method Popup")
                .field(settings)
                .finish(),
            Self::ShowPopup => {
                f.debug_tuple("Show Input Method Popup").finish()
            }
            Self::HidePopup => {
                f.debug_tuple("Hide Input Method Popup").finish()
            }
            Self::Size { id, width, height } => f
                .debug_tuple("Input method popup size changed")
                .field(id)
                .field(width)
                .field(height)
                .finish(),
        }
    }
}
