//! Loading heightmap images and mapping their samples to output heights.

use std::path::Path;

use image::{DynamicImage, ImageReader};
use rayon::prelude::*;

use crate::{Error, Result};

/// Largest supported width or height in pixels.
///
/// The limit keeps every geometric predicate in exact 64-bit integer arithmetic.
pub const MAX_DIMENSION: u32 = 1 << 24;

/// How raw image samples map to output heights.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeightMapping {
    /// Output height of the lowest sample.
    pub min: f32,
    /// Output height of the highest sample.
    pub max: f32,
    /// When `true`, the darkest pixel maps to `min` and the brightest to `max`.
    /// When `false`, black maps to `min` and full white maps to `max`.
    pub normalize: bool,
    /// Treat bright pixels as low and dark pixels as high.
    pub invert: bool,
}

impl Default for HeightMapping {
    fn default() -> Self {
        Self {
            min: 0.0,
            max: 100.0,
            normalize: true,
            invert: false,
        }
    }
}

/// A regular grid of heights in output units, stored row by row.
#[derive(Debug, Clone, PartialEq)]
pub struct Heightmap {
    width: u32,
    height: u32,
    heights: Vec<f32>,
    bit_depth: Option<u8>,
    quantization_step: Option<f32>,
}

impl Heightmap {
    /// Decodes an image file and maps its luminance to heights.
    ///
    /// Supports PNG, TIFF, JPEG, BMP, TGA and PNM in 8-bit, 16-bit or
    /// floating-point form. Color images are reduced to luminance.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Image`] when the file cannot be read or decoded, and
    /// the errors of [`Heightmap::from_samples`] for invalid content.
    pub fn load(path: impl AsRef<Path>, mapping: &HeightMapping) -> Result<Self> {
        let path = path.as_ref();
        let image_error = |source| Error::Image {
            path: path.to_path_buf(),
            source,
        };
        let mut reader = ImageReader::open(path)
            .map_err(|e| image_error(e.into()))?
            .with_guessed_format()
            .map_err(|e| image_error(e.into()))?;
        reader.no_limits();
        let image = reader.decode().map_err(image_error)?;
        Self::from_image(&image, mapping)
    }

    /// Maps the luminance of a decoded image to heights.
    ///
    /// # Errors
    ///
    /// See [`Heightmap::from_samples`].
    pub fn from_image(image: &DynamicImage, mapping: &HeightMapping) -> Result<Self> {
        let bits = match image {
            DynamicImage::ImageLuma8(_)
            | DynamicImage::ImageLumaA8(_)
            | DynamicImage::ImageRgb8(_)
            | DynamicImage::ImageRgba8(_) => Some(8),
            DynamicImage::ImageLuma16(_)
            | DynamicImage::ImageLumaA16(_)
            | DynamicImage::ImageRgb16(_)
            | DynamicImage::ImageRgba16(_) => Some(16),
            _ => None,
        };
        let luma = image.to_luma32f();
        let (width, height) = luma.dimensions();
        Self::from_samples(width, height, luma.into_raw(), bits, mapping)
    }

    /// Builds a heightmap from raw samples, where `0.0` is black and `1.0` is
    /// full white.
    ///
    /// `bit_depth` is the precision of the source (8 or 16 for integer images,
    /// `None` for floating point). It sets [`Heightmap::quantization_step`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidDimensions`], [`Error::SampleCount`],
    /// [`Error::NonFiniteSample`], or [`Error::InvalidOption`] when the
    /// mapping has a non-finite bound.
    pub fn from_samples(
        width: u32,
        height: u32,
        mut samples: Vec<f32>,
        bit_depth: Option<u8>,
        mapping: &HeightMapping,
    ) -> Result<Self> {
        check_dimensions(width, height, samples.len())?;
        if !mapping.min.is_finite() || !mapping.max.is_finite() {
            return Err(Error::InvalidOption(
                "minimum and maximum height must be finite numbers".into(),
            ));
        }
        let (smin, smax) = sample_range(&samples, width)?;
        let (lo, hi) = if mapping.normalize {
            (smin, smax)
        } else {
            (0.0, 1.0)
        };
        let span = hi - lo;
        let scale = if span > 0.0 { 1.0 / span } else { 0.0 };
        let (min, max, invert) = (mapping.min, mapping.max, mapping.invert);
        samples.par_iter_mut().for_each(|s| {
            let t = (*s - lo) * scale;
            let t = if invert { 1.0 - t } else { t };
            *s = min + t * (max - min);
        });
        let quantization_step = bit_depth.map(|bits| {
            let levels = f32::from(u16::MAX >> (16 - u16::from(bits.min(16))));
            (max - min).abs() * scale / levels
        });
        Ok(Self {
            width,
            height,
            heights: samples,
            bit_depth,
            quantization_step,
        })
    }

