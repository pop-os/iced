//! A windowing shell for Iced, on top of [`winit`].
//!
//! ![The native path of the Iced ecosystem](https://github.com/iced-rs/iced/blob/0525d76ff94e828b7b21634fa94a747022001c83/docs/graphs/native.png?raw=true)
//!
//! `iced_winit` offers some convenient abstractions on top of [`iced_runtime`]
//! to quickstart development when using [`winit`].
//!
//! It exposes a renderer-agnostic [`Program`] trait that can be implemented
//! and then run with a simple call. The use of this trait is optional.
//!
//! Additionally, a [`conversion`] module is available for users that decide to
//! implement a custom event loop.
//!
//! [`iced_runtime`]: https://github.com/iced-rs/iced/tree/0.14/runtime
//! [`winit`]: https://github.com/rust-windowing/winit
//! [`conversion`]: crate::conversion
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/iced-rs/iced/9ab6923e943f784985e9ef9ca28b10278297225d/docs/logo.svg"
)]
#![cfg_attr(docsrs, feature(doc_cfg))]
use dnd::DndSurface;
pub use iced_debug as debug;
pub use iced_program as program;
pub use program::core;
pub use program::graphics;
pub use program::runtime;
use raw_window_handle::HasWindowHandle;
pub use runtime::futures;
use window_clipboard::mime::ClipboardStoreData;
pub use winit;

#[cfg(feature = "a11y")]
pub mod a11y;
pub mod clipboard;
pub mod conversion;
pub mod platform_specific;

#[cfg(feature = "program")]
pub mod program;

#[cfg(feature = "system")]
pub mod system;

mod error;
mod proxy;
mod window;

pub use clipboard::Clipboard;
pub use error::Error;
pub use proxy::Proxy;
use winit::dpi::LogicalSize;
use winit::dpi::PhysicalPosition;
use winit::dpi::PhysicalSize;

use crate::core::mouse;
use crate::core::renderer;
use crate::core::theme;
use crate::core::time::Instant;
use crate::core::widget::operation;
use crate::core::{Point, Size};
use crate::futures::futures::channel::mpsc;
use crate::futures::futures::channel::oneshot;
use crate::futures::futures::task;
use crate::futures::futures::{Future, StreamExt};
use crate::futures::subscription;
use crate::futures::{Executor, Runtime};
use crate::graphics::{Compositor, Shell, compositor};
use crate::runtime::image;
use crate::runtime::system;
use crate::runtime::user_interface::{self, UserInterface};
use crate::runtime::{Action, Task};

use program::Program;
use window::WindowManager;

use rustc_hash::FxHashMap;
use std::borrow::Cow;
use std::mem::ManuallyDrop;
use std::slice;
use std::sync::Arc;

