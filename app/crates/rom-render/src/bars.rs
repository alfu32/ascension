//! Health bars drawn on the 2D batch: one coloured segment and one dark segment side by side,
//! in whole pixels, over the finished 3D picture. Nothing is drawn under or over anything else,
//! so there is nothing to fight in the depth buffer, and bars are never shaded or shadowed.

use crate::pick::View;
use raylib::prelude::{Color, RaylibDraw};

/// Narrowest and widest bar in pixels at HUD scale 1.
pub(crate) const MIN_WIDTH: f32 = 18.0;
/// See [`MIN_WIDTH`].
pub(crate) const MAX_WIDTH: f32 = 90.0;
/// Height of the bar in pixels at HUD scale 1, outline excluded.
pub(crate) const HEIGHT: i32 = 4;
/// Colour of the part of the bar that is gone (the outline around the bar is pure black).
pub(crate) const EMPTY: [u8; 3] = [38, 40, 46];
/// Colour at full health, half health and none.
const STOPS: [[u8; 3]; 3] = [[230, 51, 38], [242, 200, 51], [64, 217, 77]];

/// Where to hang a bar in the world: a point above an entity and how wide the entity is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BarAnchor {
    /// The entity's id, to match the selection.
    pub(crate) id: u32,
    /// The point above the entity's top, in world space.
    pub(crate) at: [f32; 3],
    /// The entity's width in world units: the bar's width follows it on screen.
    pub(crate) width: f32,
    /// Health, 0 to 1.
    pub(crate) frac: f32,
    /// Distance from the planet's centre to the ground under the entity when the ground is a
    /// sphere centred on the origin; a bar behind that sphere is hidden.
    pub(crate) ground_radius: Option<f32>,
}

/// A bar placed on the picture: the centre of its top edge's middle in pixels and its width.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ScreenBar {
    pub(crate) x: f32,
    pub(crate) y: f32,
    /// Width in pixels, already clamped.
    pub(crate) width: f32,
    pub(crate) frac: f32,
    /// Opacity, 255 for a bar that is fully shown.
    pub(crate) alpha: u8,
}

/// The colour for health `frac`: green at 1, yellow at a half, red at 0, blended in between.
pub(crate) fn health_colour(frac: f32) -> [u8; 3] {
    let f = frac.clamp(0.0, 1.0) * 2.0;
    let (lo, hi, t) = if f < 1.0 {
        (STOPS[0], STOPS[1], f)
    } else {
        (STOPS[1], STOPS[2], f - 1.0)
    };
    std::array::from_fn(|i| {
        (f32::from(lo[i]) + (f32::from(hi[i]) - f32::from(lo[i])) * t).round() as u8
    })
}

/// An integer rectangle in pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Px {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) w: i32,
    pub(crate) h: i32,
}

/// The pieces of one bar: the coloured part from the left end, the dark rest to the right end,
/// and four outline edges outside both.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Pieces {
    pub(crate) fill: Px,
    pub(crate) rest: Px,
    pub(crate) outline: [Px; 4],
}

/// Splits a bar centred at (`cx`, `cy`) into pixel rectangles: `width` pixels wide (at least 2),
/// `height` pixels tall, with an outline of `edge` pixels outside. The fill and the rest
/// together are exactly the width and never overlap; a bar with any health shows at least one
/// pixel of fill, and one with any damage shows at least one of rest.
pub(crate) fn split(cx: f32, cy: f32, width: f32, frac: f32, height: i32, edge: i32) -> Pieces {
    let w = (width.round() as i32).max(2);
    let x0 = (cx - w as f32 / 2.0).round() as i32;
    let y0 = (cy - height as f32 / 2.0).round() as i32;
    let frac = frac.clamp(0.0, 1.0);
    let mut filled = (w as f32 * frac).round() as i32;
    if frac > 0.0 {
        filled = filled.max(1);
    }
    if frac < 1.0 {
        filled = filled.min(w - 1);
    }
    let rect = |x, y, w, h| Px { x, y, w, h };
    Pieces {
        fill: rect(x0, y0, filled, height),
        rest: rect(x0 + filled, y0, w - filled, height),
        outline: [
            rect(x0 - edge, y0 - edge, w + 2 * edge, edge),
            rect(x0 - edge, y0 + height, w + 2 * edge, edge),
            rect(x0 - edge, y0, edge, height),
            rect(x0 + w, y0, edge, height),
        ],
    }
}

