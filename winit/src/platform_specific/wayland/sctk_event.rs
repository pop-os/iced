use crate::{
    platform_specific::{
        wayland::{
            conversion::{
                modifiers_to_native, pointer_axis_to_native,
                pointer_button_to_native,
            },
            keymap::{self, keysym_to_key},
            subsurface_widget::SubsurfaceState,
        },
        SurfaceIdWrapper,
    },
    program::{Control, Program},
    Clipboard,
};

use iced_futures::{
    core::event::{
        wayland::{LayerEvent, PopupEvent, SessionLockEvent},
        PlatformSpecific,
    },
    futures::channel::mpsc,
};
use iced_graphics::Compositor;
use iced_runtime::{
    core::{
        event::wayland,
        keyboard, mouse, touch,
        window::{self, Id as SurfaceId},
        Point,
    },
    keyboard::{key, Key, Location},
    user_interface, Debug,
};

use sctk::{
    output::OutputInfo,
    reexports::{
        calloop::channel,
        client::{
            backend::ObjectId,
            protocol::{
                wl_display::WlDisplay, wl_keyboard::WlKeyboard,
                wl_output::WlOutput, wl_pointer::WlPointer, wl_seat::WlSeat,
                wl_surface::WlSurface, wl_touch::WlTouch,
            },
            Proxy,
        },
        csd_frame::WindowManagerCapabilities,
    },
    seat::{
        keyboard::{KeyEvent, Modifiers},
        pointer::{PointerEvent, PointerEventKind},
        Capability,
    },
    session_lock::SessionLockSurfaceConfigure,
    shell::{
        wlr_layer::LayerSurfaceConfigure,
        xdg::{popup::PopupConfigure, window::WindowConfigure},
    },
};
use std::{
    collections::HashMap,
    num::NonZeroU32,
    sync::{Arc, Mutex},
    time::Instant,
};
use wayland_protocols::wp::viewporter::client::wp_viewport::WpViewport;
use winit::{event::WindowEvent, window::WindowId};
use xkeysym::Keysym;

use super::{
    event_loop::state::Common, keymap::raw_keycode_to_physicalkey,
    winit_window::SctkWinitWindow,
};

pub enum IcedSctkEvent {
    /// Emitted when new events arrive from the OS to be processed.
    ///
    /// This event type is useful as a place to put code that should be done before you start
    /// processing events, such as updating frame timing information for benchmarking or checking
    /// the [`StartCause`][crate::event::StartCause] to see if a timer set by
    /// [`ControlFlow::WaitUntil`](crate::platform_specific::wayland::event_loop::ControlFlow::WaitUntil) has elapsed.
    NewEvents(StartCause),

    /// An event produced by sctk
    SctkEvent(SctkEvent),

    /// Emitted when all of the event loop's input events have been processed and redraw processing
    /// is about to begin.
    ///
    /// This event is useful as a place to put your code that should be run after all
    /// state-changing events have been handled and you want to do stuff (updating state, performing
    /// calculations, etc) that happens as the "main body" of your event loop. If your program only draws
    /// graphics when something changes, it's usually better to do it in response to
    /// [`Event::RedrawRequested`](crate::event::Event::RedrawRequested), which gets emitted
    /// immediately after this event. Programs that draw graphics continuously, like most games,
    /// can render here unconditionally for simplicity.
    MainEventsCleared,

    /// Emitted after [`MainEventsCleared`] when a window should be redrawn.
    ///
    /// This gets triggered in two scenarios:
    /// - The OS has performed an operation that's invalidated the window's contents (such as
    ///   resizing the window).
    /// - The application has explicitly requested a redraw via [`Window::request_redraw`].
    ///
    /// During each iteration of the event loop, Winit will aggregate duplicate redraw requests
    /// into a single event, to help avoid duplicating rendering work.
    ///
    /// Mainly of interest to applications with mostly-static graphics that avoid redrawing unless
    /// something changes, like most non-game GUIs.
    ///
    /// [`MainEventsCleared`]: Self::MainEventsCleared
    RedrawRequested(ObjectId),

    /// Emitted after all [`RedrawRequested`] events have been processed and control flow is about to
    /// be taken away from the program. If there are no `RedrawRequested` events, it is emitted
    /// immediately after `MainEventsCleared`.
    ///
    /// This event is useful for doing any cleanup or bookkeeping work after all the rendering
    /// tasks have been completed.
    ///
    /// [`RedrawRequested`]: Self::RedrawRequested
    RedrawEventsCleared,

