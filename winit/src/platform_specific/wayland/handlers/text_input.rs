use cctk::sctk::globals::GlobalData;
use cctk::sctk::reexports::client::{Connection, Proxy, QueueHandle};

use cctk::sctk::reexports::client::delegate_dispatch;
use cctk::sctk::reexports::client::Dispatch;
use cctk::sctk::reexports::protocols::wp::text_input::zv3::client::zwp_text_input_manager_v3::ZwpTextInputManagerV3;
use cctk::sctk::reexports::protocols::wp::text_input::zv3::client::zwp_text_input_v3::Event as TextInputEvent;
use cctk::sctk::reexports::protocols::wp::text_input::zv3::client::zwp_text_input_v3::ZwpTextInputV3;
use cctk::sctk::registry::RegistryState;
use wayland_client::protocol::wl_seat::WlSeat;
use winit::event::{Ime, WindowEvent};
use winit::window::WindowId;

use crate::event_loop::state::SctkState;
use crate::sctk_event::SctkEvent;

pub struct Preedit {
    text: String,
    cursor_range: Option<(usize, usize)>,
}

pub struct TextInputManager {
    manager: ZwpTextInputManagerV3,
}

impl TextInputManager {
    pub fn try_new<D>(
        registry: &RegistryState,
        qh: &QueueHandle<D>,
    ) -> Option<Self>
    where
        D: Dispatch<ZwpTextInputManagerV3, GlobalData> + 'static,
    {
        let manager = registry
            .bind_one::<ZwpTextInputManagerV3, _, _>(qh, 1..=1, GlobalData)
            .ok()?;
        Some(Self { manager })
    }

    pub fn get_text_input(
        &self,
        seat: &WlSeat,
        qh: &QueueHandle<SctkState>,
    ) -> ZwpTextInputV3 {
        self.manager.get_text_input(&seat, &qh, ())
    }
}

impl Dispatch<ZwpTextInputManagerV3, GlobalData, SctkState>
    for TextInputManager
{
    fn event(
        _state: &mut SctkState,
        _proxy: &ZwpTextInputManagerV3,
        _event: <ZwpTextInputManagerV3 as Proxy>::Event,
        _data: &GlobalData,
        _conn: &Connection,
        _qhandle: &QueueHandle<SctkState>,
    ) {
    }
}

impl Dispatch<ZwpTextInputV3, (), SctkState> for TextInputManager {
    fn event(
        state: &mut SctkState,
        _text_input: &ZwpTextInputV3,
        event: <ZwpTextInputV3 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<SctkState>,
    ) {
        let kbd_focus =
            match state.seats.iter_mut().find_map(|s| s.kbd_focus.clone()) {
                Some(surface) => surface,
                None => return,
            };
        match event {
            TextInputEvent::PreeditString {
                text,
                cursor_begin,
                cursor_end,
            } => {
                let text = text.unwrap_or_default();
                let cursor_begin = usize::try_from(cursor_begin)
                    .ok()
                    .and_then(|idx| text.is_char_boundary(idx).then_some(idx));
                let cursor_end = usize::try_from(cursor_end)
                    .ok()
                    .and_then(|idx| text.is_char_boundary(idx).then_some(idx));
                let cursor_range =
                    cursor_begin.map(|b| (b, cursor_end.unwrap_or(b)));
                state.preedit = Some(Preedit { text, cursor_range });
            }
            TextInputEvent::CommitString { text } => {
                state.preedit = None;
                state.pending_commit = text;
            }
            TextInputEvent::Done { .. } => {
                let id = WindowId::from(kbd_focus.id().as_ptr() as u64);
                state.sctk_events.push(SctkEvent::Winit(
                    id,
                    WindowEvent::Ime(Ime::Preedit(String::new(), None)),
                ));

                // Commit string
                if let Some(text) = state.pending_commit.take() {
                    state.sctk_events.push(SctkEvent::Winit(
                        id,
                        WindowEvent::Ime(Ime::Commit(text)),
                    ));
                }

                // Update preedit string
                if let Some(preedit) = state.preedit.take() {
                    state.sctk_events.push(SctkEvent::Winit(
                        id,
                        WindowEvent::Ime(Ime::Preedit(
                            preedit.text,
                            preedit.cursor_range,
                        )),
                    ));
                }
            }
            _ => {}
        }
    }
}

delegate_dispatch!(SctkState: [ZwpTextInputManagerV3: GlobalData] => TextInputManager);
delegate_dispatch!(SctkState: [ZwpTextInputV3: ()] => TextInputManager);
