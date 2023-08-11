use crate::core::alignment;
use crate::core::font::{self, Font};
use crate::core::text::{Hit, LineHeight, Shaping};
use crate::core::{Pixels, Point, Rectangle, Size};
use crate::graphics::color;
use crate::layer::Text;

use rustc_hash::{FxHashMap, FxHashSet};
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::hash_map;
use std::hash::{BuildHasher, Hash, Hasher};
use std::sync::Arc;

#[allow(missing_debug_implementations)]
pub struct Pipeline {
    font_system: RefCell<glyphon::FontSystem>,
    renderers: Vec<glyphon::TextRenderer>,
    atlas: glyphon::TextAtlas,
    prepare_layer: usize,
    cache: RefCell<Cache>,
}

impl Pipeline {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
    ) -> Self {
        Pipeline {
            font_system: RefCell::new(glyphon::FontSystem::new_with_fonts(
                [glyphon::fontdb::Source::Binary(Arc::new(
                    include_bytes!("fonts/Iced-Icons.ttf").as_slice(),
                ))]
                .into_iter(),
            )),
            renderers: Vec::new(),
            atlas: glyphon::TextAtlas::new(
                device,
                queue,
                format,
                if color::GAMMA_CORRECTION {
                    glyphon::ColorMode::Accurate
                } else {
                    glyphon::ColorMode::Web
                },
            ),
            prepare_layer: 0,
            cache: RefCell::new(Cache::new()),
        }
    }

    pub fn load_font(&mut self, bytes: Cow<'static, [u8]>) {
        let _ = self.font_system.get_mut().db_mut().load_font_source(
            glyphon::fontdb::Source::Binary(Arc::new(bytes.into_owned())),
        );
    }

    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        sections: &[Text<'_>],
        bounds: Rectangle,
        scale_factor: f32,
        target_size: Size<u32>,
    ) -> bool {
        if self.renderers.len() <= self.prepare_layer {
            self.renderers.push(glyphon::TextRenderer::new(
                &mut self.atlas,
                device,
                Default::default(),
                None,
            ));
        }

        let font_system = self.font_system.get_mut();
        let renderer = &mut self.renderers[self.prepare_layer];
        let cache = self.cache.get_mut();

        let keys: Vec<_> = sections
            .iter()
            .map(|section| {
                let (key, _) = cache.allocate(
                    font_system,
                    Key {
                        content: section.content,
                        size: section.size,
                        line_height: f32::from(
                            section
                                .line_height
                                .to_absolute(Pixels(section.size)),
                        ),
                        font: section.font,
                        bounds: Size {
                            width: section.bounds.width,
                            height: section.bounds.height,
                        },
                        shaping: section.shaping,
                    },
                );

                key
            })
            .collect();

        let bounds = bounds * scale_factor;

        let text_areas =
            sections
                .iter()
                .zip(keys.iter())
                .filter_map(|(section, key)| {
                    let buffer = cache.get(key).expect("Get cached buffer");

                    let x = section.bounds.x * scale_factor;
                    let y = section.bounds.y * scale_factor;

                    let (max_width, total_height) = measure(buffer);

                    let max_width = max_width * scale_factor;
                    let total_height = total_height * scale_factor;

                    let left = match section.horizontal_alignment {
                        alignment::Horizontal::Left => x,
                        alignment::Horizontal::Center => x - max_width / 2.0,
                        alignment::Horizontal::Right => x - max_width,
                    };

                    let top = match section.vertical_alignment {
                        alignment::Vertical::Top => y,
                        alignment::Vertical::Center => y - total_height / 2.0,
                        alignment::Vertical::Bottom => y - total_height,
                    };

                    let section_bounds = Rectangle {
                        x: left,
                        y: top,
                        width: section.bounds.width * scale_factor,
                        height: section.bounds.height * scale_factor,
                    };

                    let clip_bounds = bounds.intersection(&section_bounds)?;

                    Some(glyphon::TextArea {
                        buffer,
                        left,
                        top,
                        scale: scale_factor,
                        bounds: glyphon::TextBounds {
                            left: clip_bounds.x as i32,
                            top: clip_bounds.y as i32,
                            right: (clip_bounds.x + clip_bounds.width) as i32,
                            bottom: (clip_bounds.y + clip_bounds.height) as i32,
                        },
                        default_color: {
                            let [r, g, b, a] =
                                color::pack(section.color).components();

                            glyphon::Color::rgba(
                                (r * 255.0) as u8,
                                (g * 255.0) as u8,
                                (b * 255.0) as u8,
                                (a * 255.0) as u8,
                            )
                        },
                    })
                });

        let result = renderer.prepare(
            device,
            queue,
            font_system,
            &mut self.atlas,
            glyphon::Resolution {
                width: target_size.width,
                height: target_size.height,
            },
            text_areas,
            &mut glyphon::SwashCache::new(),
        );

        match result {
            Ok(()) => {
                self.prepare_layer += 1;

                true
            }
            Err(glyphon::PrepareError::AtlasFull(content_type)) => {
                self.prepare_layer = 0;

                #[allow(clippy::needless_bool)]
                if self.atlas.grow(device, content_type) {
                    false
                } else {
                    // If the atlas cannot grow, then all bets are off.
                    // Instead of panicking, we will just pray that the result
                    // will be somewhat readable...
                    true
                }
            }
        }
    }

    pub fn render<'a>(
        &'a self,
        layer: usize,
        bounds: Rectangle<u32>,
        render_pass: &mut wgpu::RenderPass<'a>,
    ) {
        let renderer = &self.renderers[layer];

        render_pass.set_scissor_rect(
            bounds.x,
            bounds.y,
            bounds.width,
            bounds.height,
        );

        renderer
            .render(&self.atlas, render_pass)
            .expect("Render text");
    }

    pub fn end_frame(&mut self) {
        self.atlas.trim();
        self.cache.get_mut().trim();

        self.prepare_layer = 0;
    }

    pub fn measure(
        &self,
        content: &str,
        size: f32,
        line_height: LineHeight,
        font: Font,
        bounds: Size,
        shaping: Shaping,
    ) -> (f32, f32) {
        let mut measurement_cache = self.cache.borrow_mut();

        let line_height = f32::from(line_height.to_absolute(Pixels(size)));

        let (_, paragraph) = measurement_cache.allocate(
            &mut self.font_system.borrow_mut(),
            Key {
                content,
                size,
                line_height,
                font,
                bounds,
                shaping,
            },
        );

        measure(paragraph)
    }

    pub fn hit_test(
        &self,
        content: &str,
        size: f32,
        line_height: LineHeight,
        font: Font,
        bounds: Size,
        shaping: Shaping,
        point: Point,
        _nearest_only: bool,
    ) -> Option<Hit> {
        let mut measurement_cache = self.cache.borrow_mut();

        let line_height = f32::from(line_height.to_absolute(Pixels(size)));

        let (_, paragraph) = measurement_cache.allocate(
            &mut self.font_system.borrow_mut(),
            Key {
                content,
                size,
                line_height,
                font,
                bounds,
                shaping,
            },
        );

        let cursor = paragraph.hit(point.x, point.y)?;

        Some(Hit::CharOffset(cursor.index))
    }
}

