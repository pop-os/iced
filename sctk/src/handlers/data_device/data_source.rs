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
use std::fmt::Debug;

impl<T> DataSourceHandler for SctkState<T> {
    fn accept_mime(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        source: &WlDataSource,
        mime: Option<String>,
    ) {
        let is_active_source = self
            .dnd_source
            .as_ref()
            .and_then(|s| (s.source.as_ref().map(|s| s.inner() == source)))
            .unwrap_or(false);
        if is_active_source {
            self.sctk_events.push(SctkEvent::DataSource(
                DataSourceEvent::MimeAccepted(mime),
            ));
        }
    }

    fn send_request(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        source: &WlDataSource,
        mime: String,
        pipe: WritePipe,
    ) {
        let is_active_source = self
            .selection_source
            .as_ref()
            .map(|s| s.source.inner() == source)
            .unwrap_or(false)
            || self
                .dnd_source
                .as_ref()
                .and_then(|s| (s.source.as_ref().map(|s| s.inner() == source)))
                .unwrap_or(false);

        if !is_active_source {
            source.destroy();
            return;
        }

        if let Some(my_source) = self
            .selection_source
            .as_mut()
            .filter(|s| s.source.inner() == source)
        {
            my_source.pipe = Some(pipe);
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
        source: &WlDataSource,
    ) {
        let is_active_source = self
            .selection_source
            .as_ref()
            .map(|s| s.source.inner() == source)
            .unwrap_or(false)
            || self
                .dnd_source
                .as_ref()
                .and_then(|s| (s.source.as_ref().map(|s| s.inner() == source)))
                .unwrap_or(false);
        if is_active_source {
            self.sctk_events
                .push(SctkEvent::DataSource(DataSourceEvent::DndCancelled));
        }
    }

    fn dnd_dropped(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        source: &WlDataSource,
    ) {
        let is_active_source = self
            .dnd_source
            .as_ref()
            .and_then(|s| (s.source.as_ref().map(|s| s.inner() == source)))
            .unwrap_or(false);
        if is_active_source {
            self.sctk_events
                .push(SctkEvent::DataSource(DataSourceEvent::DndDropPerformed));
        }
    }

    fn dnd_finished(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        source: &WlDataSource,
    ) {
        let is_active_source = self
            .dnd_source
            .as_ref()
            .and_then(|s| (s.source.as_ref().map(|s| s.inner() == source)))
            .unwrap_or(false);
        if is_active_source {
            self.sctk_events
                .push(SctkEvent::DataSource(DataSourceEvent::DndFinished));
        }
    }

    fn action(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        source: &WlDataSource,
        action: DndAction,
    ) {
        let is_active_source = self
            .dnd_source
            .as_ref()
            .and_then(|s| (s.source.as_ref().map(|s| s.inner() == source)))
            .unwrap_or(false);
        if is_active_source {
            self.sctk_events
                .push(crate::sctk_event::SctkEvent::DataSource(
                    DataSourceEvent::DndActionAccepted(action),
                ));
        }
    }
}

delegate_data_source!(@<T: 'static + Debug> SctkState<T>);
