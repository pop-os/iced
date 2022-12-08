use cosmic_text::{Attrs, AttrsList, BufferLine, FontSystem, SwashCache};
use iced_graphics::font;
use iced_graphics::{Primitive, Vector};
use iced_native::layout;
use iced_native::renderer;
use iced_native::text::{self, Text};
use iced_native::{Background, Element, Font, Point, Rectangle, Size};
use std::marker::PhantomData;

lazy_static::lazy_static! {
    pub(crate) static ref FONT_SYSTEM: FontSystem = FontSystem::new();
}

pub struct Renderer<T = iced_native::Theme> {
    pub(crate) swash_cache: SwashCache<'static>,
    pub(crate) primitives: Vec<Primitive>,
    pub(crate) theme: PhantomData<T>,
}

impl<T> Renderer<T> {
    pub(crate) fn new() -> Self {
        Self {
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

    const ICON_FONT: Font = font::ICONS;
    const CHECKMARK_ICON: char = font::CHECKMARK_ICON;
    const ARROW_DOWN_ICON: char = font::ARROW_DOWN_ICON;

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
        let buffer_width = i32::max_value(); // TODO: allow wrapping
        let layout = buffer_line.layout(&FONT_SYSTEM, size as i32, buffer_width);

        let mut width = 0.0;
        let mut height = 0.0;
        for layout_line in layout.iter() {
            for glyph in layout_line.glyphs.iter() {
                let max_x = if glyph.rtl {
                    glyph.x - glyph.w
                } else {
                    glyph.x + glyph.w
                };
                if max_x > width {
                    width = max_x;
                }
            }

            height += size as f32;
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