fn measure(buffer: &glyphon::Buffer) -> (f32, f32) {
    let (width, total_lines) = buffer
        .layout_runs()
        .fold((0.0, 0usize), |(width, total_lines), run| {
            (run.line_w.max(width), total_lines + 1)
        });

    (width, total_lines as f32 * buffer.metrics().line_height)
}

fn to_family(family: font::Family) -> glyphon::Family<'static> {
    match family {
        font::Family::Name(name) => glyphon::Family::Name(name),
        font::Family::SansSerif => glyphon::Family::SansSerif,
        font::Family::Serif => glyphon::Family::Serif,
        font::Family::Cursive => glyphon::Family::Cursive,
        font::Family::Fantasy => glyphon::Family::Fantasy,
        font::Family::Monospace => glyphon::Family::Monospace,
    }
}

fn to_weight(weight: font::Weight) -> glyphon::Weight {
    match weight {
        font::Weight::Thin => glyphon::Weight::THIN,
        font::Weight::ExtraLight => glyphon::Weight::EXTRA_LIGHT,
        font::Weight::Light => glyphon::Weight::LIGHT,
        font::Weight::Normal => glyphon::Weight::NORMAL,
        font::Weight::Medium => glyphon::Weight::MEDIUM,
        font::Weight::Semibold => glyphon::Weight::SEMIBOLD,
        font::Weight::Bold => glyphon::Weight::BOLD,
        font::Weight::ExtraBold => glyphon::Weight::EXTRA_BOLD,
        font::Weight::Black => glyphon::Weight::BLACK,
    }
}

