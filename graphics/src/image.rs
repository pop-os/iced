//! Load and operate on images.
#[cfg(feature = "image")]
use crate::core::Bytes;

use crate::core::Color;
use crate::core::Radians;
use crate::core::Rectangle;
#[cfg(feature = "image")]
use crate::core::Size;
use crate::core::image;
use crate::core::svg;

/// A raster or vector image.
#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq)]
pub enum Image {
    /// A raster image.
    Raster {
        image: image::Image,
        bounds: Rectangle,
        clip_bounds: Rectangle,
    },

    /// A vector image.
    Vector {
        svg: svg::Svg,
        bounds: Rectangle,
        clip_bounds: Rectangle,
    },
}

impl Image {
    /// Returns the bounds of the [`Image`].
    pub fn bounds(&self) -> Rectangle {
        match self {
            Image::Raster { image, bounds, .. } => {
                bounds.rotate(image.rotation)
            }
            Image::Vector { svg, bounds, .. } => bounds.rotate(svg.rotation),
        }
    }
}

/// An image buffer.
#[cfg(feature = "image")]
pub type Buffer = ::image::ImageBuffer<::image::Rgba<u8>, Bytes>;

/// Chooses a stable, aspect-preserving prefilter size for a physical draw size.
///
/// Each level is a power-of-two reduction of the source and is never smaller
/// than the requested target in either dimension. This keeps resize animation
/// from producing a new cached raster for every physical pixel.
#[cfg(feature = "image")]
pub fn downsample_size(image: &Buffer, target: Size<u32>) -> Size<u32> {
    let target_width = target.width.max(1);
    let target_height = target.height.max(1);
    let width_ratio = image.width() / target_width;
    let height_ratio = image.height() / target_height;
    let ratio = width_ratio.min(height_ratio);

    if ratio < 2 {
        return Size::new(image.width(), image.height());
    }

    let factor = 1 << (31 - ratio.leading_zeros());

    Size::new(
        image.width().div_ceil(factor),
        image.height().div_ceil(factor),
    )
}

/// Builds a premultiplied-alpha Lanczos level for a physical draw size.
#[cfg(feature = "image")]
pub fn downsample(image: &Buffer, target: Size<u32>) -> Option<Buffer> {
    let size = downsample_size(image, target);

    if size == Size::new(image.width(), image.height()) {
        return None;
    }

    let mut premultiplied = image.clone().into_raw().to_vec();

    for pixel in premultiplied.chunks_exact_mut(4) {
        let alpha = u16::from(pixel[3]);

        for channel in &mut pixel[..3] {
            *channel = ((u16::from(*channel) * alpha + 127) / 255) as u8;
        }
    }

    let premultiplied =
        ::image::ImageBuffer::<::image::Rgba<u8>, Vec<u8>>::from_raw(
            image.width(),
            image.height(),
            premultiplied,
        )?;
    let resized = ::image::imageops::resize(
        &premultiplied,
        size.width,
        size.height,
        ::image::imageops::FilterType::Lanczos3,
    );
    let mut pixels = resized.into_raw();

    for pixel in pixels.chunks_exact_mut(4) {
        let alpha = u32::from(pixel[3]);

        if alpha == 0 {
            pixel[..3].fill(0);
            continue;
        }

        for channel in &mut pixel[..3] {
            *channel = ((u32::from(*channel) * 255 + alpha / 2) / alpha)
                .min(255) as u8;
        }
    }

    ::image::ImageBuffer::from_raw(size.width, size.height, Bytes::from(pixels))
}

