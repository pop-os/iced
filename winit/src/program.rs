//! Create interactive, native cross-platform applications for WGPU.
#[path = "application/drag_resize.rs"]
mod drag_resize;
mod state;
mod window_manager;

pub use runtime::{default, Appearance, DefaultStyle};

use crate::conversion;
use crate::core;
use crate::core::clipboard::Kind;
use crate::core::mouse;
use crate::core::renderer;
use crate::core::time::Instant;
use crate::core::widget::operation;
use crate::core::widget::Operation;
use crate::core::window;
use crate::core::Clipboard as CoreClipboard;
use crate::core::Length;
use crate::core::{Color, Element, Point, Size, Theme};
use crate::futures::futures::channel::mpsc;
use crate::futures::futures::channel::oneshot;
use crate::futures::futures::task;
use crate::futures::futures::{Future, StreamExt};
use crate::futures::subscription::{self, Subscription};
use crate::futures::{Executor, Runtime};
use crate::graphics;
use crate::graphics::{compositor, Compositor};
use crate::runtime::user_interface::{self, UserInterface};
use crate::runtime::Debug;
use crate::runtime::{self, Action, Task};
use crate::{Clipboard, Error, Proxy, Settings};
use dnd::DndSurface;
use dnd::Icon;
use iced_futures::core::widget::operation::search_id;
use iced_graphics::Viewport;
use iced_runtime::futures::futures::FutureExt;
pub use state::State;
use window_clipboard::mime::ClipboardStoreData;
use winit::raw_window_handle::HasWindowHandle;

use window_manager::WindowManager;

use rustc_hash::FxHashMap;
use std::any::Any;
use std::borrow::Cow;
use std::mem::ManuallyDrop;
use std::sync::Arc;

