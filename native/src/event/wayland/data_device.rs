use sctk::{data_device_manager::{data_offer::DragOffer, ReadPipe}, reexports::client::protocol::wl_data_device_manager::DndAction};
use std::{sync::{Arc}, os::fd::{OwnedFd, RawFd}};
use iced_futures::futures::lock::Mutex;


/// Dnd Offer events
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DndOfferEvent {
    /// A DnD offer has been introduced with the given mime types.
    Enter(Vec<String>),
    /// The DnD device has left.
    Leave,
    /// Drag and Drop Motion event.
    Motion {
        /// x coordinate of the pointer
        x: i32,
        /// y coordinate of the pointer
        y: i32,
        /// time of the event
        time: u32,
    },
    /// The offered actions for the current DnD offer
    Actions(DndAction),
    /// Dnd Drop event
    DropPerformed,
    /// Read the Selection data
    ReadSelectionData(ReadData),
    /// Read the DnD data
    ReadData(ReadData),
    /// Selection Offer
    /// a selection offer has been introduced with the given mime types.
    SelectionOffer(Vec<String>),
}

/// Selection Offer events
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectionOfferEvent {
    /// a selection offer has been introduced with the given mime types.
    Offer(Vec<String>),
    /// Read the Selection data
    ReadData(ReadData),
}

/// A ReadPipe and the mime type of the data.
#[derive(Debug, Clone)]
pub struct ReadData {
    raw_fd: RawFd,
    /// mime type of the data
    pub mime_type: String,
    /// The pipe to read the data from
    pub fd: Arc<Mutex<ReadPipe>>,
}

impl ReadData {
    /// Create a new ReadData
    pub fn new(mime_type: String, raw_fd: RawFd, fd: Arc<Mutex<ReadPipe>>) -> Self {
        Self {
            raw_fd,
            mime_type,
            fd,
        }
    }
}


/// Data Source events
/// Includes drag and drop events and clipboard events
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataSourceEvent {
    /// A Dnd action was selected by the compositor for your source.
    DndActionAccepted(DndAction),
    /// A mime type was accepted by a client for your source.
    MimeAccepted(Option<String>),
    /// Some client has requested the DnD data.
    /// This is used to send the data to the client.
    SendDndData(WriteData),
    /// Some client has requested the selection data.
    /// This is used to send the data to the client.
    SendSelectionData(WriteData),
    /// The data source has been cancelled and is no longer valid.
    /// This may be sent for multiple reasons
    Cancelled,
    /// Dnd Finished
    DndFinished,
    /// Dnd Drop event
    DndDropPerformed,
}

/// A WriteData and the mime type of the data to be written.
#[derive(Debug, Clone)]
pub struct WriteData {
    raw_fd: RawFd,
    /// mime type of the data
    pub mime_type: String,
    /// The fd to write the data to
    pub fd: Arc<Mutex<OwnedFd>>,
}

impl WriteData {
    /// Create a new WriteData
    pub fn new(mime_type: String, raw_fd: RawFd, fd: Arc<Mutex<OwnedFd>>) -> Self {
        Self {
            raw_fd,
            mime_type,
            fd,
        }
    }
}

impl PartialEq for WriteData {
    fn eq(&self, other: &Self) -> bool {
        self.raw_fd == other.raw_fd
    }
}

impl Eq for WriteData {}

impl PartialEq for ReadData {
    fn eq(&self, other: &Self) -> bool {
        self.raw_fd == other.raw_fd
    }
}

impl Eq for ReadData {}

