//! Access the clipboard.

use window_clipboard::mime::{self, AllowedMimeTypes, ClipboardStoreData};

/// A buffer for short-term storage and transfer within and between
/// applications.
pub trait Clipboard {
    /// Reads the current content of the [`Clipboard`] as text.
    fn read(&self) -> Option<String>;

    /// Writes the given text contents to the [`Clipboard`].
    fn write(&mut self, contents: String);

    /// Read the current content of the primary [`Clipboard`] as text.
    fn read_primary(&self) -> Option<String> {
        None
    }

    /// Writes the given text contents to the primary [`Clipboard`].
    fn write_primary(&mut self, _contents: String) {}

    /// Consider using [`read_data`] instead
    /// Reads the current content of the [`Clipboard`] as text.
    fn read_data(&self, _mimes: Vec<String>) -> Option<(Vec<u8>, String)> {
        None
    }

    /// Writes the given contents to the [`Clipboard`].
    fn write_data(
        &mut self,
        _contents: ClipboardStoreData<
            Box<dyn Send + Sync + 'static + mime::AsMimeTypes>,
        >,
    ) {
    }

    /// Consider using [`read_primary_data`] instead
    /// Reads the current content of the primary [`Clipboard`] as text.
    fn read_primary_data(
        &self,
        _mimes: Vec<String>,
    ) -> Option<(Vec<u8>, String)> {
        None
    }

    /// Writes the given text contents to the primary [`Clipboard`].
    fn write_primary_data(
        &mut self,
        _contents: ClipboardStoreData<
            Box<dyn Send + Sync + 'static + mime::AsMimeTypes>,
        >,
    ) {
    }
}

/// A null implementation of the [`Clipboard`] trait.
#[derive(Debug, Clone, Copy)]
pub struct Null;

impl Clipboard for Null {
    fn read(&self) -> Option<String> {
        None
    }

    fn write(&mut self, _contents: String) {}
}

/// Reads the current content of the [`Clipboard`].
pub fn read_data<T: AllowedMimeTypes>(
    clipboard: &mut dyn Clipboard,
) -> Option<T> {
    clipboard
        .read_data(T::allowed().into())
        .and_then(|data| T::try_from(data).ok())
}

/// Reads the current content of the primary [`Clipboard`].
pub fn read_primary_data<T: AllowedMimeTypes>(
    clipboard: &mut dyn Clipboard,
) -> Option<T> {
    clipboard
        .read_data(T::allowed().into())
        .and_then(|data| T::try_from(data).ok())
}
