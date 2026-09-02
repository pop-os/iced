//! Load and operate on images.
#[cfg(feature = "image")]
use crate::core::Bytes;
#[cfg(feature = "image")]
use crate::core::Size;

use crate::core::Color;
use crate::core::Radians;
use crate::core::Rectangle;
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

/// resample a raster image to this target
#[cfg(feature = "image")]
pub fn downsample_target(
    native: Size<u32>,
    bounds: Size<f32>,
) -> Option<Size<u32>> {
    if !(bounds.width >= 1.0 && bounds.height >= 1.0) {
        return None;
    }

    // Round up to 4px so an animated resize does not make a copy per pixel
    // TODO: maybe find a better approach
    let quantize = |length: f32| (length.ceil() as u32).div_ceil(4) * 4;

    let target = Size::new(
        quantize(bounds.width).min(native.width),
        quantize(bounds.height).min(native.height),
    );

    let minified = native.width as f32 >= target.width as f32 * 1.25
        || native.height as f32 >= target.height as f32 * 1.25;

    // If the target is big then Lanczos will be too expensive, and sampler is good enough
    let small = u64::from(target.width) * u64::from(target.height) <= 1 << 18;

    (minified && small).then_some(target)
}

/// Resamples premultiplied pixels down to `target`.
#[cfg(feature = "image")]
pub fn downsample_premultiplied(
    pixels: &[u8],
    size: Size<u32>,
    target: Size<u32>,
) -> Vec<u8> {
    let image =
        ::image::RgbaImage::from_raw(size.width, size.height, pixels.to_vec())
            .expect("pixels hold width * height RGBA pixels");

    resize(&image, target).into_raw()
}

#[cfg(feature = "image")]
fn resize(image: &::image::RgbaImage, target: Size<u32>) -> ::image::RgbaImage {
    use ::image::imageops::{self, FilterType};
    use std::borrow::Cow;

    // if image is too big compared to target, box average it to double the target
    // and then do Lanczos. The final result is almost the same, but lanczos is too
    // expensive on large images.
    let image = if image.width() >= target.width * 4
        && image.height() >= target.height * 4
    {
        Cow::Owned(imageops::thumbnail(
            image,
            target.width * 2,
            target.height * 2,
        ))
    } else {
        Cow::Borrowed(image)
    };

    imageops::resize(&*image, target.width, target.height, FilterType::Lanczos3)
}

/// Resamples RGBA pixels down to `target`.
///
/// Premultiplies the image, to avoid fringing the edges of an icon when interpolating
/// transparent pixels.
#[cfg(feature = "image")]
pub fn downsample(image: &Buffer, target: Size<u32>) -> ::image::RgbaImage {
    let mut image = ::image::RgbaImage::from_raw(
        image.width(),
        image.height(),
        image.as_raw().to_vec(),
    )
    .expect("buffer holds width * height RGBA pixels");

    for pixel in image.pixels_mut() {
        let alpha = u32::from(pixel[3]);

        for channel in &mut pixel.0[..3] {
            *channel = ((u32::from(*channel) * alpha + 127) / 255) as u8;
        }
    }

    let mut image = resize(&image, target);

    for pixel in image.pixels_mut() {
        let alpha = u32::from(pixel[3]);

        if alpha > 0 {
            for channel in &mut pixel.0[..3] {
                *channel = ((u32::from(*channel) * 255 + alpha / 2) / alpha)
                    .min(255) as u8;
            }
        }
    }

    image
}
