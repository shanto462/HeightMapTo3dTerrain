//! 3D geometry built from a triangulation.

use rayon::prelude::*;

use crate::{Error, Heightmap, Result, Triangulation};

/// How a [`Triangulation`] becomes 3D geometry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeshOptions {
    /// Horizontal distance between two pixels, in output units.
    pub pixel_size: f32,
    /// Put the center of the terrain at the origin of the X and Z axes.
    pub center: bool,
    /// Compute smooth vertex normals.
    pub normals: bool,
    /// Compute texture coordinates that map the heightmap image onto the
    /// terrain.
    pub uvs: bool,
    /// Close the terrain into a solid with side walls and a flat bottom that
    /// sits this far below the lowest point. Useful for 3D printing.
    pub base: Option<f32>,
}

impl Default for MeshOptions {
    fn default() -> Self {
        Self {
            pixel_size: 1.0,
            center: true,
            normals: true,
            uvs: true,
            base: None,
        }
    }
}

/// An indexed triangle mesh.
///
/// The Y axis points up. Image columns run along +X and image rows along +Z,
/// so the top of the image faces -Z. Triangles wind counter-clockwise when
/// seen from outside.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Mesh {
    /// Vertex positions.
    pub positions: Vec<[f32; 3]>,
    /// Unit vertex normals, if computed.
    pub normals: Option<Vec<[f32; 3]>>,
    /// Texture coordinates with the origin at the top-left corner of the
    /// image and V pointing down (the glTF convention), if computed.
    pub uvs: Option<Vec<[f32; 2]>>,
    /// Vertex indices of each triangle.
    pub indices: Vec<[u32; 3]>,
}

impl Mesh {
    /// Lifts `triangulation` into 3D using the heights of `map`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidOption`] for a non-positive pixel size or base
    /// thickness, and [`Error::TooLarge`] when the solid base would push the
    /// vertex count past 32-bit indices.
    pub fn build(
        triangulation: &Triangulation,
        map: &Heightmap,
        options: &MeshOptions,
    ) -> Result<Self> {
        let ps = options.pixel_size;
        if !(ps.is_finite() && ps > 0.0) {
            return Err(Error::InvalidOption(
                "pixel size must be a positive number".into(),
            ));
        }
        if let Some(base) = options.base
            && !(base.is_finite() && base > 0.0)
        {
            return Err(Error::InvalidOption(
                "base thickness must be a positive number".into(),
            ));
        }

        let (w, h) = (map.width() - 1, map.height() - 1);
        let (ox, oz) = if options.center {
            (w as f32 * ps / 2.0, h as f32 * ps / 2.0)
        } else {
            (0.0, 0.0)
        };
        let positions: Vec<[f32; 3]> = triangulation
            .vertices
            .par_iter()
            .map(|&[x, y]| [x as f32 * ps - ox, map.get(x, y), y as f32 * ps - oz])
            .collect();
        // Seen from above with Y up, positively oriented pixel triangles are
        // clockwise, so swap two corners.
        let indices: Vec<[u32; 3]> = triangulation
            .triangles
            .par_iter()
            .map(|&[a, b, c]| [a, c, b])
            .collect();
        let normals = options
            .normals
            .then(|| vertex_normals(&positions, &indices));
        let uvs = options.uvs.then(|| {
            let (sx, sy) = (1.0 / w as f32, 1.0 / h as f32);
            triangulation
                .vertices
                .par_iter()
                .map(|&[x, y]| [x as f32 * sx, y as f32 * sy])
                .collect()
        });

        let mut mesh = Self {
            positions,
            normals,
            uvs,
            indices,
        };
        if let Some(thickness) = options.base {
            let floor = map.height_range().0 - thickness;
            mesh.add_base(triangulation, map.width(), map.height(), floor)?;
        }
        Ok(mesh)
    }

    /// Number of vertices.
    #[must_use]
    pub fn vertex_count(&self) -> usize {
        self.positions.len()
    }

