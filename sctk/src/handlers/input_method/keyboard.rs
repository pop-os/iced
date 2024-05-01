use std::{
    env, fmt::Debug, marker::PhantomData, num::NonZeroU32,
    os::unix::io::AsRawFd, sync::Mutex,
};
#[cfg(feature = "calloop")]
use std::{sync::Arc, time::Duration};
#[doc(inline)]
pub use xkeysym::Keysym;

#[cfg(feature = "calloop")]
use sctk::reexports::calloop::{
    timer::{TimeoutAction, Timer},
    LoopHandle, RegistrationToken,
};
use sctk::{
    reexports::client::{
        protocol::wl_keyboard, Connection, Dispatch, Proxy, QueueHandle, WEnum,
    },
    seat::keyboard::{KeyEvent, KeyboardError, Modifiers, RepeatInfo, RMLVO},
};

use xkbcommon::xkb;

use wayland_protocols_misc::zwp_input_method_v2::client::{
    zwp_input_method_keyboard_grab_v2::{self, ZwpInputMethodKeyboardGrabV2},
    zwp_input_method_v2::ZwpInputMethodV2,
};

use crate::handlers::input_method::InputMethod;

#[cfg(feature = "calloop")]
pub(crate) struct RepeatedKey {
    pub(crate) key: KeyEvent,
    /// Whether this is the first event of the repeat sequence.
    pub(crate) is_first: bool,
}

#[cfg(feature = "calloop")]
pub type RepeatCallback<T> =
    Box<dyn FnMut(&mut T, &ZwpInputMethodKeyboardGrabV2, KeyEvent) + 'static>;

#[cfg(feature = "calloop")]
pub(crate) struct RepeatData<T> {
    pub(crate) current_repeat: Option<RepeatedKey>,
    pub(crate) repeat_info: RepeatInfo,
    pub(crate) loop_handle: LoopHandle<'static, T>,
    pub(crate) callback: RepeatCallback<T>,
    pub(crate) repeat_token: Option<RegistrationToken>,
}

#[cfg(feature = "calloop")]
impl<T> Drop for RepeatData<T> {
    fn drop(&mut self) {
        if let Some(token) = self.repeat_token.take() {
            self.loop_handle.remove(token);
        }
    }
}

impl InputMethod {
    /// Creates a keyboard from a seat.
    ///
    /// This function returns an [`EventSource`] that indicates when a key press is going to repeat.
    ///
    /// This keyboard implementation uses libxkbcommon for the keymap.
    ///
    /// Typically the compositor will provide a keymap, but you may specify your own keymap using the `rmlvo`
    /// field.
    ///
    /// ## Errors
    ///
    /// This will return [`SeatError::UnsupportedCapability`] if the seat does not support a keyboard.
    ///
    /// [`EventSource`]: calloop::EventSource
    #[cfg(feature = "calloop")]
    pub fn grab_keyboard_with_repeat<D, T>(
        &mut self,
        qh: &QueueHandle<D>,
        input_method: &ZwpInputMethodV2,
        rmlvo: Option<RMLVO>,
        loop_handle: LoopHandle<'static, T>,
        callback: RepeatCallback<T>,
    ) -> Result<ZwpInputMethodKeyboardGrabV2, KeyboardError>
    where
        D: Dispatch<ZwpInputMethodKeyboardGrabV2, InputMethodKeyboardData<T>>
            + InputMethodKeyboardHandler
            + 'static,
        T: 'static,
    {
        let udata = match rmlvo {
            Some(rmlvo) => InputMethodKeyboardData::from_rmlvo(rmlvo)?,
            None => InputMethodKeyboardData::new(),
        };

        Ok(self.grab_keyboard_with_repeat_with_data(
            qh,
            input_method,
            udata,
            loop_handle,
            callback,
        ))
    }

