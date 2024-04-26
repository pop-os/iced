#[cfg(feature = "a11y")]
use crate::sctk_event::ActionRequestEvent;
use crate::{
    clipboard::Clipboard,
    commands::{layer_surface::get_layer_surface, window::get_window},
    dpi::{LogicalPosition, PhysicalPosition},
    error::{self, Error},
    event_loop::{
        control_flow::ControlFlow, proxy, state::SctkState, SctkEventLoop,
    },
    sctk_event::{
        DataSourceEvent, IcedSctkEvent, KeyboardEventVariant,
        LayerSurfaceEventVariant, PopupEventVariant, SctkEvent, StartCause,
    },
    settings,
};
use float_cmp::{approx_eq, F32Margin, F64Margin};
use futures::{channel::mpsc, task, Future, FutureExt, StreamExt};
#[cfg(feature = "a11y")]
use iced_accessibility::{
    accesskit::{NodeBuilder, NodeId},
    A11yId, A11yNode,
};
use iced_futures::{
    core::{
        event::{Event as CoreEvent, Status},
        layout::Limits,
        mouse,
        renderer::Style,
        time::Instant,
        widget::{
            operation::{self, OperationWrapper},
            tree, Operation, Tree,
        },
        Widget,
    },
    Executor, Runtime, Subscription,
};
use tracing::error;

use iced_futures::core::Clipboard as IcedClipboard;
use iced_graphics::{compositor, Compositor, Viewport};
use iced_runtime::{
    clipboard,
    command::{
        self,
        platform_specific::{
            self,
            wayland::{data_device::DndIcon, popup},
        },
    },
    core::{mouse::Interaction, touch, Color, Point, Size},
    multi_window::Program,
    system, user_interface,
    window::Id as SurfaceId,
    Command, Debug, UserInterface,
};
use iced_style::application::{self, StyleSheet};
use itertools::Itertools;
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle,
    RawDisplayHandle, RawWindowHandle, WaylandDisplayHandle,
    WaylandWindowHandle, WindowHandle,
};
use sctk::{
    reexports::client::{protocol::wl_surface::WlSurface, Proxy, QueueHandle},
    seat::{keyboard::Modifiers, pointer::PointerEventKind},
};
use std::mem::ManuallyDrop;
use std::{
    collections::HashMap, hash::Hash, marker::PhantomData, os::raw::c_void,
    ptr::NonNull, time::Duration,
};
use wayland_backend::client::ObjectId;
use wayland_protocols::wp::viewporter::client::wp_viewport::WpViewport;
use window_clipboard::mime::ClipboardStoreData;

use crate::subsurface_widget::{SubsurfaceInstance, SubsurfaceState};

pub enum Event<Message> {
    /// A normal sctk event
    SctkEvent(IcedSctkEvent<Message>),
    /// TODO
    // (maybe we should also allow users to listen/react to those internal messages?)

    /// layer surface requests from the client
    LayerSurface(platform_specific::wayland::layer_surface::Action<Message>),
    /// window requests from the client
    Window(platform_specific::wayland::window::Action<Message>),
    /// popup requests from the client
    Popup(platform_specific::wayland::popup::Action<Message>),
    /// data device requests from the client
    DataDevice(platform_specific::wayland::data_device::Action<Message>),
    /// xdg-activation request from the client
    Activation(platform_specific::wayland::activation::Action<Message>),
    /// data session lock requests from the client
    SessionLock(platform_specific::wayland::session_lock::Action<Message>),
    /// request sctk to set the cursor of the active pointer
    SetCursor(Interaction),
    /// Application Message
    Message(Message),
}

pub struct IcedSctkState;

#[derive(Debug, Clone)]
pub struct SurfaceDisplayWrapper {
    backend: wayland_backend::client::Backend,
    wl_surface: WlSurface,
}

impl HasDisplayHandle for SurfaceDisplayWrapper {
    fn display_handle(&self) -> Result<DisplayHandle, HandleError> {
        let ptr = self.backend.display_ptr() as *mut c_void;
        let Some(ptr) = NonNull::new(ptr) else {
            return Err(HandleError::Unavailable);
        };
        let display_handle = WaylandDisplayHandle::new(ptr);
        Ok(unsafe {
            DisplayHandle::borrow_raw(RawDisplayHandle::Wayland(display_handle))
        })
    }
}

impl HasWindowHandle for SurfaceDisplayWrapper {
    fn window_handle(&self) -> Result<WindowHandle, HandleError> {
        let ptr = self.wl_surface.id().as_ptr() as *mut c_void;
        let Some(ptr) = NonNull::new(ptr) else {
            return Err(HandleError::Unavailable);
        };
        let window_handle = WaylandWindowHandle::new(ptr);
        Ok(unsafe {
            WindowHandle::borrow_raw(RawWindowHandle::Wayland(window_handle))
        })
    }
}

/// An interactive, native, cross-platform, multi-windowed application.
///
/// This trait is the main entrypoint of multi-window Iced. Once implemented, you can run
/// your GUI application by simply calling [`run`]. It will run in
/// its own window.
///
/// An [`Application`] can execute asynchronous actions by returning a
/// [`Command`] in some of its methods.
///
/// When using an [`Application`] with the `debug` feature enabled, a debug view
/// can be toggled by pressing `F12`.
pub trait Application: Program
where
    Self::Theme: StyleSheet,
{
    /// The data needed to initialize your [`Application`].
    type Flags;

    /// Initializes the [`Application`] with the flags provided to
    /// [`run`] as part of the [`Settings`].
    ///
    /// Here is where you should return the initial state of your app.
    ///
    /// Additionally, you can return a [`Command`] if you need to perform some
    /// async action in the background on startup. This is useful if you want to
    /// load state from a file, perform an initial HTTP request, etc.
    fn new(flags: Self::Flags) -> (Self, Command<Self::Message>);

    /// Returns the current title of the [`Application`].
    ///
    /// This title can be dynamic! The runtime will automatically update the
    /// title of your application when necessary.
    fn title(&self, window: SurfaceId) -> String;

    /// Returns the current `Theme` of the [`Application`].
    fn theme(&self, window: SurfaceId) -> Self::Theme;

    /// Returns the `Style` variation of the `Theme`.
    fn style(&self) -> <Self::Theme as StyleSheet>::Style {
        Default::default()
    }

    /// Returns the event `Subscription` for the current state of the
    /// application.
    ///
    /// The messages produced by the `Subscription` will be handled by
    /// [`update`](#tymethod.update).
    ///
    /// A `Subscription` will be kept alive as long as you keep returning it!
    ///
    /// By default, it returns an empty subscription.
    fn subscription(&self) -> Subscription<Self::Message> {
        Subscription::none()
    }

    /// Returns the scale factor of the window of the [`Application`].
    ///
    /// It can be used to dynamically control the size of the UI at runtime
    /// (i.e. zooming).
    ///
    /// For instance, a scale factor of `2.0` will make widgets twice as big,
    /// while a scale factor of `0.5` will shrink them to half their size.
    ///
    /// By default, it returns `1.0`.
    #[allow(unused_variables)]
    fn scale_factor(&self, window: SurfaceId) -> f64 {
        1.0
    }
}

/// Runs an [`Application`] with an executor, compositor, and the provided
/// settings.
pub fn run<A, E, C>(
    settings: settings::Settings<A::Flags>,
    compositor_settings: C::Settings,
) -> Result<(), error::Error>
where
    A: Application + 'static,
    E: Executor + 'static,
    C: Compositor<Renderer = A::Renderer> + 'static,
    <A as Program>::Theme: StyleSheet,
{
    let mut debug = Debug::new();
    debug.startup_started();

    let exit_on_close_request = settings.exit_on_close_request;

    let mut event_loop = SctkEventLoop::<A::Message>::new(&settings)
        .expect("Failed to initialize the event loop");

    let (runtime, ev_proxy) = {
        let ev_proxy = event_loop.proxy();
        let executor = E::new().map_err(Error::ExecutorCreationFailed)?;

        (Runtime::new(executor, ev_proxy.clone()), ev_proxy)
    };

    let (application, init_command) = {
        let flags = settings.flags;

        runtime.enter(|| A::new(flags))
    };

    let init_command = match settings.surface {
        settings::InitialSurface::LayerSurface(b) => {
            Command::batch(vec![init_command, get_layer_surface(b)])
        }
        settings::InitialSurface::XdgWindow(b) => {
            Command::batch(vec![init_command, get_window(b)])
        }
        settings::InitialSurface::None => init_command,
    };
    let wl_surface = event_loop
        .state
        .compositor_state
        .create_surface(&event_loop.state.queue_handle);

    // let (display, context, config, surface) = init_egl(&wl_surface, 100, 100);
    let backend = event_loop
        .wayland_dispatcher
        .as_source_ref()
        .connection()
        .backend();
    let qh = event_loop.state.queue_handle.clone();
    let wrapper = SurfaceDisplayWrapper {
        backend: backend.clone(),
        wl_surface,
    };

    #[allow(unsafe_code)]
    let compositor = C::new(compositor_settings, wrapper.clone()).unwrap();
    let renderer = compositor.create_renderer();

    let auto_size_surfaces = HashMap::new();

    let surface_ids = Default::default();
    let subsurface_ids = Default::default();

    let (mut sender, receiver) = mpsc::unbounded::<IcedSctkEvent<A::Message>>();
    let (control_sender, mut control_receiver) = mpsc::unbounded();

    let mut instance = Box::pin(run_instance::<A, E, C>(
        application,
        compositor,
        renderer,
        runtime,
        ev_proxy,
        debug,
        receiver,
        control_sender,
        surface_ids,
        subsurface_ids,
        auto_size_surfaces,
        // display,
        // context,
        // config,
        backend,
        init_command,
        exit_on_close_request,
        qh,
        settings.control_flow_timeout,
    ));

    let mut context = task::Context::from_waker(task::noop_waker_ref());

    let _ = event_loop.run_return(move |event, _, control_flow| {
        if let ControlFlow::ExitWithCode(_) = control_flow {
            return;
        }

        sender.start_send(event).expect("Failed to send event");

        let poll = instance.as_mut().poll(&mut context);

        match poll {
            task::Poll::Pending => {
                if let Ok(Some(flow)) = control_receiver.try_next() {
                    *control_flow = flow
                }
            }
            task::Poll::Ready(_) => {
                *control_flow = ControlFlow::ExitWithCode(1)
            }
        };
    });

    Ok(())
}

