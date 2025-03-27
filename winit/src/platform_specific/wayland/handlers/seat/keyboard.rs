use crate::platform_specific::wayland::{
    event_loop::state::SctkState,
    sctk_event::{KeyboardEventVariant, SctkEvent},
};
use cctk::sctk::reexports::client::Proxy;
use cctk::sctk::{
    delegate_keyboard,
    seat::keyboard::{KeyboardHandler, Keysym},
};

impl KeyboardHandler for SctkState {
    fn enter(
        &mut self,
        _conn: &cctk::sctk::reexports::client::Connection,
        _qh: &cctk::sctk::reexports::client::QueueHandle<Self>,
        keyboard: &cctk::sctk::reexports::client::protocol::wl_keyboard::WlKeyboard,
        surface: &cctk::sctk::reexports::client::protocol::wl_surface::WlSurface,
        _serial: u32,
        _raw: &[u32],
        _keysyms: &[Keysym],
    ) {
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

            let surface = if let Some(subsurface) =
                self.subsurfaces.iter().find(|s| {
                    s.steals_keyboard_focus && s.instance.parent == *surface
                }) {
                &subsurface.instance.wl_surface
            } else {
                surface
            };
            _ = my_seat.kbd_focus.replace(surface.clone());

            let seat = my_seat.seat.clone();
            (i, is_active, seat)
        };

        if !is_active && self.seats[0].kbd_focus.is_none() {
            is_active = true;
            self.seats.swap(0, i);
        }
        self.request_redraw(&surface);

        let surfaces = self.subsurfaces.iter().filter_map(|s| {
            (s.instance.parent == *surface).then(|| &s.instance.wl_surface)
        });
        for surface in surfaces.chain(std::iter::once(surface)) {
            if is_active {
                let id = winit::window::WindowId::from_raw(
                    surface.id().as_ptr() as usize,
                );
                if self.windows.iter().any(|w| w.window.id() == id) {
                    continue;
                }
                self.sctk_events.push(SctkEvent::Winit(
                    id,
                    winit::event::WindowEvent::Focused(true),
                ));
                self.sctk_events.push(SctkEvent::KeyboardEvent {
                    variant: KeyboardEventVariant::Enter(surface.clone()),
                    kbd_id: keyboard.clone(),
                    seat_id: seat.clone(),
                    surface: surface.clone(),
                });
            }
        }
    }

    fn leave(
        &mut self,
        _conn: &cctk::sctk::reexports::client::Connection,
        _qh: &cctk::sctk::reexports::client::QueueHandle<Self>,
        keyboard: &cctk::sctk::reexports::client::protocol::wl_keyboard::WlKeyboard,
        surface: &cctk::sctk::reexports::client::protocol::wl_surface::WlSurface,
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
        let surfaces = self.subsurfaces.iter().filter_map(|s| {
            (s.instance.parent == *surface).then(|| &s.instance.wl_surface)
        });
        for surface in surfaces.chain(std::iter::once(surface)) {
            if is_active {
                self.sctk_events.push(SctkEvent::KeyboardEvent {
                    variant: KeyboardEventVariant::Leave(surface.clone()),
                    kbd_id: kbd.clone(),
                    seat_id: seat.clone(),
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
                        continue;
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
    }

    fn press_key(
        &mut self,
        _conn: &cctk::sctk::reexports::client::Connection,
        _qh: &cctk::sctk::reexports::client::QueueHandle<Self>,
        keyboard: &cctk::sctk::reexports::client::protocol::wl_keyboard::WlKeyboard,
        serial: u32,
        event: cctk::sctk::seat::keyboard::KeyEvent,
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
            if let Some(surface) = my_seat.kbd_focus.clone() {
                self.request_redraw(&surface);
                let surfaces = self.subsurfaces.iter().filter_map(|s| {
                    (s.instance.parent == surface)
                        .then(|| &s.instance.wl_surface)
                });
                for surface in surfaces.chain(std::iter::once(&surface)) {
                    self.sctk_events.push(SctkEvent::KeyboardEvent {
                        variant: KeyboardEventVariant::Press(event.clone()),
                        kbd_id: kbd_id.clone(),
                        seat_id: seat_id.clone(),
                        surface: surface.clone(),
                    });
                }
            }
        }
    }

    fn release_key(
        &mut self,
        _conn: &cctk::sctk::reexports::client::Connection,
        _qh: &cctk::sctk::reexports::client::QueueHandle<Self>,
        keyboard: &cctk::sctk::reexports::client::protocol::wl_keyboard::WlKeyboard,
        _serial: u32,
        event: cctk::sctk::seat::keyboard::KeyEvent,
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
                let surfaces = self.subsurfaces.iter().filter_map(|s| {
                    (s.instance.parent == surface)
                        .then(|| &s.instance.wl_surface)
                });
                for surface in surfaces.chain(std::iter::once(&surface)) {
                    self.sctk_events.push(SctkEvent::KeyboardEvent {
                        variant: KeyboardEventVariant::Release(event.clone()),
                        kbd_id: kbd_id.clone(),
                        seat_id: seat_id.clone(),
                        surface: surface.clone(),
                    });
                }
            }
        }
    }

    fn update_modifiers(
        &mut self,
        _conn: &cctk::sctk::reexports::client::Connection,
        _qh: &cctk::sctk::reexports::client::QueueHandle<Self>,
        keyboard: &cctk::sctk::reexports::client::protocol::wl_keyboard::WlKeyboard,
        _serial: u32,
        modifiers: cctk::sctk::seat::keyboard::Modifiers,
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
                let surfaces = self.subsurfaces.iter().filter_map(|s| {
                    (s.instance.parent == surface)
                        .then(|| &s.instance.wl_surface)
                });
                for surface in surfaces.chain(std::iter::once(&surface)) {
                    self.sctk_events.push(SctkEvent::KeyboardEvent {
                        variant: KeyboardEventVariant::Modifiers(
                            modifiers.clone(),
                        ),
                        kbd_id: kbd_id.clone(),
                        seat_id: seat_id.clone(),
                        surface: surface.clone(),
                    });
                }
            }
        }
    }
}

delegate_keyboard!(SctkState);
