use crate::{
    platform_specific::{
        wayland::{
            handlers::{
                wp_fractional_scaling::FractionalScalingManager,
                wp_viewporter::ViewporterState,
            },
            sctk_event::{
                LayerSurfaceEventVariant, SctkEvent, WindowEventVariant
            },
        },
        Event,
    },
    program::Control,
};
use iced_futures::futures::channel::mpsc;
use std::{
    collections::{HashMap, HashSet},
    convert::Infallible,
    fmt::Debug,
    num::NonZeroU32, sync::{Mutex, Arc},
};
use wayland_backend::client::ObjectId;
use winit::dpi::{LogicalPosition, LogicalSize};

use iced_runtime::{
    core::{self, touch, Point},
    keyboard::Modifiers,
    platform_specific::{
        self,
        wayland::{
            layer_surface::{IcedMargin, IcedOutput, SctkLayerSurfaceSettings},
            popup::SctkPopupSettings,
            Action,
        },
    },
};
use sctk::{
    activation::ActivationState,
    compositor::CompositorState,
    error::GlobalError,
    output::OutputState,
    reexports::{
        calloop::LoopHandle,
        client::{
            delegate_noop,
            protocol::{
                wl_keyboard::WlKeyboard,
                wl_output::WlOutput,
                wl_region::WlRegion,
                wl_seat::WlSeat,
                wl_subsurface::WlSubsurface,
                wl_surface::{self, WlSurface},
                wl_touch::WlTouch,
            },
            Connection, Proxy, QueueHandle,
        },
    },
    registry::RegistryState,
    seat::{
        keyboard::KeyEvent,
        pointer::{CursorIcon, ThemedPointer},
        SeatState,
    },
    session_lock::{
        SessionLock, SessionLockState, SessionLockSurface,
        SessionLockSurfaceConfigure,
    },
    shell::{
        wlr_layer::{
            Anchor, KeyboardInteractivity, Layer, LayerShell, LayerSurface,
            LayerSurfaceConfigure,
        },
        xdg::{
            popup::{Popup, PopupConfigure},
            window::{Window, WindowConfigure},
            XdgPositioner, XdgShell, XdgSurface,
        },
        WaylandSurface,
    },
    shm::{multi::MultiPool, Shm},
};
use wayland_protocols::wp::{
    fractional_scale::v1::client::wp_fractional_scale_v1::WpFractionalScaleV1,
    viewporter::client::wp_viewport::WpViewport,
};


#[derive(Debug)]
pub(crate) struct SctkSeat {
    pub(crate) seat: WlSeat,
    pub(crate) kbd: Option<WlKeyboard>,
    pub(crate) kbd_focus: Option<WlSurface>,
    pub(crate) last_kbd_press: Option<(KeyEvent, u32)>,
    pub(crate) ptr: Option<ThemedPointer>,
    pub(crate) ptr_focus: Option<WlSurface>,
    pub(crate) last_ptr_press: Option<(u32, u32, u32)>, // (time, button, serial)
    pub(crate) touch: Option<WlTouch>,
    pub(crate) last_touch_down: Option<(u32, i32, u32)>, // (time, point, serial)
    pub(crate) _modifiers: Modifiers,
    // Cursor icon currently set (by CSDs, or application)
    pub(crate) active_icon: Option<CursorIcon>,
    // Cursor icon set by application
    pub(crate) icon: Option<CursorIcon>,
}

impl SctkSeat {
    pub(crate) fn set_cursor(&mut self, conn: &Connection, icon: CursorIcon) {
        if let Some(ptr) = self.ptr.as_ref() {
            ptr.set_cursor(conn, icon);
            self.active_icon = Some(icon);
        }
    }
}

#[derive(Debug, Clone)]
pub struct SctkWindow {
    pub(crate) id: core::window::Id,
    pub(crate) window: Window,
    pub(crate) scale_factor: Option<f64>,
    pub(crate) requested_size: Option<(NonZeroU32, NonZeroU32)>,
    pub(crate) current_size: (NonZeroU32, NonZeroU32),
    pub(crate) last_configure: Option<WindowConfigure>,
    pub(crate) resizable: Option<f64>,
    /// Requests that SCTK window should perform.
    pub(crate) _pending_requests:
        Vec<platform_specific::wayland::window::Action>,
    pub(crate) wp_fractional_scale: Option<WpFractionalScaleV1>,
    pub(crate) wp_viewport: Option<WpViewport>,
}

impl SctkWindow {
    pub(crate) fn set_size(&mut self, logical_size: LogicalSize<NonZeroU32>) {
        self.requested_size = Some((logical_size.width, logical_size.height));
        self.update_size((Some(logical_size.width), Some(logical_size.height)))
    }

    pub(crate) fn update_size(
        &mut self,
        (width, height): (Option<NonZeroU32>, Option<NonZeroU32>),
    ) {
        let (width, height) = (
            width.unwrap_or_else(|| self.current_size.0),
            height.unwrap_or_else(|| self.current_size.1),
        );
        if self.current_size == (width, height) {
            return;
        }
        self.window
            .set_window_geometry(0, 0, width.get(), height.get());
        self.current_size = (width, height);
        // Update the target viewport, this is used if and only if fractional scaling is in use.
        if let Some(viewport) = self.wp_viewport.as_ref() {
            // Set inner size without the borders.
            viewport.set_destination(width.get() as _, height.get() as _);
        }
    }
}

