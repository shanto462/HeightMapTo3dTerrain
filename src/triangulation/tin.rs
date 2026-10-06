//! Adaptive triangulation by greedy Delaunay insertion.
//!
//! This is the algorithm from Garland and Heckbert, "Fast Polygonal
//! Approximation of Terrains and Height Fields" (1995). It starts with two
//! triangles that cover the heightmap. Each step finds the pixel with the
//! largest vertical error, inserts it as a new vertex, and restores the
//! Delaunay property by flipping edges. Only the triangles that changed are
//! scanned again, so a step costs time proportional to the area it touches.
//!
//! All geometric predicates use exact integer arithmetic, so the
//! triangulation never degenerates, even for perfectly regular grids.

use super::{Triangulation, orient};
use crate::{Error, Heightmap, Result};

/// When to stop refining a TIN. Refinement stops as soon as any limit is met.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TinOptions {
    /// Stop when no pixel is farther than this from the mesh, in output height
    /// units.
    pub max_error: f32,
    /// Stop before the mesh would exceed this many triangles.
    pub max_triangles: Option<usize>,
    /// Stop before the mesh would exceed this many vertices.
    pub max_vertices: Option<usize>,
}

impl Default for TinOptions {
    fn default() -> Self {
        Self {
            max_error: 0.0,
            max_triangles: None,
            max_vertices: None,
        }
    }
}

/// Facts about a finished TIN.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TinStats {
    /// Largest vertical distance between any pixel and the mesh.
    pub max_error: f32,
}

/// Builds an adaptive triangulation that keeps every pixel within the error
/// limit of `options` while using as few triangles as the greedy method can.
///
/// The result always has at least the four corner vertices and two triangles.
///
/// # Errors
///
/// Returns [`Error::InvalidOption`] when the error limit is negative or not a
/// number, and [`Error::TooLarge`] when the vertex count would overflow
/// 32-bit indices.
pub fn triangulate_tin(map: &Heightmap, options: &TinOptions) -> Result<(Triangulation, TinStats)> {
    if options.max_error.is_nan() || options.max_error < 0.0 {
        return Err(Error::InvalidOption(
            "maximum error must be zero or a positive number".into(),
        ));
    }
    // Errors below the precision of f32 heights are rounding noise.
    let (lo, hi) = map.height_range();
    let noise = 2.0 * f64::from(f32::EPSILON) * f64::from(lo.abs().max(hi.abs()));
    let threshold = f64::from(options.max_error).max(noise);
    let max_vertices = options.max_vertices.unwrap_or(usize::MAX);
    let max_triangles = options.max_triangles.unwrap_or(usize::MAX);

    let mut tin = Tin::new(map);
    while let Some(error) = tin.queue.peek_error() {
        if error <= threshold
            || tin.coords.len() >= max_vertices
            || tin.triangle_count() + 2 > max_triangles
        {
            break;
        }
        if tin.coords.len() >= u32::MAX as usize {
            return Err(Error::TooLarge(
                "the TIN needs more vertices than 32-bit indices allow".into(),
            ));
        }
        tin.step();
        tin.flush();
    }

    let max_error = tin.queue.peek_error().unwrap_or(0.0) as f32;
    let triangles = tin.tris.as_chunks::<3>().0.to_vec();
    let triangulation = Triangulation {
        vertices: tin.coords,
        triangles,
    };
    Ok((triangulation, TinStats { max_error }))
}

const NONE: u32 = u32::MAX;

#[inline]
fn next(e: usize) -> usize {
    if e % 3 == 2 { e - 2 } else { e + 1 }
}

#[inline]
fn prev(e: usize) -> usize {
    if e.is_multiple_of(3) { e + 2 } else { e - 1 }
}