/// An interactive, native, cross-platform, multi-windowed application.
///
/// This trait is the main entrypoint of multi-window Iced. Once implemented, you can run
/// your GUI application by simply calling [`run`]. It will run in
/// its own window.
///
/// A [`Program`] can execute asynchronous actions by returning a
/// [`Task`] in some of its methods.
///
/// When using a [`Program`] with the `debug` feature enabled, a debug view
/// can be toggled by pressing `F12`.
pub trait Program
where
    Self: Sized,
    Self::Theme: DefaultStyle,
{
    /// The type of __messages__ your [`Program`] will produce.
    type Message: std::fmt::Debug + Send;

    /// The theme used to draw the [`Program`].
    type Theme;

    /// The [`Executor`] that will run commands and subscriptions.
    ///
    /// The [default executor] can be a good starting point!
    ///
    /// [`Executor`]: Self::Executor
    /// [default executor]: crate::futures::backend::default::Executor
    type Executor: Executor;

    /// The graphics backend to use to draw the [`Program`].
    type Renderer: core::Renderer + core::text::Renderer;

    /// The data needed to initialize your [`Program`].
    type Flags;

    /// Initializes the [`Program`] with the flags provided to
    /// [`run`] as part of the [`Settings`].
    ///
    /// Here is where you should return the initial state of your app.
    ///
    /// Additionally, you can return a [`Task`] if you need to perform some
    /// async action in the background on startup. This is useful if you want to
    /// load state from a file, perform an initial HTTP request, etc.
    fn new(flags: Self::Flags) -> (Self, Task<Self::Message>);

    /// Returns the current title of the [`Program`].
    ///
    /// This title can be dynamic! The runtime will automatically update the
    /// title of your application when necessary.
    fn title(&self, window: window::Id) -> String;

    /// Handles a __message__ and updates the state of the [`Program`].
    ///
    /// This is where you define your __update logic__. All the __messages__,
    /// produced by either user interactions or commands, will be handled by
    /// this method.
    ///
    /// Any [`Task`] returned will be executed immediately in the background by the
    /// runtime.
    fn update(&mut self, message: Self::Message) -> Task<Self::Message>;

    /// Returns the widgets to display in the [`Program`] for the `window`.
    ///
    /// These widgets can produce __messages__ based on user interaction.
    fn view(
        &self,
        window: window::Id,
    ) -> Element<'_, Self::Message, Self::Theme, Self::Renderer>;

    /// Returns the current `Theme` of the [`Program`].
    fn theme(&self, window: window::Id) -> Self::Theme;

    /// Returns the `Style` variation of the `Theme`.
    fn style(&self, theme: &Self::Theme) -> Appearance {
        theme.default_style()
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

    /// Returns the scale factor of the window of the [`Program`].
    ///
    /// It can be used to dynamically control the size of the UI at runtime
    /// (i.e. zooming).
    ///
    /// For instance, a scale factor of `2.0` will make widgets twice as big,
    /// while a scale factor of `0.5` will shrink them to half their size.
    ///
    /// By default, it returns `1.0`.
    #[allow(unused_variables)]
    fn scale_factor(&self, window: window::Id) -> f64 {
        1.0
    }
}

/// Runs a [`Program`] with an executor, compositor, and the provided
/// settings.
pub fn run<P, C>(
    settings: Settings,
    graphics_settings: graphics::Settings,
    window_settings: Option<window::Settings>,
    flags: P::Flags,
) -> Result<(), Error>
where
    P: Program + 'static,
    C: Compositor<Renderer = P::Renderer> + 'static,
    P::Theme: DefaultStyle,
{
    use winit::event_loop::EventLoop;

    let mut debug = Debug::new();
    debug.startup_started();

    let event_loop = EventLoop::with_user_event()
        .build()
        .expect("Create event loop");

    let (proxy, worker) = Proxy::new(event_loop.create_proxy());

    let mut runtime = {
        let executor =
            P::Executor::new().map_err(Error::ExecutorCreationFailed)?;
        executor.spawn(worker);

        Runtime::new(executor, proxy.clone())
    };

    let (program, task) = runtime.enter(|| P::new(flags));
    let is_daemon = window_settings.is_none();

    let task = if let Some(window_settings) = window_settings {
        let mut task = Some(task);

        let (_id, open) = runtime::window::open(window_settings);

        open.then(move |_| task.take().unwrap_or(Task::none()))
    } else {
        task
    };

    if let Some(stream) = runtime::task::into_stream(task) {
        runtime.run(stream);
    }

    runtime.track(subscription::into_recipes(
        program.subscription().map(Action::Output),
    ));

    let (boot_sender, boot_receiver) = oneshot::channel();
    let (event_sender, event_receiver) = mpsc::unbounded();
    let (control_sender, control_receiver) = mpsc::unbounded();

    let instance = Box::pin(run_instance::<P, C>(
        program,
        runtime,
        proxy.clone(),
        debug,
        boot_receiver,
        event_receiver,
        control_sender.clone(),
        is_daemon,
    ));

    let context = task::Context::from_waker(task::noop_waker_ref());

    struct Runner<Message: 'static, F, C> {
        instance: std::pin::Pin<Box<F>>,
        context: task::Context<'static>,
        id: Option<String>,
        boot: Option<BootConfig<C>>,
        sender: mpsc::UnboundedSender<Event<Action<Message>>>,
        receiver: mpsc::UnboundedReceiver<Control>,
        error: Option<Error>,

        #[cfg(target_arch = "wasm32")]
        is_booted: std::rc::Rc<std::cell::RefCell<bool>>,
        #[cfg(target_arch = "wasm32")]
        queued_events: Vec<Event<Action<Message>>>,
    }

    struct BootConfig<C> {
        sender: oneshot::Sender<Boot<C>>,
        fonts: Vec<Cow<'static, [u8]>>,
        graphics_settings: graphics::Settings,
        control_sender: mpsc::UnboundedSender<Control>,
    }

    let runner = Runner {
        instance,
        context,
        id: settings.id,
        boot: Some(BootConfig {
            sender: boot_sender,
            fonts: settings.fonts,
            graphics_settings,
            control_sender,
        }),
        sender: event_sender,
        receiver: control_receiver,
        error: None,

        #[cfg(target_arch = "wasm32")]
        is_booted: std::rc::Rc::new(std::cell::RefCell::new(false)),
        #[cfg(target_arch = "wasm32")]
        queued_events: Vec::new(),
    };

    impl<Message, F, C> winit::application::ApplicationHandler<Action<Message>>
        for Runner<Message, F, C>
    where
        Message: std::fmt::Debug,
        F: Future<Output = ()>,
        C: Compositor + 'static,
    {
        fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
            let Some(BootConfig {
                sender,
                fonts,
                graphics_settings,
                control_sender,
            }) = self.boot.take()
            else {
                return;
            };

            let window = match event_loop.create_window(
                winit::window::WindowAttributes::default().with_visible(false),
            ) {
                Ok(window) => Arc::new(window),
                Err(error) => {
                    self.error = Some(Error::WindowCreationFailed(error));
                    event_loop.exit();
                    return;
                }
            };

            let clipboard = Clipboard::connect(
                window.clone(),
                crate::clipboard::ControlSender(control_sender.clone()),
            );

            let finish_boot = async move {
                let mut compositor =
                    C::new(graphics_settings, window.clone()).await?;

                for font in fonts {
                    compositor.load_font(font);
                }

                sender
                    .send(Boot {
                        compositor,
                        clipboard,
                    })
                    .ok()
                    .expect("Send boot event");

                Ok::<_, graphics::Error>(())
            };

            #[cfg(not(target_arch = "wasm32"))]
            if let Err(error) =
                crate::futures::futures::executor::block_on(finish_boot)
            {
                self.error = Some(Error::GraphicsCreationFailed(error));
                event_loop.exit();
            }

            #[cfg(target_arch = "wasm32")]
            {
                let is_booted = self.is_booted.clone();

                wasm_bindgen_futures::spawn_local(async move {
                    finish_boot.await.expect("Finish boot!");

                    *is_booted.borrow_mut() = true;
                });
            }
        }

        fn new_events(
            &mut self,
            event_loop: &winit::event_loop::ActiveEventLoop,
            cause: winit::event::StartCause,
        ) {
            if self.boot.is_some() {
                return;
            }
            self.process_event(
                event_loop,
                Event::EventLoopAwakened(winit::event::Event::NewEvents(cause)),
            );
        }

        fn window_event(
            &mut self,
            event_loop: &winit::event_loop::ActiveEventLoop,
            window_id: winit::window::WindowId,
            event: winit::event::WindowEvent,
        ) {
            #[cfg(target_os = "windows")]
            let is_move_or_resize = matches!(
                event,
                winit::event::WindowEvent::Resized(_)
                    | winit::event::WindowEvent::Moved(_)
            );

            self.process_event(
                event_loop,
                Event::EventLoopAwakened(winit::event::Event::WindowEvent {
                    window_id,
                    event,
                }),
            );

            // TODO: Remove when unnecessary
            // On Windows, we emulate an `AboutToWait` event after every `Resized` event
            // since the event loop does not resume during resize interaction.
            // More details: https://github.com/rust-windowing/winit/issues/3272
            #[cfg(target_os = "windows")]
            {
                if is_move_or_resize {
                    self.process_event(
                        event_loop,
                        Event::EventLoopAwakened(
                            winit::event::Event::AboutToWait,
                        ),
                    );
                }
            }
        }

        fn user_event(
            &mut self,
            event_loop: &winit::event_loop::ActiveEventLoop,
            action: Action<Message>,
        ) {
            self.process_event(
                event_loop,
                Event::EventLoopAwakened(winit::event::Event::UserEvent(
                    action,
                )),
            );
        }

        fn received_url(
            &mut self,
            event_loop: &winit::event_loop::ActiveEventLoop,
            url: String,
        ) {
            self.process_event(
                event_loop,
                Event::EventLoopAwakened(
                    winit::event::Event::PlatformSpecific(
                        winit::event::PlatformSpecific::MacOS(
                            winit::event::MacOS::ReceivedUrl(url),
                        ),
                    ),
                ),
            );
        }

        fn about_to_wait(
            &mut self,
            event_loop: &winit::event_loop::ActiveEventLoop,
        ) {
            self.process_event(
                event_loop,
                Event::EventLoopAwakened(winit::event::Event::AboutToWait),
            );
        }
    }

    impl<Message, F, C> Runner<Message, F, C>
    where
        F: Future<Output = ()>,
        C: Compositor,
    {
        fn process_event(
            &mut self,
            event_loop: &winit::event_loop::ActiveEventLoop,
            event: Event<Action<Message>>,
        ) {
            #[cfg(target_arch = "wasm32")]
            if !*self.is_booted.borrow() {
                self.queued_events.push(event);
                return;
            } else if !self.queued_events.is_empty() {
                let queued_events = std::mem::take(&mut self.queued_events);

                // This won't infinitely recurse, since we `mem::take`
                for event in queued_events {
                    self.process_event(event_loop, event);
                }
            }

            if event_loop.exiting() {
                return;
            }

            self.sender.start_send(event).expect("Send event");

            loop {
                let poll = self.instance.as_mut().poll(&mut self.context);

                match poll {
                    task::Poll::Pending => match self.receiver.try_next() {
                        Ok(Some(control)) => match control {
                            Control::ChangeFlow(flow) => {
                                use winit::event_loop::ControlFlow;

                                match (event_loop.control_flow(), flow) {
                                    (
                                        ControlFlow::WaitUntil(current),
                                        ControlFlow::WaitUntil(new),
                                    ) if new < current => {}
                                    (
                                        ControlFlow::WaitUntil(target),
                                        ControlFlow::Wait,
                                    ) if target > Instant::now() => {}
                                    _ => {
                                        event_loop.set_control_flow(flow);
                                    }
                                }
                            }
                            Control::CreateWindow {
                                id,
                                settings,
                                title,
                                monitor,
                                on_open,
                            } => {
                                let exit_on_close_request =
                                    settings.exit_on_close_request;
                                let resize_border = settings.resize_border;

                                let visible = settings.visible;

                                #[cfg(target_arch = "wasm32")]
                                let target =
                                    settings.platform_specific.target.clone();

                                let window = event_loop
                                    .create_window(
                                        conversion::window_attributes(
                                            settings,
                                            &title,
                                            monitor
                                                .or(event_loop
                                                    .primary_monitor()),
                                            self.id.clone(),
                                        )
                                        .with_visible(false),
                                    )
                                    .expect("Create window");

                                #[cfg(target_arch = "wasm32")]
                                {
                                    use winit::platform::web::WindowExtWebSys;

                                    let canvas = window
                                        .canvas()
                                        .expect("Get window canvas");

                                    let _ = canvas.set_attribute(
                                        "style",
                                        "display: block; width: 100%; height: 100%",
                                    );

                                    let window = web_sys::window().unwrap();
                                    let document = window.document().unwrap();
                                    let body = document.body().unwrap();

                                    let target = target.and_then(|target| {
                                        body.query_selector(&format!(
                                            "#{target}"
                                        ))
                                        .ok()
                                        .unwrap_or(None)
                                    });

                                    match target {
                                        Some(node) => {
                                            let _ = node
                                                .replace_with_with_node_1(
                                                    &canvas,
                                                )
                                                .expect(&format!(
                                                    "Could not replace #{}",
                                                    node.id()
                                                ));
                                        }
                                        None => {
                                            let _ = body
                                                .append_child(&canvas)
                                                .expect(
                                                "Append canvas to HTML body",
                                            );
                                        }
                                    };
                                }

                                self.process_event(
                                    event_loop,
                                    Event::WindowCreated {
                                        id,
                                        window,
                                        exit_on_close_request,
                                        make_visible: visible,
                                        on_open,
                                        resize_border,
                                    },
                                );
                            }
                            Control::Exit => {
                                event_loop.exit();
                            }
                            Control::Dnd(e) => {
                                self.process_event(event_loop, Event::Dnd(e));
                            }
                            #[cfg(feature = "a11y")]
                            Control::Accessibility(event) => {
                                self.process_event(
                                    event_loop,
                                    Event::Accessibility(event),
                                );
                            }
                            #[cfg(feature = "a11y")]
                            Control::AccessibilityEnabled(event) => {
                                self.process_event(
                                    event_loop,
                                    Event::AccessibilityEnabled(event),
                                );
                            }
                        },
                        _ => {
                            break;
                        }
                    },
                    task::Poll::Ready(_) => {
                        event_loop.exit();
                        break;
                    }
                };
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut runner = runner;
        let _ = event_loop.run_app(&mut runner);

        runner.error.map(Err).unwrap_or(Ok(()))
    }

    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::EventLoopExtWebSys;
        let _ = event_loop.spawn_app(runner);

        Ok(())
    }
}