#[derive(Debug, Clone)]
pub struct SctkLayerSurface {
    pub(crate) id: core::window::Id,
    pub(crate) surface: LayerSurface,
    pub(crate) requested_size: (Option<u32>, Option<u32>),
    pub(crate) current_size: Option<LogicalSize<u32>>,
    pub(crate) layer: Layer,
    pub(crate) anchor: Anchor,
    pub(crate) keyboard_interactivity: KeyboardInteractivity,
    pub(crate) margin: IcedMargin,
    pub(crate) exclusive_zone: i32,
    pub(crate) last_configure: Option<LayerSurfaceConfigure>,
    pub(crate) _pending_requests:
        Vec<platform_specific::wayland::layer_surface::Action>,
    pub(crate) wp_fractional_scale: Option<WpFractionalScaleV1>,
    pub(crate) wp_viewport: Option<WpViewport>,
    pub(crate) common: Arc<Mutex<Common>>,
}

impl SctkLayerSurface {
    pub(crate) fn set_size(&mut self, w: Option<u32>, h: Option<u32>) {
        self.requested_size = (w, h);

        let (w, h) = (w.unwrap_or_default(), h.unwrap_or_default());
        self.surface.set_size(w, h);
    }

    pub(crate) fn update_viewport(&mut self, w: u32, h: u32) {
        self.current_size = Some(LogicalSize::new(w, h));
        if let Some(viewport) = self.wp_viewport.as_ref() {
            // Set inner size without the borders.
            viewport.set_destination(w as i32, h as i32);
        }
    }
}

#[derive(Debug, Clone)]
pub enum SctkSurface {
    LayerSurface(WlSurface),
    Window(WlSurface),
    Popup(WlSurface),
}

