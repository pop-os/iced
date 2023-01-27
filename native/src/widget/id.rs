use std::borrow;
use std::sync::atomic::{self, AtomicU64};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_WINDOW_ID: AtomicU64 = AtomicU64::new(1);

/// The identifier of a generic widget.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Id(Internal);

impl Id {
    /// Creates a custom [`Id`].
    pub fn new(id: impl Into<borrow::Cow<'static, str>>) -> Self {
        Self(Internal::Custom(Self::next(), id.into()))
    }

    /// resets the id counter
    pub fn reset() {
        NEXT_ID.store(1, atomic::Ordering::Relaxed);
    }

    fn next() -> u64 {
        NEXT_ID.fetch_add(1, atomic::Ordering::Relaxed)
    }

    /// Creates a unique [`Id`].
    ///
    /// This function produces a different [`Id`] every time it is called.
    pub fn unique() -> Self {
        let id = Self::next();

        Self(Internal::Unique(id))
    }

    #[cfg(feature = "a11y")]
    /// as accesskit::NodeId
    pub fn node_id(&self) -> accesskit::NodeId {
        use std::num::NonZeroU128;

        use accesskit::NodeId;

        match &self.0 {
            Internal::Unique(id) => {
                NodeId(NonZeroU128::try_from(*id as u128).unwrap())
            }
            Internal::Custom(id, _) => {
                NodeId(NonZeroU128::try_from(*id as u128).unwrap())
            }
        }
    }
}

impl ToString for Id {
    fn to_string(&self) -> String {
        match &self.0 {
            Internal::Unique(_) => "No Name".to_string(),
            Internal::Custom(_, id) => id.to_string(),
        }
    }
}

#[cfg(feature = "a11y")]
impl From<accesskit::NodeId> for Id {
    fn from(node_id: accesskit::NodeId) -> Self {
        Self(Internal::Unique(node_id.0.get() as u64))
    }
}

// XXX WIndow IDs are made unique by adding u64::MAX to them
#[cfg(feature = "a11y")]
/// get window node id that won't conflict with other node ids for the duration of the program
pub fn window_node_id() -> accesskit::NodeId {
    accesskit::NodeId(
        std::num::NonZeroU128::try_from(
            u64::MAX as u128
                + NEXT_WINDOW_ID.fetch_add(1, atomic::Ordering::Relaxed)
                    as u128,
        )
        .unwrap(),
    )
}

#[derive(Debug, Clone, Eq, Hash)]
pub enum Internal {
    Unique(u64),
    Custom(u64, borrow::Cow<'static, str>),
}

impl PartialEq for Internal {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Unique(l0), Self::Unique(r0)) => l0 == r0,
            (Self::Custom(l0, l1), Self::Custom(r0, r1)) => {
                l0 == r0 || l1 == r1
            }
            // allow custom ids to be equal to unique ids
            (Self::Unique(l0), Self::Custom(r0, _))
            | (Self::Custom(l0, _), Self::Unique(r0)) => l0 == r0,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Id;

    #[test]
    fn unique_generates_different_ids() {
        let a = Id::unique();
        let b = Id::unique();

        assert_ne!(a, b);
    }
}