    /// Builds a heightmap from heights that are already in output units,
    /// for example elevations in meters from a DEM.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidDimensions`], [`Error::SampleCount`] or
    /// [`Error::NonFiniteSample`].
    pub fn from_heights(width: u32, height: u32, heights: Vec<f32>) -> Result<Self> {
        check_dimensions(width, height, heights.len())?;
        sample_range(&heights, width)?;
        Ok(Self {
            width,
            height,
            heights,
            bit_depth: None,
            quantization_step: None,
        })
    }

    /// Width in pixels.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Height in pixels.
    #[must_use]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// All heights, row by row from the top row of the image.
    #[must_use]
    pub fn heights(&self) -> &[f32] {
        &self.heights
    }

    /// Height of the pixel at column `x` and row `y`.
    ///
    /// # Panics
    ///
    /// Panics when the coordinates are outside the heightmap.
    #[must_use]
    pub fn get(&self, x: u32, y: u32) -> f32 {
        assert!(x < self.width && y < self.height, "pixel out of bounds");
        self.heights[y as usize * self.width as usize + x as usize]
    }

    /// Bit depth of the source image, or `None` for floating-point sources.
    #[must_use]
    pub fn bit_depth(&self) -> Option<u8> {
        self.bit_depth
    }

    /// Height difference between two adjacent gray levels of the source, in
    /// output units. `None` for floating-point sources.
    #[must_use]
    pub fn quantization_step(&self) -> Option<f32> {
        self.quantization_step
    }

    /// Lowest and highest height.
    #[must_use]
    pub fn height_range(&self) -> (f32, f32) {
        self.heights
            .par_iter()
            .fold(
                || (f32::INFINITY, f32::NEG_INFINITY),
                |(lo, hi), &h| (lo.min(h), hi.max(h)),
            )
            .reduce(
                || (f32::INFINITY, f32::NEG_INFINITY),
                |a, b| (a.0.min(b.0), a.1.max(b.1)),
            )
    }
}

fn check_dimensions(width: u32, height: u32, len: usize) -> Result<()> {
    let valid = 2..=MAX_DIMENSION;
    if !valid.contains(&width) || !valid.contains(&height) {
        return Err(Error::InvalidDimensions {
            width: width.into(),
            height: height.into(),
        });
    }
    let expected = width as usize * height as usize;
    if len != expected {
        return Err(Error::SampleCount {
            expected,
            actual: len,
        });
    }
    Ok(())
}

