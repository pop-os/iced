use iced_graphics::{Backend, Vector};
#[cfg(feature = "image")]
use iced_native::image;
use iced_native::layout;
use iced_native::renderer;
#[cfg(feature = "svg")]
use iced_native::svg;
use iced_native::text::{self, Text};
use iced_native::{Background, Color, Element, Font, Point, Rectangle, Size};

pub enum Renderer<Theme = iced_native::Theme> {
    #[cfg(feature = "glow")]
    Glow(iced_glow::Renderer<Theme>),
    #[cfg(feature = "swbuf")]
    SwBuf(iced_swbuf::Renderer<Theme>),
    #[cfg(feature = "wgpu")]
    Wgpu(iced_wgpu::Renderer<Theme>),
}

impl<T> iced_native::Renderer for Renderer<T> {
    type Theme = T;

    fn layout<'a, Message>(
        &mut self,
        element: &Element<'a, Message, Self>,
        limits: &layout::Limits,
    ) -> layout::Node {
        let layout = element.as_widget().layout(self, limits);

        match self {
            #[cfg(feature = "glow")]
            Renderer::Glow(renderer) => {
                renderer.backend_mut().trim_measurements();
            },
            #[cfg(feature = "swbuf")]
            Renderer::SwBuf(renderer) => {
                renderer.backend_mut().trim_measurements();
            },
            #[cfg(feature = "wgpu")]
            Renderer::Wgpu(renderer) => {
                renderer.backend_mut().trim_measurements();
            }
        }

        layout
    }

    fn with_layer(&mut self, bounds: Rectangle, f: impl FnOnce(&mut Self)) {
        let self_ptr = self as *mut _;
        match self {
            #[cfg(feature = "glow")]
            Renderer::Glow(renderer) => unsafe {
                // TODO: find a way to do this safely
                renderer.with_layer(bounds, |_| f(&mut *self_ptr))
            },
            #[cfg(feature = "swbuf")]
            Renderer::SwBuf(renderer) => unsafe {
                // TODO: find a way to do this safely
                renderer.with_layer(bounds, |_| f(&mut *self_ptr))
            },
            #[cfg(feature = "wgpu")]
            Renderer::Wgpu(renderer) => unsafe {
                // TODO: find a way to do this safely
                renderer.with_layer(bounds, |_| f(&mut *self_ptr))
            }
        }
    }

    fn with_translation(
        &mut self,
        translation: Vector,
        f: impl FnOnce(&mut Self),
    ) {
        let self_ptr = self as *mut _;
        match self {
            #[cfg(feature = "glow")]
            Renderer::Glow(renderer) => unsafe {
                // TODO: find a way to do this safely
                renderer.with_translation(translation, |_| f(&mut *self_ptr))
            },
            #[cfg(feature = "swbuf")]
            Renderer::SwBuf(renderer) => unsafe {
                // TODO: find a way to do this safely
                renderer.with_translation(translation, |_| f(&mut *self_ptr))
            },
            #[cfg(feature = "wgpu")]
            Renderer::Wgpu(renderer) => unsafe {
                // TODO: find a way to do this safely
                renderer.with_translation(translation, |_| f(&mut *self_ptr))
            }
        }
    }

    fn fill_quad(
        &mut self,
        quad: renderer::Quad,
        background: impl Into<Background>,
    ) {
        match self {
            #[cfg(feature = "glow")]
            Renderer::Glow(renderer) => {
                renderer.fill_quad(quad, background)
            },
            #[cfg(feature = "swbuf")]
            Renderer::SwBuf(renderer) => {
                renderer.fill_quad(quad, background)
            },
            #[cfg(feature = "wgpu")]
            Renderer::Wgpu(renderer) => {
                renderer.fill_quad(quad, background)
            }
        }
    }

    fn clear(&mut self) {
        match self {
            #[cfg(feature = "glow")]
            Renderer::Glow(renderer) => {
                renderer.clear()
            },
            #[cfg(feature = "swbuf")]
            Renderer::SwBuf(renderer) => {
                renderer.clear()
            },
            #[cfg(feature = "wgpu")]
            Renderer::Wgpu(renderer) => {
                renderer.clear()
            }
        }
    }
}

impl<T> text::Renderer for Renderer<T> {
    type Font = Font;

    //TODO: use the right values here for each backend
    const ICON_FONT: Font = Font::Default;
    const CHECKMARK_ICON: char = '✓';
    const ARROW_DOWN_ICON: char = '⌄';