/// True when `d` lies strictly inside the circumcircle of the positively
/// oriented triangle `abc`. Exact for coordinates below 2^24.
fn in_circle(a: [i64; 2], b: [i64; 2], c: [i64; 2], d: [i64; 2]) -> bool {
    let [adx, ady, bdx, bdy, cdx, cdy] = [
        a[0] - d[0],
        a[1] - d[1],
        b[0] - d[0],
        b[1] - d[1],
        c[0] - d[0],
        c[1] - d[1],
    ]
    .map(i128::from);
    let ad = adx * adx + ady * ady;
    let bd = bdx * bdx + bdy * bdy;
    let cd = cdx * cdx + cdy * cdy;
    let det =
        adx * (bdy * cd - bd * cdy) - ady * (bdx * cd - bd * cdx) + ad * (bdx * cdy - bdy * cdx);
    det > 0
}

/// Half-edge triangulation plus the per-triangle refinement state.
///
/// Triangle `t` owns half-edges `3t`, `3t + 1` and `3t + 2`; half-edge `e`
/// runs from vertex `tris[e]` to vertex `tris[next(e)]`.
struct Tin<'a> {
    heights: &'a [f32],
    width: usize,
    coords: Vec<[u32; 2]>,
    /// Vertex of each half-edge.
    tris: Vec<u32>,
    /// Opposite half-edge, or `NONE` on the border.
    twins: Vec<u32>,
    /// Pixel with the largest error inside each triangle.
    candidates: Vec<[u32; 2]>,
    queue: ErrorQueue,
    /// Triangles created or changed since the last flush.
    pending: Vec<u32>,
    is_pending: Vec<bool>,
    /// Scratch stack for edge legalization.
    flips: Vec<usize>,
}

impl<'a> Tin<'a> {
    fn new(map: &'a Heightmap) -> Self {
        let (x1, y1) = (map.width() - 1, map.height() - 1);
        let mut tin = Self {
            heights: map.heights(),
            width: map.width() as usize,
            coords: vec![[0, 0], [x1, 0], [0, y1], [x1, y1]],
            tris: Vec::new(),
            twins: Vec::new(),
            candidates: Vec::new(),
            queue: ErrorQueue::default(),
            pending: Vec::new(),
            is_pending: Vec::new(),
            flips: Vec::new(),
        };
        let t0 = tin.new_slot();
        let t1 = tin.new_slot();
        tin.set_triangle(t0, [0, 1, 3], [NONE, NONE, 3 * t1 as u32]);
        tin.set_triangle(t1, [0, 3, 2], [3 * t0 as u32 + 2, NONE, NONE]);
        tin.flush();
        tin
    }

    fn triangle_count(&self) -> usize {
        self.tris.len() / 3
    }

    fn point(&self, v: u32) -> [i64; 2] {
        let [x, y] = self.coords[v as usize];
        [i64::from(x), i64::from(y)]
    }

    fn height(&self, x: i64, y: i64) -> f64 {
        f64::from(self.heights[y as usize * self.width + x as usize])
    }

    fn new_slot(&mut self) -> usize {
        let t = self.triangle_count();
        self.tris.extend([NONE; 3]);
        self.twins.extend([NONE; 3]);
        self.candidates.push([0, 0]);
        self.is_pending.push(false);
        self.queue.grow(t + 1);
        t
    }

    /// Writes triangle `t`, links its twins, and schedules it for scanning.
    fn set_triangle(&mut self, t: usize, verts: [u32; 3], twins: [u32; 3]) {
        self.queue.remove(t);
        for i in 0..3 {
            let e = 3 * t + i;
            self.tris[e] = verts[i];
            self.twins[e] = twins[i];
            if twins[i] != NONE {
                self.twins[twins[i] as usize] = e as u32;
            }
        }
        if !self.is_pending[t] {
            self.is_pending[t] = true;
            self.pending.push(t as u32);
        }
    }

    /// Scans every changed triangle and puts it back into the queue.
    fn flush(&mut self) {
        let mut pending = std::mem::take(&mut self.pending);
        for &t in &pending {
            let t = t as usize;
            self.is_pending[t] = false;
            let (candidate, error) = self.scan(t);
            self.candidates[t] = candidate;
            self.queue.push(t, error);
        }
        pending.clear();
        self.pending = pending;
    }