    /// Emitted when the event loop is being shut down.
    ///
    /// This is irreversible - if this event is emitted, it is guaranteed to be the last event that
    /// gets emitted. You generally want to treat this as an "do on quit" event.
    LoopDestroyed,

    /// Frame callback event
    Frame(WlSurface, u32),
}

#[derive(Debug, Clone)]
pub enum SctkEvent {
    //
    // Input events
    //
    SeatEvent {
        variant: SeatEventVariant,
        id: WlSeat,
    },
    PointerEvent {
        variant: PointerEvent,
        ptr_id: WlPointer,
        seat_id: WlSeat,
    },
    KeyboardEvent {
        variant: KeyboardEventVariant,
        kbd_id: WlKeyboard,
        seat_id: WlSeat,
        surface: WlSurface,
    },
    TouchEvent {
        variant: touch::Event,
        touch_id: WlTouch,
        seat_id: WlSeat,
        surface: WlSurface,
    },
    // TODO data device & touch

    //
    // Surface Events
    //
    WindowEvent {
        variant: WindowEventVariant,
        id: WlSurface,
    },
    LayerSurfaceEvent {
        variant: LayerSurfaceEventVariant,
        id: WlSurface,
    },
    PopupEvent {
        variant: PopupEventVariant,
        /// this may be the Id of a window or layer surface
        toplevel_id: WlSurface,
        /// this may be any SurfaceId
        parent_id: WlSurface,
        /// the id of this popup
        id: WlSurface,
    },

    //
    // output events
    //
    NewOutput {
        id: WlOutput,
        info: Option<OutputInfo>,
    },
    UpdateOutput {
        id: WlOutput,
        info: OutputInfo,
    },
    RemovedOutput(WlOutput),
    //
    // compositor events
    //
    ScaleFactorChanged {
        factor: f64,
        id: WlOutput,
        inner_size: winit::dpi::PhysicalSize<u32>,
    },

    /// session lock events
    SessionLocked,
    SessionLockFinished,
    SessionLockSurfaceCreated {
        surface: WlSurface,
        native_id: SurfaceId,
        common: Arc<Mutex<Common>>,
        display: WlDisplay,
    },
    SessionLockSurfaceConfigure {
        surface: WlSurface,
        configure: SessionLockSurfaceConfigure,
        first: bool,
    },
    SessionLockSurfaceDone {
        surface: WlSurface,
    },
    SessionUnlocked,
    SurfaceScaleFactorChanged(f64, WlSurface, window::Id),
    Winit(WindowId, WindowEvent),
    Subcompositor(SubsurfaceState),
}

#[cfg(feature = "a11y")]
#[derive(Debug, Clone)]
pub struct ActionRequestEvent {
    pub surface_id: ObjectId,
    pub request: iced_accessibility::accesskit::ActionRequest,
}

#[derive(Debug, Clone)]
pub enum SeatEventVariant {
    New,
    Remove,
    NewCapability(Capability, ObjectId),
    RemoveCapability(Capability, ObjectId),
}

#[derive(Debug, Clone)]
pub enum KeyboardEventVariant {
    Leave(WlSurface),
    Enter(WlSurface),
    Press(KeyEvent),
    Repeat(KeyEvent),
    Release(KeyEvent),
    Modifiers(Modifiers),
}

#[derive(Debug, Clone)]
pub enum WindowEventVariant {
    Created(WlSurface, SurfaceId),
    /// <https://wayland.app/protocols/xdg-shell#xdg_toplevel:event:close>
    Close,
    /// <https://wayland.app/protocols/xdg-shell#xdg_toplevel:event:wm_capabilities>
    WmCapabilities(WindowManagerCapabilities),
    /// <https://wayland.app/protocols/xdg-shell#xdg_toplevel:event:configure_bounds>
    ConfigureBounds {
        width: u32,
        height: u32,
    },
    /// <https://wayland.app/protocols/xdg-shell#xdg_toplevel:event:configure>
    Configure((NonZeroU32, NonZeroU32), WindowConfigure, WlSurface, bool),
    Size((NonZeroU32, NonZeroU32), WlSurface, bool),
    /// window state changed
    StateChanged(sctk::reexports::csd_frame::WindowState),
    /// Scale Factor
    ScaleFactorChanged(f64, Option<WpViewport>),
}

