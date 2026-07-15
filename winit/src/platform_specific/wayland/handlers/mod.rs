// handlers
pub mod activation;
pub mod compositor;
pub mod ext_background_effect;
pub mod output;
pub mod overlap;
pub mod seat;
pub mod session_lock;
pub mod shell;
pub mod subcompositor;
pub mod text_input;
pub mod toplevel;
pub mod wp_fractional_scaling;
pub mod wp_viewporter;

use cctk::sctk::{
    output::OutputState,
    registry_handlers,
    seat::SeatState,
    shm::{Shm, ShmHandler},
};

use wayland_client::globals::GlobalListHandler;

use crate::platform_specific::wayland::event_loop::state::SctkState;

impl ShmHandler for SctkState {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm_state
    }
}

impl GlobalListHandler for SctkState {
    registry_handlers![OutputState, SeatState,];
}