fn subscription_map<A, E, C>(e: A::Message) -> Event<A::Message>
where
    A: Application + 'static,
    E: Executor + 'static,
    C: Compositor<Renderer = A::Renderer> + 'static,
    <A as Program>::Theme: StyleSheet,
{
    Event::SctkEvent(IcedSctkEvent::UserEvent(e))
}

// XXX Ashley careful, A, E, C must be exact same as in update, or the subscription map type will have a different hash
async fn run_instance<A, E, C>(
    mut application: A,
    mut compositor: C,
    mut renderer: A::Renderer,
    mut runtime: Runtime<E, proxy::Proxy<Event<A::Message>>, Event<A::Message>>,
    mut ev_proxy: proxy::Proxy<Event<A::Message>>,
    mut debug: Debug,
    mut receiver: mpsc::UnboundedReceiver<IcedSctkEvent<A::Message>>,
    mut control_sender: mpsc::UnboundedSender<ControlFlow>,
    mut surface_ids: HashMap<ObjectId, SurfaceIdWrapper>,
    mut subsurface_ids: HashMap<ObjectId, (i32, i32, SurfaceIdWrapper)>,
    mut auto_size_surfaces: HashMap<SurfaceIdWrapper, (u32, u32, Limits, bool)>,
    backend: wayland_backend::client::Backend,
    init_command: Command<A::Message>,
    exit_on_close_request: bool,
    queue_handle: QueueHandle<SctkState<<A as Program>::Message>>,
    wait: Option<Duration>,
) -> Result<(), Error>
where
    A: Application + 'static,
    E: Executor + 'static,
    C: Compositor<Renderer = A::Renderer> + 'static,
    <A as Program>::Theme: StyleSheet,
{
    let mut cache = user_interface::Cache::default();

    let mut states: HashMap<SurfaceId, State<A, C>> = HashMap::new();
    let mut interfaces = ManuallyDrop::new(HashMap::new());
    let mut simple_clipboard = Clipboard::unconnected();

    let mut subsurface_state = None::<SubsurfaceState<A::Message>>;

    {
        run_command(
            &application,
            &mut cache,
            None::<&State<A, C>>,
            &mut renderer,
            init_command,
            &mut runtime,
            &mut ev_proxy,
            &mut debug,
            || compositor.fetch_information(),
            &mut auto_size_surfaces,
            &mut Vec::new(),
            &mut simple_clipboard,
        );
    }
    runtime.track(
        application
            .subscription()
            .map(subscription_map::<A, E, C>)
            .into_recipes(),
    );

    let mut mouse_interaction = Interaction::default();
    let mut sctk_events: Vec<SctkEvent> = Vec::new();
    #[cfg(feature = "a11y")]
    let mut a11y_events: Vec<crate::sctk_event::ActionRequestEvent> =
        Vec::new();
    #[cfg(feature = "a11y")]
    let mut a11y_enabled = false;
    #[cfg(feature = "a11y")]
    let mut adapters: HashMap<
        SurfaceId,
        crate::event_loop::adapter::IcedSctkAdapter,
    > = HashMap::new();

    let mut messages: Vec<A::Message> = Vec::new();
    #[cfg(feature = "a11y")]
    let mut commands: Vec<Command<A::Message>> = Vec::new();
    let mut redraw_pending = false;

    debug.startup_finished();

    // let mut current_context_window = init_id_inner;

    let mut kbd_surface_id: Option<ObjectId> = None;
    let mut mods: Modifiers = Modifiers::default();
    let mut destroyed_surface_ids: HashMap<ObjectId, SurfaceIdWrapper> =
        Default::default();

    'main: while let Some(event) = receiver.next().await {
        match event {
            IcedSctkEvent::NewEvents(start_cause) => {
                redraw_pending = matches!(
                    start_cause,
                    StartCause::Init
                        | StartCause::Poll
                        | StartCause::ResumeTimeReached { .. }
                );
            }
            IcedSctkEvent::UserEvent(message) => {
                messages.push(message);
            }
            IcedSctkEvent::SctkEvent(event) => {
                sctk_events.push(event.clone());
                match event {
                    SctkEvent::SeatEvent { .. } => {} // TODO Ashley: handle later possibly if multiseat support is wanted
                    SctkEvent::PointerEvent {
                        variant,
                        ..
                    } => {
                        let mut offset = (0., 0.);
                        let (state, _native_id) = match surface_ids
                            .get(&variant.surface.id())
                            .and_then(|id| states.get_mut(&id.inner()).map(|state| (state, id)))
                        {
                            Some(s) => s,
                            None => {
                                if let Some((x_offset, y_offset, id)) = subsurface_ids.get(&variant.surface.id()) {
                                    offset = (f64::from(*x_offset), f64::from(*y_offset));
                                    states.get_mut(&id.inner()).map(|state| (state, id)).unwrap()
                                } else {
                                    continue
                                }
                            },
                        };
                        match variant.kind {
                            PointerEventKind::Enter { .. } => {
                                state.set_cursor_position(Some(LogicalPosition { x: variant.position.0 + offset.0, y: variant.position.1 + offset.1 }));
                            }
                            PointerEventKind::Leave { .. } => {
                                state.set_cursor_position(None);
                            }
                            PointerEventKind::Motion { .. } => {
                                state.set_cursor_position(Some(LogicalPosition { x: variant.position.0 + offset.0, y: variant.position.1 + offset.1 }));
                            }
                            PointerEventKind::Press { .. }
                            | PointerEventKind::Release { .. }
                            | PointerEventKind::Axis { .. } => {}
                        }
                    }
                    SctkEvent::KeyboardEvent { variant, .. } => match variant {
                        KeyboardEventVariant::Leave(_) => {
                            kbd_surface_id.take();
                        }
                        KeyboardEventVariant::Enter(object_id) => {
                            kbd_surface_id.replace(object_id.id());
                        }
                        KeyboardEventVariant::Press(_)
                        | KeyboardEventVariant::Release(_)
                        | KeyboardEventVariant::Repeat(_) => {}
                        KeyboardEventVariant::Modifiers(mods) => {
                            if let Some(state) = kbd_surface_id
                                .as_ref()
                                .and_then(|id| surface_ids.get(id))
                                .and_then(|id| states.get_mut(&id.inner()))
                            {
                                state.modifiers = mods;
                            }
                        }
                    },
                    SctkEvent::TouchEvent { variant, surface, .. } => {
                        let mut offset = (0., 0.);
                        let (state, _native_id) = match surface_ids
                            .get(&surface.id())
                            .and_then(|id| states.get_mut(&id.inner()).map(|state| (state, id)))
                        {
                            Some(s) => s,
                            None => {
                                if let Some((x_offset, y_offset, id)) = subsurface_ids.get(&surface.id()) {
                                    offset = (f64::from(*x_offset), f64::from(*y_offset));
                                    states.get_mut(&id.inner()).map(|state| (state, id)).unwrap()
                                } else {
                                    continue
                                }
                            },
                        };
                        let position = match variant {
                            touch::Event::FingerPressed { position, .. } => position,
                            touch::Event::FingerLifted { position, .. } => position,
                            touch::Event::FingerMoved { position, .. } => position,
                            touch::Event::FingerLost { position, .. } => position,
                        };
                        state.set_cursor_position(Some(LogicalPosition { x: position.x as f64 + offset.0, y: position.y as f64 + offset.1 }));
                    },
                    SctkEvent::WindowEvent { variant, id: wl_surface } => match variant {
                        crate::sctk_event::WindowEventVariant::Created(id, native_id) => {
                            surface_ids.insert(id, SurfaceIdWrapper::Window(native_id));
                            states.insert(native_id, State::new(&application, SurfaceIdWrapper::Window(native_id), SurfaceDisplayWrapper {
                                backend: backend.clone(),
                                wl_surface
                            }));
                        }
                        crate::sctk_event::WindowEventVariant::Close => {
                            if let Some(surface_id) = surface_ids.remove(&wl_surface.id()) {
                                // drop(compositor_surfaces.remove(&surface_id.inner()));
                                auto_size_surfaces.remove(&surface_id);
                                interfaces.remove(&surface_id.inner());
                                states.remove(&surface_id.inner());
                                destroyed_surface_ids.insert(wl_surface.id(), surface_id);
                                if exit_on_close_request && states.is_empty() {
                                    break 'main;
                                }
                            }
                        }
                        crate::sctk_event::WindowEventVariant::WmCapabilities(_)
                        | crate::sctk_event::WindowEventVariant::ConfigureBounds { .. } => {}
                        crate::sctk_event::WindowEventVariant::Configure(
                            configure,
                            wl_surface,
                            first,
                        ) => {
                            if let Some(id) = surface_ids.get(&wl_surface.id()) {
                                let Some(state) = states.get_mut(&id.inner()) else {
                                    continue;
                                };
                                if state.surface.is_none() {
                                    let wrapper = SurfaceDisplayWrapper {
                                        backend: backend.clone(),
                                        wl_surface
                                    };
                                    if matches!(simple_clipboard.state(),  crate::clipboard::State::Unavailable) {
                                        if let Ok(h) = wrapper.display_handle() {
                                            simple_clipboard = unsafe {Clipboard::connect(&h)};
                                        }
                                    }
                                    let mut c_surface = compositor.create_surface(wrapper.clone(), configure.new_size.0.unwrap().get(), configure.new_size.1.unwrap().get());
                                    compositor.configure_surface(&mut c_surface, configure.new_size.0.unwrap().get(), configure.new_size.1.unwrap().get());
                                    state.surface = Some(c_surface);
                                }
                                if let Some((w, h, _, is_dirty)) = auto_size_surfaces.get_mut(id) {
                                    *is_dirty = first || *w != configure.new_size.0.map(|w| w.get()).unwrap_or_default() || *h != configure.new_size.1.map(|h| h.get()).unwrap_or_default();
                                    state.set_logical_size(*w as f32, *h as f32);
                                } else {
                                    state.set_logical_size(configure.new_size.0.unwrap().get() as f32 , configure.new_size.1.unwrap().get() as f32);
                                }
                                if first {
                                    let user_interface = build_user_interface(
                                        &application,
                                        user_interface::Cache::default(),
                                        &mut renderer,
                                        state.logical_size(),
                                        &state.title,
                                        &mut debug,
                                        *id,
                                        &mut auto_size_surfaces,
                                        &mut ev_proxy
                                    );
                                    interfaces.insert(id.inner(), user_interface);
                                }
                            }
                        }
                        crate::sctk_event::WindowEventVariant::ScaleFactorChanged(sf, viewport) => {
                            if let Some(state) = surface_ids
                                .get(&wl_surface.id())
                                .and_then(|id| states.get_mut(&id.inner()))
                            {
                                state.wp_viewport = viewport;
                                state.set_scale_factor(sf);
                            }
                        },
                        // handled by the application
                        crate::sctk_event::WindowEventVariant::StateChanged(_) => {},
                    },
                    SctkEvent::LayerSurfaceEvent { variant, id: wl_surface } => match variant {
                        LayerSurfaceEventVariant::Created(id, native_id) => {
                            surface_ids.insert(id, SurfaceIdWrapper::LayerSurface(native_id));
                            states.insert(native_id, State::new(&application, SurfaceIdWrapper::LayerSurface(native_id), SurfaceDisplayWrapper {
                                backend: backend.clone(),
                                wl_surface: wl_surface.clone()
                            }));

                        }
                        LayerSurfaceEventVariant::Done => {
                            if let Some(surface_id) = surface_ids.remove(&wl_surface.id()) {
                                if kbd_surface_id == Some(wl_surface.id()) {
                                    kbd_surface_id = None;
                                }
                                auto_size_surfaces.remove(&surface_id);
                                interfaces.remove(&surface_id.inner());
                                states.remove(&surface_id.inner());
                                destroyed_surface_ids.insert(wl_surface.id(), surface_id);
                                if exit_on_close_request && states.is_empty() {
                                    break 'main;
                                }
                            }
                        }
                        LayerSurfaceEventVariant::Configure(configure, wl_surface, first) => {
                            if let Some(id) = surface_ids.get(&wl_surface.id()) {
                                let Some(state) = states.get_mut(&id.inner()) else {
                                    continue;
                                };
                                if state.surface.is_none() {
                                     let wrapper = SurfaceDisplayWrapper {
                                         backend: backend.clone(),
                                         wl_surface
                                     };
                                     if matches!(simple_clipboard.state(),  crate::clipboard::State::Unavailable) {
                                        if let Ok(h) = wrapper.display_handle() {
                                            simple_clipboard = unsafe {Clipboard::connect(&h)};
                                        }
                                     }
                                     let mut c_surface = compositor.create_surface(wrapper.clone(), configure.new_size.0, configure.new_size.1);
                                     compositor.configure_surface(&mut c_surface, configure.new_size.0, configure.new_size.1);
                                     state.surface = Some(c_surface);
                                };
                                if let Some((w, h, _, is_dirty)) = auto_size_surfaces.get_mut(id) {
                                    *is_dirty = first || *w != configure.new_size.0 || *h != configure.new_size.1;
                                    state.set_logical_size(*w as f32, *h as f32);
                                } else {
                                    state.set_logical_size(
                                        configure.new_size.0 as f32,
                                        configure.new_size.1 as f32,
                                    );
                                }
                                if first {
                                    let user_interface = build_user_interface(
                                        &application,
                                        user_interface::Cache::default(),
                                        &mut renderer,
                                        state.logical_size(),
                                        &state.title,
                                        &mut debug,
                                        *id,
                                        &mut auto_size_surfaces,
                                        &mut ev_proxy
                                    );
                                    interfaces.insert(id.inner(), user_interface);
                                }
                            }
                        }
                        LayerSurfaceEventVariant::ScaleFactorChanged(sf, viewport) => {
                            if let Some(state) = surface_ids
                                .get(&wl_surface.id())
                                .and_then(|id| states.get_mut(&id.inner()))
                            {
                                state.wp_viewport = viewport;
                                state.set_scale_factor(sf);
                            }
                        },
                    },
                    SctkEvent::PopupEvent {
                        variant,
                        toplevel_id: _,
                        parent_id: _,
                        id: wl_surface,
                    } => match variant {
                        PopupEventVariant::Created(id, native_id) => {
                            surface_ids.insert(id, SurfaceIdWrapper::Popup(native_id));
                            states.insert(native_id, State::new(&application, SurfaceIdWrapper::Popup(native_id),SurfaceDisplayWrapper {
                                backend: backend.clone(),
                                wl_surface
                            }));
                        }
                        PopupEventVariant::Done => {
                            if let Some(surface_id) = surface_ids.remove(&wl_surface.id()) {
                                auto_size_surfaces.remove(&surface_id);
                                interfaces.remove(&surface_id.inner());
                                states.remove(&surface_id.inner());
                                destroyed_surface_ids.insert(wl_surface.id(), surface_id);
                            }
                        }
                        PopupEventVariant::Configure(configure, wl_surface, first) => {
                            if let Some(id) = surface_ids.get(&wl_surface.id()) {
                                let Some(state) = states.get_mut(&id.inner()) else {
                                    continue;
                                };
                                if state.surface.is_none() {
                                    let wrapper = SurfaceDisplayWrapper {
                                        backend: backend.clone(),
                                        wl_surface
                                    };
                                    let c_surface = compositor.create_surface(wrapper.clone(), configure.width as u32, configure.height as u32);

                                    state.surface = Some(c_surface);
                                }
                                if let Some((w, h, _, is_dirty)) = auto_size_surfaces.get_mut(id) {
                                        *is_dirty |= first || *w != configure.width as u32 || *h != configure.height as u32;
                                        state.set_logical_size(*w as f32, *h as f32);
                                } else {
                                    state.set_logical_size(
                                        configure.width as f32,
                                        configure.height as f32,
                                    );
                                };
                                if first {
                                    let user_interface = build_user_interface(
                                        &application,
                                        user_interface::Cache::default(),
                                        &mut renderer,
                                        state.logical_size(),
                                        &state.title,
                                        &mut debug,
                                        *id,
                                        &mut auto_size_surfaces,
                                        &mut ev_proxy
                                    );
                                    interfaces.insert(id.inner(), user_interface);
                                }
                            }
                        }
                        PopupEventVariant::RepositionionedPopup { .. } => {}
                        PopupEventVariant::Size(width, height) => {
                            if let Some(id) = surface_ids.get(&wl_surface.id()) {
                                if let Some(state) = states.get_mut(&id.inner()) {
                                    if let Some((w, h, _, is_dirty)) = auto_size_surfaces.get_mut(id) {
                                        *is_dirty = *w != width || *h != height;
                                        state.set_logical_size(*w as f32, *h as f32);
                                    } else {
                                        state.set_logical_size(
                                            width as f32,
                                            height as f32,
                                        );
                                    }
                                }
                            }
                        },
                        PopupEventVariant::ScaleFactorChanged(sf, viewport) => {
                            if let Some(id) = surface_ids.get(&wl_surface.id()) {
                                if let Some(state) = states.get_mut(&id.inner()) {
                                    state.wp_viewport = viewport;
                                    state.set_scale_factor(sf);
                                }
                            }
                        },
                    },
                    // TODO forward these events to an application which requests them?
                    SctkEvent::NewOutput { .. } => {
                    }
                    SctkEvent::UpdateOutput { .. } => {
                    }
                    SctkEvent::RemovedOutput( ..) => {
                    }
                    SctkEvent::ScaleFactorChanged { .. } => {}
                    SctkEvent::DataSource(DataSourceEvent::DndFinished) | SctkEvent::DataSource(DataSourceEvent::DndCancelled)=> {
                        surface_ids.retain(|id, surface_id| {
                            match surface_id {
                                SurfaceIdWrapper::Dnd(inner) => {
                                    interfaces.remove(inner);
                                    states.remove(inner);
                                    destroyed_surface_ids.insert(id.clone(), *surface_id);
                                    false
                                },
                                _ => true,
                            }
                        })
                    }
                    SctkEvent::SessionLockSurfaceCreated { surface, native_id } => {
                        surface_ids.insert(surface.id(), SurfaceIdWrapper::SessionLock(native_id));
                        states.insert(native_id, State::new(&application, SurfaceIdWrapper::SessionLock(native_id), SurfaceDisplayWrapper {
                                backend: backend.clone(),
                                wl_surface: surface.clone()
                            }
                        ));
                    }
                    SctkEvent::SessionLockSurfaceConfigure { surface, configure, first } => {
                        if let Some(id) = surface_ids.get(&surface.id()) {
                            let Some(state) = states.get_mut(&id.inner()) else {
                                continue;
                            };
                            if state.surface.is_none() {
                                let c_surface = compositor.create_surface(state.wrapper.clone(), configure.new_size.0, configure.new_size.1);

                                state.surface = Some(c_surface);
                            }
                            if first {
                                let user_interface = build_user_interface(
                                    &application,
                                    user_interface::Cache::default(),
                                    &mut renderer,
                                    state.logical_size(),
                                    &state.title,
                                    &mut debug,
                                    *id,
                                    &mut auto_size_surfaces,
                                    &mut ev_proxy
                                );
                                interfaces.insert(id.inner(), user_interface);
                            }

                            state.set_logical_size(configure.new_size.0 as f32 , configure.new_size.1 as f32);
                        }

                    }
                    SctkEvent::SessionLockSurfaceDone { surface } => {
                        if let Some(surface_id) = surface_ids.remove(&surface.id()) {
                            if kbd_surface_id == Some(surface.id()) {
                                kbd_surface_id = None;
                            }
                            auto_size_surfaces.remove(&surface_id);
                            interfaces.remove(&surface_id.inner());
                            states.remove(&surface_id.inner());
                            destroyed_surface_ids.insert(surface.id(), surface_id);
                        }
                    }
                    _ => {}
                }
            }
            IcedSctkEvent::DndSurfaceCreated(
                wl_surface,
                dnd_icon,
                origin_id,
            ) => {
                // if the surface is meant to be drawn as a custom widget by the
                // application, we should treat it like any other surfaces
                //
                // TODO if the surface is meant to be drawn by a widget that implements
                // draw_dnd_icon, we should mark it and not pass it to the view method
                // of the Application
                //
                // Dnd Surfaces are only drawn once

                let id = wl_surface.id();
                let (native_id, _e, node) = match dnd_icon {
                    DndIcon::Custom(id) => {
                        let mut e = application.view(id);
                        let state = e.as_widget().state();
                        let tag = e.as_widget().tag();
                        let mut tree = Tree {
                            id: e.as_widget().id(),
                            tag,
                            state,
                            children: e.as_widget().children(),
                        };
                        e.as_widget_mut().diff(&mut tree);
                        let node = Widget::layout(
                            e.as_widget(),
                            &mut tree,
                            &renderer,
                            &Limits::NONE,
                        );
                        (id, e, node)
                    }
                    DndIcon::Widget(id, widget_state) => {
                        let mut e = application.view(id);
                        let mut tree = Tree {
                            id: e.as_widget().id(),
                            tag: e.as_widget().tag(),
                            state: tree::State::Some(widget_state),
                            children: e.as_widget().children(),
                        };
                        e.as_widget_mut().diff(&mut tree);
                        let node = Widget::layout(
                            e.as_widget(),
                            &mut tree,
                            &renderer,
                            &Limits::NONE,
                        );
                        (id, e, node)
                    }
                };

                let bounds = node.bounds();
                let (w, h) = (
                    (bounds.width.ceil()) as u32,
                    (bounds.height.ceil()) as u32,
                );
                if w == 0 || h == 0 {
                    error!("Dnd surface has zero size, ignoring");
                    continue;
                }
                let parent_size = states
                    .get(&origin_id)
                    .map(|s| s.logical_size())
                    .unwrap_or_else(|| Size::new(1024.0, 1024.0));
                if w > parent_size.width as u32 || h > parent_size.height as u32
                {
                    error!("Dnd surface is too large, ignoring");
                    continue;
                }
                let wrapper = SurfaceDisplayWrapper {
                    backend: backend.clone(),
                    wl_surface,
                };
                let mut c_surface =
                    compositor.create_surface(wrapper.clone(), w, h);
                compositor.configure_surface(&mut c_surface, w, h);
                let mut state = State::new(
                    &application,
                    SurfaceIdWrapper::Dnd(native_id),
                    wrapper,
                );
                state.surface = Some(c_surface);
                state.set_logical_size(w as f32, h as f32);
                let mut user_interface = build_user_interface(
                    &application,
                    user_interface::Cache::default(),
                    &mut renderer,
                    state.logical_size(),
                    &state.title,
                    &mut debug,
                    SurfaceIdWrapper::Dnd(native_id),
                    &mut auto_size_surfaces,
                    &mut ev_proxy,
                );
                state.synchronize(&application);

                // Subsurface list should always be empty before `view`
                assert!(crate::subsurface_widget::take_subsurfaces().is_empty());

                // just draw here immediately and never again for dnd icons
                // TODO handle scale factor?
                let _new_mouse_interaction = user_interface.draw(
                    &mut renderer,
                    state.theme(),
                    &Style {
                        icon_color: state.icon_color(),
                        text_color: state.text_color(),
                        scale_factor: state.scale_factor(),
                    },
                    state.cursor(),
                );

                let subsurfaces = crate::subsurface_widget::take_subsurfaces();
                if let Some(subsurface_state) = subsurface_state.as_mut() {
                    subsurface_state.update_subsurfaces(
                        &mut subsurface_ids,
                        &state.wrapper.wl_surface,
                        state.id,
                        &mut state.subsurfaces,
                        &subsurfaces,
                    );
                }

                let _ = compositor.present(
                    &mut renderer,
                    state.surface.as_mut().unwrap(),
                    &state.viewport,
                    Color::TRANSPARENT,
                    &debug.overlay(),
                );

                surface_ids.insert(id, SurfaceIdWrapper::Dnd(native_id));

                states.insert(native_id, state);
                interfaces.insert(native_id, user_interface);
            }
            IcedSctkEvent::MainEventsCleared => {
                if !redraw_pending
                    && sctk_events.is_empty()
                    && messages.is_empty()
                {
                    continue;
                }

                if surface_ids.is_empty() && !messages.is_empty() {
                    // Update application
                    let pure_states: HashMap<_, _> =
                        ManuallyDrop::into_inner(interfaces)
                            .drain()
                            .map(|(id, interface)| (id, interface.into_cache()))
                            .collect();

                    // Update application
                    update::<A, E, C>(
                        &mut application,
                        &mut cache,
                        None,
                        &mut renderer,
                        &mut runtime,
                        &mut ev_proxy,
                        &mut debug,
                        &mut messages,
                        &mut Vec::new(),
                        || compositor.fetch_information(),
                        &mut auto_size_surfaces,
                        &mut simple_clipboard,
                    );

                    interfaces = ManuallyDrop::new(build_user_interfaces(
                        &application,
                        &mut renderer,
                        &mut debug,
                        &states,
                        pure_states,
                        &mut auto_size_surfaces,
                        &mut ev_proxy,
                    ));

                    let _ = control_sender.start_send(ControlFlow::Wait);
                } else {
                    let mut actions = Vec::new();
                    let mut needs_update = false;

                    for (object_id, surface_id) in &surface_ids {
                        if matches!(surface_id, SurfaceIdWrapper::Dnd(_)) {
                            continue;
                        }
                        let mut filtered_sctk =
                            Vec::with_capacity(sctk_events.len());
                        let Some(state) = states.get_mut(&surface_id.inner())
                        else {
                            continue;
                        };
                        let mut i = 0;

                        while i < sctk_events.len() {
                            let has_kbd_focus =
                                kbd_surface_id.as_ref() == Some(object_id);
                            if event_is_for_all_surfaces(&sctk_events[i]) {
                                filtered_sctk.push(sctk_events[i].clone());
                                i += 1;
                            } else if event_is_for_surface(
                                &sctk_events[i],
                                object_id,
                                state,
                                has_kbd_focus,
                            ) {
                                filtered_sctk.push(sctk_events.remove(i));
                            } else {
                                i += 1;
                            }
                        }
                        let has_events = !sctk_events.is_empty();
                        debug.event_processing_started();
                        #[allow(unused_mut)]
                        let mut native_events: Vec<_> = filtered_sctk
                            .into_iter()
                            .flat_map(|e| {
                                e.to_native(
                                    &mut mods,
                                    &surface_ids,
                                    &destroyed_surface_ids,
                                    &subsurface_ids,
                                )
                            })
                            .collect();

                        #[cfg(feature = "a11y")]
                        {
                            let mut filtered_a11y =
                                Vec::with_capacity(a11y_events.len());
                            while i < a11y_events.len() {
                                if a11y_events[i].surface_id == *object_id {
                                    filtered_a11y.push(a11y_events.remove(i));
                                } else {
                                    i += 1;
                                }
                            }
                            native_events.extend(
                                filtered_a11y.into_iter().map(|e| {
                                    iced_runtime::core::event::Event::A11y(
                                        iced_runtime::core::id::Id::from(
                                            u128::from(e.request.target.0)
                                                as u64,
                                        ),
                                        e.request,
                                    )
                                }),
                            );
                        }
                        let has_events =
                            has_events || !native_events.is_empty();

                        let (interface_state, statuses) = {
                            let Some(user_interface) =
                                interfaces.get_mut(&surface_id.inner())
                            else {
                                continue;
                            };
                            user_interface.update(
                                native_events.as_slice(),
                                state.cursor(),
                                &mut renderer,
                                &mut simple_clipboard,
                                &mut messages,
                            )
                        };
                        state.interface_state = interface_state;
                        debug.event_processing_finished();
                        for (event, status) in
                            native_events.into_iter().zip(statuses.into_iter())
                        {
                            runtime.broadcast(event, status);
                        }

                        needs_update = !messages.is_empty()
                            || matches!(
                                interface_state,
                                user_interface::State::Outdated
                            )
                            || state.first()
                            || has_events
                            || state.viewport_changed;
                        if redraw_pending || needs_update {
                            state.set_needs_redraw(
                                state.frame_pending || needs_update,
                            );
                            state.set_first(false);
                        }
                    }
                    if needs_update {
                        let mut pure_states: HashMap<_, _> =
                            ManuallyDrop::into_inner(interfaces)
                                .drain()
                                .map(|(id, interface)| {
                                    (id, interface.into_cache())
                                })
                                .collect();

                        for surface_id in surface_ids.values() {
                            let state =
                                match states.get_mut(&surface_id.inner()) {
                                    Some(s) => {
                                        if !s.needs_redraw() {
                                            continue;
                                        } else {
                                            s
                                        }
                                    }
                                    None => continue,
                                };
                            let mut cache =
                                match pure_states.remove(&surface_id.inner()) {
                                    Some(cache) => cache,
                                    None => user_interface::Cache::default(),
                                };

                            // Update application
                            update::<A, E, C>(
                                &mut application,
                                &mut cache,
                                Some(state),
                                &mut renderer,
                                &mut runtime,
                                &mut ev_proxy,
                                &mut debug,
                                &mut messages,
                                &mut actions,
                                || compositor.fetch_information(),
                                &mut auto_size_surfaces,
                                &mut simple_clipboard,
                            );

                            pure_states.insert(surface_id.inner(), cache);

                            // Update state
                            state.synchronize(&application);
                        }
                        interfaces = ManuallyDrop::new(build_user_interfaces(
                            &application,
                            &mut renderer,
                            &mut debug,
                            &states,
                            pure_states,
                            &mut auto_size_surfaces,
                            &mut ev_proxy,
                        ));
                    }
                    let mut sent_control_flow = false;
                    for (object_id, surface_id) in &surface_ids {
                        let state = match states.get_mut(&surface_id.inner()) {
                            Some(s) => {
                                if !s.needs_redraw()
                                    || auto_size_surfaces
                                        .get(surface_id)
                                        .map(|(w, h, _, dirty)| {
                                            // don't redraw yet if the autosize state is dirty
                                            *dirty || {
                                                let Size { width, height } =
                                                    s.logical_size();
                                                width.ceil() as u32 != *w
                                                    || height.ceil() as u32
                                                        != *h
                                            }
                                        })
                                        .unwrap_or_default()
                                {
                                    continue;
                                } else {
                                    s.set_needs_redraw(false);

                                    s
                                }
                            }
                            None => continue,
                        };

                        let redraw_event = CoreEvent::Window(
                            surface_id.inner(),
                            crate::core::window::Event::RedrawRequested(
                                Instant::now(),
                            ),
                        );
                        let Some(user_interface) =
                            interfaces.get_mut(&surface_id.inner())
                        else {
                            continue;
                        };
                        let (interface_state, _) = user_interface.update(
                            &[redraw_event.clone()],
                            state.cursor(),
                            &mut renderer,
                            &mut simple_clipboard,
                            &mut messages,
                        );

                        runtime.broadcast(redraw_event, Status::Ignored);

                        ev_proxy.send_event(Event::SctkEvent(
                            IcedSctkEvent::RedrawRequested(object_id.clone()),
                        ));
                        sent_control_flow = true;
                        let _ =
                            control_sender
                                .start_send(match interface_state {
                                user_interface::State::Updated {
                                    redraw_request: Some(redraw_request),
                                } => {
                                    match redraw_request {
                                        crate::core::window::RedrawRequest::NextFrame => {
                                            ControlFlow::Poll
                                        }
                                        crate::core::window::RedrawRequest::At(at) => {
                                            ControlFlow::WaitUntil(at)
                                        }
                                    }},
                                _ => if needs_update {
                                    ControlFlow::Poll
                                } else {
                                    ControlFlow::Wait
                                },
                            });
                    }
                    if !sent_control_flow {
                        if let Some(d) = wait {
                            let mut wait_until = Instant::now();
                            wait_until += d;
                            _ = control_sender
                                .start_send(ControlFlow::WaitUntil(wait_until));
                        } else {
                            _ = control_sender.start_send(ControlFlow::Wait);
                        }
                    }
                    redraw_pending = false;
                }

                let mut i = 0;
                while i < sctk_events.len() {
                    let remove = matches!(
                        sctk_events[i],
                        SctkEvent::NewOutput { .. }
                            | SctkEvent::UpdateOutput { .. }
                            | SctkEvent::RemovedOutput(_)
                            | SctkEvent::SessionLocked
                            | SctkEvent::SessionLockFinished
                            | SctkEvent::SessionUnlocked
                            | SctkEvent::PopupEvent { .. }
                            | SctkEvent::LayerSurfaceEvent { .. }
                            | SctkEvent::WindowEvent { .. }
                    );
                    if remove {
                        let event = sctk_events.remove(i);
                        for native_event in event.to_native(
                            &mut mods,
                            &surface_ids,
                            &destroyed_surface_ids,
                            &subsurface_ids,
                        ) {
                            runtime.broadcast(native_event, Status::Ignored);
                        }
                    } else {
                        i += 1;
                    }
                }

                sctk_events.clear();
                // clear the destroyed surfaces after they have been handled
                destroyed_surface_ids.clear();
            }
            IcedSctkEvent::RedrawRequested(object_id) => {
                if let Some((
                    native_id,
                    Some(mut user_interface),
                    Some(state),
                )) = surface_ids.get(&object_id).and_then(|id| {
                    if matches!(id, SurfaceIdWrapper::Dnd(_)) {
                        return None;
                    }
                    let interface = interfaces.remove(&id.inner());
                    let state = states.get_mut(&id.inner());
                    Some((*id, interface, state))
                }) {
                    let Some(mut comp_surface) = state.surface.take() else {
                        error!("missing surface!");
                        continue;
                    };

                    debug.render_started();
                    #[cfg(feature = "a11y")]
                    if let Some(Some(adapter)) = a11y_enabled
                        .then(|| adapters.get_mut(&native_id.inner()))
                    {
                        use iced_accessibility::{
                            accesskit::{Role, Tree, TreeUpdate},
                            A11yTree,
                        };
                        // TODO send a11y tree
                        let child_tree =
                            user_interface.a11y_nodes(state.cursor());
                        let mut root = NodeBuilder::new(Role::Window);
                        root.set_name(state.title.to_string());
                        let window_tree = A11yTree::node_with_child_tree(
                            A11yNode::new(root, adapter.id),
                            child_tree,
                        );
                        let tree = Tree::new(NodeId(adapter.id));
                        let mut current_operation =
                            Some(Box::new(OperationWrapper::Id(Box::new(
                                operation::focusable::find_focused(),
                            ))));
                        let mut focus = None;
                        while let Some(mut operation) = current_operation.take()
                        {
                            user_interface
                                .operate(&renderer, operation.as_mut());

                            match operation.finish() {
                                operation::Outcome::None => {
                                }
                                operation::Outcome::Some(message) => {
                                    match message {
                                        operation::OperationOutputWrapper::Message(_) => {
                                            unimplemented!();
                                        }
                                        operation::OperationOutputWrapper::Id(id) => {
                                            focus = Some(A11yId::from(id));
                                        },
                                    }
                                }
                                operation::Outcome::Chain(next) => {
                                    current_operation = Some(Box::new(OperationWrapper::Wrapper(next)));
                                }
                            }
                        }
                        tracing::debug!(
                            "focus: {:?}\ntree root: {:?}\n children: {:?}",
                            &focus,
                            window_tree
                                .root()
                                .iter()
                                .map(|n| (n.node().role(), n.id()))
                                .collect::<Vec<_>>(),
                            window_tree
                                .children()
                                .iter()
                                .map(|n| (n.node().role(), n.id()))
                                .collect::<Vec<_>>()
                        );
                        let focus = focus
                            .filter(|f_id| window_tree.contains(f_id))
                            .map(|id| id.into())
                            .unwrap_or_else(|| tree.root);
                        adapter.adapter.update_if_active(|| TreeUpdate {
                            nodes: window_tree.into(),
                            tree: Some(tree),
                            focus,
                        });
                    }

                    if state.viewport_changed() {
                        let physical_size = state.physical_size();
                        let mut logical_size = state.logical_size();
                        compositor.configure_surface(
                            &mut comp_surface,
                            physical_size.width,
                            physical_size.height,
                        );

                        debug.layout_started();
                        // XXX must add a small number to the autosize surface size here
                        if auto_size_surfaces.contains_key(&native_id) {
                            logical_size.width += 0.001;
                            logical_size.height += 0.001;
                        }
                        user_interface = user_interface
                            .relayout(logical_size, &mut renderer);
                        debug.layout_finished();
                        state.viewport_changed = false;
                    }

                    // Subsurface list should always be empty before `view`
                    assert!(
                        crate::subsurface_widget::take_subsurfaces().is_empty()
                    );

                    debug.draw_started();
                    let new_mouse_interaction = user_interface.draw(
                        &mut renderer,
                        state.theme(),
                        &Style {
                            icon_color: state.icon_color(),
                            text_color: state.text_color(),
                            scale_factor: state.scale_factor(),
                        },
                        state.cursor(),
                    );

                    // Update subsurfaces based on what view requested.
                    let subsurfaces =
                        crate::subsurface_widget::take_subsurfaces();
                    if let Some(subsurface_state) = subsurface_state.as_mut() {
                        subsurface_state.update_subsurfaces(
                            &mut subsurface_ids,
                            &state.wrapper.wl_surface,
                            state.id,
                            &mut state.subsurfaces,
                            &subsurfaces,
                        );
                    }

                    debug.draw_finished();
                    if new_mouse_interaction != mouse_interaction {
                        mouse_interaction = new_mouse_interaction;
                        ev_proxy
                            .send_event(Event::SetCursor(mouse_interaction));
                    }

                    let _ =
                        interfaces.insert(native_id.inner(), user_interface);

                    if state.frame_pending {
                        // request a new frame
                        state.wrapper.wl_surface.frame(
                            &queue_handle,
                            state.wrapper.wl_surface.clone(),
                        );
                    }
                    let _ = compositor.present(
                        &mut renderer,
                        &mut comp_surface,
                        state.viewport(),
                        state.background_color(),
                        &debug.overlay(),
                    );
                    // Need commit to get frame event, and update subsurfaces, even if main surface wasn't changed
                    state.wrapper.wl_surface.commit();
                    state.frame_pending = false;
                    state.surface = Some(comp_surface);
                    debug.render_finished();
                }
            }
            IcedSctkEvent::RedrawEventsCleared => {
                // TODO
            }
            IcedSctkEvent::LoopDestroyed => {
                panic!("Loop destroyed");
            }
            #[cfg(feature = "a11y")]
            IcedSctkEvent::A11yEvent(ActionRequestEvent {
                surface_id,
                request,
            }) => {
                use iced_accessibility::accesskit::Action;
                match request.action {
                    Action::Default => {
                        // TODO default operation?
                        // messages.push(focus(request.target.into()));
                        a11y_events.push(ActionRequestEvent {
                            surface_id,
                            request,
                        });
                    }
                    Action::Focus => {
                        commands.push(Command::widget(
                            operation::focusable::focus(
                                iced_runtime::core::id::Id::from(u128::from(
                                    request.target.0,
                                )
                                    as u64),
                            ),
                        ));
                    }
                    Action::Blur => todo!(),
                    Action::Collapse => todo!(),
                    Action::Expand => todo!(),
                    Action::CustomAction => todo!(),
                    Action::Decrement => todo!(),
                    Action::Increment => todo!(),
                    Action::HideTooltip => todo!(),
                    Action::ShowTooltip => todo!(),
                    Action::ReplaceSelectedText => todo!(),
                    Action::ScrollBackward => todo!(),
                    Action::ScrollDown => todo!(),
                    Action::ScrollForward => todo!(),
                    Action::ScrollLeft => todo!(),
                    Action::ScrollRight => todo!(),
                    Action::ScrollUp => todo!(),
                    Action::ScrollIntoView => todo!(),
                    Action::ScrollToPoint => todo!(),
                    Action::SetScrollOffset => todo!(),
                    Action::SetTextSelection => todo!(),
                    Action::SetSequentialFocusNavigationStartingPoint => {
                        todo!()
                    }
                    Action::SetValue => todo!(),
                    Action::ShowContextMenu => todo!(),
                }
            }
            #[cfg(feature = "a11y")]
            IcedSctkEvent::A11yEnabled => {
                a11y_enabled = true;
            }
            #[cfg(feature = "a11y")]
            IcedSctkEvent::A11ySurfaceCreated(surface_id, adapter) => {
                adapters.insert(surface_id.inner(), adapter);
            }
            IcedSctkEvent::Frame(surface, time) => {
                if let Some(id) = surface_ids
                    .get(&surface.id())
                    .or_else(|| Some(&subsurface_ids.get(&surface.id())?.2))
                {
                    if let Some(state) = states.get_mut(&id.inner()) {
                        state.set_frame(time);
                    }
                }
            }
            IcedSctkEvent::Subcompositor(state) => {
                subsurface_state = Some(state);
            }
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SurfaceIdWrapper {
    LayerSurface(SurfaceId),
    Window(SurfaceId),
    Popup(SurfaceId),
    Dnd(SurfaceId),
    SessionLock(SurfaceId),
}

impl SurfaceIdWrapper {
    pub fn inner(&self) -> SurfaceId {
        match self {
            SurfaceIdWrapper::LayerSurface(id) => *id,
            SurfaceIdWrapper::Window(id) => *id,
            SurfaceIdWrapper::Popup(id) => *id,
            SurfaceIdWrapper::Dnd(id) => *id,
            SurfaceIdWrapper::SessionLock(id) => *id,
        }
    }
}

/// Builds a [`UserInterface`] for the provided [`Application`], logging
/// [`struct@Debug`] information accordingly.
pub fn build_user_interface<'a, A: Application>(
    application: &'a A,
    cache: user_interface::Cache,
    renderer: &mut A::Renderer,
    size: Size,
    _title: &str,
    debug: &mut Debug,
    id: SurfaceIdWrapper,
    auto_size_surfaces: &mut HashMap<
        SurfaceIdWrapper,
        (u32, u32, Limits, bool),
    >,
    ev_proxy: &mut proxy::Proxy<Event<A::Message>>,
) -> UserInterface<'a, A::Message, A::Theme, A::Renderer>
where
    <A as Program>::Theme: StyleSheet,
{
    debug.view_started();
    let mut view = application.view(id.inner());
    debug.view_finished();

    let size = if let Some((auto_size_w, auto_size_h, limits, dirty)) =
        auto_size_surfaces.remove(&id)
    {
        // TODO would it be ok to diff against the current cache?
        let mut tree = Tree::new(view.as_widget_mut());
        let bounds = view
            .as_widget()
            .layout(&mut tree, renderer, &limits)
            .bounds()
            .size();
        let (w, h) = (
            (bounds.width.ceil()).max(1.0) as u32,
            (bounds.height.ceil()).max(1.0) as u32,
        );
        let dirty = dirty
            || w != size.width.ceil() as u32
            || h != size.height.ceil() as u32
            || w != auto_size_w
            || h != auto_size_h;

        auto_size_surfaces.insert(id, (w, h, limits, dirty));
        if dirty {
            match id {
                SurfaceIdWrapper::LayerSurface(inner) => {
                    ev_proxy.send_event(
                        Event::LayerSurface(
                            command::platform_specific::wayland::layer_surface::Action::Size { id: inner, width: Some(w), height: Some(h) },
                        )
                    );
                }
                SurfaceIdWrapper::Window(inner) => {
                    ev_proxy.send_event(
                        Event::Window(
                            command::platform_specific::wayland::window::Action::Size { id: inner, width: w, height: h },
                        )
                    );
                }
                SurfaceIdWrapper::Popup(inner) => {
                    ev_proxy.send_event(
                        Event::Popup(
                            command::platform_specific::wayland::popup::Action::Size { id: inner, width: w, height: h },
                        )
                    );
                }
                SurfaceIdWrapper::Dnd(_) => {}
                SurfaceIdWrapper::SessionLock(_) => {}
            };
        }

        // XXX must add a small amount to the size.
        // Layout seems to sometimes build the interface slightly
        // differently when given a size versus just limits
        // this is problematic for autosize surfaces that rely on the size previously calculated
        Size::new(w as f32 + 0.001, h as f32 + 0.001)
    } else {
        size
    };

    debug.layout_started();
    let user_interface = UserInterface::build(view, size, cache, renderer);
    debug.layout_finished();

    user_interface
}

/// The state of a surface created by the application [`Application`].
#[allow(missing_debug_implementations)]
pub struct State<A: Application, C: Compositor>
where
    <A as Program>::Theme: application::StyleSheet,
{
    pub(crate) id: SurfaceIdWrapper,
    title: String,
    application_scale_factor: f64,
    surface_scale_factor: f64,
    viewport: Viewport,
    viewport_changed: bool,
    cursor_position: Option<PhysicalPosition<i32>>,
    modifiers: Modifiers,
    theme: <A as Program>::Theme,
    appearance: application::Appearance,
    application: PhantomData<A>,
    // Time of last frame event, or 0
    frame_pending: bool,
    last_frame_time: u32,
    needs_redraw: bool,
    first: bool,
    wp_viewport: Option<WpViewport>,
    interface_state: user_interface::State,
    surface: Option<C::Surface>,
    wrapper: SurfaceDisplayWrapper,
    subsurfaces: Vec<SubsurfaceInstance>,
}

impl<A: Application, C: Compositor> State<A, C>
where
    <A as Program>::Theme: application::StyleSheet,
{
    /// Creates a new [`State`] for the provided [`Application`]
    pub fn new(
        application: &A,
        id: SurfaceIdWrapper,
        wrapper: SurfaceDisplayWrapper,
    ) -> Self {
        let title = application.title(id.inner());
        let scale_factor = application.scale_factor(id.inner());
        let theme = application.theme(id.inner());
        let appearance = theme.appearance(&application.style());
        let viewport = Viewport::with_physical_size(Size::new(1, 1), 1.0);

        Self {
            id,
            title,
            application_scale_factor: scale_factor,
            surface_scale_factor: 1.0, // assumed to be 1.0 at first
            viewport,
            viewport_changed: true,
            // TODO: Encode cursor availability in the type-system
            cursor_position: None,
            modifiers: Modifiers::default(),
            theme,
            appearance,
            application: PhantomData,
            frame_pending: false,
            last_frame_time: 0,
            needs_redraw: false,
            first: true,
            wp_viewport: None,
            interface_state: user_interface::State::Outdated,
            surface: None,
            wrapper,
            subsurfaces: Vec::new(),
        }
    }

    pub(crate) fn set_needs_redraw(&mut self, needs_redraw: bool) {
        self.needs_redraw = needs_redraw;
    }

    pub(crate) fn needs_redraw(&self) -> bool {
        self.needs_redraw
    }

    fn set_frame(&mut self, time: u32) {
        // If we get frame events from mulitple subsurface, should have same time. So ignore if
        // time isn't newer.
        if time == 0 || time > self.last_frame_time {
            self.frame_pending = true;
            self.last_frame_time = time;
        }
    }

    pub(crate) fn first(&self) -> bool {
        self.first
    }

    pub(crate) fn set_first(&mut self, first: bool) {
        self.first = first;
    }

    /// Returns the current [`Viewport`] of the [`State`].
    pub fn viewport(&self) -> &Viewport {
        &self.viewport
    }

    /// Returns the current title of the [`State`].
    pub fn title(&self) -> &str {
        &self.title
    }

    /// TODO
    pub fn viewport_changed(&self) -> bool {
        self.viewport_changed
    }

    /// Returns the physical [`Size`] of the [`Viewport`] of the [`State`].
    pub fn physical_size(&self) -> Size<u32> {
        self.viewport.physical_size()
    }

    /// Returns the logical [`Size`] of the [`Viewport`] of the [`State`].
    pub fn logical_size(&self) -> Size<f32> {
        self.viewport.logical_size()
    }

    /// Sets the logical [`Size`] of the [`Viewport`] of the [`State`].
    pub fn set_logical_size(&mut self, w: f32, h: f32) {
        let old_size = self.viewport.logical_size();
        if !approx_eq!(f32, w as f32, old_size.width, F32Margin::default())
            || !approx_eq!(f32, h as f32, old_size.height, F32Margin::default())
        {
            let logical_size = Size::<f32>::new(w, h);
            self.viewport_changed = true;
            self.viewport =
                Viewport::with_logical_size(logical_size, self.scale_factor());
            if let Some(wp_viewport) = self.wp_viewport.as_ref() {
                wp_viewport.set_destination(
                    logical_size.width.ceil() as i32,
                    logical_size.height.ceil() as i32,
                );
            }
        }
    }

    /// Returns the current scale factor of the [`Viewport`] of the [`State`].
    pub fn scale_factor(&self) -> f64 {
        self.viewport.scale_factor()
    }

    pub fn set_scale_factor(&mut self, scale_factor: f64) {
        if !approx_eq!(
            f64,
            scale_factor,
            self.surface_scale_factor,
            F64Margin::default()
        ) {
            self.viewport_changed = true;
            let logical_size = self.viewport.logical_size();
            self.surface_scale_factor = scale_factor;
            self.viewport = Viewport::with_logical_size(
                logical_size,
                self.application_scale_factor * self.surface_scale_factor,
            );
            if let Some(wp_viewport) = self.wp_viewport.as_ref() {
                wp_viewport.set_destination(
                    logical_size.width.ceil() as i32,
                    logical_size.height.ceil() as i32,
                );
            }
        }
    }

    // TODO use a type to encode cursor availability
    /// Returns the current cursor position of the [`State`].
    pub fn cursor(&self) -> mouse::Cursor {
        self.cursor_position
            .map(|cursor_position| {
                let scale_factor = self.application_scale_factor;
                assert!(
                    scale_factor.is_sign_positive() && scale_factor.is_normal()
                );
                let logical: LogicalPosition<f64> =
                    cursor_position.to_logical(scale_factor);

                Point {
                    x: logical.x as f32,
                    y: logical.y as f32,
                }
            })
            .map(mouse::Cursor::Available)
            .unwrap_or(mouse::Cursor::Unavailable)
    }

    /// Returns the current keyboard modifiers of the [`State`].
    pub fn modifiers(&self) -> Modifiers {
        self.modifiers
    }

    /// Returns the current theme of the [`State`].
    pub fn theme(&self) -> &<A as Program>::Theme {
        &self.theme
    }

    /// Returns the current background [`Color`] of the [`State`].
    pub fn background_color(&self) -> Color {
        self.appearance.background_color
    }

    /// Returns the current text [`Color`] of the [`State`].
    pub fn text_color(&self) -> Color {
        self.appearance.text_color
    }

    /// Returns the current icon [`Color`] of the [`State`].
    pub fn icon_color(&self) -> Color {
        self.appearance.icon_color
    }

    pub fn set_cursor_position(&mut self, p: Option<LogicalPosition<f64>>) {
        self.cursor_position =
            p.map(|p| p.to_physical(self.application_scale_factor));
    }

    fn synchronize(&mut self, application: &A) {
        // Update theme and appearance
        self.theme = application.theme(self.id.inner());
        self.appearance = self.theme.appearance(&application.style());
    }
}

// XXX Ashley careful, A, E, C must be exact same as in run_instance, or the subscription map type will have a different hash
/// Updates an [`Application`] by feeding it the provided messages, spawning any
/// resulting [`Command`], and tracking its [`Subscription`]
pub(crate) fn update<A, E, C>(
    application: &mut A,
    cache: &mut user_interface::Cache,
    state: Option<&State<A, C>>,
    renderer: &mut A::Renderer,
    runtime: MyRuntime<E, A::Message>,
    proxy: &mut proxy::Proxy<Event<A::Message>>,
    debug: &mut Debug,
    messages: &mut Vec<A::Message>,
    actions: &mut Vec<command::Action<A::Message>>,
    graphics_info: impl FnOnce() -> compositor::Information + Copy,
    auto_size_surfaces: &mut HashMap<
        SurfaceIdWrapper,
        (u32, u32, Limits, bool),
    >,
    clipboard: &mut Clipboard,
) where
    A: Application + 'static,
    E: Executor + 'static,
    C: iced_graphics::Compositor<Renderer = A::Renderer> + 'static,
    <A as Program>::Theme: StyleSheet,
{
    let actions_ = std::mem::take(actions);
    for a in actions_ {
        if let Some(a) = handle_actions(
            application,
            cache,
            state,
            renderer,
            a,
            runtime,
            proxy,
            debug,
            graphics_info,
            auto_size_surfaces,
            clipboard,
        ) {
            actions.push(a);
        }
    }
    for message in messages.drain(..) {
        debug.log_message(&message);

        debug.update_started();
        let command = runtime.enter(|| application.update(message));
        debug.update_finished();

        run_command(
            application,
            cache,
            state,
            renderer,
            command,
            runtime,
            proxy,
            debug,
            graphics_info,
            auto_size_surfaces,
            actions,
            clipboard,
        )
    }

    runtime.track(
        application
            .subscription()
            .map(subscription_map::<A, E, C>)
            .into_recipes(),
    );
}

type MyRuntime<'a, E, M> = &'a mut Runtime<E, proxy::Proxy<Event<M>>, Event<M>>;

/// Runs the actions of a [`Command`].
fn run_command<A, C, E>(
    application: &A,
    cache: &mut user_interface::Cache,
    state: Option<&State<A, C>>,
    renderer: &mut A::Renderer,
    command: Command<A::Message>,
    runtime: MyRuntime<E, A::Message>,
    proxy: &mut proxy::Proxy<Event<A::Message>>,
    debug: &mut Debug,
    graphics_info: impl FnOnce() -> compositor::Information + Copy,
    auto_size_surfaces: &mut HashMap<
        SurfaceIdWrapper,
        (u32, u32, Limits, bool),
    >,
    actions: &mut Vec<command::Action<A::Message>>,
    clipboard: &mut Clipboard,
) where
    A: Application,
    E: Executor,
    <A as Program>::Theme: StyleSheet,
    C: Compositor,
{
    for action in command.actions() {
        if let Some(a) = handle_actions(
            application,
            cache,
            state,
            renderer,
            action,
            runtime,
            proxy,
            debug,
            graphics_info,
            auto_size_surfaces,
            clipboard,
        ) {
            actions.push(a);
        }
    }
}

fn handle_actions<A, C, E>(
    application: &A,
    cache: &mut user_interface::Cache,
    state: Option<&State<A, C>>,
    renderer: &mut A::Renderer,
    action: command::Action<A::Message>,
    runtime: MyRuntime<E, A::Message>,
    proxy: &mut proxy::Proxy<Event<A::Message>>,
    debug: &mut Debug,
    _graphics_info: impl FnOnce() -> compositor::Information + Copy,
    auto_size_surfaces: &mut HashMap<
        SurfaceIdWrapper,
        (u32, u32, Limits, bool),
    >,
    clipboard: &mut Clipboard,
) -> Option<command::Action<A::Message>>
where
    A: Application,
    E: Executor,
    <A as Program>::Theme: StyleSheet,
    C: Compositor,
{
    match action {
            command::Action::Future(future) => {
                runtime
                    .spawn(Box::pin(future.map(|e| {
                        Event::SctkEvent(IcedSctkEvent::UserEvent(e))
                    })));
            }
            command::Action::Stream(stream) => {
                runtime.run(Box::pin(
                    stream.map(|e| Event::SctkEvent(IcedSctkEvent::UserEvent(e))),
                ));
            }
            command::Action::Clipboard(action) => match action {
                clipboard::Action::Read(s_to_msg) => {
                    let contents = clipboard.read();
                    let message = s_to_msg(contents);
                    proxy.send_event(Event::Message(message));
                }
                clipboard::Action::Write(contents) => {
                    clipboard.write(contents)
                }
                clipboard::Action::WriteData(contents) => {
                    clipboard.write_data(ClipboardStoreData(contents))
                },
                clipboard::Action::ReadData(allowed, to_msg) => {
                    let contents = clipboard.read_data(allowed);
                    let message = to_msg(contents);
                    proxy.send_event(Event::Message(message));
                },
                clipboard::Action::ReadPrimary(s_to_msg) => {
                    let contents = clipboard.read_primary();
                    let message = s_to_msg(contents);
                    proxy.send_event(Event::Message(message));
                },
                clipboard::Action::WritePrimary(content) => {
                    clipboard.write_primary(content)
                },
                clipboard::Action::WritePrimaryData(content) => {
                    clipboard.write_primary_data(ClipboardStoreData(content))
                },
                clipboard::Action::ReadPrimaryData(a, to_msg) => {
                    let contents = clipboard.read_primary_data(a);
                    let message = to_msg(contents);
                    proxy.send_event(Event::Message(message));
                },
            },
            command::Action::Window(..) => {
                unimplemented!("Use platform specific events instead")
            }
            command::Action::System(action) => match action {
                system::Action::QueryInformation(_tag) => {
                    #[cfg(feature = "system")]
                    {
                        let graphics_info = _graphics_info();
                        let proxy = proxy.clone();

                        let _ = std::thread::spawn(move || {
                            let information =
                                crate::system::information(graphics_info);

                            let message = _tag(information);

                            proxy
                                .send_event(Event::Message(message));
                        });
                    }
                }
            },
            command::Action::Widget(action) => {
                let state = match state {
                    Some(s) => s,
                    None => return None,
                };
                let id = &state.id;
                let mut current_cache = std::mem::take(cache);
                let mut current_operation = Some(Box::new(OperationWrapper::Message(action)));


                let mut user_interface = build_user_interface(
                    application,
                    current_cache,
                    renderer,
                    state.logical_size(),
                    &state.title,
                    debug,
                    *id, // TODO: run the operation on every widget tree ?
                    auto_size_surfaces,
                    proxy
                );
                let mut ret = None;

                while let Some(mut operation) = current_operation.take() {
                    user_interface.operate(renderer, operation.as_mut());

                    match operation.as_ref().finish() {
                        operation::Outcome::None => {
                            ret = Some(operation);
                        }
                        operation::Outcome::Some(message) => {
                            match message {
                                operation::OperationOutputWrapper::Message(m) => {
                                    proxy.send_event(Event::SctkEvent(
                                        IcedSctkEvent::UserEvent(m),
                                    ));
                                    ret = Some(operation)
                                },
                                operation::OperationOutputWrapper::Id(_) => {
                                    // should not happen
                                },
                            }
                        }
                        operation::Outcome::Chain(next) => {
                            current_operation = Some(Box::new(OperationWrapper::Wrapper(next)));
                        }
                    }
                }

                current_cache = user_interface.into_cache();
                *cache = current_cache;
                return ret.and_then(|o| match *o {
                    OperationWrapper::Message(o) => Some(command::Action::Widget(o)),
                    _ => None
                });
            }
            command::Action::PlatformSpecific(
                platform_specific::Action::Wayland(
                    platform_specific::wayland::Action::LayerSurface(
                        layer_surface_action,
                    ),
                ),
            ) => {
                if let platform_specific::wayland::layer_surface::Action::LayerSurface{ mut builder, _phantom } = layer_surface_action {
                    if builder.size.is_none() {
                        let e = application.view(builder.id);
                        let mut tree = Tree::new(e.as_widget());
                        let node = Widget::layout(e.as_widget(), &mut tree, renderer, &builder.size_limits);
                        let bounds = node.bounds();
                        let (w, h) = ((bounds.width.ceil()).max(1.0) as u32, (bounds.height.ceil()).max(1.0) as u32);
                        auto_size_surfaces.insert(SurfaceIdWrapper::LayerSurface(builder.id), (w, h, builder.size_limits, false));
                        builder.size = Some((Some(bounds.width as u32), Some(bounds.height as u32)));
                    }
                    proxy.send_event(Event::LayerSurface(platform_specific::wayland::layer_surface::Action::LayerSurface {builder, _phantom}));
                } else {
                    proxy.send_event(Event::LayerSurface(layer_surface_action));
                }
            }
            command::Action::PlatformSpecific(
                platform_specific::Action::Wayland(
                    platform_specific::wayland::Action::Window(window_action),
                ),
            ) => {
                if let platform_specific::wayland::window::Action::Window{ mut builder, _phantom } = window_action {
                    if builder.autosize {
                        let e = application.view(builder.window_id);
                        let mut tree = Tree::new(e.as_widget());
                        let node = Widget::layout(e.as_widget(), &mut tree, renderer, &builder.size_limits);
                        let bounds = node.bounds();
                        let (w, h) = ((bounds.width.ceil()).max(1.0) as u32, (bounds.height.ceil()).max(1.0) as u32);
                        auto_size_surfaces.insert(SurfaceIdWrapper::Window(builder.window_id), (w, h, builder.size_limits, false));
                        builder.size = (bounds.width as u32, bounds.height as u32);
                    }
                    proxy.send_event(Event::Window(platform_specific::wayland::window::Action::Window{builder, _phantom}));
                } else {
                    proxy.send_event(Event::Window(window_action));
                }
            }
            command::Action::PlatformSpecific(
                platform_specific::Action::Wayland(
                    platform_specific::wayland::Action::Popup(popup_action),
                ),
            ) => {
                if let popup::Action::Popup { mut popup, _phantom } = popup_action {
                    if popup.positioner.size.is_none() {
                        let e = application.view(popup.id);
                        let mut tree = Tree::new(e.as_widget());
                        let node = Widget::layout( e.as_widget(), &mut tree, renderer, &popup.positioner.size_limits);
                        let bounds = node.bounds();
                        let (w, h) = (bounds.width.ceil().max(1.0) as u32, bounds.height.ceil().max(1.0) as u32);
                        auto_size_surfaces.insert(SurfaceIdWrapper::Popup(popup.id), (w, h, popup.positioner.size_limits, false));
                        popup.positioner.size = Some((w, h));
                    }
                    proxy.send_event(Event::Popup(popup::Action::Popup{popup, _phantom}));
                } else {
                    proxy.send_event(Event::Popup(popup_action));
                }
            }
            command::Action::PlatformSpecific(platform_specific::Action::Wayland(platform_specific::wayland::Action::DataDevice(data_device_action))) => {
                proxy.send_event(Event::DataDevice(data_device_action));
            }
            command::Action::PlatformSpecific(
                platform_specific::Action::Wayland(
                    platform_specific::wayland::Action::Activation(activation_action)
                )
            ) => {
                proxy.send_event(Event::Activation(activation_action));
            }
            command::Action::PlatformSpecific(platform_specific::Action::Wayland(platform_specific::wayland::Action::SessionLock(session_lock_action))) => {
                proxy.send_event(Event::SessionLock(session_lock_action));
            }
        _ => {}
        };
    None
}
pub fn build_user_interfaces<'a, A, C>(
    application: &'a A,
    renderer: &mut A::Renderer,
    debug: &mut Debug,
    states: &HashMap<SurfaceId, State<A, C>>,
    mut pure_states: HashMap<SurfaceId, user_interface::Cache>,
    auto_size_surfaces: &mut HashMap<
        SurfaceIdWrapper,
        (u32, u32, Limits, bool),
    >,
    ev_proxy: &mut proxy::Proxy<Event<A::Message>>,
) -> HashMap<
    SurfaceId,
    UserInterface<
        'a,
        <A as Program>::Message,
        <A as Program>::Theme,
        <A as Program>::Renderer,
    >,
