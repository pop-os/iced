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

use dnd::DndSurface;
use iced_futures::{
    core::{
        event::{
            wayland::{
                LayerEvent, OverlapNotifyEvent, PopupEvent, SessionLockEvent,
            },
            PlatformSpecific,
        },
        Clipboard as _,
    },
    event,
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

use cctk::{
    cosmic_protocols::overlap_notify::v1::client::zcosmic_overlap_notification_v1,
    sctk::{
        output::OutputInfo,
        reexports::{
            calloop::channel,
            client::{
                backend::ObjectId,
                protocol::{
                    wl_display::WlDisplay, wl_keyboard::WlKeyboard,
                    wl_output::WlOutput, wl_pointer::WlPointer,
                    wl_seat::WlSeat, wl_surface::WlSurface, wl_touch::WlTouch,
                },
                Proxy, QueueHandle,
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
            wlr_layer::{Layer, LayerSurfaceConfigure},
            xdg::{popup::PopupConfigure, window::WindowConfigure},
        },
    },
};
use std::{
    collections::HashMap,
    num::NonZeroU32,
    sync::{Arc, Mutex},
};
use wayland_protocols::{
    ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1,
    wp::viewporter::client::wp_viewport::WpViewport,
};
use winit::{
    dpi::PhysicalSize, event::WindowEvent, event_loop::EventLoopProxy,
    window::WindowId,
};
use xkeysym::Keysym;

use super::{
    event_loop::state::{Common, CommonSurface, SctkState},
    keymap::raw_keycode_to_physicalkey,
    winit_window::SctkWinitWindow,
};

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
    OverlapToplevelAdd {
        surface: WlSurface,
        toplevel: ExtForeignToplevelHandleV1,
        logical_rect: iced_runtime::core::Rectangle,
    },
    OverlapToplevelRemove {
        surface: WlSurface,
        toplevel: ExtForeignToplevelHandleV1,
    },
    OverlapLayerAdd {
        surface: WlSurface,
        namespace: String,
        identifier: String,
        exclusive: u32,
        layer: Option<Layer>,
        logical_rect: iced_runtime::core::Rectangle,
    },
    OverlapLayerRemove {
        surface: WlSurface,
        identifier: String,
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
        queue_handle: QueueHandle<SctkState>,
        surface: CommonSurface,
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
    StateChanged(cctk::sctk::reexports::csd_frame::WindowState),
    /// Scale Factor
    ScaleFactorChanged(f64, Option<WpViewport>),
}

#[derive(Debug, Clone)]
pub enum PopupEventVariant {
    /// Popup Created
    Created(
        QueueHandle<SctkState>,
        CommonSurface,
        SurfaceId,
        Arc<Mutex<Common>>,
        WlDisplay,
    ),
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
    Created(
        QueueHandle<SctkState>,
        CommonSurface,
        SurfaceId,
        Arc<Mutex<Common>>,
        WlDisplay,
        String,
    ),
    /// <https://wayland.app/protocols/wlr-layer-shell-unstable-v1#zwlr_layer_surface_v1:event:closed>
    Done,
    /// <https://wayland.app/protocols/wlr-layer-shell-unstable-v1#zwlr_layer_surface_v1:event:configure>
    Configure(LayerSurfaceConfigure, WlSurface, bool),
    /// Scale Factor
    ScaleFactorChanged(f64, Option<WpViewport>),
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
        proxy: &EventLoopProxy,
        debug: &mut Debug,
        user_interfaces: &mut crate::platform_specific::UserInterfaces<'a, P>,
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
                    let physical_key = raw_keycode_to_physicalkey(ke.raw_code);
                    let physical_key =
                        crate::conversion::physical_key(physical_key);
                    events.push((
                        surface_ids.get(&surface.id()).map(|id| id.inner()),
                        iced_runtime::core::Event::Keyboard(
                            keyboard::Event::KeyReleased {
                                key: k.clone(),
                                location,
                                modifiers: modifiers_to_native(*modifiers),
                                modified_key: k,
                                physical_key: physical_key,
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
            SctkEvent::WindowEvent { .. } => {}
            SctkEvent::LayerSurfaceEvent {
                variant,
                id: surface,
            } => match variant {
                LayerSurfaceEventVariant::Done => {
                    if let Some(id) = surface_ids.remove(&surface.id()) {
                        if let Some(w) = window_manager.remove(id.inner()) {
                            if clipboard
                                .window_id()
                                .is_some_and(|id| w.raw.id() == id)
                            {
                                clipboard.register_dnd_destination(
                                    DndSurface(Arc::new(Box::new(
                                        w.raw.clone(),
                                    ))),
                                    Vec::new(),
                                );
                                *clipboard = Clipboard::unconnected();
                            }
                        }
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
                    queue_handle,
                    surface,
                    surface_id,
                    common,
                    display,
                    ..,
                ) => {
                    let wl_surface = surface.wl_surface();
                    let object_id = wl_surface.id();
                    let wrapper =
                        SurfaceIdWrapper::LayerSurface(surface_id.clone());
                    _ = surface_ids.insert(object_id.clone(), wrapper.clone());
                    let sctk_winit = SctkWinitWindow::new(
                        sctk_tx.clone(),
                        common,
                        wrapper,
                        surface,
                        display,
                        queue_handle,
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

                    if clipboard.window_id().is_none() {
                        *clipboard = Clipboard::connect(
                            window.raw.clone(),
                            crate::clipboard::ControlSender {
                                sender: control_sender.clone(),
                                proxy: proxy.clone(),
                            },
                        );
                    }

                    let mut ui = crate::program::build_user_interface(
                        program,
                        user_interface::Cache::default(),
                        &mut window.renderer,
                        logical_size,
                        debug,
                        surface_id,
                        window.raw.clone(),
                        window.prev_dnd_destination_rectangles_count,
                        clipboard,
                    );

                    _ = ui.update(
                        &vec![iced_runtime::core::Event::PlatformSpecific(
                            iced_runtime::core::event::PlatformSpecific::Wayland(
                                iced_runtime::core::event::wayland::Event::RequestResize,
                            ),
                        )],
                        window.state.cursor(),
                        &mut window.renderer,
                        clipboard,
                        &mut Vec::new(),
                    );

                    if let Some(requested_size) =
                        clipboard.requested_logical_size.lock().unwrap().take()
                    {
                        let requested_physical_size =
                            winit::dpi::PhysicalSize::new(
                                (requested_size.width as f64
                                    * window.state.scale_factor())
                                .ceil() as u32,
                                (requested_size.height as f64
                                    * window.state.scale_factor())
                                .ceil() as u32,
                            );
                        let physical_size = window.state.physical_size();
                        if requested_physical_size.width != physical_size.width
                            || requested_physical_size.height
                                != physical_size.height
                        {
                            // FIXME what to do when we are stuck in a configure event/resize request loop
                            // We don't have control over how winit handles this.
                            window.resize_enabled = true;

                            let s = winit::dpi::Size::Physical(
                                requested_physical_size,
                            );
                            _ = window.raw.request_surface_size(s);
                            window.raw.set_min_surface_size(Some(s));
                            window.raw.set_max_surface_size(Some(s));
                            window.state.synchronize(
                                &program,
                                surface_id,
                                window.raw.as_ref(),
                            );
                        }
                    }

                    let _ = user_interfaces.insert(surface_id, ui);
                }
                LayerSurfaceEventVariant::ScaleFactorChanged(..) => {}
                LayerSurfaceEventVariant::Configure(
                    configure,
                    surface,
                    first,
                ) => {
                    if let Some(w) = surface_ids
                        .get(&surface.id())
                        .and_then(|id| window_manager.get_mut(id.inner()))
                    {
                        let scale = w.state.scale_factor();
                        let p_w = (configure.new_size.0.max(1) as f64 * scale)
                            .ceil() as u32;
                        let p_h = (configure.new_size.1.max(1) as f64 * scale)
                            .ceil() as u32;
                        w.state.update(
                            w.raw.as_ref(),
                            &WindowEvent::SurfaceResized(PhysicalSize::new(
                                p_w, p_h,
                            )),
                            debug,
                        );
                    }
                }
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
                                if let Some(w) =
                                    window_manager.remove(id.inner())
                                {
                                    clipboard.register_dnd_destination(
                                        DndSurface(Arc::new(Box::new(
                                            w.raw.clone(),
                                        ))),
                                        Vec::new(),
                                    );
                                    if clipboard
                                        .window_id()
                                        .is_some_and(|id| w.raw.id() == id)
                                    {
                                        *clipboard = Clipboard::unconnected();
                                    }
                                }
                                _ = user_interfaces.remove(&id.inner());

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
                        queue_handle,
                        surface,
                        surface_id,
                        common,
                        display,
                    ) => {
                        let wl_surface = surface.wl_surface();
                        let wrapper = SurfaceIdWrapper::Popup(surface_id);
                        _ = surface_ids
                            .insert(wl_surface.id(), wrapper.clone());
                        let sctk_winit = SctkWinitWindow::new(
                            sctk_tx.clone(),
                            common,
                            wrapper,
                            surface,
                            display,
                            queue_handle,
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

                        if clipboard.window_id().is_none() {
                            *clipboard = Clipboard::connect(
                                sctk_winit.clone(),
                                crate::clipboard::ControlSender {
                                    sender: control_sender.clone(),
                                    proxy: proxy.clone(),
                                },
                            );
                        }

                        let window = window_manager.insert(
                            surface_id, sctk_winit, program, compositor,
                            false, // TODO do we want to get this value here?
                            0,
                        );
                        let logical_size = window.size();

                        let mut ui = crate::program::build_user_interface(
                            program,
                            user_interface::Cache::default(),
                            &mut window.renderer,
                            logical_size,
                            debug,
                            surface_id,
                            window.raw.clone(),
                            window.prev_dnd_destination_rectangles_count,
                            clipboard,
                        );

                        _ = ui.update(
                            &vec![iced_runtime::core::Event::PlatformSpecific(
                                iced_runtime::core::event::PlatformSpecific::Wayland(
                                    iced_runtime::core::event::wayland::Event::RequestResize,
                                ),
                            )],
                            window.state.cursor(),
                            &mut window.renderer,
                            clipboard,
                            &mut Vec::new(),
                        );

                        if let Some(requested_size) = clipboard
                            .requested_logical_size
                            .lock()
                            .unwrap()
                            .take()
                        {
                            let requested_physical_size =
                                winit::dpi::PhysicalSize::new(
                                    (requested_size.width as f64
                                        * window.state.scale_factor())
                                    .ceil()
                                        as u32,
                                    (requested_size.height as f64
                                        * window.state.scale_factor())
                                    .ceil()
                                        as u32,
                                );
                            let physical_size = window.state.physical_size();
                            if requested_physical_size.width
                                != physical_size.width
                                || requested_physical_size.height
                                    != physical_size.height
                            {
                                // FIXME what to do when we are stuck in a configure event/resize request loop
                                // We don't have control over how winit handles this.
                                window.resize_enabled = true;

                                let s = winit::dpi::Size::Physical(
                                    requested_physical_size,
                                );
                                _ = window.raw.request_surface_size(s);
                                window.raw.set_min_surface_size(Some(s));
                                window.raw.set_max_surface_size(Some(s));
                                window.state.synchronize(
                                    &program,
                                    surface_id,
                                    window.raw.as_ref(),
                                );
                            }
                        }

                        let _ = user_interfaces.insert(surface_id, ui);
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
                queue_handle,
                surface,
                native_id: surface_id,
                common,
                display,
            } => {
                let wl_surface = surface.wl_surface();
                let object_id = wl_surface.id().clone();
                let wrapper = SurfaceIdWrapper::SessionLock(surface_id.clone());
                _ = surface_ids.insert(object_id.clone(), wrapper.clone());
                let sctk_winit = SctkWinitWindow::new(
                    sctk_tx.clone(),
                    common,
                    wrapper,
                    surface,
                    display,
                    queue_handle,
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

                if clipboard.window_id().is_none() {
                    *clipboard = Clipboard::connect(
                        sctk_winit.clone(),
                        crate::clipboard::ControlSender {
                            sender: control_sender.clone(),
                            proxy: proxy.clone(),
                        },
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
            SctkEvent::OverlapToplevelAdd {
                surface,
                toplevel,
                logical_rect,
            } => {
                if let Some(id) = surface_ids.get(&surface.id()) {
                    events.push((
                        Some(id.inner()),
                        iced_runtime::core::Event::PlatformSpecific(
                            PlatformSpecific::Wayland(
                                wayland::Event::OverlapNotify(
                                    OverlapNotifyEvent::OverlapToplevelAdd {
                                        toplevel,
                                        logical_rect,
                                    },
                                ),
                            ),
                        ),
                    ))
                }
            }
            SctkEvent::OverlapToplevelRemove { surface, toplevel } => {
                if let Some(id) = surface_ids.get(&surface.id()) {
                    events.push((
                        Some(id.inner()),
                        iced_runtime::core::Event::PlatformSpecific(
                            PlatformSpecific::Wayland(
                                wayland::Event::OverlapNotify(
                                    OverlapNotifyEvent::OverlapToplevelRemove {
                                        toplevel,
                                    },
                                ),
                            ),
                        ),
                    ))
                }
            }
            SctkEvent::OverlapLayerAdd {
                surface,
                namespace,
                identifier,
                exclusive,
                layer,
                logical_rect,
            } => {
                if let Some(id) = surface_ids.get(&surface.id()) {
                    events.push((
                        Some(id.inner()),
                        iced_runtime::core::Event::PlatformSpecific(
                            PlatformSpecific::Wayland(
                                wayland::Event::OverlapNotify(
                                    OverlapNotifyEvent::OverlapLayerAdd {
                                        identifier,
                                        namespace,
                                        exclusive,
                                        layer,
                                        logical_rect,
                                    },
                                ),
                            ),
                        ),
                    ))
                }
            }
            SctkEvent::OverlapLayerRemove {
                surface,
                identifier,
            } => {
                if let Some(id) = surface_ids.get(&surface.id()) {
                    events.push((
                        Some(id.inner()),
                        iced_runtime::core::Event::PlatformSpecific(
                            PlatformSpecific::Wayland(
                                wayland::Event::OverlapNotify(
                                    OverlapNotifyEvent::OverlapLayerRemove {
                                        identifier,
                                    },
                                ),
                            ),
                        ),
                    ))
                }
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
