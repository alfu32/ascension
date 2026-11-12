//! HUD text: a glyph atlas baked with `fontdue` from the bundled DejaVu Sans Bold, plus
//! measuring and layout with left, centre and right alignment.
//!
//! Glyphs are rasterised white with coverage in the alpha channel and drawn tinted. Advances are
//! whole pixels and every digit has the same advance, so changing numbers do not jitter.

use std::collections::HashMap;

use fontdue::{Font, FontSettings};

/// The bundled font file.
const FONT_BYTES: &[u8] = include_bytes!("../../../assets/fonts/DejaVuSans-Bold.ttf");
/// Pixel size of [`TextSize::Small`] in design pixels.
const SMALL_DESIGN: f32 = 12.0;
/// Pixel size of [`TextSize::Large`] in design pixels.
const LARGE_DESIGN: f32 = 16.0;
const ATLAS_WIDTH: u32 = 1024;
const PADDING: u32 = 1;

/// The two baked sizes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextSize {
    /// Labels and dense numbers (12 design pixels).
    Small,
    /// Amounts and headings (16 design pixels).
    Large,
}

impl TextSize {
    fn index(self) -> usize {
        match self {
            TextSize::Small => 0,
            TextSize::Large => 1,
        }
    }
}

/// Horizontal alignment relative to the anchor x.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    /// The text starts at the anchor.
    Left,
    /// The text is centred on the anchor.
    Centre,
    /// The text ends at the anchor.
    Right,
}

/// Appearance and alignment of one live text run.
#[derive(Clone, Copy, Debug)]
pub struct TextStyle {
    /// Baked size to use.
    pub size: TextSize,
    /// Alignment relative to the anchor.
    pub align: Align,
    /// RGBA tint for the white glyphs.
    pub colour: [u8; 4],
}

#[derive(Clone, Copy, Debug)]
struct Glyph {
    /// Atlas rectangle.
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    /// Offset of the bitmap's left edge from the pen.
    left: i32,
    /// Offset of the bitmap's top edge above the baseline.
    top: i32,
    advance: i32,
}

#[derive(Clone, Debug)]
struct SizeData {
    glyphs: HashMap<char, Glyph>,
    ascent: i32,
    line_height: i32,
}

/// One glyph to draw: its atlas rectangle and its destination in frame pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlyphQuad {
    /// Source rectangle in the atlas `(x, y, w, h)`.
    pub src: (u32, u32, u32, u32),
    /// Destination top-left corner in pixels.
    pub dst: (i32, i32),
    /// Straight RGBA text colour.
    pub colour: [u8; 4],
}

/// The baked glyphs of both sizes.
#[derive(Clone, Debug)]
pub struct GlyphAtlas {
    /// Width of the atlas image.
    pub width: u32,
    /// Height of the atlas image.
    pub height: u32,
    /// RGBA8 pixels: white with coverage as alpha.
    pub rgba: Vec<u8>,
    /// HUD scale the glyphs were rasterised at.
    pub scale: f32,
    sizes: [SizeData; 2],
}

fn characters() -> impl Iterator<Item = char> {
    (0x20_u32..=0x7e)
        .chain(0xa0..=0xff)
        .filter_map(char::from_u32)
}

impl GlyphAtlas {
    /// Rasterises Basic Latin and Latin-1 at both sizes for HUD scale `scale`.
    pub fn bake(scale: f32) -> Self {
        let font = Font::from_bytes(FONT_BYTES, FontSettings::default())
            .expect("the bundled font is a valid TrueType file");
        let mut bitmaps: Vec<(usize, char, fontdue::Metrics, Vec<u8>)> = Vec::new();
        let mut metrics_of = [(0, 0); 2];
        for (index, design) in [SMALL_DESIGN, LARGE_DESIGN].into_iter().enumerate() {
            let px = (design * scale).round().max(6.0);
            let line = font.horizontal_line_metrics(px);
            let ascent = line.map_or(px * 0.8, |l| l.ascent).round() as i32;
            let height = line.map_or(px * 1.2, |l| l.new_line_size).round() as i32;
            metrics_of[index] = (ascent, height);
            for ch in characters() {
                let (metrics, bitmap) = font.rasterize(ch, px);
                bitmaps.push((index, ch, metrics, bitmap));
            }
        }
        // Digits share the widest digit advance.
        let digit_advance = |index: usize| {
            bitmaps
                .iter()
                .filter(|(i, ch, ..)| *i == index && ch.is_ascii_digit())
                .map(|(_, _, m, _)| m.advance_width.round() as i32)
                .max()
                .unwrap_or(0)
        };
        let digits = [digit_advance(0), digit_advance(1)];

        let mut placements = Vec::with_capacity(bitmaps.len());
        let (mut x, mut y, mut row) = (PADDING, PADDING, 0);
        for (_, _, metrics, _) in &bitmaps {
            let (w, h) = (metrics.width as u32, metrics.height as u32);
            if x + w + PADDING > ATLAS_WIDTH {
                x = PADDING;
                y += row + PADDING;
                row = 0;
            }
            placements.push((x, y));
            x += w + PADDING;
            row = row.max(h);
        }
        let height = y + row + PADDING;
        let mut rgba = vec![0_u8; ATLAS_WIDTH as usize * height as usize * 4];
        let mut sizes = [0, 1].map(|index| SizeData {
            glyphs: HashMap::new(),
            ascent: metrics_of[index].0,
            line_height: metrics_of[index].1,
        });
        for ((index, ch, metrics, bitmap), (px, py)) in bitmaps.iter().zip(placements) {
            for row in 0..metrics.height {
                for col in 0..metrics.width {
                    let coverage = bitmap[row * metrics.width + col];
                    let at = ((py as usize + row) * ATLAS_WIDTH as usize + px as usize + col) * 4;
                    rgba[at..at + 4].copy_from_slice(&[255, 255, 255, coverage]);
                }
            }
            let advance = if ch.is_ascii_digit() {
                digits[*index]
            } else {
                metrics.advance_width.round() as i32
            };
            sizes[*index].glyphs.insert(
                *ch,
                Glyph {
                    x: px,
                    y: py,
                    w: metrics.width as u32,
                    h: metrics.height as u32,
                    left: metrics.xmin,
                    top: metrics.ymin + metrics.height as i32,
                    advance,
                },
            );
        }
        Self {
            width: ATLAS_WIDTH,
            height,
            rgba,
            scale,
            sizes,
        }
    }

