//! Create interactive, native cross-platform applications.
mod drag_resize;
mod state;

use dnd::DndAction;
use dnd::DndEvent;
use dnd::DndSurface;
use dnd::Icon;
#[cfg(feature = "a11y")]
use iced_graphics::core::widget::operation::focusable::focus;
use iced_graphics::core::widget::operation::OperationWrapper;
use iced_graphics::core::widget::Operation;
use iced_graphics::Viewport;
use iced_runtime::futures::futures::FutureExt;
use iced_style::core::clipboard::DndSource;
use iced_style::core::Clipboard as CoreClipboard;
use iced_style::core::Length;
pub use state::State;
use window_clipboard::mime;
use window_clipboard::mime::ClipboardStoreData;

use crate::conversion;
use crate::core;
use crate::core::mouse;
use crate::core::renderer;
use crate::core::time::Instant;
use crate::core::widget::operation;
use crate::core::window;
use crate::core::{Event, Size};
use crate::futures::futures;
use crate::futures::{Executor, Runtime, Subscription};
use crate::graphics::compositor::{self, Compositor};
use crate::runtime::clipboard;
use crate::runtime::program::Program;
use crate::runtime::user_interface::{self, UserInterface};
use crate::runtime::{Command, Debug};
use crate::style::application::{Appearance, StyleSheet};
use crate::{Clipboard, Error, Proxy, Settings};
use futures::channel::mpsc;
use futures::stream::StreamExt;

use std::any::Any;
use std::mem::ManuallyDrop;
use std::sync::Arc;

#[cfg(feature = "trace")]
pub use profiler::Profiler;
#[cfg(feature = "trace")]
use tracing::{info_span, instrument::Instrument};

/// Wrapper aroun application Messages to allow for more UserEvent variants
pub enum UserEventWrapper<Message> {
    /// Application Message
    Message(Message),
    #[cfg(feature = "a11y")]
    /// A11y Action Request
    A11y(iced_accessibility::accesskit_winit::ActionRequestEvent),
    #[cfg(feature = "a11y")]
    /// A11y was enabled
    A11yEnabled,
    /// CLipboard Message
    StartDnd {
        /// internal dnd
        internal: bool,
        /// the surface the dnd is started from
        source_surface: Option<DndSource>,
        /// the icon if any
        /// This is actually an Element
        icon_surface: Option<Box<dyn Any>>,
        /// the content of the dnd
        content: Box<dyn mime::AsMimeTypes + Send + 'static>,
        /// the actions of the dnd
        actions: DndAction,
    },
    /// Dnd Event
    Dnd(DndEvent<DndSurface>),
}

unsafe impl<M> Send for UserEventWrapper<M> {}

impl<M: std::fmt::Debug> std::fmt::Debug for UserEventWrapper<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserEventWrapper::Message(m) => write!(f, "Message({:?})", m),
            #[cfg(feature = "a11y")]
            UserEventWrapper::A11y(a) => write!(f, "A11y({:?})", a),
            #[cfg(feature = "a11y")]
            UserEventWrapper::A11yEnabled => write!(f, "A11yEnabled"),
            UserEventWrapper::StartDnd {
                internal,
                source_surface: _,
                icon_surface,
                content: _,
                actions,
            } => write!(
                f,
                "StartDnd {{ internal: {:?}, icon_surface: {}, actions: {:?} }}",
                internal, icon_surface.is_some(), actions
            ),
            UserEventWrapper::Dnd(_) => write!(f, "Dnd"),
        }
    }
}

#[cfg(feature = "a11y")]
impl<Message> From<iced_accessibility::accesskit_winit::ActionRequestEvent>
    for UserEventWrapper<Message>
{
    fn from(
        action_request: iced_accessibility::accesskit_winit::ActionRequestEvent,
    ) -> Self {
        UserEventWrapper::A11y(action_request)
    }
}

/// An interactive, native cross-platform application.
///
/// This trait is the main entrypoint of Iced. Once implemented, you can run
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
    fn title(&self) -> String;

    /// Returns the current `Theme` of the [`Application`].
    fn theme(&self) -> Self::Theme;

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

    /// Returns the scale factor of the [`Application`].
    ///
    /// It can be used to dynamically control the size of the UI at runtime
    /// (i.e. zooming).
    ///
    /// For instance, a scale factor of `2.0` will make widgets twice as big,
    /// while a scale factor of `0.5` will shrink them to half their size.
    ///
    /// By default, it returns `1.0`.
    fn scale_factor(&self) -> f64 {
        1.0
    }
}

