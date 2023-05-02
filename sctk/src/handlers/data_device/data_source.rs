use crate::event_loop::state::SctkState;
use crate::sctk_event::{DataSourceEvent, SctkEvent};
use sctk::data_device_manager::WritePipe;
use sctk::{
    data_device_manager::data_source::DataSourceHandler,
    reexports::{
        calloop::PostAction,
        client::{
            protocol::{
                wl_data_device_manager::DndAction, wl_data_source::WlDataSource,
            },
            Connection, QueueHandle,
        },
    },
};
use std::io::{BufWriter, Write};
use tracing::error;

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
            .and_then(|s| (s.source.as_ref().map(|s| s.0.inner() == source)))
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
            .dnd_source
            .as_ref()
            .and_then(|s| s.source.as_ref().map(|s| s.0.inner() == source))
            .unwrap_or(false);

        if !is_active_source {
            source.destroy();
            return;
        }

        if let Some(source) = self.dnd_source.as_mut().filter(|s| {
            s.source
                .as_ref()
                .map(|s| (s.0.inner() == source))
                .unwrap_or(false)
        }) {
            let (_my_source, data) = match source.source.as_ref() {
                Some((source, data)) => (source, data),
                None => return,
            };
            match self.loop_handle.insert_source(
                pipe,
                move |_, f, state| -> PostAction {
                    let loop_handle = &state.loop_handle;
                    let dnd_source = match state.dnd_source.as_mut() {
                        Some(s) => s,
                        None => return PostAction::Continue,
                    };
                    let (data, mut cur_index, token) =
                        match dnd_source.cur_write.take() {
                            Some(s) => s,
                            None => return PostAction::Continue,
                        };
                    let mut writer = BufWriter::new(f.as_ref());
                    let slice = &data.as_slice()[cur_index
                        ..(cur_index + writer.capacity()).min(data.len())];
                    match writer.write(slice) {
                        Ok(num_written) => {
                            cur_index += num_written;
                            if cur_index == data.len() {
                                loop_handle.remove(token);
                            } else {
                                dnd_source.cur_write =
                                    Some((data, cur_index, token));
                            }
                            if let Err(err) = writer.flush() {
                                loop_handle.remove(token);
                                error!("Failed to flush pipe: {}", err);
                            }
                        }
                        Err(e)
                            if matches!(
                                e.kind(),
                                std::io::ErrorKind::Interrupted
                            ) =>
                        {
                            // try again
                            dnd_source.cur_write =
                                Some((data, cur_index, token));
                        }
                        Err(_) => {
                            loop_handle.remove(token);
                            error!("Failed to write to pipe");
                        }
                    };
                    PostAction::Continue
                },
            ) {
                Ok(s) => {
                    source.cur_write = Some((
                        data.from_mime_type(&mime).unwrap_or_default(),
                        0,
                        s,
                    ));
                }
                Err(_) => {
                    error!("Failed to insert source");
                }
            };
        }
    }

    fn cancelled(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        source: &WlDataSource,
    ) {
        let is_active_source = self
            .dnd_source
            .as_ref()
            .and_then(|s| s.source.as_ref().map(|s| s.0.inner() == source))
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
            .and_then(|s| (s.source.as_ref().map(|s| s.0.inner() == source)))
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
            .and_then(|s| (s.source.as_ref().map(|s| s.0.inner() == source)))
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
            .and_then(|s| (s.source.as_ref().map(|s| s.0.inner() == source)))
            .unwrap_or(false);
        if is_active_source {
            self.sctk_events
                .push(crate::sctk_event::SctkEvent::DataSource(
                    DataSourceEvent::DndActionAccepted(action),
                ));
        }
    }
}
