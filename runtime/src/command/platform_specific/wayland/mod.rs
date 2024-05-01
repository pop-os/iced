//! Wayland specific actions

use std::fmt::Debug;

use iced_futures::MaybeSend;

/// activation Actions
pub mod activation;
/// data device Actions
pub mod data_device;
/// input method actions
pub mod input_method;
/// input method popup actions
pub mod input_method_popup;
/// layer surface actions
pub mod layer_surface;
/// popup actions
pub mod popup;
/// session locks
pub mod session_lock;
/// virtual keyboard actions
pub mod virtual_keyboard;
/// window actions
pub mod window;

/// Platform specific actions defined for wayland
pub enum Action<T> {
    /// LayerSurface Actions
    LayerSurface(layer_surface::Action<T>),
    /// Window Actions
    Window(window::Action<T>),
    /// popup
    Popup(popup::Action<T>),
    /// data device
    DataDevice(data_device::Action<T>),
    /// activation
    Activation(activation::Action<T>),
    /// session lock
    SessionLock(session_lock::Action<T>),
    /// virtual keyboard
    VirtualKeyboard(virtual_keyboard::Action<T>),
    /// input method
    InputMethod(input_method::Action<T>),
    /// input method popup
    InputMethodPopup(input_method_popup::Action<T>),
}

impl<T> Action<T> {
    /// Maps the output of an [`Action`] using the given function.
    pub fn map<A>(
        self,
        f: impl Fn(T) -> A + 'static + MaybeSend + Sync,
    ) -> Action<A>
    where
        T: 'static,
        A: 'static,
    {
        match self {
            Action::LayerSurface(a) => Action::LayerSurface(a.map(f)),
            Action::Window(a) => Action::Window(a.map(f)),
            Action::Popup(a) => Action::Popup(a.map(f)),
            Action::DataDevice(a) => Action::DataDevice(a.map(f)),
            Action::Activation(a) => Action::Activation(a.map(f)),
            Action::SessionLock(a) => Action::SessionLock(a.map(f)),
            Action::VirtualKeyboard(a) => Action::VirtualKeyboard(a.map(f)),
            Action::InputMethod(a) => Action::InputMethod(a.map(f)),
            Action::InputMethodPopup(a) => Action::InputMethodPopup(a.map(f)),
        }
    }
}

impl<T> Debug for Action<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LayerSurface(arg0) => {
                f.debug_tuple("LayerSurface").field(arg0).finish()
            }
            Self::Window(arg0) => f.debug_tuple("Window").field(arg0).finish(),
            Self::Popup(arg0) => f.debug_tuple("Popup").field(arg0).finish(),
            Self::DataDevice(arg0) => {
                f.debug_tuple("DataDevice").field(arg0).finish()
            }
            Self::Activation(arg0) => {
                f.debug_tuple("Activation").field(arg0).finish()
            }
            Self::SessionLock(arg0) => {
                f.debug_tuple("SessionLock").field(arg0).finish()
            }
            Self::VirtualKeyboard(arg0) => {
                f.debug_tuple("VirtualKeyboard").field(arg0).finish()
            }
            Self::InputMethod(arg0) => {
                f.debug_tuple("InputMethod").field(arg0).finish()
            }
            Self::InputMethodPopup(arg0) => {
                f.debug_tuple("InputMethodPopup").field(arg0).finish()
            }
        }
    }
}