/// Returns the lowest and highest sample, or the first non-finite one.
fn sample_range(samples: &[f32], width: u32) -> Result<(f32, f32)> {
    if let Some(i) = samples.par_iter().position_first(|s| !s.is_finite()) {
        return Err(Error::NonFiniteSample {
            x: (i % width as usize) as u32,
            y: (i / width as usize) as u32,
        });
    }
    Ok(samples
        .par_iter()
        .fold(
            || (f32::INFINITY, f32::NEG_INFINITY),
            |(lo, hi), &s| (lo.min(s), hi.max(s)),
        )
        .reduce(
            || (f32::INFINITY, f32::NEG_INFINITY),
            |a, b| (a.0.min(b.0), a.1.max(b.1)),
        ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mapping(min: f32, max: f32, normalize: bool, invert: bool) -> HeightMapping {
        HeightMapping {
            min,
            max,
            normalize,
            invert,
        }
    }

    #[test]
    fn normalize_stretches_data_range() {
        let map = Heightmap::from_samples(
            2,
            2,
            vec![0.25, 0.5, 0.5, 0.75],
            Some(8),
            &mapping(-200.0, 200.0, true, false),
        )
        .unwrap();
        assert_eq!(map.heights(), &[-200.0, 0.0, 0.0, 200.0]);
        assert_eq!(map.height_range(), (-200.0, 200.0));
    }

    #[test]
    fn absolute_mapping_keeps_black_and_white_fixed() {
        let map = Heightmap::from_samples(
            2,
            2,
            vec![0.0, 0.5, 0.5, 1.0],
            Some(8),
            &mapping(10.0, 20.0, false, false),
        )
        .unwrap();
        assert_eq!(map.heights(), &[10.0, 15.0, 15.0, 20.0]);
        assert!((map.quantization_step().unwrap() - 10.0 / 255.0).abs() < 1e-6);
    }

    #[test]
    fn invert_swaps_low_and_high() {
        let map = Heightmap::from_samples(
            2,
            2,
            vec![0.0, 1.0, 1.0, 0.0],
            None,
            &mapping(0.0, 1.0, true, true),
        )
        .unwrap();
        assert_eq!(map.heights(), &[1.0, 0.0, 0.0, 1.0]);
        assert_eq!(map.quantization_step(), None);
    }

    #[test]
    fn flat_image_maps_to_min_height() {
        let map = Heightmap::from_samples(
            3,
            2,
            vec![0.4; 6],
            Some(16),
            &mapping(5.0, 9.0, true, false),
        )
        .unwrap();
        assert!(map.heights().iter().all(|&h| h == 5.0));
        assert_eq!(map.quantization_step(), Some(0.0));
    }

    #[test]
    fn quantization_step_follows_normalized_range() {
        // 16-bit image using a quarter of its range, stretched to 0..400.
        let map = Heightmap::from_samples(
            2,
            1 + 1,
            vec![0.0, 0.25, 0.1, 0.2],
            Some(16),
            &mapping(0.0, 400.0, true, false),
        )
        .unwrap();
        let expected = 400.0 / (0.25 * 65535.0);
        assert!((map.quantization_step().unwrap() - expected).abs() < 1e-6);
    }

    #[test]
    fn rejects_bad_input() {
        let m = HeightMapping::default();
        assert!(matches!(
            Heightmap::from_samples(1, 5, vec![0.0; 5], None, &m),
            Err(Error::InvalidDimensions { .. })
        ));
        assert!(matches!(
            Heightmap::from_samples(2, 2, vec![0.0; 3], None, &m),
            Err(Error::SampleCount { .. })
        ));
        assert!(matches!(
            Heightmap::from_samples(2, 2, vec![0.0, 0.0, f32::NAN, 0.0], None, &m),
            Err(Error::NonFiniteSample { x: 0, y: 1 })
        ));
        let bad = mapping(f32::INFINITY, 1.0, true, false);
        assert!(matches!(
            Heightmap::from_samples(2, 2, vec![0.0; 4], None, &bad),
            Err(Error::InvalidOption(_))
        ));
    }

    #[test]
    fn reads_16_bit_png_without_losing_precision() {
        let image =
            image::ImageBuffer::<image::Luma<u16>, _>::from_raw(2, 2, vec![0u16, 1, 65534, 65535])
                .unwrap();
        let map = Heightmap::from_image(
            &DynamicImage::ImageLuma16(image),
            &mapping(0.0, 65535.0, false, false),
        )
        .unwrap();
        assert_eq!(map.bit_depth(), Some(16));
        assert_eq!(map.heights(), &[0.0, 1.0, 65534.0, 65535.0]);
    }
}
