use winit::window::{CursorIcon, ResizeDirection};

/// If supported by winit, returns a closure that implements cursor resize support.
pub fn event_func(
    window: &dyn winit::window::Window,
    border_size: f64,
) -> Option<
    Box<
        dyn FnMut(
            &dyn winit::window::Window,
            &winit::event::WindowEvent,
        ) -> bool,
    >,
> {
    if drag_resize_supported() {
        // Keep track of cursor when it is within a resizeable border.
        let mut cursor_prev_resize_direction = None;

        Some(Box::new(
            move |window: &dyn winit::window::Window,
                  window_event: &winit::event::WindowEvent|
                  -> bool {
                // Keep track of border resize state and set cursor icon when in range
                match window_event {
                    winit::event::WindowEvent::CursorMoved {
                        position, ..
                    } => {
                        if !window.is_decorated() {
                            let location = cursor_resize_direction(
                                window.surface_size(),
                                *position,
                                border_size,
                            );
                            if location != cursor_prev_resize_direction {
                                window.set_cursor(
                                    resize_direction_cursor_icon(location)
                                        .into(),
                                );
                                cursor_prev_resize_direction = location;
                                return true;
                            }
                        }
                    }
                    winit::event::WindowEvent::MouseInput {
                        state: winit::event::ElementState::Pressed,
                        button: winit::event::MouseButton::Left,
                        ..
                    } => {
                        if let Some(direction) = cursor_prev_resize_direction {
                            let _res = window.drag_resize_window(direction);
                            return true;
                        }
                    }
                    _ => (),
                }

                false
            },
        ))
    } else {
        None
    }
}

/// Test if the current target should be assumed to have winit drag_resize support
const fn drag_resize_supported() -> bool {
    #[cfg(all(
        unix,
        not(target_vendor = "apple"),
        not(target_os = "android"),
        not(target_os = "emscripten")
    ))]
    {
        return true;
    }

    #[cfg(target_os = "windows")]
    {
        return true;
    }

    #[allow(unreachable_code)]
    false
}

/// Get the cursor icon that corresponds to the resize direction.
fn resize_direction_cursor_icon(
    resize_direction: Option<ResizeDirection>,
) -> CursorIcon {
    match resize_direction {
        Some(resize_direction) => match resize_direction {
            ResizeDirection::East => CursorIcon::EResize,
            ResizeDirection::North => CursorIcon::NResize,
            ResizeDirection::NorthEast => CursorIcon::NeResize,
            ResizeDirection::NorthWest => CursorIcon::NwResize,
            ResizeDirection::South => CursorIcon::SResize,
            ResizeDirection::SouthEast => CursorIcon::SeResize,
            ResizeDirection::SouthWest => CursorIcon::SwResize,
            ResizeDirection::West => CursorIcon::WResize,
        },
        None => CursorIcon::Default,
    }
}

/// Identifies resize direction based on cursor position and window dimensions.
#[allow(clippy::similar_names)]
fn cursor_resize_direction(
    win_size: winit::dpi::PhysicalSize<u32>,
    position: winit::dpi::PhysicalPosition<f64>,
    border_size: f64,
) -> Option<ResizeDirection> {
    enum XDirection {
        West,
        East,
        Default,
    }

    enum YDirection {
        North,
        South,
        Default,
    }

    let xdir = if position.x < border_size {
        XDirection::West
    } else if position.x > (win_size.width as f64 - border_size) {
        XDirection::East
    } else {
        XDirection::Default
    };

    let ydir = if position.y < border_size {
        YDirection::North
    } else if position.y > (win_size.height as f64 - border_size) {
        YDirection::South
    } else {
        YDirection::Default
    };

    Some(match xdir {
        XDirection::West => match ydir {
            YDirection::North => ResizeDirection::NorthWest,
            YDirection::South => ResizeDirection::SouthWest,
            YDirection::Default => ResizeDirection::West,
        },

        XDirection::East => match ydir {
            YDirection::North => ResizeDirection::NorthEast,
            YDirection::South => ResizeDirection::SouthEast,
            YDirection::Default => ResizeDirection::East,
        },

        XDirection::Default => match ydir {
            YDirection::North => ResizeDirection::North,
            YDirection::South => ResizeDirection::South,
            YDirection::Default => return None,
        },
    })
}
