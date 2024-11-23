use crate::platform_specific::wayland::{
    event_loop::state::SctkState, sctk_event::SctkEvent,
};
use cctk::sctk::{delegate_output, output::OutputHandler};

impl OutputHandler for SctkState {
    fn output_state(&mut self) -> &mut cctk::sctk::output::OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &cctk::sctk::reexports::client::Connection,
        _qh: &cctk::sctk::reexports::client::QueueHandle<Self>,
        output: cctk::sctk::reexports::client::protocol::wl_output::WlOutput,
    ) {
        self.sctk_events.push(SctkEvent::NewOutput {
            id: output.clone(),
            info: self.output_state.info(&output),
        });
        self.outputs.push(output);
    }

    fn update_output(
        &mut self,
        _conn: &cctk::sctk::reexports::client::Connection,
        _qh: &cctk::sctk::reexports::client::QueueHandle<Self>,
        output: cctk::sctk::reexports::client::protocol::wl_output::WlOutput,
    ) {
        if let Some(info) = self.output_state.info(&output) {
            self.sctk_events.push(SctkEvent::UpdateOutput {
                id: output.clone(),
                info,
            });
        }
    }

    fn output_destroyed(
        &mut self,
        _conn: &cctk::sctk::reexports::client::Connection,
        _qh: &cctk::sctk::reexports::client::QueueHandle<Self>,
        output: cctk::sctk::reexports::client::protocol::wl_output::WlOutput,
    ) {
        self.sctk_events.push(SctkEvent::RemovedOutput(output));
        // TODO clean up any layer surfaces on this output?
    }
}

delegate_output!(SctkState);
