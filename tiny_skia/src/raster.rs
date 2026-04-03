use crate::core::image as raster;
use crate::core::{Rectangle, Size};
use crate::graphics;
use crate::graphics::image::image_rs;

use rustc_hash::{FxHashMap, FxHashSet};
use std::cell::RefCell;
use std::collections::hash_map;
use std::ops::Deref;

type SourceImage = image_rs::ImageBuffer<image_rs::Rgba<u8>, raster::Bytes>;

#[derive(Debug)]
pub struct Pipeline {
    cache: RefCell<Cache>,
}

impl Pipeline {
    pub fn new() -> Self {
        Self {
            cache: RefCell::new(Cache::default()),
        }
    }

    pub fn load(
        &self,
        handle: &raster::Handle,
    ) -> Result<raster::Allocation, raster::Error> {
        let mut cache = self.cache.borrow_mut();
        let image = cache.source(handle).ok_or(raster::Error::Empty)?;

        #[allow(unsafe_code)]
        Ok(unsafe {
            raster::allocate(handle, Size::new(image.width(), image.height()))
        })
    }

    pub fn dimensions(&self, handle: &raster::Handle) -> Option<Size<u32>> {
        let mut cache = self.cache.borrow_mut();
        let image = cache.source(handle)?;

        Some(Size::new(image.width(), image.height()))
    }

    pub fn draw(
        &mut self,
        handle: &raster::Handle,
        filter_method: raster::FilterMethod,
        bounds: Rectangle,
        opacity: f32,
        pixels: &mut tiny_skia::PixmapMut<'_>,
        transform: tiny_skia::Transform,
        clip_mask: Option<&tiny_skia::Mask>,
        border_radius: [f32; 4],
    ) {
        let mut cache = self.cache.borrow_mut();
        let target_size = Size::new(
            bounds.width.ceil().max(1.0) as u32,
            bounds.height.ceil().max(1.0) as u32,
        );

        let Ok(mut image) = cache.allocate(handle, target_size) else {
            return;
        };

        let width_scale = bounds.width / image.width() as f32;
        let height_scale = bounds.height / image.height() as f32;
        let quality = match filter_method {
            raster::FilterMethod::Linear => tiny_skia::FilterQuality::Bilinear,
            raster::FilterMethod::Nearest => tiny_skia::FilterQuality::Nearest,
        };
        let mut scratch;

        // Round the borders if a border radius is defined
        if border_radius.iter().any(|&corner| corner != 0.0) {
            scratch = image.to_owned();
            round(&mut scratch.as_mut(), {
                let [a, b, c, d] = border_radius;
                let scale_by = width_scale.min(height_scale);
                let max_radius = image.width().min(image.height()) / 2;
                [
                    ((a / scale_by) as u32).max(1).min(max_radius),
                    ((b / scale_by) as u32).max(1).min(max_radius),
                    ((c / scale_by) as u32).max(1).min(max_radius),
                    ((d / scale_by) as u32).max(1).min(max_radius),
                ]
            });
            image = scratch.as_ref();
        }

        let transform = transform.pre_scale(width_scale, height_scale);

        let quality = match filter_method {
            raster::FilterMethod::Linear => tiny_skia::FilterQuality::Bilinear,
            raster::FilterMethod::Nearest => tiny_skia::FilterQuality::Nearest,
        };

        pixels.draw_pixmap(
            (bounds.x / width_scale) as i32,
            (bounds.y / height_scale) as i32,
            image,
            &tiny_skia::PixmapPaint {
                quality,
                opacity,
                ..Default::default()
            },
            transform,
            clip_mask,
        );
    }

    pub fn trim_cache(&mut self) {
        self.cache.borrow_mut().trim();
    }
}

#[derive(Debug, Default)]
struct Cache {
    sources: FxHashMap<raster::Id, Option<SourceImage>>,
    entries: FxHashMap<UploadKey, Option<Entry>>,
    source_hits: FxHashSet<raster::Id>,
    entry_hits: FxHashSet<UploadKey>,
}

impl Cache {
    pub fn source(&mut self, handle: &raster::Handle) -> Option<&SourceImage> {
        let id = handle.id();

        if let hash_map::Entry::Vacant(entry) = self.sources.entry(id) {
            let _ = entry.insert(graphics::image::load(handle).ok());
        }

        let _ = self.source_hits.insert(id);
        self.sources.get(&id).and_then(Option::as_ref)
    }

