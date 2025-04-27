//! Handle events of a user interface.
use dnd::DndEvent;
use dnd::DndSurface;

use crate::input_method;
use crate::keyboard;
use crate::mouse;
use crate::touch;
use crate::window;
#[cfg(feature = "wayland")]
/// A platform specific event for wayland
pub mod wayland;
/// A user interface event.
///
/// _**Note:** This type is largely incomplete! If you need to track
/// additional events, feel free to [open an issue] and share your use case!_
///
/// [open an issue]: https://github.com/iced-rs/iced/issues
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// A keyboard event
    Keyboard(keyboard::Event),

    /// A mouse event
    Mouse(mouse::Event),

    /// A window event
    Window(window::Event),

    /// A touch event
    Touch(touch::Event),

    /// An input method event
    InputMethod(input_method::Event),

    #[cfg(feature = "a11y")]
    /// An Accesskit event for a specific Accesskit Node in an accessible widget
    A11y(
        crate::widget::Id,
        iced_accessibility::accesskit::ActionRequest,
    ),

    /// A DnD event.
    Dnd(DndEvent<DndSurface>),

    /// Platform specific events
    PlatformSpecific(PlatformSpecific),
}

/// A platform specific event
#[derive(Debug, Clone, PartialEq)]
pub enum PlatformSpecific {
    #[cfg(feature = "wayland")]
    /// A Wayland specific event
    Wayland(wayland::Event),
}

/// The status of an [`Event`] after being processed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// The [`Event`] was **NOT** handled by any widget.
    Ignored,

    /// The [`Event`] was handled and processed by a widget.
    Captured,
}

impl Status {
    /// Merges two [`Status`] into one.
    ///
    /// `Captured` takes precedence over `Ignored`:
    ///
    /// ```
    /// use iced_core::event::Status;
    ///
    /// assert_eq!(Status::Ignored.merge(Status::Ignored), Status::Ignored);
    /// assert_eq!(Status::Ignored.merge(Status::Captured), Status::Captured);
    /// assert_eq!(Status::Captured.merge(Status::Ignored), Status::Captured);
    /// assert_eq!(Status::Captured.merge(Status::Captured), Status::Captured);
    /// ```
    pub fn merge(self, b: Self) -> Self {
        match self {
            Status::Ignored => b,
            Status::Captured => Status::Captured,
        }
    }
}
