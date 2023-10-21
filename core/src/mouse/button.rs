/// The button of a mouse.
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum Button {
    /// The left mouse button.
    Left,

    /// The right mouse button.
    Right,

    /// The middle (wheel) button.
    Middle,

    /// The side button often used as "back" in web browsers.
    Back,

    /// The side button often used as "forward" in web browsers.
    Forward,

    /// Some other button.
    Other(u16),
}
