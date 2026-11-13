//! Tile geometry for borders, visibility and placement: the one place that knows the
//! map is a flat row-major grid, so another surface topology can replace it here.

use crate::{MapGrid, Pos, integer_sqrt};
use std::ops::RangeInclusive;

/// A tile coordinate.
pub(crate) type Tile = (i32, i32);

/// The tile containing a milli-tile position.
pub(crate) fn tile_of(pos: Pos) -> Tile {
    (pos.x.div_euclid(1000), pos.y.div_euclid(1000))
}

/// The centre of a tile in milli-tiles.
pub(crate) fn tile_centre(tile: Tile) -> Pos {
    Pos {
        x: tile.0 * 1000 + 500,
        y: tile.1 * 1000 + 500,
    }
}

/// The storage index of a tile, or None off the map.
pub(crate) fn index_of(map: &MapGrid, tile: Tile) -> Option<usize> {
    (tile.0 >= 0 && tile.1 >= 0 && (tile.0 as u32) < map.width && (tile.1 as u32) < map.height)
        .then(|| tile.1 as usize * map.width as usize + tile.0 as usize)
}

/// The tile stored at an index.
pub(crate) fn tile_at(map: &MapGrid, index: usize) -> Tile {
    let width = map.width as usize;
    ((index % width) as i32, (index / width) as i32)
}

/// Squared distance between two positions, in milli-tiles squared.
pub(crate) fn distance_squared(a: Pos, b: Pos) -> u128 {
    a.distance_squared_wide(b)
}

/// Distance between two positions, rounded down to whole milli-tiles.
pub(crate) fn distance(a: Pos, b: Pos) -> u64 {
    integer_sqrt(distance_squared(a, b))
}

/// Whether two positions are at most `radius` apart; a negative radius reaches nothing.
pub(crate) fn within(a: Pos, b: Pos, radius: i64) -> bool {
    radius >= 0 && distance_squared(a, b) <= (radius as u128).pow(2)
}

/// Whether two positions are strictly less than `radius` apart.
pub(crate) fn closer_than(a: Pos, b: Pos, radius: i64) -> bool {
    radius > 0 && distance_squared(a, b) < (radius as u128).pow(2)
}

/// Runs of tile indices whose centres lie within `radius` of `centre`, clipped to the map.
pub(crate) fn disc_runs(
    map: &MapGrid,
    centre: Pos,
    radius: i64,
) -> impl Iterator<Item = RangeInclusive<usize>> {
    let (width, height) = (i64::from(map.width), i64::from(map.height));
    let radius = radius.max(-1);
    let (cx, cy) = (i64::from(centre.x), i64::from(centre.y));
    let squared = i128::from(radius.max(0)).pow(2);
    // Rows whose centre y lies within the radius: centre(y) = 1000·y + 500.
    let first = (cy - radius - 500 + 999).div_euclid(1000).max(0);
    let last = (cy + radius - 500).div_euclid(1000).min(height - 1);
    (first..=last).filter_map(move |y| {
        if radius < 0 {
            return None;
        }
        let dy = i128::from(y * 1000 + 500 - cy);
        let rest = squared - dy * dy;
        if rest < 0 {
            return None;
        }
        let half = integer_sqrt(rest as u128) as i64;
        let x0 = (cx - half - 500 + 999).div_euclid(1000).max(0);
        let x1 = (cx + half - 500).div_euclid(1000).min(width - 1);
        (x0 <= x1).then(|| {
            let row = (y * width) as usize;
            row + x0 as usize..=row + x1 as usize
        })
    })
}

/// Tiles covered by footprint bounds `[left, top, right, bottom]` on whole tiles.
pub(crate) fn footprint_tiles(bounds: [i64; 4]) -> impl Iterator<Item = Tile> {
    (bounds[1].div_euclid(1000)..bounds[3].div_euclid(1000)).flat_map(move |y| {
        (bounds[0].div_euclid(1000)..bounds[2].div_euclid(1000)).map(move |x| (x as i32, y as i32))
    })
}

/// The footprint and its neighbouring tiles, including diagonals, out to `reach`
/// milli-tiles rounded up to whole tiles and at least one tile.
pub(crate) fn footprint_surroundings(bounds: [i64; 4], reach: i64) -> impl Iterator<Item = Tile> {
    let ring = (reach.max(1) + 999).div_euclid(1000) * 1000;
    footprint_tiles([
        bounds[0] - ring,
        bounds[1] - ring,
        bounds[2] + ring,
        bounds[3] + ring,
    ])
}

/// Whether a position lies inside footprint bounds, edges included.
pub(crate) fn footprint_contains(bounds: [i64; 4], pos: Pos) -> bool {
    (bounds[0]..=bounds[2]).contains(&i64::from(pos.x))
        && (bounds[1]..=bounds[3]).contains(&i64::from(pos.y))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rom_rules::DefId;

    #[test]
    fn disc_runs_match_a_direct_scan() {
        let map = MapGrid::new(20, 15, vec![DefId(1); 300]).unwrap();
        for (centre, radius) in [
            (Pos { x: 5500, y: 5500 }, 2000),
            (Pos { x: 5000, y: 7300 }, 3700),
            (Pos { x: 0, y: 0 }, 4500),
            (
                Pos {
                    x: 19_900,
                    y: 14_900,
                },
                0,
            ),
            (Pos { x: 10_500, y: 7500 }, 0),
            (Pos { x: 10_500, y: 7500 }, -5),
        ] {
            let runs: Vec<usize> = disc_runs(&map, centre, radius).flatten().collect();
            let scan: Vec<usize> = (0..300)
                .filter(|i| within(tile_centre(tile_at(&map, *i)), centre, radius))
                .collect();
            assert_eq!(runs, scan, "{centre:?} {radius}");
        }
    }
}