fn to_stretch(stretch: font::Stretch) -> glyphon::Stretch {
    match stretch {
        font::Stretch::UltraCondensed => glyphon::Stretch::UltraCondensed,
        font::Stretch::ExtraCondensed => glyphon::Stretch::ExtraCondensed,
        font::Stretch::Condensed => glyphon::Stretch::Condensed,
        font::Stretch::SemiCondensed => glyphon::Stretch::SemiCondensed,
        font::Stretch::Normal => glyphon::Stretch::Normal,
        font::Stretch::SemiExpanded => glyphon::Stretch::SemiExpanded,
        font::Stretch::Expanded => glyphon::Stretch::Expanded,
        font::Stretch::ExtraExpanded => glyphon::Stretch::ExtraExpanded,
        font::Stretch::UltraExpanded => glyphon::Stretch::UltraExpanded,
    }
}

fn to_shaping(shaping: Shaping) -> glyphon::Shaping {
    match shaping {
        Shaping::Basic => glyphon::Shaping::Basic,
        Shaping::Advanced => glyphon::Shaping::Advanced,
    }
}

struct Cache {
    entries: FxHashMap<KeyHash, glyphon::Buffer>,
    recently_used: FxHashSet<KeyHash>,
    hasher: HashBuilder,
}

#[cfg(not(target_arch = "wasm32"))]
type HashBuilder = twox_hash::RandomXxHashBuilder64;

#[cfg(target_arch = "wasm32")]
type HashBuilder = std::hash::BuildHasherDefault<twox_hash::XxHash64>;

impl Cache {
    fn new() -> Self {
        Self {
            entries: FxHashMap::default(),
            recently_used: FxHashSet::default(),
            hasher: HashBuilder::default(),
        }
    }

    fn get(&self, key: &KeyHash) -> Option<&glyphon::Buffer> {
        self.entries.get(key)
    }

    fn allocate(
        &mut self,
        font_system: &mut glyphon::FontSystem,
        key: Key<'_>,
    ) -> (KeyHash, &mut glyphon::Buffer) {
        let hash = {
            let mut hasher = self.hasher.build_hasher();

            key.content.hash(&mut hasher);
            key.size.to_bits().hash(&mut hasher);
            key.line_height.to_bits().hash(&mut hasher);
            key.font.hash(&mut hasher);
            key.bounds.width.to_bits().hash(&mut hasher);
            key.bounds.height.to_bits().hash(&mut hasher);
            key.shaping.hash(&mut hasher);

            hasher.finish()
        };

        if let hash_map::Entry::Vacant(entry) = self.entries.entry(hash) {
            let metrics = glyphon::Metrics::new(key.size, key.line_height);
            let mut buffer = glyphon::Buffer::new(font_system, metrics);

            buffer.set_size(
                font_system,
                key.bounds.width,
                key.bounds.height.max(key.line_height),
            );
            buffer.set_text(
                font_system,
                key.content,
                glyphon::Attrs::new()
                    .family(to_family(key.font.family))
                    .weight(to_weight(key.font.weight))
                    .stretch(to_stretch(key.font.stretch)),
                to_shaping(key.shaping),
            );

            let _ = entry.insert(buffer);
        }

        let _ = self.recently_used.insert(hash);

        (hash, self.entries.get_mut(&hash).unwrap())
    }

    fn trim(&mut self) {
        self.entries
            .retain(|key, _| self.recently_used.contains(key));

        self.recently_used.clear();
    }
}

#[derive(Debug, Clone, Copy)]
struct Key<'a> {
    content: &'a str,
    size: f32,
    line_height: f32,
    font: Font,
    bounds: Size,
    shaping: Shaping,
}

type KeyHash = u64;