/// Runs an [`Application`] with an executor, compositor, and the provided
/// settings.
pub fn run<A, E, C>(
    settings: Settings<A::Flags>,
    compositor_settings: C::Settings,
) -> Result<(), Error>
where
    A: Application + 'static,
    E: Executor + 'static,
    C: Compositor<Renderer = A::Renderer> + 'static,
    A::Theme: StyleSheet,
{
    use futures::task;
    use futures::Future;
    use winit::event_loop::EventLoopBuilder;

    let mut debug = Debug::new();
    debug.startup_started();

    let resize_border = settings.window.resize_border;

    #[cfg(feature = "trace")]
    let _ = info_span!("Application", "RUN").entered();

    let event_loop = EventLoopBuilder::with_user_event()
        .build()
        .expect("Create event loop");
    let proxy = event_loop.create_proxy();

    let runtime = {
        let proxy = Proxy::new(event_loop.create_proxy());
        let executor = E::new().map_err(Error::ExecutorCreationFailed)?;

        Runtime::new(executor, proxy)
    };

    let (application, init_command) = {
        let flags = settings.flags;

        runtime.enter(|| A::new(flags))
    };

    #[cfg(target_arch = "wasm32")]
    let target = settings.window.platform_specific.target.clone();

    let should_be_visible = settings.window.visible;
    let exit_on_close_request = settings.window.exit_on_close_request;

    let builder = conversion::window_settings(
        settings.window,
        &application.title(),
        event_loop.primary_monitor(),
        settings.id,
    )
    .with_visible(false);

    log::debug!("Window builder: {builder:#?}");

    let window = Arc::new(
        builder
            .build(&event_loop)
            .map_err(Error::WindowCreationFailed)?,
    );

    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::WindowExtWebSys;

        let canvas = window.canvas().expect("Get window canvas");

        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let body = document.body().unwrap();

        let target = target.and_then(|target| {
            body.query_selector(&format!("#{target}"))
                .ok()
                .unwrap_or(None)
        });

        match target {
            Some(node) => {
                let _ = node
                    .replace_with_with_node_1(&canvas)
                    .expect(&format!("Could not replace #{}", node.id()));
            }
            None => {
                let _ = body
                    .append_child(&canvas)
                    .expect("Append canvas to HTML body");
            }
        };
    }

    let compositor = C::new(compositor_settings, window.clone())?;
    let mut renderer = compositor.create_renderer();

    for font in settings.fonts {
        use crate::core::text::Renderer;

        renderer.load_font(font);
    }

    let (mut event_sender, event_receiver) = mpsc::unbounded();
    let (control_sender, mut control_receiver) = mpsc::unbounded();

    let mut instance = Box::pin(run_instance::<A, E, C>(
        application,
        compositor,
        renderer,
        runtime,
        proxy,
        debug,
        event_receiver,
        control_sender,
        init_command,
        window,
        should_be_visible,
        exit_on_close_request,
        resize_border,
    ));

    let mut context = task::Context::from_waker(task::noop_waker_ref());

    let _ = event_loop.run(move |event, event_loop| {
        if event_loop.exiting() {
            return;
        }

        event_sender.start_send(event).expect("Send event");

        let poll = instance.as_mut().poll(&mut context);

        match poll {
            task::Poll::Pending => {
                if let Ok(Some(flow)) = control_receiver.try_next() {
                    event_loop.set_control_flow(flow);
                }
            }
            task::Poll::Ready(_) => {
                event_loop.exit();
            }
        };
    });

    Ok(())
}

