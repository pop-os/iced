//! A renderer-agnostic native GUI runtime.
//!
//! ![The native path of the Iced ecosystem](https://github.com/iced-rs/iced/blob/master/docs/graphs/native.png?raw=true)
//!
//! `iced_runtime` takes [`iced_core`] and builds a native runtime on top of it.
//!
//! [`iced_core`]: https://github.com/iced-rs/iced/tree/0.14/core
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/iced-rs/iced/9ab6923e943f784985e9ef9ca28b10278297225d/docs/logo.svg"
)]
#![cfg_attr(docsrs, feature(doc_cfg))]
pub mod clipboard;
pub mod dnd;
pub mod font;
pub mod image;
pub mod keyboard;
pub mod platform_specific;
pub mod system;
pub mod task;
pub mod user_interface;
pub mod widget;
pub mod window;

pub use iced_core as core;
pub use iced_futures as futures;

pub use task::Task;
pub use user_interface::UserInterface;
pub use window::Window;

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
    Widget(Box<dyn core::widget::Operation>),

    /// Run a clipboard action.
    Clipboard(clipboard::Action),

    /// Run a window action.
    Window(window::Action),

    /// Run a system action.
    System(system::Action),

    /// An image action.
    Image(image::Action),

    /// Recreate all user interfaces and redraw all windows.
    Reload,

    /// Exits the runtime.
    ///
    /// This will normally close any application windows and
    /// terminate the runtime loop.
    Exit,
    /// Run a Dnd action.
    Dnd(crate::dnd::DndAction),

    /// Run a platform specific action
    PlatformSpecific(crate::platform_specific::Action),
}

impl<T> Action<T> {
    /// Creates a new [`Action::Widget`] with the given [`widget::Operation`](core::widget::Operation).
    pub fn widget(operation: impl core::widget::Operation + 'static) -> Self {
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
            Action::Image(action) => Err(Action::Image(action)),
            Action::Reload => Err(Action::Reload),
            Action::Exit => Err(Action::Exit),
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
            Action::Image(_) => write!(f, "Action::Image"),
            Action::Reload => write!(f, "Action::Reload"),
            Action::Exit => write!(f, "Action::Exit"),
            Action::PlatformSpecific(action) => {
                write!(f, "Action::PlatformSpecific({:?})", action)
            }
            Action::Dnd(action) => write!(f, "Action::Dnd({:?})", action),
        }
    }
}

/// Creates a [`Task`] that exits the iced runtime.
///
/// This will normally close any application windows and
/// terminate the runtime loop.
pub fn exit<T>() -> Task<T> {
    task::effect(Action::Exit)
}

/// The appearance of a program.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Appearance {
    /// The background [`Color`] of the application.
    pub background_color: iced_core::Color,

    /// The default text [`Color`] of the application.
    pub text_color: iced_core::Color,

    /// The default icon [`Color`] of the application.
    pub icon_color: iced_core::Color,
}

/// The default style of a [`Program`].
pub trait DefaultStyle {
    /// Returns the default style of a [`Program`].
    fn default_style(&self) -> Appearance;
}

impl DefaultStyle for iced_core::Theme {
    fn default_style(&self) -> Appearance {
        default(self)
    }
}

/// The default [`Appearance`] of a [`Program`] with the built-in [`Theme`].
pub fn default(theme: &iced_core::Theme) -> Appearance {
    let palette = theme.extended_palette();

    Appearance {
        background_color: palette.background.base.color,
        text_color: palette.background.base.text,
        icon_color: palette.background.base.text,
    }
}