/// Runs a [`Program`] with the provided settings.
pub fn run<P>(program: P) -> Result<(), Error>
where
    P: Program + 'static,
    P::Theme: theme::Base,
{
    use winit::event_loop::EventLoop;

    let boot_span = debug::boot();
    let settings = program.settings();
    let window_settings = program.window();

    let event_loop = EventLoop::new().expect("Create event loop");

    #[cfg(feature = "wayland")]
    let is_wayland =
        winit::platform::wayland::EventLoopExtWayland::is_wayland(&event_loop);
    #[cfg(not(feature = "wayland"))]
    let is_wayland = false;

    // TODO this is new..
    let graphics_settings = settings.clone().into();
    let display_handle = event_loop.owned_display_handle();

    let (event_sender, event_receiver) = mpsc::unbounded();
    let (proxy, worker): (Proxy<<P as Program>::Message>, _) =
        Proxy::new(event_loop.create_proxy(), event_sender.clone());

    #[cfg(feature = "debug")]
    {
        let proxy = proxy.clone();

        debug::on_hotpatch(move || {
            proxy.send_action(Action::Reload);
        });
    }

    let mut runtime = {
        let executor =
            P::Executor::new().map_err(Error::ExecutorCreationFailed)?;
        executor.spawn(worker);

        Runtime::new(executor, proxy.clone())
    };

    let (program, task) = runtime.enter(|| program::Instance::new(program));
    let is_daemon = settings.is_daemon || window_settings.is_none();

    let task = if let Some(window_settings) = window_settings {
        let mut task = Some(task);

        let (_id, open) = runtime::window::open(window_settings);

        open.then(move |_| task.take().unwrap_or_else(Task::none))
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
    let (system_theme_sender, system_theme_receiver) = oneshot::channel();

    let instance = Box::pin(run_instance::<P>(
        program,
        runtime,
        proxy.clone(),
        boot_receiver,
        event_receiver,
        control_sender.clone(),
        display_handle,
        is_daemon,
        is_wayland,
        graphics_settings,
        settings.fonts.clone(),
        system_theme_receiver,
    ));

    let context = task::Context::from_waker(task::noop_waker_ref());

    struct BootConfig {
        sender: oneshot::Sender<()>,
        fonts: Vec<Cow<'static, [u8]>>,
        graphics_settings: graphics::Settings,
        control_sender: mpsc::UnboundedSender<Control>,
        is_wayland: bool,
    }
    struct Runner<Message: 'static, F> {
        instance: std::pin::Pin<Box<F>>,
        context: task::Context<'static>,
        id: Option<String>,
        boot: Option<BootConfig>,
        sender: mpsc::UnboundedSender<Event<Message>>,
        receiver: mpsc::UnboundedReceiver<Control>,
        error: Option<Error>,
        system_theme: Option<oneshot::Sender<theme::Mode>>,

        #[cfg(target_arch = "wasm32")]
        is_booted: std::rc::Rc<std::cell::RefCell<bool>>,
        #[cfg(target_arch = "wasm32")]
        canvas: Option<web_sys::HtmlCanvasElement>,
    }

    let runner = Runner {
        instance,
        context,
        boot: Some(BootConfig {
            sender: boot_sender,
            fonts: settings.fonts,
            graphics_settings,
            control_sender,
            is_wayland,
        }),
        id: settings.id,
        sender: event_sender,
        receiver: control_receiver,
        error: None,
        system_theme: Some(system_theme_sender),

        #[cfg(target_arch = "wasm32")]
        canvas: None,
    };

    boot_span.finish();

    impl<Message, F> winit::application::ApplicationHandler for Runner<Message, F>
    where
        F: Future<Output = ()>,
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
                    self.process_event(
                        event_loop,
                        Some(Event::EventLoopAwakened(
                            winit::Event::AboutToWait,
                        )),
                    );
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

            let finish_boot = async move {
                sender.send(()).ok().expect("Send boot event");
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

    impl<Message, F> Runner<Message, F>
    where
        F: Future<Output = ()>,
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
                                    ) if current < new => {}
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
                                scale_factor,
                                monitor,
                                on_open,
                            } => {
                                let exit_on_close_request =
                                    settings.exit_on_close_request;

                                let visible = settings.visible;

                                #[cfg(target_arch = "wasm32")]
                                let target =
                                    settings.platform_specific.target.clone();

                                let window_attributes =
                                    conversion::window_attributes(
                                        settings,
                                        &title,
                                        scale_factor,
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

                                log::info!(
                                    "Window attributes for id `{id:#?}`: {window_attributes:#?}"
                                );

                                // On macOS, the `position` in `WindowAttributes` represents the "inner"
                                // position of the window; while on other platforms it's the "outer" position.
                                // We fix the inconsistency on macOS by positioning the window after creation.
                                #[cfg(target_os = "macos")]
                                let mut window_attributes = window_attributes;

                                #[cfg(target_os = "macos")]
                                let position =
                                    window_attributes.position.take();

                                let window = event_loop
                                    .create_window(window_attributes)
                                    .expect("Create window");

                                #[cfg(target_os = "macos")]
                                if let Some(position) = position {
                                    window.set_outer_position(position);
                                }

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
                                        window: Arc::from(window),
                                        exit_on_close_request,
                                        make_visible: visible,
                                        on_open,
                                    }),
                                );
                            }
                            Control::Exit => {
                                self.process_event(
                                    event_loop,
                                    Some(Event::Exit),
                                );
                                event_loop.exit();
                                break;
                            }
                            Control::Crash(error) => {
                                self.error = Some(error);
                                event_loop.exit();
                            }
                            Control::SetAutomaticWindowTabbing(_enabled) => {
                                #[cfg(target_os = "macos")]
                                {
                                    use winit::platform::macos::ActiveEventLoopExtMacOS;
                                    event_loop
                                        .set_allows_automatic_window_tabbing(
                                            _enabled,
                                        );
                                }
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
        event_loop.run_app(runner).map_err(error::Error::EventLoop)
    }

    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::EventLoopExtWebSys;
        let _ = event_loop.spawn_app(runner);

        Ok(())
    }
}

#[derive(Debug)]
enum Event<Message: 'static> {
    WindowCreated {
        id: window::Id,
        window: Arc<dyn winit::window::Window>,
        exit_on_close_request: bool,
        make_visible: bool,
        on_open: oneshot::Sender<window::Id>,
    },
    Exit,
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

