//! Pixel conversion between raylib's read-back layout and the UI's image layout.

/// Converts bottom-up RGBA rows (raylib read-back) into top-down BGRA rows (GPUI).
///
/// Returns `None` when `rgba` does not hold exactly `width * height` pixels.
pub fn rgba_to_bgra_flipped(rgba: &[u8], width: usize, height: usize) -> Option<Vec<u8>> {
    let row = width.checked_mul(4)?;
    if rgba.len() != row.checked_mul(height)? {
        return None;
    }
    let mut bgra = vec![0_u8; rgba.len()];
    if row == 0 {
        return Some(bgra);
    }
    for (dst, src) in bgra.chunks_exact_mut(row).zip(rgba.chunks_exact(row).rev()) {
        let (dst, _) = dst.as_chunks_mut::<4>();
        let (src, _) = src.as_chunks::<4>();
        for (d, s) in dst.iter_mut().zip(src) {
            *d = [s[2], s[1], s[0], s[3]];
        }
    }
    Some(bgra)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgba_to_bgra_flips_rows_and_swaps_channels() {
        // Bottom row first, as raylib reads it back.
        let rgba = [
            1, 2, 3, 4, 5, 6, 7, 8, // bottom row: pixels A, B
            9, 10, 11, 12, 13, 14, 15, 16, // top row: pixels C, D
        ];
        let bgra = rgba_to_bgra_flipped(&rgba, 2, 2).expect("2x2 buffer");
        assert_eq!(
            bgra,
            vec![
                11, 10, 9, 12, 15, 14, 13, 16, // top row: C, D
                3, 2, 1, 4, 7, 6, 5, 8, // bottom row: A, B
            ]
        );
    }

    #[test]
    fn rgba_to_bgra_rejects_wrong_length() {
        assert_eq!(rgba_to_bgra_flipped(&[0; 12], 2, 2), None);
    }
}
