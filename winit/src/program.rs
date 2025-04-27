//! Create interactive, native cross-platform applications for WGPU.
#[path = "application/drag_resize.rs"]
mod drag_resize;
mod state;
pub(crate) mod window_manager;

use iced_futures::core::window::Id;
pub use runtime::{default, Appearance, DefaultStyle};
use window_manager::ViewFn;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::OwnedDisplayHandle;
#[cfg(feature = "wayland")]
use winit::platform::wayland::WindowExtWayland;

use crate::conversion;
use crate::core;
use crate::core::mouse;
use crate::core::renderer;
use crate::core::time::Instant;
use crate::core::widget::operation;
use crate::core::widget::Operation;
use crate::core::window;
use crate::core::Clipboard as CoreClipboard;
use crate::core::Length;
use crate::core::{Element, Point, Size};
use crate::futures::futures::channel::mpsc;
use crate::futures::futures::channel::oneshot;
use crate::futures::futures::task;
use crate::futures::futures::{Future, StreamExt};
use crate::futures::subscription::{self, Subscription};
use crate::futures::{Executor, Runtime};
use crate::graphics;
use crate::graphics::{compositor, Compositor};
use crate::platform_specific;
use crate::runtime::user_interface::{self, UserInterface};
use crate::runtime::Debug;
use crate::runtime::{self, Action, Task};
use crate::{Clipboard, Error, Proxy, Settings};
use dnd::DndSurface;
use dnd::Icon;
use iced_futures::core::widget::operation::search_id;
use iced_graphics::Viewport;
pub use state::State;
use window_clipboard::mime::ClipboardStoreData;
use winit::raw_window_handle::HasWindowHandle;

pub(crate) use window_manager::WindowManager;