async fn run_instance<A, E, C>(
    mut application: A,
    mut compositor: C,
    mut renderer: A::Renderer,
    mut runtime: Runtime<
        E,
        Proxy<UserEventWrapper<A::Message>>,
        UserEventWrapper<A::Message>,
    >,
    mut proxy: winit::event_loop::EventLoopProxy<UserEventWrapper<A::Message>>,
    mut debug: Debug,
    mut event_receiver: mpsc::UnboundedReceiver<
        winit::event::Event<UserEventWrapper<A::Message>>,
    >,
    mut control_sender: mpsc::UnboundedSender<winit::event_loop::ControlFlow>,
    init_command: Command<A::Message>,
    window: Arc<winit::window::Window>,
    should_be_visible: bool,
    exit_on_close_request: bool,
    resize_border: u32,
) where
    A: Application + 'static,
    E: Executor + 'static,
    C: Compositor<Renderer = A::Renderer> + 'static,
    A::Theme: StyleSheet,
    A::Message: Send + 'static,
{
    use winit::event;
    use winit::event_loop::ControlFlow;

    let mut state = State::new(&application, &window);
    let mut viewport_version = state.viewport_version();
    let physical_size = state.physical_size();

    let mut clipboard =
        Clipboard::connect(&window, crate::proxy::Proxy::new(proxy.clone()));
    let mut cache = user_interface::Cache::default();
    let mut surface = compositor.create_surface(
        window.clone(),
        physical_size.width,
        physical_size.height,
    );
    let mut should_exit = false;

    if should_be_visible {
        window.set_visible(true);
    }

    run_command(
        &application,
        &mut compositor,
        &mut surface,
        &mut cache,
        &state,
        &mut renderer,
        init_command,
        &mut runtime,
        &mut clipboard,
        &mut should_exit,
        &mut proxy,
        &mut debug,
        &window,
    );
    runtime.track(
        application
            .subscription()
            .map(subscription_map::<A, E>)
            .into_recipes(),
    );

    let mut user_interface = ManuallyDrop::new(build_user_interface(
        &application,
        cache,
        &mut renderer,
        state.logical_size(),
        &mut debug,
    ));

    let mut prev_dnd_rectangles_count = 0;

    // Creates closure for handling the window drag resize state with winit.
    let mut drag_resize_window_func = drag_resize::event_func(
        &window,
        resize_border as f64 * window.scale_factor(),
    );

    let mut mouse_interaction = mouse::Interaction::default();
    let mut events = Vec::new();
    let mut messages = Vec::new();
    let mut redraw_pending = false;
    #[cfg(feature = "a11y")]
    let mut commands: Vec<Command<A::Message>> = Vec::new();

    #[cfg(feature = "a11y")]
    let (window_a11y_id, adapter, mut a11y_enabled) = {
        let node_id = core::id::window_node_id();

        use iced_accessibility::accesskit::{
            NodeBuilder, NodeId, Role, Tree, TreeUpdate,
        };
        use iced_accessibility::accesskit_winit::Adapter;
        let title = state.title().to_string();
        let proxy_clone = proxy.clone();
        (
            node_id,
            Adapter::new(
                &window,
                move || {
                    let _ =
                        proxy_clone.send_event(UserEventWrapper::A11yEnabled);
                    let mut node = NodeBuilder::new(Role::Window);
                    node.set_name(title.clone());
                    let node = node.build(&mut iced_accessibility::accesskit::NodeClassSet::lock_global());
                    let root = NodeId(node_id);
                    TreeUpdate {
                        nodes: vec![(root, node)],
                        tree: Some(Tree::new(root)),
                        focus: root,
                    }
                },
                proxy.clone(),
            ),
            false,
        )
    };

    debug.startup_finished();

    while let Some(event) = event_receiver.next().await {
        match event {
            event::Event::NewEvents(
                event::StartCause::Init
                | event::StartCause::ResumeTimeReached { .. },
            ) if !redraw_pending => {
                window.request_redraw();
                redraw_pending = true;
            }
            event::Event::PlatformSpecific(event::PlatformSpecific::MacOS(
                event::MacOS::ReceivedUrl(url),
            )) => {
                use crate::core::event;

                events.push(Event::PlatformSpecific(
                    event::PlatformSpecific::MacOS(event::MacOS::ReceivedUrl(
                        url,
                    )),
                ));
            }
            event::Event::UserEvent(message) => {
                match message {
                    UserEventWrapper::Message(m) => messages.push(m),
                    #[cfg(feature = "a11y")]
                    UserEventWrapper::A11y(request) => {
                        match request.request.action {
                            iced_accessibility::accesskit::Action::Focus => {
                                commands.push(Command::widget(focus(
                                    core::widget::Id::from(u128::from(
                                        request.request.target.0,
                                    )
                                        as u64),
                                )));
                            }
                            _ => {}
                        }
                        events.push(conversion::a11y(request.request));
                    }
                    #[cfg(feature = "a11y")]
                    UserEventWrapper::A11yEnabled => a11y_enabled = true,
                    UserEventWrapper::StartDnd {
                        internal,
                        source_surface: _, // not needed if there is only one window
                        icon_surface,
                        content,
                        actions,
                    } => {
                        let mut renderer = compositor.create_renderer();
                        let icon_surface = icon_surface
                            .map(|i| {
                                let i: Box<dyn Any> = i;
                                i
                            })
                            .and_then(|i| {
                                i.downcast::<Arc<(
                                    core::Element<
                                        'static,
                                        A::Message,
                                        A::Theme,
                                        A::Renderer,
                                    >,
                                    core::widget::tree::State,
                                )>>()
                                .ok()
                            })
                            .map(|e| {
                                let e = Arc::into_inner(*e).unwrap();
                                let (mut e, widget_state) = e;
                                let lim = core::layout::Limits::new(
                                    Size::new(1., 1.),
                                    Size::new(
                                        state.viewport().physical_width()
                                            as f32,
                                        state.viewport().physical_height()
                                            as f32,
                                    ),
                                );

                                let mut tree = core::widget::Tree {
                                    id: e.as_widget().id(),
                                    tag: e.as_widget().tag(),
                                    state: widget_state,
                                    children: e.as_widget().children(),
                                };

                                let size = e
                                    .as_widget()
                                    .layout(&mut tree, &renderer, &lim);
                                e.as_widget_mut().diff(&mut tree);

                                let size = lim.resolve(
                                    Length::Shrink,
                                    Length::Shrink,
                                    size.size(),
                                );
                                let mut surface = compositor.create_surface(
                                    window.clone(),
                                    size.width.ceil() as u32,
                                    size.height.ceil() as u32,
                                );
                                let viewport = Viewport::with_logical_size(
                                    size,
                                    state.viewport().scale_factor(),
                                );

                                let mut ui = UserInterface::build(
                                    e,
                                    size,
                                    user_interface::Cache::default(),
                                    &mut renderer,
                                );
                                _ = ui.draw(
                                    &mut renderer,
                                    state.theme(),
                                    &renderer::Style {
                                        icon_color: state.icon_color(),
                                        text_color: state.text_color(),
                                        scale_factor: state.scale_factor(),
                                    },
                                    Default::default(),
                                );
                                let mut bytes = compositor.screenshot(
                                    &mut renderer,
                                    &mut surface,
                                    &viewport,
                                    core::Color::TRANSPARENT,
                                    &debug.overlay(),
                                );
                                for pix in bytes.chunks_exact_mut(4) {
                                    // rgba -> argb little endian
                                    pix.swap(0, 2);
                                }
                                Icon::Buffer {
                                    data: Arc::new(bytes),
                                    width: viewport.physical_width(),
                                    height: viewport.physical_height(),
                                    transparent: true,
                                }
                            });

                        clipboard.start_dnd_winit(
                            internal,
                            DndSurface(Arc::new(Box::new(window.clone()))),
                            icon_surface,
                            content,
                            actions,
                        );
                    }
                    UserEventWrapper::Dnd(e) => events.push(Event::Dnd(e)),
                };
            }
            event::Event::WindowEvent {
                event: event::WindowEvent::RedrawRequested { .. },
                ..
            } => {
                let physical_size = state.physical_size();

                if physical_size.width == 0 || physical_size.height == 0 {
                    continue;
                }

                let current_viewport_version = state.viewport_version();

                if viewport_version != current_viewport_version {
                    let logical_size = state.logical_size();

                    debug.layout_started();
                    user_interface = ManuallyDrop::new(
                        ManuallyDrop::into_inner(user_interface)
                            .relayout(logical_size, &mut renderer),
                    );
                    debug.layout_finished();

                    compositor.configure_surface(
                        &mut surface,
                        physical_size.width,
                        physical_size.height,
                    );

                    viewport_version = current_viewport_version;
                }

                // TODO: Avoid redrawing all the time by forcing widgets to
                // request redraws on state changes
                //
                // Then, we can use the `interface_state` here to decide if a redraw
                // is needed right away, or simply wait until a specific time.
                let redraw_event = Event::Window(
                    window::Id::MAIN,
                    window::Event::RedrawRequested(Instant::now()),
                );

                let (interface_state, _) = user_interface.update(
                    &[redraw_event.clone()],
                    state.cursor(),
                    &mut renderer,
                    &mut clipboard,
                    &mut messages,
                );

                let _ = control_sender.start_send(match interface_state {
                    user_interface::State::Updated {
                        redraw_request: Some(redraw_request),
                    } => match redraw_request {
                        window::RedrawRequest::NextFrame => {
                            window.request_redraw();

                            ControlFlow::Wait
                        }
                        window::RedrawRequest::At(at) => {
                            ControlFlow::WaitUntil(at)
                        }
                    },
                    _ => ControlFlow::Wait,
                });

                runtime.broadcast(redraw_event, core::event::Status::Ignored);

                debug.draw_started();
                let new_mouse_interaction = user_interface.draw(
                    &mut renderer,
                    state.theme(),
                    &renderer::Style {
                        icon_color: state.icon_color(),
                        text_color: state.text_color(),
                        scale_factor: state.scale_factor(),
                    },
                    state.cursor(),
                );

                debug.draw_finished();

                if new_mouse_interaction != mouse_interaction {
                    window.set_cursor_icon(conversion::mouse_interaction(
                        new_mouse_interaction,
                    ));

                    mouse_interaction = new_mouse_interaction;
                }

                redraw_pending = false;

                let physical_size = state.physical_size();

                if physical_size.width == 0 || physical_size.height == 0 {
                    continue;
                }

                #[cfg(feature = "a11y")]
                if a11y_enabled {
                    use iced_accessibility::{
                        accesskit::{
                            NodeBuilder, NodeId, Role, Tree, TreeUpdate,
                        },
                        A11yId, A11yNode, A11yTree,
                    };
                    // TODO send a11y tree
                    let child_tree = user_interface.a11y_nodes(state.cursor());
                    let mut root = NodeBuilder::new(Role::Window);
                    root.set_name(state.title());

                    let window_tree = A11yTree::node_with_child_tree(
                        A11yNode::new(root, window_a11y_id),
                        child_tree,
                    );
                    let tree = Tree::new(NodeId(window_a11y_id));
                    let mut current_operation =
                        Some(Box::new(OperationWrapper::Id(Box::new(
                            operation::focusable::find_focused(),
                        ))));

                    let mut focus = None;
                    while let Some(mut operation) = current_operation.take() {
                        user_interface.operate(&renderer, operation.as_mut());

                        match operation.finish() {
                            operation::Outcome::None => {}
                            operation::Outcome::Some(message) => match message {
                                operation::OperationOutputWrapper::Message(
                                    _,
                                ) => {
                                    unimplemented!();
                                }
                                operation::OperationOutputWrapper::Id(id) => {
                                    focus = Some(A11yId::from(id));
                                }
                            },
                            operation::Outcome::Chain(next) => {
                                current_operation = Some(Box::new(
                                    OperationWrapper::Wrapper(next),
                                ));
                            }
                        }
                    }

                    log::debug!(
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
                    // TODO maybe optimize this?
                    let focus = focus
                        .filter(|f_id| window_tree.contains(f_id))
                        .map(|id| id.into())
                        .unwrap_or_else(|| tree.root);
                    adapter.update_if_active(|| TreeUpdate {
                        nodes: window_tree.into(),
                        tree: Some(tree),
                        focus,
                    });
                }

                debug.render_started();
                match compositor.present(
                    &mut renderer,
                    &mut surface,
                    state.viewport(),
                    state.background_color(),
                    &debug.overlay(),
                ) {
                    Ok(()) => {
                        debug.render_finished();

                        // TODO: Handle animations!
                        // Maybe we can use `ControlFlow::WaitUntil` for this.
                    }
                    Err(error) => match error {
                        // This is an unrecoverable error.
                        compositor::SurfaceError::OutOfMemory => {
                            panic!("{error:?}");
                        }
                        _ => {
                            debug.render_finished();

                            // Try rendering again next frame.
                            window.request_redraw();
                        }
                    },
                }
            }
            event::Event::WindowEvent {
                event: window_event,
                ..
            } => {
                // Initiates a drag resize window state when found.
                if let Some(func) = drag_resize_window_func.as_mut() {
                    if func(&window, &window_event) {
                        continue;
                    }
                }

                if requests_exit(&window_event, state.modifiers())
                    && exit_on_close_request
                {
                    break;
                }

                state.update(&window, &window_event, &mut debug);

                if let Some(event) = conversion::window_event(
                    window::Id::MAIN,
                    window_event,
                    state.scale_factor(),
                    state.modifiers(),
                ) {
                    events.push(event);
                }
            }
            event::Event::AboutToWait => {
                if events.is_empty() && messages.is_empty() {
                    continue;
                }

                debug.event_processing_started();

                let (interface_state, statuses) = user_interface.update(
                    &events,
                    state.cursor(),
                    &mut renderer,
                    &mut clipboard,
                    &mut messages,
                );

                debug.event_processing_finished();

                for (event, status) in
                    events.drain(..).zip(statuses.into_iter())
                {
                    runtime.broadcast(event, status);
                }

                if !messages.is_empty()
                    || matches!(
                        interface_state,
                        user_interface::State::Outdated
                    )
                {
                    let mut cache =
                        ManuallyDrop::into_inner(user_interface).into_cache();

                    // Update application
                    update(
                        &mut application,
                        &mut compositor,
                        &mut surface,
                        &mut cache,
                        &mut state,
                        &mut renderer,
                        &mut runtime,
                        &mut clipboard,
                        &mut should_exit,
                        &mut proxy,
                        &mut debug,
                        &mut messages,
                        &window,
                    );

                    user_interface = ManuallyDrop::new(build_user_interface(
                        &application,
                        cache,
                        &mut renderer,
                        state.logical_size(),
                        &mut debug,
                    ));

                    let dnd_rectangles = user_interface
                        .dnd_rectangles(prev_dnd_rectangles_count, &renderer);
                    let new_dnd_rectangles_count =
                        dnd_rectangles.as_ref().len();

                    if new_dnd_rectangles_count > 0
                        || prev_dnd_rectangles_count > 0
                    {
                        clipboard.register_dnd_destination(
                            DndSurface(Arc::new(Box::new(window.clone()))),
                            dnd_rectangles.into_rectangles(),
                        );
                    }

                    prev_dnd_rectangles_count = new_dnd_rectangles_count;

                    if should_exit {
                        break;
                    }
                }

                if !redraw_pending {
                    window.request_redraw();
                    redraw_pending = true;
                }
            }
            _ => {}
        }
    }

    // Manually drop the user interface
    drop(ManuallyDrop::into_inner(user_interface));
}

/// Returns true if the provided event should cause an [`Application`] to
/// exit.
pub fn requests_exit(
    event: &winit::event::WindowEvent,
    _modifiers: winit::keyboard::ModifiersState,
) -> bool {
    use winit::event::WindowEvent;

    match event {
        WindowEvent::CloseRequested => true,
        #[cfg(target_os = "macos")]
        WindowEvent::KeyboardInput {
            event:
                winit::event::KeyEvent {
                    logical_key: winit::keyboard::Key::Character(c),
                    state: winit::event::ElementState::Pressed,
                    ..
                },
            ..
        } if c == "q" && _modifiers.super_key() => true,
        _ => false,
    }
}

/// Builds a [`UserInterface`] for the provided [`Application`], logging
/// [`struct@Debug`] information accordingly.
pub fn build_user_interface<'a, A: Application>(
    application: &'a A,
    cache: user_interface::Cache,
    renderer: &mut A::Renderer,
    size: Size,
    debug: &mut Debug,
) -> UserInterface<'a, A::Message, A::Theme, A::Renderer>
where
    A::Theme: StyleSheet,
{
    debug.view_started();
    let view = application.view();
    debug.view_finished();

    debug.layout_started();
    let user_interface = UserInterface::build(view, size, cache, renderer);
    debug.layout_finished();

    user_interface
}

