use std::num::NonZeroU128;

use iced_core::Id;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum A11yId {
    Window(NonZeroU128),
    Widget(iced_core::Id),
}

// impl A11yId {
//     pub fn new_widget() -> Self {
//         Self::Widget(Id::unique())
//     }

//     pub fn new_window() -> Self {
//         Self::Window(iced_core::window_node_id())
//     }
// }

impl From<NonZeroU128> for A11yId {
    fn from(id: NonZeroU128) -> Self {
        Self::Window(id)
    }
}

impl From<iced_core::Id> for A11yId {
    fn from(id: iced_core::Id) -> Self {
        Self::Widget(id)
    }
}

impl From<accesskit::NodeId> for A11yId {
    fn from(value: accesskit::NodeId) -> Self {
        let val = u128::from(value.0);
        if val > u64::MAX as u128 {
            Self::Window(value.0)
        } else {
            Self::Widget(Id::from(val as u64))
        }
    }
}

impl Into<accesskit::NodeId> for A11yId {
    fn into(self) -> accesskit::NodeId {
        let node_id = match self {
            Self::Window(id) => id,
            Self::Widget(id) => id.into(),
        };
        accesskit::NodeId(node_id)
    }
}
