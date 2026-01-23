// handlers
pub mod activation;
pub mod compositor;
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

use cctk::{sctk::{
    delegate_registry, delegate_shm,
    output::OutputState,
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::SeatState,
    shm::{Shm, ShmHandler},
}};

use crate::platform_specific::wayland::event_loop::state::SctkState;

impl ShmHandler for SctkState {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm_state
    }
}

impl ProvidesRegistryState for SctkState {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState,];
}

delegate_shm!(SctkState);
delegate_registry!(SctkState);
