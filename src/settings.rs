//! Configure your application.
use crate::{Font, Pixels};

use std::borrow::Cow;

/// The settings of an iced program.
#[derive(Debug, Clone)]
pub struct Settings {
    /// The identifier of the application.
    ///
    /// If provided, this identifier may be used to identify the application or
    /// communicate with it through the windowing system.
    pub id: Option<String>,

    /// The fonts to load on boot.
    pub fonts: Vec<Cow<'static, [u8]>>,

    /// The default [`Font`] to be used.
    ///
    /// By default, it uses [`Family::SansSerif`](crate::font::Family::SansSerif).
    pub default_font: Font,

    /// The text size that will be used by default.
    ///
    /// The default value is `16.0`.
    pub default_text_size: Pixels,

    /// If set to true, the renderer will try to perform antialiasing for some
    /// primitives.
    ///
    /// Enabling it can produce a smoother result in some widgets
    ///
    /// By default, it is disabled.
    pub antialiasing: bool,

    /// If set to true the application will exit when the main window is closed.
    pub exit_on_close_request: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            id: None,
            fonts: Vec::new(),
            default_font: Font::default(),
            default_text_size: Pixels(14.0),
            antialiasing: false,
            exit_on_close_request: false,
        }
    }
}

#[cfg(feature = "winit")]
impl From<Settings> for iced_winit::Settings {
    fn from(settings: Settings) -> iced_winit::Settings {
        iced_winit::Settings {
            id: settings.id,
            fonts: settings.fonts,
        }
    }
}

// #[cfg(feature = "wayland")]
// impl<Flags> Settings<Flags> {
//     /// Initialize [`Application`] settings using the given data.
//     ///
//     /// [`Application`]: crate::Application
//     pub fn with_flags(flags: Flags) -> Self {
//         let default_settings = Settings::<()>::default();

//         Self {
//             flags,
//             id: default_settings.id,
//             initial_surface: default_settings.initial_surface,
//             default_font: default_settings.default_font,
//             default_text_size: default_settings.default_text_size,
//             antialiasing: default_settings.antialiasing,
//             exit_on_close_request: default_settings.exit_on_close_request,
//             fonts: default_settings.fonts,
//         }
//     }
// }

// #[cfg(feature = "wayland")]
// impl<Flags> Default for Settings<Flags>
// where
//     Flags: Default,
// {
//     fn default() -> Self {
//         Self {
//             id: None,
//             initial_surface: Default::default(),
//             flags: Default::default(),
//             default_font: Default::default(),
//             default_text_size: Pixels(14.0),
//             antialiasing: false,
//             fonts: Vec::new(),
//             exit_on_close_request: true,
//         }
//     }
// }

// #[cfg(feature = "wayland")]
// impl<Flags> From<Settings<Flags>> for iced_sctk::Settings<Flags> {
//     fn from(settings: Settings<Flags>) -> iced_sctk::Settings<Flags> {
//         iced_sctk::Settings {
//             kbd_repeat: Default::default(),
//             surface: settings.initial_surface,
//             flags: settings.flags,
//             exit_on_close_request: settings.exit_on_close_request,
//             ptr_theme: None,
//             control_flow_timeout: Some(std::time::Duration::from_millis(250)),
//         }
//     }
// }
