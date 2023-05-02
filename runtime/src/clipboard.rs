//! Access the clipboard.
use window_clipboard::mime::{AllowedMimeTypes, AsMimeTypes};

use crate::core::clipboard::Kind;
use crate::futures::futures::channel::oneshot;
use crate::Task;

/// A clipboard action to be performed by some [`Task`].
///
/// [`Task`]: crate::Task
pub enum Action {
    /// Read the clipboard and produce `String` with the result.
    Read {
        /// The clipboard target.
        target: Kind,
        /// The channel to send the read contents.
        channel: oneshot::Sender<Option<String>>,
    },

    /// Write the given contents to the clipboard.
    Write {
        /// The clipboard target.
        target: Kind,
        /// The contents to be written.
        contents: String,
    },

    /// Write the given contents to the clipboard.
    WriteData(Box<dyn AsMimeTypes + Send + Sync + 'static>, Kind),

    #[allow(clippy::type_complexity)]
    /// Read the clipboard and produce `T` with the result.
    ReadData(
        Vec<String>,
        oneshot::Sender<Option<(Vec<u8>, String)>>,
        // Box<dyn Fn(Option<(Vec<u8>, String)>) -> T + Send + 'static>,
        Kind,
    ),
}

impl std::fmt::Debug for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read { channel: _, target } => {
                write!(f, "Action::Read{target:?}")
            }
            Self::Write {
                contents: _,
                target,
            } => {
                write!(f, "Action::Write({target:?})")
            }
            Self::WriteData(_, target) => {
                write!(f, "Action::WriteData({target:?})")
            }
            Self::ReadData(_, _, target) => {
                write!(f, "Action::ReadData({target:?})")
            }
        }
    }
}

/// Read the current contents of the clipboard.
pub fn read() -> Task<Option<String>> {
    Task::oneshot(|channel| {
        crate::Action::Clipboard(Action::Read {
            target: Kind::Standard,
            channel,
        })
    })
}

/// Read the current contents of the primary clipboard.
pub fn read_primary() -> Task<Option<String>> {
    Task::oneshot(|channel| {
        crate::Action::Clipboard(Action::Read {
            target: Kind::Primary,
            channel,
        })
    })
}

/// Write the given contents to the clipboard.
pub fn write<T>(contents: String) -> Task<T> {
    Task::effect(crate::Action::Clipboard(Action::Write {
        target: Kind::Standard,
        contents,
    }))
}

/// Write the given contents to the primary clipboard.
pub fn write_primary<Message>(contents: String) -> Task<Message> {
    Task::effect(crate::Action::Clipboard(Action::Write {
        target: Kind::Primary,
        contents,
    }))
}
/// Read the current contents of the clipboard.
pub fn read_data<T: AllowedMimeTypes>() -> Task<Option<T>> {
    Task::oneshot(|tx| {
        crate::Action::Clipboard(Action::ReadData(
            T::allowed().into(),
            tx,
            Kind::Standard,
        ))
    })
    .map(|d| d.and_then(|d| T::try_from(d).ok()))
}

/// Read the current contents of the clipboard.
// pub fn read_data<
//     T: AllowedMimeTypes + Send + Sync + 'static,
//     Message: Send + 'static,
// >(
//     f: impl Fn(Option<T>) -> Message + 'static + Send,
// ) -> Task<Option<(Vec<u8>, String)> {
//     Task::oneshot(|tx| {
//         crate::Action::Clipboard(Action::ReadData(
//             T::allowed().into(),
//             tx,
//             // Box::new(move |d| f(d.and_then(|d| T::try_from(d).ok()))),
//             Kind::Standard,
//         ))
//     })
// }

/// Write the given contents to the clipboard.
pub fn write_data<Message>(
    contents: impl AsMimeTypes + std::marker::Sync + std::marker::Send + 'static,
) -> Task<Message> {
    Task::effect(crate::Action::Clipboard(Action::WriteData(
        Box::new(contents),
        Kind::Standard,
    )))
}

/// Read from the primary clipboard
pub fn read_primary_data<T: AllowedMimeTypes>() -> Task<Option<T>> {
    Task::oneshot(|tx| {
        crate::Action::Clipboard(Action::ReadData(
            T::allowed().into(),
            tx,
            // Box::new(move |d| f(d.and_then(|d| T::try_from(d).ok()))),
            Kind::Primary,
        ))
    })
    .map(|d| d.and_then(|d| T::try_from(d).ok()))
}

/// Write the given contents to the clipboard.
pub fn write_primary_data<Message>(
    contents: impl AsMimeTypes + std::marker::Sync + std::marker::Send + 'static,
) -> Task<Message> {
    Task::effect(crate::Action::Clipboard(Action::WriteData(
        Box::new(contents),
        Kind::Primary,
    )))
}
