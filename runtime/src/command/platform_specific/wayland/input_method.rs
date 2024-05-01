use iced_futures::MaybeSend;
use std::fmt;
use std::marker::PhantomData;

/// Input Method Action
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

/// Input Method Actions
pub enum ActionInner {
    /// Apply state
    Commit,
    /// Send string to client
    CommitString(String),
    /// Set preedit string
    SetPreeditString {
        /// String to be set
        string: String,
        /// Start of preedit cursor
        cursor_begin: i32,
        /// Preedit cursor end
        cursor_end: i32,
    },
    /// Delete surrounding text
    DeleteSurroundingText {
        /// Number of bytes before current cursor index (excluding the preedit text) to delete
        before_length: u32,
        /// Number of bytes after current cursor index (excluding the preedit text) to delete
        after_length: u32,
    },
}

impl<T> Action<T> {
    /// Maps the output of a window [`Action`] using the provided closure.
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
            Self::Commit => f.debug_tuple("Commit").finish(),
            Self::CommitString(string) => {
                f.debug_tuple("Commit String").field(string).finish()
            }
            Self::SetPreeditString {
                string,
                cursor_begin,
                cursor_end,
            } => f
                .debug_tuple("Set Preedit String")
                .field(string)
                .field(cursor_begin)
                .field(cursor_end)
                .finish(),
            Self::DeleteSurroundingText {
                before_length,
                after_length,
            } => f
                .debug_tuple("Delete Sorrunding Text")
                .field(before_length)
                .field(after_length)
                .finish(),
        }
    }
}
