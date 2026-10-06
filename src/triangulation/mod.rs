//! Triangulations of the pixel grid.
//!
//! Both strategies produce a [`Triangulation`] in pixel space. [`crate::Mesh`]
//! turns it into 3D geometry.

mod grid;
mod tin;

pub use grid::triangulate_grid;
pub use tin::{TinOptions, TinStats, triangulate_tin};

/// A triangulation whose vertices sit on pixel centers.
///
/// Triangles are positively oriented in pixel coordinates (x to the right,
/// y down): `(b - a) x (c - a) > 0` for every triangle `[a, b, c]`.
/// The triangles cover the full rectangle from pixel `(0, 0)` to
/// `(width - 1, height - 1)` without gaps or overlaps.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Triangulation {
    /// Pixel coordinates `[x, y]` of each vertex.
    pub vertices: Vec<[u32; 2]>,
    /// Vertex indices of each triangle.
    pub triangles: Vec<[u32; 3]>,
}

impl Triangulation {
    /// Number of vertices.
    #[must_use]
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// Number of triangles.
    #[must_use]
    pub fn triangle_count(&self) -> usize {
        self.triangles.len()
    }
}

/// Twice the signed area of triangle `abc`; positive when it turns left in
/// pixel coordinates.
#[inline]
pub(crate) fn orient(a: [i64; 2], b: [i64; 2], c: [i64; 2]) -> i64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}

#[cfg(test)]
pub(crate) mod test_support {
    //! Independent checks shared by the triangulation tests.

    use std::collections::HashMap;

    use super::{Triangulation, orient};
    use crate::Heightmap;

    fn point(t: &Triangulation, v: u32) -> [i64; 2] {
        let [x, y] = t.vertices[v as usize];
        [i64::from(x), i64::from(y)]
    }

    /// Asserts orientation, full coverage, and a manifold edge structure.
    pub fn assert_valid(t: &Triangulation, width: u32, height: u32) {
        let mut area = 0i64;
        let mut edges: HashMap<(u32, u32), u32> = HashMap::new();
        for tri in &t.triangles {
            let [a, b, c] = tri.map(|v| point(t, v));
            let o = orient(a, b, c);
            assert!(o > 0, "triangle {tri:?} is not positively oriented");
            area += o;
            for (p, q) in [(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])] {
                *edges.entry((p, q)).or_default() += 1;
            }
        }
        let full = 2 * i64::from(width - 1) * i64::from(height - 1);
        assert_eq!(area, full, "triangles do not cover the rectangle exactly");
        for (&(p, q), &n) in &edges {
            assert_eq!(n, 1, "edge {p}->{q} is used {n} times");
            if !edges.contains_key(&(q, p)) {
                let [a, b] = [point(t, p), point(t, q)];
                let on_border = (a[0] == b[0] && (a[0] == 0 || a[0] == i64::from(width - 1)))
                    || (a[1] == b[1] && (a[1] == 0 || a[1] == i64::from(height - 1)));
                assert!(on_border, "open edge {p}->{q} is not on the border");
            }
        }
    }

    /// Largest vertical distance between the heightmap and the linear
    /// interpolation of the triangulation, checked pixel by pixel.
    pub fn brute_force_max_error(t: &Triangulation, map: &Heightmap) -> f64 {
        let mut worst = 0f64;
        for tri in &t.triangles {
            let [a, b, c] = tri.map(|v| point(t, v));
            let area = orient(a, b, c) as f64;
            let z = tri.map(|v| {
                let [x, y] = t.vertices[v as usize];
                f64::from(map.get(x, y))
            });
            let (x0, x1) = (a[0].min(b[0]).min(c[0]), a[0].max(b[0]).max(c[0]));
            let (y0, y1) = (a[1].min(b[1]).min(c[1]), a[1].max(b[1]).max(c[1]));
            for y in y0..=y1 {
                for x in x0..=x1 {
                    let p = [x, y];
                    let (w0, w1, w2) = (orient(b, c, p), orient(c, a, p), orient(a, b, p));
                    if w0 < 0 || w1 < 0 || w2 < 0 {
                        continue;
                    }
                    let interp = (z[0] * w0 as f64 + z[1] * w1 as f64 + z[2] * w2 as f64) / area;
                    let actual = f64::from(map.get(x as u32, y as u32));
                    worst = worst.max((interp - actual).abs());
                }
            }
        }
        worst
    }
}