#[cfg(feature = "image")]
/// Tries to load an image by its [`Handle`].
///
/// [`Handle`]: image::Handle
pub fn load(handle: &image::Handle) -> Result<Buffer, image::Error> {
    use bitflags::bitflags;

    bitflags! {
        struct Operation: u8 {
            const FLIP_HORIZONTALLY = 0b1;
            const ROTATE_180 = 0b10;
            const FLIP_VERTICALLY= 0b100;
            const ROTATE_90 = 0b1000;
            const ROTATE_270 = 0b10000;
        }
    }

    impl Operation {
        // Meaning of the returned value is described e.g. at:
        // https://magnushoff.com/articles/jpeg-orientation/
        fn from_exif<R>(reader: &mut R) -> Result<Self, exif::Error>
        where
            R: std::io::BufRead + std::io::Seek,
        {
            let exif = exif::Reader::new().read_from_container(reader)?;

            Ok(exif
                .get_field(exif::Tag::Orientation, exif::In::PRIMARY)
                .and_then(|field| field.value.get_uint(0))
                .and_then(|value| u8::try_from(value).ok())
                .map(|value| match value {
                    1 => Operation::empty(),
                    2 => Operation::FLIP_HORIZONTALLY,
                    3 => Operation::ROTATE_180,
                    4 => Operation::FLIP_VERTICALLY,
                    5 => Operation::ROTATE_90 | Operation::FLIP_HORIZONTALLY,
                    6 => Operation::ROTATE_90,
                    7 => Operation::ROTATE_90 | Operation::FLIP_VERTICALLY,
                    8 => Operation::ROTATE_270,
                    _ => Operation::empty(),
                })
                .unwrap_or_else(Self::empty))
        }

        fn perform(
            self,
            mut image: ::image::DynamicImage,
        ) -> ::image::DynamicImage {
            use ::image::imageops;

            if self.contains(Operation::ROTATE_90) {
                image = imageops::rotate90(&image).into();
            }

            if self.contains(Self::ROTATE_180) {
                imageops::rotate180_in_place(&mut image);
            }

            if self.contains(Operation::ROTATE_270) {
                image = imageops::rotate270(&image).into();
            }

            if self.contains(Self::FLIP_VERTICALLY) {
                imageops::flip_vertical_in_place(&mut image);
            }

            if self.contains(Self::FLIP_HORIZONTALLY) {
                imageops::flip_horizontal_in_place(&mut image);
            }

            image
        }
    }

    let (width, height, pixels) = match handle {
        image::Handle::Path(_, path) => {
            use std::sync::Arc;

            let image = ::image::ImageReader::open(&path)
                .map_err(|e| image::Error::Inaccessible(Arc::new(e)))?
                .with_guessed_format()
                .map_err(|e| image::Error::Invalid(Arc::new(e)))?
                .decode()
                .map_err(|e| image::Error::Invalid(Arc::new(e)))?;

            let operation = std::fs::File::open(path)
                .ok()
                .map(std::io::BufReader::new)
                .and_then(|mut reader| Operation::from_exif(&mut reader).ok())
                .unwrap_or_else(Operation::empty);

            let rgba = operation.perform(image).into_rgba8();

            (rgba.width(), rgba.height(), Bytes::from(rgba.into_raw()))
        }
        image::Handle::Bytes(_, bytes) => {
            let image = ::image::load_from_memory(bytes).map_err(to_error)?;

            let operation =
                Operation::from_exif(&mut std::io::Cursor::new(bytes))
                    .ok()
                    .unwrap_or_else(Operation::empty);

            let rgba = operation.perform(image).into_rgba8();

            (rgba.width(), rgba.height(), Bytes::from(rgba.into_raw()))
        }
        image::Handle::Rgba {
            width,
            height,
            pixels,
            ..
        } => (*width, *height, pixels.clone()),
    };

    if let Some(image) = ::image::ImageBuffer::from_raw(width, height, pixels) {
        Ok(image)
    } else {
        Err(to_error(::image::error::ImageError::Limits(
            ::image::error::LimitError::from_kind(
                ::image::error::LimitErrorKind::DimensionError,
            ),
        )))
    }
}

#[cfg(feature = "image")]
fn to_error(error: ::image::ImageError) -> image::Error {
    use std::sync::Arc;

    match error {
        ::image::ImageError::IoError(error) => {
            image::Error::Inaccessible(Arc::new(error))
        }
        error => image::Error::Invalid(Arc::new(error)),
    }
}

#[cfg(all(test, feature = "image"))]
mod tests {
    use super::*;

    fn solid(width: u32, height: u32) -> Buffer {
        Buffer::from_raw(
            width,
            height,
            Bytes::from(vec![255; width as usize * height as usize * 4]),
        )
        .expect("valid solid image dimensions")
    }

    #[test]
    fn downsample_levels_are_stable_and_never_smaller_than_the_target() {
        let image = solid(512, 384);

        assert_eq!(
            downsample_size(&image, Size::new(48, 36)),
            Size::new(64, 48)
        );
        assert_eq!(
            downsample_size(&image, Size::new(63, 40)),
            Size::new(64, 48)
        );
        assert_eq!(
            downsample_size(&image, Size::new(300, 200)),
            Size::new(512, 384)
        );
    }

    #[test]
    fn premultiplied_downsample_avoids_transparent_color_bleed() {
        let mut pixels = vec![0; 4 * 4 * 4];

        for y in 0..4 {
            for x in 0..2 {
                let offset = (y * 4 + x) * 4;
                pixels[offset..offset + 4].copy_from_slice(&[255, 0, 0, 255]);
            }

            for x in 2..4 {
                let offset = (y * 4 + x) * 4;
                pixels[offset..offset + 4].copy_from_slice(&[0, 0, 255, 0]);
            }
        }

        let image = Buffer::from_raw(4, 4, Bytes::from(pixels))
            .expect("valid test image dimensions");
        let resized = downsample(&image, Size::new(1, 1))
            .expect("the image should be downsampled");
        let pixel = resized.get_pixel(0, 0).0;

        assert!(pixel[0] > 200);
        assert_eq!(pixel[1], 0);
        assert_eq!(pixel[2], 0);
    }
}
