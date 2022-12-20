#[cfg(feature = "glow")]
use iced_glow::window::Compositor as GlowCompositor;
use iced_graphics::{
    compositor::{self, Compositor as _, Information, SurfaceError},
    Color, Error, Viewport,
};
#[cfg(feature = "softbuffer")]
use iced_softbuffer::window::Compositor as softbufferCompositor;
#[cfg(feature = "wgpu")]
use iced_wgpu::window::Compositor as WgpuCompositor;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use std::env;

use crate::Renderer;

/// A window graphics backend for iced powered by `glow`.
pub enum Compositor<Theme> {
    #[cfg(feature = "glow")]
    Glow(GlowCompositor<Theme>),
    #[cfg(feature = "softbuffer")]
    softbuffer(softbufferCompositor<Theme>),
    #[cfg(feature = "wgpu")]
    Wgpu(WgpuCompositor<Theme>),
}

pub enum Surface<Theme> {
    #[cfg(feature = "glow")]
    Glow(<GlowCompositor<Theme> as compositor::Compositor>::Surface),
    #[cfg(feature = "softbuffer")]
    softbuffer(
        <softbufferCompositor<Theme> as compositor::Compositor>::Surface,
    ),
    #[cfg(feature = "wgpu")]
    Wgpu(<WgpuCompositor<Theme> as compositor::Compositor>::Surface),
}

impl<Theme> Compositor<Theme> {
    #[cfg(feature = "glow")]
    fn new_glow<W: HasRawWindowHandle + HasRawDisplayHandle>(
        settings: crate::Settings,
        compatible_window: Option<&W>,
    ) -> Result<(Self, Renderer<Theme>), Error> {
        match GlowCompositor::new(
            iced_glow::Settings {
                default_font: settings.default_font,
                default_text_size: settings.default_text_size,
                text_multithreading: settings.text_multithreading,
                antialiasing: settings.antialiasing,
                ..iced_glow::Settings::from_env()
            },
            compatible_window,
        ) {
            Ok((compositor, renderer)) => {
                Ok((Compositor::Glow(compositor), Renderer::Glow(renderer)))
            }
            Err(err) => Err(err),
        }
    }

    #[cfg(feature = "softbuffer")]
    fn new_softbuffer<W: HasRawWindowHandle + HasRawDisplayHandle>(
        settings: crate::Settings,
        compatible_window: Option<&W>,
    ) -> Result<(Self, Renderer<Theme>), Error> {
        match softbufferCompositor::new(
            iced_softbuffer::Settings {
                default_font: settings.default_font,
                default_text_size: settings.default_text_size,
                text_multithreading: settings.text_multithreading,
                antialiasing: settings.antialiasing,
                ..iced_softbuffer::Settings::from_env()
            },
            compatible_window,
        ) {
            Ok((compositor, renderer)) => Ok((
                Compositor::softbuffer(compositor),
                Renderer::softbuffer(renderer),
            )),
            Err(err) => Err(err),
        }
    }

    #[cfg(feature = "wgpu")]
    fn new_wgpu<W: HasRawWindowHandle + HasRawDisplayHandle>(
        settings: crate::Settings,
        compatible_window: Option<&W>,
    ) -> Result<(Self, Renderer<Theme>), Error> {
        match WgpuCompositor::new(
            iced_wgpu::Settings {
                default_font: settings.default_font,
                default_text_size: settings.default_text_size,
                text_multithreading: settings.text_multithreading,
                antialiasing: settings.antialiasing,
                ..iced_wgpu::Settings::from_env()
            },
            compatible_window,
        ) {
            Ok((compositor, renderer)) => {
                Ok((Compositor::Wgpu(compositor), Renderer::Wgpu(renderer)))
            }
            Err(err) => Err(err),
        }
    }
}

/// A graphics compositor that can draw to windows.
impl<Theme> compositor::Compositor for Compositor<Theme> {
    /// The settings of the backend.
    type Settings = crate::Settings;

    /// The iced renderer of the backend.
    type Renderer = Renderer<Theme>;

    /// The surface of the backend.
    type Surface = Surface<Theme>;

