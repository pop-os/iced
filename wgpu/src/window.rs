//! Display rendering results on windows.
pub mod compositor;
#[cfg(unix)]
mod wayland;

pub use compositor::Compositor;
pub use wgpu::Surface;
