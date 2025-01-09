use std::fmt;

use cctk::{
    sctk::output::{OutputInfo, OutputState},
    wayland_client::protocol::wl_output::WlOutput,
};

use crate::oneshot;

pub enum Action {
    // WlOutput getter
    GetOutput {
        f: Box<dyn Fn(&OutputState) -> Option<WlOutput> + Send + Sync>,
        channel: oneshot::Sender<Option<WlOutput>>,
    },
    // OutputInfo getter
    GetOutputInfo {
        f: Box<dyn Fn(&OutputState) -> Option<OutputInfo> + Send + Sync>,
        channel: oneshot::Sender<Option<OutputInfo>>,
    },
}

impl fmt::Debug for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Action::GetOutput { .. } => write!(f, "Action::GetOutput"),
            Action::GetOutputInfo { .. } => write!(f, "Action::GetOutputInfo",),
        }
    }
}
