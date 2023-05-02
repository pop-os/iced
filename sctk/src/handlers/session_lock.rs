use crate::{handlers::SctkState, sctk_event::SctkEvent};
use sctk::{
    delegate_session_lock,
    reexports::client::{Connection, QueueHandle},
    session_lock::{
        SessionLock, SessionLockHandler, SessionLockSurface,
        SessionLockSurfaceConfigure,
    },
};
use std::fmt::Debug;

impl<T: 'static + Debug> SessionLockHandler for SctkState<T> {
    fn locked(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _session_lock: SessionLock,
    ) {
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
        let first = lock_surface.last_configure.is_none();
        lock_surface.last_configure.replace(configure.clone());
        self.sctk_events
            .push(SctkEvent::SessionLockSurfaceConfigure {
                surface: session_lock_surface.wl_surface().clone(),
                configure,
                first,
            });
    }
}

delegate_session_lock!(@<T: 'static + Debug> SctkState<T>);
