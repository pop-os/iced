pub mod control_flow;
pub mod proxy;
pub mod state;

#[cfg(feature = "a11y")]
use crate::platform_specific::SurfaceIdWrapper;
use crate::{
    Control,
    futures::futures::channel::mpsc,
    handlers::overlap::OverlapNotifyV1,
    platform_specific::wayland::{
        handlers::{
            wp_fractional_scaling::FractionalScalingManager,
            wp_viewporter::ViewporterState,
        },
        sctk_event::SctkEvent,
    },
    subsurface_widget::SubsurfaceState,
};

use cctk::sctk::reexports::calloop_wayland_source::WaylandSource;
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
    toplevel_info::ToplevelInfoState,
    toplevel_management::ToplevelManagerState,
};
use raw_window_handle::HasDisplayHandle;
use state::{FrameStatus, SctkWindow};
#[cfg(feature = "a11y")]
use std::sync::{Arc, Mutex};
use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
};
use tracing::error;
use wayland_backend::client::Backend;
use winit::event_loop::OwnedDisplayHandle;

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
            return Err(Box::new(()));
        };
        let raw_window_handle::RawDisplayHandle::Wayland(wayland_dh) =
            dh.as_raw()
        else {
            panic!("Invalid wayland display handle.");
        };

        let backend = unsafe {
            Backend::from_foreign_display(wayland_dh.display.as_ptr().cast())
        };
        let connection = Connection::from_backend(backend);

        let (action_tx, action_rx) = calloop::channel::channel();
        let res = std::thread::spawn(move || {
            let _display = connection.display();
            let (globals, event_queue) =
                registry_queue_init(&connection).unwrap();
            let event_loop =
                calloop::EventLoop::<SctkState>::try_new().unwrap();
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
                            crate::platform_specific::Action::TrackWindow(
                                window,
                                id,
                            ) => {
                                state.windows.push(SctkWindow { window, id });
                            }
                            crate::platform_specific::Action::RemoveWindow(
                                id,
                            ) => {
                                // TODO clean up popups matching the window.
                                state.windows.retain(|window| id != window.id);
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
                        let viewporter_state =
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
                    activation_state: ActivationState::bind(&globals, &qh).ok(),
                    session_lock_state: SessionLockState::new(&globals, &qh),
                    session_lock: None,
                    overlap_notify: OverlapNotifyV1::bind(&globals, &qh).ok(),
                    toplevel_info: ToplevelInfoState::try_new(
                        &registry_state,
                        &qh,
                    ),
                    toplevel_manager: ToplevelManagerState::try_new(
                        &registry_state,
                        &qh,
                    ),
                    registry_state,

                    queue_handle: qh,
                    loop_handle,

                    _cursor_surface: None,
                    _multipool: None,
                    outputs: Vec::new(),
                    seats: Vec::new(),
                    windows: Vec::new(),
                    layer_surfaces: Vec::new(),
                    popups: Vec::new(),
                    lock_surfaces: Vec::new(),
                    _kbd_focus: None,
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
                state::send_event(
                    &state.state.events_sender,
                    &state.state.proxy,
                    SctkEvent::Subcompositor(SubsurfaceState {
                        wl_compositor,
                        wl_subcompositor,
                        wp_viewporter,
                        wl_shm,
                        wp_dmabuf,
                        wp_alpha_modifier,
                        qh: state.state.queue_handle.clone(),
                        buffers: HashMap::new(),
                        unmapped_subsurfaces: Vec::new(),
                    }),
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

                for s in state
                    .state
                    .layer_surfaces
                    .iter()
                    .map(|s| s.surface.wl_surface())
                    .chain(
                        state.state.popups.iter().map(|s| s.popup.wl_surface()),
                    )
                    .chain(
                        state
                            .state
                            .lock_surfaces
                            .iter()
                            .map(|s| s.session_lock_surface.wl_surface()),
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
                        _ = state.state.events_sender.unbounded_send(
                            Control::PlatformSpecific(
                                crate::platform_specific::Event::Wayland(e),
                            ),
                        );
                    }
                }
                if wake_up {
                    state.state.proxy.wake_up();
                }
            }
        });

        if res.is_finished() {
            log::warn!("SCTK thread finished.");
            res.join().map(|_: Result<(), ConnectError>| action_tx)
        } else {
            Ok(action_tx)
        }
    }
}
