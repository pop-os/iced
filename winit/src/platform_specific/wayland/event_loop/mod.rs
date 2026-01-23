pub mod control_flow;
pub mod proxy;
pub mod state;

#[cfg(feature = "a11y")]
use crate::platform_specific::SurfaceIdWrapper;
use crate::{
    Control,
    futures::futures::channel::mpsc,
    handlers::{overlap::OverlapNotifyV1, text_input::TextInputManager},
    platform_specific::wayland::{
        handlers::{
            wp_fractional_scaling::FractionalScalingManager,
            wp_viewporter::ViewporterState,
        },
        sctk_event::SctkEvent,
    },
    subsurface_widget::SubsurfaceState,
};

use cctk::{
    cosmic_protocols::corner_radius::v1::client::cosmic_corner_radius_manager_v1::CosmicCornerRadiusManagerV1, sctk::reexports::calloop_wayland_source::WaylandSource, toplevel_info::ToplevelInfoState
};
use cctk::{
    sctk::{
        activation::ActivationState,
        compositor::CompositorState,
        globals::GlobalData,
        output::OutputState,
        reexports::{
            calloop::{self, EventLoop},
            client::{
                ConnectError, Connection, Proxy, globals::registry_queue_init,
            },
        },
        registry::RegistryState,
        seat::SeatState,
        session_lock::SessionLockState,
        shell::{WaylandSurface, wlr_layer::LayerShell, xdg::XdgShell},
        shm::Shm,
    },
    toplevel_management::ToplevelManagerState,
};
use raw_window_handle::HasDisplayHandle;
use state::{FrameStatus, SctkWindow, send_event};
#[cfg(feature = "a11y")]
use std::sync::{Arc, Mutex};
use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
};
use log::error;
use wayland_backend::client::Backend;
use wayland_client::globals::GlobalError;
use wayland_protocols::wp::{keyboard_shortcuts_inhibit::zv1::client::zwp_keyboard_shortcuts_inhibit_manager_v1, text_input::zv3::client::zwp_text_input_v3::{ContentHint, ContentPurpose}};
use winit::{dpi::LogicalSize, event_loop::OwnedDisplayHandle, window::ImePurpose};

use self::state::SctkState;

#[derive(Debug, Default, Clone, Copy)]
pub struct Features {
    // TODO
}

pub struct SctkEventLoop {
    pub(crate) event_loop: EventLoop<'static, SctkState>,
    pub(crate) _features: Features,
    pub(crate) state: SctkState,
}

pub enum Error {
    Connect(ConnectError),
    Calloop(calloop::Error),
    Global(GlobalError),
    NoDisplayHandle,
    NoWaylandDisplay,
}

