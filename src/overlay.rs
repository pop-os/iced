//! Display interactive elements on top of other widgets.

/// A generic [`Overlay`].
///
/// This is an alias of an `iced_native` element with a default `Renderer`.
///
/// [`Overlay`]: iced_native::Overlay
#[cfg(any(feature = "swbuf", feature = "glow", feature = "wgpu"))]
pub type Element<'a, Message, Renderer = crate::Renderer> =
    iced_native::overlay::Element<'a, Message, Renderer>;
#[cfg(not(any(feature = "swbuf", feature = "glow", feature = "wgpu")))]
pub use iced_native::overlay::Element;

pub mod menu {
    //! Build and show dropdown menus.
    pub use iced_native::overlay::menu::{Appearance, State, StyleSheet};

    /// A widget that produces a message when clicked.
    #[cfg(any(feature = "swbuf", feature = "glow", feature = "wgpu"))]
    pub type Menu<'a, Message, Renderer = crate::Renderer> =
        iced_native::overlay::Menu<'a, Message, Renderer>;
    #[cfg(not(any(
        feature = "swbuf",
        feature = "glow",
        feature = "wgpu"
    )))]
    pub use iced_native::overlay::Menu;
}