    /// Number of triangles.
    #[must_use]
    pub fn triangle_count(&self) -> usize {
        self.indices.len()
    }

    /// Smallest and largest coordinate on each axis.
    #[must_use]
    pub fn bounds(&self) -> ([f32; 3], [f32; 3]) {
        let empty = ([f32::INFINITY; 3], [f32::NEG_INFINITY; 3]);
        self.positions
            .par_iter()
            .fold(
                || empty,
                |(mut lo, mut hi), p| {
                    for i in 0..3 {
                        lo[i] = lo[i].min(p[i]);
                        hi[i] = hi[i].max(p[i]);
                    }
                    (lo, hi)
                },
            )
            .reduce(
                || empty,
                |(mut lo, mut hi), (l, h)| {
                    for i in 0..3 {
                        lo[i] = lo[i].min(l[i]);
                        hi[i] = hi[i].max(h[i]);
                    }
                    (lo, hi)
                },
            )
    }

    /// Adds four side walls and a bottom so the mesh encloses a volume.
    ///
    /// Walls and bottom get their own vertices so their normals stay sharp;
    /// the result is watertight once coincident vertices are merged.
    fn add_base(
        &mut self,
        triangulation: &Triangulation,
        width: u32,
        height: u32,
        floor: f32,
    ) -> Result<()> {
        let (xm, ym) = (width - 1, height - 1);
        // Position along the border, clockwise in pixel space from (0, 0).
        let perimeter = |[x, y]: [u32; 2]| -> Option<u64> {
            let (x, y, xm, ym) = (u64::from(x), u64::from(y), u64::from(xm), u64::from(ym));
            if y == 0 {
                Some(x)
            } else if x == xm {
                Some(xm + y)
            } else if y == ym {
                Some(2 * xm + ym - x)
            } else if x == 0 {
                Some(2 * (xm + ym) - y)
            } else {
                None
            }
        };
        let mut border: Vec<(u64, u32)> = triangulation
            .vertices
            .iter()
            .enumerate()
            .filter_map(|(i, &v)| perimeter(v).map(|p| (p, i as u32)))
            .collect();
        border.sort_unstable();
        let border: Vec<u32> = border.into_iter().map(|(_, i)| i).collect();

        let added = 3 * border.len() + 9;
        if u32::try_from(self.positions.len() + added).is_err() {
            return Err(Error::TooLarge(
                "the solid base needs more than 2^32 vertices".into(),
            ));
        }

        // Corner positions along the border, used to split it into sides.
        let corners = [0, u64::from(xm), u64::from(xm + ym), u64::from(2 * xm + ym)];
        let outward: [[f32; 3]; 4] = [
            [0.0, 0.0, -1.0],
            [1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [-1.0, 0.0, 0.0],
        ];
        let n = border.len();
        let start_of = |side: usize| {
            border
                .iter()
                .position(|&v| perimeter(triangulation.vertices[v as usize]) == Some(corners[side]))
                .expect("corners are always vertices")
        };
        for (side, normal) in outward.into_iter().enumerate() {
            let (from, to) = (start_of(side), start_of((side + 1) % 4));
            let len = (to + n - from) % n + 1;
            let strip: Vec<u32> = (0..len).map(|k| border[(from + k) % n]).collect();
            let first = self.positions.len() as u32;
            for &v in &strip {
                let top = self.positions[v as usize];
                self.push_copy(v, top, normal);
                self.push_copy(v, [top[0], floor, top[2]], normal);
            }
            for k in 0..strip.len() as u32 - 1 {
                let (pt, pb, qt, qb) = (
                    first + 2 * k,
                    first + 2 * k + 1,
                    first + 2 * k + 2,
                    first + 2 * k + 3,
                );
                self.indices.push([pt, qt, qb]);
                self.indices.push([pt, qb, pb]);
            }
        }

        let first = self.positions.len() as u32;
        let down = [0.0, -1.0, 0.0];
        for &v in &border {
            let top = self.positions[v as usize];
            self.push_copy(v, [top[0], floor, top[2]], down);
        }
        let ([x0, _, z0], [x1, _, z1]) = self.bounds();
        let center = self.positions.len() as u32;
        self.positions
            .push([f32::midpoint(x0, x1), floor, f32::midpoint(z0, z1)]);
        if let Some(normals) = &mut self.normals {
            normals.push(down);
        }
        if let Some(uvs) = &mut self.uvs {
            uvs.push([0.5, 0.5]);
        }
        for k in 0..n as u32 {
            let next = (k + 1) % n as u32;
            self.indices.push([first + k, first + next, center]);
        }
        Ok(())
    }

    /// Appends a vertex at `position` that reuses the texture coordinate of
    /// vertex `source`.
    fn push_copy(&mut self, source: u32, position: [f32; 3], normal: [f32; 3]) {
        self.positions.push(position);
        if let Some(normals) = &mut self.normals {
            normals.push(normal);
        }
        if let Some(uvs) = &mut self.uvs {
            let uv = uvs[source as usize];
            uvs.push(uv);
        }
    }
}

/// Unnormalized normal of a triangle: its length is twice the area.
#[inline]
pub(crate) fn face_normal(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> [f32; 3] {
    let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ]
}

/// Scales `v` to unit length, or returns `fallback` for a zero vector.
#[inline]
pub(crate) fn normalize(v: [f32; 3], fallback: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len > 0.0 && len.is_finite() {
        [v[0] / len, v[1] / len, v[2] / len]
    } else {
        fallback
    }
}

/// Area-weighted vertex normals.
fn vertex_normals(positions: &[[f32; 3]], indices: &[[u32; 3]]) -> Vec<[f32; 3]> {
    let faces: Vec<[f32; 3]> = indices
        .par_iter()
        .map(|&[a, b, c]| {
            face_normal(
                positions[a as usize],
                positions[b as usize],
                positions[c as usize],
            )
        })
        .collect();
    let mut sums = vec![[0f32; 3]; positions.len()];
    for (tri, n) in indices.iter().zip(&faces) {
        for &v in tri {
            let s = &mut sums[v as usize];
            s[0] += n[0];
            s[1] += n[1];
            s[2] += n[2];
        }
    }
    sums.par_iter_mut()
        .for_each(|n| *n = normalize(*n, [0.0, 1.0, 0.0]));
    sums
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::{TinOptions, triangulate_grid, triangulate_tin};

    fn bumpy(width: u32, height: u32) -> Heightmap {
        let heights = (0..height)
            .flat_map(|y| {
                (0..width).map(move |x| (x as f32 * 0.7).sin() * 3.0 + (y as f32 * 0.4).cos() * 2.0)
            })
            .collect();
        Heightmap::from_heights(width, height, heights).unwrap()
    }

    /// Merges vertices with identical positions and checks that every edge
    /// is shared by exactly two triangles running in opposite directions.
    fn assert_watertight(mesh: &Mesh) {
        let mut ids: HashMap<[u32; 3], u32> = HashMap::new();
        let welded: Vec<u32> = mesh
            .positions
            .iter()
            .map(|p| {
                let next = ids.len() as u32;
                *ids.entry(p.map(f32::to_bits)).or_insert(next)
            })
            .collect();
        let mut edges: HashMap<(u32, u32), i32> = HashMap::new();
        for tri in &mesh.indices {
            let t = tri.map(|v| welded[v as usize]);
            for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                assert_ne!(a, b, "degenerate edge");
                *edges.entry((a, b)).or_default() += 1;
            }
        }
        for (&(a, b), &n) in &edges {
            assert_eq!(n, 1, "edge {a}->{b} used {n} times");
            assert_eq!(edges.get(&(b, a)), Some(&1), "edge {a}->{b} has no partner");
        }
    }

