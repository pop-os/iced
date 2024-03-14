//! Access the clipboard.

use window_clipboard::mime::{self, ClipboardStoreData};

/// A buffer for short-term storage and transfer within and between
/// applications.
#[allow(missing_debug_implementations)]
pub struct Clipboard {
    state: State,
}

enum State {
    Connected(window_clipboard::Clipboard),
    Unavailable,
}

impl Clipboard {
    /// Creates a new [`Clipboard`] for the given window.
    pub fn connect(window: &winit::window::Window) -> Clipboard {
        #[allow(unsafe_code)]
        let state = unsafe { window_clipboard::Clipboard::connect(window) }
            .ok()
            .map(State::Connected)
            .unwrap_or(State::Unavailable);

        Clipboard { state }
    }

    /// Creates a new [`Clipboard`] that isn't associated with a window.
    /// This clipboard will never contain a copied value.
    pub fn unconnected() -> Clipboard {
        Clipboard {
            state: State::Unavailable,
        }
    }

    /// Reads the current content of the [`Clipboard`] as text.
    pub fn read(&self) -> Option<String> {
        match &self.state {
            State::Connected(clipboard) => clipboard.read().ok(),
            State::Unavailable => None,
        }
    }

    /// Writes the given text contents to the [`Clipboard`].
    pub fn write(&mut self, contents: String) {
        match &mut self.state {
            State::Connected(clipboard) => match clipboard.write(contents) {
                Ok(()) => {}
                Err(error) => {
                    log::warn!("error writing to clipboard: {error}");
                }
            },
            State::Unavailable => {}
        }
    }
}

impl crate::core::Clipboard for Clipboard {
    fn read(&self) -> Option<String> {
        self.read()
    }

    fn write(&mut self, contents: String) {
        self.write(contents);
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
