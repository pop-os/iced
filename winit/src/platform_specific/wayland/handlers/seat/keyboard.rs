use crate::platform_specific::wayland::{
    event_loop::state::SctkState,
    sctk_event::{KeyboardEventVariant, SctkEvent},
};
use sctk::reexports::client::Proxy;
use sctk::{
    delegate_keyboard,
    seat::keyboard::{KeyboardHandler, Keysym},
};

impl KeyboardHandler for SctkState {
    fn enter(
        &mut self,
        _conn: &sctk::reexports::client::Connection,
        _qh: &sctk::reexports::client::QueueHandle<Self>,
        keyboard: &sctk::reexports::client::protocol::wl_keyboard::WlKeyboard,
        surface: &sctk::reexports::client::protocol::wl_surface::WlSurface,
        _serial: u32,
        _raw: &[u32],
        _keysyms: &[Keysym],
    ) {
        self.request_redraw(surface);
        let (i, mut is_active, seat) = {
            let (i, is_active, my_seat) =
                match self.seats.iter_mut().enumerate().find_map(|(i, s)| {
                    if s.kbd.as_ref() == Some(keyboard) {
                        Some((i, s))
                    } else {
                        None
                    }
                }) {
                    Some((i, s)) => (i, i == 0, s),
                    None => return,
                };
            _ = my_seat.kbd_focus.replace(surface.clone());

            let seat = my_seat.seat.clone();
            (i, is_active, seat)
        };

        // TODO Ashley: thoroughly test this
        // swap the active seat to be the current seat if the current "active" seat is not focused on the application anyway
        if !is_active && self.seats[0].kbd_focus.is_none() {
            is_active = true;
            self.seats.swap(0, i);
        }

        if is_active {
            let id = winit::window::WindowId::from_raw(
                surface.id().as_ptr() as usize
            );
            if self.windows.iter().any(|w| w.window.id() == id) {
                return;
            }
            self.sctk_events.push(SctkEvent::Winit(
                id,
                winit::event::WindowEvent::Focused(true),
            ));
            self.sctk_events.push(SctkEvent::KeyboardEvent {
                 variant: KeyboardEventVariant::Enter(surface.clone()),
                 kbd_id: keyboard.clone(),
                 seat_id: seat,
                 surface: surface.clone(),
            });
        }
    }

    fn leave(
        &mut self,
        _conn: &sctk::reexports::client::Connection,
        _qh: &sctk::reexports::client::QueueHandle<Self>,
        keyboard: &sctk::reexports::client::protocol::wl_keyboard::WlKeyboard,
        surface: &sctk::reexports::client::protocol::wl_surface::WlSurface,
        _serial: u32,
    ) {
        self.request_redraw(surface);
        let (is_active, seat, kbd) = {
            let (is_active, my_seat) =
                match self.seats.iter_mut().enumerate().find_map(|(i, s)| {
                    if s.kbd.as_ref() == Some(keyboard) {
                        Some((i, s))
                    } else {
                        None
                    }
                }) {
                    Some((i, s)) => (i == 0, s),
                    None => return,
                };
            let seat = my_seat.seat.clone();
            let kbd = keyboard.clone();
            _ = my_seat.kbd_focus.take();
            (is_active, seat, kbd)
        };

        if is_active {
            self.sctk_events.push(SctkEvent::KeyboardEvent {
                variant: KeyboardEventVariant::Leave(surface.clone()),
                kbd_id: kbd,
                seat_id: seat,
                surface: surface.clone(),
            });
            // if there is another seat with a keyboard focused on a surface make that the new active seat
            if let Some(i) =
                self.seats.iter().position(|s| s.kbd_focus.is_some())
            {
                self.seats.swap(0, i);
                let s = &self.seats[0];
                let id = winit::window::WindowId::from_raw(
                    surface.id().as_ptr() as usize,
                );
                if self.windows.iter().any(|w| w.window.id() == id) {
                    return;
                }
                self.sctk_events.push(SctkEvent::Winit(
                    id,
                    winit::event::WindowEvent::Focused(true),
                ));
                self.sctk_events.push(SctkEvent::KeyboardEvent {
                    variant: KeyboardEventVariant::Enter(
                        s.kbd_focus.clone().unwrap(),
                    ),
                    kbd_id: s.kbd.clone().unwrap(),
                    seat_id: s.seat.clone(),
                    surface: surface.clone(),
                })
            }
        }
    }