/// Whether the straight line from `eye` to `point` passes through the sphere at `centre`.
pub(crate) fn behind_sphere(eye: [f32; 3], point: [f32; 3], centre: [f32; 3], radius: f32) -> bool {
    let d: [f32; 3] = std::array::from_fn(|i| point[i] - eye[i]);
    let m: [f32; 3] = std::array::from_fn(|i| eye[i] - centre[i]);
    let a: f32 = d.iter().map(|c| c * c).sum();
    if a < 1e-12 {
        return false;
    }
    let b: f32 = (0..3).map(|i| m[i] * d[i]).sum();
    let c: f32 = m.iter().map(|c| c * c).sum::<f32>() - radius * radius;
    // The eye inside the sphere sees past nothing; otherwise the point is hidden when the
    // segment enters the sphere before it reaches the point.
    let disc = b * b - a * c;
    c > 0.0 && disc > 0.0 && (-b - disc.sqrt()) / a < 1.0 && -b > 0.0
}

/// Places the bars of the anchors that `wanted` selects on `view`: hidden behind the camera,
/// off the picture and (for a round ground) behind the planet. `scale` is the HUD scale.
pub(crate) fn project(
    anchors: &[BarAnchor],
    wanted: &dyn Fn(&BarAnchor) -> bool,
    view: &View,
    scale: f32,
    out: &mut Vec<ScreenBar>,
) {
    let (w, h) = view.size();
    let per_unit = h / (2.0 * view.tan());
    let eye = view.eye();
    for a in anchors.iter().filter(|a| wanted(a)) {
        if a.ground_radius
            .is_some_and(|r| behind_sphere(eye, a.at, [0.0; 3], r * 0.999))
        {
            continue;
        }
        let Some((x, y, depth)) = view.project(a.at) else {
            continue;
        };
        let width = (a.width * per_unit / depth).clamp(MIN_WIDTH, MAX_WIDTH) * scale;
        if x + width / 2.0 < 0.0 || x - width / 2.0 > w || y < 0.0 || y > h {
            continue;
        }
        out.push(ScreenBar {
            x,
            y,
            width,
            frac: a.frac,
            alpha: 255,
        });
    }
}