    fn glyph(&self, size: TextSize, ch: char) -> Option<&Glyph> {
        let data = &self.sizes[size.index()];
        data.glyphs.get(&ch).or_else(|| data.glyphs.get(&'?'))
    }

    /// Distance from the top of a line to its baseline, in pixels.
    pub fn ascent(&self, size: TextSize) -> i32 {
        self.sizes[size.index()].ascent
    }

    /// Distance between two baselines, in pixels.
    pub fn line_height(&self, size: TextSize) -> i32 {
        self.sizes[size.index()].line_height
    }

    /// Width of `text` in pixels: the sum of the advances of its glyphs.
    pub fn measure(&self, text: &str, size: TextSize) -> i32 {
        text.chars()
            .filter_map(|ch| self.glyph(size, ch))
            .map(|glyph| glyph.advance)
            .sum()
    }

    /// Appends tinted glyph quads at the anchor and baseline, returning the pen end.
    pub fn draw(
        &self,
        text: &str,
        style: TextStyle,
        x: i32,
        baseline: i32,
        out: &mut Vec<GlyphQuad>,
    ) -> i32 {
        let start = out.len();
        let end = self.layout(text, style.size, style.align, x, baseline, out);
        for q in &mut out[start..] {
            q.colour = style.colour;
        }
        end
    }

    /// Lays `text` out with its baseline at `baseline` and appends the glyph quads to `out`.
    ///
    /// Returns the x position after the last glyph (the pen end). With [`Align::Right`] that is
    /// `x`, with [`Align::Left`] it is `x` plus the width.
    pub fn layout(
        &self,
        text: &str,
        size: TextSize,
        align: Align,
        x: i32,
        baseline: i32,
        out: &mut Vec<GlyphQuad>,
    ) -> i32 {
        let width = self.measure(text, size);
        let mut pen = match align {
            Align::Left => x,
            Align::Centre => x - width / 2,
            Align::Right => x - width,
        };
        for ch in text.chars() {
            let Some(glyph) = self.glyph(size, ch) else {
                continue;
            };
            if glyph.w > 0 && glyph.h > 0 {
                out.push(GlyphQuad {
                    src: (glyph.x, glyph.y, glyph.w, glyph.h),
                    dst: (pen + glyph.left, baseline - glyph.top),
                    colour: [255; 4],
                });
            }
            pen += glyph.advance;
        }
        pen
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measure_matches_sum_of_advances() {
        let atlas = GlyphAtlas::bake(1.0);
        for size in [TextSize::Small, TextSize::Large] {
            let text = "FOOD 4,280 +20";
            let sum: i32 = text
                .chars()
                .map(|ch| atlas.glyph(size, ch).expect("glyph").advance)
                .sum();
            assert_eq!(atlas.measure(text, size), sum);
            assert!(sum > 0);
            assert_eq!(atlas.measure("", size), 0);
        }
        // Digits are tabular: any two numbers of equal length measure the same.
        assert_eq!(
            atlas.measure("1111", TextSize::Large),
            atlas.measure("8080", TextSize::Large)
        );
        // Latin-1 text is covered.
        assert!(atlas.measure("Café ×", TextSize::Small) > 0);
        // The larger size is wider.
        assert!(atlas.measure("ABC", TextSize::Large) > atlas.measure("ABC", TextSize::Small));
    }

    #[test]
    fn right_aligned_numbers_end_at_the_anchor() {
        let atlas = GlyphAtlas::bake(1.3);
        for text in ["5", "320", "1,240", "1,000,000"] {
            let mut quads = Vec::new();
            let end = atlas.layout(text, TextSize::Large, Align::Right, 500, 40, &mut quads);
            assert_eq!(end, 500, "{text}");
            let first = atlas.layout(text, TextSize::Large, Align::Left, 0, 40, &mut Vec::new());
            assert_eq!(first, atlas.measure(text, TextSize::Large));
            for quad in &quads {
                assert!(quad.dst.0 + quad.src.2 as i32 <= 502);
            }
        }
        // Equal-length numbers occupy the same span, so digits do not shift as they change.
        let mut a = Vec::new();
        let mut b = Vec::new();
        atlas.layout("1,111", TextSize::Small, Align::Right, 100, 20, &mut a);
        atlas.layout("9,876", TextSize::Small, Align::Right, 100, 20, &mut b);
        assert_eq!(a.len(), b.len());
        let comma_a = a.iter().find(|q| q.src.3 < 10).map(|q| q.dst.0);
        let comma_b = b.iter().find(|q| q.src.3 < 10).map(|q| q.dst.0);
        assert_eq!(comma_a, comma_b);
    }

    #[test]
    fn scale_changes_pixel_size() {
        let small = GlyphAtlas::bake(0.5);
        let big = GlyphAtlas::bake(2.0);
        assert!(big.line_height(TextSize::Large) > small.line_height(TextSize::Large));
        assert!(big.ascent(TextSize::Small) > small.ascent(TextSize::Small));
    }
}
