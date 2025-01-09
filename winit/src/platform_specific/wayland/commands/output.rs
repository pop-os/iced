pub use cctk::sctk::output::{OutputInfo, OutputState};
use iced_runtime::{
    platform_specific::{self, wayland},
    task, Action, Task,
};
pub use wayland_client::protocol::wl_output::WlOutput;

/// Get a
/// [WlOutput](https://docs.rs/wayland-client/latest/wayland_client/protocol/wl_output/struct.WlOutput.html) by calling a closure on a
/// [&OutputState](https://docs.rs/smithay-client-toolkit/latest/smithay_client_toolkit/output/struct.OutputState.html)
pub fn get_output<F>(f: F) -> Task<Option<WlOutput>>
where
    F: Fn(&OutputState) -> Option<WlOutput> + Send + Sync + 'static,
{
    task::oneshot(|channel| {
        Action::PlatformSpecific(platform_specific::Action::Wayland(
            wayland::Action::Output(wayland::output::Action::GetOutput {
                f: Box::new(f),
                channel,
            }),
        ))
    })
}

/// Get a
/// [OutputInfo](https://docs.rs/smithay-client-toolkit/latest/smithay_client_toolkit/output/struct.OutputInfo.html) by calling a closure on a
/// [&OutputState](https://docs.rs/smithay-client-toolkit/latest/smithay_client_toolkit/output/struct.OutputState.html)
pub fn get_output_info<F>(f: F) -> Task<Option<OutputInfo>>
where
    F: Fn(&OutputState) -> Option<OutputInfo> + Send + Sync + 'static,
{
    task::oneshot(|channel| {
        Action::PlatformSpecific(platform_specific::Action::Wayland(
            wayland::Action::Output(wayland::output::Action::GetOutputInfo {
                f: Box::new(f),
                channel,
            }),
        ))
    })
}
