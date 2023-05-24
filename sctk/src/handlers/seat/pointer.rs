use crate::{event_loop::state::SctkState, sctk_event::SctkEvent};
use sctk::{
    delegate_pointer,
    reexports::protocols::xdg::shell::client::xdg_toplevel::ResizeEdge,
    seat::pointer::{CursorIcon, PointerEventKind, PointerHandler, BTN_LEFT},
    shell::WaylandSurface,
};
use std::fmt::Debug;

impl<T: Debug> PointerHandler for SctkState<T> {
    fn pointer_frame(
        &mut self,
        conn: &sctk::reexports::client::Connection,
        _qh: &sctk::reexports::client::QueueHandle<Self>,
        pointer: &sctk::reexports::client::protocol::wl_pointer::WlPointer,
        events: &[sctk::seat::pointer::PointerEvent],
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
            // check if it is over a resizable window's border and handle the event yourself if it is.
            if let Some((resize_edge, window)) = self
                .windows
                .iter()
                .find(|w| w.window.wl_surface() == &e.surface)
                .and_then(|w| {
                    w.resizable.zip(w.current_size).and_then(
                        |(border, (width, height))| {
                            let (width, height) =
                                (width.get() as f64, height.get() as f64);
                            let (x, y) = e.position;
                            let left_edge = x < border;
                            let top_edge = y < border;
                            let right_edge = x > width - border;
                            let bottom_edge = y > height - border;

                            if left_edge && top_edge {
                                Some((ResizeEdge::TopLeft, w))
                            } else if left_edge && bottom_edge {
                                Some((ResizeEdge::BottomLeft, w))
                            } else if right_edge && top_edge {
                                Some((ResizeEdge::TopRight, w))
                            } else if right_edge && bottom_edge {
                                Some((ResizeEdge::BottomRight, w))
                            } else if left_edge {
                                Some((ResizeEdge::Left, w))
                            } else if right_edge {
                                Some((ResizeEdge::Right, w))
                            } else if top_edge {
                                Some((ResizeEdge::Top, w))
                            } else if bottom_edge {
                                Some((ResizeEdge::Bottom, w))
                            } else {
                                None
                            }
                        },
                    )
                })
            {
                let icon = match resize_edge {
                    ResizeEdge::Top => CursorIcon::NResize,
                    ResizeEdge::Bottom => CursorIcon::SResize,
                    ResizeEdge::Left => CursorIcon::WResize,
                    ResizeEdge::TopLeft => CursorIcon::NwResize,
                    ResizeEdge::BottomLeft => CursorIcon::SwResize,
                    ResizeEdge::Right => CursorIcon::EResize,
                    ResizeEdge::TopRight => CursorIcon::NeResize,
                    ResizeEdge::BottomRight => CursorIcon::SeResize,
                    _ => unimplemented!(),
                };
                match e.kind {
                    PointerEventKind::Press {
                        time,
                        button,
                        serial,
                    } if button == BTN_LEFT => {
                        my_seat.last_ptr_press.replace((time, button, serial));
                        window.window.resize(
                            &my_seat.seat,
                            serial,
                            resize_edge,
                        );
                        return;
                    }
                    PointerEventKind::Motion { .. } => {
                        if my_seat.icon != Some(icon) {
                            let _ = my_seat
                                .ptr
                                .as_ref()
                                .unwrap()
                                .set_cursor(conn, icon);
                            my_seat.icon = Some(icon);
                        }
                        return;
                    }
                    PointerEventKind::Enter { .. } => {
                        my_seat.ptr_focus.replace(e.surface.clone());
                        if my_seat.icon != Some(icon) {
                            let _ = my_seat
                                .ptr
                                .as_ref()
                                .unwrap()
                                .set_cursor(conn, icon);
                            my_seat.icon = Some(icon);
                        }
                    }
                    PointerEventKind::Leave { .. } => {
                        my_seat.ptr_focus.take();
                        my_seat.icon = None;
                    }
                    _ => {}
                }
                let _ = my_seat.ptr.as_ref().unwrap().set_cursor(conn, icon);
            } else if my_seat.icon.is_some() {
                let _ = my_seat
                    .ptr
                    .as_ref()
                    .unwrap()
                    .set_cursor(conn, CursorIcon::Default);
                my_seat.icon = None;
            }

            if is_active {
                self.sctk_events.push(SctkEvent::PointerEvent {
                    variant: e.clone(),
                    ptr_id: pointer.clone(),
                    seat_id: my_seat.seat.clone(),
                });
            }
            match e.kind {
                PointerEventKind::Enter { .. } => {
                    my_seat.ptr_focus.replace(e.surface.clone());
                }
                PointerEventKind::Leave { .. } => {
                    my_seat.ptr_focus.take();
                    my_seat.icon = None;
                }
                PointerEventKind::Press {
                    time,
                    button,
                    serial,
                } => {
                    my_seat.last_ptr_press.replace((time, button, serial));
                }
                // TODO revisit events that ought to be handled and change internal state
                _ => {}
            }
        }
    }
}

delegate_pointer!(@<T: 'static + Debug> SctkState<T>);