    fn signed_volume(mesh: &Mesh) -> f64 {
        mesh.indices
            .iter()
            .map(|&[a, b, c]| {
                let [a, b, c] = [a, b, c].map(|i| mesh.positions[i as usize].map(f64::from));
                (a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
                    + a[2] * (b[0] * c[1] - b[1] * c[0]))
                    / 6.0
            })
            .sum()
    }

    #[test]
    fn top_faces_point_up() {
        let map = bumpy(9, 7);
        let mesh = Mesh::build(
            &triangulate_grid(&map).unwrap(),
            &map,
            &MeshOptions::default(),
        )
        .unwrap();
        for &[a, b, c] in &mesh.indices {
            let n = face_normal(
                mesh.positions[a as usize],
                mesh.positions[b as usize],
                mesh.positions[c as usize],
            );
            assert!(n[1] > 0.0, "face normal {n:?} points down");
        }
        for n in mesh.normals.as_ref().unwrap() {
            assert!(n[1] > 0.0);
            assert!(((n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt() - 1.0).abs() < 1e-5);
        }
    }

    #[test]
    fn layout_matches_image_orientation() {
        let map = Heightmap::from_heights(3, 2, vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0]).unwrap();
        let options = MeshOptions {
            pixel_size: 2.0,
            ..MeshOptions::default()
        };
        let mesh = Mesh::build(&triangulate_grid(&map).unwrap(), &map, &options).unwrap();
        assert_eq!(mesh.positions[0], [-2.0, 0.0, -1.0]);
        assert_eq!(mesh.positions[5], [2.0, 5.0, 1.0]);
        let uvs = mesh.uvs.as_ref().unwrap();
        assert_eq!(uvs[0], [0.0, 0.0]);
        assert_eq!(uvs[5], [1.0, 1.0]);
        assert_eq!(mesh.bounds(), ([-2.0, 0.0, -1.0], [2.0, 5.0, 1.0]));
    }