struct Boot<C> {
    compositor: C,
    clipboard: Clipboard,
}

#[derive(Debug)]
enum Event<Message: 'static> {
    WindowCreated {
        id: window::Id,
        window: winit::window::Window,
        exit_on_close_request: bool,
        make_visible: bool,
        on_open: oneshot::Sender<window::Id>,
        resize_border: u32,
    },
    Dnd(dnd::DndEvent<dnd::DndSurface>),
    #[cfg(feature = "a11y")]
    Accessibility(iced_accessibility::accesskit::ActionRequest),
    #[cfg(feature = "a11y")]
    AccessibilityEnabled(bool),
    EventLoopAwakened(winit::event::Event<Message>),
}

#[derive(Debug)]
pub(crate) enum Control {
    ChangeFlow(winit::event_loop::ControlFlow),
    Exit,
    CreateWindow {
        id: window::Id,
        settings: window::Settings,
        title: String,
        monitor: Option<winit::monitor::MonitorHandle>,
        on_open: oneshot::Sender<window::Id>,
    },
    Dnd(dnd::DndEvent<dnd::DndSurface>),
    #[cfg(feature = "a11y")]
    Accessibility(iced_accessibility::accesskit::ActionRequest),
    #[cfg(feature = "a11y")]
    AccessibilityEnabled(bool),
}

