use crate::platform_specific::wayland::{
    event_loop::state::SctkState, sctk_event::SctkEvent,
};
use sctk::{
    delegate_pointer,
    reexports::{
        client::Proxy, protocols::xdg::shell::client::xdg_toplevel::ResizeEdge,
    },
    seat::pointer::{
        CursorIcon, PointerEvent, PointerEventKind, PointerHandler, BTN_LEFT,
    },
    shell::WaylandSurface,
};
use winit::{
    dpi::PhysicalPosition,
    event::{MouseButton, MouseScrollDelta, TouchPhase, WindowEvent},
};

impl PointerHandler for SctkState {
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
                    w.resizable.and_then(|border| {
                        let (width, height) = w.current_size;
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
                    })
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
                        _ = my_seat
                            .last_ptr_press
                            .replace((time, button, serial));
                        window.window.resize(
                            &my_seat.seat,
                            serial,
                            resize_edge,
                        );
                        return;
                    }
                    PointerEventKind::Motion { .. } => {
                        if my_seat.active_icon != Some(icon) {
                            let _ = my_seat.set_cursor(conn, icon);
                        }
                        return;
                    }
                    PointerEventKind::Enter { .. } => {}
                    PointerEventKind::Leave { .. } => {}
                    _ => {}
                }
                if my_seat.active_icon != Some(icon) {
                    my_seat.set_cursor(conn, icon);
                }
            } else if my_seat.active_icon != my_seat.icon {
                // Restore cursor that was set by appliction, or default
                my_seat.set_cursor(
                    conn,
                    my_seat.icon.unwrap_or(CursorIcon::Default),
                );
            }

            if is_active {
                let id = winit::window::WindowId::from(
                    e.surface.id().as_ptr() as u64
                );
                if let PointerEventKind::Motion { time } = &e.kind {
                    self.sctk_events.push(SctkEvent::PointerEvent {
                        variant: PointerEvent {
                            surface: e.surface.clone(),
                            position: e.position,
                            kind: PointerEventKind::Motion { time: *time },
                        },
                        ptr_id: pointer.clone(),
                        seat_id: my_seat.seat.clone(),
                    });
                } else {
                    self.sctk_events.push(SctkEvent::Winit(
                        id,
                        match e.kind {
                            PointerEventKind::Enter { serial } => {
                                WindowEvent::CursorEntered {
                                    device_id: Default::default(),
                                }
                            }
                            PointerEventKind::Leave { serial } => {
                                WindowEvent::CursorLeft {
                                    device_id: Default::default(),
                                }
                            }
                            PointerEventKind::Motion { time } => {
                                WindowEvent::CursorMoved {
                                    device_id: Default::default(),
                                    position: e.position.into(),
                                }
                            }
                            PointerEventKind::Press {
                                time,
                                button,
                                serial,
                            } => WindowEvent::MouseInput {
                                device_id: Default::default(),
                                state: winit::event::ElementState::Pressed,
                                button: wayland_button_to_winit(button),
                            },
                            PointerEventKind::Release {
                                time,
                                button,
                                serial,
                            } => WindowEvent::MouseInput {
                                device_id: Default::default(),
                                state: winit::event::ElementState::Released,
                                button: wayland_button_to_winit(button),
                            },
                            PointerEventKind::Axis {
                                time,
                                horizontal,
                                vertical,
                                source,
                            } => WindowEvent::MouseWheel {
                                device_id: Default::default(),
                                delta: if horizontal.discrete > 0 {
                                    MouseScrollDelta::LineDelta(
                                        horizontal.discrete as f32,
                                        vertical.discrete as f32,
                                    )
                                } else {
                                    MouseScrollDelta::PixelDelta(
                                        PhysicalPosition::new(
                                            horizontal.absolute,
                                            vertical.absolute,
                                        ),
                                    )
                                },
                                phase: if horizontal.stop {
                                    TouchPhase::Ended
                                } else {
                                    TouchPhase::Moved
                                },
                            },
                        },
                    ));
                }
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

/// Convert the Wayland button into winit.
fn wayland_button_to_winit(button: u32) -> MouseButton {
    // These values are coming from <linux/input-event-codes.h>.
    const BTN_LEFT: u32 = 0x110;
    const BTN_RIGHT: u32 = 0x111;
    const BTN_MIDDLE: u32 = 0x112;
    const BTN_SIDE: u32 = 0x113;
    const BTN_EXTRA: u32 = 0x114;
    const BTN_FORWARD: u32 = 0x115;
    const BTN_BACK: u32 = 0x116;

    match button {
        BTN_LEFT => MouseButton::Left,
        BTN_RIGHT => MouseButton::Right,
        BTN_MIDDLE => MouseButton::Middle,
        BTN_BACK | BTN_SIDE => MouseButton::Back,
        BTN_FORWARD | BTN_EXTRA => MouseButton::Forward,
        button => MouseButton::Other(button as u16),
    }
}

delegate_pointer!(SctkState);