    /// Creates a new [`Compositor`].
    fn new<W: HasRawWindowHandle + HasRawDisplayHandle>(
        settings: Self::Settings,
        compatible_window: Option<&W>,
    ) -> Result<(Self, Self::Renderer), Error> {
        //TODO: move to settings!
        if let Ok(var) = env::var("ICED_DYREND") {
            return match var.as_str() {
                #[cfg(feature = "glow")]
                "glow" => Self::new_glow(settings, compatible_window),
                #[cfg(feature = "softbuffer")]
                "softbuffer" => {
                    Self::new_softbuffer(settings, compatible_window)
                }
                #[cfg(feature = "wgpu")]
                "wgpu" => Self::new_wgpu(settings, compatible_window),
                _ => Err(Error::BackendError(format!(
                    "ICED_DYREND value {:?} not supported",
                    var
                ))),
            };
        }

        #[cfg(feature = "wgpu")]
        {
            eprintln!("trying wgpu compositor");
            match Self::new_wgpu(settings, compatible_window) {
                Ok(ok) => {
                    eprintln!("initialized wgpu compositor");
                    return Ok(ok);
                }
                Err(err) => {
                    eprintln!(
                        "failed to initialize wgpu compositor: {:?}",
                        err
                    );
                }
            }
        }

        #[cfg(feature = "glow")]
        {
            eprintln!("trying glow compositor");
            match Self::new_glow(settings, compatible_window) {
                Ok(ok) => {
                    eprintln!("initialized glow compositor");
                    return Ok(ok);
                }
                Err(err) => {
                    eprintln!(
                        "failed to initialize glow compositor: {:?}",
                        err
                    );
                }
            }
        }

        #[cfg(feature = "softbuffer")]
        {
            eprintln!("trying softbuffer compositor");
            match Self::new_softbuffer(settings, compatible_window) {
                Ok(ok) => {
                    eprintln!("initialized softbuffer compositor");
                    return Ok(ok);
                }
                Err(err) => {
                    eprintln!(
                        "failed to initialize softbuffer compositor: {:?}",
                        err
                    );
                }
            }
        }

        Err(Error::GraphicsAdapterNotFound)
    }

    /// Crates a new [`Surface`] for the given window.
    ///
    /// [`Surface`]: Self::Surface
    fn create_surface<W: HasRawWindowHandle + HasRawDisplayHandle>(
        &mut self,
        window: &W,
    ) -> Self::Surface {
        match self {
            #[cfg(feature = "glow")]
            Compositor::Glow(compositor) => {
                Surface::Glow(compositor.create_surface(window))
            }
            #[cfg(feature = "softbuffer")]
            Compositor::softbuffer(compositor) => {
                Surface::softbuffer(compositor.create_surface(window))
            }
            #[cfg(feature = "wgpu")]
            Compositor::Wgpu(compositor) => {
                Surface::Wgpu(compositor.create_surface(window))
            }
        }
    }

    /// Configures a new [`Surface`] with the given dimensions.
    ///
    /// [`Surface`]: Self::Surface
    fn configure_surface(
        &mut self,
        surface: &mut Self::Surface,
        width: u32,
        height: u32,
    ) {
        match (self, surface) {
            #[cfg(feature = "glow")]
            (Compositor::Glow(compositor), Surface::Glow(surface)) => {
                compositor.configure_surface(surface, width, height)
            }
            #[cfg(feature = "softbuffer")]
            (
                Compositor::softbuffer(compositor),
                Surface::softbuffer(surface),
            ) => compositor.configure_surface(surface, width, height),
            #[cfg(feature = "wgpu")]
            (Compositor::Wgpu(compositor), Surface::Wgpu(surface)) => {
                compositor.configure_surface(surface, width, height)
            }
            _ => panic!("dyrand configuring incorrect surface"),
        }
    }

    /// Returns [`Information`] used by this [`Compositor`].
    fn fetch_information(&self) -> Information {
        match self {
            #[cfg(feature = "glow")]
            Compositor::Glow(compositor) => compositor.fetch_information(),
            #[cfg(feature = "softbuffer")]
            Compositor::softbuffer(compositor) => {
                compositor.fetch_information()
            }
            #[cfg(feature = "wgpu")]
            Compositor::Wgpu(compositor) => compositor.fetch_information(),
        }
    }

    /// Presents the [`Renderer`] primitives to the next frame of the given [`Surface`].
    ///
    /// [`Renderer`]: Self::Renderer
    /// [`Surface`]: Self::Surface
    fn present<T: AsRef<str>>(
        &mut self,
        renderer: &mut Self::Renderer,
        surface: &mut Self::Surface,
        viewport: &Viewport,
        background: Color,
        overlay: &[T],
    ) -> Result<(), SurfaceError> {
        match (self, renderer, surface) {
            #[cfg(feature = "glow")]
            (
                Compositor::Glow(compositor),
                Renderer::Glow(renderer),
                Surface::Glow(surface),
            ) => compositor
                .present(renderer, surface, viewport, background, overlay),
            #[cfg(feature = "softbuffer")]
            (
                Compositor::softbuffer(compositor),
                Renderer::softbuffer(renderer),
                Surface::softbuffer(surface),
            ) => compositor
                .present(renderer, surface, viewport, background, overlay),
            #[cfg(feature = "wgpu")]
            (
                Compositor::Wgpu(compositor),
                Renderer::Wgpu(renderer),
                Surface::Wgpu(surface),
            ) => compositor
                .present(renderer, surface, viewport, background, overlay),
            _ => panic!("dyrand presenting incorrect renderer or surface"),
        }
    }
}
