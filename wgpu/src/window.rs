//! Display rendering results on windows.
pub mod compositor;
#[cfg(all(unix, not(target_os = "macos")))]
mod wayland;

pub use compositor::Compositor;
pub use wgpu::Surface;
