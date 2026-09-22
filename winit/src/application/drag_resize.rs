use crate::conversion;
use crate::core::mouse;
use cursor_icon::CursorIcon;
use winit::window::ResizeDirection;

#[cfg(any(
    all(
        unix,
        not(target_vendor = "apple"),
        not(target_os = "android"),
        not(target_os = "emscripten"),
    ),
    target_os = "windows",
))]
const DRAG_RESIZE_SUPPORTED: bool = true;

#[cfg(not(any(
    all(
        unix,
        not(target_vendor = "apple"),
        not(target_os = "android"),
        not(target_os = "emscripten"),
    ),
    target_os = "windows",
)))]
const DRAG_RESIZE_SUPPORTED: bool = false;

/// If supported by winit, returns a closure that implements cursor resize support.
///
/// The closure takes the cursor last set by the UI, which it restores when
/// the pointer leaves the resize border.
pub fn event_func(
    window: &dyn winit::window::Window,
    border_size: f64,
) -> Option<
    Box<
        dyn FnMut(
            &dyn winit::window::Window,
            &winit::event::WindowEvent,
            mouse::Interaction,
        ) -> bool,
    >,
> {
    if DRAG_RESIZE_SUPPORTED {
        // Keep track of cursor when it is within a resizeable border.
        let mut cursor_prev_resize_direction = None;

        Some(Box::new(
            move |window: &dyn winit::window::Window,
                  window_event: &winit::event::WindowEvent,
                  ui_interaction: mouse::Interaction|
                  -> bool {
                // Keep track of border resize state and set cursor icon when in range
                match window_event {
                    winit::event::WindowEvent::PointerMoved {
                        position,
                        ..
                    } => {
                        let location = if is_resizable(window) {
                            cursor_resize_direction(
                                window.surface_size(),
                                *position,
                                border_size,
                            )
                        } else {
                            None
                        };
                        if location != cursor_prev_resize_direction {
                            match location {
                                Some(direction) => window.set_cursor(
                                    resize_direction_cursor_icon(direction)
                                        .into(),
                                ),
                                None => restore_cursor(window, ui_interaction),
                            }
                            cursor_prev_resize_direction = location;
                            return true;
                        }
                    }
                    winit::event::WindowEvent::PointerLeft { .. } => {
                        if cursor_prev_resize_direction.take().is_some() {
                            restore_cursor(window, ui_interaction);
                        }
                    }
                    winit::event::WindowEvent::PointerButton {
                        state: winit::event::ElementState::Pressed,
                        button:
                            winit::event::ButtonSource::Mouse(
                                winit::event::MouseButton::Left,
                            )
                            | winit::event::ButtonSource::Touch {
                                finger_id: _, ..
                            },
                        primary: true,
                        ..
                    } => {
                        if let Some(direction) = cursor_prev_resize_direction
                            && is_resizable(window)
                        {
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

/// Whether the window can currently be resized from its border.
fn is_resizable(window: &dyn winit::window::Window) -> bool {
    !window.is_decorated()
        && !window.is_maximized()
        && window.fullscreen().is_none()
}

/// Set the cursor back to the one the UI last set.
fn restore_cursor(
    window: &dyn winit::window::Window,
    ui_interaction: mouse::Interaction,
) {
    // `None` means the UI hid the cursor, which a cursor icon doesn't undo.
    if let Some(icon) = conversion::mouse_interaction(ui_interaction) {
        window.set_cursor(icon.into());
    }
}

/// Get the cursor icon that corresponds to the resize direction.
fn resize_direction_cursor_icon(
    resize_direction: ResizeDirection,
) -> CursorIcon {
    match resize_direction {
        ResizeDirection::East => CursorIcon::EResize,
        ResizeDirection::North => CursorIcon::NResize,
        ResizeDirection::NorthEast => CursorIcon::NeResize,
        ResizeDirection::NorthWest => CursorIcon::NwResize,
        ResizeDirection::South => CursorIcon::SResize,
        ResizeDirection::SouthEast => CursorIcon::SeResize,
        ResizeDirection::SouthWest => CursorIcon::SwResize,
        ResizeDirection::West => CursorIcon::WResize,
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
