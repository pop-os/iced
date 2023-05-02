use crate::{Element, Executor, Settings as Settings_, Subscription};
use iced_core::window::Id;

use crate::core::text;
pub use crate::{platform_specific::wayland as actions, Task};
use iced_renderer::graphics::{compositor, Antialiasing};
pub use iced_sctk::application::Appearance;
pub use iced_sctk::{
    application::{DefaultStyle, SurfaceIdWrapper},
    commands::*,
    settings::*,
};

/// A pure version of [`Application`].
///
/// Unlike the impure version, the `view` method of this trait takes an
/// immutable reference to `self` and returns a pure [`Element`].
pub trait Application: Sized {
    /// The [`Executor`] that will run commands and subscriptions.
    ///
    /// The [default executor] can be a good starting point!
    ///
    /// [`Executor`]: Self::Executor
    /// [default executor]: crate::executor::Default
    type Executor: Executor;

    /// The type of __messages__ your [`Application`] will produce.
    type Message: std::fmt::Debug + Send;

    /// The theme of your [`Application`].
    type Theme: Default + DefaultStyle;

    /// The renderer of your [`Application`].
    type Renderer: text::Renderer + compositor::Default;

    /// The data needed to initialize your [`Application`].
    type Flags;

    /// Initializes the [`Application`] with the flags provided to
    /// [`run`] as part of the [`Settings`].
    ///
    /// Here is where you should return the initial state of your app.
    ///
    /// Additionally, you can return a [`Task`] if you need to perform some
    /// async action in the background on startup. This is useful if you want to
    /// load state from a file, perform an initial HTTP request, etc.
    ///
    /// [`run`]: Self::run
    fn new(flags: Self::Flags) -> (Self, Task<Self::Message>);

    /// Returns the current title of the [`Application`].
    ///
    /// This title can be dynamic! The runtime will automatically update the
    /// title of your application when necessary.
    fn title(&self, id: Id) -> String;

    /// Handles a __message__ and updates the state of the [`Application`].
    ///
    /// This is where you define your __update logic__. All the __messages__,
    /// produced by either user interactions or commands, will be handled by
    /// this method.
    ///
    /// Any [`Task`] returned will be executed immediately in the background.
    fn update(&mut self, message: Self::Message) -> Task<Self::Message>;

    /// Returns the current [`Theme`] of the [`Application`].
    ///
    /// [`Theme`]: Self::Theme
    fn theme(&self, _id: Id) -> Self::Theme {
        Self::Theme::default()
    }

    /// Returns the current Style of the Theme.
    fn style(&self, theme: &Self::Theme) -> Appearance {
        theme.default_style()
    }

    /// Returns the event [`Subscription`] for the current state of the
    /// application.
    ///
    /// A [`Subscription`] will be kept alive as long as you keep returning it,
    /// and the __messages__ produced will be handled by
    /// [`update`](#tymethod.update).
    ///
    /// By default, this method returns an empty [`Subscription`].
    fn subscription(&self) -> Subscription<Self::Message> {
        Subscription::none()
    }

    /// Returns the widgets to display in the [`Application`].
    ///
    /// These widgets can produce __messages__ based on user interaction.
    fn view(
        &self,
        id: Id,
    ) -> Element<'_, Self::Message, Self::Theme, Self::Renderer>;

    /// Returns the scale factor of the [`Application`].
    ///
    /// It can be used to dynamically control the size of the UI at runtime
    /// (i.e. zooming).
    ///
    /// For instance, a scale factor of `2.0` will make widgets twice as big,
    /// while a scale factor of `0.5` will shrink them to half their size.
    ///
    /// By default, it returns `1.0`.
    fn scale_factor(&self, _id: Id) -> f64 {
        1.0
    }

    /// Runs the [`Application`].
    ///
    /// On native platforms, this method will take control of the current thread
    /// until the [`Application`] exits.
    ///
    /// On the web platform, this method __will NOT return__ unless there is an
    /// [`Error`] during startup.
    ///
    /// [`Error`]: crate::Error
    fn run(settings: Settings_<Self::Flags>) -> crate::Result
    where
        Self: 'static,
    {
        #[allow(clippy::needless_update)]
        let renderer_settings = crate::graphics::Settings {
            default_font: settings.default_font,
            default_text_size: settings.default_text_size,
            antialiasing: if settings.antialiasing {
                Some(Antialiasing::MSAAx4)
            } else {
                None
            },
            ..crate::graphics::Settings::default()
        };

        let run = crate::shell::application::run::<
            Instance<Self>,
            Self::Executor,
            <Self::Renderer as compositor::Default>::Compositor,
        >(settings.into(), renderer_settings);
        #[cfg(target_arch = "wasm32")]
        {
            use crate::futures::FutureExt;
            use iced_futures::backend::wasm::wasm_bindgen::Executor;

            Executor::new()
                .expect("Create Wasm executor")
                .spawn(run.map(|_| ()));

            Ok(())
        }

        #[cfg(not(target_arch = "wasm32"))]
        Ok(crate::futures::executor::block_on(run)?)
    }
}

struct Instance<A: Application>(A);

impl<A> crate::runtime::multi_window::Program for Instance<A>
where
    A: Application,
{
    type Theme = A::Theme;
    type Renderer = A::Renderer;
    type Message = A::Message;

    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        self.0.update(message)
    }

    fn view(
        &self,
        id: Id,
    ) -> Element<'_, Self::Message, Self::Theme, Self::Renderer> {
        self.0.view(id)
    }
}

impl<A> crate::shell::Application for Instance<A>
where
    A: Application,
{
    type Flags = A::Flags;

    fn new(flags: Self::Flags) -> (Self, Task<A::Message>) {
        let (app, command) = A::new(flags);

        (Instance(app), command)
    }

    fn title(&self, window: Id) -> String {
        self.0.title(window)
    }

    fn theme(&self, id: Id) -> A::Theme {
        self.0.theme(id)
    }

    fn style(&self, theme: &A::Theme) -> Appearance {
        self.0.style(theme)
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        self.0.subscription()
    }

    fn scale_factor(&self, window: Id) -> f64 {
        self.0.scale_factor(window)
    }
}
