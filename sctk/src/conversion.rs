use iced_futures::core::mouse::Interaction;
use iced_runtime::core::{
    keyboard,
    mouse::{self, ScrollDelta},
};
use sctk::{
    reexports::client::protocol::wl_pointer::AxisSource,
    seat::{
        keyboard::Modifiers,
        pointer::{
            AxisScroll, CursorIcon, BTN_EXTRA, BTN_LEFT, BTN_MIDDLE, BTN_RIGHT,
            BTN_SIDE,
        },
    },
};

/// An error that occurred while running an application.
#[derive(Debug, thiserror::Error)]
#[error("the futures executor could not be created")]
pub struct KeyCodeError(u32);

pub fn pointer_button_to_native(button: u32) -> Option<mouse::Button> {
    if button == BTN_LEFT {
        Some(mouse::Button::Left)
    } else if button == BTN_RIGHT {
        Some(mouse::Button::Right)
    } else if button == BTN_MIDDLE {
        Some(mouse::Button::Middle)
    } else if button == BTN_SIDE {
        Some(mouse::Button::Back)
    } else if button == BTN_EXTRA {
        Some(mouse::Button::Forward)
    } else {
        button.try_into().ok().map(mouse::Button::Other)
    }
}

pub fn pointer_axis_to_native(
    source: Option<AxisSource>,
    horizontal: AxisScroll,
    vertical: AxisScroll,
) -> Option<ScrollDelta> {
    source.map(|source| match source {
        AxisSource::Wheel | AxisSource::WheelTilt => ScrollDelta::Lines {
            x: -1. * horizontal.discrete as f32,
            y: -1. * vertical.discrete as f32,
        },
        _ => ScrollDelta::Pixels {
            x: -1. * horizontal.absolute as f32,
            y: -1. * vertical.absolute as f32,
        },
    })
}

pub fn modifiers_to_native(mods: Modifiers) -> keyboard::Modifiers {
    let mut native_mods = keyboard::Modifiers::empty();
    if mods.alt {
        native_mods = native_mods.union(keyboard::Modifiers::ALT);
    }
    if mods.ctrl {
        native_mods = native_mods.union(keyboard::Modifiers::CTRL);
    }
    if mods.logo {
        native_mods = native_mods.union(keyboard::Modifiers::LOGO);
    }
    if mods.shift {
        native_mods = native_mods.union(keyboard::Modifiers::SHIFT);
    }
    // TODO Ashley: missing modifiers as platform specific additions?
    // if mods.caps_lock {
    // native_mods = native_mods.union(keyboard::Modifier);
    // }
    // if mods.num_lock {
    //     native_mods = native_mods.union(keyboard::Modifiers::);
    // }
    native_mods
}

// pub fn keysym_to_vkey(keysym: RawKeysym) -> Option<KeyCode> {
//     key_conversion.get(&keysym).cloned()
// }

pub(crate) fn cursor_icon(cursor: Interaction) -> Option<CursorIcon> {
    Some(match cursor {
        Interaction::Idle => CursorIcon::Default,
        Interaction::Pointer => CursorIcon::Pointer,
        Interaction::Grab => CursorIcon::Grab,
        Interaction::Text => CursorIcon::Text,
        Interaction::Crosshair => CursorIcon::Crosshair,
        Interaction::Working => CursorIcon::Progress,
        Interaction::Grabbing => CursorIcon::Grabbing,
        Interaction::ResizingHorizontally => CursorIcon::EwResize,
        Interaction::ResizingVertically => CursorIcon::NsResize,
        Interaction::NotAllowed => CursorIcon::NotAllowed,
        Interaction::Hide => return None,
    })
}
