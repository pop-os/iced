use std::collections::HashMap;

use iced_futures::core::mouse::Interaction;
use iced_runtime::core::{
    keyboard::{self, KeyCode},
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
use xkeysym::{key, RawKeysym};

lazy_static::lazy_static! {
    pub static ref key_conversion: HashMap<u32, KeyCode> = [
        (key::_1, KeyCode::Key1), (key::_2, KeyCode::Key2), (key::_3, KeyCode::Key3), (key::_4, KeyCode::Key4), (key::_5, KeyCode::Key5), (key::_6, KeyCode::Key6), (key::_7, KeyCode::Key7), (key::_8, KeyCode::Key8), (key::_9, KeyCode::Key9), (key::_0, KeyCode::Key0),
        // Letters.
        (key::A, KeyCode::A), (key::a, KeyCode::A), (key::B, KeyCode::B), (key::b, KeyCode::B), (key::C, KeyCode::C), (key::c, KeyCode::C), (key::D, KeyCode::D), (key::d, KeyCode::D), (key::E, KeyCode::E), (key::e, KeyCode::E), (key::F, KeyCode::F), (key::f, KeyCode::F), (key::G, KeyCode::G), (key::g, KeyCode::G), (key::H, KeyCode::H), (key::h, KeyCode::H), (key::I, KeyCode::I), (key::i, KeyCode::I), (key::J, KeyCode::J), (key::j, KeyCode::J), (key::K, KeyCode::K), (key::k, KeyCode::K), (key::L, KeyCode::L), (key::l, KeyCode::L), (key::M, KeyCode::M), (key::m, KeyCode::M), (key::N, KeyCode::N), (key::n, KeyCode::N), (key::O, KeyCode::O), (key::o, KeyCode::O), (key::P, KeyCode::P), (key::p, KeyCode::P), (key::Q, KeyCode::Q), (key::q, KeyCode::Q), (key::R, KeyCode::R), (key::r, KeyCode::R), (key::S, KeyCode::S), (key::s, KeyCode::S), (key::T, KeyCode::T), (key::t, KeyCode::T), (key::U, KeyCode::U), (key::u, KeyCode::U), (key::V, KeyCode::V), (key::v, KeyCode::V), (key::W, KeyCode::W), (key::w, KeyCode::W), (key::X, KeyCode::X), (key::x, KeyCode::X), (key::Y, KeyCode::Y), (key::y, KeyCode::Y), (key::Z, KeyCode::Z), (key::z, KeyCode::Z),
        // Escape.
        (key::Escape, KeyCode::Escape),
        // Function keys.
        (key::F1, KeyCode::F1), (key::F2, KeyCode::F2), (key::F3, KeyCode::F3), (key::F4, KeyCode::F4), (key::F5, KeyCode::F5), (key::F6, KeyCode::F6), (key::F7, KeyCode::F7), (key::F8, KeyCode::F8), (key::F9, KeyCode::F9), (key::F10, KeyCode::F10), (key::F11, KeyCode::F11), (key::F12, KeyCode::F12), (key::F13, KeyCode::F13), (key::F14, KeyCode::F14), (key::F15, KeyCode::F15), (key::F16, KeyCode::F16), (key::F17, KeyCode::F17), (key::F18, KeyCode::F18), (key::F19, KeyCode::F19), (key::F20, KeyCode::F20), (key::F21, KeyCode::F21), (key::F22, KeyCode::F22), (key::F23, KeyCode::F23), (key::F24, KeyCode::F24),
        // Flow control.
        (key::Print, KeyCode::Snapshot), (key::Scroll_Lock, KeyCode::Scroll), (key::Pause, KeyCode::Pause), (key::Insert, KeyCode::Insert), (key::Home, KeyCode::Home), (key::Delete, KeyCode::Delete), (key::End, KeyCode::End), (key::Page_Down, KeyCode::PageDown), (key::Page_Up, KeyCode::PageUp),
        // Arrows.
        (key::Left, KeyCode::Left), (key::Up, KeyCode::Up), (key::Right, KeyCode::Right), (key::Down, KeyCode::Down), (key::BackSpace, KeyCode::Backspace), (key::Return, KeyCode::Enter), (key::space, KeyCode::Space), (key::Multi_key, KeyCode::Compose), (key::caret, KeyCode::Caret),
        // Keypad.
        (key::Num_Lock, KeyCode::Numlock), (key::KP_0, KeyCode::Numpad0), (key::KP_1, KeyCode::Numpad1), (key::KP_2, KeyCode::Numpad2), (key::KP_3, KeyCode::Numpad3), (key::KP_4, KeyCode::Numpad4), (key::KP_5, KeyCode::Numpad5), (key::KP_6, KeyCode::Numpad6), (key::KP_7, KeyCode::Numpad7), (key::KP_8, KeyCode::Numpad8), (key::KP_9, KeyCode::Numpad9),
        // Misc.
        // => Some(KeyCode::AbntC1),
        // => Some(KeyCode::AbntC2),
        (key::plus, KeyCode::Plus), (key::apostrophe, KeyCode::Apostrophe),
        // => Some(KeyCode::Apps),
        (key::at, KeyCode::At),
        // => Some(KeyCode::Ax),
        (key::backslash, KeyCode::Backslash), (key::XF86_Calculator, KeyCode::Calculator),
        // => Some(KeyCode::Capital),
        (key::colon, KeyCode::Colon), (key::comma, KeyCode::Comma),
        // => Some(KeyCode::Convert),
        (key::equal, KeyCode::Equals), (key::grave, KeyCode::Grave),
        // => Some(KeyCode::Kana),
        (key::Kanji, KeyCode::Kanji), (key::Alt_L, KeyCode::LAlt), (key::bracketleft, KeyCode::LBracket), (key::Control_L, KeyCode::LControl), (key::Shift_L, KeyCode::LShift), (key::Super_L, KeyCode::LWin), (key::XF86_Mail, KeyCode::Mail),
        // => Some(KeyCode::MediaSelect),
        // => Some(KeyCode::MediaStop),
        (key::minus, KeyCode::Minus), (key::asterisk, KeyCode::Asterisk), (key::XF86_AudioMute, KeyCode::Mute),
        // => Some(KeyCode::MyComputer),
        (key::XF86_AudioNext, KeyCode::NextTrack),
        // => Some(KeyCode::NoConvert),
        (key::KP_Separator, KeyCode::NumpadComma), (key::KP_Enter, KeyCode::NumpadEnter), (key::KP_Equal, KeyCode::NumpadEquals), (key::KP_Add, KeyCode::NumpadAdd), (key::KP_Subtract, KeyCode::NumpadSubtract), (key::KP_Multiply, KeyCode::NumpadMultiply), (key::KP_Divide, KeyCode::NumpadDivide), (key::KP_Decimal, KeyCode::NumpadDecimal), (key::KP_Page_Up, KeyCode::PageUp), (key::KP_Page_Down, KeyCode::PageDown), (key::KP_Home, KeyCode::Home), (key::KP_End, KeyCode::End), (key::KP_Left, KeyCode::Left), (key::KP_Up, KeyCode::Up), (key::KP_Right, KeyCode::Right), (key::KP_Down, KeyCode::Down),
        // => Some(KeyCode::OEM102),
        (key::period, KeyCode::Period),
        // => Some(KeyCode::Playpause),
        (key::XF86_PowerOff, KeyCode::Power), (key::XF86_AudioPrev, KeyCode::PrevTrack), (key::Alt_R, KeyCode::RAlt), (key::bracketright, KeyCode::RBracket), (key::Control_R, KeyCode::RControl), (key::Shift_R, KeyCode::RShift), (key::Super_R, KeyCode::RWin), (key::semicolon, KeyCode::Semicolon), (key::slash, KeyCode::Slash), (key::XF86_Sleep, KeyCode::Sleep),
        // => Some(KeyCode::Stop),
        // => Some(KeyCode::Sysrq),
        (key::Tab, KeyCode::Tab), (key::ISO_Left_Tab, KeyCode::Tab), (key::underscore, KeyCode::Underline),
        // => Some(KeyCode::Unlabeled),
        (key::XF86_AudioLowerVolume, KeyCode::VolumeDown), (key::XF86_AudioRaiseVolume, KeyCode::VolumeUp),
        // => Some(KeyCode::Wake),
        // => Some(KeyCode::Webback),
        // => Some(KeyCode::WebFavorites),
        // => Some(KeyCode::WebForward),
        // => Some(KeyCode::WebHome),
        // => Some(KeyCode::WebRefresh),
        // => Some(KeyCode::WebSearch),
        // => Some(KeyCode::WebStop),
        (key::yen, KeyCode::Yen), (key::XF86_Copy, KeyCode::Copy), (key::XF86_Paste, KeyCode::Paste), (key::XF86_Cut, KeyCode::Cut)
    ].iter().copied().collect();
}

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
    natural_scroll: bool,
) -> Option<ScrollDelta> {
    source.map(|source| match source {
        AxisSource::Wheel | AxisSource::WheelTilt => {
            if natural_scroll {
                ScrollDelta::Lines {
                    x: horizontal.discrete as f32,
                    y: vertical.discrete as f32,
                }
            } else {
                ScrollDelta::Lines {
                    x: -horizontal.discrete as f32,
                    y: -vertical.discrete as f32,
                }
            }
        }
        _ => {
            if natural_scroll {
                ScrollDelta::Pixels {
                    x: horizontal.absolute as f32,
                    y: vertical.absolute as f32,
                }
            } else {
                ScrollDelta::Pixels {
                    x: (-1.0 * horizontal.absolute) as f32,
                    y: (-1.0 * vertical.absolute) as f32,
                }
            }
        }
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

pub fn keysym_to_vkey(keysym: RawKeysym) -> Option<KeyCode> {
    key_conversion.get(&keysym).cloned()
}

pub(crate) fn cursor_icon(cursor: Interaction) -> CursorIcon {
    match cursor {
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
    }
}
