//! Configure the window of your application in native platforms.
#[cfg(feature = "winit")]
pub mod icon;
mod position;
mod settings;
#[cfg(feature = "winit")]
pub use icon::Icon;
pub use position::Position;
pub use settings::Settings;

#[cfg(all(not(target_arch = "wasm32"), feature = "winit"))]
pub use crate::runtime::window::move_to;
#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "wayland", feature = "winit")
))]
pub use crate::runtime::window::resize;

pub use iced_native::window::Id;