    /// Finds the pixel inside triangle `t` with the largest vertical error.
    ///
    /// Rasterizes the triangle with incremental edge functions, so each pixel
    /// costs three additions and one interpolation.
    fn scan(&self, t: usize) -> ([u32; 2], f64) {
        let [v0, v1, v2] = [self.tris[3 * t], self.tris[3 * t + 1], self.tris[3 * t + 2]];
        let (p0, p1, p2) = (self.point(v0), self.point(v1), self.point(v2));
        let min_x = p0[0].min(p1[0]).min(p2[0]);
        let max_x = p0[0].max(p1[0]).max(p2[0]);
        let min_y = p0[1].min(p1[1]).min(p2[1]);
        let max_y = p0[1].max(p1[1]).max(p2[1]);

        // Edge function i is zero on the edge opposite vertex i and grows
        // toward it; `dx[i]` and `dy[i]` are its steps per pixel.
        let dx = [p1[1] - p2[1], p2[1] - p0[1], p0[1] - p1[1]];
        let dy = [p2[0] - p1[0], p0[0] - p2[0], p1[0] - p0[0]];
        let origin = [min_x, min_y];
        let mut row = [
            orient(p1, p2, origin),
            orient(p2, p0, origin),
            orient(p0, p1, origin),
        ];

        let area = orient(p0, p1, p2) as f64;
        let z = [
            self.height(p0[0], p0[1]) / area,
            self.height(p1[0], p1[1]) / area,
            self.height(p2[0], p2[1]) / area,
        ];

        let mut best = (0f64, [p0[0], p0[1]]);
        for y in min_y..=max_y {
            // Skip straight to the first pixel that can be inside.
            let mut skip = 0i64;
            let mut empty = false;
            for i in 0..3 {
                if row[i] < 0 {
                    if dx[i] > 0 {
                        skip = skip.max((-row[i] + dx[i] - 1) / dx[i]);
                    } else {
                        empty = true;
                    }
                }
            }
            if !empty && min_x + skip <= max_x {
                let mut w = [0; 3];
                for i in 0..3 {
                    w[i] = row[i] + dx[i] * skip;
                }
                let line = &self.heights[y as usize * self.width..];
                for x in min_x + skip..=max_x {
                    if w[0] < 0 || w[1] < 0 || w[2] < 0 {
                        break;
                    }
                    let interpolated = z[0] * w[0] as f64 + z[1] * w[1] as f64 + z[2] * w[2] as f64;
                    let error = (interpolated - f64::from(line[x as usize])).abs();
                    if error > best.0 {
                        best = (error, [x, y]);
                    }
                    for i in 0..3 {
                        w[i] += dx[i];
                    }
                }
            }
            for i in 0..3 {
                row[i] += dy[i];
            }
        }

        let (mut error, [x, y]) = best;
        if [p0, p1, p2].contains(&[x, y]) {
            error = 0.0;
        }
        ([x as u32, y as u32], error)
    }

    /// Inserts the worst pixel of the worst triangle.
    fn step(&mut self) {
        let t = self.queue.pop().expect("step needs a queued triangle");
        let [x, y] = self.candidates[t];
        let p = [i64::from(x), i64::from(y)];
        let pn = self.coords.len() as u32;
        self.coords.push([x, y]);

        let e = 3 * t;
        let [a, b, c] = [self.tris[e], self.tris[e + 1], self.tris[e + 2]].map(|v| self.point(v));
        if orient(a, b, p) == 0 {
            self.split_edge(pn, e);
        } else if orient(b, c, p) == 0 {
            self.split_edge(pn, e + 1);
        } else if orient(c, a, p) == 0 {
            self.split_edge(pn, e + 2);
        } else {
            self.split_triangle(pn, t);
        }
    }

    /// Splits triangle `t` into three around the interior point `pn`.
    fn split_triangle(&mut self, pn: u32, t: usize) {
        let e = 3 * t;
        let [v0, v1, v2] = [self.tris[e], self.tris[e + 1], self.tris[e + 2]];
        let [h0, h1, h2] = [self.twins[e], self.twins[e + 1], self.twins[e + 2]];
        let t1 = self.new_slot();
        let t2 = self.new_slot();
        let (e0, e1, e2) = (e as u32, 3 * t1 as u32, 3 * t2 as u32);
        self.set_triangle(t, [v0, v1, pn], [h0, e1 + 2, e2 + 1]);
        self.set_triangle(t1, [v1, v2, pn], [h1, e2 + 2, e0 + 1]);
        self.set_triangle(t2, [v2, v0, pn], [h2, e0 + 2, e1 + 1]);
        self.legalize(&[e, 3 * t1, 3 * t2]);
    }

