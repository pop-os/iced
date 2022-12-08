use cosmic_text::{Attrs, AttrsList, BufferLine, FontSystem, SwashCache};
#[cfg(feature = "image")]
use iced_graphics::image::raster;
use iced_graphics::image::storage;
#[cfg(feature = "svg")]
use iced_graphics::image::vector;
use iced_graphics::{Primitive, Vector};
#[cfg(feature = "image")]
use iced_native::image;
use iced_native::layout;
use iced_native::renderer;
#[cfg(feature = "svg")]
use iced_native::svg;
use iced_native::text::{self, Text};
use iced_native::{Background, Element, Font, Point, Rectangle, Size};
use std::cell::RefCell;
use std::fmt;
use std::marker::PhantomData;

lazy_static::lazy_static! {
    pub(crate) static ref FONT_SYSTEM: FontSystem = FontSystem::new();
}

/// An entry in some [`Storage`],
pub(crate) struct CpuEntry {
    pub(crate) size: Size<u32>,
    pub(crate) data: Vec<u32>,
}

impl fmt::Debug for CpuEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CpuEntry")
         .field("size", &self.size)
         .finish()
    }
}

impl storage::Entry for CpuEntry {
    /// The [`Size`] of the [`Entry`].
    fn size(&self) -> Size<u32> {
        self.size
    }
}

/// Stores cached image data for use in rendering
#[derive(Debug)]
pub(crate) struct CpuStorage;

impl storage::Storage for CpuStorage {
    /// The type of an [`Entry`] in the [`Storage`].
    type Entry = CpuEntry;

    /// State provided to upload or remove a [`Self::Entry`].
    type State<'a> = ();

    /// Upload the image data of a [`Self::Entry`].
    fn upload(
        &mut self,
        width: u32,
        height: u32,
        data_u8: &[u8],
        state: &mut Self::State<'_>,
    ) -> Option<Self::Entry> {
        let mut data = Vec::with_capacity(data_u8.len() / 4);
        for chunk in data_u8.chunks_exact(4) {
            data.push(
                raqote::SolidSource::from_unpremultiplied_argb(
                    chunk[3],
                    chunk[0],
                    chunk[1],
                    chunk[2],
                ).to_u32()
            );
        }
        Some(Self::Entry {
            size: Size::new(width, height),
            data,
        })
    }

    /// Romve a [`Self::Entry`] from the [`Storage`].
    fn remove(&mut self, entry: &Self::Entry, state: &mut Self::State<'_>) {
        // no-op
    }
}

pub struct Renderer<T = iced_native::Theme> {
    #[cfg(feature = "image")]
    pub(crate) raster_cache: RefCell<raster::Cache<CpuStorage>>,
    #[cfg(feature = "svg")]
    pub(crate) vector_cache: RefCell<vector::Cache<CpuStorage>>,
    pub(crate) swash_cache: SwashCache<'static>,
    pub(crate) primitives: Vec<Primitive>,
    pub(crate) theme: PhantomData<T>,
}

impl<T> Renderer<T> {
    pub(crate) fn new() -> Self {
        Self {
            #[cfg(feature = "image")]
            raster_cache: RefCell::new(raster::Cache::default()),
            #[cfg(feature = "svg")]
            vector_cache: RefCell::new(vector::Cache::default()),
            swash_cache: SwashCache::new(&FONT_SYSTEM),
            primitives: Vec::new(),
            theme: PhantomData,
        }
    }
}

impl<T> iced_native::Renderer for Renderer<T> {
    type Theme = T;

    fn layout<'a, Message>(
        &mut self,
        element: &Element<'a, Message, Self>,
        limits: &layout::Limits,
    ) -> layout::Node {
        element.as_widget().layout(self, limits)
    }

    fn with_layer(&mut self, bounds: Rectangle, f: impl FnOnce(&mut Self)) {
        let current_primitives = std::mem::take(&mut self.primitives);

        f(self);

        let layer_primitives =
            std::mem::replace(&mut self.primitives, current_primitives);

        self.primitives.push(Primitive::Clip {
            bounds,
            content: Box::new(Primitive::Group {
                primitives: layer_primitives,
            }),
        });
    }

    fn with_translation(
        &mut self,
        translation: Vector,
        f: impl FnOnce(&mut Self),
    ) {
        let current_primitives = std::mem::take(&mut self.primitives);

        f(self);

        let layer_primitives =
            std::mem::replace(&mut self.primitives, current_primitives);

        self.primitives.push(Primitive::Translate {
            translation,
            content: Box::new(Primitive::Group {
                primitives: layer_primitives,
            }),
        });
    }

    fn fill_quad(
        &mut self,
        quad: renderer::Quad,
        background: impl Into<Background>,
    ) {
        self.primitives.push(Primitive::Quad {
            bounds: quad.bounds,
            background: background.into(),
            border_radius: quad.border_radius,
            border_width: quad.border_width,
            border_color: quad.border_color,
        });
    }

    fn clear(&mut self) {
        self.primitives.clear();
    }
}

impl<T> text::Renderer for Renderer<T> {
    type Font = Font;

    const ICON_FONT: Font = Font::Default;
    const CHECKMARK_ICON: char = '✓';
    const ARROW_DOWN_ICON: char = '⌄';

    fn default_size(&self) -> u16 {
        //TODO: get from settings
        16
    }

    fn measure(
        &self,
        content: &str,
        size: u16,
        font: Font,
        bounds: Size,
    ) -> (f32, f32) {
        //TODO: improve implementation
        let mut buffer_line = BufferLine::new(content, AttrsList::new(Attrs::new()));
        let layout = buffer_line.layout(&FONT_SYSTEM, size as i32, bounds.width as i32);

        //TODO: how to properly calculate line height?
        let line_height = size * 5 / 4;
        let mut width = 0.0;
        let mut height = 0.0;
        for layout_line in layout.iter() {
            for glyph in layout_line.glyphs.iter() {
                let max_x = if glyph.rtl {
                    glyph.x - glyph.w
                } else {
                    glyph.x + glyph.w
                };
                if max_x + 1.0 > width {
                    width = max_x + 1.0;
                }
            }

            height += line_height as f32;
        }
        (width, height)
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
        //TODO: implement hit test
        None
    }

    fn fill_text(&mut self, text: Text<'_, Self::Font>) {
        self.primitives.push(Primitive::Text {
            content: text.content.to_string(),
            bounds: text.bounds,
            size: text.size,
            color: text.color,
            font: text.font,
            horizontal_alignment: text.horizontal_alignment,
            vertical_alignment: text.vertical_alignment,
        });
    }
}

#[cfg(feature = "image")]
impl<T> image::Renderer for Renderer<T> {
    type Handle = image::Handle;

    fn dimensions(&self, handle: &image::Handle) -> Size<u32> {
        let mut cache = self.raster_cache.borrow_mut();
        let memory = cache.load(handle);

        memory.dimensions()
    }

    fn draw(&mut self, handle: image::Handle, bounds: Rectangle) {
        self.primitives.push(Primitive::Image { handle, bounds })
    }
}

#[cfg(feature = "svg")]
impl<T> svg::Renderer for Renderer<T> {
    fn dimensions(&self, handle: &svg::Handle) -> Size<u32> {
        let mut cache = self.vector_cache.borrow_mut();
        let svg = cache.load(handle);

        svg.viewport_dimensions()
    }

    fn draw(&mut self, handle: svg::Handle, bounds: Rectangle) {
        self.primitives.push(Primitive::Svg { handle, bounds })
    }
}