/// subscription mapper helper
pub fn subscription_map<A, E>(e: A::Message) -> UserEventWrapper<A::Message>
where
    A: Application,
    E: Executor,
    <A as Program>::Theme: StyleSheet,
{
    UserEventWrapper::Message(e)
}

/// Updates an [`Application`] by feeding it the provided messages, spawning any
/// resulting [`Command`], and tracking its [`Subscription`].
pub fn update<A: Application + 'static, C, E: Executor + 'static>(
    application: &mut A,
    compositor: &mut C,
    surface: &mut C::Surface,
    cache: &mut user_interface::Cache,
    state: &mut State<A>,
    renderer: &mut A::Renderer,
    runtime: &mut Runtime<
        E,
        Proxy<UserEventWrapper<A::Message>>,
        UserEventWrapper<A::Message>,
    >,
    clipboard: &mut Clipboard<A::Message>,
    should_exit: &mut bool,
    proxy: &mut winit::event_loop::EventLoopProxy<UserEventWrapper<A::Message>>,
    debug: &mut Debug,
    messages: &mut Vec<A::Message>,
    window: &winit::window::Window,
) where
    C: Compositor<Renderer = A::Renderer> + 'static,
    A::Theme: StyleSheet,
{
    for message in messages.drain(..) {
        debug.log_message(&message);

        debug.update_started();
        let command = runtime.enter(|| application.update(message));
        debug.update_finished();

        run_command(
            application,
            compositor,
            surface,
            cache,
            state,
            renderer,
            command,
            runtime,
            clipboard,
            should_exit,
            proxy,
            debug,
            window,
        );
    }

    state.synchronize(application, window);

    runtime.track(
        application
            .subscription()
            .map(subscription_map::<A, E>)
            .into_recipes(),
    );
}

