//! Listen for events from game controllers.
use crate::core;

use std::sync::mpsc;

/// Gamepads are not listened for on the web, where gilrs cannot block.
pub fn listen(
    on_event: impl Fn() + Send + Sync + 'static,
) -> Option<mpsc::Receiver<core::event::gamepad::Event>> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = on_event;

        None
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::time::Duration;

        let (sender, receiver) = mpsc::channel();

        let _ = std::thread::Builder::new()
            .name(String::from("gamepad"))
            .spawn(move || {
                let Ok(mut gilrs) = gilrs::Gilrs::new() else {
                    return;
                };

                loop {
                    let Some(event) = gilrs.next_event_blocking(None) else {
                        std::thread::sleep(Duration::from_millis(1));
                        continue;
                    };

                    let send = |event| {
                        if let Some(event) = convert(event) {
                            if sender.send(event).is_err() {
                                return false;
                            }

                            on_event();
                        }

                        true
                    };

                    if !send(event) {
                        return;
                    }

                    while let Some(event) = gilrs.next_event() {
                        if !send(event) {
                            return;
                        }
                    }
                }
            })
            .ok()?;

        Some(receiver)
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn convert(event: gilrs::Event) -> Option<core::event::gamepad::Event> {
    let gamepad = core::event::gamepad::Gamepad(event.id.into());

    Some(match event.event {
        gilrs::EventType::Connected => {
            core::event::gamepad::Event::Connected(gamepad)
        }
        gilrs::EventType::Disconnected => {
            core::event::gamepad::Event::Disconnected(gamepad)
        }
        gilrs::EventType::ButtonPressed(button, _) => {
            core::event::gamepad::Event::ButtonPressed {
                gamepad,
                button: convert_button(button)?,
            }
        }
        gilrs::EventType::ButtonReleased(button, _) => {
            core::event::gamepad::Event::ButtonReleased {
                gamepad,
                button: convert_button(button)?,
            }
        }
        gilrs::EventType::ButtonChanged(button, value, _) => {
            core::event::gamepad::Event::ButtonChanged {
                gamepad,
                button: convert_button(button)?,
                value,
            }
        }
        gilrs::EventType::AxisChanged(axis, value, _) => {
            core::event::gamepad::Event::AxisChanged {
                gamepad,
                axis: convert_axis(axis)?,
                value,
            }
        }
        _ => return None,
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn convert_button(
    button: gilrs::Button,
) -> Option<core::event::gamepad::Button> {
    use core::event::gamepad::Button;

    Some(match button {
        gilrs::Button::South => Button::South,
        gilrs::Button::East => Button::East,
        gilrs::Button::North => Button::North,
        gilrs::Button::West => Button::West,
        gilrs::Button::LeftTrigger => Button::LeftTrigger,
        gilrs::Button::LeftTrigger2 => Button::LeftTrigger2,
        gilrs::Button::RightTrigger => Button::RightTrigger,
        gilrs::Button::RightTrigger2 => Button::RightTrigger2,
        gilrs::Button::Select => Button::Select,
        gilrs::Button::Start => Button::Start,
        gilrs::Button::Mode => Button::Mode,
        gilrs::Button::LeftThumb => Button::LeftThumb,
        gilrs::Button::RightThumb => Button::RightThumb,
        gilrs::Button::DPadUp => Button::DPadUp,
        gilrs::Button::DPadDown => Button::DPadDown,
        gilrs::Button::DPadLeft => Button::DPadLeft,
        gilrs::Button::DPadRight => Button::DPadRight,
        gilrs::Button::C | gilrs::Button::Z | gilrs::Button::Unknown => {
            return None;
        }
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn convert_axis(axis: gilrs::Axis) -> Option<core::event::gamepad::Axis> {
    use core::event::gamepad::Axis;

    Some(match axis {
        gilrs::Axis::LeftStickX => Axis::LeftStickX,
        gilrs::Axis::LeftStickY => Axis::LeftStickY,
        gilrs::Axis::LeftZ => Axis::LeftZ,
        gilrs::Axis::RightStickX => Axis::RightStickX,
        gilrs::Axis::RightStickY => Axis::RightStickY,
        gilrs::Axis::RightZ => Axis::RightZ,
        gilrs::Axis::DPadX => Axis::DPadX,
        gilrs::Axis::DPadY => Axis::DPadY,
        gilrs::Axis::Unknown => return None,
    })
}
