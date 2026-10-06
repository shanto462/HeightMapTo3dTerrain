//! Mesh file writers.

mod glb;
mod obj;
mod ply;
mod stl;

use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use rayon::prelude::*;

use crate::{Mesh, Result};

/// Supported output formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Format {
    /// Binary glTF 2.0 (`.glb`), the standard for web and game engines.
    Glb,
    /// Wavefront OBJ text (`.obj`).
    Obj,
    /// Binary STL (`.stl`), for 3D printing. Stores no normals or UVs.
    Stl,
    /// Binary little-endian PLY (`.ply`).
    Ply,
}

impl Format {
    /// Picks the format from a file extension, ignoring case.
    #[must_use]
    pub fn from_path(path: &Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        match ext.as_str() {
            "glb" => Some(Self::Glb),
            "obj" => Some(Self::Obj),
            "stl" => Some(Self::Stl),
            "ply" => Some(Self::Ply),
            _ => None,
        }
    }

    /// Lowercase name of the format.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Glb => "glb",
            Self::Obj => "obj",
            Self::Stl => "stl",
            Self::Ply => "ply",
        }
    }
}

/// Writes `mesh` to `out` in the given format.
///
/// # Errors
///
/// Returns [`crate::Error::Io`] when writing fails and
/// [`crate::Error::TooLarge`] when the mesh exceeds the limits of the format.
pub fn write_mesh(mesh: &Mesh, format: Format, out: &mut impl Write) -> Result<()> {
    match format {
        Format::Glb => glb::write(mesh, out),
        Format::Obj => obj::write(mesh, out),
        Format::Stl => stl::write(mesh, out),
        Format::Ply => ply::write(mesh, out),
    }
}

/// Writes `mesh` to the file at `path`.
///
/// The data goes to a temporary file next to `path` that replaces `path`
/// only after everything was written, so a failed run never leaves a
/// truncated mesh behind.
///
/// # Errors
///
/// See [`write_mesh`].
pub fn save(mesh: &Mesh, format: Format, path: &Path) -> Result<()> {
    let mut partial = path.as_os_str().to_owned();
    partial.push(".partial");
    let partial = PathBuf::from(partial);
    let result = (|| {
        let mut out = BufWriter::with_capacity(1 << 20, File::create(&partial)?);
        write_mesh(mesh, format, &mut out)?;
        out.into_inner()
            .map_err(std::io::IntoInnerError::into_error)?
            .sync_all()?;
        fs::rename(&partial, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&partial);
    }
    result
}

/// Formats `items` into bytes on all cores and writes them in order.
///
/// Works in bounded waves so memory stays small for huge meshes.
pub(crate) fn write_parallel<T: Sync>(
    out: &mut impl Write,
    items: &[T],
    bytes_per_item: usize,
    format: impl Fn(&mut Vec<u8>, usize, &T) + Sync,
) -> std::io::Result<()> {
    const CHUNK: usize = 1 << 14;
    let wave = CHUNK * rayon::current_num_threads().max(1) * 2;
    for (w, wave_items) in items.chunks(wave).enumerate() {
        let buffers: Vec<Vec<u8>> = wave_items
            .par_chunks(CHUNK)
            .enumerate()
            .map(|(c, chunk)| {
                let mut buf = Vec::with_capacity(chunk.len() * bytes_per_item);
                let first = w * wave + c * CHUNK;
                for (i, item) in chunk.iter().enumerate() {
                    format(&mut buf, first + i, item);
                }
                buf
            })
            .collect();
        for buf in buffers {
            out.write_all(&buf)?;
        }
    }
    Ok(())
}

#[inline]
pub(crate) fn put_f32s(buf: &mut Vec<u8>, values: &[f32]) {
    for v in values {
        buf.extend_from_slice(&v.to_le_bytes());
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use crate::{Heightmap, Mesh, MeshOptions, triangulate_grid};

    pub fn sample_mesh(options: &MeshOptions) -> Mesh {
        let heights = (0..12).map(|i| (i * 7 % 5) as f32 * 0.5).collect();
        let map = Heightmap::from_heights(4, 3, heights).unwrap();
        Mesh::build(&triangulate_grid(&map).unwrap(), &map, options).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_from_extension() {
        assert_eq!(Format::from_path(Path::new("a/b.GLB")), Some(Format::Glb));
        assert_eq!(Format::from_path(Path::new("x.obj")), Some(Format::Obj));
        assert_eq!(Format::from_path(Path::new("x.stl")), Some(Format::Stl));
        assert_eq!(Format::from_path(Path::new("x.ply")), Some(Format::Ply));
        assert_eq!(Format::from_path(Path::new("x.fbx")), None);
        assert_eq!(Format::from_path(Path::new("noext")), None);
    }

    #[test]
    fn parallel_writer_keeps_order_across_waves() {
        let items: Vec<u32> = (0..100_000).collect();
        let mut out = Vec::new();
        write_parallel(&mut out, &items, 4, |buf, i, &v| {
            assert_eq!(i as u32, v);
            buf.extend_from_slice(&v.to_le_bytes());
        })
        .unwrap();
        let back: Vec<u32> = out
            .as_chunks::<4>()
            .0
            .iter()
            .map(|&c| u32::from_le_bytes(c))
            .collect();
        assert_eq!(back, items);
    }

    #[test]
    fn save_replaces_file_atomically() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.stl");
        fs::write(&path, b"old").unwrap();
        let mesh = test_support::sample_mesh(&crate::MeshOptions::default());
        save(&mesh, Format::Stl, &path).unwrap();
        assert_eq!(
            fs::metadata(&path).unwrap().len(),
            84 + 50 * mesh.triangle_count() as u64
        );
        assert!(!dir.path().join("t.stl.partial").exists());
    }
}
