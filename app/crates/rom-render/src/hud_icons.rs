//! Model pictures for the palette (task 91): the CPU half. The render thread draws each model
//! [`SUPERSAMPLE`] times larger than its icon; here the picture is filtered down and given the
//! HUD's icon treatment, a dark outline like the one round every vector icon, so models sit
//! with the gold glyphs of `assets/hud/icons/`.

/// How many rendered pixels make one icon pixel along each side.
pub const SUPERSAMPLE: u32 = 4;
/// Largest edge of a rendered model picture, in pixels.
pub const MAX_RENDER_EDGE: u32 = 512;
/// Colour of the outline (the vector icons' outline, `#2e1e0a`).
const OUTLINE: [u8; 3] = [0x2e, 0x1e, 0x0a];
/// Opacity of the outline.
const OUTLINE_ALPHA: f32 = 0.92;

/// How many times larger than an `edge`-pixel icon its model is rendered.
pub fn supersample(edge: u32) -> u32 {
    (MAX_RENDER_EDGE / edge.max(1)).clamp(1, SUPERSAMPLE)
}

/// Filters a straight-alpha RGBA picture of `factor × edge` pixels a side down to `edge`
/// pixels: the average of each block, weighted by coverage, so edges come out smooth.
pub fn downsample(raw: &[u8], edge: u32, factor: u32) -> Vec<u8> {
    let (edge, factor) = (edge as usize, factor.max(1) as usize);
    let raw_edge = edge * factor;
    assert_eq!(raw.len(), raw_edge * raw_edge * 4, "picture size");
    let mut out = vec![0_u8; edge * edge * 4];
    let block = (factor * factor) as f32;
    for y in 0..edge {
        for x in 0..edge {
            let mut sum = [0.0_f32; 4];
            for dy in 0..factor {
                let row = (y * factor + dy) * raw_edge;
                for dx in 0..factor {
                    let p = &raw[(row + x * factor + dx) * 4..][..4];
                    let a = f32::from(p[3]) / 255.0;
                    for c in 0..3 {
                        sum[c] += f32::from(p[c]) * a;
                    }
                    sum[3] += a;
                }
            }
            let o = &mut out[(y * edge + x) * 4..][..4];
            if sum[3] > 0.0 {
                for c in 0..3 {
                    o[c] = (sum[c] / sum[3]).round().clamp(0.0, 255.0) as u8;
                }
                o[3] = (sum[3] / block * 255.0).round().clamp(0.0, 255.0) as u8;
            }
        }
    }
    out
}

/// Puts a dark outline `width` pixels wide round the picture (straight-alpha RGBA, `edge`
/// pixels a side): every pixel takes the outline under the picture, as strong as the most
/// covered pixel within reach.
pub fn outline(pixels: &[u8], edge: u32, width: f32) -> Vec<u8> {
    let edge = edge as usize;
    assert_eq!(pixels.len(), edge * edge * 4, "picture size");
    let reach = width.ceil() as isize;
    let mut out = vec![0_u8; pixels.len()];
    for y in 0..edge as isize {
        for x in 0..edge as isize {
            let mut halo = 0.0_f32;
            for dy in -reach..=reach {
                for dx in -reach..=reach {
                    let distance = ((dx * dx + dy * dy) as f32).sqrt();
                    // Full strength for neighbours within the width, fading over the next pixel.
                    let weight = (width + 1.0 - distance).clamp(0.0, 1.0);
                    let (sx, sy) = (x + dx, y + dy);
                    if weight <= 0.0
                        || sx < 0
                        || sy < 0
                        || sx >= edge as isize
                        || sy >= edge as isize
                    {
                        continue;
                    }
                    let a = f32::from(pixels[(sy as usize * edge + sx as usize) * 4 + 3]) / 255.0;
                    halo = halo.max(a * weight);
                }
            }
            let i = (y as usize * edge + x as usize) * 4;
            let top = f32::from(pixels[i + 3]) / 255.0;
            let under = halo * OUTLINE_ALPHA;
            let alpha = top + under * (1.0 - top);
            if alpha <= 0.0 {
                continue;
            }
            for c in 0..3 {
                let colour = (f32::from(pixels[i + c]) * top
                    + f32::from(OUTLINE[c]) * under * (1.0 - top))
                    / alpha;
                out[i + c] = colour.round().clamp(0.0, 255.0) as u8;
            }
            out[i + 3] = (alpha * 255.0).round() as u8;
        }
    }
    out
}

/// The finished icon of a model rendered `factor` times larger than `edge`: filtered down and
/// outlined (about one design pixel wide at the HUD's `scale`).
pub fn finish(raw: &[u8], edge: u32, factor: u32, scale: f32) -> Vec<u8> {
    outline(
        &downsample(raw, edge, factor),
        edge,
        (1.1 * scale).clamp(0.8, 2.5),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `size`-pixel picture with an opaque square of `side` pixels at `at`.
    fn square(size: usize, at: usize, side: usize, rgb: [u8; 3]) -> Vec<u8> {
        let mut p = vec![0_u8; size * size * 4];
        for y in at..at + side {
            for x in at..at + side {
                p[(y * size + x) * 4..][..4].copy_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
            }
        }
        p
    }

    #[test]
    fn downsampling_averages_colour_by_coverage() {
        // A 4 × 4 block a quarter covered by red: a quarter opaque, and still pure red.
        let raw = square(8, 2, 2, [200, 10, 10]);
        let small = downsample(&raw, 2, 4);
        assert_eq!(&small[..4], &[200, 10, 10, 64]);
        let full = downsample(&square(8, 0, 8, [9, 99, 199]), 2, 4);
        assert!(full.chunks(4).all(|p| p == [9, 99, 199, 255]));
    }

    #[test]
    fn the_outline_surrounds_the_shape_and_leaves_it_alone() {
        let pixels = square(12, 4, 4, [250, 200, 120]);
        let lined = outline(&pixels, 12, 1.0);
        let at = |x: usize, y: usize| &lined[(y * 12 + x) * 4..][..4];
        // Inside: unchanged.
        assert_eq!(at(5, 5), &[250, 200, 120, 255]);
        // Beside: dark and nearly opaque.
        let beside = at(3, 5);
        assert!(beside[3] > 200 && beside[0] < 60, "{beside:?}");
        // Far away: clear.
        assert_eq!(at(0, 0)[3], 0);
        assert_eq!(at(11, 11)[3], 0);
    }

    #[test]
    fn large_icons_render_at_most_the_largest_edge() {
        assert_eq!(supersample(44), SUPERSAMPLE);
        assert!(supersample(200) * 200 <= MAX_RENDER_EDGE);
        assert_eq!(supersample(900), 1);
        let finished = finish(&square(176, 40, 96, [1, 2, 3]), 44, 4, 1.0);
        assert_eq!(finished.len(), 44 * 44 * 4);
    }
}
