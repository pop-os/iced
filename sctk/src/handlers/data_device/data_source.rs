use crate::event_loop::state::SctkState;
use crate::sctk_event::{DataSourceEvent, SctkEvent};
use sctk::data_device_manager::WritePipe;
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
use std::{
    fmt::Debug,
    sync::{Arc, Mutex},
};

impl<T> DataSourceHandler for SctkState<T> {
    fn accept_mime(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _source: &WlDataSource,
        mime: Option<String>,
    ) {
        self.sctk_events
            .push(SctkEvent::DataSource(DataSourceEvent::MimeAccepted(mime)));
    }

    fn send_request(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        source: &WlDataSource,
        mime: String,
        pipe: WritePipe,
    ) {
        if let Some(source) = self
            .selection_source
            .as_mut()
            .filter(|s| s.source.inner() == source)
        {
            source.pipe = Some(pipe);
            self.sctk_events.push(SctkEvent::DataSource(
                DataSourceEvent::SendSelectionData { mime_type: mime },
            ));
        } else if let Some(source) = self.dnd_source.as_mut().filter(|s| {
            s.source
                .as_ref()
                .map(|s| (s.inner() == source))
                .unwrap_or(false)
        }) {
            source.pipe = Some(pipe);
            self.sctk_events.push(SctkEvent::DataSource(
                DataSourceEvent::SendDndData { mime_type: mime },
            ));
        }
    }

    fn cancelled(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _source: &WlDataSource,
    ) {
        self.sctk_events
            .push(SctkEvent::DataSource(DataSourceEvent::DndCancelled));
    }

    fn dnd_dropped(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _source: &WlDataSource,
    ) {
        self.sctk_events
            .push(SctkEvent::DataSource(DataSourceEvent::DndDropPerformed));
    }

    fn dnd_finished(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _source: &WlDataSource,
    ) {
        self.sctk_events
            .push(SctkEvent::DataSource(DataSourceEvent::DndFinished));
    }

    fn action(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _source: &WlDataSource,
        action: DndAction,
    ) {
        self.sctk_events
            .push(crate::sctk_event::SctkEvent::DataSource(
                DataSourceEvent::DndActionAccepted(action),
            ));
    }
}

delegate_data_source!(@<T: 'static + Debug> SctkState<T>);
