//! Access the clipboard.

use std::{any::Any, borrow::Cow};

use crate::core::clipboard::DndSource;
use crate::core::clipboard::Kind;
use crate::futures::futures::Sink;
use crate::runtime::Action;
use crate::Proxy;
use std::sync::Arc;
use winit::window::{Window, WindowId};

use dnd::{DndAction, DndDestinationRectangle, DndSurface, Icon};
use window_clipboard::{
    dnd::DndProvider,
    mime::{self, ClipboardData, ClipboardStoreData},
};

/// A buffer for short-term storage and transfer within and between
/// applications.
#[allow(missing_debug_implementations)]
pub struct Clipboard {
    state: State,
}

enum State {
    Connected {
        clipboard: window_clipboard::Clipboard,
        sender: ControlSender,
        // Held until drop to satisfy the safety invariants of
        // `window_clipboard::Clipboard`.
        //
        // Note that the field ordering is load-bearing.
        #[allow(dead_code)]
        window: Arc<Window>,
    },
    Unavailable,
}

#[derive(Debug, Clone)]
pub(crate) struct ControlSender(
    pub(crate)  iced_futures::futures::channel::mpsc::UnboundedSender<
        crate::program::Control,
    >,
);

impl dnd::Sender<DndSurface> for ControlSender {
    fn send(
        &self,
        event: dnd::DndEvent<DndSurface>,
    ) -> Result<(), std::sync::mpsc::SendError<dnd::DndEvent<DndSurface>>> {
        self.0
            .unbounded_send(crate::program::Control::Dnd(event))
            .map_err(|_err| {
                std::sync::mpsc::SendError(dnd::DndEvent::Offer(
                    None,
                    dnd::OfferEvent::Leave,
                ))
            })
    }
}

impl Clipboard {
    /// Creates a new [`Clipboard`] for the given window.
    pub fn connect(window: Arc<Window>, proxy: ControlSender) -> Clipboard {
        #[allow(unsafe_code)]
        let state =
            unsafe { window_clipboard::Clipboard::connect(window.as_ref()) }
                .ok()
                .map(|c| State::Connected {
                    clipboard: c,
                    sender: proxy.clone(),
                    window,
                })
                .unwrap_or(State::Unavailable);

        #[cfg(target_os = "linux")]
        if let State::Connected { clipboard, .. } = &state {
            clipboard.init_dnd(Box::new(proxy));
        }

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
    pub fn read(&self, kind: Kind) -> Option<String> {
        match &self.state {
            State::Connected { clipboard, .. } => match kind {
                Kind::Standard => clipboard.read().ok(),
                Kind::Primary => clipboard.read_primary().and_then(Result::ok),
            },
            State::Unavailable => None,
        }
    }

    /// Writes the given text contents to the [`Clipboard`].
    pub fn write(&mut self, kind: Kind, contents: String) {
        match &mut self.state {
            State::Connected { clipboard, .. } => {
                let result = match kind {
                    Kind::Standard => clipboard.write(contents),
                    Kind::Primary => {
                        clipboard.write_primary(contents).unwrap_or(Ok(()))
                    }
                };

                match result {
                    Ok(()) => {}
                    Err(error) => {
                        log::warn!("error writing to clipboard: {error}");
                    }
                }
            }
            State::Unavailable => {}
        }
    }

    /// Returns the identifier of the window used to create the [`Clipboard`], if any.
    pub fn window_id(&self) -> Option<WindowId> {
        match &self.state {
            State::Connected { window, .. } => Some(window.id()),
            State::Unavailable => None,
        }
    }

    pub(crate) fn start_dnd_winit(
        &self,
        internal: bool,
        source_surface: DndSurface,
        icon_surface: Option<Icon>,
        content: Box<dyn mime::AsMimeTypes + Send + 'static>,
        actions: DndAction,
    ) {
        match &self.state {
            State::Connected { clipboard, .. } => {
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

impl crate::core::Clipboard for Clipboard {
    fn read(&self, kind: Kind) -> Option<String> {
        match (&self.state, kind) {
            (State::Connected { clipboard, .. }, Kind::Standard) => {
                clipboard.read().ok()
            }
            (State::Connected { clipboard, .. }, Kind::Primary) => {
                clipboard.read_primary().and_then(|res| res.ok())
            }
            (State::Unavailable, _) => None,
        }
    }

    fn write(&mut self, kind: Kind, contents: String) {
        match (&mut self.state, kind) {
            (State::Connected { clipboard, .. }, Kind::Standard) => {
                _ = clipboard.write(contents)
            }
            (State::Connected { clipboard, .. }, Kind::Primary) => {
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
            (State::Connected { clipboard, .. }, Kind::Standard) => {
                clipboard.read_raw(mimes).and_then(|res| res.ok())
            }
            (State::Connected { clipboard, .. }, Kind::Primary) => {
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
            (State::Connected { clipboard, .. }, Kind::Standard) => {
                _ = clipboard.write_data(contents)
            }
            (State::Connected { clipboard, .. }, Kind::Primary) => {
                _ = clipboard.write_primary_data(contents)
            }
            (State::Unavailable, _) => {}
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
            State::Connected { sender, .. } => {
                // TODO
                // tx.raw.send_event(UserEventWrapper::StartDnd {
                //     internal,
                //     source_surface,
                //     icon_surface,
                //     content,
                //     actions,
                // });
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
            State::Connected { clipboard, .. } => {
                _ = clipboard.register_dnd_destination(surface, rectangles)
            }
            State::Unavailable => {}
        }
    }

    fn end_dnd(&self) {
        match &self.state {
            State::Connected { clipboard, .. } => _ = clipboard.end_dnd(),
            State::Unavailable => {}
        }
    }

    fn peek_dnd(&self, mime: String) -> Option<(Vec<u8>, String)> {
        match &self.state {
            State::Connected { clipboard, .. } => clipboard
                .peek_offer::<ClipboardData>(Some(Cow::Owned(mime)))
                .ok()
                .map(|res| (res.0, res.1)),
            State::Unavailable => None,
        }
    }

    fn set_action(&self, action: DndAction) {
        match &self.state {
            State::Connected { clipboard, .. } => {
                _ = clipboard.set_action(action)
            }
            State::Unavailable => {}
        }
    }
}
