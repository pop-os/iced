use std::{fmt, marker::PhantomData};

use iced_core::event::wayland::{KeyEvent, RawModifiers};
use iced_futures::MaybeSend;

/// Virtual keyboard action
pub struct Action<T> {
    /// The inner action
    pub inner: ActionInner,
    /// The phantom data
    _phantom: PhantomData<T>,
}

impl<T> From<ActionInner> for Action<T> {
    fn from(inner: ActionInner) -> Self {
        Self {
            inner,
            _phantom: PhantomData,
        }
    }
}

impl<T> fmt::Debug for Action<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

/// Virtual keyboard Actions
pub enum ActionInner {
    /// Forward a keypress to client
    KeyPressed(KeyEvent),
    /// Forward a keyrelease to client
    KeyReleased(KeyEvent),
    /// Forward modifiers to client
    Modifiers(RawModifiers),
}

impl<T> Action<T> {
    /// Maps the output of a virtual keyboard [`Action`] using the provided closure.
    pub fn map<A>(
        self,
        _: impl Fn(T) -> A + 'static + MaybeSend + Sync,
    ) -> Action<A>
    where
        T: 'static,
    {
        Action::from(self.inner)
    }
}

impl fmt::Debug for ActionInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::KeyPressed(key) => {
                f.debug_tuple("Key event").field(key).finish()
            }
            Self::KeyReleased(key) => {
                f.debug_tuple("Key event").field(key).finish()
            }
            Self::Modifiers(modifiers) => {
                f.debug_tuple("Modifier event").field(modifiers).finish()
            }
        }
    }
}