use rustc_hash::FxHashMap;
use std::any::Any;
use std::borrow::Cow;
use std::collections::HashMap;
use std::mem::ManuallyDrop;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use log::trace;

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

    fn with_program<T>(&self, f: impl Fn(&Self) -> T) -> T {
        f(self)
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

    let event_loop = EventLoop::new().expect("Create event loop");
    #[cfg(feature = "wayland")]
    let is_wayland =
        winit::platform::wayland::EventLoopExtWayland::is_wayland(&event_loop);
    #[cfg(not(feature = "wayland"))]
    let is_wayland = false;

    let (event_sender, event_receiver) = mpsc::unbounded();
    let (proxy, worker): (Proxy<<P as Program>::Message>, _) =
        Proxy::new(event_loop.create_proxy(), event_sender.clone());

    let mut runtime = {
        let executor =
            P::Executor::new().map_err(Error::ExecutorCreationFailed)?;
        executor.spawn(worker);

        Runtime::new(executor, proxy.clone())
    };

    let (program, task) = runtime.enter(|| P::new(flags));
    let is_daemon = window_settings.is_none() || settings.is_daemon;

    let task = if let Some(window_settings) = window_settings {
        let mut task = Some(task);

        let open = iced_runtime::task::oneshot(|channel| {
            iced_runtime::Action::Window(iced_runtime::window::Action::Open(
                iced_runtime::core::window::Id::RESERVED,
                window_settings,
                channel,
            ))
        });

        open.then(move |_| task.take().unwrap_or(Task::none()))
    } else {
        task
    };

    if let Some(stream) = runtime::task::into_stream(task) {
        runtime.run(stream);
    }

    runtime.track(subscription::into_recipes(
        runtime.enter(|| program.subscription().map(Action::Output)),
    ));

    let (boot_sender, boot_receiver) = oneshot::channel();
    let (control_sender, control_receiver) = mpsc::unbounded();

    let instance = Box::pin(run_instance::<P, C>(
        program,
        runtime,
        proxy.clone(),
        debug,
        boot_receiver,
        event_receiver,
        control_sender.clone(),
        event_loop.owned_display_handle(),
        is_daemon,
    ));

    let context = task::Context::from_waker(task::noop_waker_ref());

    struct Runner<Message: 'static, F, C> {
        instance: std::pin::Pin<Box<F>>,
        context: task::Context<'static>,
        id: Option<String>,
        boot: Option<BootConfig<C>>,
        sender: mpsc::UnboundedSender<Event<Message>>,
        receiver: mpsc::UnboundedReceiver<Control>,
        error: Option<Error>,
        proxy: Proxy<Message>,

        #[cfg(target_arch = "wasm32")]
        is_booted: std::rc::Rc<std::cell::RefCell<bool>>,
        #[cfg(target_arch = "wasm32")]
        canvas: Option<web_sys::HtmlCanvasElement>,
    }

    struct BootConfig<C> {
        sender: oneshot::Sender<Boot<C>>,
        fonts: Vec<Cow<'static, [u8]>>,
        graphics_settings: graphics::Settings,
        control_sender: mpsc::UnboundedSender<Control>,
        is_wayland: bool,
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
            is_wayland,
        }),
        sender: event_sender,
        receiver: control_receiver,
        error: None,
        proxy: proxy.clone(),

        #[cfg(target_arch = "wasm32")]
        is_booted: std::rc::Rc::new(std::cell::RefCell::new(false)),
        #[cfg(target_arch = "wasm32")]
        canvas: None,
    };

    impl<Message, F, C> winit::application::ApplicationHandler
        for Runner<Message, F, C>
    where
        Message: std::fmt::Debug,
        F: Future<Output = ()>,
        C: Compositor + 'static,
    {
        fn proxy_wake_up(
            &mut self,
            event_loop: &dyn winit::event_loop::ActiveEventLoop,
        ) {
            self.process_event(event_loop, None);
        }

        fn new_events(
            &mut self,
            event_loop: &dyn winit::event_loop::ActiveEventLoop,
            cause: winit::event::StartCause,
        ) {
            if self.boot.is_some() {
                return;
            }
            self.process_event(event_loop, Some(Event::NewEvents(cause)));
        }

        fn window_event(
            &mut self,
            event_loop: &dyn winit::event_loop::ActiveEventLoop,
            window_id: winit::window::WindowId,
            event: winit::event::WindowEvent,
        ) {
            #[cfg(target_os = "windows")]
            let is_move_or_resize = matches!(
                event,
                winit::event::WindowEvent::SurfaceResized(_)
                    | winit::event::WindowEvent::Moved(_)
            );

            #[cfg(feature = "wayland")]
            {
                if matches!(event, WindowEvent::RedrawRequested) {
                    for id in
                        crate::subsurface_widget::subsurface_ids(window_id)
                    {
                        _ = self.sender.start_send(Event::Winit(
                            id,
                            WindowEvent::RedrawRequested,
                        ));
                    }
                } else if matches!(event, WindowEvent::RedrawRequested) {
                    for id in
                        crate::subsurface_widget::subsurface_ids(window_id)
                    {
                        _ = self.sender.start_send(Event::Winit(
                            id,
                            WindowEvent::CloseRequested,
                        ));
                    }
                }
            }

            self.process_event(
                event_loop,
                Some(Event::Winit(window_id, event)),
            );

            // TODO: Remove when unnecessary
            // On Windows, we emulate an `AboutToWait` event after every `Resized` event
            // since the event loop does not resume during resize interaction.
            // More details: https://github.com/rust-windowing/winit/issues/3272
            #[cfg(target_os = "windows")]
            {
                if is_move_or_resize {
                    self.process_event(event_loop, Some(Event::AboutToWait));
                }
            }
        }

        fn about_to_wait(
            &mut self,
            event_loop: &dyn winit::event_loop::ActiveEventLoop,
        ) {
            self.process_event(event_loop, Some(Event::AboutToWait));
        }

        fn can_create_surfaces(
            &mut self,
            event_loop: &dyn winit::event_loop::ActiveEventLoop,
        ) {
            // create initial window
            let Some(BootConfig {
                sender,
                fonts,
                graphics_settings,
                control_sender,
                is_wayland,
            }) = self.boot.take()
            else {
                return;
            };

            let window: Arc<dyn winit::window::Window> = match event_loop
                .create_window(
                    winit::window::WindowAttributes::default()
                        .with_visible(false),
                ) {
                Ok(window) => Arc::from(window),
                Err(error) => {
                    self.error = Some(Error::WindowCreationFailed(error));
                    event_loop.exit();
                    return;
                }
            };

            #[cfg(target_arch = "wasm32")]
            {
                use winit::platform::web::WindowExtWebSys;
                self.canvas = window.canvas();
            }

            let proxy = self.proxy.raw.clone();
            let finish_boot = async move {
                let mut compositor =
                    C::new(graphics_settings, window.clone()).await?;

                for font in fonts {
                    compositor.load_font(font);
                }

                sender
                    .send(Boot {
                        compositor,
                        is_wayland,
                        clipboard: Clipboard::connect(
                            window,
                            crate::clipboard::ControlSender {
                                sender: control_sender,
                                proxy,
                            },
                        ),
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

                event_loop
                    .set_control_flow(winit::event_loop::ControlFlow::Poll);
            }
        }
    }

    impl<Message, F, C> Runner<Message, F, C>
    where
        F: Future<Output = ()>,
        C: Compositor,
    {
        fn process_event(
            &mut self,
            event_loop: &dyn winit::event_loop::ActiveEventLoop,
            event: Option<Event<Message>>,
        ) {
            if event_loop.exiting() {
                return;
            }

            if let Some(event) = event {
                self.sender.start_send(event).expect("Send event");
            }

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

                                let window_attributes =
                                    conversion::window_attributes(
                                        settings,
                                        &title,
                                        monitor
                                            .or(event_loop.primary_monitor()),
                                        self.id.clone(),
                                    )
                                    .with_visible(false);

                                #[cfg(target_arch = "wasm32")]
                                let window_attributes = {
                                    use winit::platform::web::WindowAttributesExtWebSys;
                                    window_attributes
                                        .with_canvas(self.canvas.take())
                                };

                                log::info!("Window attributes for id `{id:#?}`: {window_attributes:#?}");

                                let window = Arc::from(
                                    event_loop
                                        .create_window(window_attributes)
                                        .expect("Create window"),
                                );

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
                                    Some(Event::WindowCreated {
                                        id,
                                        window,
                                        exit_on_close_request,
                                        make_visible: visible,
                                        on_open,
                                        resize_border,
                                    }),
                                );
                            }
                            Control::Exit => {
                                event_loop.exit();
                            }
                            Control::Dnd(e) => {
                                self.sender.start_send(Event::Dnd(e)).unwrap();
                            }
                            #[cfg(feature = "a11y")]
                            Control::Accessibility(id, event) => {
                                self.process_event(
                                    event_loop,
                                    Some(Event::Accessibility(id, event)),
                                );
                            }
                            #[cfg(feature = "a11y")]
                            Control::AccessibilityEnabled(event) => {
                                self.process_event(
                                    event_loop,
                                    Some(Event::AccessibilityEnabled(event)),
                                );
                            }
                            Control::PlatformSpecific(e) => {
                                self.sender
                                    .start_send(Event::PlatformSpecific(e))
                                    .unwrap();
                            }
                            Control::AboutToWait => {
                                self.sender
                                    .start_send(Event::AboutToWait)
                                    .expect("Send event");
                            }
                            Control::Winit(id, e) => {
                                #[cfg(feature = "wayland")]
                                {
                                    if matches!(e, WindowEvent::RedrawRequested)
                                    {
                                        for id in crate::subsurface_widget::subsurface_ids(id) {
                                            _ = self.sender
                                                .start_send(Event::Winit(
                                                id,
                                                WindowEvent::RedrawRequested,
                                            ));
                                        }
                                    } else if matches!(
                                        e,
                                        WindowEvent::RedrawRequested
                                    ) {
                                        for id in crate::subsurface_widget::subsurface_ids(id) {
                                            _ = self.sender
                                                .start_send(Event::Winit(
                                                id,
                                                WindowEvent::CloseRequested,
                                            ));
                                        }
                                    }
                                }
                                self.sender
                                    .start_send(Event::Winit(id, e))
                                    .expect("Send event");
                            }
                            Control::StartDnd => {
                                self.sender
                                    .start_send(Event::StartDnd)
                                    .expect("Send event");
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
    is_wayland: bool,
    clipboard: Clipboard,
}

pub(crate) enum Event<Message: 'static> {
    WindowCreated {
        id: window::Id,
        window: Arc<dyn winit::window::Window>,
        exit_on_close_request: bool,
        make_visible: bool,
        on_open: oneshot::Sender<window::Id>,
        resize_border: u32,
    },
    Dnd(dnd::DndEvent<dnd::DndSurface>),
    #[cfg(feature = "a11y")]
    Accessibility(window::Id, iced_accessibility::accesskit::ActionRequest),
    #[cfg(feature = "a11y")]
    AccessibilityEnabled(bool),
    Winit(winit::window::WindowId, winit::event::WindowEvent),
    AboutToWait,
    UserEvent(Action<Message>),
    NewEvents(winit::event::StartCause),
    PlatformSpecific(crate::platform_specific::Event),
    StartDnd,
}

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
    Accessibility(window::Id, iced_accessibility::accesskit::ActionRequest),
    #[cfg(feature = "a11y")]
    AccessibilityEnabled(bool),
    PlatformSpecific(crate::platform_specific::Event),
    AboutToWait,
    Winit(winit::window::WindowId, winit::event::WindowEvent),
    StartDnd,
}