    /// Splits half-edge `a` (and its twin, if any) at the point `pn`.
    fn split_edge(&mut self, pn: u32, a: usize) {
        let ta = a / 3;
        let (an, ap) = (next(a), prev(a));
        let [s, e, o] = [self.tris[a], self.tris[an], self.tris[ap]];
        let (h_an, h_ap) = (self.twins[an], self.twins[ap]);
        let b = self.twins[a];
        let ea = 3 * ta as u32;

        if b == NONE {
            let t2 = self.new_slot();
            let e2 = 3 * t2 as u32;
            self.set_triangle(ta, [o, s, pn], [h_ap, NONE, e2 + 1]);
            self.set_triangle(t2, [e, o, pn], [h_an, ea + 2, NONE]);
            self.legalize(&[3 * ta, 3 * t2]);
            return;
        }

        let b = b as usize;
        let tb = b / 3;
        let (bn, bp) = (next(b), prev(b));
        let q = self.tris[bp];
        let (h_bn, h_bp) = (self.twins[bn], self.twins[bp]);
        let t2 = self.new_slot();
        let t3 = self.new_slot();
        let (eb, e2, e3) = (3 * tb as u32, 3 * t2 as u32, 3 * t3 as u32);
        self.set_triangle(ta, [o, s, pn], [h_ap, eb + 2, e3 + 1]);
        self.set_triangle(tb, [s, q, pn], [h_bn, e2 + 2, ea + 1]);
        self.set_triangle(t2, [q, e, pn], [h_bp, e3 + 2, eb + 1]);
        self.set_triangle(t3, [e, o, pn], [h_an, ea + 2, e2 + 1]);
        self.legalize(&[3 * ta, 3 * tb, 3 * t2, 3 * t3]);
    }

    /// Restores the Delaunay property by flipping edges opposite the new
    /// vertex until none of them is illegal (Lawson's algorithm).
    fn legalize(&mut self, edges: &[usize]) {
        let mut stack = std::mem::take(&mut self.flips);
        stack.extend_from_slice(edges);
        while let Some(a) = stack.pop() {
            let b = self.twins[a];
            if b == NONE {
                continue;
            }
            let b = b as usize;
            let (an, ap, bn, bp) = (next(a), prev(a), next(b), prev(b));
            // Triangle A = (s, e, p0) and its neighbor B = (e, s, p1).
            let [s, e, p0, p1] = [self.tris[a], self.tris[an], self.tris[ap], self.tris[bp]];
            if !in_circle(self.point(s), self.point(e), self.point(p0), self.point(p1)) {
                continue;
            }
            let (h_an, h_ap, h_bn, h_bp) = (
                self.twins[an],
                self.twins[ap],
                self.twins[bn],
                self.twins[bp],
            );
            let (ta, tb) = (a / 3, b / 3);
            let (ea, eb) = (3 * ta as u32, 3 * tb as u32);
            self.set_triangle(ta, [p0, s, p1], [h_ap, h_bn, eb + 2]);
            self.set_triangle(tb, [p1, e, p0], [h_bp, h_an, ea + 2]);
            stack.push(3 * ta + 1);
            stack.push(3 * tb);
        }
        self.flips = stack;
    }
}

/// Indexed binary max-heap of triangles keyed by their error.
#[derive(Default)]
struct ErrorQueue {
    heap: Vec<u32>,
    /// Error of each triangle.
    errors: Vec<f64>,
    /// Heap position of each triangle, or `NONE` when not queued.
    position: Vec<u32>,
}

impl ErrorQueue {
    fn grow(&mut self, triangles: usize) {
        self.errors.resize(triangles, 0.0);
        self.position.resize(triangles, NONE);
    }

    fn peek_error(&self) -> Option<f64> {
        self.heap.first().map(|&t| self.errors[t as usize])
    }