>
where
    A: Application + 'static,
    <A as Program>::Theme: StyleSheet,
    C: Compositor,
{
    let mut interfaces = HashMap::new();

    // TODO ASHLEY make sure Ids are iterated in the same order every time for a11y
    for (id, pure_state) in pure_states.drain().sorted_by(|a, b| a.0.cmp(&b.0))
    {
        let state = &states.get(&id).unwrap();

        let user_interface = build_user_interface(
            application,
            pure_state,
            renderer,
            state.logical_size(),
            &state.title,
            debug,
            state.id,
            auto_size_surfaces,
            ev_proxy,
        );

        let _ = interfaces.insert(id, user_interface);
    }

    interfaces
}

fn event_is_for_all_surfaces(evt: &SctkEvent) -> bool {
    match evt {
        SctkEvent::DataSource(_) => true,
        _ => false,
    }
}

// Determine if `SctkEvent` is for surface with given object id.
fn event_is_for_surface<'a, A, C>(
    evt: &SctkEvent,
    object_id: &ObjectId,
    state: &State<A, C>,
    has_kbd_focus: bool,
) -> bool
where
    A: Application + 'static,
    <A as Program>::Theme: StyleSheet,
    C: Compositor,
{
    match evt {
        SctkEvent::SeatEvent { id, .. } => &id.id() == object_id,
        SctkEvent::PointerEvent { variant, .. } => {
            let event_object_id = variant.surface.id();
            &event_object_id == object_id
                || state
                    .subsurfaces
                    .iter()
                    .any(|s| s.wl_surface.id() == event_object_id)
        }
        SctkEvent::KeyboardEvent { variant, .. } => match variant {
            KeyboardEventVariant::Leave(id) => &id.id() == object_id,
            _ => has_kbd_focus,
        },
        SctkEvent::TouchEvent { surface, .. } => {
            let event_object_id = surface.id();
            &event_object_id == object_id
                || state
                    .subsurfaces
                    .iter()
                    .any(|s| s.wl_surface.id() == event_object_id)
        }
        SctkEvent::WindowEvent { id, .. } => &id.id() == object_id,
        SctkEvent::LayerSurfaceEvent { id, .. } => &id.id() == object_id,
        SctkEvent::PopupEvent { id, .. } => &id.id() == object_id,
        SctkEvent::NewOutput { .. }
        | SctkEvent::UpdateOutput { .. }
        | SctkEvent::RemovedOutput(_) => false,
        SctkEvent::ScaleFactorChanged { id, .. } => &id.id() == object_id,
        SctkEvent::DndOffer { surface, .. } => {
            let event_object_id = surface.id();
            &event_object_id == object_id
                || state
                    .subsurfaces
                    .iter()
                    .any(|s| s.wl_surface.id() == event_object_id)
        }
        SctkEvent::DataSource(_) => true,
        SctkEvent::SessionLocked => false,
        SctkEvent::SessionLockFinished => false,
        SctkEvent::SessionLockSurfaceCreated { surface, .. } => {
            &surface.id() == object_id
        }
        SctkEvent::SessionLockSurfaceConfigure { surface, .. } => {
            &surface.id() == object_id
        }
        SctkEvent::SessionLockSurfaceDone { surface } => {
            &surface.id() == object_id
        }
        SctkEvent::SessionUnlocked => false,
    }
}
