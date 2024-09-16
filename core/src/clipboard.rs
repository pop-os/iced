//! Access the clipboard.

use std::{any::Any, sync::Arc};

use dnd::{DndAction, DndDestinationRectangle, DndSurface};
use mime::{self, AllowedMimeTypes, AsMimeTypes, ClipboardStoreData};

use crate::{widget::tree::State, window, Element};

/// A buffer for short-term storage and transfer within and between
/// applications.
pub trait Clipboard {
    /// Reads the current content of the [`Clipboard`] as text.
    fn read(&self, kind: Kind) -> Option<String>;

    /// Writes the given text contents to the [`Clipboard`].
    fn write(&mut self, kind: Kind, contents: String);

    /// Consider using [`read_data`] instead
    /// Reads the current content of the [`Clipboard`] as text.
    fn read_data(
        &self,
        kind: Kind,
        _mimes: Vec<String>,
    ) -> Option<(Vec<u8>, String)> {
        None
    }

    /// Writes the given contents to the [`Clipboard`].
    fn write_data(
        &mut self,
        kind: Kind,
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
        &mut self,
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

/// The kind of [`Clipboard`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// The standard clipboard.
    Standard,
    /// The primary clipboard.
    ///
    /// Normally only present in X11 and Wayland.
    Primary,
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
    fn read(&self, _kind: Kind) -> Option<String> {
        None
    }

    fn write(&mut self, _kind: Kind, _contents: String) {}
}

/// Reads the current content of the [`Clipboard`].
pub fn read_data<T: AllowedMimeTypes>(
    clipboard: &mut dyn Clipboard,
) -> Option<T> {
    clipboard
        .read_data(Kind::Standard, T::allowed().into())
        .and_then(|data| T::try_from(data).ok())
}

/// Reads the current content of the primary [`Clipboard`].
pub fn read_primary_data<T: AllowedMimeTypes>(
    clipboard: &mut dyn Clipboard,
) -> Option<T> {
    clipboard
        .read_data(Kind::Primary, T::allowed().into())
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

/// A list of DnD destination rectangles.
#[derive(Debug, Clone)]
pub struct DndDestinationRectangles {
    /// The rectangle of the DnD destination.
    rectangles: Vec<DndDestinationRectangle>,
}

impl DndDestinationRectangles {
    /// Creates a new [`DndDestinationRectangles`].
    pub fn new() -> Self {
        Self {
            rectangles: Vec::new(),
        }
    }

    /// Creates a new [`DndDestinationRectangles`] with the given capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            rectangles: Vec::with_capacity(capacity),
        }
    }

    /// Pushes a new rectangle to the list of DnD destination rectangles.
    pub fn push(&mut self, rectangle: DndDestinationRectangle) {
        self.rectangles.push(rectangle);
    }

    /// Appends the list of DnD destination rectangles to the current list.
    pub fn append(&mut self, other: &mut Vec<DndDestinationRectangle>) {
        self.rectangles.append(other);
    }

    /// Returns the list of DnD destination rectangles.
    /// This consumes the [`DndDestinationRectangles`].
    pub fn into_rectangles(mut self) -> Vec<DndDestinationRectangle> {
        self.rectangles.reverse();
        self.rectangles
    }
}

impl AsRef<[DndDestinationRectangle]> for DndDestinationRectangles {
    fn as_ref(&self) -> &[DndDestinationRectangle] {
        &self.rectangles
    }
}
