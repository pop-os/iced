//! Interact with the window of your application.
use crate::command::{self, Command};
use iced_native::window;

pub use window::{Event, Mode};

/// Begins dragging the window while the left mouse button is held.
pub fn drag<Message>() -> Command<Message> {
    Command::single(command::Action::Window(window::Action::Drag))
}

/// Maximize the window
pub fn maximize<Message>() -> Command<Message> {
    Command::single(command::Action::Window(window::Action::Maximize))
}

/// Minimize the window
pub fn minimize<Message>() -> Command<Message> {
    Command::single(command::Action::Window(window::Action::Minimize))
}

/// Moves a window to the given logical coordinates.
pub fn move_to<Message>(x: i32, y: i32) -> Command<Message> {
    Command::single(command::Action::Window(window::Action::Move { x, y }))
}

/// Resizes the window to the given logical dimensions.
pub fn resize<Message>(width: u32, height: u32) -> Command<Message> {
    Command::single(command::Action::Window(window::Action::Resize {
        width,
        height,
    }))
}

/// Begins resizing a window with the mouse.
pub fn resize_mouse<Message>() -> Command<Message> {
    Command::single(command::Action::Window(window::Action::ResizeMouse))
}

/// Sets the [`Mode`] of the window.
pub fn set_mode<Message>(mode: Mode) -> Command<Message> {
    Command::single(command::Action::Window(window::Action::SetMode(mode)))
}

/// Fetches the current [`Mode`] of the window.
pub fn fetch_mode<Message>(
    f: impl FnOnce(Mode) -> Message + 'static,
) -> Command<Message> {
    Command::single(command::Action::Window(window::Action::FetchMode(
        Box::new(f),
    )))
}
