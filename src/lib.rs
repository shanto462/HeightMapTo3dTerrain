//! Convert grayscale heightmaps into 3D terrain meshes.
//!
//! The pipeline has four steps:
//!
//! 1. [`Heightmap::load`] decodes an image and maps its gray levels to heights.
//! 2. [`triangulate_tin`] builds an adaptive mesh within an error limit, or
//!    [`triangulate_grid`] keeps every pixel.
//! 3. [`Mesh::build`] lifts the triangulation into 3D, with normals, texture
//!    coordinates, and an optional solid base.
//! 4. [`export::save`] writes glTF, OBJ, STL or PLY.
//!
//! ```
//! use heightmap_terrain::{Heightmap, Mesh, MeshOptions, TinOptions, export, triangulate_tin};
//!
//! # fn main() -> Result<(), heightmap_terrain::Error> {
//! // A 64x64 dome, already in output units.
//! let heights = (0..64 * 64)
//!     .map(|i| {
//!         let (x, y) = ((i % 64) as f32 - 31.5, (i / 64) as f32 - 31.5);
//!         (1000.0 - x * x - y * y).max(0.0).sqrt()
//!     })
//!     .collect();
//! let map = Heightmap::from_heights(64, 64, heights)?;
//!
//! let options = TinOptions { max_error: 0.25, ..TinOptions::default() };
//! let (triangulation, stats) = triangulate_tin(&map, &options)?;
//! assert!(stats.max_error <= 0.25);
//! assert!(triangulation.vertex_count() < 64 * 64);
//!
//! let mesh = Mesh::build(&triangulation, &map, &MeshOptions::default())?;
//! let mut glb = Vec::new();
//! export::write_mesh(&mesh, export::Format::Glb, &mut glb)?;
//! assert!(glb.starts_with(b"glTF"));
//! # Ok(())
//! # }
//! ```

#![cfg_attr(test, allow(clippy::float_cmp))]

mod error;
pub mod export;
mod heightmap;
mod mesh;
mod triangulation;

pub use error::{Error, Result};
pub use heightmap::{HeightMapping, Heightmap, MAX_DIMENSION};
pub use mesh::{Mesh, MeshOptions};
pub use triangulation::{TinOptions, TinStats, Triangulation, triangulate_grid, triangulate_tin};
