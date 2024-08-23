//! A renderer-agnostic native GUI runtime.
//!
//! ![The native path of the Iced ecosystem](https://github.com/iced-rs/iced/blob/master/docs/graphs/native.png?raw=true)
//!
//! `iced_runtime` takes [`iced_core`] and builds a native runtime on top of it.
//!
//! [`iced_core`]: https://github.com/iced-rs/iced/tree/0.12/core
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/iced-rs/iced/9ab6923e943f784985e9ef9ca28b10278297225d/docs/logo.svg"
)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
pub mod clipboard;
pub mod dnd;
pub mod font;
pub mod keyboard;
pub mod overlay;
pub mod platform_specific;
pub mod program;
pub mod system;
pub mod user_interface;
pub mod window;

#[cfg(feature = "multi-window")]
pub mod multi_window;

mod task;

// We disable debug capabilities on release builds unless the `debug` feature
// is explicitly enabled.
#[cfg(feature = "debug")]
#[path = "debug/basic.rs"]
mod debug;
#[cfg(not(feature = "debug"))]
#[path = "debug/null.rs"]
mod debug;

pub use iced_core as core;
pub use iced_futures as futures;

pub use debug::Debug;
pub use program::Program;
pub use task::Task;
pub use user_interface::UserInterface;

use crate::core::widget;
use crate::futures::futures::channel::oneshot;
use dnd::DndAction;

use std::borrow::Cow;
use std::fmt;

/// An action that the iced runtime can perform.
pub enum Action<T> {
    /// Output some value.
    Output(T),

    /// Load a font from its bytes.
    LoadFont {
        /// The bytes of the font to load.
        bytes: Cow<'static, [u8]>,
        /// The channel to send back the load result.
        channel: oneshot::Sender<Result<(), font::Error>>,
    },

    /// Run a widget operation.
    Widget(Box<dyn widget::Operation<()> + Send>),

    /// Run a clipboard action.
    Clipboard(clipboard::Action),

    /// Run a window action.
    Window(window::Action),

    /// Run a system action.
    System(system::Action),

    /// Run a Dnd action.
    Dnd(crate::dnd::DndAction),

    /// Run a platform specific action
    PlatformSpecific(crate::platform_specific::Action),
}

impl<T> Action<T> {
    /// Creates a new [`Action::Widget`] with the given [`widget::Operation`].
    pub fn widget(operation: impl widget::Operation<()> + 'static) -> Self {
        Self::Widget(Box::new(operation))
    }

    fn output<O>(self) -> Result<T, Action<O>> {
        match self {
            Action::Output(output) => Ok(output),
            Action::LoadFont { bytes, channel } => {
                Err(Action::LoadFont { bytes, channel })
            }
            Action::Widget(operation) => Err(Action::Widget(operation)),
            Action::Clipboard(action) => Err(Action::Clipboard(action)),
            Action::Window(action) => Err(Action::Window(action)),
            Action::System(action) => Err(Action::System(action)),
            Action::Dnd(a) => Err(Action::Dnd(a)),
            Action::PlatformSpecific(a) => Err(Action::PlatformSpecific(a)),
        }
    }
}

impl<T> fmt::Debug for Action<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Action::Output(output) => write!(f, "Action::Output({output:?})"),
            Action::LoadFont { .. } => {
                write!(f, "Action::LoadFont")
            }
            Action::Widget { .. } => {
                write!(f, "Action::Widget")
            }
            Action::Clipboard(action) => {
                write!(f, "Action::Clipboard({action:?})")
            }
            Action::Window(_) => write!(f, "Action::Window"),
            Action::System(action) => write!(f, "Action::System({action:?})"),
            Action::PlatformSpecific(action) => {
                write!(f, "Action::PlatformSpecific({:?})", action)
            }
            Action::Dnd(action) => write!(f, "Action::Dnd"),
        }
    }
}
