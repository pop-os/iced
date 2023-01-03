/// A generic widget.
///
/// This is an alias of an `iced_native` element with a default `Renderer`.
#[cfg(any(feature = "winit", feature = "wayland"))]
pub type Element<'a, Message, Renderer = crate::Renderer> =
    crate::runtime::Element<'a, Message, Renderer>;