async fn run_instance<P, C>(
    mut program: P,
    mut runtime: Runtime<P::Executor, Proxy<P::Message>, Action<P::Message>>,
    mut proxy: Proxy<P::Message>,
    mut debug: Debug,
    boot: oneshot::Receiver<Boot<C>>,
    mut event_receiver: mpsc::UnboundedReceiver<Event<Action<P::Message>>>,
    mut control_sender: mpsc::UnboundedSender<Control>,
    is_daemon: bool,
) where
    P: Program + 'static,
    C: Compositor<Renderer = P::Renderer> + 'static,
    P::Theme: DefaultStyle,
{
    use winit::event;
    use winit::event_loop::ControlFlow;

    let Boot {
        mut compositor,
        mut clipboard,
    } = boot.await.expect("Receive boot");

    let mut window_manager = WindowManager::new();
    let mut is_window_opening = !is_daemon;

    let mut events = Vec::new();
    let mut messages = Vec::new();
    let mut actions = 0;

    #[cfg(feature = "a11y")]
    let (window_a11y_id, adapter, mut a11y_enabled) = if let Some((
        main_id,
        title,
        raw,
    )) =
        window_manager.ids().next().and_then(|id| {
            window_manager
                .get(id)
                .map(|w| (id, w.state.title.clone(), w.raw.clone()))
        }) {
        let node_id = core::id::window_node_id();
        use iced_accessibility::accesskit::{
            ActivationHandler, NodeBuilder, NodeId, Role, Tree, TreeUpdate,
        };
        use iced_accessibility::accesskit_winit::Adapter;

        pub struct WinitActivationHandler {
            pub proxy: mpsc::UnboundedSender<Control>,
            pub title: String,
        }

        impl ActivationHandler for WinitActivationHandler {
            fn request_initial_tree(
                &mut self,
            ) -> Option<iced_accessibility::accesskit::TreeUpdate> {
                let node_id = core::id::window_node_id();

                let _ = self
                    .proxy
                    .unbounded_send(Control::AccessibilityEnabled(true));
                let mut node = NodeBuilder::new(Role::Window);
                node.set_name(self.title.clone());
                let node = node.build();
                let root = NodeId(node_id);
                Some(TreeUpdate {
                    nodes: vec![(root, node)],
                    tree: Some(Tree::new(root)),
                    focus: root,
                })
            }
        }

        let activation_handler = WinitActivationHandler {
            proxy: control_sender.clone(),
            title: title.clone(),
        };

        pub struct WinitActionHandler {
            pub proxy: mpsc::UnboundedSender<Control>,
        }

        impl iced_accessibility::accesskit::ActionHandler for WinitActionHandler {
            fn do_action(
                &mut self,
                request: iced_accessibility::accesskit::ActionRequest,
            ) {
                let _ =
                    self.proxy.unbounded_send(Control::Accessibility(request));
            }
        }

        let action_handler = WinitActionHandler {
            proxy: control_sender.clone(),
        };

        pub struct WinitDeactivationHandler {
            pub proxy: mpsc::UnboundedSender<Control>,
        }

        impl iced_accessibility::accesskit::DeactivationHandler
            for WinitDeactivationHandler
        {
            fn deactivate_accessibility(&mut self) {
                let _ = self
                    .proxy
                    .unbounded_send(Control::AccessibilityEnabled(false));
            }
        }

        let deactivation_handler = WinitDeactivationHandler {
            proxy: control_sender.clone(),
        };
        (
            node_id,
            Some(Adapter::with_direct_handlers(
                &raw,
                activation_handler,
                action_handler,
                deactivation_handler,
            )),
            false,
        )
    } else {
        (Default::default(), None, false)
    };

    let mut ui_caches = FxHashMap::default();
    let mut user_interfaces = ManuallyDrop::new(FxHashMap::default());

    let mut cur_dnd_surface: Option<window::Id> = None;

    debug.startup_finished();

    loop {
        // Empty the queue if possible
        let event = if let Ok(event) = event_receiver.try_next() {
            event
        } else {
            event_receiver.next().await
        };

        let Some(event) = event else {
            break;
        };
        let mut cur_dnd_surface: Option<window::Id> = None;

        match event {
            Event::WindowCreated {
                id,
                window,
                exit_on_close_request,
                make_visible,
                on_open,
                resize_border,
            } => {
                let window = window_manager.insert(
                    id,
                    Arc::new(window),
                    &program,
                    &mut compositor,
                    exit_on_close_request,
                    resize_border,
                );

                let logical_size = window.state.logical_size();

                let _ = user_interfaces.insert(
                    id,
                    build_user_interface(
                        &program,
                        user_interface::Cache::default(),
                        &mut window.renderer,
                        logical_size,
                        &mut debug,
                        id,
                    ),
                );
                let _ = ui_caches.insert(id, user_interface::Cache::default());

                if make_visible {
                    window.raw.set_visible(true);
                }

                events.push((
                    Some(id),
                    core::Event::Window(window::Event::Opened {
                        position: window.position(),
                        size: window.size(),
                    }),
                ));

                let _ = on_open.send(id);
                is_window_opening = false;
            }
            Event::EventLoopAwakened(event) => {
                match event {
                    event::Event::NewEvents(
                        event::StartCause::Init
                        | event::StartCause::ResumeTimeReached { .. },
                    ) => {
                        for (_id, window) in window_manager.iter_mut() {
                            window.raw.request_redraw();
                        }
                    }
                    event::Event::PlatformSpecific(
                        event::PlatformSpecific::MacOS(
                            event::MacOS::ReceivedUrl(url),
                        ),
                    ) => {
                        runtime.broadcast(
                            subscription::Event::PlatformSpecific(
                                subscription::PlatformSpecific::MacOS(
                                    subscription::MacOS::ReceivedUrl(url),
                                ),
                            ),
                        );
                    }
                    event::Event::UserEvent(action) => {
                        run_action(
                            action,
                            &program,
                            &mut compositor,
                            &mut events,
                            &mut messages,
                            &mut clipboard,
                            &mut control_sender,
                            &mut debug,
                            &mut user_interfaces,
                            &mut window_manager,
                            &mut ui_caches,
                            &mut is_window_opening,
                        );
                        actions += 1;
                    }
                    event::Event::WindowEvent {
                        window_id: id,
                        event: event::WindowEvent::RedrawRequested,
                        ..
                    } => {
                        let Some((id, window)) =
                            window_manager.get_mut_alias(id)
                        else {
                            continue;
                        };

                        // TODO: Avoid redrawing all the time by forcing widgets to
                        // request redraws on state changes
                        //
                        // Then, we can use the `interface_state` here to decide if a redraw
                        // is needed right away, or simply wait until a specific time.
                        let redraw_event = core::Event::Window(
                            window::Event::RedrawRequested(Instant::now()),
                        );

                        let cursor = window.state.cursor();

                        let ui = user_interfaces
                            .get_mut(&id)
                            .expect("Get user interface");

                        let (ui_state, _) = ui.update(
                            &[redraw_event.clone()],
                            cursor,
                            &mut window.renderer,
                            &mut clipboard,
                            &mut messages,
                        );

                        debug.draw_started();
                        let new_mouse_interaction = ui.draw(
                            &mut window.renderer,
                            window.state.theme(),
                            &renderer::Style {
                                icon_color: window.state.icon_color(),
                                text_color: window.state.text_color(),
                                scale_factor: window.state.scale_factor(),
                            },
                            cursor,
                        );
                        debug.draw_finished();

                        if new_mouse_interaction != window.mouse_interaction {
                            window.raw.set_cursor(
                                conversion::mouse_interaction(
                                    new_mouse_interaction,
                                ),
                            );

                            window.mouse_interaction = new_mouse_interaction;
                        }

                        runtime.broadcast(subscription::Event::Interaction {
                            window: id,
                            event: redraw_event,
                            status: core::event::Status::Ignored,
                        });

                        if let Err(err) = control_sender.start_send(
                            Control::ChangeFlow(match ui_state {
                                user_interface::State::Updated {
                                    redraw_request: Some(redraw_request),
                                } => match redraw_request {
                                    window::RedrawRequest::NextFrame => {
                                        window.raw.request_redraw();

                                        ControlFlow::Wait
                                    }
                                    window::RedrawRequest::At(at) => {
                                        ControlFlow::WaitUntil(at)
                                    }
                                },
                                _ => ControlFlow::Wait,
                            }),
                        ) {
                            panic!("send error");
                        }

                        let physical_size = window.state.physical_size();
                        if physical_size.width == 0 || physical_size.height == 0
                        {
                            continue;
                        }

                        if window.viewport_version
                            != window.state.viewport_version()
                        {
                            let logical_size = window.state.logical_size();

                            debug.layout_started();
                            let ui = user_interfaces
                                .remove(&id)
                                .expect("Remove user interface");

                            let _ = user_interfaces.insert(
                                id,
                                ui.relayout(logical_size, &mut window.renderer),
                            );
                            debug.layout_finished();

                            debug.draw_started();
                            let new_mouse_interaction = user_interfaces
                                .get_mut(&id)
                                .expect("Get user interface")
                                .draw(
                                    &mut window.renderer,
                                    window.state.theme(),
                                    &renderer::Style {
                                        icon_color: window.state.icon_color(),
                                        text_color: window.state.text_color(),
                                        scale_factor: window
                                            .state
                                            .scale_factor(),
                                    },
                                    window.state.cursor(),
                                );
                            debug.draw_finished();

                            if new_mouse_interaction != window.mouse_interaction
                            {
                                window.raw.set_cursor(
                                    conversion::mouse_interaction(
                                        new_mouse_interaction,
                                    ),
                                );

                                window.mouse_interaction =
                                    new_mouse_interaction;
                            }

                            compositor.configure_surface(
                                &mut window.surface,
                                physical_size.width,
                                physical_size.height,
                            );

                            window.viewport_version =
                                window.state.viewport_version();
                        }

                        debug.render_started();
                        match compositor.present(
                            &mut window.renderer,
                            &mut window.surface,
                            window.state.viewport(),
                            window.state.background_color(),
                            &debug.overlay(),
                        ) {
                            Ok(()) => {
                                debug.render_finished();
                            }
                            Err(error) => match error {
                                // This is an unrecoverable error.
                                compositor::SurfaceError::OutOfMemory => {
                                    panic!("{:?}", error);
                                }
                                _ => {
                                    debug.render_finished();
                                    log::error!(
                                        "Error {error:?} when \
                                        presenting surface."
                                    );

                                    // Try rendering all windows again next frame.
                                    for (_id, window) in
                                        window_manager.iter_mut()
                                    {
                                        window.raw.request_redraw();
                                    }
                                }
                            },
                        }
                    }
                    event::Event::WindowEvent {
                        event: window_event,
                        window_id,
                    } => {
                        if !is_daemon
                            && matches!(
                                window_event,
                                winit::event::WindowEvent::Destroyed
                            )
                            && !is_window_opening
                            && window_manager.is_empty()
                        {
                            control_sender
                                .start_send(Control::Exit)
                                .expect("Send control action");

                            continue;
                        }

                        let Some((id, window)) =
                            window_manager.get_mut_alias(window_id)
                        else {
                            continue;
                        };

                        // Initiates a drag resize window state when found.
                        if let Some(func) =
                            window.drag_resize_window_func.as_mut()
                        {
                            if func(&window.raw, &window_event) {
                                continue;
                            }
                        }

                        if matches!(
                            window_event,
                            winit::event::WindowEvent::CloseRequested
                        ) && window.exit_on_close_request
                        {
                            let w = window_manager.remove(id);
                            let _ = user_interfaces.remove(&id);
                            let _ = ui_caches.remove(&id);
                            // XXX Empty rectangle list un-registers the window
                            if let Some(w) = w {
                                clipboard.register_dnd_destination(
                                    DndSurface(Arc::new(Box::new(
                                        w.raw.clone(),
                                    ))),
                                    Vec::new(),
                                );
                            }
                            events.push((
                                Some(id),
                                core::Event::Window(window::Event::Closed),
                            ));
                        } else {
                            window.state.update(
                                &window.raw,
                                &window_event,
                                &mut debug,
                            );

                            if let Some(event) = conversion::window_event(
                                window_event,
                                window.state.scale_factor(),
                                window.state.modifiers(),
                            ) {
                                events.push((Some(id), event));
                            }
                        }
                    }
                    event::Event::AboutToWait => {
                        if events.is_empty() && messages.is_empty() {
                            continue;
                        }

                        debug.event_processing_started();
                        let mut uis_stale = false;

                        for (id, window) in window_manager.iter_mut() {
                            let mut window_events = vec![];

                            events.retain(|(window_id, event)| {
                                if *window_id == Some(id) {
                                    window_events.push(event.clone());
                                    false
                                } else {
                                    true
                                }
                            });

                            if window_events.is_empty() && messages.is_empty() {
                                continue;
                            }

                            let (ui_state, statuses) = user_interfaces
                                .get_mut(&id)
                                .expect("Get user interface")
                                .update(
                                    &window_events,
                                    window.state.cursor(),
                                    &mut window.renderer,
                                    &mut clipboard,
                                    &mut messages,
                                );

                            window.raw.request_redraw();

                            if !uis_stale {
                                uis_stale = matches!(
                                    ui_state,
                                    user_interface::State::Outdated
                                );
                            }

                            for (event, status) in window_events
                                .into_iter()
                                .zip(statuses.into_iter())
                            {
                                runtime.broadcast(
                                    subscription::Event::Interaction {
                                        window: id,
                                        event,
                                        status,
                                    },
                                );
                            }
                        }

                        for (id, event) in events.drain(..) {
                            runtime.broadcast(
                                subscription::Event::Interaction {
                                    window: id.unwrap_or(window::Id::NONE),
                                    event,
                                    status: core::event::Status::Ignored,
                                },
                            );
                        }

                        debug.event_processing_finished();

                        if !messages.is_empty() || uis_stale {
                            let cached_interfaces: FxHashMap<
                                window::Id,
                                user_interface::Cache,
                            > = ManuallyDrop::into_inner(user_interfaces)
                                .drain()
                                .map(|(id, ui)| (id, ui.into_cache()))
                                .collect();

                            update(
                                &mut program,
                                &mut runtime,
                                &mut debug,
                                &mut messages,
                            );

                            for (id, window) in window_manager.iter_mut() {
                                window.state.synchronize(
                                    &program,
                                    id,
                                    &window.raw,
                                );

                                window.raw.request_redraw();
                            }

                            user_interfaces =
                                ManuallyDrop::new(build_user_interfaces(
                                    &program,
                                    &mut debug,
                                    &mut window_manager,
                                    cached_interfaces,
                                    &mut clipboard,
                                ));

                            if actions > 0 {
                                proxy.free_slots(actions);
                                actions = 0;
                            }
                        }

                        debug.draw_started();

                        for (id, window) in window_manager.iter_mut() {
                            // TODO: Avoid redrawing all the time by forcing widgets to
                            //  request redraws on state changes
                            //
                            // Then, we can use the `interface_state` here to decide if a redraw
                            // is needed right away, or simply wait until a specific time.
                            let redraw_event = core::Event::Window(
                                window::Event::RedrawRequested(Instant::now()),
                            );

                            let cursor = window.state.cursor();

                            let ui = user_interfaces
                                .get_mut(&id)
                                .expect("Get user interface");

                            let (ui_state, _) = ui.update(
                                &[redraw_event.clone()],
                                cursor,
                                &mut window.renderer,
                                &mut clipboard,
                                &mut messages,
                            );

                            let new_mouse_interaction = {
                                let state = &window.state;

                                ui.draw(
                                    &mut window.renderer,
                                    state.theme(),
                                    &renderer::Style {
                                        icon_color: state.icon_color(),
                                        text_color: state.text_color(),
                                        scale_factor: state.scale_factor(),
                                    },
                                    cursor,
                                )
                            };

                            if new_mouse_interaction != window.mouse_interaction
                            {
                                window.raw.set_cursor_icon(
                                    conversion::mouse_interaction(
                                        new_mouse_interaction,
                                    ),
                                );

                                window.mouse_interaction =
                                    new_mouse_interaction;
                            }

                            // TODO once widgets can request to be redrawn, we can avoid always requesting a
                            // redraw
                            window.raw.request_redraw();
                            runtime.broadcast(
                                subscription::Event::Interaction {
                                    window: id,
                                    event: redraw_event,
                                    status: core::event::Status::Ignored,
                                },
                            );

                            let _ = control_sender.start_send(
                                Control::ChangeFlow(match ui_state {
                                    user_interface::State::Updated {
                                        redraw_request: Some(redraw_request),
                                    } => match redraw_request {
                                        window::RedrawRequest::NextFrame => {
                                            window.raw.request_redraw();

                                            ControlFlow::Wait
                                        }
                                        window::RedrawRequest::At(at) => {
                                            ControlFlow::WaitUntil(at)
                                        }
                                    },
                                    _ => ControlFlow::Wait,
                                }),
                            );
                        }

                        debug.draw_finished();
                    }
                    event::Event::PlatformSpecific(
                        event::PlatformSpecific::MacOS(
                            event::MacOS::ReceivedUrl(url),
                        ),
                    ) => {
                        use crate::core::event;

                        // events.push((
                        //     None,
                        //     event::Event::PlatformSpecific(
                        //         event::PlatformSpecific::MacOS(
                        //             event::MacOS::ReceivedUrl(url),
                        //         ),
                        //     ),
                        // ));
                    }
                    event::Event::WindowEvent {
                        event: window_event,
                        window_id,
                    } => {
                        let Some((id, window)) =
                            window_manager.get_mut_alias(window_id)
                        else {
                            continue;
                        };

                        if matches!(
                            window_event,
                            winit::event::WindowEvent::CloseRequested
                        ) {
                            let w = window_manager.remove(id);
                            let _ = user_interfaces.remove(&id);
                            let _ = ui_caches.remove(&id);
                            if let Some(w) = w.as_ref() {
                                clipboard.register_dnd_destination(
                                    DndSurface(Arc::new(Box::new(
                                        w.raw.clone(),
                                    ))),
                                    Vec::new(),
                                );
                            }

                            events.push((
                                Some(id),
                                core::Event::Window(window::Event::Closed),
                            ));

                            if window_manager.is_empty()
                                && w.is_some_and(|w| w.exit_on_close_request)
                            {
                                break;
                            }
                        } else {
                            window.state.update(
                                &window.raw,
                                &window_event,
                                &mut debug,
                            );

                            if let Some(event) = conversion::window_event(
                                window_event,
                                window.state.scale_factor(),
                                window.state.modifiers(),
                            ) {
                                events.push((Some(id), event));
                            }
                        }
                    }
                    _ => {}
                }
            }
            Event::Dnd(e) => {
                match &e {
                    dnd::DndEvent::Offer(_, dnd::OfferEvent::Leave) => {
                        events.push((cur_dnd_surface, core::Event::Dnd(e)));
                        cur_dnd_surface = None;
                    }
                    dnd::DndEvent::Offer(
                        _,
                        dnd::OfferEvent::Enter { surface, .. },
                    ) => {
                        let window_handle = surface.0.window_handle().ok();
                        let window_id = window_manager.iter_mut().find_map(
                            |(id, window)| {
                                if window
                                    .raw
                                    .window_handle()
                                    .ok()
                                    .zip(window_handle)
                                    .map(|(a, b)| a == b)
                                    .unwrap_or_default()
                                {
                                    Some(id)
                                } else {
                                    None
                                }
                            },
                        );

                        cur_dnd_surface = window_id;

                        events.push((cur_dnd_surface, core::Event::Dnd(e)));
                    }
                    dnd::DndEvent::Offer(..) => {
                        events.push((cur_dnd_surface, core::Event::Dnd(e)));
                    }
                    dnd::DndEvent::Source(_) => {
                        events.push((None, core::Event::Dnd(e.clone())))
                    }
                };
            }
            #[cfg(feature = "a11y")]
            Event::Accessibility(e) => {
                match e.action {
                    iced_accessibility::accesskit::Action::Focus => {
                        // TODO send a command for this
                    }
                    _ => {}
                }
                events.push((None, conversion::a11y(e)));
            }
            #[cfg(feature = "a11y")]
            Event::AccessibilityEnabled(enabled) => {
                a11y_enabled = enabled;
            }
        }
    }

    let _ = ManuallyDrop::into_inner(user_interfaces);
}