#[derive(Debug, Clone)]
pub enum PopupEventVariant {
    /// Popup Created
    Created(WlSurface, SurfaceId, Arc<Mutex<Common>>, WlDisplay),
    /// <https://wayland.app/protocols/xdg-shell#xdg_popup:event:popup_done>
    Done,
    /// <https://wayland.app/protocols/xdg-shell#xdg_popup:event:configure>
    Configure(PopupConfigure, WlSurface, bool),
    /// <https://wayland.app/protocols/xdg-shell#xdg_popup:event:repositioned>
    RepositionionedPopup { token: u32 },
    /// size
    Size(u32, u32),
    /// Scale Factor
    ScaleFactorChanged(f64, Option<WpViewport>),
}

#[derive(Debug, Clone)]
pub enum LayerSurfaceEventVariant {
    /// sent after creation of the layer surface
    Created(WlSurface, SurfaceId, Arc<Mutex<Common>>, WlDisplay, String),
    /// <https://wayland.app/protocols/wlr-layer-shell-unstable-v1#zwlr_layer_surface_v1:event:closed>
    Done,
    /// <https://wayland.app/protocols/wlr-layer-shell-unstable-v1#zwlr_layer_surface_v1:event:configure>
    Configure(LayerSurfaceConfigure, WlSurface, bool),
    /// Scale Factor
    ScaleFactorChanged(f64, Option<WpViewport>),
}

/// Describes the reason the event loop is resuming.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartCause {
    /// Sent if the time specified by [`ControlFlow::WaitUntil`] has been reached. Contains the
    /// moment the timeout was requested and the requested resume time. The actual resume time is
    /// guaranteed to be equal to or after the requested resume time.
    ///
    /// [`ControlFlow::WaitUntil`]: crate::platform_specific::wayland::event_loop::ControlFlow::WaitUntil
    ResumeTimeReached {
        start: Instant,
        requested_resume: Instant,
    },

    /// Sent if the OS has new events to send to the window, after a wait was requested. Contains
    /// the moment the wait was requested and the resume time, if requested.
    WaitCancelled {
        start: Instant,
        requested_resume: Option<Instant>,
    },

    /// Sent if the event loop is being resumed after the loop's control flow was set to
    /// [`ControlFlow::Poll`].
    ///
    /// [`ControlFlow::Poll`]: crate::platform_specific::wayland::event_loop::ControlFlow::Poll
    Poll,

    /// Sent once, immediately after `run` is called. Indicates that the loop was just initialized.
    Init,
}

/// Pending update to a window requested by the user.
#[derive(Default, Debug, Clone, Copy)]
pub struct SurfaceUserRequest {
    /// Whether `redraw` was requested.
    pub redraw_requested: bool,

    /// Wether the frame should be refreshed.
    pub refresh_frame: bool,
}

// The window update coming from the compositor.
#[derive(Default, Debug, Clone)]
pub struct SurfaceCompositorUpdate {
    /// New window configure.
    pub configure: Option<WindowConfigure>,

    /// New scale factor.
    pub scale_factor: Option<i32>,
}
pub type UserInterfaces<'a, P> = HashMap<
    SurfaceId,
    user_interface::UserInterface<
        'a,
        <P as Program>::Message,
        <P as Program>::Theme,
        <P as Program>::Renderer,
    >,
    rustc_hash::FxBuildHasher,
>;

