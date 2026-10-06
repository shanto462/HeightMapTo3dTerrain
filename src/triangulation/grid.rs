use rayon::prelude::*;

use super::Triangulation;
use crate::{Error, Heightmap, Result};

/// Triangulates every pixel: two triangles per grid cell.
///
/// Each cell is split along the diagonal whose end points have the closer
/// heights, so ridges and valleys that run diagonally stay sharp instead of
/// turning into zig-zags.
///
/// # Errors
///
/// Returns [`Error::TooLarge`] when the heightmap has more pixels than a
/// 32-bit index can address.
pub fn triangulate_grid(map: &Heightmap) -> Result<Triangulation> {
    let (w, h) = (map.width() as usize, map.height() as usize);
    let count = w * h;
    if u32::try_from(count).is_err() {
        return Err(Error::TooLarge(format!(
            "{count} vertices do not fit in 32-bit indices; use the TIN mode"
        )));
    }

    let mut vertices = vec![[0u32; 2]; count];
    vertices.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
        for (x, v) in row.iter_mut().enumerate() {
            *v = [x as u32, y as u32];
        }
    });

    let heights = map.heights();
    let mut triangles = vec![[0u32; 3]; 2 * (w - 1) * (h - 1)];
    triangles
        .par_chunks_mut(2 * (w - 1))
        .enumerate()
        .for_each(|(y, row)| {
            for (x, cell) in row.as_chunks_mut::<2>().0.iter_mut().enumerate() {
                let i00 = y * w + x;
                let (i10, i01, i11) = (i00 + 1, i00 + w, i00 + w + 1);
                let main = (heights[i00] - heights[i11]).abs();
                let anti = (heights[i10] - heights[i01]).abs();
                let [i00, i10, i01, i11] = [i00, i10, i01, i11].map(|i| i as u32);
                if main <= anti {
                    cell[0] = [i00, i10, i11];
                    cell[1] = [i00, i11, i01];
                } else {
                    cell[0] = [i00, i10, i01];
                    cell[1] = [i10, i11, i01];
                }
            }
        });

    Ok(Triangulation {
        vertices,
        triangles,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::triangulation::test_support::{assert_valid, brute_force_max_error};

    #[test]
    fn covers_every_cell_with_two_triangles() {
        let map = Heightmap::from_heights(5, 3, (0..15).map(|i| i as f32).collect()).unwrap();
        let t = triangulate_grid(&map).unwrap();
        assert_eq!(t.vertex_count(), 15);
        assert_eq!(t.triangle_count(), 2 * 4 * 2);
        assert_valid(&t, 5, 3);
        assert!(brute_force_max_error(&t, &map) < 1e-9);
    }

    #[test]
    fn diagonal_follows_the_ridge() {
        // Ridge from top-right to bottom-left: the cell must split along it.
        let map = Heightmap::from_heights(2, 2, vec![0.0, 9.0, 8.0, 5.0]).unwrap();
        let t = triangulate_grid(&map).unwrap();
        assert_eq!(t.triangles, vec![[0, 1, 2], [1, 3, 2]]);
        let map = Heightmap::from_heights(2, 2, vec![9.0, 0.0, 5.0, 8.0]).unwrap();
        let t = triangulate_grid(&map).unwrap();
        assert_eq!(t.triangles, vec![[0, 1, 3], [0, 3, 2]]);
    }
}