    #[test]
    fn solid_base_is_closed_and_outward() {
        let map = bumpy(12, 8);
        let options = MeshOptions {
            base: Some(2.0),
            ..MeshOptions::default()
        };
        for triangulation in [
            triangulate_grid(&map).unwrap(),
            triangulate_tin(
                &map,
                &TinOptions {
                    max_error: 0.3,
                    ..TinOptions::default()
                },
            )
            .unwrap()
            .0,
        ] {
            let mesh = Mesh::build(&triangulation, &map, &options).unwrap();
            assert_watertight(&mesh);
            assert!(signed_volume(&mesh) > 0.0, "normals point inward");
            let (lo, _) = mesh.bounds();
            assert!((lo[1] - (map.height_range().0 - 2.0)).abs() < 1e-6);
            assert_eq!(mesh.normals.as_ref().unwrap().len(), mesh.vertex_count());
            assert_eq!(mesh.uvs.as_ref().unwrap().len(), mesh.vertex_count());
        }
    }

    #[test]
    fn optional_attributes_can_be_skipped() {
        let map = bumpy(4, 4);
        let options = MeshOptions {
            normals: false,
            uvs: false,
            base: Some(1.0),
            ..MeshOptions::default()
        };
        let mesh = Mesh::build(&triangulate_grid(&map).unwrap(), &map, &options).unwrap();
        assert!(mesh.normals.is_none() && mesh.uvs.is_none());
        assert_watertight(&mesh);
    }

    #[test]
    fn rejects_bad_options() {
        let map = bumpy(3, 3);
        let t = triangulate_grid(&map).unwrap();
        for options in [
            MeshOptions {
                pixel_size: 0.0,
                ..MeshOptions::default()
            },
            MeshOptions {
                base: Some(-1.0),
                ..MeshOptions::default()
            },
        ] {
            assert!(matches!(
                Mesh::build(&t, &map, &options),
                Err(Error::InvalidOption(_))
            ));
        }
    }
}
