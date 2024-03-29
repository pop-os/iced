//! Access the clipboard.

use std::{any::Any, borrow::Cow, sync::Arc};

use dnd::{DndAction, DndDestinationRectangle, DndSurface};
use mime::{self, AllowedMimeTypes, AsMimeTypes, ClipboardStoreData};

use crate::{widget::tree::State, window, Element};

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

    /// Starts a DnD operation.
    fn register_dnd_destination(
        &self,
        _surface: DndSurface,
        _rectangles: Vec<DndDestinationRectangle>,
    ) {
    }

    /// Set the final action for the DnD operation.
    /// Only should be done if it is requested.
    fn set_action(&self, _action: DndAction) {}

    /// Registers Dnd destinations
    fn start_dnd(
        &self,
        _internal: bool,
        _source_surface: Option<DndSource>,
        _icon_surface: Option<Box<dyn Any>>,
        _content: Box<dyn AsMimeTypes + Send + 'static>,
        _actions: DndAction,
    ) {
    }

    /// Ends a DnD operation.
    fn end_dnd(&self) {}

    /// Consider using [`peek_dnd`] instead
    /// Peeks the data on the DnD with a specific mime type.
    /// Will return an error if there is no ongoing DnD operation.
    fn peek_dnd(&self, _mime: String) -> Option<(Vec<u8>, String)> {
        None
    }
}

/// Starts a DnD operation.
/// icon surface is a tuple of the icon element and optionally the icon element state.
pub fn start_dnd<T: 'static, R: 'static, M: 'static>(
    clipboard: &mut dyn Clipboard,
    internal: bool,
    source_surface: Option<DndSource>,
    icon_surface: Option<(Element<'static, M, T, R>, State)>,
    content: Box<dyn AsMimeTypes + Send + 'static>,
    actions: DndAction,
) {
    clipboard.start_dnd(
        internal,
        source_surface,
        icon_surface.map(|i| {
            let i: Box<dyn Any> = Box::new(Arc::new(i));
            i
        }),
        content,
        actions,
    );
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

/// Reads the current content of the primary [`Clipboard`].
pub fn peek_dnd<T: AllowedMimeTypes>(
    clipboard: &mut dyn Clipboard,
    mime: Option<String>,
) -> Option<T> {
    let Some(mime) = mime.or_else(|| T::allowed().first().cloned().into())
    else {
        return None;
    };
    clipboard
        .peek_dnd(mime)
        .and_then(|data| T::try_from(data).ok())
}

/// Source of a DnD operation.
#[derive(Debug, Clone)]
pub enum DndSource {
    /// A widget is the source of the DnD operation.
    Widget(crate::id::Id),
    /// A surface is the source of the DnD operation.
    Surface(window::Id),
}
