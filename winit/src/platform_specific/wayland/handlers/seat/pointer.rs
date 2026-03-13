use crate::{
    event_loop::state::FrameStatus,
    platform_specific::wayland::{
        event_loop::state::SctkState, sctk_event::SctkEvent,
    },
};
use cctk::sctk::{
    delegate_pointer,
    reexports::client::Proxy,
    seat::pointer::{
        CursorIcon, PointerEvent, PointerEventKind, PointerHandler,
    },
};

impl PointerHandler for SctkState {
    fn pointer_frame(
        &mut self,
        conn: &cctk::sctk::reexports::client::Connection,
        _qh: &cctk::sctk::reexports::client::QueueHandle<Self>,
        pointer: &cctk::sctk::reexports::client::protocol::wl_pointer::WlPointer,
        events: &[cctk::sctk::seat::pointer::PointerEvent],
    ) {
        let (is_active, my_seat) =
            match self.seats.iter_mut().enumerate().find_map(|(i, s)| {
                if s.ptr.as_ref().map(|p| p.pointer()) == Some(pointer) {
                    Some((i, s))
                } else {
                    None
                }
            }) {
                Some((i, s)) => (i == 0, s),
                None => return,
            };

        // track events, but only forward for the active seat
        for e in events {
            if my_seat.active_icon != my_seat.icon {
                // Restore cursor that was set by appliction, or default
                my_seat.set_cursor(
                    conn,
                    my_seat.icon.unwrap_or(CursorIcon::Default),
                );
            }

            if is_active {
                let id = winit::window::WindowId::from_raw(
                    e.surface.id().as_ptr() as usize,
                );
                if self.windows.iter().any(|w| w.window.id() == id) {
                    continue;
                }

                let entry = self
                    .frame_status
                    .entry(e.surface.id())
                    .or_insert(FrameStatus::RequestedRedraw);
                if matches!(entry, FrameStatus::Received) {
                    *entry = FrameStatus::Ready;
                }

                self.sctk_events.push(SctkEvent::PointerEvent {
                    variant: PointerEvent {
                        surface: e.surface.clone(),
                        position: e.position,
                        kind: e.kind.clone(),
                    },
                    ptr_id: pointer.clone(),
                    seat_id: my_seat.seat.clone(),
                });
            }
            match e.kind {
                PointerEventKind::Enter { .. } => {
                    _ = my_seat.ptr_focus.replace(e.surface.clone());
                }
                PointerEventKind::Leave { .. } => {
                    _ = my_seat.ptr_focus.take();
                    _ = my_seat.active_icon = None;
                }
                PointerEventKind::Press {
                    time,
                    button,
                    serial,
                } => {
                    _ = my_seat.last_ptr_press.replace((time, button, serial));
                }
                // TODO revisit events that ought to be handled and change internal state
                _ => {}
            }
        }
    }
}

delegate_pointer!(SctkState);
