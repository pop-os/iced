#[cfg(feature = "clipboard")]
mod clipboard;

#[cfg(not(feature = "clipboard"))]
mod clipboard {
    use std::ffi::c_void;
    /// A buffer for short-term storage and transfer within and between
    /// applications.
    #[allow(missing_debug_implementations)]
    pub struct Clipboard;

    pub(crate) enum State {
        Connected(()),
        Unavailable,
    }

    impl Clipboard {
        pub unsafe fn connect(_display: *mut c_void) -> Clipboard {
            Clipboard
        }

        pub(crate) fn state(&self) -> &State {
            &State::Connected(())
        }

        /// Creates a new [`Clipboard`]
        pub fn unconnected() -> Clipboard {
            Clipboard
        }
    }

    impl iced_runtime::core::clipboard::Clipboard for Clipboard {
        fn read(&self) -> Option<String> {
            None
        }

        fn write(&mut self, _contents: String) {}
    }
}

pub use clipboard::*;