    fn press_key(
        &mut self,
        _conn: &sctk::reexports::client::Connection,
        _qh: &sctk::reexports::client::QueueHandle<Self>,
        keyboard: &sctk::reexports::client::protocol::wl_keyboard::WlKeyboard,
        serial: u32,
        event: sctk::seat::keyboard::KeyEvent,
    ) {
        let (is_active, my_seat) =
            match self.seats.iter_mut().enumerate().find_map(|(i, s)| {
                if s.kbd.as_ref() == Some(keyboard) {
                    Some((i, s))
                } else {
                    None
                }
            }) {
                Some((i, s)) => (i == 0, s),
                None => return,
            };
        let seat_id = my_seat.seat.clone();
        let kbd_id = keyboard.clone();
        _ = my_seat.last_kbd_press.replace((event.clone(), serial));
        if is_active {
            // FIXME can't create winit key events because of private field
            // if let Some(id) = id {
            //     let physical_key = raw_keycode_to_physicalkey(event.raw_code);
            //     let (logical_key, location) =
            //         keysym_to_vkey_location(event.keysym);
            //     self.sctk_events.push(SctkEvent::Winit(
            //         id,
            //         winit::event::WindowEvent::KeyboardInput {
            //             device_id: Default::default(),
            //             event: winit::event::KeyEvent {
            //                 physical_key,
            //                 logical_key,
            //                 text: event.utf8.map(|s| s.into()),
            //                 location,
            //                 state: winit::event::ElementState::Pressed,
            //                 repeat: false, // TODO we don't have this info...
            //             },
            //             is_synthetic: false,
            //         },
            //     ))
            // }
            if let Some(surface) = my_seat.kbd_focus.clone() {
                self.request_redraw(&surface);
                self.sctk_events.push(SctkEvent::KeyboardEvent {
                    variant: KeyboardEventVariant::Press(event),
                    kbd_id,
                    seat_id,
                    surface,
                });
            }
        }
    }

    fn release_key(
        &mut self,
        _conn: &sctk::reexports::client::Connection,
        _qh: &sctk::reexports::client::QueueHandle<Self>,
        keyboard: &sctk::reexports::client::protocol::wl_keyboard::WlKeyboard,
        _serial: u32,
        event: sctk::seat::keyboard::KeyEvent,
    ) {
        let (is_active, my_seat) =
            match self.seats.iter_mut().enumerate().find_map(|(i, s)| {
                if s.kbd.as_ref() == Some(keyboard) {
                    Some((i, s))
                } else {
                    None
                }
            }) {
                Some((i, s)) => (i == 0, s),
                None => return,
            };
        let seat_id = my_seat.seat.clone();
        let kbd_id = keyboard.clone();

        if is_active {
            if let Some(surface) = my_seat.kbd_focus.clone() {
                self.request_redraw(&surface);
                self.sctk_events.push(SctkEvent::KeyboardEvent {
                    variant: KeyboardEventVariant::Release(event),
                    kbd_id,
                    seat_id,
                    surface,
                });
            }
        }
    }

    fn update_modifiers(
        &mut self,
        _conn: &sctk::reexports::client::Connection,
        _qh: &sctk::reexports::client::QueueHandle<Self>,
        keyboard: &sctk::reexports::client::protocol::wl_keyboard::WlKeyboard,
        _serial: u32,
        modifiers: sctk::seat::keyboard::Modifiers,
        layout: u32,
    ) {
        let (is_active, my_seat) =
            match self.seats.iter_mut().enumerate().find_map(|(i, s)| {
                if s.kbd.as_ref() == Some(keyboard) {
                    Some((i, s))
                } else {
                    None
                }
            }) {
                Some((i, s)) => (i == 0, s),
                None => return,
            };
        let seat_id = my_seat.seat.clone();
        let kbd_id = keyboard.clone();

        if is_active {
            if let Some(surface) = my_seat.kbd_focus.clone() {
                self.request_redraw(&surface);
                self.sctk_events.push(SctkEvent::KeyboardEvent {
                    variant: KeyboardEventVariant::Modifiers(modifiers),
                    kbd_id,
                    seat_id,
                    surface,
                });
            }
        }
    }
}

delegate_keyboard!(SctkState);
