//! Access the clipboard.
pub use iced_runtime::clipboard::Action;

use iced_runtime::command::{self, Command};
use raw_window_handle::HasDisplayHandle;
use window_clipboard::mime::{self, ClipboardStoreData};

/// A buffer for short-term storage and transfer within and between
/// applications.
#[allow(missing_debug_implementations)]
pub struct Clipboard {
    pub(crate) state: State,
}

pub(crate) enum State {
    Connected(window_clipboard::Clipboard),
    Unavailable,
}

impl Clipboard {
    pub unsafe fn connect(display: &impl HasDisplayHandle) -> Clipboard {
        let context = window_clipboard::Clipboard::connect(display);

        Clipboard {
            state: context.map(State::Connected).unwrap_or(State::Unavailable),
        }
    }

    pub(crate) fn state(&self) -> &State {
        &self.state
    }

    /// Creates a new [`Clipboard`] that isn't associated with a window.
    /// This clipboard will never contain a copied value.
    pub fn unconnected() -> Clipboard {
        Clipboard {
            state: State::Unavailable,
        }
    }
}

impl iced_runtime::core::clipboard::Clipboard for Clipboard {
    fn read(&self) -> Option<String> {
        match &self.state {
            State::Connected(clipboard) => clipboard.read().ok(),
            State::Unavailable => None,
        }
    }

    fn write(&mut self, contents: String) {
        match &mut self.state {
            State::Connected(clipboard) => _ = clipboard.write(contents),
            State::Unavailable => {}
        }
    }

    /// Read the current content of the primary [`Clipboard`] as text.
    fn read_primary(&self) -> Option<String> {
        match &self.state {
            State::Connected(clipboard) => {
                clipboard.read_primary().and_then(|res| res.ok())
            }
            State::Unavailable => None,
        }
    }

    /// Writes the given text contents to the primary [`Clipboard`].
    fn write_primary(&mut self, contents: String) {
        match &mut self.state {
            State::Connected(clipboard) => {
                _ = clipboard.write_primary(contents)
            }
            State::Unavailable => {}
        }
    }

    /// Consider using [`read_data`] instead
    /// Reads the current content of the [`Clipboard`] as text.
    fn read_data(&self, mimes: Vec<String>) -> Option<(Vec<u8>, String)> {
        match &self.state {
            State::Connected(clipboard) => {
                clipboard.read_raw(mimes).and_then(|res| res.ok())
            }
            State::Unavailable => None,
        }
    }

    /// Writes the given contents to the [`Clipboard`].
    fn write_data(
        &mut self,
        contents: ClipboardStoreData<
            Box<dyn Send + Sync + 'static + mime::AsMimeTypes>,
        >,
    ) {
        match &mut self.state {
            State::Connected(clipboard) => _ = clipboard.write_data(contents),
            State::Unavailable => {}
        }
    }

    /// Consider using [`read_primary_data`] instead
    /// Reads the current content of the primary [`Clipboard`] as text.
    fn read_primary_data(
        &self,
        mimes: Vec<String>,
    ) -> Option<(Vec<u8>, String)> {
        match &self.state {
            State::Connected(clipboard) => {
                clipboard.read_primary_raw(mimes).and_then(|res| res.ok())
            }
            State::Unavailable => None,
        }
    }

    /// Writes the given text contents to the primary [`Clipboard`].
    fn write_primary_data(
        &mut self,
        contents: ClipboardStoreData<
            Box<dyn Send + Sync + 'static + mime::AsMimeTypes>,
        >,
    ) {
        match &mut self.state {
            State::Connected(clipboard) => {
                _ = clipboard.write_primary_data(contents)
            }
            State::Unavailable => {}
        }
    }
}

/// Read the current contents of the clipboard.
pub fn read<Message>(
    f: impl Fn(Option<String>) -> Message + 'static,
) -> Command<Message> {
    Command::single(command::Action::Clipboard(Action::Read(Box::new(f))))
}

/// Write the given contents to the clipboard.
pub fn write<Message>(contents: String) -> Command<Message> {
    Command::single(command::Action::Clipboard(Action::Write(contents)))
}