impl SctkEventLoop {
    pub(crate) fn new(
        winit_event_sender: mpsc::UnboundedSender<Control>,
        proxy: winit::event_loop::EventLoopProxy,
        display: OwnedDisplayHandle,
    ) -> Result<
        calloop::channel::Sender<super::Action>,
        Box<dyn std::any::Any + std::marker::Send>,
    > {
        let Ok(dh) = display.display_handle() else {
            log::error!("Failed to get display handle");
            return Err(Box::new(Error::NoDisplayHandle));
        };
        let raw_window_handle::RawDisplayHandle::Wayland(wayland_dh) =
            dh.as_raw()
        else {
            log::error!("Display handle is not Wayland");
            return Err(Box::new(Error::NoWaylandDisplay));
        };

        let backend = unsafe {
            Backend::from_foreign_display(wayland_dh.display.as_ptr().cast())
        };
        let connection = Connection::from_backend(backend);

        let (action_tx, action_rx) = calloop::channel::channel();
        let res: std::thread::JoinHandle<Result<(), Error>> =
            std::thread::spawn(move || {
                let _display = connection.display();
                let (globals, event_queue) =
                    registry_queue_init(&connection).map_err(Error::Global)?;
                let event_loop = calloop::EventLoop::<SctkState>::try_new()
                    .map_err(Error::Calloop)?;
                let loop_handle = event_loop.handle();

                let qh = event_queue.handle();
                let registry_state = RegistryState::new(&globals);

                _ = loop_handle
                .insert_source(action_rx, |event, _, state| {
                    match event {
                        calloop::channel::Event::Msg(e) => match e {
                            crate::platform_specific::Action::Action(a) => {
                                if let Err(err) = state.handle_action(a) {
                                    log::warn!("{err:?}");
                                }
                            }
                            crate::platform_specific::Action::ResizeWindow(id) => {
                                if let Some((_, v)) = state.windows.iter()
                                    .find(|w| w.id == id)
                                    .map(|w| w.corner_radius.as_ref())
                                    .unwrap_or_default() {
                                    _ = state.handle_action(iced_runtime::platform_specific::wayland::Action::RoundedCorners(id, *v));
                                }
                            }
                            crate::platform_specific::Action::TrackWindow(
                                window,
                                id,
                            ) => {
                                state.windows.push(SctkWindow { window, id, corner_radius: Default::default() });
                                if let Some(v) = state.pending_corner_radius.remove(&id) {
                                    _ = state.handle_action(iced_runtime::platform_specific::wayland::Action::RoundedCorners(id, Some(v)));
                                }
                            }
                            crate::platform_specific::Action::RemoveWindow(
                                id,
                            ) => {
                                // TODO clean up popups matching the window.
                                if let Some(pos) = state
                                    .windows
                                    .iter()
                                    .position(|window| id == window.id)
                                {
                                    let w = state.windows.remove(pos);
                                    for subsurface_id in state
                                        .subsurfaces
                                        .iter()
                                        .enumerate()
                                        .filter_map(|(i, s)| {
                                            (winit::window::WindowId::from_raw(
                                                s.instance.parent.id().as_ptr()
                                                    as usize,
                                            ) == w.window.id())
                                            .then_some(i)
                                        })
                                        .collect::<Vec<_>>()
                                    {
                                        let s = state
                                            .subsurfaces
                                            .remove(subsurface_id);
                                        crate::subsurface_widget::remove_iced_subsurface(
                                            &s.instance.wl_surface,
                                        );
                                        send_event(&state.events_sender, &state.proxy,
                                            SctkEvent::SubsurfaceEvent( crate::sctk_event::SubsurfaceEventVariant::Destroyed(s.instance) )
                                        );
                                    }
                                }
                            }
                            crate::platform_specific::Action::SetCursor(
                                icon,
                            ) => {
                                if let Some(seat) = state.seats.get_mut(0) {
                                    seat.icon = Some(icon);
                                    seat.set_cursor(&state.connection, icon);
                                }
                            }
                            crate::platform_specific::Action::RequestRedraw(
                                id,
                            ) => {
                                let e = state
                                    .frame_status
                                    .entry(id)
                                    .or_insert(FrameStatus::RequestedRedraw);
                                if matches!(e, FrameStatus::Received) {
                                    *e = FrameStatus::Ready;
                                }
                            }
                            crate::platform_specific::Action::Dropped(id) => {
                                _ = state.destroyed.remove(&id.inner());
                            }
                            crate::platform_specific::Action::SetImeAllowed(allowed) => {
                                if let Some(text_input) = state.text_input.as_ref() {
                                    if allowed {
                                        text_input.enable();
                                    } else {
                                        text_input.disable();
                                    }
                                    text_input.commit();
                                }
                            }
                            crate::platform_specific::Action::SetImeCursorArea(x, y, width, height) => {
                                if let Some(text_input) = state.text_input.as_ref() {
                                    text_input.set_cursor_rectangle(x, y, width, height);
                                    text_input.commit();
                                }
                            }
                            crate::platform_specific::Action::SetImePurpose(purpose) => {
                                if let Some(text_input) = state.text_input.as_ref() {
                                    let (hint, purpose) = match purpose {
                                        ImePurpose::Password => (ContentHint::SensitiveData, ContentPurpose::Password),
                                        ImePurpose::Terminal => (ContentHint::None, ContentPurpose::Terminal),
                                        _ => (ContentHint::None, ContentPurpose::Normal),
                                    };
                                    text_input.set_content_type(hint, purpose);
                                    text_input.commit();
                                }
                            }
                            crate::platform_specific::Action::SubsurfaceResize(id, size) => {
                                // reposition the surface
                                if let Some(pos) = state
                                    .subsurfaces
                                    .iter()
                                    .position(|window| id == window.id)
                                {
                                    let subsurface = &mut state.subsurfaces[pos];
                                    let settings = &subsurface.settings;
                                    let mut loc = settings.loc;
                                    let guard = subsurface.common.lock().unwrap();
                                    let size: LogicalSize<f32> = size.to_logical(guard.fractional_scale.unwrap_or(1.));
                                    let half_w = size.width / 2.;
                                    let half_h = size.height / 2.;
                                    match settings.gravity {
                                        wayland_protocols::xdg::shell::client::xdg_positioner::Gravity::None => {
                                            // center on
                                            loc.x -= half_w;
                                            loc.y -= half_h;
                                        },
                                        wayland_protocols::xdg::shell::client::xdg_positioner::Gravity::Top => {
                                            loc.x -= half_w;
                                            loc.y -= size.height;
                                        },
                                        wayland_protocols::xdg::shell::client::xdg_positioner::Gravity::Bottom => {
                                            loc.x -= half_w;
                                        },
                                        wayland_protocols::xdg::shell::client::xdg_positioner::Gravity::Left => {
                                            loc.y -= half_h;
                                            loc.x -= size.width;
                                        },
                                        wayland_protocols::xdg::shell::client::xdg_positioner::Gravity::Right => {
                                            loc.y -= half_h;
                                        },
                                        wayland_protocols::xdg::shell::client::xdg_positioner::Gravity::TopLeft => {
                                            loc.y -= size.height;
                                            loc.x -= size.width;
                                        },
                                        wayland_protocols::xdg::shell::client::xdg_positioner::Gravity::BottomLeft => {
                                            loc.x -= size.width;
                                        },
                                        wayland_protocols::xdg::shell::client::xdg_positioner::Gravity::TopRight => {
                                            loc.y -= size.height;
                                        },
                                        wayland_protocols::xdg::shell::client::xdg_positioner::Gravity::BottomRight => {},
                                        _ => unimplemented!(),
                                    };
                                    subsurface.instance.wl_subsurface.set_position(loc.x as i32, loc.y as i32);

                            }
                                send_event(&state.events_sender, &state.proxy, SctkEvent::SubsurfaceEvent(crate::sctk_event::SubsurfaceEventVariant::Resized(id, size)))},
                        },
                        calloop::channel::Event::Closed => {
                            log::info!("Calloop channel closed.");
                        }
                    }
                })
                .unwrap();
                let wayland_source =
                    WaylandSource::new(connection.clone(), event_queue);

                let wayland_dispatcher = calloop::Dispatcher::new(
                    wayland_source,
                    |_, queue, winit_state| queue.dispatch_pending(winit_state),
                );

                let _wayland_source_dispatcher = event_loop
                    .handle()
                    .register_dispatcher(wayland_dispatcher.clone())
                    .unwrap();

                let (viewporter_state, fractional_scaling_manager) =
                    match FractionalScalingManager::new(&globals, &qh) {
                        Ok(m) => {
                            let viewporter_state: Option<ViewporterState> =
                                match ViewporterState::new(&globals, &qh) {
                                    Ok(s) => Some(s),
                                    Err(e) => {
                                        error!(
                                            "Failed to initialize viewporter: {}",
                                            e
                                        );
                                        None
                                    }
                                };
                            (viewporter_state, Some(m))
                        }
                        Err(e) => {
                            error!(
                                "Failed to initialize fractional scaling manager: {}",
                                e
                            );
                            (None, None)
                        }
                    };

                let mut state = Self {
                    event_loop,
                    state: SctkState {
                        connection,
                        seat_state: SeatState::new(&globals, &qh),
                        output_state: OutputState::new(&globals, &qh),
                        compositor_state: CompositorState::bind(&globals, &qh)
                            .expect("wl_compositor is not available"),
                        shm_state: Shm::bind(&globals, &qh)
                            .expect("wl_shm is not available"),
                        xdg_shell_state: XdgShell::bind(&globals, &qh)
                            .expect("xdg shell is not available"),
                        layer_shell: LayerShell::bind(&globals, &qh).ok(),
                        activation_state: ActivationState::bind(&globals, &qh)
                            .ok(),
                        session_lock_state: SessionLockState::new(
                            &globals, &qh,
                        ),
                        session_lock: None,
                        overlap_notify: OverlapNotifyV1::bind(&globals, &qh)
                            .ok(),
                        toplevel_info: ToplevelInfoState::try_new(
                            &registry_state,
                            &qh,
                        ),
                        corner_radius_manager: registry_state.bind_one::<CosmicCornerRadiusManagerV1, _, _>(
                        &qh,
                        1..=1,
                        (),
                    ).ok(),
                        toplevel_manager: ToplevelManagerState::try_new(
                            &registry_state,
                            &qh,
                        ),
                                            inhibitor_manager: registry_state.bind_one::<zwp_keyboard_shortcuts_inhibit_manager_v1::ZwpKeyboardShortcutsInhibitManagerV1, _, _>(
                        &qh,
                        1..=1,
                        (),
                    ).ok(),
                        text_input_manager: TextInputManager::try_new(&registry_state, &qh),
                        registry_state,

                        queue_handle: qh,
                        loop_handle,


                        inhibitor: None,
                        inhibited: false,
                        _cursor_surface: None,
                        _multipool: None,
                        outputs: Vec::new(),
                        seats: Vec::new(),
                        windows: Vec::new(),
                        layer_surfaces: Vec::new(),
                        popups: Vec::new(),
                        lock_surfaces: Vec::new(),
                        subsurfaces: Vec::new(),
                        touch_points: HashMap::new(),
                        sctk_events: Vec::new(),
                        frame_status: HashMap::new(),
                        fractional_scaling_manager,
                        viewporter_state,
                        compositor_updates: Default::default(),
                        events_sender: winit_event_sender,
                        proxy,
                        id_map: Default::default(),
                        to_commit: HashMap::new(),
                        destroyed: HashSet::new(),
                        pending_popup: Default::default(),
                        activation_token_ctr: 0,
                        token_senders: HashMap::new(),
                        overlap_notifications: HashMap::new(),
                        subsurface_state: None,
                        pending_corner_radius: HashMap::new(),
                        text_input: None,
                        preedit: None,
                        pending_commit: None,
                    },
                    _features: Default::default(),
                };
                let wl_compositor = state
                    .state
                    .registry_state
                    .bind_one(&state.state.queue_handle, 1..=6, GlobalData)
                    .unwrap();
                let wl_subcompositor = state.state.registry_state.bind_one(
                    &state.state.queue_handle,
                    1..=1,
                    GlobalData,
                );
                let wp_viewporter = state.state.registry_state.bind_one(
                    &state.state.queue_handle,
                    1..=1,
                    GlobalData,
                );
                let wl_shm = state
                    .state
                    .registry_state
                    .bind_one(&state.state.queue_handle, 1..=1, GlobalData)
                    .unwrap();
                let wp_dmabuf = state
                    .state
                    .registry_state
                    .bind_one(&state.state.queue_handle, 2..=4, GlobalData)
                    .ok();
                let wp_alpha_modifier = state
                    .state
                    .registry_state
                    .bind_one(&state.state.queue_handle, 1..=1, ())
                    .ok();

                if let (Ok(wl_subcompositor), Ok(wp_viewporter)) =
                    (wl_subcompositor, wp_viewporter)
                {
                    let subsurface_state = SubsurfaceState {
                        wl_compositor,
                        wl_subcompositor,
                        wp_viewporter,
                        wl_shm,
                        wp_dmabuf,
                        wp_alpha_modifier,
                        qh: state.state.queue_handle.clone(),
                        buffers: HashMap::new(),
                        unmapped_subsurfaces: Vec::new(),
                        new_iced_subsurfaces: Vec::new(),
                    };
                    state.state.subsurface_state =
                        Some(subsurface_state.clone());
                    state::send_event(
                        &state.state.events_sender,
                        &state.state.proxy,
                        SctkEvent::Subcompositor(subsurface_state),
                    );
                } else {
                    log::warn!("Subsurfaces not supported.")
                }

                log::info!("SCTK setup complete.");
                loop {
                    match state
                        .state
                        .events_sender
                        .unbounded_send(Control::AboutToWait)
                    {
                        Ok(_) => {}
                        Err(err) => {
                            log::error!(
                                "SCTK failed to send Control::AboutToWait. {err:?}"
                            );
                            if state.state.events_sender.is_closed() {
                                return Ok(());
                            }
                        }
                    }

                    if let Err(err) =
                        state.event_loop.dispatch(None, &mut state.state)
                    {
                        log::error!("SCTK dispatch error: {err}");
                    }
                    let had_events = !state.state.sctk_events.is_empty();
                    let mut wake_up = had_events;

                    for s in
                        state
                            .state
                            .layer_surfaces
                            .iter()
                            .map(|s| s.surface.wl_surface())
                            .chain(
                                state
                                    .state
                                    .popups
                                    .iter()
                                    .map(|s| s.popup.wl_surface()),
                            )
                            .chain(
                                state.state.lock_surfaces.iter().map(|s| {
                                    s.session_lock_surface.wl_surface()
                                }),
                            )
                    {
                        let id = s.id();
                        if state
                            .state
                            .frame_status
                            .get(&id)
                            .map(|v| !matches!(v, state::FrameStatus::Ready))
                            .unwrap_or(true)
                            || !state.state.id_map.contains_key(&id)
                        {
                            continue;
                        }
                        wake_up = true;

                        _ = s.frame(&state.state.queue_handle, s.clone());
                        _ = state.state.frame_status.remove(&id);
                        _ = state.state.events_sender.unbounded_send(
                            Control::Winit(
                                winit::window::WindowId::from_raw(
                                    id.as_ptr() as usize
                                ),
                                winit::event::WindowEvent::RedrawRequested,
                            ),
                        );
                    }

                    for e in state.state.sctk_events.drain(..) {
                        if let SctkEvent::Winit(id, e) = e {
                            _ = state
                                .state
                                .events_sender
                                .unbounded_send(Control::Winit(id, e));
                        } else {
                            _ =
                                state
                                    .state
                                    .events_sender
                                    .unbounded_send(Control::PlatformSpecific(
                                    crate::platform_specific::Event::Wayland(e),
                                ));
                        }
                    }
                    if wake_up {
                        state.state.proxy.wake_up();
                    }
                }
            });

        if res.is_finished() {
            log::warn!("SCTK thread finished.");
            match res.join() {
                Ok(_) => Ok(action_tx),
                Err(e) => {
                    log::error!("SCTK thread exited with error: {e:?}");
                    return Err(e);
                }
            }
        } else {
            Ok(action_tx)
        }
    }
}