    fn push(&mut self, t: usize, error: f64) {
        self.errors[t] = error;
        self.position[t] = self.heap.len() as u32;
        self.heap.push(t as u32);
        self.sift_up(self.heap.len() - 1);
    }

    fn pop(&mut self) -> Option<usize> {
        let &t = self.heap.first()?;
        self.remove(t as usize);
        Some(t as usize)
    }

    fn remove(&mut self, t: usize) {
        let i = self.position[t];
        if i == NONE {
            return;
        }
        let i = i as usize;
        let last = self.heap.len() - 1;
        self.swap(i, last);
        self.heap.pop();
        self.position[t] = NONE;
        if i < last && !self.sift_down(i) {
            self.sift_up(i);
        }
    }

    fn key(&self, i: usize) -> f64 {
        self.errors[self.heap[i] as usize]
    }

    fn swap(&mut self, i: usize, j: usize) {
        self.heap.swap(i, j);
        self.position[self.heap[i] as usize] = i as u32;
        self.position[self.heap[j] as usize] = j as u32;
    }

    fn sift_up(&mut self, mut i: usize) {
        while i > 0 {
            let parent = (i - 1) / 2;
            if self.key(i) <= self.key(parent) {
                break;
            }
            self.swap(i, parent);
            i = parent;
        }
    }

