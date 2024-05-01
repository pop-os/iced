pub mod keyboard;
use std::fmt::Debug;
use std::io::Write;
use std::marker::PhantomData;
use std::os::fd::AsFd;
use std::sync::{Arc, Mutex};

use iced_runtime::command::platform_specific::wayland::input_method_popup::InputMethodPopupSettings;
use iced_runtime::window;
use sctk::reexports::calloop::LoopHandle;
use sctk::reexports::client::globals::{BindError, GlobalList};
use sctk::reexports::client::protocol::wl_keyboard::KeymapFormat;
use sctk::reexports::client::protocol::wl_seat::WlSeat;
use sctk::reexports::client::protocol::wl_surface::WlSurface;
use sctk::reexports::client::Dispatch;
use sctk::reexports::client::{
    delegate_dispatch, Connection, Proxy, QueueHandle,
};
use sctk::seat::keyboard::{KeyEvent, Modifiers};
use wayland_protocols_misc::zwp_input_method_v2::client::zwp_input_method_v2;
use wayland_protocols_misc::zwp_input_method_v2::client::zwp_input_popup_surface_v2;
use wayland_protocols_misc::zwp_input_method_v2::client::{
    zwp_input_method_keyboard_grab_v2::ZwpInputMethodKeyboardGrabV2,
    zwp_input_method_manager_v2::ZwpInputMethodManagerV2,
    zwp_input_method_v2::ZwpInputMethodV2,
    zwp_input_popup_surface_v2::ZwpInputPopupSurfaceV2,
};

use sctk::globals::GlobalData;

use crate::delegate_input_method_keyboard;
use crate::event_loop::state::SctkState;
use crate::sctk_event::{
    InputMethodEventVariant, InputMethodKeyboardEventVariant, SctkEvent,
};

use self::keyboard::{InputMethodKeyboardHandler, RawModifiers};

#[derive(Debug)]
pub struct InputMethodManager<T> {
    manager: ZwpInputMethodManagerV2,
    _phantom: PhantomData<T>,
}

impl<T: 'static> InputMethodManager<T> {
    pub fn new(
        globals: &GlobalList,
        queue_handle: &QueueHandle<SctkState<T>>,
    ) -> Result<Self, BindError> {
        let manager = globals.bind(queue_handle, 1..=1, GlobalData)?;
        Ok(Self {
            manager,
            _phantom: PhantomData,
        })
    }

    pub fn input_method(
        &self,
        seat: &WlSeat,
        queue_handle: &QueueHandle<SctkState<T>>,
        loop_handle: LoopHandle<'static, SctkState<T>>,
    ) -> ZwpInputMethodV2 {
        let mut data = InputMethod {
            inner: Arc::new(Mutex::new(Inner::default())),
        };
        let im =
            self.manager
                .get_input_method(seat, queue_handle, data.clone());
        data.grab_keyboard_with_repeat(
            queue_handle,
            &im,
            None,
            loop_handle,
            Box::new(move |state, _kbd: &ZwpInputMethodKeyboardGrabV2, e| {
                state.sctk_events.push(SctkEvent::InputMethodKeyboardEvent {
                    variant: InputMethodKeyboardEventVariant::Repeat(e),
                })
            }),
        )
        .expect("Input method keyboard grab failed");
        im
    }
}

impl<T: 'static> Dispatch<ZwpInputMethodManagerV2, GlobalData, SctkState<T>>
    for InputMethodManager<T>
{
    fn event(
        _: &mut SctkState<T>,
        _: &ZwpInputMethodManagerV2,
        _: <ZwpInputMethodManagerV2 as Proxy>::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<SctkState<T>>,
    ) {
        // No events.
    }
}

#[derive(Clone, Default)]
struct Inner {
    serial: u32,
}

#[derive(Clone)]
pub struct InputMethod {
    inner: Arc<Mutex<Inner>>,
}

