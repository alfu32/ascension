//! Energy Shield domes (task 157), built into the overlay geometry from the HUD snapshot: a
//! translucent dome over what a city or a fort covers, as bright as its pool is full and
//! shimmering slowly, with a brighter rim on the ground, high enough to clear the building under
//! it; and a flare where a blow struck it, on the side facing where the blow came from: a hot
//! spot and a ripple spreading over the dome. Nothing here allocates: the points are made as they
//! are pushed.

use super::{OverlayGeometry, Tri3};
use crate::entities::{EntityView, Footing, building_height};
use crate::hud::HudSnapshot;
use crate::{HudShield, HudShieldHit};
use std::f32::consts::{FRAC_PI_2, TAU};

/// Segments round a dome and rings from its rim to its top.
const AROUND: usize = 40;
const RINGS: usize = 8;
/// A dome's height as a share of its reach, and at most this many tiles; at least this much
/// higher than the building under it, and never higher than its reach.
const HEIGHT_SHARE: f32 = 0.4;
const MAX_HEIGHT: f32 = 9.0;
const CLEARANCE: f32 = 1.2;
/// Where along the way from the struck point to the rim, toward the blow, the dome flares.
const FLARE_ALONG: f32 = 0.6;
/// The dome's colour.
const DOME: [u8; 3] = [110, 205, 255];
/// Seconds a flare lasts and how far its ripple spreads, tiles.
const FLARE_SECS: f32 = 1.2;
const RIPPLE: f32 = 3.5;
/// Segments of a ripple, and sparks round a flare.
const RIPPLE_SEGMENTS: usize = 16;
const SPARKS: usize = 10;

fn rgba(rgb: [u8; 3], alpha: f32) -> [u8; 4] {
    [
        rgb[0],
        rgb[1],
        rgb[2],
        (alpha.clamp(0.0, 1.0) * 255.0) as u8,
    ]
}

fn add(a: [f32; 3], b: [f32; 3], s: f32) -> [f32; 3] {
    std::array::from_fn(|i| a[i] + b[i] * s)
}

/// The height at its top of a dome of `reach` tiles over a building `building` tiles high.
fn top(reach: f32, building: f32) -> f32 {
    (reach * HEIGHT_SHARE)
        .min(MAX_HEIGHT)
        .max(building * CLEARANCE)
        .min(reach)
}

/// The height of the building at the centre of a dome, by the shield building's id.
fn building_at(views: &[EntityView], id: u32) -> f32 {
    views
        .binary_search_by_key(&id, |v| v.id)
        .map_or(0.0, |i| building_height(&views[i]))
}

/// Draws every dome and flare of the frame.
pub(super) fn domes(
    out: &mut OverlayGeometry,
    ground: &dyn Footing,
    right: [f32; 3],
    views: &[EntityView],
    hud: &HudSnapshot,
) {
    let tps = hud.ticks_per_second.max(1) as f32;
    let clock = hud.game_ticks as f32 / tps;
    for shield in &hud.shields {
        dome(out, ground, shield, building_at(views, shield.id), clock);
    }
    for hit in &hud.shield_hits {
        let building = building_at(views, hit.id);
        flare(
            out,
            ground,
            right,
            hit,
            building,
            hit.age_ticks as f32 / tps,
        );
    }
}

fn dome(out: &mut OverlayGeometry, ground: &dyn Footing, s: &HudShield, building: f32, clock: f32) {
    let charge = (f32::from(s.charge) / 1000.0).clamp(0.0, 1.0);
    if charge <= 0.0 || s.reach <= 0 {
        return;
    }
    let (face, cx, cy) = (s.face, s.x as f32 / 1000.0, s.y as f32 / 1000.0);
    let reach = s.reach as f32 / 1000.0;
    let base = ground.height_at(face, cx, cy);
    let high = top(reach, building);
    let point = |ring: usize, k: usize| {
        let latitude = ring as f32 / RINGS as f32 * FRAC_PI_2;
        let (across, up) = (reach * latitude.cos(), high * latitude.sin());
        let angle = (k % AROUND) as f32 / AROUND as f32 * TAU;
        ground.datum_point(
            face,
            cx,
            cy,
            (across * angle.cos(), across * angle.sin()),
            base + up,
        )
    };
    // A slow shimmer runs round the dome; a weak pool shows faint.
    let phase = clock * 0.8 + (s.id % 97) as f32 * 0.31;
    for ring in 0..RINGS {
        // Clearer near the top, denser toward the rim where the eye looks through more of it.
        let density = 0.10 + 0.10 * (1.0 - ring as f32 / RINGS as f32);
        for k in 0..AROUND {
            let wave = 0.75 + 0.25 * (phase + k as f32 * TAU * 3.0 / AROUND as f32).sin();
            let colour = rgba(DOME, density * wave * (0.35 + 0.65 * charge));
            let (a, b, c, d) = (
                point(ring, k),
                point(ring, k + 1),
                point(ring + 1, k + 1),
                point(ring + 1, k),
            );
            out.glass.push(Tri3 {
                v: [a, b, c],
                colour,
            });
            out.glass.push(Tri3 {
                v: [a, c, d],
                colour,
            });
        }
    }
    // The rim on the ground and two bands of latitude, bright lines.
    for (ring, alpha) in [(0, 0.9), (3, 0.35), (6, 0.25)] {
        let colour = rgba([170, 230, 255], alpha * (0.4 + 0.6 * charge));
        for k in 0..AROUND {
            out.line(point(ring, k), point(ring, k + 1), colour);
        }
    }
}