    /// Creates a keyboard from a seat.
    ///
    /// This function returns an [`EventSource`] that indicates when a key press is going to repeat.
    ///
    /// This keyboard implementation uses libxkbcommon for the keymap.
    ///
    /// Typically the compositor will provide a keymap, but you may specify your own keymap using the `rmlvo`
    /// field.
    ///
    /// ## Errors
    ///
    /// This will return [`SeatError::UnsupportedCapability`] if the seat does not support a keyboard.
    ///
    /// [`EventSource`]: calloop::EventSource
    #[cfg(feature = "calloop")]
    pub fn grab_keyboard_with_repeat_with_data<D, U>(
        &mut self,
        qh: &QueueHandle<D>,
        input_method: &ZwpInputMethodV2,
        mut udata: U,
        loop_handle: LoopHandle<
            'static,
            <U as InputMethodKeyboardDataExt>::State,
        >,
        callback: RepeatCallback<<U as InputMethodKeyboardDataExt>::State>,
    ) -> ZwpInputMethodKeyboardGrabV2
    where
        D: Dispatch<ZwpInputMethodKeyboardGrabV2, U>
            + InputMethodKeyboardHandler
            + 'static,
        U: InputMethodKeyboardDataExt + 'static,
    {
        let kbd_data = udata.keyboard_data_mut();
        kbd_data.repeat_data.lock().unwrap().replace(RepeatData {
            current_repeat: None,
            repeat_info: RepeatInfo::Disable,
            loop_handle: loop_handle.clone(),
            callback,
            repeat_token: None,
        });
        kbd_data.init_compose();

        input_method.grab_keyboard(qh, udata)
    }

    /// Creates a keyboard from a seat.
    ///
    /// This keyboard implementation uses libxkbcommon for the keymap.
    ///
    /// Typically the compositor will provide a keymap, but you may specify your own keymap using the `rmlvo`
    /// field.
    pub fn grab_keyboard<D, T: 'static>(
        &mut self,
        qh: &QueueHandle<D>,
        input_method: &ZwpInputMethodV2,
        rmlvo: Option<RMLVO>,
    ) -> Result<ZwpInputMethodKeyboardGrabV2, KeyboardError>
    where
        D: Dispatch<ZwpInputMethodKeyboardGrabV2, InputMethodKeyboardData<T>>
            + InputMethodKeyboardHandler
            + 'static,
    {
        let udata = match rmlvo {
            Some(rmlvo) => InputMethodKeyboardData::from_rmlvo(rmlvo)?,
            None => InputMethodKeyboardData::new(),
        };

        Ok(self.grab_keyboard_with_data(qh, input_method, udata))
    }

    /// Creates a keyboard from a seat.
    ///
    /// This keyboard implementation uses libxkbcommon for the keymap.
    ///
    /// Typically the compositor will provide a keymap, but you may specify your own keymap using the `rmlvo`
    /// field.
    pub fn grab_keyboard_with_data<D, U>(
        &mut self,
        qh: &QueueHandle<D>,
        input_method: &ZwpInputMethodV2,
        udata: U,
    ) -> ZwpInputMethodKeyboardGrabV2
    where
        D: Dispatch<ZwpInputMethodKeyboardGrabV2, U>
            + InputMethodKeyboardHandler
            + 'static,
        U: InputMethodKeyboardDataExt + 'static,
    {
        input_method.grab_keyboard(qh, udata)
    }
}

/// Wrapper around a libxkbcommon keymap
#[allow(missing_debug_implementations)]
pub struct Keymap<'a>(&'a xkb::Keymap);

impl<'a> Keymap<'a> {
    /// Get keymap as string in text format. The keymap should always be valid.
    pub fn as_string(&self) -> String {
        self.0.get_as_string(xkb::KEYMAP_FORMAT_TEXT_V1)
    }
}

/// Handler trait for keyboard input.
///
/// The functions defined in this trait are called as keyboard events are received from the compositor.
pub trait InputMethodKeyboardHandler: Sized {
    /// A key has been pressed on the keyboard.
    ///
    /// The key will repeat if there is no other press event afterwards or the key is released.
    fn press_key(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        keyboard: &ZwpInputMethodKeyboardGrabV2,
        serial: u32,
        event: KeyEvent,
    );

    /// A key has been released.
    ///
    /// This stops the key from being repeated if the key is the last key which was pressed.
    fn release_key(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        keyboard: &ZwpInputMethodKeyboardGrabV2,
        serial: u32,
        event: KeyEvent,
    );

    /// Keyboard modifiers have been updated.
    ///
    /// This happens when one of the modifier keys, such as "Shift", "Control" or "Alt" is pressed or
    /// released.
    fn update_modifiers(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        keyboard: &ZwpInputMethodKeyboardGrabV2,
        serial: u32,
        modifiers: Modifiers,
        raw_modifiers: RawModifiers,
    );

    /// The keyboard has updated the rate and delay between repeating key inputs.
    ///
    /// This function does nothing by default but is provided if a repeat mechanism outside of calloop is\
    /// used.
    fn update_repeat_info(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &ZwpInputMethodKeyboardGrabV2,
        _info: RepeatInfo,
    ) {
    }

