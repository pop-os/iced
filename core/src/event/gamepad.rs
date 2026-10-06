//! Handle events from game controllers.

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Connected(Gamepad),

    Disconnected(Gamepad),

    ButtonPressed {
        gamepad: Gamepad,
        button: Button,
        repeated: bool,
    },

    ButtonReleased {
        gamepad: Gamepad,
        button: Button,
    },

    ButtonChanged {
        gamepad: Gamepad,
        button: Button,
        value: f32,
    },

    AxisChanged {
        gamepad: Gamepad,
        axis: Axis,
        value: f32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Gamepad(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Button {
    South,
    East,
    North,
    West,
    LeftTrigger,
    LeftTrigger2,
    RightTrigger,
    RightTrigger2,
    Select,
    Start,
    Mode,
    LeftThumb,
    RightThumb,
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Axis {
    LeftStickX,
    LeftStickY,
    LeftZ,
    RightStickX,
    RightStickY,
    RightZ,
    DPadX,
    DPadY,
}
