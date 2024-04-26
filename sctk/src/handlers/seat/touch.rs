// TODO handle multiple seats?

use crate::{event_loop::state::SctkState, sctk_event::SctkEvent};
use iced_runtime::core::{touch, Point};
use sctk::{
    delegate_touch,
    reexports::client::{
        protocol::{wl_surface::WlSurface, wl_touch::WlTouch},
        Connection, QueueHandle,
    },
    seat::touch::TouchHandler,
};
use std::fmt::Debug;

impl<T: Debug> TouchHandler for SctkState<T> {
    fn down(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        touch: &WlTouch,
        serial: u32,
        time: u32,
        surface: WlSurface,
        id: i32,
        position: (f64, f64),
    ) {
        let Some(my_seat) = self
            .seats
            .iter_mut()
            .find(|s| s.touch.as_ref() == Some(touch))
        else {
            return;
        };

        my_seat.last_touch_down.replace((time, id, serial));

        let id = touch::Finger(id as u64);
        let position = Point::new(position.0 as f32, position.1 as f32);
        self.touch_points.insert(id, (surface.clone(), position));
        self.sctk_events.push(SctkEvent::TouchEvent {
            variant: touch::Event::FingerPressed { id, position },
            touch_id: touch.clone(),
            seat_id: my_seat.seat.clone(),
            surface,
        });
    }

    fn up(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        touch: &WlTouch,
        _serial: u32,
        _time: u32,
        id: i32,
    ) {
        let Some(my_seat) =
            self.seats.iter().find(|s| s.touch.as_ref() == Some(touch))
        else {
            return;
        };

        let id = touch::Finger(id as u64);
        if let Some((surface, position)) = self.touch_points.get(&id).cloned() {
            self.sctk_events.push(SctkEvent::TouchEvent {
                variant: touch::Event::FingerLifted { id, position },
                touch_id: touch.clone(),
                seat_id: my_seat.seat.clone(),
                surface,
            });
        }
    }
    fn motion(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        touch: &WlTouch,
        _time: u32,
        id: i32,
        position: (f64, f64),
    ) {
        let Some(my_seat) =
            self.seats.iter().find(|s| s.touch.as_ref() == Some(touch))
        else {
            return;
        };

        let id = touch::Finger(id as u64);
        let position = Point::new(position.0 as f32, position.1 as f32);
        if let Some((surface, position_ref)) = self.touch_points.get_mut(&id) {
            *position_ref = position;
            self.sctk_events.push(SctkEvent::TouchEvent {
                variant: touch::Event::FingerMoved { id, position },
                touch_id: touch.clone(),
                seat_id: my_seat.seat.clone(),
                surface: surface.clone(),
            });
        }
    }

    fn shape(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlTouch,
        _: i32,
        _: f64,
        _: f64,
    ) {
    }

    fn orientation(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlTouch,
        _: i32,
        _: f64,
    ) {
    }

    fn cancel(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        touch: &WlTouch,
    ) {
        let Some(my_seat) =
            self.seats.iter().find(|s| s.touch.as_ref() == Some(touch))
        else {
            return;
        };

        for (id, (surface, position)) in self.touch_points.drain() {
            self.sctk_events.push(SctkEvent::TouchEvent {
                variant: touch::Event::FingerLost { id, position },
                touch_id: touch.clone(),
                seat_id: my_seat.seat.clone(),
                surface,
            });
        }
    }
}

delegate_touch!(@<T: 'static + Debug> SctkState<T>);
