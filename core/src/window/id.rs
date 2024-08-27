use std::hash::Hash;

use std::sync::atomic::{self, AtomicU64};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
/// The id of the window.
///
/// Internally Iced reserves `window::Id::MAIN` for the first window spawned.
pub struct Id(u64);

static COUNT: AtomicU64 = AtomicU64::new(1);

impl Id {
    /// No window will match this Id
    pub const NONE: Id = Id(0);

    /// Creates a new unique window [`Id`].
    pub fn unique() -> Id {
        let id = Id(COUNT.fetch_add(1, atomic::Ordering::Relaxed));
        if id.0 == 0 {
            Id(COUNT.fetch_add(1, atomic::Ordering::Relaxed))
        } else {
            id
        }
    }
}
