use crate::event_loop::state::SctkState;
use futures::lock::Mutex;
use sctk::{
    data_device_manager::data_source::DataSourceHandler,
    delegate_data_source,
    reexports::client::{
        protocol::{
            wl_data_device_manager::DndAction, wl_data_source::WlDataSource,
        },
        Connection, QueueHandle,
    },
};
use crate::sctk_event::{SctkEvent, DataSourceEvent};
use std::{fmt::Debug, sync::Arc, os::unix::io::AsRawFd};

impl<T> DataSourceHandler for SctkState<T> {
    fn accept_mime(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _source: &WlDataSource,
        mime: Option<String>,
    ) {
        self.sctk_events.push(SctkEvent::DataSource(DataSourceEvent::MimeAccepted(mime)));
    }

    fn send_request(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _source: &WlDataSource,
        mime: String,
        fd: wayland_backend::io_lifetimes::OwnedFd,
    ) {
        let raw_fd = fd.as_raw_fd();
        let fd = Arc::new(Mutex::new(fd));
        // XXX: the user should send a Finish action when they are done sending the data.
        self.sctk_events.push(SctkEvent::DataSource(DataSourceEvent::SendDndData {
            raw_fd,
            mime_type: mime,
            fd,
        }));
    }

    fn cancelled(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _source: &WlDataSource,
    ) {
        self.sctk_events.push(SctkEvent::DataSource(DataSourceEvent::DndCancelled));
    }

    fn dnd_dropped(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        source: &WlDataSource,
    ) {
        self.sctk_events.push(SctkEvent::DataSource(DataSourceEvent::DndDropPerformed));
    }

    fn dnd_finished(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        source: &WlDataSource,
    ) {
        // self.copy_paste_sources.iter().position(|s| s.inner() == source).map(|pos| {
        //     self.copy_paste_sources.remove(pos);
        // });
        // source.destroy();
    }

    fn action(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        source: &WlDataSource,
        action: DndAction,
    ) {
        self.sctk_events.push(crate::sctk_event::SctkEvent::DataSource(DataSourceEvent::DndActionAccepted(action)));
    }
}

delegate_data_source!(@<T: 'static + Debug> SctkState<T>);