async fn run_instance<'a, P, C>(
    mut program: P,
    mut runtime: Runtime<P::Executor, Proxy<P::Message>, Action<P::Message>>,
    mut proxy: Proxy<P::Message>,
    mut debug: Debug,
    boot: oneshot::Receiver<Boot<C>>,
    mut event_receiver: mpsc::UnboundedReceiver<Event<P::Message>>,
    mut control_sender: mpsc::UnboundedSender<Control>,
    display_handle: OwnedDisplayHandle,
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
        is_wayland,
        mut clipboard,
    } = boot.await.expect("Receive boot");

    let mut platform_specific_handler =
        crate::platform_specific::PlatformSpecific::default();
    #[cfg(all(feature = "wayland", target_os = "linux"))]
    if is_wayland {
        platform_specific_handler = platform_specific_handler.with_wayland(
            control_sender.clone(),
            proxy.raw.clone(),
            display_handle,
        );
    }

    let mut window_manager = WindowManager::new();
    let mut is_window_opening = !is_daemon;

    let mut events = Vec::new();
    let mut messages = Vec::new();
    let mut actions = 0;

    #[cfg(feature = "a11y")]
    let (mut adapters, mut a11y_enabled) = if let Some((main_id, title, raw)) =
        window_manager.ids().next().and_then(|id| {
            window_manager
                .get(id)
                .map(|w| (id, w.state.title.clone(), w.raw.clone()))
        }) {
        let node_id = core::id::window_node_id();
        use crate::a11y::*;
        use iced_accessibility::accesskit::{
            ActivationHandler, NodeBuilder, NodeId, Role, Tree, TreeUpdate,
        };
        use iced_accessibility::accesskit_winit::Adapter;

        let activation_handler = WinitActivationHandler {
            proxy: control_sender.clone(),
            title: title.clone(),
        };

        let action_handler = WinitActionHandler {
            id: main_id,
            proxy: control_sender.clone(),
        };

        let deactivation_handler = WinitDeactivationHandler {
            proxy: control_sender.clone(),
        };
        (
            HashMap::from([(
                main_id,
                (
                    node_id,
                    Adapter::with_direct_handlers(
                        raw.as_ref(),
                        activation_handler,
                        action_handler,
                        deactivation_handler,
                    ),
                ),
            )]),
            false,
        )
    } else {
        (Default::default(), false)
    };

    let mut ui_caches = FxHashMap::default();
    let mut user_interfaces: ManuallyDrop<
        HashMap<
            window::Id,
            UserInterface<
                '_,
                <P as Program>::Message,
                <P as Program>::Theme,
                <P as Program>::Renderer,
            >,
            rustc_hash::FxBuildHasher,
        >,
    > = ManuallyDrop::new(FxHashMap::default());

    let mut cur_dnd_surface: Option<window::Id> = None;

    let mut dnd_surface: Option<
        Arc<Box<dyn HasWindowHandle + Send + Sync + 'static>>,
    > = None;
    let mut dnd_surface_id: Option<window::Id> = None;

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
        let mut rebuild_a11y_tree = false;

        match event {
            Event::StartDnd => {
                let queued = clipboard.get_queued();
                for crate::clipboard::StartDnd {
                    internal,
                    source_surface,
                    icon_surface,
                    content,
                    actions,
                } in queued
                {
                    let Some(window_id) = source_surface.and_then(|source| {
                        match source {
                            core::clipboard::DndSource::Surface(s) => {
                                log::trace!(
                                    "start_dnd: using explicit surface {:?}",
                                    s
                                );
                                Some(s)
                            }
                            core::clipboard::DndSource::Widget(w) => {
                                log::debug!(
                                    "start_dnd: searching for widget {:?}",
                                    w
                                );
                                // search windows for widget with operation
                                let result = user_interfaces.iter_mut().find_map(
                                    |(ui_id, ui)| {
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
                                        let mut current_operation =
                                            Some(operation);

                                        while let Some(mut operation) =
                                            current_operation.take()
                                        {
                                            ui.operate(
                                                ui_renderer,
                                                operation.as_mut(),
                                            );

                                            match operation.finish() {
                                                operation::Outcome::None => {}
                                                operation::Outcome::Some(
                                                    (),
                                                ) => {
                                                    log::debug!(
                                                        "start_dnd: widget {:?} found in {:?}",
                                                        w,
                                                        ui_id
                                                    );
                                                    return Some(ui_id.clone());
                                                }
                                                operation::Outcome::Chain(
                                                    next,
                                                ) => {
                                                    current_operation =
                                                        Some(next);
                                                }
                                            }
                                        }
                                        None
                                    },
                                );

                                if result.is_none() {
                                    log::warn!(
                                        "start_dnd: widget {:?} not found; drag will fail",
                                        w
                                    );
                                }

                                result
                            }
                        }
                    }) else {
                        log::warn!("start_dnd: No source surface resolved");
                        continue;
                    };

                    let Some(window) = window_manager.get_mut(window_id) else {
                        eprintln!("No window");
                        continue;
                    };

                    let state = &window.state;
                    let mut dnd_buffer = None;
                    let icon_surface = icon_surface.map(|i| {
                        let mut icon_surface =
                            i.downcast::<P::Theme, P::Renderer>();

                        let mut renderer = compositor.create_renderer();

                        let lim = core::layout::Limits::new(
                            Size::new(1., 1.),
                            Size::new(
                                state.viewport().physical_width() as f32,
                                state.viewport().physical_height() as f32,
                            ),
                        );

                        let mut tree = core::widget::Tree {
                            id: icon_surface.element.as_widget().id(),
                            tag: icon_surface.element.as_widget().tag(),
                            state: icon_surface.state,
                            children: icon_surface
                                .element
                                .as_widget()
                                .children(),
                        };

                        let size = icon_surface
                            .element
                            .as_widget()
                            .layout(&mut tree, &renderer, &lim);
                        icon_surface.element.as_widget_mut().diff(&mut tree);

                        let size = lim.resolve(
                            Length::Shrink,
                            Length::Shrink,
                            size.size(),
                        );
                        let viewport = Viewport::with_logical_size(
                            size,
                            state.viewport().scale_factor(),
                        );

                        let mut ui = UserInterface::build(
                            icon_surface.element,
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
                            &viewport,
                            core::Color::TRANSPARENT,
                            &debug.overlay(),
                        );
                        for pix in bytes.chunks_exact_mut(4) {
                            // rgba -> argb little endian
                            pix.swap(0, 2);
                        }
                        // update subsurfaces
                        if let Some(surface) =
                            platform_specific_handler.create_surface()
                        {
                            // TODO Remove id
                            let id = window::Id::unique();
                            platform_specific_handler
                                .update_subsurfaces(id, &surface);
                            let surface = Arc::new(surface);
                            dnd_surface = Some(surface.clone());
                            dnd_surface_id = Some(id);
                            dnd_buffer = Some((
                                viewport.physical_size(),
                                state.scale_factor(),
                                bytes,
                                icon_surface.offset,
                            ));
                            Icon::Surface(dnd::DndSurface(surface))
                        } else {
                            platform_specific_handler.clear_subsurface_list();
                            Icon::Buffer {
                                data: Arc::new(bytes),
                                width: viewport.physical_width(),
                                height: viewport.physical_height(),
                                transparent: true,
                            }
                        }
                    });

                    clipboard.start_dnd_winit(
                        internal,
                        DndSurface(Arc::new(Box::new(window.raw.clone()))),
                        icon_surface,
                        content,
                        actions,
                    );

                    // This needs to be after `wl_data_device::start_drag` for the offset to have an effect
                    if let (Some(surface), Some((size, scale, bytes, offset))) =
                        (dnd_surface.as_ref(), dnd_buffer)
                    {
                        platform_specific_handler.update_surface_shm(
                            &surface,
                            size.width,
                            size.height,
                            scale,
                            &bytes,
                            offset,
                        );
                    }
                }
            }
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
                    window,
                    &program,
                    &mut compositor,
                    exit_on_close_request,
                    resize_border,
                );
                #[cfg(feature = "wayland")]
                platform_specific_handler.send_wayland(
                    platform_specific::Action::TrackWindow(
                        window.raw.clone(),
                        id,
                    ),
                );
                #[cfg(feature = "a11y")]
                {
                    use crate::a11y::*;
                    use iced_accessibility::accesskit::{
                        ActivationHandler, NodeBuilder, NodeId, Role, Tree,
                        TreeUpdate,
                    };
                    use iced_accessibility::accesskit_winit::Adapter;

                    let node_id = core::id::window_node_id();

                    let activation_handler = WinitActivationHandler {
                        proxy: control_sender.clone(),
                        title: window.state.title.clone(),
                    };

                    let action_handler = WinitActionHandler {
                        id,
                        proxy: control_sender.clone(),
                    };

                    let deactivation_handler = WinitDeactivationHandler {
                        proxy: control_sender.clone(),
                    };
                    _ = adapters.insert(
                        id,
                        (
                            node_id,
                            Adapter::with_direct_handlers(
                                window.raw.as_ref(),
                                activation_handler,
                                action_handler,
                                deactivation_handler,
                            ),
                        ),
                    );
                }

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
                        window.raw.clone(),
                        window.prev_dnd_destination_rectangles_count,
                        &mut clipboard,
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

                if clipboard.window_id().is_none() {
                    clipboard = Clipboard::connect(
                        window.raw.clone(),
                        crate::clipboard::ControlSender {
                            sender: control_sender.clone(),
                            proxy: proxy.raw.clone(),
                        },
                    );
                }

                let _ = on_open.send(id);
                is_window_opening = false;
            }
            Event::UserEvent(action) => {
                rebuild_a11y_tree = true;
                let exited = run_action(
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
                    &mut platform_specific_handler,
                );
                if exited {
                    runtime.track(None.into_iter());
                }
                actions += 1;
            }
            Event::NewEvents(
                event::StartCause::Init
                | event::StartCause::ResumeTimeReached { .. },
            ) => {
                if window_manager.ids().next().is_none() {
                    _ = control_sender
                        .start_send(Control::ChangeFlow(ControlFlow::Wait));
                }
                for (_id, window) in window_manager.iter_mut() {
                    window.request_redraw();
                }
            }
            Event::Winit(window_id, event) => {
                #[cfg(feature = "a11y")]
                {
                    if let Some((id, window)) =
                        window_manager.get_mut_alias(window_id)
                    {
                        if let Some(Some((_, adapter))) =
                            a11y_enabled.then(|| adapters.get_mut(&id))
                        {
                            adapter.process_event(window.raw.as_ref(), &event);
                        };
                    }
                }
                match event {
                    event::WindowEvent::RedrawRequested => {
                        let Some((id, window)) =
                            window_manager.get_mut_alias(window_id)
                        else {
                            continue;
                        };

                        // XX must force update to corner radius before the surface is committed.
                        #[cfg(feature = "wayland")]
                        if window.viewport_version
                            != window.state.viewport_version()
                            || window.size() != window.state.logical_size()
                        {
                            platform_specific_handler.send_wayland(
                                platform_specific::Action::ResizeWindow(id),
                            );
                            window.redraw_requested = true;
                            control_sender
                                .start_send(Control::Winit(
                                    window.raw.id(),
                                    WindowEvent::RedrawRequested,
                                ))
                                .expect("Send redraw event");
                        }

                        window.redraw_requested = false;

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
                        platform_specific_handler.update_subsurfaces(
                            id,
                            window.raw.rwh_06_window_handle(),
                        );
                        debug.draw_finished();

                        if new_mouse_interaction != window.mouse_interaction {
                            if let Some(interaction) =
                                conversion::mouse_interaction(
                                    new_mouse_interaction,
                                )
                            {
                                if matches!(
                                    window.mouse_interaction,
                                    mouse::Interaction::Hide
                                ) {
                                    window.raw.set_cursor_visible(true);
                                }
                                window.raw.set_cursor(interaction.into())
                            } else {
                                window.raw.set_cursor_visible(false);
                            }

                            window.mouse_interaction = new_mouse_interaction;
                        }

                        runtime.broadcast(subscription::Event::Interaction {
                            window: id,
                            event: redraw_event,
                            status: core::event::Status::Ignored,
                        });

                        if control_sender
                            .start_send(Control::ChangeFlow(match ui_state {
                                user_interface::State::Updated {
                                    redraw_request: Some(redraw_request),
                                    input_method,
                                } => {
                                    window.request_input_method(input_method);
                                    match redraw_request {
                                        window::RedrawRequest::NextFrame => {
                                            window.request_redraw();

                                            ControlFlow::Wait
                                        }
                                        window::RedrawRequest::At(at) => {
                                            ControlFlow::WaitUntil(at)
                                        }
                                    }
                                }
                                _ => ControlFlow::Wait,
                            }))
                            .is_err()
                        {
                            panic!("send error");
                        }

                        window.draw_preedit();

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
                            let mut ui = user_interfaces
                                .remove(&id)
                                .expect("Remove user interface")
                                .relayout(logical_size, &mut window.renderer);

                            let _ = user_interfaces.insert(id, ui);
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
                            platform_specific_handler.update_subsurfaces(
                                id,
                                window.raw.rwh_06_window_handle(),
                            );
                            debug.draw_finished();

                            if new_mouse_interaction != window.mouse_interaction
                            {
                                if let Some(interaction) =
                                    conversion::mouse_interaction(
                                        new_mouse_interaction,
                                    )
                                {
                                    if matches!(
                                        window.mouse_interaction,
                                        mouse::Interaction::Hide
                                    ) {
                                        window.raw.set_cursor_visible(true);
                                    }
                                    window.raw.set_cursor(interaction.into())
                                } else {
                                    window.raw.set_cursor_visible(false);
                                }

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

                        window.raw.pre_present_notify();
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
                            Err(error) => {
                                match error {
                                    // This is an unrecoverable error.
                                    compositor::SurfaceError::OutOfMemory => {
                                        panic!("{:?}", error);
                                    }
                                    compositor::SurfaceError::NoDamage => {
                                        debug.render_finished();

                                        // TODO Ideally there would be a way to know if some widget wants to animate?
                                        let _ = control_sender.start_send(
                                        Control::ChangeFlow(
                                                ControlFlow::WaitUntil(
                                                    Instant::now().checked_add(
                                                        Duration::from_millis(100),
                                                    ).unwrap_or(Instant::now()),
                                                ),
                                            ),
                                        );
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
                                            window.request_redraw();
                                        }
                                    }
                                }
                            }
                        }
                    }
                    window_event => {
                        if !is_daemon
                            && matches!(
                                window_event,
                                winit::event::WindowEvent::Destroyed
                            )
                            && !is_window_opening
                            && window_manager.is_empty()
                        {
                            runtime.track(None.into_iter());
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
                            if func(window.raw.as_ref(), &window_event) {
                                continue;
                            }
                        }

                        if matches!(
                            window_event,
                            winit::event::WindowEvent::CloseRequested
                        ) && window.exit_on_close_request
                        {
                            _ = run_action(
                                Action::Window(runtime::window::Action::Close(
                                    id,
                                )),
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
                                &mut platform_specific_handler,
                            );
                        } else {
                            window.state.update(
                                window.raw.as_ref(),
                                &window_event,
                                &mut debug,
                            );
                            if let Some(event) = conversion::window_event(
                                window_event,
                                window.state.scale_factor(),
                                window.state.modifiers(),
                                window.raw.as_ref(),
                            ) {
                                events.push((Some(id), event));
                            }
                        }
                    }
                    _ => {}
                }
            }
            Event::AboutToWait => {
                let skip = events.is_empty() && messages.is_empty();
                if skip
                    && window_manager.iter_mut().all(|(_, w)| !w.resize_enabled)
                {
                    _ = control_sender
                        .start_send(Control::ChangeFlow(ControlFlow::Wait));
                    continue;
                }

                debug.event_processing_started();
                let mut uis_stale = false;
                let mut resized = false;
                for (id, window) in window_manager.iter_mut() {
                    if skip && !window.resize_enabled {
                        _ = control_sender
                            .start_send(Control::ChangeFlow(ControlFlow::Wait));
                        continue;
                    }
                    let mut window_events = vec![];

                    events.retain(|(window_id, event)| {
                        if *window_id == Some(id) {
                            window_events.push(event.clone());
                            false
                        } else {
                            true
                        }
                    });
                    let no_window_events = window_events.is_empty();
                    #[cfg(feature = "wayland")]
                    window_events.push(core::Event::PlatformSpecific(
                        core::event::PlatformSpecific::Wayland(
                            core::event::wayland::Event::RequestResize,
                        ),
                    ));
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

                    let mut needs_redraw =
                        !no_window_events || !messages.is_empty();

                    if let Some(requested_size) =
                        clipboard.requested_logical_size.lock().unwrap().take()
                    {
                        let requested_physical_size: PhysicalSize<u32> =
                            winit::dpi::PhysicalSize::from_logical(
                                requested_size.cast::<u32>(),
                                window.state.scale_factor(),
                            );

                        let physical_size = window.state.physical_size();
                        if requested_physical_size.width != physical_size.width
                            || requested_physical_size.height
                                != physical_size.height
                        {
                            // FIXME what to do when we are stuck in a configure event/resize request loop
                            // We don't have control over how winit handles this.
                            window.resize_enabled = true;
                            resized = true;
                            needs_redraw = true;
                            let s = winit::dpi::Size::Logical(
                                requested_size.cast(),
                            );
                            _ = window.raw.request_surface_size(s);
                            window.raw.set_min_surface_size(Some(s));
                            window.raw.set_max_surface_size(Some(s));
                            window.state.synchronize(
                                &program,
                                id,
                                window.raw.as_ref(),
                            );
                        }
                    }
                    if needs_redraw {
                        window.request_redraw();
                    } else {
                        _ = control_sender
                            .start_send(Control::ChangeFlow(ControlFlow::Wait));
                        continue;
                    }

                    if !uis_stale {
                        uis_stale =
                            matches!(ui_state, user_interface::State::Outdated);
                    }

                    for (event, status) in
                        window_events.into_iter().zip(statuses.into_iter())
                    {
                        runtime.broadcast(subscription::Event::Interaction {
                            window: id,
                            event,
                            status,
                        });
                    }
                }

                if !resized && skip {
                    _ = control_sender
                        .start_send(Control::ChangeFlow(ControlFlow::Wait));
                    continue;
                }

                for (id, event) in events.drain(..) {
                    if id.is_none()
                        && matches!(
                            event,
                            core::Event::Keyboard(_)
                                | core::Event::Touch(_)
                                | core::Event::Mouse(_)
                        )
                    {
                        _ = control_sender
                            .start_send(Control::ChangeFlow(ControlFlow::Wait));
                        continue;
                    }
                    runtime.broadcast(subscription::Event::Interaction {
                        window: id.unwrap_or(window::Id::NONE),
                        event,
                        status: core::event::Status::Ignored,
                    });
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
                            window.raw.as_ref(),
                        );

                        window.request_redraw();
                    }
                    rebuild_a11y_tree = true;

                    user_interfaces = ManuallyDrop::new(build_user_interfaces(
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

                    platform_specific_handler.retain_subsurfaces(|id| {
                        window_manager.get(id).is_some()
                            || dnd_surface_id == Some(id)
                    });
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
                    platform_specific_handler.clear_subsurface_list();

                    if new_mouse_interaction != window.mouse_interaction {
                        if let Some(interaction) =
                            conversion::mouse_interaction(new_mouse_interaction)
                        {
                            if matches!(
                                window.mouse_interaction,
                                mouse::Interaction::Hide
                            ) {
                                window.raw.set_cursor_visible(true);
                            }
                            window.raw.set_cursor(interaction.into())
                        } else {
                            window.raw.set_cursor_visible(false);
                        }

                        window.mouse_interaction = new_mouse_interaction;
                    }

                    // TODO once widgets can request to be redrawn, we can avoid always requesting a
                    // redraw
                    window.request_redraw();
                    runtime.broadcast(subscription::Event::Interaction {
                        window: id,
                        event: redraw_event,
                        status: core::event::Status::Ignored,
                    });

                    let _ = control_sender.start_send(Control::ChangeFlow(
                        match ui_state {
                            user_interface::State::Updated {
                                redraw_request: Some(redraw_request),
                                input_method,
                            } => {
                                window.request_input_method(input_method);
                                match redraw_request {
                                    window::RedrawRequest::NextFrame => {
                                        window.request_redraw();

                                        ControlFlow::Wait
                                    }
                                    window::RedrawRequest::At(at) => {
                                        ControlFlow::WaitUntil(at)
                                    }
                                }
                            }
                            _ => ControlFlow::Wait,
                        },
                    ));

                    window.draw_preedit();
                }

                debug.draw_finished();
            }

            Event::Dnd(e) => {
                trace!(target: "iced::winit::program", "Dnd event {:?}", e);
                match &e {
                    dnd::DndEvent::Offer(_, dnd::OfferEvent::Leave) => {
                        events.push((cur_dnd_surface, core::Event::Dnd(e)));
                        // XXX can't clear the dnd surface on leave because
                        // the data event comes after
                        // cur_dnd_surface = None;
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
                    dnd::DndEvent::Source(evt) => {
                        match evt {
                            dnd::SourceEvent::Finished
                            | dnd::SourceEvent::Cancelled => {
                                dnd_surface = None;
                                dnd_surface_id = None;
                            }
                            _ => {}
                        }
                        for w in window_manager.ids() {
                            events.push((Some(w), core::Event::Dnd(e.clone())));
                        }
                    }
                };
            }
            #[cfg(feature = "a11y")]
            Event::Accessibility(id, e) => {
                rebuild_a11y_tree = true;
                match e.action {
                    iced_accessibility::accesskit::Action::Focus => {
                        // TODO send a command for this
                    }
                    _ => {}
                }
                events.push((Some(id), conversion::a11y(e)));
            }
            #[cfg(feature = "a11y")]
            Event::AccessibilityEnabled(enabled) => {
                a11y_enabled = enabled;
            }
            Event::PlatformSpecific(e) => {
                crate::platform_specific::handle_event(
                    e,
                    &mut events,
                    &mut platform_specific_handler,
                    &program,
                    &mut compositor,
                    &mut window_manager,
                    &mut debug,
                    &mut user_interfaces,
                    &mut clipboard,
                    #[cfg(feature = "a11y")]
                    &mut adapters,
                );
            }
            _ => {
                // log ignored events?
            }
        }
        #[cfg(feature = "a11y")]
        {
            use iced_accessibility::{
                accesskit::{NodeBuilder, NodeId, Role, Tree, TreeUpdate},
                A11yId, A11yNode, A11yTree,
            };
            if !a11y_enabled || !rebuild_a11y_tree {
                continue;
            }

            for id in window_manager.ids() {
                let Some((a11y_id, adapter)) = adapters.get_mut(&id) else {
                    continue;
                };
                let Some(window) = window_manager.get(id) else {
                    continue;
                };
                let interface =
                    user_interfaces.get_mut(&id).expect("Get user interface");

                // TODO cleanup duplication
                let child_tree = interface.a11y_nodes(window.state.cursor());
                let mut root = NodeBuilder::new(Role::Window);
                root.set_name(window.state.title.to_string());
                let window_tree = A11yTree::node_with_child_tree(
                    A11yNode::new(root, *a11y_id),
                    child_tree,
                );
                let tree = Tree::new(NodeId(*a11y_id));

                let focus = Arc::new(std::sync::Mutex::new(None));
                let focus_clone = focus.clone();
                let operation: Box<dyn Operation<()>> =
                    Box::new(operation::map(
                        Box::new(operation::focusable::find_focused()),
                        move |id| {
                            let mut guard = focus.lock().unwrap();
                            _ = guard.replace(id);
                        },
                    ));
                let mut current_operation = Some(operation);

                while let Some(mut operation) = current_operation.take() {
                    interface.operate(&window.renderer, operation.as_mut());

                    match operation.finish() {
                        operation::Outcome::None => {}
                        operation::Outcome::Some(()) => {
                            break;
                        }
                        operation::Outcome::Chain(next) => {
                            current_operation = Some(next);
                        }
                    }
                }
                let mut guard = focus_clone.lock().unwrap();
                let focus = guard
                    .take()
                    .map(|id| A11yId::Widget(id))
                    .filter(|f_id| window_tree.contains(f_id));
                tracing::debug!(
                    "tree root: {:?}\nchildren: {:?}\nfocus: {:?}\n",
                    window_tree
                        .root()
                        .iter()
                        .map(|n| (n.node(), n.id()))
                        .collect::<Vec<_>>(),
                    window_tree
                        .children()
                        .iter()
                        .map(|n| (n.node(), n.id()))
                        .collect::<Vec<_>>(),
                    &focus,
                );
                let focus =
                    focus.map(|id| id.into()).unwrap_or_else(|| tree.root);
                adapter.update_if_active(|| TreeUpdate {
                    nodes: window_tree.into(),
                    tree: Some(tree),
                    focus,
                });
            }
        }
    }

    let _ = ManuallyDrop::into_inner(user_interfaces);
}

