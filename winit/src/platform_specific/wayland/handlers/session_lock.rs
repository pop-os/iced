use crate::{
    event_loop::state::CommonSurface,
    platform_specific::wayland::{handlers::SctkState, sctk_event::SctkEvent},
};
use cctk::sctk::{
    delegate_session_lock,
    reexports::client::{Connection, QueueHandle},
    session_lock::{
        SessionLock, SessionLockHandler, SessionLockSurface,
        SessionLockSurfaceConfigure,
    },
};

impl SessionLockHandler for SctkState {
    fn locked(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        session_lock: SessionLock,
    ) {
        self.session_lock = Some(session_lock);
        self.sctk_events.push(SctkEvent::SessionLocked);
    }

    fn finished(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _session_lock: SessionLock,
    ) {
        self.sctk_events.push(SctkEvent::SessionLockFinished);
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        session_lock_surface: SessionLockSurface,
        configure: SessionLockSurfaceConfigure,
        _serial: u32,
    ) {
        let lock_surface = match self.lock_surfaces.iter_mut().find(|s| {
            s.session_lock_surface.wl_surface()
                == session_lock_surface.wl_surface()
        }) {
            Some(l) => l,
            None => return,
        };
        lock_surface
            .update_viewport(configure.new_size.0, configure.new_size.1);

        let first = lock_surface.last_configure.is_none();
        _ = lock_surface.last_configure.replace(configure.clone());

        self.sctk_events.push(SctkEvent::SessionLockSurfaceCreated {
            queue_handle: self.queue_handle.clone(),
            surface: CommonSurface::Lock(
                lock_surface.session_lock_surface.clone(),
            ),
            native_id: lock_surface.id,
            common: lock_surface.common.clone(),
            display: self.connection.display(),
        });
        self.sctk_events
            .push(SctkEvent::SessionLockSurfaceConfigure {
                surface: session_lock_surface.wl_surface().clone(),
                configure,
                first,
            });
    }
}

delegate_session_lock!(SctkState);
