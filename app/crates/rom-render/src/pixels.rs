//! Pixel conversion between raylib's read-back layout and the UI's image layout.

/// Converts bottom-up RGBA rows (raylib read-back) into top-down BGRA rows (GPUI).
///
/// The world view is opaque, so alpha is forced to 255 (translucent water would otherwise lower it).
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
            *d = [s[2], s[1], s[0], 255];
        }
    }
    Some(bgra)
}

/// Copies bottom-up BGRA rows, `stride` bytes apart, into tightly packed top-down BGRA rows.
///
/// Whole rows are copied; channels and alpha are left as they are. Returns `None` when `stride`
/// is shorter than a row or `bgra` is too short for `height` rows.
pub fn bgra_rows_flipped(
    bgra: &[u8],
    width: usize,
    height: usize,
    stride: usize,
) -> Option<Vec<u8>> {
    let row = width.checked_mul(4)?;
    if stride < row {
        return None;
    }
    if let Some(last) = height.checked_sub(1)
        && bgra.len() < last.checked_mul(stride)?.checked_add(row)?
    {
        return None;
    }
    let mut out = Vec::with_capacity(row.checked_mul(height)?);
    for y in (0..height).rev() {
        let start = y * stride;
        out.extend_from_slice(&bgra[start..start + row]);
    }
    Some(out)
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
                11, 10, 9, 255, 15, 14, 13, 255, // top row: C, D
                3, 2, 1, 255, 7, 6, 5, 255, // bottom row: A, B
            ]
        );
    }

    #[test]
    fn rgba_to_bgra_rejects_wrong_length() {
        assert_eq!(rgba_to_bgra_flipped(&[0; 12], 2, 2), None);
    }

    #[test]
    fn bgra_rows_flipped_reverses_rows_only() {
        let bgra = [
            1, 2, 3, 4, 5, 6, 7, 8, // bottom row
            9, 10, 11, 12, 13, 14, 15, 16, // top row
        ];
        let flipped = bgra_rows_flipped(&bgra, 2, 2, 8).expect("2x2 buffer");
        assert_eq!(
            flipped,
            vec![9, 10, 11, 12, 13, 14, 15, 16, 1, 2, 3, 4, 5, 6, 7, 8]
        );
    }

    #[test]
    fn bgra_rows_flipped_skips_row_padding() {
        // One pixel per row, rows padded to 8 bytes; the last row needs no padding.
        let bgra = [
            1, 2, 3, 4, 0, 0, 0, 0, 5, 6, 7, 8, 0, 0, 0, 0, 9, 10, 11, 12,
        ];
        let flipped = bgra_rows_flipped(&bgra, 1, 3, 8).expect("padded buffer");
        assert_eq!(flipped, vec![9, 10, 11, 12, 5, 6, 7, 8, 1, 2, 3, 4]);
    }

    #[test]
    fn bgra_rows_flipped_rejects_short_input() {
        assert_eq!(bgra_rows_flipped(&[0; 12], 1, 3, 8), None);
        assert_eq!(
            bgra_rows_flipped(&[0; 16], 2, 2, 4),
            None,
            "stride below a row"
        );
        assert_eq!(bgra_rows_flipped(&[], 3, 0, 12), Some(Vec::new()));
    }
}