/// Where on the dome of `reach` tiles a blow on the point `struck` (tiles from its centre)
/// that came from `from` flares: along the ground from the struck point toward the blow,
/// [`FLARE_ALONG`] of the way to the rim, then up to the dome; above the struck point when the
/// blow's origin is unknown.
fn flare_spot(reach: f32, struck: (f32, f32), from: Option<(f32, f32)>) -> (f32, f32) {
    let length = (struck.0 * struck.0 + struck.1 * struck.1).sqrt();
    let inside = (reach * 0.9).max(0.0);
    let s = if length > inside {
        (struck.0 * inside / length, struck.1 * inside / length)
    } else {
        struck
    };
    let Some(f) = from else {
        return s;
    };
    let (dx, dy) = (f.0 - s.0, f.1 - s.1);
    let d = (dx * dx + dy * dy).sqrt();
    if d < 0.01 {
        return s;
    }
    let (ux, uy) = (dx / d, dy / d);
    // The way along (ux, uy) from s to the rim: |s + u t| = reach.
    let b = s.0 * ux + s.1 * uy;
    let c = s.0 * s.0 + s.1 * s.1 - reach * reach;
    let t = -b + (b * b - c).max(0.0).sqrt();
    (s.0 + ux * t * FLARE_ALONG, s.1 + uy * t * FLARE_ALONG)
}

fn flare(
    out: &mut OverlayGeometry,
    ground: &dyn Footing,
    right: [f32; 3],
    hit: &HudShieldHit,
    building: f32,
    t: f32,
) {
    if t >= FLARE_SECS || hit.reach <= 0 {
        return;
    }
    let age = t / FLARE_SECS;
    let fade = 1.0 - age;
    let reach = hit.reach as f32 / 1000.0;
    let (face, cx, cy) = (hit.face, hit.x as f32 / 1000.0, hit.y as f32 / 1000.0);
    let base = ground.height_at(face, cx, cy);
    let tiles = |p: (i32, i32)| (p.0 as f32 / 1000.0, p.1 as f32 / 1000.0);
    let spot = flare_spot(reach, tiles(hit.at), hit.from.map(tiles));
    let share = ((spot.0 * spot.0 + spot.1 * spot.1).sqrt() / reach.max(0.01)).clamp(0.0, 1.0);
    let lift = (top(reach, building) * (1.0 - share * share).sqrt()).max(0.5);
    let at = ground.datum_point(face, cx, cy, spot, base + lift);
    let up = ground.frame(face, cx, cy).up;
    // The hot spot: a small bright diamond facing the camera inside a pale blue one, and sparks
    // thrown off round it.
    for (size, rgb, alpha) in [
        (0.9 * (1.0 - 0.5 * age), DOME, 0.45 * fade),
        (0.35 * (1.0 - 0.5 * age), [240, 252, 255], 0.95 * fade),
    ] {
        out.quad(
            [
                add(at, right, -size),
                add(at, up, -size),
                add(at, right, size),
                add(at, up, size),
            ],
            rgba(rgb, alpha),
        );
    }
    let spark = rgba([200, 240, 255], 0.9 * fade);
    for i in 0..SPARKS {
        let angle = (i as f32 + 0.5) / SPARKS as f32 * TAU;
        let (inner, outer) = (0.3 + 0.6 * age, 0.8 + 1.6 * age);
        let ray = |r: f32| add(add(at, right, r * angle.cos()), up, r * angle.sin());
        out.line(ray(inner), ray(outer), spark);
    }
    // The ripple spreading from it.
    let radius = 0.6 + RIPPLE * age;
    let colour = rgba(DOME, 0.8 * fade);
    let ring = |i: usize| {
        let angle = (i % RIPPLE_SEGMENTS) as f32 / RIPPLE_SEGMENTS as f32 * TAU;
        add(
            add(at, right, radius * angle.cos()),
            up,
            radius * angle.sin(),
        )
    };
    for i in 0..RIPPLE_SEGMENTS {
        out.line(ring(i), ring(i + 1), colour);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dome_flares_on_its_side_facing_the_blow() {
        // A blow on the building at the centre from the east flares east of it.
        let (x, y) = flare_spot(10.0, (0.0, 0.0), Some((30.0, 0.0)));
        assert!((x - 6.0).abs() < 1e-4 && y.abs() < 1e-4, "{x} {y}");
        // A blow on a point near the rim from beyond it flares between it and the rim.
        let (x, _) = flare_spot(10.0, (8.0, 0.0), Some((20.0, 0.0)));
        assert!(x > 8.0 && x < 10.0, "{x}");
        // Unknown origin: above the struck point, kept inside the dome.
        assert_eq!(flare_spot(10.0, (3.0, 4.0), None), (3.0, 4.0));
        let (x, y) = flare_spot(10.0, (30.0, 40.0), None);
        assert!(((x * x + y * y).sqrt() - 9.0).abs() < 1e-3);
    }

    #[test]
    fn a_dome_clears_the_building_under_it() {
        assert!(top(6.5, 4.0) > 4.0);
        assert!(top(6.5, 20.0) <= 6.5);
        assert_eq!(top(40.0, 1.0), MAX_HEIGHT);
    }
}