/// Runs the actions of a [`Command`].
pub fn run_command<A, C, E>(
    application: &A,
    compositor: &mut C,
    surface: &mut C::Surface,
    cache: &mut user_interface::Cache,
    state: &State<A>,
    renderer: &mut A::Renderer,
    command: Command<A::Message>,
    runtime: &mut Runtime<
        E,
        Proxy<UserEventWrapper<A::Message>>,
        UserEventWrapper<A::Message>,
    >,
    clipboard: &mut Clipboard<A::Message>,
    should_exit: &mut bool,
    proxy: &mut winit::event_loop::EventLoopProxy<UserEventWrapper<A::Message>>,
    debug: &mut Debug,
    window: &winit::window::Window,
) where
    A: Application,
    E: Executor,
    C: Compositor<Renderer = A::Renderer> + 'static,
    A::Theme: StyleSheet,
{
    use crate::runtime::command;
    use crate::runtime::system;
    use crate::runtime::window;

    for action in command.actions() {
        match action {
            command::Action::Future(future) => {
                runtime.spawn(Box::pin(future.map(UserEventWrapper::Message)));
            }
            command::Action::Stream(stream) => {
                runtime.run(Box::pin(stream.map(UserEventWrapper::Message)));
            }
            command::Action::Clipboard(action) => match action {
                clipboard::Action::Read(tag) => {
                    let message = tag(clipboard.read());

                    proxy
                        .send_event(UserEventWrapper::Message(message))
                        .expect("Send message to event loop");
                }
                clipboard::Action::Write(contents) => {
                    clipboard.write(contents);
                }
                clipboard::Action::WriteData(contents) => {
                    clipboard.write_data(ClipboardStoreData(contents))
                }
                clipboard::Action::ReadData(allowed, to_msg) => {
                    let contents = clipboard.read_data(allowed);
                    let message = to_msg(contents);
                    _ = proxy.send_event(UserEventWrapper::Message(message));
                }
                clipboard::Action::ReadPrimary(s_to_msg) => {
                    let contents = clipboard.read_primary();
                    let message = s_to_msg(contents);
                    _ = proxy.send_event(UserEventWrapper::Message(message));
                }
                clipboard::Action::WritePrimary(content) => {
                    clipboard.write_primary(content)
                }
                clipboard::Action::WritePrimaryData(content) => {
                    clipboard.write_primary_data(ClipboardStoreData(content))
                }
                clipboard::Action::ReadPrimaryData(a, to_msg) => {
                    let contents = clipboard.read_primary_data(a);
                    let message = to_msg(contents);
                    _ = proxy.send_event(UserEventWrapper::Message(message));
                }
            },
            command::Action::Window(action) => match action {
                window::Action::Close(_id) => {
                    *should_exit = true;
                }
                window::Action::Drag(_id) => {
                    let _res = window.drag_window();
                }
                window::Action::Spawn { .. } => {
                    log::warn!(
                        "Spawning a window is only available with \
                        multi-window applications."
                    );
                }
                window::Action::Resize(_id, size) => {
                    let _ =
                        window.request_inner_size(winit::dpi::LogicalSize {
                            width: size.width,
                            height: size.height,
                        });
                }
                window::Action::FetchSize(_id, callback) => {
                    let size =
                        window.inner_size().to_logical(window.scale_factor());

                    proxy
                        .send_event(UserEventWrapper::Message(callback(
                            Size::new(size.width, size.height),
                        )))
                        .expect("Send message to event loop");
                }
                window::Action::FetchMaximized(_id, callback) => {
                    proxy
                        .send_event(UserEventWrapper::Message(callback(
                            window.is_maximized(),
                        )))
                        .expect("Send message to event loop");
                }
                window::Action::Maximize(_id, maximized) => {
                    window.set_maximized(maximized);
                }
                window::Action::FetchMinimized(_id, callback) => {
                    proxy
                        .send_event(UserEventWrapper::Message(callback(
                            window.is_minimized(),
                        )))
                        .expect("Send message to event loop");
                }
                window::Action::Minimize(_id, minimized) => {
                    window.set_minimized(minimized);
                }
                window::Action::Move(_id, position) => {
                    window.set_outer_position(winit::dpi::LogicalPosition {
                        x: position.x,
                        y: position.y,
                    });
                }
                window::Action::ChangeMode(_id, mode) => {
                    window.set_visible(conversion::visible(mode));
                    window.set_fullscreen(conversion::fullscreen(
                        window.current_monitor(),
                        mode,
                    ));
                }
                window::Action::ChangeIcon(_id, icon) => {
                    window.set_window_icon(conversion::icon(icon));
                }
                window::Action::FetchMode(_id, tag) => {
                    let mode = if window.is_visible().unwrap_or(true) {
                        conversion::mode(window.fullscreen())
                    } else {
                        core::window::Mode::Hidden
                    };

                    proxy
                        .send_event(UserEventWrapper::Message(tag(mode)))
                        .expect("Send message to event loop");
                }
                window::Action::ToggleMaximize(_id) => {
                    window.set_maximized(!window.is_maximized());
                }
                window::Action::ToggleDecorations(_id) => {
                    window.set_decorations(!window.is_decorated());
                }
                window::Action::RequestUserAttention(_id, user_attention) => {
                    window.request_user_attention(
                        user_attention.map(conversion::user_attention),
                    );
                }
                window::Action::GainFocus(_id) => {
                    window.focus_window();
                }
                window::Action::ChangeLevel(_id, level) => {
                    window.set_window_level(conversion::window_level(level));
                }
                window::Action::ShowWindowMenu(_id) => {
                    if let mouse::Cursor::Available(point) = state.cursor() {
                        window.show_window_menu(winit::dpi::LogicalPosition {
                            x: point.x,
                            y: point.y,
                        });
                    }
                }
                window::Action::FetchId(_id, tag) => {
                    proxy
                        .send_event(UserEventWrapper::Message(tag(window
                            .id()
                            .into())))
                        .expect("Send message to event loop");
                }
                window::Action::Screenshot(_id, tag) => {
                    let bytes = compositor.screenshot(
                        renderer,
                        surface,
                        state.viewport(),
                        state.background_color(),
                        &debug.overlay(),
                    );

                    proxy
                        .send_event(UserEventWrapper::Message(tag(
                            window::Screenshot::new(
                                bytes,
                                state.physical_size(),
                            ),
                        )))
                        .expect("Send message to event loop.");
                }
            },
            command::Action::System(action) => match action {
                system::Action::QueryInformation(_tag) => {
                    #[cfg(feature = "system")]
                    {
                        let graphics_info = compositor.fetch_information();
                        let proxy = proxy.clone();

                        let _ = std::thread::spawn(move || {
                            let information =
                                crate::system::information(graphics_info);

                            let message = _tag(information);

                            proxy
                                .send_event(UserEventWrapper::Message(message))
                                .expect("Send message to event loop")
                        });
                    }
                }
            },
            command::Action::Widget(action) => {
                let mut current_cache = std::mem::take(cache);
                let mut current_operation =
                    Some(Box::new(OperationWrapper::Message(action)));
                let mut user_interface = build_user_interface(
                    application,
                    current_cache,
                    renderer,
                    state.logical_size(),
                    debug,
                );

                while let Some(mut operation) = current_operation.take() {
                    user_interface.operate(renderer, operation.as_mut());

                    match operation.finish() {
                        operation::Outcome::None => {}
                        operation::Outcome::Some(message) => {
                            match message {
                                operation::OperationOutputWrapper::Message(
                                    m,
                                ) => {
                                    proxy
                                        .send_event(UserEventWrapper::Message(
                                            m,
                                        ))
                                        .expect("Send message to event loop");
                                }
                                operation::OperationOutputWrapper::Id(_) => {
                                    // TODO ASHLEY should not ever happen, should this panic!()?
                                }
                            }
                        }
                        operation::Outcome::Chain(next) => {
                            current_operation =
                                Some(Box::new(OperationWrapper::Wrapper(next)));
                        }
                    }
                }

                current_cache = user_interface.into_cache();
                *cache = current_cache;
            }
            command::Action::LoadFont { bytes, tagger } => {
                use crate::core::text::Renderer;

                // TODO: Error handling (?)
                renderer.load_font(bytes);

                proxy
                    .send_event(UserEventWrapper::Message(tagger(Ok(()))))
                    .expect("Send message to event loop");
            }
            command::Action::PlatformSpecific(_) => todo!(),
            command::Action::Dnd(a) => match a {
                iced_runtime::dnd::DndAction::RegisterDndDestination {
                    surface,
                    rectangles,
                } => {
                    clipboard.register_dnd_destination(surface, rectangles);
                }
                iced_runtime::dnd::DndAction::StartDnd {
                    internal,
                    source_surface,
                    icon_surface,
                    content,
                    actions,
                } => clipboard.start_dnd(
                    internal,
                    source_surface,
                    icon_surface,
                    content,
                    actions,
                ),
                iced_runtime::dnd::DndAction::EndDnd => {
                    clipboard.end_dnd();
                }
                iced_runtime::dnd::DndAction::PeekDnd(m, to_msg) => {
                    let data = clipboard.peek_dnd(m);
                    let message = to_msg(data);
                    proxy
                        .send_event(UserEventWrapper::Message(message))
                        .expect("Send message to event loop");
                }
                iced_runtime::dnd::DndAction::SetAction(a) => {
                    clipboard.set_action(a);
                }
            },
        }
    }
}
