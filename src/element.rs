/// A generic widget.
///
/// This is an alias of an `iced_native` element with a default `Renderer`.
#[cfg(any(
    feature = "softbuffer",
    feature = "glow",
    feature = "wgpu",
    feature = "dyrend"
))]
pub type Element<'a, Message, Renderer = crate::Renderer> =
    iced_native::Element<'a, Message, Renderer>;
