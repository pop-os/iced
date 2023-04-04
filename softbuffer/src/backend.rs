use cosmic_text::{
    Attrs, AttrsList, BufferLine, FontSystem, Metrics, SwashCache, Weight, Wrap,
};
#[cfg(feature = "image")]
use iced_graphics::image::raster;
use iced_graphics::image::storage;
#[cfg(feature = "svg")]
use iced_graphics::image::vector;
#[cfg(feature = "image")]
use iced_native::image;
#[cfg(feature = "svg")]
use iced_native::svg;
use iced_native::text;
use iced_native::{Font, Point, Size};
use std::cell::RefCell;
use std::fmt;
use std::sync::Mutex;

lazy_static::lazy_static! {
    pub(crate) static ref FONT_SYSTEM: Mutex<FontSystem> = Mutex::new(FontSystem::new());
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
                    chunk[3], chunk[0], chunk[1], chunk[2],
                )
                .to_u32(),
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

pub struct Backend {
    pub(crate) swash_cache: SwashCache,
    #[cfg(feature = "image")]
    pub(crate) raster_cache: RefCell<raster::Cache<CpuStorage>>,
    #[cfg(feature = "svg")]
    pub(crate) vector_cache: RefCell<vector::Cache<CpuStorage>>,
}

impl Backend {
    pub fn new() -> Self {
        Self {
            swash_cache: SwashCache::new(),
            #[cfg(feature = "image")]
            raster_cache: RefCell::new(raster::Cache::default()),
            #[cfg(feature = "svg")]
            vector_cache: RefCell::new(vector::Cache::default()),
        }
    }

    pub(crate) fn cosmic_metrics_attrs(
        &self,
        size: f32,
        font: &Font,
    ) -> (Metrics, Attrs) {
        //TODO: why is this conversion necessary?
        let font_size = (size * 5.0 / 6.0) as i32;

        //TODO: how to properly calculate line height?
        let line_height = size as i32;

        let attrs = match font {
            Font::Default => Attrs::new().weight(Weight::NORMAL),
            //TODO: support using the bytes field. Right now this is just a hack for libcosmic
            Font::External { name, bytes } => match *name {
                "Fira Sans Regular" => Attrs::new().weight(Weight::NORMAL),
                "Fira Sans Light" => Attrs::new().weight(Weight::LIGHT),
                "Fira Sans SemiBold" => Attrs::new().weight(Weight::SEMIBOLD),
                _ => {
                    log::warn!("Unsupported font name {:?}", name);
                    Attrs::new()
                }
            },
        };

        (Metrics::new(font_size as f32, line_height as f32), attrs)
    }

    #[cfg(any(feature = "image", feature = "svg"))]
    pub(crate) fn trim_cache(&self) {
        #[cfg(feature = "image")]
        self.raster_cache.borrow_mut().trim(&mut CpuStorage, &mut ());

        #[cfg(feature = "svg")]
        self.vector_cache.borrow_mut().trim(&mut CpuStorage, &mut ());
    }
}

impl iced_graphics::backend::Backend for Backend {
    fn trim_measurements(&mut self) {
        // no-op
    }
}

impl iced_graphics::backend::Text for Backend {
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
        size: f32,
        font: Font,
        bounds: Size,
    ) -> (f32, f32) {
        let (metrics, attrs) = self.cosmic_metrics_attrs(size, &font);

        //TODO: improve implementation
        let mut buffer_line = BufferLine::new(content, AttrsList::new(attrs));
        let layout = buffer_line.layout(
            &mut FONT_SYSTEM.lock().unwrap(),
            metrics.font_size,
            bounds.width,
            Wrap::Word,
        );

        let mut width = 0.0;
        let mut height = 0.0;
        for line in content.lines() {
            let mut buffer_line = BufferLine::new(line, AttrsList::new(attrs));
            let layout = buffer_line.layout(
                &mut FONT_SYSTEM.lock().unwrap(),
                metrics.font_size,
                bounds.width,
                Wrap::Word,
            );

            for layout_line in layout.iter() {
                for glyph in layout_line.glyphs.iter() {
                    let max_x = if glyph.level.is_rtl() {
                        glyph.x - glyph.w
                    } else {
                        glyph.x + glyph.w
                    };
                    if max_x + 1.0 > width {
                        width = max_x + 1.0;
                    }
                }

                height += metrics.line_height as f32;
            }
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
        let (metrics, attrs) = self.cosmic_metrics_attrs(size, &font);

        //TODO: improve implementation
        let mut buffer_line = BufferLine::new(content, AttrsList::new(attrs));
        let layout = buffer_line.layout(
            &mut FONT_SYSTEM.lock().unwrap(),
            metrics.font_size,
            bounds.width,
            Wrap::Word,
        );

        // Find exact hit
        if !nearest_only {
            let mut line_y = 0.0;
            for layout_line in layout.iter() {
                if point.y > line_y
                    && point.y < line_y + metrics.line_height as f32
                {
                    for glyph in layout_line.glyphs.iter() {
                        let (min_x, max_x) = if glyph.level.is_rtl() {
                            (glyph.x - glyph.w, glyph.x)
                        } else {
                            (glyph.x, glyph.x + glyph.w)
                        };

                        if point.x > min_x && point.x < max_x {
                            return Some(text::Hit::CharOffset(glyph.start));
                        }
                    }
                }

                line_y += metrics.line_height as f32;
            }
        }

        // Find nearest
        let mut nearest_opt = None;
        let mut line_y = 0.0;
        for layout_line in layout.iter() {
            let center_y = line_y + metrics.line_height as f32 / 2.0;

            for glyph in layout_line.glyphs.iter() {
                let (min_x, max_x) = if glyph.level.is_rtl() {
                    (glyph.x - glyph.w, glyph.x)
                } else {
                    (glyph.x, glyph.x + glyph.w)
                };

                let center_x = (min_x + max_x) / 2.0;
                let center = Point::new(center_x, center_y);

                let distance = center.distance(point);
                let vector = point - center;
                nearest_opt = match nearest_opt {
                    Some((
                        nearest_offset,
                        nearest_vector,
                        nearest_distance,
                    )) => {
                        if distance < nearest_distance {
                            Some((glyph.start, vector, distance))
                        } else {
                            Some((
                                nearest_offset,
                                nearest_vector,
                                nearest_distance,
                            ))
                        }
                    }
                    None => Some((glyph.start, vector, distance)),
                };
            }

            line_y += metrics.line_height as f32;
        }

        match nearest_opt {
            Some((offset, vector, _)) => {
                Some(text::Hit::NearestCharOffset(offset, vector))
            }
            None => None,
        }
    }
}

#[cfg(feature = "image")]
impl iced_graphics::backend::Image for Backend {
    fn dimensions(&self, handle: &image::Handle) -> Size<u32> {
        let mut cache = self.raster_cache.borrow_mut();
        let memory = cache.load(handle);

        memory.dimensions()
    }
}

#[cfg(feature = "svg")]
impl iced_graphics::backend::Svg for Backend {
    fn viewport_dimensions(&self, handle: &svg::Handle) -> Size<u32> {
        let mut cache = self.vector_cache.borrow_mut();
        let svg = cache.load(handle);

        svg.viewport_dimensions()
    }
}