impl SctkSurface {
    pub fn wl_surface(&self) -> &WlSurface {
        match self {
            SctkSurface::LayerSurface(s)
            | SctkSurface::Window(s)
            | SctkSurface::Popup(s) => s,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Common {
    pub(crate) fractional_scale: Option<f64>,
    pub(crate) has_focus: bool,
    pub(crate) ime_pos: LogicalPosition<u32>,
    pub(crate) ime_size: LogicalSize<u32>,
    pub(crate) size: LogicalSize<u32>
}

impl Default for Common {
    fn default() -> Self {
        Self { fractional_scale: Default::default(), has_focus: Default::default(), ime_pos: Default::default(), ime_size: Default::default(), size: LogicalSize::new(1, 1) }
    }
}

impl From<LogicalSize<u32>> for Common {
    fn from(value: LogicalSize<u32>) -> Self {
        Common {
            size: value,
            ..Default::default()
        }
    }
}



#[derive(Debug)]
pub struct SctkPopup {
    pub(crate) popup: Popup,
    pub(crate) last_configure: Option<PopupConfigure>,
    pub(crate) _pending_requests:
        Vec<platform_specific::wayland::popup::Action>,
    pub(crate) data: SctkPopupData,
    pub(crate) common: Arc<Mutex<Common>>,
    pub(crate) wp_fractional_scale: Option<WpFractionalScaleV1>,
    pub(crate) wp_viewport: Option<WpViewport>,
}

impl SctkPopup {
    pub(crate) fn set_size(&mut self, w: u32, h: u32, token: u32) {
        // update geometry
        self.popup
            .xdg_surface()
            .set_window_geometry(0, 0, w as i32, h as i32);
        // update positioner
        self.data.positioner.set_size(w as i32, h as i32);
        self.popup.reposition(&self.data.positioner, token);
    }
}

#[derive(Debug)]
pub struct SctkLockSurface {
    pub(crate) id: core::window::Id,
    pub(crate) session_lock_surface: SessionLockSurface,
    pub(crate) last_configure: Option<SessionLockSurfaceConfigure>,
    pub(crate) wp_fractional_scale: Option<WpFractionalScaleV1>,
    pub(crate) wp_viewport: Option<WpViewport>,
    pub(crate) common: Arc<Mutex<Common>>,
}

#[derive(Debug)]
pub struct SctkPopupData {
    pub(crate) id: core::window::Id,
    pub(crate) parent: SctkSurface,
    pub(crate) toplevel: WlSurface,
    pub(crate) positioner: XdgPositioner,
}

/// Wrapper to carry sctk state.
pub struct SctkState {
    pub(crate) connection: Connection,

    /// the cursor wl_surface
    pub(crate) _cursor_surface: Option<wl_surface::WlSurface>,
    /// a memory pool
    pub(crate) _multipool: Option<MultiPool<WlSurface>>,

    // all present outputs
    pub(crate) outputs: Vec<WlOutput>,
    // though (for now) only one seat will be active in an iced application at a time, all ought to be tracked
    // Active seat is the first seat in the list
    pub(crate) seats: Vec<SctkSeat>,
    // Windows / Surfaces
    /// Window list containing all SCTK windows. Since those windows aren't allowed
    /// to be sent to other threads, they live on the event loop's thread
    /// and requests from winit's windows are being forwarded to them either via
    /// `WindowUpdate` or buffer on the associated with it `WindowHandle`.
    pub(crate) windows: Vec<SctkWindow>,
    pub(crate) layer_surfaces: Vec<SctkLayerSurface>,
    pub(crate) popups: Vec<SctkPopup>,
    pub(crate) lock_surfaces: Vec<SctkLockSurface>,
    pub(crate) _kbd_focus: Option<WlSurface>,
    pub(crate) touch_points: HashMap<touch::Finger, (WlSurface, Point)>,

    /// Window updates, which are coming from SCTK or the compositor, which require
    /// calling back to the sctk's downstream. They are handled right in the event loop,
    /// unlike the ones coming from buffers on the `WindowHandle`'s.
    pub compositor_updates: Vec<SctkEvent>,

    /// A sink for window and device events that is being filled during dispatching
    /// event loop and forwarded downstream afterwards.
    pub(crate) sctk_events: Vec<SctkEvent>,
    pub(crate) requested_frame: HashSet<ObjectId>,

    /// Send events to winit
    pub(crate) events_sender: mpsc::UnboundedSender<Control>,

    // handles
    pub(crate) queue_handle: QueueHandle<Self>,
    pub(crate) loop_handle: LoopHandle<'static, Self>,

    // sctk state objects
    /// Viewporter state on the given window.
    pub viewporter_state: Option<ViewporterState>,
    pub(crate) fractional_scaling_manager: Option<FractionalScalingManager>,
    pub(crate) registry_state: RegistryState,
    pub(crate) seat_state: SeatState,
    pub(crate) output_state: OutputState,
    pub(crate) compositor_state: CompositorState,
    pub(crate) shm_state: Shm,
    pub(crate) xdg_shell_state: XdgShell,
    pub(crate) layer_shell: Option<LayerShell>,
    pub(crate) activation_state: Option<ActivationState>,
    pub(crate) session_lock_state: SessionLockState,
    pub(crate) session_lock: Option<SessionLock>,
    pub(crate) token_ctr: u32,
    pub(crate) id_map: HashMap<ObjectId, core::window::Id>,
    pub(crate) to_commit: HashMap<core::window::Id, WlSurface>,

    pub(crate) ready: bool,
}

/// An error that occurred while running an application.
#[derive(Debug, thiserror::Error)]
pub enum PopupCreationError {
    /// Positioner creation failed
    #[error("Positioner creation failed")]
    PositionerCreationFailed(GlobalError),

    /// The specified parent is missing
    #[error("The specified parent is missing")]
    ParentMissing,

    /// The specified size is missing
    #[error("The specified size is missing")]
    SizeMissing,

    /// Popup creation failed
    #[error("Popup creation failed")]
    PopupCreationFailed(GlobalError),
}

/// An error that occurred while running an application.
#[derive(Debug, thiserror::Error)]
pub enum LayerSurfaceCreationError {
    /// Layer shell is not supported by the compositor
    #[error("Layer shell is not supported by the compositor")]
    LayerShellNotSupported,

    /// WlSurface creation failed
    #[error("WlSurface creation failed")]
    WlSurfaceCreationFailed(GlobalError),

    /// LayerSurface creation failed
    #[error("Layer Surface creation failed")]
    LayerSurfaceCreationFailed(GlobalError),
}

impl SctkState {
    pub fn scale_factor_changed(
        &mut self,
        surface: &WlSurface,
        scale_factor: f64,
        legacy: bool,
    ) {
        let mut id = None;
        if let Some(window) = self
            .windows
            .iter_mut()
            .find(|w| w.window.wl_surface() == surface)
        {
            id = Some(window.id);
            if legacy && window.wp_fractional_scale.is_some() {
                return;
            }
            window.scale_factor = Some(scale_factor);
            if legacy {
                let _ = window.window.set_buffer_scale(scale_factor as u32);
            }
        }

        if let Some(popup) = self
            .popups
            .iter_mut()
            .find(|p| p.popup.wl_surface() == surface)
        {
            id = Some(popup.data.id);
            if legacy && popup.wp_fractional_scale.is_some() {
                return;
            }
            let mut common = popup.common.lock().unwrap();
            common.fractional_scale = Some(scale_factor);
            if legacy {
                popup.popup.wl_surface().set_buffer_scale(scale_factor as _);
            }
        }

        if let Some(layer_surface) = self
            .layer_surfaces
            .iter_mut()
            .find(|l| l.surface.wl_surface() == surface)
        {
            id = Some(layer_surface.id);
            if legacy && layer_surface.wp_fractional_scale.is_some() {
                return;
            }
            let mut common = layer_surface.common.lock().unwrap();
            common.fractional_scale = Some(scale_factor);
            if legacy {
                let _ =
                    layer_surface.surface.wl_surface().set_buffer_scale(scale_factor as i32);
            }
            
        }

        if let Some(lock_surface) = self
            .lock_surfaces
            .iter_mut()
            .find(|l| l.session_lock_surface.wl_surface() == surface)
        {
            id = Some(lock_surface.id);
            if legacy && lock_surface.wp_fractional_scale.is_some() {
                return;
            }
            let mut common = lock_surface.common.lock().unwrap();
            common.fractional_scale = Some(scale_factor);
            if legacy {
                let _ =
                    lock_surface.session_lock_surface.wl_surface().set_buffer_scale(scale_factor as i32);
            }
        }

        if let Some(id) = id {
            self.sctk_events.push(SctkEvent::SurfaceScaleFactorChanged(scale_factor, surface.clone(), id));
        }

        // TODO winit sets cursor size after handling the change for the window, so maybe that should be done as well.
    }
}

impl SctkState {
    pub fn get_popup(
        &mut self,
        settings: SctkPopupSettings,
    ) -> Result<
        (core::window::Id, WlSurface, WlSurface, WlSurface, Arc<Mutex<Common>>),
        PopupCreationError,
    > {
        let (parent, toplevel) = if let Some(parent) =
            self.layer_surfaces.iter().find(|l| l.id == settings.parent)
        {
            (
                SctkSurface::LayerSurface(parent.surface.wl_surface().clone()),
                parent.surface.wl_surface().clone(),
            )
        } else if let Some(parent) =
            self.windows.iter().find(|w| w.id == settings.parent)
        {
            (
                SctkSurface::Window(parent.window.wl_surface().clone()),
                parent.window.wl_surface().clone(),
            )
        } else if let Some(i) = self
            .popups
            .iter()
            .position(|p| p.data.id == settings.parent)
        {
            let parent = &self.popups[i];
            (
                SctkSurface::Popup(parent.popup.wl_surface().clone()),
                parent.data.toplevel.clone(),
            )
        } else {
            return Err(PopupCreationError::ParentMissing);
        };

        let size = if settings.positioner.size.is_none() {
            return Err(PopupCreationError::SizeMissing);
        } else {
            settings.positioner.size.unwrap()
        };

        let positioner = XdgPositioner::new(&self.xdg_shell_state)
            .map_err(PopupCreationError::PositionerCreationFailed)?;
        positioner.set_anchor(settings.positioner.anchor);
        positioner.set_anchor_rect(
            settings.positioner.anchor_rect.x,
            settings.positioner.anchor_rect.y,
            settings.positioner.anchor_rect.width,
            settings.positioner.anchor_rect.height,
        );
        if let Ok(constraint_adjustment) =
            settings.positioner.constraint_adjustment.try_into()
        {
            positioner.set_constraint_adjustment(constraint_adjustment);
        }
        positioner.set_gravity(settings.positioner.gravity);
        positioner.set_offset(
            settings.positioner.offset.0,
            settings.positioner.offset.1,
        );
        if settings.positioner.reactive {
            positioner.set_reactive();
        }
        positioner.set_size(size.0 as i32, size.1 as i32);

        let grab = settings.grab;

        let wl_surface =
            self.compositor_state.create_surface(&self.queue_handle);
        _ = self.id_map.insert(wl_surface.id(), settings.id.clone());

        let (toplevel, popup) = match &parent {
            SctkSurface::LayerSurface(parent) => {
                let Some(parent_layer_surface) = self
                    .layer_surfaces
                    .iter()
                    .find(|w| w.surface.wl_surface() == parent)
                else {
                    return Err(PopupCreationError::ParentMissing);
                };
                let popup = Popup::from_surface(
                    None,
                    &positioner,
                    &self.queue_handle,
                    wl_surface.clone(),
                    &self.xdg_shell_state,
                )
                .map_err(PopupCreationError::PopupCreationFailed)?;
                parent_layer_surface.surface.get_popup(popup.xdg_popup());
                (parent_layer_surface.surface.wl_surface(), popup)
            }
            SctkSurface::Window(parent) => {
                let Some(parent_window) = self
                    .windows
                    .iter()
                    .find(|w| w.window.wl_surface() == parent)
                else {
                    return Err(PopupCreationError::ParentMissing);
                };
                (
                    parent_window.window.wl_surface(),
                    Popup::from_surface(
                        Some(parent_window.window.xdg_surface()),
                        &positioner,
                        &self.queue_handle,
                        wl_surface.clone(),
                        &self.xdg_shell_state,
                    )
                    .map_err(PopupCreationError::PopupCreationFailed)?,
                )
            }
            SctkSurface::Popup(parent) => {
                let Some(parent_xdg) = self.popups.iter().find_map(|p| {
                    (p.popup.wl_surface() == parent)
                        .then(|| p.popup.xdg_surface())
                }) else {
                    return Err(PopupCreationError::ParentMissing);
                };

                (
                    &toplevel,
                    Popup::from_surface(
                        Some(parent_xdg),
                        &positioner,
                        &self.queue_handle,
                        wl_surface.clone(),
                        &self.xdg_shell_state,
                    )
                    .map_err(PopupCreationError::PopupCreationFailed)?,
                )
            }
        };
        if grab {
            if let Some(s) = self.seats.first() {
                popup.xdg_popup().grab(
                    &s.seat,
                    s.last_ptr_press.map(|p| p.2).unwrap_or_else(|| {
                        s.last_kbd_press
                            .as_ref()
                            .map(|p| p.1)
                            .unwrap_or_default()
                    }),
                )
            }
        }
        wl_surface.commit();

        let wp_viewport = self.viewporter_state.as_ref().map(|state| {
            let viewport =
                state.get_viewport(popup.wl_surface(), &self.queue_handle);
            viewport.set_destination(size.0 as i32, size.1 as i32);
            viewport
        });
        let wp_fractional_scale =
            self.fractional_scaling_manager.as_ref().map(|fsm| {
                fsm.fractional_scaling(popup.wl_surface(), &self.queue_handle)
            });
            let common = Arc::new(Mutex::new(LogicalSize::new(size.0, size.1).into()));

        self.popups.push(SctkPopup {
            popup: popup.clone(),
            data: SctkPopupData {
                id: settings.id,
                parent: parent.clone(),
                toplevel: toplevel.clone(),
                positioner,
            },
            last_configure: None,
            _pending_requests: Default::default(),
            wp_viewport,
            wp_fractional_scale,
            common: common.clone()
        });

        Ok((
            settings.id,
            parent.wl_surface().clone(),
            toplevel.clone(),
            popup.wl_surface().clone(),
            common
        ))
    }

    pub fn get_layer_surface(
        &mut self,
        SctkLayerSurfaceSettings {
            id,
            layer,
            keyboard_interactivity,
            pointer_interactivity,
            anchor,
            output,
            namespace,
            margin,
            size,
            exclusive_zone,
            ..
        }: SctkLayerSurfaceSettings,
    ) -> Result<(core::window::Id, WlSurface, Arc<Mutex<Common>>), LayerSurfaceCreationError> {
        let wl_output = match output {
            IcedOutput::All => None, // TODO
            IcedOutput::Active => None,
            IcedOutput::Output(output) => Some(output),
        };

        let layer_shell = self
            .layer_shell
            .as_ref()
            .ok_or(LayerSurfaceCreationError::LayerShellNotSupported)?;
        let wl_surface =
            self.compositor_state.create_surface(&self.queue_handle);
        _ = self.id_map.insert(wl_surface.id(), id.clone());
        let mut size = size.unwrap_or((None, None));
        if anchor.contains(Anchor::BOTTOM.union(Anchor::TOP)) {
            size.1 = None;
        } else {
            size.1 = Some(size.1.unwrap_or(1).max(1));
        }
        if anchor.contains(Anchor::LEFT.union(Anchor::RIGHT)) {
            size.0 = None;
        } else {
            size.0 = Some(size.0.unwrap_or(1).max(1));
        }
        let layer_surface = layer_shell.create_layer_surface(
            &self.queue_handle,
            wl_surface.clone(),
            layer,
            Some(namespace),
            wl_output.as_ref(),
        );
        layer_surface.set_anchor(anchor);
        layer_surface.set_keyboard_interactivity(keyboard_interactivity);
        layer_surface.set_margin(
            margin.top,
            margin.right,
            margin.bottom,
            margin.left,
        );
        layer_surface
            .set_size(size.0.unwrap_or_default(), size.1.unwrap_or_default());
        layer_surface.set_exclusive_zone(exclusive_zone);
        if !pointer_interactivity {
            let region = self
                .compositor_state
                .wl_compositor()
                .create_region(&self.queue_handle, ());
            layer_surface.set_input_region(Some(&region));
            region.destroy();
        }
        layer_surface.commit();

        let wp_viewport = self.viewporter_state.as_ref().map(|state| {
            state.get_viewport(layer_surface.wl_surface(), &self.queue_handle)
        });
        let wp_fractional_scale =
            self.fractional_scaling_manager.as_ref().map(|fsm| {
                fsm.fractional_scaling(
                    layer_surface.wl_surface(),
                    &self.queue_handle,
                )
            });
        let common = Arc::new(Mutex::new(Common::from(LogicalSize::new(size.0.unwrap_or(1), size.1.unwrap_or(1)))));
        self.layer_surfaces.push(SctkLayerSurface {
            id,
            surface: layer_surface,
            requested_size: size,
            current_size: None,
            layer,
            // builder needs to be refactored such that these fields are accessible
            anchor,
            keyboard_interactivity,
            margin,
            exclusive_zone,
            last_configure: None,
            _pending_requests: Vec::new(),
            wp_viewport,
            wp_fractional_scale,
            common: common.clone()
        });
        Ok((id, wl_surface, common))
    }
    pub fn get_lock_surface(
        &mut self,
        id: core::window::Id,
        output: &WlOutput,
    ) -> Option<(WlSurface, Arc<Mutex<Common>>)> {
        if let Some(lock) = self.session_lock.as_ref() {
            let wl_surface =
                self.compositor_state.create_surface(&self.queue_handle);
            _ = self.id_map.insert(wl_surface.id(), id.clone());
            let session_lock_surface = lock.create_lock_surface(
                wl_surface.clone(),
                output,
                &self.queue_handle,
            );
            let wp_viewport = self.viewporter_state.as_ref().map(|state| {
                let viewport =
                    state.get_viewport(&wl_surface, &self.queue_handle);
                viewport
            });
            let wp_fractional_scale =
            self.fractional_scaling_manager.as_ref().map(|fsm| {
                fsm.fractional_scaling(&wl_surface, &self.queue_handle)
            });
            let common = Arc::new(Mutex::new(Common::from(
                LogicalSize::new(1, 1)
            )));
            self.lock_surfaces.push(SctkLockSurface {
                id,
                session_lock_surface,
                last_configure: None,
                wp_fractional_scale,
                wp_viewport,
                common: common.clone()
            });
            Some((wl_surface, common))
        } else {
            None
        }
    }

    pub(crate) fn handle_action(
        &mut self,
        action: iced_runtime::platform_specific::wayland::Action,
    ) -> Result<(), Infallible> {
        match action {
            Action::LayerSurface(action) => match action {
                        platform_specific::wayland::layer_surface::Action::LayerSurface {
                            builder,
                        } => {
                            let title = builder.namespace.clone();
                            if let Ok((id, wl_surface, common)) = self.get_layer_surface(builder) {
                                let object_id = wl_surface.id();
                                // TODO Ashley: all surfaces should probably have an optional title for a11y if nothing else
                                send_event(&self.events_sender,
                                    SctkEvent::LayerSurfaceEvent {
                                        variant: LayerSurfaceEventVariant::Created(wl_surface.clone(), id, common, self.connection.display(), title),
                                        id: wl_surface.clone(),
                                    }
                                );
                            }
                        }
                        platform_specific::wayland::layer_surface::Action::Size {
                            id,
                            width,
                            height,
                        } => {
                            if let Some(layer_surface) = self.layer_surfaces.iter_mut().find(|l| l.id == id) {
                                layer_surface.set_size(width, height);
                                // pending_redraws.push(layer_surface.surface.wl_surface().id());
                                    let wl_surface = layer_surface.surface.wl_surface();

                                if let Some(mut prev_configure) = layer_surface.last_configure.clone() {
                                    prev_configure.new_size = (width.unwrap_or(prev_configure.new_size.0), width.unwrap_or(prev_configure.new_size.1));
                                    _ = send_event(&self.events_sender,
                                        SctkEvent::LayerSurfaceEvent { variant: LayerSurfaceEventVariant::Configure(prev_configure, wl_surface.clone(), false), id: wl_surface.clone()});
                                    
                                }
                            }
                        },
                        platform_specific::wayland::layer_surface::Action::Destroy(id) => {
                            if let Some(i) = self.layer_surfaces.iter().position(|l| l.id == id) {
                                let l = self.layer_surfaces.remove(i);
                                send_event(&self.events_sender, SctkEvent::LayerSurfaceEvent {
                                            variant: LayerSurfaceEventVariant::Done,
                                            id: l.surface.wl_surface().clone(),
                                    }
                                );
                                
                            }
                        },
                        platform_specific::wayland::layer_surface::Action::Anchor { id, anchor } => {
                            if let Some(layer_surface) = self.layer_surfaces.iter_mut().find(|l| l.id == id) {
                                layer_surface.anchor = anchor;
                                layer_surface.surface.set_anchor(anchor);
                                _ = self.to_commit.insert(id, layer_surface.surface.wl_surface().clone());

                            }
                        }
                        platform_specific::wayland::layer_surface::Action::ExclusiveZone {
                            id,
                            exclusive_zone,
                        } => {
                            if let Some(layer_surface) = self.layer_surfaces.iter_mut().find(|l| l.id == id) {
                                layer_surface.exclusive_zone = exclusive_zone;
                                layer_surface.surface.set_exclusive_zone(exclusive_zone);
                                _ = self.to_commit.insert(id, layer_surface.surface.wl_surface().clone());
                            }
                        },
                        platform_specific::wayland::layer_surface::Action::Margin {
                            id,
                            margin,
                        } => {
                            if let Some(layer_surface) = self.layer_surfaces.iter_mut().find(|l| l.id == id) {
                                layer_surface.margin = margin;
                                layer_surface.surface.set_margin(margin.top, margin.right, margin.bottom, margin.left);
                                _ = self.to_commit.insert(id, layer_surface.surface.wl_surface().clone());
                            }
                        },
                        platform_specific::wayland::layer_surface::Action::KeyboardInteractivity { id, keyboard_interactivity } => {
                            if let Some(layer_surface) = self.layer_surfaces.iter_mut().find(|l| l.id == id) {
                                layer_surface.keyboard_interactivity = keyboard_interactivity;
                                layer_surface.surface.set_keyboard_interactivity(keyboard_interactivity);
                                _ = self.to_commit.insert(id, layer_surface.surface.wl_surface().clone());

                            }
                        },
                        platform_specific::wayland::layer_surface::Action::Layer { id, layer } => {
                            if let Some(layer_surface) = self.layer_surfaces.iter_mut().find(|l| l.id == id) {
                                layer_surface.layer = layer;
                                layer_surface.surface.set_layer(layer);
                                _ = self.to_commit.insert(id, layer_surface.surface.wl_surface().clone());

                            }
                        },
                },
            Action::Window(action) => match action {
                platform_specific::wayland::window::Action::Window { builder } => {
                    panic!("TODO remove this action");
                },
                platform_specific::wayland::window::Action::Size { id, width, height } => {
                    if let Some(window) = self.windows.iter_mut().find(|w| w.id == id) {
                        window.set_size(LogicalSize::new(NonZeroU32::new(width).unwrap_or(NonZeroU32::new(1).unwrap()), NonZeroU32::new(height).unwrap_or(NonZeroU32::new(1).unwrap())));
                        // TODO Ashley maybe don't force window size?
                        // pending_redraws.push(window.window.wl_surface().id());
                        if window.last_configure.is_some() {
                            _ = send_event(&self.events_sender,
                                SctkEvent::WindowEvent { variant: WindowEventVariant::Size(window.current_size, window.window.wl_surface().clone(), false), id: window.window.wl_surface().clone()}
                            );
                        }
                    }
                },
                platform_specific::wayland::window::Action::MinSize { id, size } => {
                    if let Some(window) = self.windows.iter_mut().find(|w| w.id == id) {
                        window.window.set_min_size(size);
                        _ = self.to_commit.insert(id, window.window.wl_surface().clone());
                    }
                },
                platform_specific::wayland::window::Action::MaxSize { id, size } => {
                    if let Some(window) = self.windows.iter_mut().find(|w| w.id == id) {
                        window.window.set_max_size(size);
                        _ = self.to_commit.insert(id, window.window.wl_surface().clone());
                    }
                },
                platform_specific::wayland::window::Action::Title { id, title } => {
                    if let Some(window) = self.windows.iter_mut().find(|w| w.id == id) {
                        window.window.set_title(title);
                        _ = self.to_commit.insert(id, window.window.wl_surface().clone());
                    }
                },
                platform_specific::wayland::window::Action::InteractiveMove { id } => {
                    if let (Some(window), Some((seat, last_press))) = (self.windows.iter_mut().find(|w| w.id == id), self.seats.first().and_then(|seat| seat.last_ptr_press.map(|p| (&seat.seat, p.2)))) {
                        window.window.xdg_toplevel()._move(seat, last_press);
                        _ = self.to_commit.insert(id, window.window.wl_surface().clone());
                    }
                },
                platform_specific::wayland::window::Action::InteractiveResize { id, edge } => {
                    if let (Some(window), Some((seat, last_press))) = (self.windows.iter_mut().find(|w| w.id == id), self.seats.first().and_then(|seat| seat.last_ptr_press.map(|p| (&seat.seat, p.2)))) {
                        window.window.xdg_toplevel().resize(seat, last_press, edge);
                        _ = self.to_commit.insert(id, window.window.wl_surface().clone());
                    }
                },
                // platform_specific::wayland::window::Action::ShowWindowMenu { id } => {
                //     if let (Some(window), Some((seat, last_press))) = (self.windows.iter_mut().find(|w| w.id == id), self.seats.first().and_then(|seat| seat.last_ptr_press.map(|p| (&seat.seat, p.2)))) {
                //         let surface_id = window.window.wl_surface().id();

                //         let cursor_position = cursor_position.get(&surface_id)
                //             .cloned()
                //             .unwrap_or_default();

                //         // Cursor position does not need to be scaled here.
                //         let PhysicalPosition { x, y } = cursor_position.to_physical::<i32>(1.0);

                //         window.window.xdg_toplevel().show_window_menu(seat, last_press, x as i32, y as i32);
                //         _ = self.to_commit.insert(id, window.window.wl_surface().clone());
                //     }
                // },
                platform_specific::wayland::window::Action::Destroy(id) => {
                    if let Some(i) = self.windows.iter().position(|l| l.id == id) {
                        let window = self.windows.remove(i);
                        window.window.xdg_toplevel().destroy();
                        // TODO should we remove this??
                        // sticky_exit_callback(
                        //     IcedSctkEvent::SctkEvent(SctkEvent::WindowEvent {
                        //         variant: WindowEventVariant::Close,
                        //         id: window.window.wl_surface().clone(),
                        //     }),
                        //     &self,
                        //     &mut control_flow,
                        //     &mut callback,
                        // );
                    }
                }, 
                platform_specific::wayland::window::Action::AppId { id, app_id } => {
                    if let Some(window) = self.windows.iter_mut().find(|w| w.id == id) {
                        window.window.set_app_id(app_id);
                        _ = self.to_commit.insert(id, window.window.wl_surface().clone());
                    }
                },
            },
            Action::Popup(action) => match action {
                platform_specific::wayland::popup::Action::Popup { popup, .. } => {
                    if let Ok((id, parent_id, toplevel_id, wl_surface, common)) = self.get_popup(popup) {
                        let object_id = wl_surface.id();
                        send_event(&self.events_sender, 
                            SctkEvent::PopupEvent {
                                variant: crate::platform_specific::wayland::sctk_event::PopupEventVariant::Created(wl_surface.clone(), id, common, self.connection.display()),
                                toplevel_id, parent_id, id: wl_surface.clone() });
                    }
                },
                // XXX popup destruction must be done carefully
                // first destroy the uppermost popup, then work down to the requested popup
                platform_specific::wayland::popup::Action::Destroy { id } => {
                    let sctk_popup = match self
                        .popups
                        .iter()
                        .position(|s| s.data.id == id)
                    {
                        Some(p) => self.popups.remove(p),
                        None => panic!("TODO return error..."),
                    };
                    let mut to_destroy = vec![sctk_popup];
                    while let Some(popup_to_destroy) = to_destroy.last() {
                        match popup_to_destroy.data.parent.clone() {
                            SctkSurface::LayerSurface(_) | SctkSurface::Window(_) => {
                                break;
                            }
                            SctkSurface::Popup(popup_to_destroy_first) => {
                                let popup_to_destroy_first = self
                                    .popups
                                    .iter()
                                    .position(|p| p.popup.wl_surface() == &popup_to_destroy_first)
                                    .unwrap();
                                let popup_to_destroy_first = self.popups.remove(popup_to_destroy_first);
                                to_destroy.push(popup_to_destroy_first);
                            }
                        }
                    }
                    for popup in to_destroy.into_iter().rev() {
                        // sticky_exit_callback(IcedSctkEvent::SctkEvent(SctkEvent::PopupEvent {
                        //     variant: PopupEventVariant::Done,
                        //     toplevel_id: popup.data.toplevel.clone(),
                        //     parent_id: popup.data.parent.wl_surface().clone(),
                        //     id: popup.popup.wl_surface().clone(),
                        // }),
                        //     &self,
                        //     &mut control_flow,
                        //     &mut callback,
                        // );
                    }
                },
                platform_specific::wayland::popup::Action::Size { id, width, height } => {
                    if let Some(sctk_popup) = self
                        .popups
                        .iter_mut()
                        .find(|s| s.data.id == id)
                    {
                        // update geometry
                        // update positioner
                        self.token_ctr += 1;
                        sctk_popup.set_size(width, height, self.token_ctr);

                        // self.pending_redraws.push(sctk_popup.popup.wl_surface().id());

                        // sticky_exit_callback(IcedSctkEvent::SctkEvent(SctkEvent::PopupEvent {
                        //     variant: PopupEventVariant::Size(width, height),
                        //     toplevel_id: sctk_popup.data.toplevel.clone(),
                        //     parent_id: sctk_popup.data.parent.wl_surface().clone(),
                        //     id: sctk_popup.popup.wl_surface().clone(),
                        // }),
                        //     &self,
                        //     &mut control_flow,
                        //     &mut callback,
                        // );
                    }
                },
                // TODO probably remove this?
                platform_specific::wayland::popup::Action::Grab { .. } => {},
            },
            Action::Activation(activation_event) => match activation_event {
                platform_specific::wayland::activation::Action::RequestToken { app_id, window, channel } => {
                    if let Some(activation_state) = self.activation_state.as_ref() {
                        let (seat_and_serial, surface) = if let Some(id) = window {
                            let surface = self.windows.iter().find(|w| w.id == id)
                                .map(|w| w.window.wl_surface().clone())
                                .or_else(|| self.layer_surfaces.iter().find(|l| l.id == id)
                                    .map(|l| l.surface.wl_surface().clone())
                                );
                            let seat_and_serial = surface.as_ref().and_then(|surface| {
                                self.seats.first().and_then(|seat| if seat.kbd_focus.as_ref().map(|focus| focus == surface).unwrap_or(false) {
                                    seat.last_kbd_press.as_ref().map(|(_, serial)| (seat.seat.clone(), *serial))
                                } else if seat.ptr_focus.as_ref().map(|focus| focus == surface).unwrap_or(false) {
                                    seat.last_ptr_press.as_ref().map(|(_, _, serial)| (seat.seat.clone(), *serial))
                                } else {
                                    None
                                })
                            });

                            (seat_and_serial, surface)
                        } else {
                            (None, None)
                        };

                        // TODO cleanup
                        // activation_state.request_token_with_data(&self.queue_handle, IcedRequestData::new(
                        //     RequestData {
                        //         app_id,
                        //         seat_and_serial,
                        //         surface,
                        //     },
                        //     message,
                        // ));
                    } else {
                        // if we don't have the global, we don't want to stall the app
                        // TODO cleanup
                        // sticky_exit_callback(
                        //     IcedSctkEvent::UserEvent(message(None)),
                        //     &self,
                        //     &mut control_flow,
                        //     &mut callback,
                        // )
                    }
                },
                platform_specific::wayland::activation::Action::Activate { window, token } => {
                    if let Some(activation_state) = self.activation_state.as_ref() {
                        if let Some(surface) = self.windows.iter().find(|w| w.id == window).map(|w| w.window.wl_surface()) {
                            activation_state.activate::<SctkState>(surface, token)
                        }
                    }
                },
            },
            Action::SessionLock(action) => match action {
                platform_specific::wayland::session_lock::Action::Lock => {
                    if self.session_lock.is_none() {
                        // TODO send message on error? When protocol doesn't exist.
                        self.session_lock = self.session_lock_state.lock(&self.queue_handle).ok();
                    }
                }
                platform_specific::wayland::session_lock::Action::Unlock => {
                    if let Some(session_lock) = self.session_lock.take() {
                        session_lock.unlock();
                    }
                    // Make sure server processes unlock before client exits
                    let _ = self.connection.roundtrip();
                    // sticky_exit_callback(
                    //     IcedSctkEvent::SctkEvent(SctkEvent::SessionUnlocked),
                    //     &self,
                    //     &mut control_flow,
                    //     &mut callback,
                    // );
                }
                platform_specific::wayland::session_lock::Action::LockSurface { id, output } => {
                    // TODO how to handle this when there's no lock?
                    if let Some(surface) = self.get_lock_surface(id, &output) {
                        // sticky_exit_callback(
                        //     IcedSctkEvent::SctkEvent(SctkEvent::SessionLockSurfaceCreated {surface, native_id: id}),
                        //     &self,
                        //     &mut control_flow,
                        //     &mut callback,
                        // );
                    }
                }
                platform_specific::wayland::session_lock::Action::DestroyLockSurface { id } => {
                    if let Some(i) =
                        self.lock_surfaces.iter().position(|s| {
                            s.id == id
                        })
                    {
                        let surface = self.lock_surfaces.remove(i);
                        // sticky_exit_callback(
                        //     IcedSctkEvent::SctkEvent(SctkEvent::SessionLockSurfaceDone {
                        //         surface: surface.session_lock_surface.wl_surface().clone()
                        //     }),
                        //     &self,
                        //     &mut control_flow,
                        //     &mut callback,
                        // );
                    }
                }
            }
        };
        Ok(())
    }
}

pub(crate) fn send_event(sender: &mpsc::UnboundedSender<Control>, sctk_event: SctkEvent) {
    _ = sender.unbounded_send(Control::PlatformSpecific(Event::Wayland(sctk_event)));
}

delegate_noop!(SctkState: ignore WlSubsurface);
delegate_noop!(SctkState: ignore WlRegion);
