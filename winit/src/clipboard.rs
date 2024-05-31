//! Access the clipboard.

use std::{any::Any, borrow::Cow};

use crate::futures::futures::Sink;
use dnd::{DndAction, DndDestinationRectangle, DndSurface, Icon};
use iced_style::core::clipboard::DndSource;
use window_clipboard::{
    dnd::DndProvider,
    mime::{self, ClipboardData, ClipboardStoreData},
};

use crate::{application::UserEventWrapper, Proxy};

/// A buffer for short-term storage and transfer within and between
/// applications.
#[allow(missing_debug_implementations)]
pub struct Clipboard<M: 'static> {
    state: State<M>,
}

enum State<M: 'static> {
    Connected(window_clipboard::Clipboard, Proxy<UserEventWrapper<M>>),
    Unavailable,
}

impl<M: Send + 'static> Clipboard<M> {
    /// Creates a new [`Clipboard`] for the given window.
    pub fn connect(
        window: &winit::window::Window,
        proxy: Proxy<UserEventWrapper<M>>,
    ) -> Clipboard<M> {
        #[allow(unsafe_code)]
        let state = unsafe { window_clipboard::Clipboard::connect(window) }
            .ok()
            .map(|c| (c, proxy.clone()))
            .map(|c| State::Connected(c.0, c.1))
            .unwrap_or(State::Unavailable);

        if let State::Connected(clipboard, _) = &state {
            clipboard.init_dnd(Box::new(proxy));
        }

        Clipboard { state }
    }

    /// Creates a new [`Clipboard`] that isn't associated with a window.
    /// This clipboard will never contain a copied value.
    pub fn unconnected() -> Clipboard<M> {
        Clipboard {
            state: State::Unavailable,
        }
    }

    /// Reads the current content of the [`Clipboard`] as text.
    pub fn read(&self) -> Option<String> {
        match &self.state {
            State::Connected(clipboard, _) => clipboard.read().ok(),
            State::Unavailable => None,
        }
    }

    /// Writes the given text contents to the [`Clipboard`].
    pub fn write(&mut self, contents: String) {
        match &mut self.state {
            State::Connected(clipboard, _) => match clipboard.write(contents) {
                Ok(()) => {}
                Err(error) => {
                    log::warn!("error writing to clipboard: {error}");
                }
            },
            State::Unavailable => {}
        }
    }

    /// Reads the current content of the Primary as text.
    pub fn read_primary(&self) -> Option<String> {
        match &self.state {
            State::Connected(clipboard, _) => {
                clipboard.read_primary().and_then(|res| res.ok())
            }
            State::Unavailable => None,
        }
    }

    /// Writes the given text contents to the Primary.
    pub fn write_primary(&mut self, contents: String) {
        match &mut self.state {
            State::Connected(clipboard, _) => {
                match clipboard.write_primary(contents) {
                    Some(Ok(())) => {}
                    Some(Err(error)) => {
                        log::warn!("error writing to clipboard: {error}");
                    }
                    None => {} //Primary not available
                }
            }
            State::Unavailable => {}
        }
    }

    //
    pub(crate) fn start_dnd_winit(
        &self,
        internal: bool,
        source_surface: DndSurface,
        icon_surface: Option<Icon>,
        content: Box<dyn mime::AsMimeTypes + Send + 'static>,
        actions: DndAction,
    ) {
        match &self.state {
            State::Connected(clipboard, _) => {
                _ = clipboard.start_dnd(
                    internal,
                    source_surface,
                    icon_surface,
                    content,
                    actions,
                )
            }
            State::Unavailable => {}
        }
    }
}

impl<M> crate::core::Clipboard for Clipboard<M> {
    fn read(&self) -> Option<String> {
        match &self.state {
            State::Connected(clipboard, _) => clipboard.read().ok(),
            State::Unavailable => None,
        }
    }

    fn write(&mut self, contents: String) {
        match &mut self.state {
            State::Connected(clipboard, _) => _ = clipboard.write(contents),
            State::Unavailable => {}
        }
    }

    fn read_primary(&self) -> Option<String> {
        match &self.state {
            State::Connected(clipboard, _) => {
                clipboard.read_primary().and_then(|res| res.ok())
            }
            State::Unavailable => None,
        }
    }

    fn write_primary(&mut self, contents: String) {
        match &mut self.state {
            State::Connected(clipboard, _) => {
                _ = clipboard.write_primary(contents)
            }
            State::Unavailable => {}
        }
    }

    fn read_data(&self, mimes: Vec<String>) -> Option<(Vec<u8>, String)> {
        match &self.state {
            State::Connected(clipboard, _) => {
                clipboard.read_raw(mimes).and_then(|res| res.ok())
            }
            State::Unavailable => None,
        }
    }

    fn write_data(
        &mut self,
        contents: ClipboardStoreData<
            Box<dyn Send + Sync + 'static + mime::AsMimeTypes>,
        >,
    ) {
        match &mut self.state {
            State::Connected(clipboard, _) => {
                _ = clipboard.write_data(contents)
            }
            State::Unavailable => {}
        }
    }

    fn read_primary_data(
        &self,
        mimes: Vec<String>,
    ) -> Option<(Vec<u8>, String)> {
        match &self.state {
            State::Connected(clipboard, _) => {
                clipboard.read_primary_raw(mimes).and_then(|res| res.ok())
            }
            State::Unavailable => None,
        }
    }

    fn write_primary_data(
        &mut self,
        contents: ClipboardStoreData<
            Box<dyn Send + Sync + 'static + mime::AsMimeTypes>,
        >,
    ) {
        match &mut self.state {
            State::Connected(clipboard, _) => {
                _ = clipboard.write_primary_data(contents)
            }
            State::Unavailable => {}
        }
    }

    fn start_dnd(
        &self,
        internal: bool,
        source_surface: Option<DndSource>,
        icon_surface: Option<Box<dyn Any>>,
        content: Box<dyn mime::AsMimeTypes + Send + 'static>,
        actions: DndAction,
    ) {
        match &self.state {
            State::Connected(_, tx) => {
                tx.raw.send_event(UserEventWrapper::StartDnd {
                    internal,
                    source_surface,
                    icon_surface,
                    content,
                    actions,
                });
            }
            State::Unavailable => {}
        }
    }

    fn register_dnd_destination(
        &self,
        surface: DndSurface,
        rectangles: Vec<DndDestinationRectangle>,
    ) {
        match &self.state {
            State::Connected(clipboard, _) => {
                _ = clipboard.register_dnd_destination(surface, rectangles)
            }
            State::Unavailable => {}
        }
    }

    fn end_dnd(&self) {
        match &self.state {
            State::Connected(clipboard, _) => _ = clipboard.end_dnd(),
            State::Unavailable => {}
        }
    }

    fn peek_dnd(&self, mime: String) -> Option<(Vec<u8>, String)> {
        match &self.state {
            State::Connected(clipboard, _) => clipboard
                .peek_offer::<ClipboardData>(Some(Cow::Owned(mime)))
                .ok()
                .map(|res| (res.0, res.1)),
            State::Unavailable => None,
        }
    }

    fn set_action(&self, action: DndAction) {
        match &self.state {
            State::Connected(clipboard, _) => _ = clipboard.set_action(action),
            State::Unavailable => {}
        }
    }
}