    pub fn allocate(
        &mut self,
        handle: &raster::Handle,
        target_size: Size<u32>,
    ) -> Result<tiny_skia::PixmapRef<'_>, raster::Error> {
        let key = UploadKey::new(handle.id(), target_size);
        let _ = self.source_hits.insert(handle.id());
        let _ = self.entry_hits.insert(key);

        if !self.entries.contains_key(&key) {
            let image = self.source(handle).ok_or(raster::Error::Empty)?;
            let upload_size = upload_size(image, target_size);
            let resized = if upload_size.width == image.width()
                && upload_size.height == image.height()
            {
                None
            } else {
                Some(image_rs::imageops::resize(
                    image,
                    upload_size.width,
                    upload_size.height,
                    image_rs::imageops::FilterType::Lanczos3,
                ))
            };

            let uploaded = if let Some(resized) = resized.as_ref() {
                entry_from_image(resized)
            } else {
                entry_from_image(image)
            };

            let _ = self.entries.insert(key, uploaded);
        }

        let Some(ret) = self.entries.get(&key).unwrap().as_ref().map(|entry| {
            tiny_skia::PixmapRef::from_bytes(
                bytemuck::cast_slice(&entry.pixels),
                entry.width,
                entry.height,
            )
            .expect("Build pixmap from image bytes")
        }) else {
            return Err(raster::Error::Empty);
        };

        Ok(ret)
    }

    fn trim(&mut self) {
        self.sources.retain(|key, _| self.source_hits.contains(key));
        self.entries.retain(|key, _| self.entry_hits.contains(key));
        self.source_hits.clear();
        self.entry_hits.clear();
    }
}

#[derive(Debug)]
struct Entry {
    width: u32,
    height: u32,
    pixels: Vec<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct UploadKey {
    image_id: raster::Id,
    width: u32,
    height: u32,
}

impl UploadKey {
    fn new(image_id: raster::Id, target_size: Size<u32>) -> Self {
        Self {
            image_id,
            width: target_size.width.max(1),
            height: target_size.height.max(1),
        }
    }
}

fn upload_size(image: &SourceImage, target_size: Size<u32>) -> Size<u32> {
    Size::new(
        target_size.width.max(1).min(image.width()),
        target_size.height.max(1).min(image.height()),
    )
}

fn entry_from_image<Pixels>(
    image: &image_rs::ImageBuffer<image_rs::Rgba<u8>, Pixels>,
) -> Option<Entry>
where
    Pixels: Deref<Target = [u8]>,
{
    if image.width() == 0 || image.height() == 0 {
        return None;
    }

    let mut pixels =
        vec![0u32; image.width() as usize * image.height() as usize];

    for (i, pixel) in image.pixels().enumerate() {
        let [r, g, b, a] = pixel.0;

        pixels[i] = bytemuck::cast(
            tiny_skia::ColorU8::from_rgba(b, g, r, a).premultiply(),
        );
    }

    Some(Entry {
        width: image.width(),
        height: image.height(),
        pixels,
    })
}

// https://users.rust-lang.org/t/how-to-trim-image-to-circle-image-without-jaggy/70374/2
fn round(img: &mut tiny_skia::PixmapMut<'_>, radius: [u32; 4]) {
    let (width, height) = (img.width(), img.height());
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
    img: &mut tiny_skia::PixmapMut<'_>,
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

    fn pixel_id(width: u32, (x, y): (u32, u32)) -> usize {
        ((width as usize * y as usize) + x as usize) * 4
    }

    let clear_pixel = |img: &mut tiny_skia::PixmapMut<'_>,
                       (x, y): (u32, u32)| {
        let pixel = pixel_id(img.width(), (x, y));
        img.data_mut()[pixel..pixel + 4].copy_from_slice(&[0; 4]);
    };

    let draw = |img: &mut tiny_skia::PixmapMut<'_>, alpha, x, y| {
        debug_assert!((1..=256).contains(&alpha));
        let pixel = pixel_id(img.width(), coordinates(r0 - x, r0 - y));
        let pixel_alpha = &mut img.data_mut()[pixel + 3];
        *pixel_alpha = ((alpha * *pixel_alpha as u16 + 128) / 256) as u8;
    };

    'l: loop {
        // (comments for bottom_right case:)
        // remove contents below current position
        {
            let i = x / 16;
            for j in y / 16 + 1..r0 {
                clear_pixel(img, coordinates(r0 - i, r0 - j));
            }
        }
        // remove contents right of current position mirrored
        {
            let j = x / 16;
            for i in y / 16 + 1..r0 {
                clear_pixel(img, coordinates(r0 - i, r0 - j));
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
            clear_pixel(img, coordinates(r0 - i, r0 - j));
        }
    }
}