/// Draws every bar as plain rectangles in one pass.
pub(crate) fn draw(d: &mut impl RaylibDraw, bars: &[ScreenBar], scale: f32) {
    let height = (HEIGHT as f32 * scale).round().max(1.0) as i32;
    let edge = scale.round().max(1.0) as i32;
    for bar in bars {
        let pieces = split(bar.x, bar.y, bar.width, bar.frac, height, edge);
        let [r, g, b] = health_colour(bar.frac);
        let mut paint = |p: Px, c: [u8; 3]| {
            if p.w > 0 && p.h > 0 {
                d.draw_rectangle(p.x, p.y, p.w, p.h, Color::new(c[0], c[1], c[2], bar.alpha));
            }
        };
        for edge in pieces.outline {
            paint(edge, [0, 0, 0]);
        }
        paint(pieces.rest, EMPTY);
        paint(pieces.fill, [r, g, b]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colour_runs_green_to_red_through_yellow() {
        assert_eq!(health_colour(1.0), STOPS[2]);
        assert_eq!(health_colour(0.5), STOPS[1]);
        assert_eq!(health_colour(0.0), STOPS[0]);
        assert_eq!(health_colour(2.0), STOPS[2]);
        assert_eq!(health_colour(-1.0), STOPS[0]);
        // Continuous: neighbouring health levels differ by a few steps only.
        let mut last = health_colour(0.0);
        for i in 1..=100 {
            let c = health_colour(i as f32 / 100.0);
            for k in 0..3 {
                assert!(i32::from(c[k]).abs_diff(i32::from(last[k])) <= 8);
            }
            last = c;
        }
    }

    #[test]
    fn segments_fill_the_width_without_overlap() {
        for width in [18.0, 19.0, 45.5, 90.0] {
            for i in 0..=40 {
                let p = split(300.0, 200.0, width, i as f32 / 40.0, 4, 1);
                assert_eq!(p.fill.x + p.fill.w, p.rest.x, "touching, no overlap");
                assert_eq!(p.fill.w + p.rest.w, width.round() as i32);
                assert_eq!((p.fill.y, p.fill.h), (p.rest.y, p.rest.h));
                // The outline lies outside the bar.
                let bar = Px {
                    x: p.fill.x,
                    y: p.fill.y,
                    w: p.fill.w + p.rest.w,
                    h: 4,
                };
                for e in p.outline {
                    let apart = e.x + e.w <= bar.x
                        || e.x >= bar.x + bar.w
                        || e.y + e.h <= bar.y
                        || e.y >= bar.y + bar.h;
                    assert!(apart, "{e:?} overlaps the bar");
                }
            }
        }
    }

    #[test]
    fn empty_and_full_bars_are_one_segment() {
        let none = split(100.0, 50.0, 40.0, 0.0, 4, 1);
        assert_eq!((none.fill.w, none.rest.w), (0, 40));
        let all = split(100.0, 50.0, 40.0, 1.0, 4, 1);
        assert_eq!((all.fill.w, all.rest.w), (40, 0));
        let sliver = split(100.0, 50.0, 40.0, 0.001, 4, 1);
        assert_eq!(sliver.fill.w, 1);
        let nearly = split(100.0, 50.0, 40.0, 0.999, 4, 1);
        assert_eq!(nearly.rest.w, 1);
    }

    #[test]
    fn a_bar_behind_the_planet_is_hidden() {
        let eye = [0.0, 0.0, 30.0];
        let near = [0.0, 10.5, 9.0];
        let far = [0.0, 0.0, -10.5];
        assert!(!behind_sphere(eye, [0.0, 0.0, 10.5], [0.0; 3], 10.0));
        assert!(behind_sphere(eye, far, [0.0; 3], 10.0));
        assert!(!behind_sphere(eye, near, [0.0; 3], 10.0));
    }

    fn view() -> View {
        View::looking(
            [0.0, 0.0, 30.0],
            [0.0; 3],
            [0.0, 1.0, 0.0],
            (800.0, 600.0),
            0.0,
        )
    }

    fn anchor(at: [f32; 3], ground_radius: Option<f32>) -> BarAnchor {
        BarAnchor {
            id: 1,
            at,
            width: 1.0,
            frac: 0.5,
            ground_radius,
        }
    }

    #[test]
    fn hidden_behind_the_camera_off_screen_and_past_the_horizon() {
        let mut out = Vec::new();
        let all = |_: &BarAnchor| true;
        let anchors = [
            anchor([0.0, 0.0, 10.5], Some(10.0)),
            anchor([0.0, 0.0, 40.0], None),
            anchor([500.0, 0.0, 0.0], None),
            anchor([0.0, 0.0, -10.5], Some(10.0)),
        ];
        project(&anchors, &all, &view(), 1.0, &mut out);
        assert_eq!(out.len(), 1, "{out:?}");
        assert!((out[0].x - 400.0).abs() < 1.0);
        assert!((MIN_WIDTH..=MAX_WIDTH).contains(&out[0].width));
    }
}