/// Builds a window's [`UserInterface`] for the [`Program`].
fn build_user_interface<'a, P: Program>(
    program: &'a P,
    cache: user_interface::Cache,
    renderer: &mut P::Renderer,
    size: Size,
    debug: &mut Debug,
    id: window::Id,
) -> UserInterface<'a, P::Message, P::Theme, P::Renderer>
where
    P::Theme: DefaultStyle,
{
    debug.view_started();
    let view = program.view(id);
    debug.view_finished();

    debug.layout_started();
    let user_interface = UserInterface::build(view, size, cache, renderer);
    debug.layout_finished();

    user_interface
}

fn update<P: Program, E: Executor>(
    program: &mut P,
    runtime: &mut Runtime<E, Proxy<P::Message>, Action<P::Message>>,
    debug: &mut Debug,
    messages: &mut Vec<P::Message>,
) where
    P::Theme: DefaultStyle,
{
    for message in messages.drain(..) {
        debug.log_message(&message);
        debug.update_started();

        let task = runtime.enter(|| program.update(message));
        debug.update_finished();

        if let Some(stream) = runtime::task::into_stream(task) {
            runtime.run(stream);
        }
    }

    let subscription = program.subscription();
    runtime.track(subscription::into_recipes(subscription.map(Action::Output)));
}

