mod a11y_tree;
mod id;
mod node;

pub use a11y_tree::*;
pub use accesskit;
pub use id::*;
pub use node::*;

#[cfg(feature = "accesskit_macos")]
pub use accesskit_macos;
#[cfg(feature = "accesskit_unix")]
pub use accesskit_unix;
#[cfg(feature = "accesskit_windows")]
pub use accesskit_windows;
#[cfg(feature = "accesskit_winit")]
pub use accesskit_winit;
