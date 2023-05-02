use std::fmt;
use std::hash::Hash;
use std::sync::atomic::{self, AtomicU64};

/// The id of the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Id(u64);

static COUNT: AtomicU64 = AtomicU64::new(1);

impl Id {
    /// No window will match this Id
    pub const NONE: Id = Id(0);
    pub const RESERVED: Id = Id(1);

    /// Creates a new unique window [`Id`].
    pub fn unique() -> Id {
        let id = Id(COUNT.fetch_add(1, atomic::Ordering::Relaxed));
        if id.0 == 0 {
            Id(COUNT.fetch_add(2, atomic::Ordering::Relaxed))
        } else if id.0 == 1 {
            Id(COUNT.fetch_add(1, atomic::Ordering::Relaxed))
        } else {
            id
        }
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