    /// Keyboard keymap has been updated.
    ///
    /// `keymap.as_string()` can be used get the keymap as a string. It cannot be exposed directly
    /// as an `xkbcommon::xkb::Keymap` due to the fact xkbcommon uses non-thread-safe reference
    /// counting. But can be used to create an independent `Keymap`.
    ///
    /// This is called after the default handler for keymap changes and does nothing by default.
    fn update_keymap(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        keyboard: &ZwpInputMethodKeyboardGrabV2,
        keymap: Keymap<'_>,
    );
}

pub struct InputMethodKeyboardData<T> {
    xkb_context: Mutex<xkb::Context>,
    /// If the user manually specified the RMLVO to use.
    user_specified_rmlvo: bool,
    xkb_state: Mutex<Option<xkb::State>>,
    xkb_compose: Mutex<Option<xkb::compose::State>>,
    #[cfg(feature = "calloop")]
    repeat_data: Arc<Mutex<Option<RepeatData<T>>>>,
    _phantom_data: PhantomData<T>,
}

impl<T> Debug for InputMethodKeyboardData<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeyboardData").finish_non_exhaustive()
    }
}

#[macro_export]
macro_rules! delegate_input_method_keyboard {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        sctk::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
            [
                wayland_protocols_misc::zwp_input_method_v2::client::zwp_input_method_keyboard_grab_v2::ZwpInputMethodKeyboardGrabV2:
                    $crate::handlers::input_method::keyboard::InputMethodKeyboardData<$ty>
            ] => $crate::handlers::input_method::InputMethod
        );
    };
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty, keyboard: [$($udata:ty),* $(,)?]) => {
        sctk::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
            [
                $(
                    wayland_protocols_misc::zwp_input_method_v2::client::zwp_input_method_keyboard_grab_v2::ZwpInputMethodKeyboardGrabV2: $udata,
                )*
            ] => $crate::handlers::input_method::InputMethod
        );
    };
}

// SAFETY: The state does not share state with any other rust types.
unsafe impl<T> Send for InputMethodKeyboardData<T> {}
// SAFETY: The state is guarded by a mutex since libxkbcommon has no internal synchronization.
unsafe impl<T> Sync for InputMethodKeyboardData<T> {}

impl<T> InputMethodKeyboardData<T> {
    pub fn new() -> Self {
        let xkb_context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
        let udata = InputMethodKeyboardData {
            xkb_context: Mutex::new(xkb_context),
            xkb_state: Mutex::new(None),
            user_specified_rmlvo: false,
            xkb_compose: Mutex::new(None),
            #[cfg(feature = "calloop")]
            repeat_data: Arc::new(Mutex::new(None)),
            _phantom_data: PhantomData,
        };

        udata.init_compose();

        udata
    }

    pub fn from_rmlvo(rmlvo: RMLVO) -> Result<Self, KeyboardError> {
        let xkb_context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
        let keymap = xkb::Keymap::new_from_names(
            &xkb_context,
            &rmlvo.rules.unwrap_or_default(),
            &rmlvo.model.unwrap_or_default(),
            &rmlvo.layout.unwrap_or_default(),
            &rmlvo.variant.unwrap_or_default(),
            rmlvo.options,
            xkb::COMPILE_NO_FLAGS,
        );

        if keymap.is_none() {
            return Err(KeyboardError::InvalidKeymap);
        }

        let xkb_state = Some(xkb::State::new(&keymap.unwrap()));

        let udata = InputMethodKeyboardData {
            xkb_context: Mutex::new(xkb_context),
            xkb_state: Mutex::new(xkb_state),
            user_specified_rmlvo: true,
            xkb_compose: Mutex::new(None),
            #[cfg(feature = "calloop")]
            repeat_data: Arc::new(Mutex::new(None)),
            _phantom_data: PhantomData,
        };

        udata.init_compose();

        Ok(udata)
    }

    fn init_compose(&self) {
        let xkb_context = self.xkb_context.lock().unwrap();

        if let Some(locale) = env::var_os("LC_ALL")
            .and_then(|v| if v.is_empty() { None } else { Some(v) })
            .or_else(|| env::var_os("LC_CTYPE"))
            .and_then(|v| if v.is_empty() { None } else { Some(v) })
            .or_else(|| env::var_os("LANG"))
            .and_then(|v| if v.is_empty() { None } else { Some(v) })
            .unwrap_or_else(|| "C".into())
            .to_str()
        {
            // TODO: Pending new release of xkbcommon to use new_from_locale with OsStr
            if let Ok(table) = xkb::compose::Table::new_from_locale(
                &xkb_context,
                locale.as_ref(),
                xkb::compose::COMPILE_NO_FLAGS,
            ) {
                let compose_state = xkb::compose::State::new(
                    &table,
                    xkb::compose::COMPILE_NO_FLAGS,
                );
                *self.xkb_compose.lock().unwrap() = Some(compose_state);
            }
        }
    }