#[derive(Debug)]
enum Control {
    ChangeFlow(winit::event_loop::ControlFlow),
    Exit,
    Crash(Error),
    CreateWindow {
        id: window::Id,
        settings: window::Settings,
        title: String,
        monitor: Option<winit::monitor::MonitorHandle>,
        on_open: oneshot::Sender<window::Id>,
        scale_factor: f64,
    },
    SetAutomaticWindowTabbing(bool),
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

async fn run_instance<P>(
    mut program: program::Instance<P>,
    mut runtime: Runtime<P::Executor, Proxy<P::Message>, Action<P::Message>>,
    mut proxy: Proxy<P::Message>,
    boot: oneshot::Receiver<()>,
    mut event_receiver: mpsc::UnboundedReceiver<Event<P::Message>>,
    mut control_sender: mpsc::UnboundedSender<Control>,
    display_handle: winit::event_loop::OwnedDisplayHandle,
    is_daemon: bool,
    is_wayland: bool,
    graphics_settings: graphics::Settings,
    default_fonts: Vec<Cow<'static, [u8]>>,
    mut _system_theme: oneshot::Receiver<theme::Mode>,
) where
    P: Program + 'static,
    P::Theme: theme::Base,
{
    use winit::event;
    use winit::event_loop::ControlFlow;

    _ = boot.await.expect("Receive boot");

    let mut platform_specific_handler =
        crate::platform_specific::PlatformSpecific::default();
    #[cfg(all(feature = "wayland", target_os = "linux"))]
    if is_wayland {
        platform_specific_handler = platform_specific_handler.with_wayland(
            control_sender.clone(),
            proxy.raw.clone(),
            display_handle.clone(),
        );
    }

    let mut window_manager: WindowManager<
        P,
        <<P as Program>::Renderer as compositor::Default>::Compositor,
    > = WindowManager::new();
    let mut is_window_opening = !is_daemon;

    let mut compositor = None;
    let mut events = Vec::new();
    let mut messages = Vec::new();
    let mut actions = 0;

    let mut ui_caches = FxHashMap::default();
    let mut user_interfaces = ManuallyDrop::new(FxHashMap::default());
    let mut clipboard = Clipboard::unconnected();
    let mut cur_dnd_surface: Option<window::Id> = None;

    let mut dnd_surface: Option<
        Arc<Box<dyn HasWindowHandle + Send + Sync + 'static>>,
    > = None;
    let mut dnd_surface_id: Option<window::Id> = None;

    #[cfg(feature = "a11y")]
    let (mut adapters, mut a11y_enabled) = (Default::default(), false);
    // let (mut adapters, mut a11y_enabled) = if let Some((main_id, title, raw)) =
    //     window_manager.ids().next().and_then(|id| {
    //         window_manager
    //             .get(id)
    //             .map(|w| (id, w.state.title.clone(), w.raw.clone()))
    //     }) {
    //     let node_id = core::id::window_node_id();
    //     use crate::a11y::*;
    //     use iced_accessibility::accesskit::{
    //         ActivationHandler, Node, NodeId, Role, Tree, TreeUpdate,
    //     };
    //     use iced_accessibility::accesskit_winit::Adapter;

    //     let activation_handler = WinitActivationHandler {
    //         proxy: control_sender.clone(),
    //         title: title.clone(),
    //     };

    //     let action_handler = WinitActionHandler {
    //         id: main_id,
    //         proxy: control_sender.clone(),
    //     };

    //     let deactivation_handler = WinitDeactivationHandler {
    //         proxy: control_sender.clone(),
    //     };
    //     (
    //         HashMap::from([(
    //             main_id,
    //             (
    //                 node_id,
    //                 Adapter::with_direct_handlers(
    //                     raw.as_ref(),
    //                     activation_handler,
    //                     action_handler,
    //                     deactivation_handler,
    //                 ),
    //             ),
    //         )]),
    //         false,
    //     )
    // } else {
    //     (Default::default(), false)
    // };

    #[cfg(all(feature = "linux-theme-detection", target_os = "linux"))]
    let mut system_theme = {
        let to_mode = |color_scheme| match color_scheme {
            mundy::ColorScheme::NoPreference => theme::Mode::None,
            mundy::ColorScheme::Light => theme::Mode::Light,
            mundy::ColorScheme::Dark => theme::Mode::Dark,
        };

        runtime.run(
            mundy::Preferences::stream(mundy::Interest::ColorScheme)
                .map(move |preferences| {
                    Action::System(system::Action::NotifyTheme(to_mode(
                        preferences.color_scheme,
                    )))
                })
                .boxed(),
        );

        runtime
            .enter(|| {
                mundy::Preferences::once_blocking(
                    mundy::Interest::ColorScheme,
                    core::time::Duration::from_millis(200),
                )
            })
            .map(|preferences| to_mode(preferences.color_scheme))
            .unwrap_or_default()
    };

    #[cfg(not(all(feature = "linux-theme-detection", target_os = "linux")))]
    let mut system_theme =
        _system_theme.try_recv().ok().flatten().unwrap_or_default();

    log::info!("System theme: {system_theme:?}");

    'next_event: loop {
        // Empty the queue if possible
        let event = if let Ok(event) = event_receiver.try_next() {
            event
        } else {
            event_receiver.next().await
        };

        let Some(event) = event else {
            break;
        };

        match event {
            Event::WindowCreated {
                id,
                window,
                exit_on_close_request,
                make_visible,
                on_open,
            } => {
                if compositor.is_none() {
                    let (compositor_sender, compositor_receiver) =
                        oneshot::channel();

                    let create_compositor = {
                        let window = window.clone();
                        let display_handle = display_handle.clone();
                        let proxy = proxy.clone();
                        let default_fonts = default_fonts.clone();

                        async move {
                            let shell = Shell::new(proxy.clone());

                            let mut compositor =
                                <P::Renderer as compositor::Default>::Compositor::new(
                                    graphics_settings,
                                    display_handle,
                                    window,
                                    shell,
                                ).await;

                            if let Ok(compositor) = &mut compositor {
                                for font in default_fonts {
                                    compositor.load_font(font.clone());
                                }
                            }

                            compositor_sender
                                .send(compositor)
                                .ok()
                                .expect("Send compositor");

                            // HACK! Send a proxy event on completion to trigger
                            // a runtime re-poll
                            // TODO: Send compositor through proxy (?)
                            {
                                let (sender, _receiver) = oneshot::channel();

                                proxy.send_action(Action::Window(
                                    runtime::window::Action::GetLatest(sender),
                                ));
                            }
                        }
                    };

                    #[cfg(target_arch = "wasm32")]
                    wasm_bindgen_futures::spawn_local(create_compositor);

                    #[cfg(not(target_arch = "wasm32"))]
                    runtime.block_on(create_compositor);

                    match compositor_receiver
                        .await
                        .expect("Wait for compositor")
                    {
                        Ok(new_compositor) => {
                            compositor = Some(new_compositor);
                        }
                        Err(error) => {
                            let _ = control_sender
                                .start_send(Control::Crash(error.into()));
                            continue;
                        }
                    }
                }

                let window_theme = window
                    .theme()
                    .map(conversion::theme_mode)
                    .unwrap_or_default();

                if system_theme != window_theme {
                    system_theme = window_theme;

                    runtime.broadcast(subscription::Event::SystemThemeChanged(
                        window_theme,
                    ));
                }

                let is_first = window_manager.is_empty();
                let compositor: &mut <<P as Program>::Renderer as compositor::Default >::Compositor = compositor
                    .as_mut()
                    .expect("Compositor must be initialized");
                let window = window_manager.insert(
                    id,
                    window,
                    &program,
                    compositor,
                    exit_on_close_request,
                    system_theme,
                );

                window.raw.set_theme(conversion::window_theme(
                    window.state.theme_mode(),
                ));

                debug::theme_changed(|| {
                    if is_first {
                        theme::Base::palette(window.state.theme())
                    } else {
                        None
                    }
                });

                let logical_size = window.state.logical_size();

                let _ = user_interfaces.insert(
                    id,
                    build_user_interface(
                        &program,
                        user_interface::Cache::default(),
                        &mut window.renderer,
                        logical_size,
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
                        size: window.logical_size(),
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
            Event::NewEvents(event::StartCause::Init) => {
                for (_id, window) in window_manager.iter_mut() {
                    window.raw.request_redraw();
                }
            }
            Event::NewEvents(event::StartCause::ResumeTimeReached {
                ..
            }) => {
                let now = Instant::now();

                for (_id, window) in window_manager.iter_mut() {
                    if let Some(redraw_at) = window.redraw_at
                        && redraw_at <= now
                    {
                        window.raw.request_redraw();
                        window.redraw_at = None;
                    }
                }

                if let Some(redraw_at) = window_manager.redraw_at() {
                    let _ = control_sender.start_send(Control::ChangeFlow(
                        ControlFlow::WaitUntil(redraw_at),
                    ));
                } else {
                    let _ = control_sender
                        .start_send(Control::ChangeFlow(ControlFlow::Wait));
                }
            }
            Event::UserEvent(action) => {
                let exited = run_action(
                    action,
                    &program,
                    &mut runtime,
                    &mut compositor,
                    &mut events,
                    &mut messages,
                    &mut clipboard,
                    &mut control_sender,
                    &mut user_interfaces,
                    &mut window_manager,
                    &mut ui_caches,
                    &mut is_window_opening,
                    &mut system_theme,
                    &mut platform_specific_handler,
                );
                if exited {
                    runtime.track(None.into_iter());
                }
                actions += 1;
            }
            Event::Winit(window_id, event::WindowEvent::RedrawRequested) => {
                let Some(mut current_compositor) = compositor.as_mut() else {
                    continue;
                };

                let Some((id, mut window)) =
                    window_manager.get_mut_alias(window_id)
                else {
                    continue;
                };
                window.redraw_requested = false;

                let physical_size = window.state.physical_size();
                let mut logical_size = window.state.logical_size();

                if physical_size.width == 0 || physical_size.height == 0 {
                    continue;
                }

                // Window was resized between redraws
                if window.surface_version != window.state.surface_version() {
                    let ui = user_interfaces
                        .remove(&id)
                        .expect("Remove user interface");

                    let layout_span = debug::layout(id);
                    let _ = user_interfaces.insert(
                        id,
                        ui.relayout(logical_size, &mut window.renderer),
                    );
                    layout_span.finish();

                    current_compositor.configure_surface(
                        &mut window.surface,
                        physical_size.width,
                        physical_size.height,
                    );

                    window.surface_version = window.state.surface_version();
                }

                let redraw_event = core::Event::Window(
                    window::Event::RedrawRequested(Instant::now()),
                );

                let cursor = window.state.cursor();

                let mut interface =
                    user_interfaces.get_mut(&id).expect("Get user interface");

                let interact_span = debug::interact(id);
                let mut redraw_count = 0;

                let state = loop {
                    let message_count = messages.len();
                    let (state, _) = interface.update(
                        slice::from_ref(&redraw_event),
                        cursor,
                        &mut window.renderer,
                        &mut clipboard,
                        &mut messages,
                    );

                    if message_count == messages.len()
                        && !state.has_layout_changed()
                    {
                        break state;
                    }

                    if redraw_count >= 2 {
                        log::warn!(
                            "More than 3 consecutive RedrawRequested events \
                                    produced layout invalidation"
                        );

                        break state;
                    }

                    redraw_count += 1;

                    if !messages.is_empty() {
                        let caches: FxHashMap<_, _> =
                            ManuallyDrop::into_inner(user_interfaces)
                                .into_iter()
                                .map(|(id, interface)| {
                                    (id, interface.into_cache())
                                })
                                .collect();

                        let actions =
                            update(&mut program, &mut runtime, &mut messages);

                        user_interfaces =
                            ManuallyDrop::new(build_user_interfaces(
                                &program,
                                &mut window_manager,
                                caches,
                                &mut clipboard,
                            ));

                        for action in actions {
                            // Defer all window actions to avoid compositor
                            // race conditions while redrawing
                            if let Action::Window(_) = action {
                                proxy.send_action(action);
                                continue;
                            }

                            _ = run_action(
                                action,
                                &program,
                                &mut runtime,
                                &mut compositor,
                                &mut events,
                                &mut messages,
                                &mut clipboard,
                                &mut control_sender,
                                &mut user_interfaces,
                                &mut window_manager,
                                &mut ui_caches,
                                &mut is_window_opening,
                                &mut system_theme,
                                &mut platform_specific_handler,
                            );
                        }

                        for (window_id, window) in window_manager.iter_mut() {
                            // We are already redrawing this window
                            if window_id == id {
                                continue;
                            }

                            window.raw.request_redraw();
                        }

                        let Some(next_compositor) = compositor.as_mut() else {
                            continue 'next_event;
                        };

                        current_compositor = next_compositor;
                        window = window_manager.get_mut(id).unwrap();

                        // Window scale factor changed during a redraw request
                        if logical_size != window.state.logical_size() {
                            logical_size = window.state.logical_size();

                            log::debug!(
                                "Window scale factor changed during a redraw request"
                            );

                            let ui = user_interfaces
                                .remove(&id)
                                .expect("Remove user interface");

                            let layout_span = debug::layout(id);
                            let _ = user_interfaces.insert(
                                id,
                                ui.relayout(logical_size, &mut window.renderer),
                            );
                            layout_span.finish();
                        }

                        interface = user_interfaces.get_mut(&id).unwrap();
                    }
                };
                interact_span.finish();

                let draw_span = debug::draw(id);
                interface.draw(
                    &mut window.renderer,
                    window.state.theme(),
                    &renderer::Style {
                        icon_color: window.state.icon_color(),
                        text_color: window.state.text_color(),
                        scale_factor: window.state.scale_factor(),
                    },
                    cursor,
                );
                platform_specific_handler
                    .update_subsurfaces(id, window.raw.rwh_06_window_handle());
                draw_span.finish();

                if let user_interface::State::Updated {
                    redraw_request,
                    input_method,
                    mouse_interaction,
                    ..
                } = state
                {
                    window.request_redraw(redraw_request);
                    window.request_input_method(input_method);
                    window.update_mouse(mouse_interaction);
                }

                runtime.broadcast(subscription::Event::Interaction {
                    window: id,
                    event: redraw_event,
                    status: core::event::Status::Ignored,
                });

                window.draw_preedit();

                let present_span = debug::present(id);
                match current_compositor.present(
                    &mut window.renderer,
                    &mut window.surface,
                    window.state.viewport(),
                    window.state.background_color(),
                    || window.raw.pre_present_notify(),
                ) {
                    Ok(()) => {
                        present_span.finish();
                    }
                    Err(error) => match error {
                        compositor::SurfaceError::OutOfMemory => {
                            // This is an unrecoverable error.
                            panic!("{error:?}");
                        }
                        compositor::SurfaceError::Outdated
                        | compositor::SurfaceError::Lost => {
                            present_span.finish();

                            // Reconfigure surface and try redrawing
                            let physical_size = window.state.physical_size();

                            if error == compositor::SurfaceError::Lost {
                                window.surface = current_compositor
                                    .create_surface(
                                        window.raw.clone(),
                                        physical_size.width,
                                        physical_size.height,
                                    );
                            } else {
                                current_compositor.configure_surface(
                                    &mut window.surface,
                                    physical_size.width,
                                    physical_size.height,
                                );
                            }

                            window.raw.request_redraw();
                        }
                        _ => {
                            present_span.finish();

                            log::error!(
                                "Error {error:?} when \
                                        presenting surface."
                            );

                            // Try rendering all windows again next frame.
                            for (_id, window) in window_manager.iter_mut() {
                                window.raw.request_redraw();
                            }
                        }
                    },
                }
            }
            Event::Winit(window_id, event) => {
                if !is_daemon
                    && matches!(event, winit::event::WindowEvent::Destroyed)
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

                match event {
                    winit::event::WindowEvent::SurfaceResized(_) => {
                        window.raw.request_redraw();
                    }
                    winit::event::WindowEvent::ThemeChanged(theme) => {
                        let mode = conversion::theme_mode(theme);

                        if mode != system_theme {
                            system_theme = mode;

                            runtime.broadcast(
                                subscription::Event::SystemThemeChanged(mode),
                            );
                        }
                    }
                    _ => {}
                }

                if matches!(event, winit::event::WindowEvent::CloseRequested)
                    && window.exit_on_close_request
                {
                    _ = run_action(
                        Action::Window(runtime::window::Action::Close(id)),
                        &program,
                        &mut runtime,
                        &mut compositor,
                        &mut events,
                        &mut messages,
                        &mut clipboard,
                        &mut control_sender,
                        &mut user_interfaces,
                        &mut window_manager,
                        &mut ui_caches,
                        &mut is_window_opening,
                        &mut system_theme,
                        &mut platform_specific_handler,
                    );
                } else {
                    window.state.update(&program, window.raw.as_ref(), &event);

                    if let Some(event) = conversion::window_event(
                        event,
                        window.state.scale_factor(),
                        window.state.modifiers(),
                    ) {
                        events.push((Some(id), event));
                    }
                }
            }
            Event::AboutToWait => {
                if actions > 0 {
                    proxy.free_slots(actions);
                    actions = 0;
                }
                let skip = events.is_empty() && messages.is_empty();

                if skip && window_manager.is_idle() {
                    _ = control_sender
                        .start_send(Control::ChangeFlow(ControlFlow::Wait));
                    continue;
                }

                let mut uis_stale = false;
                let mut resized = false;
                for (id, window) in window_manager.iter_mut() {
                    if skip && !window.resize_enabled {
                        continue;
                    }
                    let interact_span = debug::interact(id);
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

                    #[cfg(feature = "unconditional-rendering")]
                    window.request_redraw(window::RedrawRequest::NextFrame);

                    if needs_redraw {
                        window.request_redraw(
                            core::window::RedrawRequest::NextFrame,
                        );
                    }
                    match ui_state {
                        user_interface::State::Updated {
                            redraw_request: _redraw_request,
                            mouse_interaction,
                            ..
                        } => {
                            window.update_mouse(mouse_interaction);

                            #[cfg(not(feature = "unconditional-rendering"))]
                            window.request_redraw(_redraw_request);
                        }
                        user_interface::State::Outdated => {
                            uis_stale = true;
                        }
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

                    interact_span.finish();
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

                if !messages.is_empty() || uis_stale {
                    let cached_interfaces: FxHashMap<_, _> =
                        ManuallyDrop::into_inner(user_interfaces)
                            .into_iter()
                            .map(|(id, ui)| (id, ui.into_cache()))
                            .collect();

                    let actions =
                        update(&mut program, &mut runtime, &mut messages);

                    user_interfaces = ManuallyDrop::new(build_user_interfaces(
                        &program,
                        &mut window_manager,
                        cached_interfaces,
                        &mut clipboard,
                    ));

                    for action in actions {
                        let exited = run_action(
                            action,
                            &program,
                            &mut runtime,
                            &mut compositor,
                            &mut events,
                            &mut messages,
                            &mut clipboard,
                            &mut control_sender,
                            &mut user_interfaces,
                            &mut window_manager,
                            &mut ui_caches,
                            &mut is_window_opening,
                            &mut system_theme,
                            &mut platform_specific_handler,
                        );
                        if exited {
                            runtime.track(None.into_iter());
                        }
                    }

                    platform_specific_handler.retain_subsurfaces(|id| {
                        window_manager.get(id).is_some()
                            || dnd_surface_id == Some(id)
                    });

                    for (_id, window) in window_manager.iter_mut() {
                        window.raw.request_redraw();
                    }
                }

                if let Some(redraw_at) = window_manager.redraw_at() {
                    let _ = control_sender.start_send(Control::ChangeFlow(
                        ControlFlow::WaitUntil(redraw_at),
                    ));
                } else {
                    let _ = control_sender
                        .start_send(Control::ChangeFlow(ControlFlow::Wait));
                }
            }
            Event::Exit => break,
            Event::Dnd(e) => {
                use winit::raw_window_handle::HasWindowHandle;
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
                                dnd_surface_id = None;
                                dnd_surface = None;
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
            Event::Accessibility(id, action_request) => {
                match action_request.action {
                    iced_accessibility::accesskit::Action::Focus => {
                        // TODO send a command for this
                    }
                    _ => {}
                }
                events.push((Some(id), conversion::a11y(action_request)));
            }
            #[cfg(feature = "a11y")]
            Event::AccessibilityEnabled(enabled) => {
                a11y_enabled = enabled;
            }
            Event::PlatformSpecific(e) => {
                if let Some(c) = compositor.as_mut() {
                    crate::platform_specific::handle_event(
                        e,
                        &mut events,
                        &mut platform_specific_handler,
                        &program,
                        c,
                        &mut window_manager,
                        &mut user_interfaces,
                        &mut clipboard,
                        #[cfg(feature = "a11y")]
                        &mut adapters,
                    );
                } else {
                    // TODO should we break?
                    break;
                }
            }
            Event::StartDnd => {
                let compositor = match compositor.as_mut() {
                    Some(c) => c,
                    None => {
                        tracing::error!("No compositor for DnD");
                        continue;
                    }
                };
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
                            core::clipboard::DndSource::Surface(s) => Some(s),
                            core::clipboard::DndSource::Widget(w) => {
                                // search windows for widget with operation
                                user_interfaces.iter_mut().find_map(
                                    |(ui_id, ui)| {
                                        let Some(ui_renderer) = window_manager
                                            .get_mut(ui_id.clone())
                                            .map(|w| &w.renderer)
                                        else {
                                            return None;
                                        };

                                        let operation: Box<
                                            dyn operation::Operation<()>,
                                        > = Box::new(operation::map(
                                            Box::new(
                                                operation::search_id::search_id(
                                                    w.clone(),
                                                ),
                                            ),
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
                                )
                            }
                        }
                    }) else {
                        eprintln!("No source surface");
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
                            .as_widget_mut()
                            .layout(&mut tree, &renderer, &lim);
                        icon_surface.element.as_widget_mut().diff(&mut tree);

                        let size = lim.resolve(
                            core::Length::Shrink,
                            core::Length::Shrink,
                            size.size(),
                        );
                        let viewport =
                            iced_graphics::Viewport::with_logical_size(
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
                            dnd::Icon::Surface(dnd::DndSurface(surface))
                        } else {
                            platform_specific_handler.clear_subsurface_list();
                            dnd::Icon::Buffer {
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
            _ => {}
        }
    }

    let _ = ManuallyDrop::into_inner(user_interfaces);
}

/// Builds a window's [`UserInterface`] for the [`Program`].
fn build_user_interface<'a, P: Program>(
    program: &'a program::Instance<P>,
    cache: user_interface::Cache,
    renderer: &mut P::Renderer,
    size: Size,
    id: window::Id,
    raw: Arc<dyn winit::window::Window>,
    prev_dnd_destination_rectangles_count: usize,
    clipboard: &mut Clipboard,
) -> UserInterface<'a, P::Message, P::Theme, P::Renderer>
where
    P::Theme: theme::Base,
{
    let view_span = debug::view(id);
    let view = program.view(id);
    view_span.finish();

    let layout_span = debug::layout(id);
    let user_interface = UserInterface::build(view, size, cache, renderer);
    layout_span.finish();

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
    program: &mut program::Instance<P>,
    runtime: &mut Runtime<E, Proxy<P::Message>, Action<P::Message>>,
    messages: &mut Vec<P::Message>,
) -> Vec<Action<P::Message>>
where
    P::Theme: theme::Base,
{
    use futures::futures;

    let mut actions = Vec::new();

    for message in messages.drain(..) {
        let task = runtime.enter(|| program.update(message));

        if let Some(mut stream) = runtime::task::into_stream(task) {
            let waker = futures::task::noop_waker_ref();
            let mut context = futures::task::Context::from_waker(waker);

            // Run immediately available actions synchronously (e.g. widget operations)
            loop {
                match runtime.enter(|| stream.poll_next_unpin(&mut context)) {
                    futures::task::Poll::Ready(Some(action)) => {
                        actions.push(action);
                    }
                    futures::task::Poll::Ready(None) => {
                        break;
                    }
                    futures::task::Poll::Pending => {
                        runtime.run(stream);
                        break;
                    }
                }
            }
        }
    }

    let subscription = runtime.enter(|| program.subscription());
    let recipes = subscription::into_recipes(subscription.map(Action::Output));

    runtime.track(recipes);

    actions
}

fn run_action<'a, P, C>(
    action: Action<P::Message>,
    program: &'a program::Instance<P>,
    runtime: &mut Runtime<P::Executor, Proxy<P::Message>, Action<P::Message>>,
    compositor: &mut Option<C>,
    events: &mut Vec<(Option<window::Id>, core::Event)>,
    messages: &mut Vec<P::Message>,
    clipboard: &mut Clipboard,
    control_sender: &mut mpsc::UnboundedSender<Control>,
    interfaces: &mut FxHashMap<
        window::Id,
        UserInterface<'a, P::Message, P::Theme, P::Renderer>,
    >,
    window_manager: &mut WindowManager<P, C>,
    ui_caches: &mut FxHashMap<window::Id, user_interface::Cache>,
    is_window_opening: &mut bool,
    system_theme: &mut theme::Mode,
    platform_specific: &mut crate::platform_specific::PlatformSpecific,
) -> bool
where
    P: Program,
    C: Compositor<Renderer = P::Renderer> + 'static,
    P::Theme: theme::Base,
{
    use crate::runtime::clipboard;
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
                        scale_factor: program.scale_factor(id),
                        monitor,
                        on_open: channel,
                    })
                    .expect("Send control action");

                *is_window_opening = true;
            }
            window::Action::Close(id) => {
                let _ = ui_caches.remove(&id);
                let _ = interfaces.remove(&id);
                let proxy = clipboard.proxy();

                if let Some(window) = window_manager.remove(id) {
                    if clipboard.window_id() == Some(window.raw.id()) {
                        *clipboard = window_manager
                            .first()
                            .map(|window| window.raw.clone())
                            .zip(proxy.clone())
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

                if window_manager.is_empty() {
                    *compositor = None;
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
            window::Action::DragResize(id, direction) => {
                if let Some(window) = window_manager.get_mut(id) {
                    let _ = window.raw.drag_resize_window(
                        conversion::resize_direction(direction),
                    );
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
            window::Action::SetMinSize(id, size) => {
                if let Some(window) = window_manager.get_mut(id) {
                    window.raw.set_min_surface_size(size.map(|size| {
                        winit::dpi::LogicalSize {
                            width: size.width,
                            height: size.height,
                        }
                        .into()
                    }));
                }
            }
            window::Action::SetMaxSize(id, size) => {
                if let Some(window) = window_manager.get_mut(id) {
                    window.raw.set_max_surface_size(size.map(|size| {
                        winit::dpi::LogicalSize {
                            width: size.width,
                            height: size.height,
                        }
                        .into()
                    }));
                }
            }
            window::Action::SetResizeIncrements(id, increments) => {
                if let Some(window) = window_manager.get_mut(id) {
                    window.raw.set_surface_resize_increments(increments.map(
                        |size| {
                            winit::dpi::LogicalSize {
                                width: size.width,
                                height: size.height,
                            }
                            .into()
                        },
                    ));
                }
            }
            window::Action::SetResizable(id, resizable) => {
                if let Some(window) = window_manager.get_mut(id) {
                    window.raw.set_resizable(resizable);
                }
            }
            window::Action::GetSize(id, channel) => {
                if let Some(window) = window_manager.get_mut(id) {
                    let size = window.logical_size();
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
                        .outer_position()
                        .map(|position: PhysicalPosition<i32>| {
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
            window::Action::SetMode(id, mode) => {
                if let Some(window) = window_manager.get_mut(id) {
                    window.raw.set_visible(conversion::visible(mode));
                    window.raw.set_fullscreen(conversion::fullscreen(
                        window.raw.current_monitor(),
                        mode,
                    ));
                }
            }
            window::Action::SetIcon(id, icon) => {
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
            window::Action::SetLevel(id, level) => {
                if let Some(window) = window_manager.get_mut(id) {
                    window
                        .raw
                        .set_window_level(conversion::window_level(level));
                }
            }
            window::Action::ShowSystemMenu(id) => {
                if let Some(window) = window_manager.get_mut(id)
                    && let mouse::Cursor::Available(point) =
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
            window::Action::GetRawId(id, channel) => {
                if let Some(window) = window_manager.get_mut(id) {
                    let _ = channel.send(window.raw.id().into_raw() as u64); // TODO make these types match?
                }
            }
            window::Action::Run(id, f) => {
                if let Some(window) = window_manager.get_mut(id) {
                    f(window);
                }
            }
            window::Action::Screenshot(id, channel) => {
                if let Some(window) = window_manager.get_mut(id)
                    && let Some(compositor) = compositor
                {
                    let bytes = compositor.screenshot(
                        &mut window.renderer,
                        window.state.viewport(),
                        window.state.background_color(),
                    );

                    let _ = channel.send(core::window::Screenshot::new(
                        bytes,
                        window.state.physical_size(),
                        window.state.scale_factor() as f32,
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
            window::Action::GetMonitorSize(id, channel) => {
                if let Some(window) = window_manager.get(id) {
                    let size = window
                        .raw
                        .current_monitor()
                        .and_then(|m| m.current_video_mode())
                        .map(|monitor| {
                            let scale = window.state.scale_factor();
                            let size =
                                monitor.size().to_logical(f64::from(scale));

                            Size::new(size.width, size.height)
                        });

                    let _ = channel.send(size);
                }
            }
            window::Action::SetAllowAutomaticTabbing(enabled) => {
                control_sender
                    .start_send(Control::SetAutomaticWindowTabbing(enabled))
                    .expect("Send control action");
            }
            window::Action::RedrawAll => {
                for (_id, window) in window_manager.iter_mut() {
                    window.raw.request_redraw();
                }
            }
            window::Action::RelayoutAll => {
                for (id, window) in window_manager.iter_mut() {
                    if let Some(ui) = interfaces.remove(&id) {
                        let _ = interfaces.insert(
                            id,
                            ui.relayout(
                                window.state.logical_size(),
                                &mut window.renderer,
                            ),
                        );
                    }

                    window.raw.request_redraw();
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
            system::Action::GetInformation(_channel) => {
                #[cfg(feature = "sysinfo")]
                {
                    if let Some(compositor) = compositor {
                        let graphics_info = compositor.information();

                        let _ = std::thread::spawn(move || {
                            let information = system_information(graphics_info);

                            let _ = _channel.send(information);
                        });
                    }
                }
            }
            system::Action::GetTheme(channel) => {
                let _ = channel.send(*system_theme);
            }
            system::Action::NotifyTheme(mode) => {
                if mode != *system_theme {
                    *system_theme = mode;

                    runtime.broadcast(subscription::Event::SystemThemeChanged(
                        mode,
                    ));
                }

                let Some(theme) = conversion::window_theme(mode) else {
                    return false;
                };

                for (_id, window) in window_manager.iter_mut() {
                    window.state.update(
                        program,
                        window.raw.as_ref(),
                        &winit::event::WindowEvent::ThemeChanged(theme),
                    );
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
        Action::Image(action) => match action {
            image::Action::Allocate(handle, sender) => {
                use core::Renderer as _;

                // TODO: Shared image cache in compositor
                if let Some((_id, window)) = window_manager.iter_mut().next() {
                    window.renderer.allocate_image(
                        &handle,
                        move |allocation| {
                            let _ = sender.send(allocation);
                        },
                    );
                }
            }
        },
        Action::LoadFont { bytes, channel } => {
            if let Some(compositor) = compositor {
                // TODO: Error handling (?)
                compositor.load_font(bytes.clone());

                let _ = channel.send(Ok(()));
            }
        }
        Action::Reload => {
            for (id, window) in window_manager.iter_mut() {
                let Some(ui) = interfaces.remove(&id) else {
                    continue;
                };

                let cache = ui.into_cache();
                let size = window.logical_size();

                let _ = interfaces.insert(
                    id,
                    build_user_interface(
                        program,
                        cache,
                        &mut window.renderer,
                        size,
                        id,
                        window.raw.clone(),
                        window.prev_dnd_destination_rectangles_count,
                        clipboard,
                    ),
                );

                window.raw.request_redraw();
            }
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
    program: &'a program::Instance<P>,
    window_manager: &mut WindowManager<P, C>,
    mut cached_user_interfaces: FxHashMap<window::Id, user_interface::Cache>,
    clipboard: &mut Clipboard,
) -> FxHashMap<window::Id, UserInterface<'a, P::Message, P::Theme, P::Renderer>>
where
    C: Compositor<Renderer = P::Renderer>,
    P::Theme: theme::Base,
{
    for (id, window) in window_manager.iter_mut() {
        window.state.synchronize(program, id, window.raw.as_ref());
    }

    debug::theme_changed(|| {
        window_manager
            .first()
            .and_then(|window| theme::Base::palette(window.state.theme()))
    });

    cached_user_interfaces
        .drain()
        .filter_map(|(id, cache)| {
            let window = window_manager.get_mut(id)?;

            Some((
                id,
                build_user_interface(
                    program,
                    cache,
                    &mut window.renderer,
                    window.state.logical_size(),
                    id,
                    window.raw.clone(),
                    window.prev_dnd_destination_rectangles_count,
                    clipboard,
                ),
            ))
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

#[cfg(feature = "sysinfo")]
fn system_information(
    graphics: compositor::Information,
) -> system::Information {
    use sysinfo::{Process, System};

    let mut system = System::new_all();
    system.refresh_all();

    let cpu_brand = system
        .cpus()
        .first()
        .map(|cpu| cpu.brand().to_string())
        .unwrap_or_default();

    let memory_used = sysinfo::get_current_pid()
        .and_then(|pid| system.process(pid).ok_or("Process not found"))
        .map(Process::memory)
        .ok();

    system::Information {
        system_name: System::name(),
        system_kernel: System::kernel_version(),
        system_version: System::long_os_version(),
        system_short_version: System::os_version(),
        cpu_brand,
        cpu_cores: system.physical_core_count(),
        memory_total: system.total_memory(),
        memory_used,
        graphics_adapter: graphics.adapter,
        graphics_backend: graphics.backend,
    }
}
#[cfg(feature = "program")]
pub use program::Program;

pub use platform_specific::*;