/// Builds a window's [`UserInterface`] for the [`Program`].
pub(crate) fn build_user_interface<'a, P: Program>(
    program: &'a P,
    cache: user_interface::Cache,
    renderer: &mut P::Renderer,
    size: Size,
    debug: &mut Debug,
    id: window::Id,
    raw: Arc<dyn winit::window::Window>,
    prev_dnd_destination_rectangles_count: usize,
    clipboard: &mut Clipboard,
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

    let dnd_rectangles = user_interface
        .dnd_rectangles(prev_dnd_destination_rectangles_count, renderer);
    let new_dnd_rectangles_count = dnd_rectangles.as_ref().len();
    if new_dnd_rectangles_count > 0 || prev_dnd_destination_rectangles_count > 0
    {
        clipboard.register_dnd_destination(
            DndSurface(Arc::new(Box::new(raw.clone()))),
            dnd_rectangles.into_rectangles(),
        );
    }

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

    let subscription = runtime.enter(|| program.subscription());
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
    platform_specific: &mut crate::platform_specific::PlatformSpecific,
) -> bool
where
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
                let _ = ui_caches.remove(&id);
                let _ = interfaces.remove(&id);
                #[cfg(feature = "wayland")]
                platform_specific
                    .send_wayland(platform_specific::Action::RemoveWindow(id));

                if let Some(window) = window_manager.remove(id) {
                    clipboard.register_dnd_destination(
                        DndSurface(Arc::new(Box::new(window.raw.clone()))),
                        Vec::new(),
                    );
                    let proxy = clipboard.proxy();
                    if clipboard.window_id() == Some(window.raw.id()) {
                        *clipboard = window_manager
                            .first()
                            .map(|window| window.raw.clone())
                            .zip(proxy)
                            .map(|(w, proxy)| {
                                Clipboard::connect(
                                    w,
                                    crate::clipboard::ControlSender {
                                        sender: control_sender.clone(),
                                        proxy,
                                    },
                                )
                            })
                            .unwrap_or_else(Clipboard::unconnected);
                    }

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
                    let _ = window.raw.request_surface_size(
                        winit::dpi::LogicalSize {
                            width: size.width,
                            height: size.height,
                        }
                        .into(),
                    );
                }
            }
            window::Action::GetSize(id, channel) => {
                if let Some(window) = window_manager.get_mut(id) {
                    let size = window
                        .raw
                        .surface_size()
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
                if let Some(window) = window_manager.get(id) {
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
            window::Action::GetScaleFactor(id, channel) => {
                if let Some(window) = window_manager.get_mut(id) {
                    let scale_factor = window.raw.scale_factor();

                    let _ = channel.send(scale_factor as f32);
                }
            }
            window::Action::Move(id, position) => {
                if let Some(window) = window_manager.get_mut(id) {
                    window.raw.set_outer_position(
                        winit::dpi::LogicalPosition {
                            x: position.x,
                            y: position.y,
                        }
                        .into(),
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
                            }
                            .into(),
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
            window::Action::EnableMousePassthrough(id) => {
                if let Some(window) = window_manager.get_mut(id) {
                    let _ = window.raw.set_cursor_hittest(false);
                }
            }
            window::Action::DisableMousePassthrough(id) => {
                if let Some(window) = window_manager.get_mut(id) {
                    let _ = window.raw.set_cursor_hittest(true);
                }
            }
            window::Action::EnableBlur(id) => {
                if let Some(window) = window_manager.get_mut(id) {
                    window.raw.set_blur(true);
                }
            }
            window::Action::DisableBlur(id) => {
                if let Some(window) = window_manager.get_mut(id) {
                    window.raw.set_blur(false);
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
            return true;
        }
        Action::Dnd(a) => match a {
            iced_runtime::dnd::DndAction::RegisterDndDestination {
                surface,
                rectangles,
            } => {
                clipboard.register_dnd_destination(surface, rectangles);
            }
            iced_runtime::dnd::DndAction::EndDnd => {
                clipboard.end_dnd();
            }
            iced_runtime::dnd::DndAction::PeekDnd(m, channel) => {
                let data = clipboard.peek_dnd(m);
                _ = channel.send(data);
            }
            iced_runtime::dnd::DndAction::SetAction(a) => {
                clipboard.set_action(a);
            }
        },
        Action::PlatformSpecific(a) => {
            platform_specific.send_action(a);
        }
    }
    false
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
                window.raw.clone(),
                window.prev_dnd_destination_rectangles_count,
                clipboard,
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