    fn update_modifiers(&self) -> Modifiers {
        let guard = self.xkb_state.lock().unwrap();
        let state = guard.as_ref().unwrap();

        Modifiers {
            ctrl: state.mod_name_is_active(
                xkb::MOD_NAME_CTRL,
                xkb::STATE_MODS_EFFECTIVE,
            ),
            alt: state.mod_name_is_active(
                xkb::MOD_NAME_ALT,
                xkb::STATE_MODS_EFFECTIVE,
            ),
            shift: state.mod_name_is_active(
                xkb::MOD_NAME_SHIFT,
                xkb::STATE_MODS_EFFECTIVE,
            ),
            caps_lock: state.mod_name_is_active(
                xkb::MOD_NAME_CAPS,
                xkb::STATE_MODS_EFFECTIVE,
            ),
            logo: state.mod_name_is_active(
                xkb::MOD_NAME_LOGO,
                xkb::STATE_MODS_EFFECTIVE,
            ),
            num_lock: state.mod_name_is_active(
                xkb::MOD_NAME_NUM,
                xkb::STATE_MODS_EFFECTIVE,
            ),
        }
    }
}

pub trait InputMethodKeyboardDataExt: Send + Sync {
    type State: 'static;
    fn keyboard_data(&self) -> &InputMethodKeyboardData<Self::State>;
    fn keyboard_data_mut(
        &mut self,
    ) -> &mut InputMethodKeyboardData<Self::State>;
}

impl<T: 'static> InputMethodKeyboardDataExt for InputMethodKeyboardData<T> {
    /// The type of the user defined state
    type State = T;
    fn keyboard_data(&self) -> &InputMethodKeyboardData<T> {
        self
    }

    fn keyboard_data_mut(&mut self) -> &mut InputMethodKeyboardData<T> {
        self
    }
}