fn run_action<P, C>(
    action: Action<P::Message>,
    program: &P,
    compositor: &mut C,
    events: &mut Vec<(Option<window::Id>, core::Event)>,
    messages: &mut Vec<P::Message>,
    clipboard: &mut Clipboard,
    control_sender: &mut mpsc::UnboundedSender<Control>,
    debug: &mut Debug,
    interfaces: &mut FxHashMap<
        window::Id,
        UserInterface<'_, P::Message, P::Theme, P::Renderer>,
    >,
    window_manager: &mut WindowManager<P, C>,
    ui_caches: &mut FxHashMap<window::Id, user_interface::Cache>,
    is_window_opening: &mut bool,
) where
    P: Program,
    C: Compositor<Renderer = P::Renderer> + 'static,
    P::Theme: DefaultStyle,
{
    use crate::runtime::clipboard;
    use crate::runtime::system;
    use crate::runtime::window;

    match action {
        Action::Output(message) => {
            messages.push(message);
        }
        Action::Clipboard(action) => match action {
            clipboard::Action::Read { target, channel } => {
                let _ = channel.send(clipboard.read(target));
            }
            clipboard::Action::Write { target, contents } => {
                clipboard.write(target, contents);
            }
            clipboard::Action::WriteData(contents, kind) => {
                clipboard.write_data(kind, ClipboardStoreData(contents))
            }
            clipboard::Action::ReadData(allowed, tx, kind) => {
                let contents = clipboard.read_data(kind, allowed);
                _ = tx.send(contents);
            }
        },
        Action::Window(action) => match action {
            window::Action::Open(id, settings, channel) => {
                let monitor = window_manager.last_monitor();

                control_sender
                    .start_send(Control::CreateWindow {
                        id,
                        settings,
                        title: program.title(id),
                        monitor,
                        on_open: channel,
                    })
                    .expect("Send control action");

                *is_window_opening = true;
            }
            window::Action::Close(id) => {
                let window = window_manager.remove(id);
                let _ = ui_caches.remove(&id);

                if window.is_some() {
                    events.push((
                        Some(id),
                        core::Event::Window(core::window::Event::Closed),
                    ));
                }
            }
            window::Action::GetOldest(channel) => {
                let id =
                    window_manager.iter_mut().next().map(|(id, _window)| id);

                let _ = channel.send(id);
            }
            window::Action::GetLatest(channel) => {
                let id =
                    window_manager.iter_mut().last().map(|(id, _window)| id);

                let _ = channel.send(id);
            }
            window::Action::Drag(id) => {
                if let Some(window) = window_manager.get_mut(id) {
                    let _ = window.raw.drag_window();
                }
            }
            window::Action::Resize(id, size) => {
                if let Some(window) = window_manager.get_mut(id) {
                    let _ = window.raw.request_inner_size(
                        winit::dpi::LogicalSize {
                            width: size.width,
                            height: size.height,
                        },
                    );
                }
            }
            window::Action::GetSize(id, channel) => {
                if let Some(window) = window_manager.get_mut(id) {
                    let size = window
                        .raw
                        .inner_size()
                        .to_logical(window.raw.scale_factor());

                    let _ = channel.send(Size::new(size.width, size.height));
                }
            }
            window::Action::GetMaximized(id, channel) => {
                if let Some(window) = window_manager.get_mut(id) {
                    let _ = channel.send(window.raw.is_maximized());
                }
            }
            window::Action::Maximize(id, maximized) => {
                if let Some(window) = window_manager.get_mut(id) {
                    window.raw.set_maximized(maximized);
                }
            }
            window::Action::GetMinimized(id, channel) => {
                if let Some(window) = window_manager.get_mut(id) {
                    let _ = channel.send(window.raw.is_minimized());
                }
            }
            window::Action::Minimize(id, minimized) => {
                if let Some(window) = window_manager.get_mut(id) {
                    window.raw.set_minimized(minimized);
                }
            }
            window::Action::GetPosition(id, channel) => {
                if let Some(window) = window_manager.get_mut(id) {
                    let position = window
                        .raw
                        .inner_position()
                        .map(|position| {
                            let position = position
                                .to_logical::<f32>(window.raw.scale_factor());

                            Point::new(position.x, position.y)
                        })
                        .ok();

                    let _ = channel.send(position);
                }
            }
            window::Action::Move(id, position) => {
                if let Some(window) = window_manager.get_mut(id) {
                    window.raw.set_outer_position(
                        winit::dpi::LogicalPosition {
                            x: position.x,
                            y: position.y,
                        },
                    );
                }
            }
            window::Action::ChangeMode(id, mode) => {
                if let Some(window) = window_manager.get_mut(id) {
                    window.raw.set_visible(conversion::visible(mode));
                    window.raw.set_fullscreen(conversion::fullscreen(
                        window.raw.current_monitor(),
                        mode,
                    ));
                }
            }
            window::Action::ChangeIcon(id, icon) => {
                if let Some(window) = window_manager.get_mut(id) {
                    window.raw.set_window_icon(conversion::icon(icon));
                }
            }
            window::Action::GetMode(id, channel) => {
                if let Some(window) = window_manager.get_mut(id) {
                    let mode = if window.raw.is_visible().unwrap_or(true) {
                        conversion::mode(window.raw.fullscreen())
                    } else {
                        core::window::Mode::Hidden
                    };

                    let _ = channel.send(mode);
                }
            }
            window::Action::ToggleMaximize(id) => {
                if let Some(window) = window_manager.get_mut(id) {
                    window.raw.set_maximized(!window.raw.is_maximized());
                }
            }
            window::Action::ToggleDecorations(id) => {
                if let Some(window) = window_manager.get_mut(id) {
                    window.raw.set_decorations(!window.raw.is_decorated());
                }
            }
            window::Action::RequestUserAttention(id, attention_type) => {
                if let Some(window) = window_manager.get_mut(id) {
                    window.raw.request_user_attention(
                        attention_type.map(conversion::user_attention),
                    );
                }
            }
            window::Action::GainFocus(id) => {
                if let Some(window) = window_manager.get_mut(id) {
                    window.raw.focus_window();
                }
            }
            window::Action::ChangeLevel(id, level) => {
                if let Some(window) = window_manager.get_mut(id) {
                    window
                        .raw
                        .set_window_level(conversion::window_level(level));
                }
            }
            window::Action::ShowSystemMenu(id) => {
                if let Some(window) = window_manager.get_mut(id) {
                    if let mouse::Cursor::Available(point) =
                        window.state.cursor()
                    {
                        window.raw.show_window_menu(
                            winit::dpi::LogicalPosition {
                                x: point.x,
                                y: point.y,
                            },
                        );
                    }
                }
            }
            window::Action::GetRawId(id, channel) => {
                if let Some(window) = window_manager.get_mut(id) {
                    let _ = channel.send(window.raw.id().into());
                }
            }
            window::Action::RunWithHandle(id, f) => {
                use window::raw_window_handle::HasWindowHandle;

                if let Some(handle) = window_manager
                    .get_mut(id)
                    .and_then(|window| window.raw.window_handle().ok())
                {
                    f(handle);
                }
            }
            window::Action::Screenshot(id, channel) => {
                if let Some(window) = window_manager.get_mut(id) {
                    let bytes = compositor.screenshot(
                        &mut window.renderer,
                        &mut window.surface,
                        window.state.viewport(),
                        window.state.background_color(),
                        &debug.overlay(),
                    );

                    let _ = channel.send(window::Screenshot::new(
                        bytes,
                        window.state.physical_size(),
                        window.state.viewport().scale_factor(),
                    ));
                }
            }
        },
        Action::System(action) => match action {
            system::Action::QueryInformation(_channel) => {
                #[cfg(feature = "system")]
                {
                    let graphics_info = compositor.fetch_information();

                    let _ = std::thread::spawn(move || {
                        let information =
                            crate::system::information(graphics_info);

                        let _ = _channel.send(information);
                    });
                }
            }
        },
        Action::Widget(operation) => {
            let mut current_operation = Some(operation);

            while let Some(mut operation) = current_operation.take() {
                for (id, ui) in interfaces.iter_mut() {
                    if let Some(window) = window_manager.get_mut(*id) {
                        ui.operate(&window.renderer, operation.as_mut());
                    }
                }

                match operation.finish() {
                    operation::Outcome::None => {}
                    operation::Outcome::Some(()) => {}
                    operation::Outcome::Chain(next) => {
                        current_operation = Some(next);
                    }
                }
            }
        }
        Action::LoadFont { bytes, channel } => {
            // TODO: Error handling (?)
            compositor.load_font(bytes.clone());

            let _ = channel.send(Ok(()));
        }
        Action::Exit => {
            control_sender
                .start_send(Control::Exit)
                .expect("Send control action");
        }
        Action::Dnd(a) => match a {
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
            } => {
                let Some(window_id) = source_surface.and_then(|source| {
                    match source {
                        core::clipboard::DndSource::Surface(s) => Some(s),
                        core::clipboard::DndSource::Widget(w) => {
                            // search windows for widget with operation
                            interfaces.iter_mut().find_map(|(ui_id, ui)| {
                                let Some(ui_renderer) = window_manager
                                    .get_mut(ui_id.clone())
                                    .map(|w| &w.renderer)
                                else {
                                    return None;
                                };

                                let operation: Box<dyn Operation<()>> =
                                    Box::new(operation::map(
                                        Box::new(search_id::search_id(
                                            w.clone(),
                                        )),
                                        |_| {},
                                    ));
                                let mut current_operation = Some(operation);

                                while let Some(mut operation) =
                                    current_operation.take()
                                {
                                    ui.operate(ui_renderer, operation.as_mut());

                                    match operation.finish() {
                                        operation::Outcome::None => {}
                                        operation::Outcome::Some(()) => {
                                            return Some(ui_id.clone());
                                        }
                                        operation::Outcome::Chain(next) => {
                                            current_operation = Some(next);
                                        }
                                    }
                                }
                                None
                            })
                        }
                    }
                }) else {
                    eprintln!("No source surface");
                    return;
                };

                let Some(window) = window_manager.get_mut(window_id) else {
                    eprintln!("No window");
                    return;
                };

                let state = &window.state;
                let icon_surface = icon_surface
                    .map(|i| {
                        let i: Box<dyn Any> = i;
                        i
                    })
                    .and_then(|i| {
                        None
                        // i.downcast::<Arc<(
                        //     core::Element<
                        //         'static,
                        //         P::Message,
                        //         P::Theme,
                        //         P::Renderer,
                        //     >,
                        //     core::widget::tree::State,
                        // )>>()
                        // .ok()
                    })
                    .map(
                        |e: Box<
                            Arc<(
                                core::Element<
                                    'static,
                                    P::Message,
                                    P::Theme,
                                    P::Renderer,
                                >,
                                core::widget::tree::State,
                            )>,
                        >| {
                            let mut renderer = compositor.create_renderer();

                            let e = Arc::into_inner(*e).unwrap();
                            let (mut e, widget_state) = e;
                            let lim = core::layout::Limits::new(
                                Size::new(1., 1.),
                                Size::new(
                                    state.viewport().physical_width() as f32,
                                    state.viewport().physical_height() as f32,
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
                                window.raw.clone(),
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
                        },
                    );

                clipboard.start_dnd_winit(
                    internal,
                    DndSurface(Arc::new(Box::new(window.raw.clone()))),
                    icon_surface,
                    content,
                    actions,
                );
            }
            iced_runtime::dnd::DndAction::EndDnd => {
                clipboard.end_dnd();
            }
            iced_runtime::dnd::DndAction::PeekDnd(m, channel) => {
                let data = clipboard.peek_dnd(m);
                channel.send(data);
            }
            iced_runtime::dnd::DndAction::SetAction(a) => {
                clipboard.set_action(a);
            }
        },
        Action::PlatformSpecific(_) => todo!(),
    }
}

/// Build the user interface for every window.
pub fn build_user_interfaces<'a, P: Program, C>(
    program: &'a P,
    debug: &mut Debug,
    window_manager: &mut WindowManager<P, C>,
    mut cached_user_interfaces: FxHashMap<window::Id, user_interface::Cache>,
    clipboard: &mut Clipboard,
) -> FxHashMap<window::Id, UserInterface<'a, P::Message, P::Theme, P::Renderer>>
where
    C: Compositor<Renderer = P::Renderer>,
    P::Theme: DefaultStyle,
{
    cached_user_interfaces
        .drain()
        .filter_map(|(id, cache)| {
            let window = window_manager.get_mut(id)?;
            let interface = build_user_interface(
                program,
                cache,
                &mut window.renderer,
                window.state.logical_size(),
                debug,
                id,
            );

            let dnd_rectangles = interface.dnd_rectangles(
                window.prev_dnd_destination_rectangles_count,
                &window.renderer,
            );
            let new_dnd_rectangles_count = dnd_rectangles.as_ref().len();
            if new_dnd_rectangles_count > 0
                || window.prev_dnd_destination_rectangles_count > 0
            {
                clipboard.register_dnd_destination(
                    DndSurface(Arc::new(Box::new(window.raw.clone()))),
                    dnd_rectangles.into_rectangles(),
                );
            }

            window.prev_dnd_destination_rectangles_count =
                new_dnd_rectangles_count;

            Some((id, interface))
        })
        .collect()
}

/// Returns true if the provided event should cause a [`Program`] to
/// exit.
pub fn user_force_quit(
    event: &winit::event::WindowEvent,
    _modifiers: winit::keyboard::ModifiersState,
) -> bool {
    match event {
        #[cfg(target_os = "macos")]
        winit::event::WindowEvent::KeyboardInput {
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
