use crate::platform_specific::wayland::{
    event_loop::{state::SctkSeat, state::SctkState},
    sctk_event::{KeyboardEventVariant, SctkEvent, SeatEventVariant},
};
use cctk::sctk::{
    delegate_seat,
    reexports::client::{protocol::wl_keyboard::WlKeyboard, Proxy},
    seat::{pointer::ThemeSpec, SeatHandler},
};
use iced_runtime::keyboard::Modifiers;
use std::sync::Arc;

impl SeatHandler for SctkState {
    fn seat_state(&mut self) -> &mut cctk::sctk::seat::SeatState {
        &mut self.seat_state
    }

    fn new_seat(
        &mut self,
        _conn: &cctk::sctk::reexports::client::Connection,
        qh: &cctk::sctk::reexports::client::QueueHandle<Self>,
        seat: cctk::sctk::reexports::client::protocol::wl_seat::WlSeat,
    ) {
        self.sctk_events.push(SctkEvent::SeatEvent {
            variant: SeatEventVariant::New,
            id: seat.clone(),
        });

        self.seats.push(SctkSeat {
            seat,
            kbd: None,
            ptr: None,
            touch: None,
            _modifiers: Modifiers::default(),
            kbd_focus: None,
            ptr_focus: None,
            last_ptr_press: None,
            last_kbd_press: None,
            last_touch_down: None,
            icon: None,
            active_icon: None,
        });
    }

    fn new_capability(
        &mut self,
        _conn: &cctk::sctk::reexports::client::Connection,
        qh: &cctk::sctk::reexports::client::QueueHandle<Self>,
        seat: cctk::sctk::reexports::client::protocol::wl_seat::WlSeat,
        capability: cctk::sctk::seat::Capability,
    ) {
        let my_seat = match self.seats.iter_mut().find(|s| s.seat == seat) {
            Some(s) => s,
            None => {
                self.seats.push(SctkSeat {
                    seat: seat.clone(),
                    kbd: None,
                    ptr: None,
                    touch: None,

                    _modifiers: Modifiers::default(),
                    kbd_focus: None,
                    ptr_focus: None,
                    last_ptr_press: None,
                    last_kbd_press: None,
                    last_touch_down: None,
                    icon: None,
                    active_icon: None,
                });
                self.seats.last_mut().unwrap()
            }
        };
        // TODO data device
        match capability {
            cctk::sctk::seat::Capability::Keyboard => {
                let seat_clone = seat.clone();
                let seat_clone_2 = seat.clone();
                if let Ok(kbd) = self.seat_state.get_keyboard_with_repeat(
                    qh,
                    &seat,
                    None,
                    self.loop_handle.clone(),
                    Box::new(move |state, kbd: &WlKeyboard, e| {
                        let Some(my_seat) = state
                            .seats
                            .iter_mut()
                            .find(|s| s.seat == seat_clone_2)
                        else {
                            return;
                        };
                        if let Some(surface) = my_seat.kbd_focus.clone() {
                            state.sctk_events.push(SctkEvent::KeyboardEvent {
                                variant: KeyboardEventVariant::Repeat(e),
                                kbd_id: kbd.clone(),
                                seat_id: seat_clone.clone(),
                                surface,
                            });
                        }
                    }),
                ) {
                    self.sctk_events.push(SctkEvent::SeatEvent {
                        variant: SeatEventVariant::NewCapability(
                            capability,
                            kbd.id(),
                        ),
                        id: seat.clone(),
                    });
                    _ = my_seat.kbd.replace(kbd);
                }
            }
            cctk::sctk::seat::Capability::Pointer => {
                let surface = self.compositor_state.create_surface(qh);

                if let Ok(ptr) = self.seat_state.get_pointer_with_theme(
                    qh,
                    &seat,
                    self.shm_state.wl_shm(),
                    surface,
                    ThemeSpec::default(),
                ) {
                    self.sctk_events.push(SctkEvent::SeatEvent {
                        variant: SeatEventVariant::NewCapability(
                            capability,
                            ptr.pointer().id(),
                        ),
                        id: seat.clone(),
                    });
                    _ = my_seat.ptr.replace(ptr);
                }
            }
            cctk::sctk::seat::Capability::Touch => {
                if let Some(touch) = self.seat_state.get_touch(qh, &seat).ok() {
                    self.sctk_events.push(SctkEvent::SeatEvent {
                        variant: SeatEventVariant::NewCapability(
                            capability,
                            touch.id(),
                        ),
                        id: seat.clone(),
                    });
                    _ = my_seat.touch.replace(touch);
                }
            }
            _ => unimplemented!(),
        }

        if let Some(text_input_manager) = self
            .text_input
            .is_none()
            .then_some(self.text_input_manager.as_ref())
            .flatten()
        {
            self.text_input =
                Some(Arc::new((text_input_manager).get_text_input(&seat, &qh)));
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &cctk::sctk::reexports::client::Connection,
        _qh: &cctk::sctk::reexports::client::QueueHandle<Self>,
        seat: cctk::sctk::reexports::client::protocol::wl_seat::WlSeat,
        capability: cctk::sctk::seat::Capability,
    ) {
        let my_seat = match self.seats.iter_mut().find(|s| s.seat == seat) {
            Some(s) => s,
            None => return,
        };

        // TODO data device
        match capability {
            // TODO use repeating kbd?
            cctk::sctk::seat::Capability::Keyboard => {
                if let Some(kbd) = my_seat.kbd.take() {
                    self.sctk_events.push(SctkEvent::SeatEvent {
                        variant: SeatEventVariant::RemoveCapability(
                            capability,
                            kbd.id(),
                        ),
                        id: seat.clone(),
                    });
                }
            }
            cctk::sctk::seat::Capability::Pointer => {
                if let Some(ptr) = my_seat.ptr.take() {
                    self.sctk_events.push(SctkEvent::SeatEvent {
                        variant: SeatEventVariant::RemoveCapability(
                            capability,
                            ptr.pointer().id(),
                        ),
                        id: seat.clone(),
                    });
                }
            }
            cctk::sctk::seat::Capability::Touch => {
                if let Some(touch) = my_seat.touch.take() {
                    self.sctk_events.push(SctkEvent::SeatEvent {
                        variant: SeatEventVariant::RemoveCapability(
                            capability,
                            touch.id(),
                        ),
                        id: seat.clone(),
                    });
                }
            }
            _ => unimplemented!(),
        }

        if let Some(text_input) = self.text_input.take() {
            text_input.destroy();
        }
    }

    fn remove_seat(
        &mut self,
        _conn: &cctk::sctk::reexports::client::Connection,
        _qh: &cctk::sctk::reexports::client::QueueHandle<Self>,
        seat: cctk::sctk::reexports::client::protocol::wl_seat::WlSeat,
    ) {
        self.sctk_events.push(SctkEvent::SeatEvent {
            variant: SeatEventVariant::Remove,
            id: seat.clone(),
        });
        if let Some(i) = self.seats.iter().position(|s| s.seat == seat) {
            _ = self.seats.remove(i);
        }
    }
}

delegate_seat!(SctkState);