impl<T: 'static> Dispatch<ZwpInputMethodV2, InputMethod, SctkState<T>>
    for InputMethodManager<T>
{
    fn event(
        state: &mut SctkState<T>,
        _: &ZwpInputMethodV2,
        event: <ZwpInputMethodV2 as Proxy>::Event,
        data: &InputMethod,
        _: &Connection,
        _: &QueueHandle<SctkState<T>>,
    ) {
        match event {
            zwp_input_method_v2::Event::Activate => {
                state.sctk_events.push(SctkEvent::InputMethodEvent {
                    variant: InputMethodEventVariant::Activate,
                })
            }
            zwp_input_method_v2::Event::Deactivate => {
                state.sctk_events.push(SctkEvent::InputMethodEvent {
                    variant: InputMethodEventVariant::Deactivate,
                })
            }
            zwp_input_method_v2::Event::SurroundingText {
                text,
                cursor,
                anchor,
            } => state.sctk_events.push(SctkEvent::InputMethodEvent {
                variant: InputMethodEventVariant::SurroundingText {
                    text,
                    cursor,
                    anchor,
                },
            }),
            zwp_input_method_v2::Event::TextChangeCause { cause } => {
                state.sctk_events.push(SctkEvent::InputMethodEvent {
                    variant: InputMethodEventVariant::TextChangeCause(cause),
                })
            }
            zwp_input_method_v2::Event::ContentType { hint, purpose } => {
                state.sctk_events.push(SctkEvent::InputMethodEvent {
                    variant: InputMethodEventVariant::ContentType(
                        hint, purpose,
                    ),
                })
            }
            zwp_input_method_v2::Event::Done => {
                data.inner.lock().unwrap().serial += 1;
                state.sctk_events.push(SctkEvent::InputMethodEvent {
                    variant: InputMethodEventVariant::Done,
                })
            }
            zwp_input_method_v2::Event::Unavailable => {
                panic!("Another input method already present!")
            }
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct InputMethodPopup {
    popup_role: Option<ZwpInputPopupSurfaceV2>,
    pub wl_surface: WlSurface,
    pub wp_viewport: Option<wayland_protocols::wp::viewporter::client::wp_viewport::WpViewport>,
    pub scale_factor: Option<f64>,
    pub wp_fractional_scale: Option<wayland_protocols::wp::fractional_scale::v1::client::wp_fractional_scale_v1::WpFractionalScaleV1>,
}

impl<T: 'static>
    Dispatch<ZwpInputPopupSurfaceV2, InputMethodPopup, SctkState<T>>
    for InputMethodManager<T>
{
    fn event(
        _: &mut SctkState<T>,
        _: &ZwpInputPopupSurfaceV2,
        event: <ZwpInputPopupSurfaceV2 as Proxy>::Event,
        _: &InputMethodPopup,
        _: &Connection,
        _: &QueueHandle<SctkState<T>>,
    ) {
        match event {
            zwp_input_popup_surface_v2::Event::TextInputRectangle {
                x: _,
                y: _,
                width: _,
                height: _,
            } => {
                // just let the compositor decide placement
            }
            _ => unreachable!(),
        }
    }
}

delegate_dispatch!(@<T: 'static> SctkState<T>: [ZwpInputMethodManagerV2: GlobalData] => InputMethodManager<T>);
delegate_dispatch!(@<T: 'static> SctkState<T>: [ZwpInputMethodV2: InputMethod] => InputMethodManager<T>);
delegate_dispatch!(@<T: 'static> SctkState<T>: [ZwpInputPopupSurfaceV2: InputMethodPopup] => InputMethodManager<T>);

impl<T: 'static> InputMethodKeyboardHandler for SctkState<T> {
    fn press_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &ZwpInputMethodKeyboardGrabV2,
        _serial: u32,
        event: KeyEvent,
    ) {
        self.sctk_events.push(SctkEvent::InputMethodKeyboardEvent {
            variant: InputMethodKeyboardEventVariant::Press(event),
        });
    }

    fn release_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &ZwpInputMethodKeyboardGrabV2,
        _serial: u32,
        event: KeyEvent,
    ) {
        self.sctk_events.push(SctkEvent::InputMethodKeyboardEvent {
            variant: InputMethodKeyboardEventVariant::Release(event),
        });
    }

    fn update_modifiers(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &ZwpInputMethodKeyboardGrabV2,
        _serial: u32,
        modifiers: Modifiers,
        raw_modifiers: RawModifiers,
    ) {
        self.sctk_events.push(SctkEvent::InputMethodKeyboardEvent {
            variant: InputMethodKeyboardEventVariant::Modifiers(
                modifiers,
                raw_modifiers,
            ),
        });
    }

    fn update_keymap(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &ZwpInputMethodKeyboardGrabV2,
        keymap: keyboard::Keymap<'_>,
    ) {
        self.seats.iter().for_each(|seat| {
            if let Some(vk) = &seat.virtual_keyboard {
                let mut file =
                    tempfile::tempfile().expect("tempfile cannot be set");
                let keymap = keymap.as_string();
                file.write_all(keymap.as_bytes())
                    .expect("Tempfile not writeable");
                file.flush().expect("Tempfile not flushable");
                vk.keymap(
                    KeymapFormat::XkbV1.into(),
                    file.as_fd(),
                    keymap.len() as u32,
                );
            }
        });
    }
}

delegate_input_method_keyboard!(@<T: 'static> SctkState<T>);

impl<T> SctkState<T>
where
    T: 'static + Debug,
{
    pub fn commit(&self) {
        let seat = self.seats.first().expect("seat not present");
        if let Some(im) = seat.input_method.as_ref() {
            let inner = im.data::<InputMethod>().unwrap().inner.lock().unwrap();
            im.commit(inner.serial);
        }
    }

    pub fn commit_string(&self, string: String) {
        let seat = self.seats.first().expect("seat not present");
        if let Some(im) = seat.input_method.as_ref() {
            im.commit_string(string)
        }
    }

    pub fn set_preedit_string(
        &self,
        string: String,
        cursor_begin: i32,
        cursor_end: i32,
    ) {
        let seat = self.seats.first().expect("seat not present");
        if let Some(im) = seat.input_method.as_ref() {
            im.set_preedit_string(string, cursor_begin, cursor_end)
        }
    }

    pub fn delete_surrounding_text(
        &self,
        before_length: u32,
        after_length: u32,
    ) {
        let seat = self.seats.first().expect("seat not present");
        if let Some(im) = seat.input_method.as_ref() {
            im.delete_surrounding_text(before_length, after_length)
        }
    }

    pub fn get_input_method_popup(
        &mut self,
        settings: InputMethodPopupSettings,
    ) -> (window::Id, WlSurface) {
        let wl_surface =
            self.compositor_state.create_surface(&self.queue_handle);
        wl_surface.commit();
        let wp_viewport = self
            .viewporter_state
            .as_ref()
            .map(|state| state.get_viewport(&wl_surface, &self.queue_handle));
        let wp_fractional_scale = self
            .fractional_scaling_manager
            .as_ref()
            .map(|fsm| fsm.fractional_scaling(&wl_surface, &self.queue_handle));
        self.input_method_popup = Some(InputMethodPopup {
            wl_surface: wl_surface.clone(),
            popup_role: None,
            wp_viewport,
            scale_factor: None,
            wp_fractional_scale,
        });
        (settings.id, wl_surface)
    }

    pub fn show_input_method_popup(&mut self) {
        let seat = self.seats.first().expect("seat not present");
        let popup_state = self
            .input_method_popup
            .as_mut()
            .expect("Input Method popup not present");
        if popup_state.popup_role.is_none() {
            popup_state.popup_role = seat.input_method.as_ref().map(|im| {
                im.get_input_popup_surface(
                    &popup_state.wl_surface,
                    &self.queue_handle,
                    popup_state.clone(),
                )
            });
        }
    }

    pub fn hide_input_method_popup(&mut self) {
        let popup_state = self
            .input_method_popup
            .as_mut()
            .expect("Input Method popup not present");
        if let Some(role) = &popup_state.popup_role {
            role.destroy();
            popup_state.popup_role = None;
        }
    }
}