impl SctkEvent {
    pub(crate) fn process<'a, P, C>(
        self,
        modifiers: &mut Modifiers,
        program: &'a P,
        compositor: &mut C,
        window_manager: &mut crate::program::WindowManager<P, C>,
        surface_ids: &mut HashMap<ObjectId, SurfaceIdWrapper>,
        subsurface_ids: &mut HashMap<ObjectId, (i32, i32, window::Id)>,
        sctk_tx: &channel::Sender<super::Action>,
        control_sender: &mpsc::UnboundedSender<Control>,
        debug: &mut Debug,
        user_interfaces: &mut UserInterfaces<'a, P>,
        events: &mut Vec<(Option<window::Id>, iced_runtime::core::Event)>,
        clipboard: &mut Clipboard,
        subsurface_state: &mut Option<SubsurfaceState>,
        #[cfg(feature = "a11y")] adapters: &mut HashMap<
            window::Id,
            (u64, iced_accessibility::accesskit_winit::Adapter),
        >,
    ) where
        P: Program,
        C: Compositor<Renderer = P::Renderer>,
    {
        match self {
            // TODO Ashley: Platform specific multi-seat events?
            SctkEvent::SeatEvent { .. } => Default::default(),
            SctkEvent::PointerEvent { variant, .. } => match variant.kind {
                PointerEventKind::Enter { .. } => {
                    events.push((
                        surface_ids
                            .get(&variant.surface.id())
                            .map(|id| id.inner()),
                        iced_runtime::core::Event::Mouse(
                            mouse::Event::CursorEntered,
                        ),
                    ));
                }
                PointerEventKind::Leave { .. } => events.push((
                    surface_ids.get(&variant.surface.id()).map(|id| id.inner()),
                    iced_runtime::core::Event::Mouse(mouse::Event::CursorLeft),
                )),
                PointerEventKind::Motion { .. } => {
                    let offset = if let Some((x_offset, y_offset, _)) =
                        subsurface_ids.get(&variant.surface.id())
                    {
                        (*x_offset, *y_offset)
                    } else {
                        (0, 0)
                    };
                    let id = surface_ids
                        .get(&variant.surface.id())
                        .map(|id| id.inner());
                    if let Some(w) =
                        id.clone().and_then(|id| window_manager.get_mut(id))
                    {
                        w.state.set_logical_cursor_pos(
                            (
                                variant.position.0 + offset.0 as f64,
                                variant.position.1 + offset.1 as f64,
                            )
                                .into(),
                        )
                    }
                    events.push((
                        id,
                        iced_runtime::core::Event::Mouse(
                            mouse::Event::CursorMoved {
                                position: Point::new(
                                    variant.position.0 as f32 + offset.0 as f32,
                                    variant.position.1 as f32 + offset.1 as f32,
                                ),
                            },
                        ),
                    ));
                }
                PointerEventKind::Press {
                    time: _,
                    button,
                    serial: _,
                } => {
                    if let Some(e) = pointer_button_to_native(button).map(|b| {
                        iced_runtime::core::Event::Mouse(
                            mouse::Event::ButtonPressed(b),
                        )
                    }) {
                        events.push((
                            surface_ids
                                .get(&variant.surface.id())
                                .map(|id| id.inner()),
                            e,
                        ));
                    }
                } // TODO Ashley: conversion
                PointerEventKind::Release {
                    time: _,
                    button,
                    serial: _,
                } => {
                    if let Some(e) = pointer_button_to_native(button).map(|b| {
                        iced_runtime::core::Event::Mouse(
                            mouse::Event::ButtonReleased(b),
                        )
                    }) {
                        events.push((
                            surface_ids
                                .get(&variant.surface.id())
                                .map(|id| id.inner()),
                            e,
                        ));
                    }
                } // TODO Ashley: conversion
                PointerEventKind::Axis {
                    time: _,
                    horizontal,
                    vertical,
                    source,
                } => {
                    if let Some(e) =
                        pointer_axis_to_native(source, horizontal, vertical)
                            .map(|a| {
                                iced_runtime::core::Event::Mouse(
                                    mouse::Event::WheelScrolled { delta: a },
                                )
                            })
                    {
                        events.push((
                            surface_ids
                                .get(&variant.surface.id())
                                .map(|id| id.inner()),
                            e,
                        ));
                    }
                } // TODO Ashley: conversion
            },
            SctkEvent::KeyboardEvent {
                variant,
                kbd_id: _,
                seat_id,
                surface,
            } => match variant {
                KeyboardEventVariant::Leave(surface) => {
                    if let Some(e) =
                        surface_ids.get(&surface.id()).and_then(|id| match id {
                            SurfaceIdWrapper::LayerSurface(_id) => Some(
                                iced_runtime::core::Event::PlatformSpecific(
                                    PlatformSpecific::Wayland(
                                        wayland::Event::Layer(
                                            LayerEvent::Unfocused,
                                            surface.clone(),
                                            id.inner(),
                                        ),
                                    ),
                                ),
                            ),
                            SurfaceIdWrapper::Window(id) => {
                                Some(iced_runtime::core::Event::Window(
                                    window::Event::Unfocused,
                                ))
                            }
                            SurfaceIdWrapper::Popup(_id) => Some(
                                iced_runtime::core::Event::PlatformSpecific(
                                    PlatformSpecific::Wayland(
                                        wayland::Event::Popup(
                                            PopupEvent::Unfocused,
                                            surface.clone(),
                                            id.inner(),
                                        ),
                                    ),
                                ),
                            ),
                            SurfaceIdWrapper::SessionLock(_) => Some(
                                iced_runtime::core::Event::PlatformSpecific(
                                    PlatformSpecific::Wayland(
                                        wayland::Event::SessionLock(
                                            SessionLockEvent::Unfocused(
                                                surface.clone(),
                                                id.inner(),
                                            ),
                                        ),
                                    ),
                                ),
                            ),
                        })
                    {
                        events.push((
                            surface_ids.get(&surface.id()).map(|id| id.inner()),
                            e,
                        ));
                    }

                    events.push((
                        surface_ids.get(&surface.id()).map(|id| id.inner()),
                        iced_runtime::core::Event::PlatformSpecific(
                            PlatformSpecific::Wayland(wayland::Event::Seat(
                                wayland::SeatEvent::Leave,
                                seat_id,
                            )),
                        ),
                    ))
                }
                KeyboardEventVariant::Enter(surface) => {
                    if let Some(e) =
                        surface_ids.get(&surface.id()).and_then(|id| {
                            match id {
                                SurfaceIdWrapper::LayerSurface(_id) => Some(
                                    iced_runtime::core::Event::PlatformSpecific(
                                        PlatformSpecific::Wayland(
                                            wayland::Event::Layer(
                                                LayerEvent::Focused,
                                                surface.clone(),
                                                id.inner(),
                                            ),
                                        ),
                                    ),
                                ),
                                SurfaceIdWrapper::Window(id) => {
                                    Some(iced_runtime::core::Event::Window(
                                        window::Event::Focused,
                                    ))
                                }
                                SurfaceIdWrapper::Popup(_id) => Some(
                                    iced_runtime::core::Event::PlatformSpecific(
                                        PlatformSpecific::Wayland(
                                            wayland::Event::Popup(
                                                PopupEvent::Focused,
                                                surface.clone(),
                                                id.inner(),
                                            ),
                                        ),
                                    ),
                                ),
                                SurfaceIdWrapper::SessionLock(_) => Some(
                                    iced_runtime::core::Event::PlatformSpecific(
                                        PlatformSpecific::Wayland(
                                            wayland::Event::SessionLock(
                                                SessionLockEvent::Focused(
                                                    surface.clone(),
                                                    id.inner(),
                                                ),
                                            ),
                                        ),
                                    ),
                                ),
                            }
                            .map(|e| (Some(id.inner()), e))
                        })
                    {
                        events.push(e);
                    }

                    events.push((
                        surface_ids.get(&surface.id()).map(|id| id.inner()),
                        iced_runtime::core::Event::PlatformSpecific(
                            PlatformSpecific::Wayland(wayland::Event::Seat(
                                wayland::SeatEvent::Enter,
                                seat_id,
                            )),
                        ),
                    ));
                }
                KeyboardEventVariant::Press(ke) => {
                    let (key, location) = keysym_to_vkey_location(ke.keysym);
                    let physical_key = raw_keycode_to_physicalkey(ke.raw_code);
                    let physical_key =
                        crate::conversion::physical_key(physical_key);

                    events.push((
                        surface_ids.get(&surface.id()).map(|id| id.inner()),
                        iced_runtime::core::Event::Keyboard(
                            keyboard::Event::KeyPressed {
                                key: key.clone(),
                                location: location,
                                text: ke.utf8.map(|s| s.into()),
                                modifiers: modifiers_to_native(*modifiers),
                                physical_key,
                                modified_key: key, // TODO calculate without Ctrl?
                            },
                        ),
                    ))
                }
                KeyboardEventVariant::Repeat(KeyEvent {
                    keysym,
                    utf8,
                    raw_code,
                    ..
                }) => {
                    let (key, location) = keysym_to_vkey_location(keysym);
                    let physical_key = raw_keycode_to_physicalkey(raw_code);
                    let physical_key =
                        crate::conversion::physical_key(physical_key);

                    events.push((
                        surface_ids.get(&surface.id()).map(|id| id.inner()),
                        iced_runtime::core::Event::Keyboard(
                            keyboard::Event::KeyPressed {
                                key: key.clone(),
                                location: location,
                                text: utf8.map(|s| s.into()),
                                modifiers: modifiers_to_native(*modifiers),
                                physical_key,
                                modified_key: key, // TODO calculate without Ctrl?
                            },
                        ),
                    ))
                }
                KeyboardEventVariant::Release(ke) => {
                    let (k, location) = keysym_to_vkey_location(ke.keysym);
                    events.push((
                        surface_ids.get(&surface.id()).map(|id| id.inner()),
                        iced_runtime::core::Event::Keyboard(
                            keyboard::Event::KeyReleased {
                                key: k,
                                location: location,
                                modifiers: modifiers_to_native(*modifiers),
                            },
                        ),
                    ))
                }
                KeyboardEventVariant::Modifiers(new_mods) => {
                    *modifiers = new_mods;
                    events.push((
                        surface_ids.get(&surface.id()).map(|id| id.inner()),
                        iced_runtime::core::Event::Keyboard(
                            keyboard::Event::ModifiersChanged(
                                modifiers_to_native(new_mods),
                            ),
                        ),
                    ))
                }
            },
            SctkEvent::TouchEvent {
                variant,
                touch_id: _,
                seat_id: _,
                surface,
            } => events.push((
                surface_ids.get(&surface.id()).map(|id| id.inner()),
                iced_runtime::core::Event::Touch(variant),
            )),
            SctkEvent::WindowEvent {
                variant,
                id: surface,
            } => {
                let id = surface_ids.get(&surface.id()).map(|id| id.inner());
                match variant {
                WindowEventVariant::Created(..) => {},
                WindowEventVariant::Close => {
                    if let Some(e) =
                        surface_ids.remove(&surface.id()).map(|id| {
                            (
                                Some(id.inner()),
                                iced_runtime::core::Event::Window(
                                    window::Event::Closed,
                                ),
                            )
                        })
                    {
                        events.push(e);
                    }
                }
                WindowEventVariant::WmCapabilities(caps) => {
                    if let Some(e) = surface_ids
                        .get(&surface.id())
                        .map(|id| id.inner())
                        .map(|id| {
                            (Some(id), iced_runtime::core::Event::PlatformSpecific(
                            PlatformSpecific::Wayland(wayland::Event::Window(
                                wayland::WindowEvent::WmCapabilities(caps),
                                surface,
                                id,
                            )),
                        ))
                        })
                    {
                        events.push(e);
                    }
                }
                WindowEventVariant::ConfigureBounds { .. } => {}
                WindowEventVariant::Configure(
                    (new_width, new_height),
                    configure,
                    surface,
                    _,
                ) => {}
                WindowEventVariant::ScaleFactorChanged(..) => {}
                WindowEventVariant::StateChanged(s) => {}
                WindowEventVariant::Size(_, _, _) => {}}
            }
            SctkEvent::LayerSurfaceEvent {
                variant,
                id: surface,
            } => match variant {
                LayerSurfaceEventVariant::Done => {
                    if let Some(id) = surface_ids.remove(&surface.id()) {
                        _ = window_manager.remove(id.inner());

                        events.push((
                            Some(id.inner()),
                            iced_runtime::core::Event::PlatformSpecific(
                                PlatformSpecific::Wayland(
                                    wayland::Event::Layer(
                                        LayerEvent::Done,
                                        surface,
                                        id.inner(),
                                    ),
                                ),
                            ),
                        ));
                    }
                }
                LayerSurfaceEventVariant::Created(
                    surface,
                    surface_id,
                    common,
                    display,
                    title,
                ) => {
                    let object_id = surface.id();
                    let wrapper =
                        SurfaceIdWrapper::LayerSurface(surface_id.clone());
                    _ = surface_ids.insert(surface.id(), wrapper.clone());
                    let sctk_winit = SctkWinitWindow::new(
                        sctk_tx.clone(),
                        common,
                        wrapper,
                        surface,
                        display,
                    );

                    #[cfg(feature = "a11y")]
                    {
                        use crate::a11y::*;
                        use iced_accessibility::accesskit::{
                            ActivationHandler, NodeBuilder, NodeId, Role, Tree,
                            TreeUpdate,
                        };
                        use iced_accessibility::accesskit_winit::Adapter;

                        let node_id = iced_runtime::core::id::window_node_id();

                        let activation_handler = WinitActivationHandler {
                            proxy: control_sender.clone(),
                            title: String::new(),
                        };

                        let action_handler = WinitActionHandler {
                            id: surface_id,
                            proxy: control_sender.clone(),
                        };

                        let deactivation_handler = WinitDeactivationHandler {
                            proxy: control_sender.clone(),
                        };
                        _ = adapters.insert(
                            surface_id,
                            (
                                node_id,
                                Adapter::with_direct_handlers(
                                    sctk_winit.as_ref(),
                                    activation_handler,
                                    action_handler,
                                    deactivation_handler,
                                ),
                            ),
                        );
                    }

                    let window = window_manager.insert(
                        surface_id, sctk_winit, program, compositor,
                        false, // TODO do we want to get this value here?
                        0,
                    );
                    _ = surface_ids.insert(object_id, wrapper.clone());
                    let logical_size = window.size();

                    let _ = user_interfaces.insert(
                        surface_id,
                        crate::program::build_user_interface(
                            program,
                            user_interface::Cache::default(),
                            &mut window.renderer,
                            logical_size,
                            debug,
                            surface_id,
                            window.raw.clone(),
                            window.prev_dnd_destination_rectangles_count,
                            clipboard,
                        ),
                    );
                }
                LayerSurfaceEventVariant::ScaleFactorChanged(..) => {}
                _ => {}
            },
            SctkEvent::PopupEvent {
                variant,
                id: surface,
                ..
            } => {
                match variant {
                    PopupEventVariant::Done => {
                        if let Some(e) =
                            surface_ids.remove(&surface.id()).map(|id| {
                                _ = window_manager.remove(id.inner());
                                (
                                    Some(id.inner()),
                                    iced_runtime::core::Event::PlatformSpecific(
                                        PlatformSpecific::Wayland(
                                            wayland::Event::Popup(
                                                PopupEvent::Done,
                                                surface,
                                                id.inner(),
                                            ),
                                        ),
                                    ),
                                )
                            })
                        {
                            events.push(e)
                        }
                    }
                    PopupEventVariant::Created(
                        surface,
                        surface_id,
                        common,
                        display,
                    ) => {
                        let wrapper = SurfaceIdWrapper::Popup(surface_id);
                        _ = surface_ids.insert(surface.id(), wrapper.clone());
                        let sctk_winit = SctkWinitWindow::new(
                            sctk_tx.clone(),
                            common,
                            wrapper,
                            surface,
                            display,
                        );
                        #[cfg(feature = "a11y")]
                        {
                            use crate::a11y::*;
                            use iced_accessibility::accesskit::{
                                ActivationHandler, NodeBuilder, NodeId, Role,
                                Tree, TreeUpdate,
                            };
                            use iced_accessibility::accesskit_winit::Adapter;

                            let node_id =
                                iced_runtime::core::id::window_node_id();

                            let activation_handler = WinitActivationHandler {
                                proxy: control_sender.clone(),
                                title: String::new(),
                            };

                            let action_handler = WinitActionHandler {
                                id: surface_id,
                                proxy: control_sender.clone(),
                            };

                            let deactivation_handler =
                                WinitDeactivationHandler {
                                    proxy: control_sender.clone(),
                                };
                            _ = adapters.insert(
                                surface_id,
                                (
                                    node_id,
                                    Adapter::with_direct_handlers(
                                        sctk_winit.as_ref(),
                                        activation_handler,
                                        action_handler,
                                        deactivation_handler,
                                    ),
                                ),
                            );
                        }

                        _ = window_manager.insert(
                            surface_id, sctk_winit, program, compositor,
                            false, // TODO do we want to get this value here?
                            0,
                        );
                    }
                    PopupEventVariant::Configure(_, _, _) => {} // TODO
                    PopupEventVariant::RepositionionedPopup { token: _ } => {}
                    PopupEventVariant::Size(_, _) => {}
                    PopupEventVariant::ScaleFactorChanged(..) => {}
                }
            }
            SctkEvent::NewOutput { id, info } => events.push((
                None,
                iced_runtime::core::Event::PlatformSpecific(
                    PlatformSpecific::Wayland(wayland::Event::Output(
                        wayland::OutputEvent::Created(info),
                        id,
                    )),
                ),
            )),
            SctkEvent::UpdateOutput { id, info } => events.push((
                None,
                iced_runtime::core::Event::PlatformSpecific(
                    PlatformSpecific::Wayland(wayland::Event::Output(
                        wayland::OutputEvent::InfoUpdate(info),
                        id,
                    )),
                ),
            )),
            SctkEvent::RemovedOutput(id) => events.push((
                None,
                iced_runtime::core::Event::PlatformSpecific(
                    PlatformSpecific::Wayland(wayland::Event::Output(
                        wayland::OutputEvent::Removed,
                        id,
                    )),
                ),
            )),
            SctkEvent::ScaleFactorChanged {
                factor: _,
                id: _,
                inner_size: _,
            } => Default::default(),
            SctkEvent::SessionLocked => events.push((
                None,
                iced_runtime::core::Event::PlatformSpecific(
                    PlatformSpecific::Wayland(wayland::Event::SessionLock(
                        wayland::SessionLockEvent::Locked,
                    )),
                ),
            )),
            SctkEvent::SessionLockFinished => events.push((
                None,
                iced_runtime::core::Event::PlatformSpecific(
                    PlatformSpecific::Wayland(wayland::Event::SessionLock(
                        wayland::SessionLockEvent::Finished,
                    )),
                ),
            )),
            SctkEvent::SessionLockSurfaceCreated {
                surface,
                native_id: surface_id,
                common,
                display,
            } => {
                let wrapper = SurfaceIdWrapper::SessionLock(surface_id.clone());
                _ = surface_ids.insert(surface.id().clone(), wrapper.clone());
                let sctk_winit = SctkWinitWindow::new(
                    sctk_tx.clone(),
                    common,
                    wrapper,
                    surface,
                    display,
                );

                #[cfg(feature = "a11y")]
                {
                    use crate::a11y::*;
                    use iced_accessibility::accesskit::{
                        ActivationHandler, NodeBuilder, NodeId, Role, Tree,
                        TreeUpdate,
                    };
                    use iced_accessibility::accesskit_winit::Adapter;

                    let node_id = iced_runtime::core::id::window_node_id();

                    let activation_handler = WinitActivationHandler {
                        proxy: control_sender.clone(),
                        // TODO lock screen title
                        title: String::new(),
                    };

                    let action_handler = WinitActionHandler {
                        id: surface_id,
                        proxy: control_sender.clone(),
                    };

                    let deactivation_handler = WinitDeactivationHandler {
                        proxy: control_sender.clone(),
                    };
                    _ = adapters.insert(
                        surface_id,
                        (
                            node_id,
                            Adapter::with_direct_handlers(
                                sctk_winit.as_ref(),
                                activation_handler,
                                action_handler,
                                deactivation_handler,
                            ),
                        ),
                    );
                }

                _ = window_manager.insert(
                    surface_id, sctk_winit, program, compositor,
                    false, // TODO do we want to get this value here?
                    0,
                );
            }
            SctkEvent::SessionLockSurfaceConfigure { .. } => {}
            SctkEvent::SessionLockSurfaceDone { surface } => {
                if let Some(id) = surface_ids.remove(&surface.id()) {
                    _ = window_manager.remove(id.inner());
                }
            }
            SctkEvent::SessionUnlocked => events.push((
                None,
                iced_runtime::core::Event::PlatformSpecific(
                    PlatformSpecific::Wayland(wayland::Event::SessionLock(
                        wayland::SessionLockEvent::Unlocked,
                    )),
                ),
            )),
            SctkEvent::Winit(_, _) => {}
            SctkEvent::SurfaceScaleFactorChanged(scale, _, id) => {
                if let Some(w) = window_manager.get_mut(id) {
                    w.state.update_scale_factor(scale);
                }
            }
            SctkEvent::Subcompositor(s) => {
                *subsurface_state = Some(s);
            }
        }
    }
}

fn keysym_to_vkey_location(keysym: Keysym) -> (Key, Location) {
    let raw = keysym.raw();
    let mut key = keysym_to_key(raw);
    if matches!(key, key::Key::Unidentified) {
        // XXX is there a better way to do this?
        // we need to be able to determine the actual character for the key
        // not the combination, so this seems to be correct
        let mut utf8 = xkbcommon::xkb::keysym_to_utf8(keysym);
        // remove null terminator
        _ = utf8.pop();
        if utf8.len() > 0 {
            key = Key::Character(utf8.into());
        }
    }

    let location = keymap::keysym_location(raw);
    (key, location)
}