    /// Returns `true` when the element moved.
    fn sift_down(&mut self, start: usize) -> bool {
        let mut i = start;
        let n = self.heap.len();
        loop {
            let mut largest = i;
            for child in [2 * i + 1, 2 * i + 2] {
                if child < n && self.key(child) > self.key(largest) {
                    largest = child;
                }
            }
            if largest == i {
                return i != start;
            }
            self.swap(i, largest);
            i = largest;
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::HeightMapping;
    use crate::triangulation::test_support::{assert_valid, brute_force_max_error};

    fn map_from(width: u32, height: u32, f: impl Fn(f32, f32) -> f32) -> Heightmap {
        let heights = (0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .map(|(x, y)| f(x as f32, y as f32))
            .collect();
        Heightmap::from_heights(width, height, heights).unwrap()
    }

    /// Asserts that no interior edge has a vertex inside the circumcircle of
    /// the triangle on its other side.
    fn assert_delaunay(t: &Triangulation) {
        let mut opposite = std::collections::HashMap::new();
        for tri in &t.triangles {
            for i in 0..3 {
                opposite.insert((tri[i], tri[(i + 1) % 3]), tri[(i + 2) % 3]);
            }
        }
        let p = |v: u32| t.vertices[v as usize].map(i64::from);
        for (&(a, b), &c) in &opposite {
            if let Some(&d) = opposite.get(&(b, a)) {
                assert!(
                    !in_circle(p(a), p(b), p(c), p(d)),
                    "edge {a}-{b} is not Delaunay"
                );
            }
        }
    }

    #[test]
    fn plane_needs_only_two_triangles() {
        let map = map_from(64, 33, |x, y| 3.0 * x - 2.0 * y + 7.0);
        let (t, stats) = triangulate_tin(&map, &TinOptions::default()).unwrap();
        assert_eq!(t.triangle_count(), 2);
        assert_eq!(t.vertex_count(), 4);
        assert!(stats.max_error < 1e-9);
        assert_valid(&t, 64, 33);
    }

    #[test]
    fn zero_error_reproduces_every_pixel() {
        let map = map_from(40, 30, |x, y| {
            ((x * 0.37).sin() * (y * 0.23).cos() * 50.0).round()
        });
        let (t, stats) = triangulate_tin(&map, &TinOptions::default()).unwrap();
        assert_valid(&t, 40, 30);
        assert_delaunay(&t);
        assert!(stats.max_error <= 1e-4);
        assert!(brute_force_max_error(&t, &map) <= 1e-4);
    }

    #[test]
    fn error_limit_is_respected_and_reduces_size() {
        let map = map_from(97, 65, |x, y| {
            let (dx, dy) = (x - 48.0, y - 32.0);
            100.0 * (-(dx * dx + dy * dy) / 400.0).exp()
        });
        let full = 97 * 65;
        let (t, stats) = triangulate_tin(
            &map,
            &TinOptions {
                max_error: 0.5,
                ..TinOptions::default()
            },
        )
        .unwrap();
        assert_valid(&t, 97, 65);
        assert_delaunay(&t);
        assert!(stats.max_error <= 0.5);
        assert!(brute_force_max_error(&t, &map) <= 0.5 + 1e-4);
        assert!(
            t.vertex_count() * 10 < full,
            "{} vertices",
            t.vertex_count()
        );
    }

    #[test]
    fn budgets_are_respected() {
        let map = map_from(50, 50, |x, y| {
            (x * 0.5).sin() * 10.0 + (y * 0.3).cos() * 10.0
        });
        let (t, _) = triangulate_tin(
            &map,
            &TinOptions {
                max_triangles: Some(101),
                ..TinOptions::default()
            },
        )
        .unwrap();
        assert!(t.triangle_count() <= 101 && t.triangle_count() >= 99);
        assert_valid(&t, 50, 50);

        let (t, _) = triangulate_tin(
            &map,
            &TinOptions {
                max_vertices: Some(30),
                ..TinOptions::default()
            },
        )
        .unwrap();
        assert_eq!(t.vertex_count(), 30);
        assert_valid(&t, 50, 50);
    }

    #[test]
    fn regular_grid_spikes_hit_collinear_cases() {
        // Spikes on a lattice put many candidates exactly on existing edges.
        let map = map_from(33, 33, |x, y| {
            if (x as u32).is_multiple_of(4) && (y as u32).is_multiple_of(4) {
                10.0
            } else {
                0.0
            }
        });
        let (t, stats) = triangulate_tin(&map, &TinOptions::default()).unwrap();
        assert_valid(&t, 33, 33);
        assert_delaunay(&t);
        assert_eq!(stats.max_error, 0.0);
        assert!(brute_force_max_error(&t, &map) < 1e-9);
    }

    #[test]
    fn thin_strip_works() {
        let map = map_from(200, 2, |x, y| (x * 0.1).sin() * 5.0 + y);
        let (t, _) = triangulate_tin(&map, &TinOptions::default()).unwrap();
        assert_valid(&t, 200, 2);
        assert!(brute_force_max_error(&t, &map) < 1e-4);
    }

    #[test]
    fn rejects_negative_error() {
        let map = map_from(4, 4, |_, _| 0.0);
        let options = TinOptions {
            max_error: -1.0,
            ..TinOptions::default()
        };
        assert!(matches!(
            triangulate_tin(&map, &options),
            Err(Error::InvalidOption(_))
        ));
    }

    #[test]
    fn in_circle_is_exact_for_cocircular_points() {
        // Four corners of a square are cocircular: not strictly inside.
        assert!(!in_circle([0, 0], [10, 0], [10, 10], [0, 10]));
        assert!(in_circle([0, 0], [10, 0], [10, 10], [1, 9]));
        assert!(!in_circle([0, 0], [10, 0], [10, 10], [-1, 11]));
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        #[test]
        fn random_heightmaps_stay_valid_and_within_error(
            width in 2u32..24,
            height in 2u32..24,
            seed in any::<u64>(),
            levels in 1u32..12,
            max_error in 0.0f32..3.0,
        ) {
            // Quantized noise: many ties and flat areas, the hardest case.
            let mut state = seed | 1;
            let samples = (0..width * height)
                .map(|_| {
                    state ^= state << 13;
                    state ^= state >> 7;
                    state ^= state << 17;
                    (state % u64::from(levels + 1)) as f32 / levels as f32
                })
                .collect();
            let mapping = HeightMapping { min: -5.0, max: 5.0, normalize: false, invert: false };
            let map = Heightmap::from_samples(width, height, samples, None, &mapping).unwrap();
            let options = TinOptions { max_error, ..TinOptions::default() };
            let (t, stats) = triangulate_tin(&map, &options).unwrap();
            assert_valid(&t, width, height);
            assert_delaunay(&t);
            prop_assert!(stats.max_error <= max_error + 1e-5);
            prop_assert!(brute_force_max_error(&t, &map) <= f64::from(max_error) + 1e-4);
        }
    }
}
