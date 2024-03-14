//! Access the clipboard.
use window_clipboard::mime::{AllowedMimeTypes, AsMimeTypes};

use crate::command::{self, Command};
use crate::futures::MaybeSend;

use std::fmt;

/// A clipboard action to be performed by some [`Command`].
///
/// [`Command`]: crate::Command
pub enum Action<T> {
    /// Read the clipboard and produce `T` with the result.
    Read(Box<dyn Fn(Option<String>) -> T>),

    /// Write the given contents to the clipboard.
    Write(String),

    /// Write the given contents to the clipboard.
    WriteData(Box<dyn AsMimeTypes + Send + Sync + 'static>),

    /// Read the clipboard and produce `T` with the result.
    ReadData(Vec<String>, Box<dyn Fn(Option<(Vec<u8>, String)>) -> T>),

    /// Read the clipboard and produce `T` with the result.
    ReadPrimary(Box<dyn Fn(Option<String>) -> T>),

    /// Write the given contents to the clipboard.
    WritePrimary(String),

    /// Write the given contents to the clipboard.
    WritePrimaryData(Box<dyn AsMimeTypes + Send + Sync + 'static>),

    /// Read the clipboard and produce `T` with the result.
    ReadPrimaryData(Vec<String>, Box<dyn Fn(Option<(Vec<u8>, String)>) -> T>),
}

impl<T> Action<T> {
    /// Maps the output of a clipboard [`Action`] using the provided closure.
    pub fn map<A>(
        self,
        f: impl Fn(T) -> A + 'static + MaybeSend + Sync,
    ) -> Action<A>
    where
        T: 'static,
    {
        match self {
            Self::Read(o) => Action::Read(Box::new(move |s| f(o(s)))),
            Self::Write(content) => Action::Write(content),
            Self::WriteData(content) => Action::WriteData(content),
            Self::ReadData(a, o) => {
                Action::ReadData(a, Box::new(move |s| f(o(s))))
            }
            Self::ReadPrimary(o) => {
                Action::ReadPrimary(Box::new(move |s| f(o(s))))
            }
            Self::WritePrimary(content) => Action::WritePrimary(content),
            Self::WritePrimaryData(content) => {
                Action::WritePrimaryData(content)
            }
            Self::ReadPrimaryData(a, o) => {
                Action::ReadPrimaryData(a, Box::new(move |s| f(o(s))))
            }
        }
    }
}

impl<T> fmt::Debug for Action<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(_) => write!(f, "Action::Read"),
            Self::Write(_) => write!(f, "Action::Write"),
            Self::WriteData(_) => write!(f, "Action::WriteData"),
            Self::ReadData(_, _) => write!(f, "Action::ReadData"),
            Self::ReadPrimary(_) => write!(f, "Action::ReadPrimary"),
            Self::WritePrimary(_) => write!(f, "Action::WritePrimary"),
            Self::WritePrimaryData(_) => write!(f, "Action::WritePrimaryData"),
            Self::ReadPrimaryData(_, _) => write!(f, "Action::ReadPrimaryData"),
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

/// Read the current contents of the clipboard.
pub fn read_data<T: AllowedMimeTypes + Send + Sync + 'static, Message>(
    f: impl Fn(Option<T>) -> Message + 'static,
) -> Command<Message> {
    Command::single(command::Action::Clipboard(Action::ReadData(
        T::allowed().into(),
        Box::new(move |d| f(d.and_then(|d| T::try_from(d).ok()))),
    )))
}

/// Write the given contents to the clipboard.
pub fn write_data<Message>(
    contents: impl AsMimeTypes + std::marker::Sync + std::marker::Send + 'static,
) -> Command<Message> {
    Command::single(command::Action::Clipboard(Action::WriteData(Box::new(
        contents,
    ))))
}

/// Read the current contents of the clipboard.
pub fn read_primary_data<
    T: AllowedMimeTypes + Send + Sync + 'static,
    Message,
>(
    f: impl Fn(Option<T>) -> Message + 'static,
) -> Command<Message> {
    Command::single(command::Action::Clipboard(Action::ReadPrimaryData(
        T::allowed().into(),
        Box::new(move |d| f(d.and_then(|d| T::try_from(d).ok()))),
    )))
}

/// Write the given contents to the clipboard.
pub fn write_primary_data<Message>(
    contents: impl AsMimeTypes + std::marker::Sync + std::marker::Send + 'static,
) -> Command<Message> {
    Command::single(command::Action::Clipboard(Action::WritePrimaryData(
        Box::new(contents),
    )))
}
