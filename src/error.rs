use std::path::PathBuf;

/// Errors produced while loading heightmaps, building meshes, or writing them.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The heightmap image could not be opened or decoded.
    #[error("cannot read heightmap {}: {source}", path.display())]
    Image {
        /// Path of the image.
        path: PathBuf,
        /// Underlying decoder error.
        #[source]
        source: image::ImageError,
    },

    /// The heightmap is smaller than 2x2 or larger than the supported maximum.
    #[error(
        "heightmap is {width}x{height} pixels; each side must be between 2 and {} pixels",
        crate::heightmap::MAX_DIMENSION
    )]
    InvalidDimensions {
        /// Width in pixels.
        width: u64,
        /// Height in pixels.
        height: u64,
    },

    /// The number of samples does not match `width * height`.
    #[error("expected {expected} samples for the given size, got {actual}")]
    SampleCount {
        /// Samples required by the dimensions.
        expected: usize,
        /// Samples supplied.
        actual: usize,
    },

    /// A sample is NaN or infinite.
    #[error("sample at ({x}, {y}) is not a finite number")]
    NonFiniteSample {
        /// Column of the sample.
        x: u32,
        /// Row of the sample.
        y: u32,
    },

    /// An option has a value outside its valid range.
    #[error("invalid option: {0}")]
    InvalidOption(String),

    /// The mesh is too large for 32-bit indices or for the chosen file format.
    #[error("mesh is too large: {0}")]
    TooLarge(String),

    /// Writing the output failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Result type used throughout this crate.
pub type Result<T, E = Error> = std::result::Result<T, E>;
