//! Integer geometry in milli-tiles.

use crate::BodyId;

/// A position in milli-tiles on one face of one body's surface; positive y faces 90
/// degrees. A flat map has only face 0; on a cube sphere x and y are local to the face.
/// Positions on different bodies are never in reach of each other.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Pos {
    /// Horizontal position in milli-tiles.
    pub x: i32,
    /// Vertical position in milli-tiles.
    pub y: i32,
    /// Face of the surface, 0..6 on a cube sphere and always 0 on a flat map.
    pub face: u8,
    /// The body whose surface this is; 0 for the single surface of a one-surface world.
    pub body: BodyId,
}

impl Pos {
    /// A position on face 0 of body 0: the only face of a flat map.
    pub const fn flat(x: i32, y: i32) -> Self {
        Self {
            x,
            y,
            face: 0,
            body: BodyId(0),
        }
    }

    /// A position on a face of a body.
    pub const fn on(body: BodyId, face: u8, x: i32, y: i32) -> Self {
        Self { x, y, face, body }
    }

    /// Planar squared distance on one face, saturating only beyond the range of i64.
    /// It ignores faces and bodies; `World::distance` measures across a sphere's face
    /// edges and treats other bodies as out of reach.
    pub fn distance_squared(self, other: Self) -> i64 {
        self.distance_squared_wide(other).min(i64::MAX as u128) as i64
    }

    /// Planar Euclidean distance rounded down to whole milli-tiles; ignores faces and bodies.
    pub fn distance(self, other: Self) -> u64 {
        integer_sqrt(self.distance_squared_wide(other))
    }

    pub(crate) fn distance_squared_wide(self, other: Self) -> u128 {
        let dx = i128::from(self.x) - i128::from(other.x);
        let dy = i128::from(self.y) - i128::from(other.y);
        (dx * dx + dy * dy) as u128
    }
}

/// Returns the exact floor of the square root without floating point.
pub fn integer_sqrt(n: u128) -> u64 {
    if let Ok(small) = u64::try_from(n) {
        return small.isqrt();
    }
    let mut x = 1_u128 << (128 - n.leading_zeros()).div_ceil(2);
    loop {
        let next = (x + n / x) / 2;
        if next >= x {
            return x as u64;
        }
        x = next;
    }
}

/// Exact `a * b / c` (truncating), in 64-bit arithmetic whenever the product fits.
pub(crate) fn mul_div(a: i64, b: i64, c: i64) -> i64 {
    match a.checked_mul(b) {
        Some(product) => product / c,
        None => (i128::from(a) * i128::from(b) / i128::from(c)) as i64,
    }
}

/// Facing of a local vector in degrees, clockwise from positive x.
pub(crate) fn facing_of(dx: i64, dy: i64) -> u16 {
    let (ax, ay) = (dx.abs(), dy.abs());
    if ax == 0 && ay == 0 {
        return 0;
    }
    // A linear octant approximation, exact on axes and diagonals.
    let angle = if ax >= ay {
        45 * ay / ax
    } else {
        90 - 45 * ax / ay
    };
    let angle = match (dx < 0, dy < 0) {
        (false, false) => angle,
        (true, false) => 180 - angle,
        (true, true) => 180 + angle,
        (false, true) => 360 - angle,
    };
    (angle % 360) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roots_and_extreme_positions_do_not_overflow() {
        for n in [0, 1, 2, 3, 4, 99, 100, 101, u64::MAX as u128, u128::MAX] {
            let root = u128::from(integer_sqrt(n));
            assert!(root * root <= n);
            assert!((root + 1).checked_mul(root + 1).is_none_or(|v| v > n));
        }
        let a = Pos::flat(i32::MIN, i32::MIN);
        let b = Pos::flat(i32::MAX, i32::MAX);
        assert_eq!(a.distance_squared(b), i64::MAX);
        assert_eq!(a.distance(b), 6_074_000_998);
    }

    #[test]
    fn facing_covers_all_octants() {
        for (i, (x, y)) in [
            (1, 0),
            (1, 1),
            (0, 1),
            (-1, 1),
            (-1, 0),
            (-1, -1),
            (0, -1),
            (1, -1),
        ]
        .into_iter()
        .enumerate()
        {
            assert_eq!(facing_of(x, y), i as u16 * 45);
        }
    }
}
