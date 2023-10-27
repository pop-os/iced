use crate::core::image as raster;
use crate::core::{Rectangle, Size};
use crate::graphics;

use graphics::image::image_rs::{ImageBuffer, Rgba, RgbaImage};
use rustc_hash::{FxHashMap, FxHashSet};
use std::cell::RefCell;
use std::collections::hash_map;

pub struct Pipeline {
    cache: RefCell<Cache>,
}

impl Pipeline {
    pub fn new() -> Self {
        Self {
            cache: RefCell::new(Cache::default()),
        }
    }

    pub fn dimensions(
        &self,
        handle: &raster::Handle,
        border_radius: [f32; 4],
    ) -> Size<u32> {
        if let Some(image) =
            self.cache.borrow_mut().allocate(handle, border_radius)
        {
            Size::new(image.width(), image.height())
        } else {
            Size::new(0, 0)
        }
    }

    pub fn draw(
        &mut self,
        handle: &raster::Handle,
        bounds: Rectangle,
        pixels: &mut tiny_skia::PixmapMut<'_>,
        transform: tiny_skia::Transform,
        clip_mask: Option<&tiny_skia::Mask>,
        border_radius: [f32; 4],
    ) {
        if let Some(image) =
            self.cache.borrow_mut().allocate(handle, border_radius)
        {
            let width_scale = bounds.width / image.width() as f32;
            let height_scale = bounds.height / image.height() as f32;

            let transform = transform.pre_scale(width_scale, height_scale);

            pixels.draw_pixmap(
                (bounds.x / width_scale) as i32,
                (bounds.y / height_scale) as i32,
                image,
                &tiny_skia::PixmapPaint {
                    quality: tiny_skia::FilterQuality::Bilinear,
                    ..Default::default()
                },
                transform,
                clip_mask,
            );
        }
    }

    pub fn trim_cache(&mut self) {
        self.cache.borrow_mut().trim();
    }
}

#[derive(Default)]
struct Cache {
    entries: FxHashMap<u64, Option<Entry>>,
    hits: FxHashSet<u64>,
}

impl Cache {
    pub fn allocate(
        &mut self,
        handle: &raster::Handle,
        border_radius: [f32; 4],
    ) -> Option<tiny_skia::PixmapRef<'_>> {
        let id = handle.id();

        if let hash_map::Entry::Vacant(entry) = self.entries.entry(id) {
            let mut image = graphics::image::load(handle).ok()?.into_rgba8();

            // Round the borders if a border radius is defined
            if border_radius.iter().any(|&corner| corner != 0.0) {
                round(&mut image, {
                    let [a, b, c, d] = border_radius;
                    [a as u32, b as u32, c as u32, d as u32]
                });
            }

            let mut buffer =
                vec![0u32; image.width() as usize * image.height() as usize];

            for (i, pixel) in image.pixels().enumerate() {
                let [r, g, b, a] = pixel.0;

                buffer[i] = bytemuck::cast(
                    tiny_skia::ColorU8::from_rgba(b, g, r, a).premultiply(),
                );
            }

            entry.insert(Some(Entry {
                width: image.width(),
                height: image.height(),
                pixels: buffer,
            }));
        }

        self.hits.insert(id);
        self.entries.get(&id).unwrap().as_ref().map(|entry| {
            tiny_skia::PixmapRef::from_bytes(
                bytemuck::cast_slice(&entry.pixels),
                entry.width,
                entry.height,
            )
            .expect("Build pixmap from image bytes")
        })
    }

    fn trim(&mut self) {
        self.entries.retain(|key, _| self.hits.contains(key));
        self.hits.clear();
    }
}

struct Entry {
    width: u32,
    height: u32,
    pixels: Vec<u32>,
}

// https://users.rust-lang.org/t/how-to-trim-image-to-circle-image-without-jaggy/70374/2
fn round(img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, radius: [u32; 4]) {
    let (width, height) = img.dimensions();
    assert!(radius[0] + radius[1] <= width);
    assert!(radius[3] + radius[2] <= width);
    assert!(radius[0] + radius[3] <= height);
    assert!(radius[1] + radius[2] <= height);

    // top left
    border_radius(img, radius[0], |x, y| (x - 1, y - 1));
    // top right
    border_radius(img, radius[1], |x, y| (width - x, y - 1));
    // bottom right
    border_radius(img, radius[2], |x, y| (width - x, height - y));
    // bottom left
    border_radius(img, radius[3], |x, y| (x - 1, height - y));
}

fn border_radius(
    img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    r: u32,
    coordinates: impl Fn(u32, u32) -> (u32, u32),
) {
    if r == 0 {
        return;
    }
    let r0 = r;

    // 16x antialiasing: 16x16 grid creates 256 possible shades, great for u8!
    let r = 16 * r;

    let mut x = 0;
    let mut y = r - 1;
    let mut p: i32 = 2 - r as i32;

    // ...

    let mut alpha: u16 = 0;
    let mut skip_draw = true;

    let draw = |img: &mut RgbaImage, alpha, x, y| {
        debug_assert!((1..=256).contains(&alpha));
        let pixel_alpha = &mut img[coordinates(r0 - x, r0 - y)].0[3];
        *pixel_alpha = ((alpha * *pixel_alpha as u16 + 128) / 256) as u8;
    };

    'l: loop {
        // (comments for bottom_right case:)
        // remove contents below current position
        {
            let i = x / 16;
            for j in y / 16 + 1..r0 {
                img[coordinates(r0 - i, r0 - j)].0[3] = 0;
            }
        }
        // remove contents right of current position mirrored
        {
            let j = x / 16;
            for i in y / 16 + 1..r0 {
                img[coordinates(r0 - i, r0 - j)].0[3] = 0;
            }
        }

        // draw when moving to next pixel in x-direction
        if !skip_draw {
            draw(img, alpha, x / 16 - 1, y / 16);
            draw(img, alpha, y / 16, x / 16 - 1);
            alpha = 0;
        }

        for _ in 0..16 {
            skip_draw = false;

            if x >= y {
                break 'l;
            }

            alpha += y as u16 % 16 + 1;
            if p < 0 {
                x += 1;
                p += (2 * x + 2) as i32;
            } else {
                // draw when moving to next pixel in y-direction
                if y % 16 == 0 {
                    draw(img, alpha, x / 16, y / 16);
                    draw(img, alpha, y / 16, x / 16);
                    skip_draw = true;
                    alpha = (x + 1) as u16 % 16 * 16;
                }

                x += 1;
                p -= (2 * (y - x) + 2) as i32;
                y -= 1;
            }
        }
    }

    // one corner pixel left
    if x / 16 == y / 16 {
        // column under current position possibly not yet accounted
        if x == y {
            alpha += y as u16 % 16 + 1;
        }
        let s = y as u16 % 16 + 1;
        let alpha = 2 * alpha - s * s;
        draw(img, alpha, x / 16, y / 16);
    }

    // remove remaining square of content in the corner
    let range = y / 16 + 1..r0;
    for i in range.clone() {
        for j in range.clone() {
            img[coordinates(r0 - i, r0 - j)].0[3] = 0;
        }
    }
}
