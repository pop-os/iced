use cctk::sctk::output::OutputState;
use iced_runtime::{platform_specific::{self, wayland}, task, Action, Task};
use wayland_client::protocol::wl_output::WlOutput;


pub fn get_output<F>(f: F) -> Task<Option<WlOutput>> where F: Fn(&OutputState) -> Option<WlOutput> + Send + Sync + 'static {
    task::oneshot(|channel| Action::PlatformSpecific(
        platform_specific::Action::Wayland(
            wayland::Action::GetOutput {
                f: Box::new(f),
                channel,
            }
        )
    ))
}