    fn default_size(&self) -> u16 {
        match self {
            #[cfg(feature = "glow")]
            Renderer::Glow(renderer) => {
                renderer.default_size()
            },
            #[cfg(feature = "swbuf")]
            Renderer::SwBuf(renderer) => {
                renderer.default_size()
            },
            #[cfg(feature = "wgpu")]
            Renderer::Wgpu(renderer) => {
                renderer.default_size()
            }
        }
    }

    fn measure(
        &self,
        content: &str,
        size: u16,
        font: Font,
        bounds: Size,
    ) -> (f32, f32) {
        match self {
            #[cfg(feature = "glow")]
            Renderer::Glow(renderer) => {
                renderer.measure(content, size, font, bounds)
            },
            #[cfg(feature = "swbuf")]
            Renderer::SwBuf(renderer) => {
                renderer.measure(content, size, font, bounds)
            },
            #[cfg(feature = "wgpu")]
            Renderer::Wgpu(renderer) => {
                renderer.measure(content, size, font, bounds)
            }
        }
    }

    fn hit_test(
        &self,
        content: &str,
        size: f32,
        font: Font,
        bounds: Size,
        point: Point,
        nearest_only: bool,
    ) -> Option<text::Hit> {
        match self {
            #[cfg(feature = "glow")]
            Renderer::Glow(renderer) => {
                renderer.hit_test(content, size, font, bounds, point, nearest_only)
            },
            #[cfg(feature = "swbuf")]
            Renderer::SwBuf(renderer) => {
                renderer.hit_test(content, size, font, bounds, point, nearest_only)
            },
            #[cfg(feature = "wgpu")]
            Renderer::Wgpu(renderer) => {
                renderer.hit_test(content, size, font, bounds, point, nearest_only)
            }
        }
    }

    fn fill_text(&mut self, text: Text<'_, Self::Font>) {
        match self {
            #[cfg(feature = "glow")]
            Renderer::Glow(renderer) => {
                renderer.fill_text(text)
            },
            #[cfg(feature = "swbuf")]
            Renderer::SwBuf(renderer) => {
                renderer.fill_text(text)
            },
            #[cfg(feature = "wgpu")]
            Renderer::Wgpu(renderer) => {
                renderer.fill_text(text)
            }
        }
    }
}

#[cfg(feature = "image")]
impl<T> image::Renderer for Renderer<T> {
    type Handle = image::Handle;

    fn dimensions(&self, handle: &image::Handle) -> Size<u32> {
        match self {
            #[cfg(feature = "glow")]
            Renderer::Glow(renderer) => {
                renderer.dimensions(handle)
            },
            #[cfg(feature = "swbuf")]
            Renderer::SwBuf(renderer) => {
                renderer.dimensions(handle)
            },
            #[cfg(feature = "wgpu")]
            Renderer::Wgpu(renderer) => {
                renderer.dimensions(handle)
            }
        }
    }

    fn draw(&mut self, handle: image::Handle, bounds: Rectangle) {
        match self {
            #[cfg(feature = "glow")]
            Renderer::Glow(renderer) => {
                renderer.draw(handle, bounds)
            },
            #[cfg(feature = "swbuf")]
            Renderer::SwBuf(renderer) => {
                renderer.draw(handle, bounds)
            },
            #[cfg(feature = "wgpu")]
            Renderer::Wgpu(renderer) => {
                renderer.draw(handle, bounds)
            }
        }
    }
}

#[cfg(feature = "svg")]
impl<T> svg::Renderer for Renderer<T> {
    fn dimensions(&self, handle: &svg::Handle) -> Size<u32> {
        match self {
            #[cfg(feature = "glow")]
            Renderer::Glow(renderer) => {
                renderer.dimensions(handle)
            },
            #[cfg(feature = "swbuf")]
            Renderer::SwBuf(renderer) => {
                renderer.dimensions(handle)
            },
            #[cfg(feature = "wgpu")]
            Renderer::Wgpu(renderer) => {
                renderer.dimensions(handle)
            }
        }
    }

    fn draw(
        &mut self,
        handle: svg::Handle,
        color: Option<Color>,
        bounds: Rectangle,
    ) {
        match self {
            #[cfg(feature = "glow")]
            Renderer::Glow(renderer) => {
                renderer.draw(handle, color, bounds)
            },
            #[cfg(feature = "swbuf")]
            Renderer::SwBuf(renderer) => {
                renderer.draw(handle, color, bounds)
            },
            #[cfg(feature = "wgpu")]
            Renderer::Wgpu(renderer) => {
                renderer.draw(handle, color, bounds)
            }
        }
    }
}
