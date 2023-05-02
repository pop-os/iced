//! Access the clipboard.
use crate::core::clipboard::Kind;
use iced_runtime::{self, Task};
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
    fn read(&self, kind: Kind) -> Option<String> {
        match (&self.state, kind) {
            (State::Connected(clipboard), Kind::Standard) => {
                clipboard.read().ok()
            }
            (State::Connected(clipboard), Kind::Primary) => {
                clipboard.read_primary().and_then(|res| res.ok())
            }
            (State::Unavailable, _) => None,
        }
    }

    fn write(&mut self, kind: Kind, contents: String) {
        match (&mut self.state, kind) {
            (State::Connected(clipboard), Kind::Standard) => {
                _ = clipboard.write(contents)
            }
            (State::Connected(clipboard), Kind::Primary) => {
                _ = clipboard.write_primary(contents)
            }
            (State::Unavailable, _) => {}
        }
    }

    fn read_data(
        &self,
        kind: Kind,
        mimes: Vec<String>,
    ) -> Option<(Vec<u8>, String)> {
        match (&self.state, kind) {
            (State::Connected(clipboard), Kind::Standard) => {
                clipboard.read_raw(mimes).and_then(|res| res.ok())
            }
            (State::Connected(clipboard), Kind::Primary) => {
                clipboard.read_primary_raw(mimes).and_then(|res| res.ok())
            }
            (State::Unavailable, _) => None,
        }
    }

    fn write_data(
        &mut self,
        kind: Kind,
        contents: ClipboardStoreData<
            Box<dyn Send + Sync + 'static + mime::AsMimeTypes>,
        >,
    ) {
        match (&mut self.state, kind) {
            (State::Connected(clipboard), Kind::Standard) => {
                _ = clipboard.write_data(contents)
            }
            (State::Connected(clipboard), Kind::Primary) => {
                _ = clipboard.write_primary_data(contents)
            }
            (State::Unavailable, _) => {}
        }
    }
}