impl<D, U> Dispatch<ZwpInputMethodKeyboardGrabV2, U, D> for InputMethod
where
    D: Dispatch<ZwpInputMethodKeyboardGrabV2, U> + InputMethodKeyboardHandler,
    U: InputMethodKeyboardDataExt,
{
    fn event(
        data: &mut D,
        keyboard: &ZwpInputMethodKeyboardGrabV2,
        event: <ZwpInputMethodKeyboardGrabV2 as Proxy>::Event,
        udata: &U,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        let udata = udata.keyboard_data();
        match event {
            zwp_input_method_keyboard_grab_v2::Event::Keymap {
                format,
                fd,
                size,
            } => {
                match format {
                    WEnum::Value(format) => match format {
                        wl_keyboard::KeymapFormat::NoKeymap => {
                            log::warn!(target: "sctk", "non-xkb compatible keymap");
                        }

                        wl_keyboard::KeymapFormat::XkbV1 => {
                            if udata.user_specified_rmlvo {
                                // state is locked, ignore keymap updates
                                return;
                            }

                            let context = udata.xkb_context.lock().unwrap();

                            // SAFETY:
                            // - wayland-client guarantees we have received a valid file descriptor.
                            #[allow(unused_unsafe)]
                            // Upstream release will change this
                            match unsafe {
                                xkb::Keymap::new_from_fd(
                                    &context,
                                    fd,
                                    size as usize,
                                    xkb::KEYMAP_FORMAT_TEXT_V1,
                                    xkb::COMPILE_NO_FLAGS,
                                )
                            } {
                                Ok(Some(keymap)) => {
                                    let state = xkb::State::new(&keymap);
                                    {
                                        let mut state_guard =
                                            udata.xkb_state.lock().unwrap();
                                        *state_guard = Some(state);
                                    }
                                    data.update_keymap(
                                        conn,
                                        qh,
                                        keyboard,
                                        Keymap(&keymap),
                                    );
                                }

                                Ok(None) => {
                                    log::error!(target: "sctk", "invalid keymap");
                                }

                                Err(err) => {
                                    log::error!(target: "sctk", "{}", err);
                                }
                            }
                        }

                        _ => unreachable!(),
                    },

                    WEnum::Unknown(value) => {
                        log::warn!(target: "sctk", "unknown keymap format 0x{:x}", value)
                    }
                }
            }

            zwp_input_method_keyboard_grab_v2::Event::Key {
                serial,
                time,
                key,
                state,
            } => match state {
                WEnum::Value(state) => {
                    let state_guard = udata.xkb_state.lock().unwrap();

                    if let Some(guard) = state_guard.as_ref() {
                        // We must add 8 to the keycode for any functions we pass the raw keycode into per
                        // wl_keyboard protocol.
                        let keysym = guard.key_get_one_sym((key + 8).into());
                        let utf8 = if state == wl_keyboard::KeyState::Pressed {
                            let mut compose = udata.xkb_compose.lock().unwrap();

                            match compose.as_mut() {
                                Some(compose) => match compose.feed(keysym) {
                                    xkb::FeedResult::Ignored => None,
                                    xkb::FeedResult::Accepted => match compose
                                        .status()
                                    {
                                        xkb::Status::Composed => compose.utf8(),
                                        xkb::Status::Nothing => Some(
                                            guard
                                                .key_get_utf8((key + 8).into()),
                                        ),
                                        _ => None,
                                    },
                                },

                                // No compose
                                None => {
                                    Some(guard.key_get_utf8((key + 8).into()))
                                }
                            }
                        } else {
                            None
                        };

                        // Drop guard before calling user code.
                        drop(state_guard);

                        let event = KeyEvent {
                            time,
                            raw_code: key,
                            keysym: keysym.into(),
                            utf8,
                        };

                        match state {
                            wl_keyboard::KeyState::Released => {
                                #[cfg(feature = "calloop")]
                                {
                                    if let Some(repeat_data) = udata
                                        .repeat_data
                                        .lock()
                                        .unwrap()
                                        .as_mut()
                                    {
                                        if Some(event.raw_code)
                                            == repeat_data
                                                .current_repeat
                                                .as_ref()
                                                .map(|r| r.key.raw_code)
                                        {
                                            repeat_data.current_repeat = None;
                                        }
                                    }
                                }
                                data.release_key(
                                    conn, qh, keyboard, serial, event,
                                );
                            }

                            wl_keyboard::KeyState::Pressed => {
                                #[cfg(feature = "calloop")]
                                {
                                    if let Some(repeat_data) = udata
                                        .repeat_data
                                        .lock()
                                        .unwrap()
                                        .as_mut()
                                    {
                                        let loop_handle =
                                            &mut repeat_data.loop_handle;
                                        let state_guard =
                                            udata.xkb_state.lock().unwrap();
                                        let key_repeats = state_guard
                                            .as_ref()
                                            .map(|guard| {
                                                guard.get_keymap().key_repeats(
                                                    (event.raw_code + 8).into(),
                                                )
                                            })
                                            .unwrap_or_default();
                                        if key_repeats {
                                            // Cancel the previous timer / repeat.
                                            if let Some(token) =
                                                repeat_data.repeat_token.take()
                                            {
                                                loop_handle.remove(token);
                                            }

                                            // Update the current repeat key.
                                            repeat_data.current_repeat.replace(
                                                RepeatedKey {
                                                    key: event.clone(),
                                                    is_first: true,
                                                },
                                            );

                                            let (delay, rate) =
                                                match repeat_data.repeat_info {
                                                    RepeatInfo::Disable => {
                                                        return
                                                    }
                                                    RepeatInfo::Repeat {
                                                        delay,
                                                        rate,
                                                    } => (delay, rate),
                                                };
                                            let gap = Duration::from_micros(
                                                1_000_000 / rate.get() as u64,
                                            );
                                            let timer = Timer::from_duration(
                                                Duration::from_millis(
                                                    delay as u64,
                                                ),
                                            );
                                            let repeat_data2 =
                                                udata.repeat_data.clone();

                                            // Start the timer.
                                            let kbd = keyboard.clone();
                                            if let Ok(token) = loop_handle.insert_source(
                                                timer,
                                                move |_, _, state| {
                                                    let mut repeat_data =
                                                        repeat_data2.lock().unwrap();
                                                    let repeat_data = match repeat_data.as_mut() {
                                                        Some(repeat_data) => repeat_data,
                                                        None => return TimeoutAction::Drop,
                                                    };

                                                    let callback = &mut repeat_data.callback;
                                                    let key = &mut repeat_data.current_repeat;
                                                    if key.is_none() {
                                                        return TimeoutAction::Drop;
                                                    }
                                                    let key = key.as_mut().unwrap();
                                                    key.key.time += if key.is_first {
                                                        key.is_first = false;
                                                        delay
                                                    } else {
                                                        gap.as_millis() as u32
                                                    };
                                                    callback(state, &kbd, key.key.clone());
                                                    TimeoutAction::ToDuration(gap)
                                                },
                                            ) {
                                                repeat_data.repeat_token = Some(token);
                                            }
                                        }
                                    }
                                }
                                data.press_key(
                                    conn, qh, keyboard, serial, event,
                                );
                            }

                            _ => unreachable!(),
                        }
                    };
                }

                WEnum::Unknown(unknown) => {
                    log::warn!(target: "sctk", "{}: compositor sends invalid key state: {:x}", keyboard.id(), unknown);
                }
            },

            zwp_input_method_keyboard_grab_v2::Event::Modifiers {
                serial,
                mods_depressed,
                mods_latched,
                mods_locked,
                group,
            } => {
                let raw_modifiers = RawModifiers {
                    mods_depressed,
                    mods_latched,
                    mods_locked,
                    group,
                };
                let mut guard = udata.xkb_state.lock().unwrap();

                let state = match guard.as_mut() {
                    Some(state) => state,
                    None => return,
                };

                // Apply the new xkb state with the new modifiers.
                let _ = state.update_mask(
                    mods_depressed,
                    mods_latched,
                    mods_locked,
                    0,
                    0,
                    group,
                );

                // Update the currently repeating key if any.
                #[cfg(feature = "calloop")]
                if let Some(repeat_data) =
                    udata.repeat_data.lock().unwrap().as_mut()
                {
                    if let Some(mut event) = repeat_data.current_repeat.take() {
                        // Apply new modifiers to get new utf8.
                        event.key.utf8 = {
                            let mut compose = udata.xkb_compose.lock().unwrap();

                            match compose.as_mut() {
                                Some(compose) => match compose
                                    .feed(event.key.keysym.into())
                                {
                                    xkb::FeedResult::Ignored => None,
                                    xkb::FeedResult::Accepted => match compose
                                        .status()
                                    {
                                        xkb::Status::Composed => compose.utf8(),
                                        xkb::Status::Nothing => {
                                            Some(state.key_get_utf8(
                                                (event.key.raw_code + 8).into(),
                                            ))
                                        }
                                        _ => None,
                                    },
                                },

                                // No compose.
                                None => Some(state.key_get_utf8(
                                    (event.key.raw_code + 8).into(),
                                )),
                            }
                        };

                        // Update the stored event.
                        repeat_data.current_repeat = Some(event);
                    }
                }

                // Drop guard before calling user code.
                drop(guard);

                // Always issue the modifiers update for the user.
                let modifiers = udata.update_modifiers();
                data.update_modifiers(
                    conn,
                    qh,
                    keyboard,
                    serial,
                    modifiers,
                    raw_modifiers,
                );
            }

            zwp_input_method_keyboard_grab_v2::Event::RepeatInfo {
                rate,
                delay,
            } => {
                let info = if rate != 0 {
                    RepeatInfo::Repeat {
                        rate: NonZeroU32::new(rate as u32).unwrap(),
                        delay: delay as u32,
                    }
                } else {
                    RepeatInfo::Disable
                };

                #[cfg(feature = "calloop")]
                {
                    if let Some(repeat_data) =
                        udata.repeat_data.lock().unwrap().as_mut()
                    {
                        repeat_data.repeat_info = info;
                    }
                }
                data.update_repeat_info(conn, qh, keyboard, info);
            }

            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct RawModifiers {
    pub mods_depressed: u32,
    pub mods_latched: u32,
    pub mods_locked: u32,
    pub group: u32,
}

impl From<iced_futures::core::event::wayland::RawModifiers> for RawModifiers {
    fn from(value: iced_futures::core::event::wayland::RawModifiers) -> Self {
        RawModifiers {
            mods_depressed: value.mods_depressed,
            mods_latched: value.mods_latched,
            mods_locked: value.mods_locked,
            group: value.group,
        }
    }
}

impl From<RawModifiers> for iced_futures::core::event::wayland::RawModifiers {
    fn from(value: RawModifiers) -> Self {
        iced_futures::core::event::wayland::RawModifiers {
            mods_depressed: value.mods_depressed,
            mods_latched: value.mods_latched,
            mods_locked: value.mods_locked,
            group: value.group,
        }
    }
}
