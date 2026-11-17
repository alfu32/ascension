//! Bombardment effects (task 75), built into the overlay geometry from the HUD snapshot: the
//! flash, light column, secondary bursts, shockwave, fireball and debris of a blast, the
//! scorched ground it leaves for a minute, buildings that collapse when they fall instead of
//! vanishing, and fire and smoke over ruins and badly damaged buildings.

use super::{LIFT, OverlayGeometry};
use crate::entities::{EntityView, Footing, building_height, owner_color};
use crate::hud::HudSnapshot;
use crate::{HudBlast, HudWreck};

/// Seconds a flash and its light column last.
const FLASH_SECS: f32 = 0.6;
/// Seconds the shockwave takes to reach the edge of the blast.
const WAVE_SECS: f32 = 1.2;
/// Seconds the fireball burns.
const FIREBALL_SECS: f32 = 2.5;
/// Seconds debris flies.
const DEBRIS_SECS: f32 = 2.2;
/// Seconds the scorched ground takes to fade.
const SCORCH_SECS: f32 = 60.0;
/// Seconds a falling building takes to sink into its foundations (the views cut its model lower
/// over the same time).
const COLLAPSE_SECS: f32 = 2.0;
/// Seconds a ruin burns.
const BURN_SECS: f32 = 45.0;
/// Secondary bursts of a blast and debris pieces.
const BURSTS: u32 = 6;
const DEBRIS: u32 = 18;
/// Segments of the scorch disc and the shockwave.
const SEGMENTS: usize = 40;

/// A number in [0, 1) from a seed and an index, the same every frame.
fn noise(seed: u32, index: u32) -> f32 {
    let mut x = seed
        .wrapping_mul(0x9E37_79B1)
        .wrapping_add(index.wrapping_mul(0x85EB_CA77));
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    x = x.wrapping_mul(0x297A_2D39);
    x ^= x >> 15;
    (x >> 8) as f32 / (1u32 << 24) as f32
}

fn rgba(rgb: [u8; 3], alpha: f32) -> [u8; 4] {
    [
        rgb[0],
        rgb[1],
        rgb[2],
        (alpha.clamp(0.0, 1.0) * 255.0) as u8,
    ]
}

fn mix(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    let t = t.clamp(0.0, 1.0);
    std::array::from_fn(|i| (f32::from(a[i]) + (f32::from(b[i]) - f32::from(a[i])) * t) as u8)
}

fn ease_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t) * (1.0 - t)
}

fn add(a: [f32; 3], b: [f32; 3], s: f32) -> [f32; 3] {
    std::array::from_fn(|i| a[i] + b[i] * s)
}

/// Draws every bombardment effect of the frame.
pub(super) fn bombardment(
    out: &mut OverlayGeometry,
    ground: &dyn Footing,
    right: [f32; 3],
    views: &[EntityView],
    hud: &HudSnapshot,
) {
    let tps = hud.ticks_per_second.max(1) as f32;
    let clock = hud.game_ticks as f32 / tps;
    for blast in &hud.blasts {
        draw_blast(out, ground, right, blast, blast.age_ticks as f32 / tps);
    }
    for wreck in &hud.wrecks {
        draw_wreck(
            out,
            ground,
            right,
            wreck,
            wreck.age_ticks as f32 / tps,
            clock,
        );
    }
    // Ruined and badly hurt buildings still standing burn and smoke.
    for view in views.iter().filter(|v| {
        v.building && v.altitude == 0.0 && v.under_construction.is_none() && v.hp_frac < 0.35
    }) {
        let strength = if view.hp_frac <= 0.0 {
            1.0
        } else {
            0.4 + (0.35 - view.hp_frac) * 1.5
        };
        let seed = view.id.wrapping_mul(2_654_435_761);
        let half = (
            f32::from(view.footprint.0.max(1)) / 2.0,
            f32::from(view.footprint.1.max(1)) / 2.0,
        );
        let height = if view.hp_frac <= 0.0 {
            building_height(view) * 0.35
        } else {
            building_height(view) * 0.9
        };
        let blaze = Blaze {
            at: (view.face, view.x, view.y),
            half,
            height,
            strength,
            seed,
        };
        fires(out, ground, right, &blaze, clock);
    }
}

/// Segments of a puff of light, fire, smoke or dust.
const PUFF_SEGMENTS: usize = 12;

/// A camera-facing oval of `width` by `height` standing on `base`, fading to half its
/// opacity at the rim.
fn puff(
    out: &mut OverlayGeometry,
    base: [f32; 3],
    right: [f32; 3],
    up: [f32; 3],
    width: f32,
    height: f32,
    colour: [u8; 4],
) {
    let centre = add(base, up, height / 2.0);
    let rim = [colour[0], colour[1], colour[2], colour[3] / 2];
    let point = |i: usize| {
        let angle = i as f32 / PUFF_SEGMENTS as f32 * std::f32::consts::TAU;
        add(
            add(centre, right, angle.cos() * width / 2.0),
            up,
            angle.sin() * height / 2.0,
        )
    };
    let inner = |i: usize| {
        let angle = i as f32 / PUFF_SEGMENTS as f32 * std::f32::consts::TAU;
        add(
            add(centre, right, angle.cos() * width * 0.3),
            up,
            angle.sin() * height * 0.3,
        )
    };
    for i in 0..PUFF_SEGMENTS {
        let j = (i + 1) % PUFF_SEGMENTS;
        out.tri([centre, inner(i), inner(j)], colour);
        out.quad([inner(i), point(i), point(j), inner(j)], rim);
    }
}

/// A tongue of flame `width` across and `height` tall standing on `base`.
fn flame(
    out: &mut OverlayGeometry,
    base: [f32; 3],
    right: [f32; 3],
    up: [f32; 3],
    width: f32,
    height: f32,
    colour: [u8; 4],
) {
    let left = add(add(base, right, -width / 2.0), up, height * 0.3);
    let rightmost = add(add(base, right, width / 2.0), up, height * 0.3);
    let tip = add(base, up, height);
    out.tri([base, rightmost, tip], colour);
    out.tri([base, tip, left], colour);
}

/// A camera-facing quad of `width` by `height` standing on `base`.
fn billboard(
    out: &mut OverlayGeometry,
    base: [f32; 3],
    right: [f32; 3],
    up: [f32; 3],
    width: f32,
    height: f32,
    colour: [u8; 4],
) {
    let a = add(base, right, -width / 2.0);
    let b = add(base, right, width / 2.0);
    out.quad([a, b, add(b, up, height), add(a, up, height)], colour);
}

fn draw_blast(
    out: &mut OverlayGeometry,
    ground: &dyn Footing,
    right: [f32; 3],
    blast: &HudBlast,
    t: f32,
) {
    let (face, cx, cy) = (blast.face, blast.x as f32 / 1000.0, blast.y as f32 / 1000.0);
    let radius = (blast.radius as f32 / 1000.0).max(0.5);
    let frame = ground.frame(face, cx, cy);
    let at = |dx: f32, dy: f32, lift: f32| ground.point(face, cx, cy, (dx, dy), lift);
    // Scorched ground: a dark disc, ragged at the rim, fading over a minute.
    if t < SCORCH_SECS {
        let fade = 1.0 - t / SCORCH_SECS;
        let scorch = radius * 0.75;
        let rim: Vec<(f32, f32)> = (0..SEGMENTS)
            .map(|i| {
                let angle = i as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
                let ragged = 0.8 + 0.3 * noise(blast.seed, 200 + i as u32);
                (scorch * ragged * angle.cos(), scorch * ragged * angle.sin())
            })
            .collect();
        let inner: Vec<(f32, f32)> = rim.iter().map(|(x, y)| (x * 0.45, y * 0.45)).collect();
        let centre = at(0.0, 0.0, LIFT * 0.6);
        let (dark, edge) = (
            rgba([18, 14, 12], 0.85 * fade),
            rgba([40, 30, 24], 0.55 * fade),
        );
        for i in 0..SEGMENTS {
            let j = (i + 1) % SEGMENTS;
            let (a, b) = (inner[i], inner[j]);
            out.tri(
                [centre, at(a.0, a.1, LIFT * 0.6), at(b.0, b.1, LIFT * 0.6)],
                dark,
            );
            let (c, d) = (rim[i], rim[j]);
            out.quad(
                [
                    at(a.0, a.1, LIFT * 0.6),
                    at(c.0, c.1, LIFT * 0.6),
                    at(d.0, d.1, LIFT * 0.6),
                    at(b.0, b.1, LIFT * 0.6),
                ],
                edge,
            );
        }
        // Embers glow in the crater for the first seconds.
        if t < 8.0 {
            let glow = 1.0 - t / 8.0;
            for i in 0..12u32 {
                let angle = noise(blast.seed, 300 + i) * std::f32::consts::TAU;
                let distance = noise(blast.seed, 320 + i) * radius * 0.5;
                let (dx, dy) = (distance * angle.cos(), distance * angle.sin());
                let size = 0.25 + 0.35 * noise(blast.seed, 340 + i);
                out.quad(
                    [
                        at(dx - size, dy, LIFT),
                        at(dx, dy - size, LIFT),
                        at(dx + size, dy, LIFT),
                        at(dx, dy + size, LIFT),
                    ],
                    rgba([255, 120, 30], 0.9 * glow),
                );
            }
        }
    }
    // The flash: a white dome of light and a column reaching back up to orbit.
    if t < FLASH_SECS {
        let k = t / FLASH_SECS;
        let fade = 1.0 - k;
        let size = radius * (0.35 + 0.35 * ease_out(k));
        for (width, height, colour) in [
            (size * 2.0, size * 1.4, rgba([255, 230, 170], 0.75 * fade)),
            (size * 1.2, size * 1.0, rgba([255, 255, 240], fade)),
        ] {
            puff(
                out,
                add(frame.origin, frame.up, -height * 0.3),
                right,
                frame.up,
                width,
                height,
                colour,
            );
        }
        let disc: Vec<(f32, f32)> = (0..SEGMENTS)
            .map(|i| {
                let angle = i as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
                (size * angle.cos(), size * angle.sin())
            })
            .collect();
        let centre = at(0.0, 0.0, LIFT * 1.5);
        for i in 0..SEGMENTS {
            let (a, b) = (disc[i], disc[(i + 1) % SEGMENTS]);
            out.tri(
                [centre, at(a.0, a.1, LIFT * 1.5), at(b.0, b.1, LIFT * 1.5)],
                rgba([255, 240, 200], 0.8 * fade),
            );
        }
        let column = radius * 0.12;
        billboard(
            out,
            frame.origin,
            right,
            frame.up,
            column * 2.5,
            40.0,
            rgba([255, 220, 160], 0.35 * fade),
        );
        billboard(
            out,
            frame.origin,
            right,
            frame.up,
            column,
            40.0,
            rgba([255, 255, 255], 0.9 * fade),
        );
    }
    // Secondary bursts: the rest of the volley striking around the centre.
    for i in 0..BURSTS {
        let start = 0.05 + 0.08 * i as f32;
        let k = (t - start) / 0.4;
        if !(0.0..1.0).contains(&k) {
            continue;
        }
        let angle = noise(blast.seed, i) * std::f32::consts::TAU;
        let distance = (0.25 + 0.4 * noise(blast.seed, 20 + i)) * radius;
        let (dx, dy) = (distance * angle.cos(), distance * angle.sin());
        let base = at(dx, dy, 0.0);
        let size = radius * 0.22 * (0.5 + ease_out(k));
        puff(
            out,
            base,
            right,
            frame.up,
            size * 1.6,
            size,
            rgba(mix([255, 250, 220], [255, 110, 30], k), 1.0 - k),
        );
    }
    // The shockwave: a bright ring running out to the edge of the blast, and a fainter one
    // behind it.
    if t < WAVE_SECS {
        let k = t / WAVE_SECS;
        for (lag, alpha) in [(0.0, 0.9), (0.18, 0.45)] {
            let reach = ease_out((k - lag).max(0.0)) * radius;
            if reach <= 0.05 {
                continue;
            }
            let width = (radius * 0.07).max(0.3) * (1.0 - k * 0.5);
            let colour = rgba(mix([255, 245, 210], [255, 150, 70], k), alpha * (1.0 - k));
            for i in 0..SEGMENTS {
                let a0 = i as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
                let a1 = (i + 1) as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
                let point =
                    |angle: f32, r: f32, lift: f32| at(r * angle.cos(), r * angle.sin(), lift);
                out.quad(
                    [
                        point(a0, reach - width, LIFT * 2.0),
                        point(a0, reach, LIFT * 2.0 + width * 0.6),
                        point(a1, reach, LIFT * 2.0 + width * 0.6),
                        point(a1, reach - width, LIFT * 2.0),
                    ],
                    colour,
                );
            }
        }
    }
    // The fireball rising and darkening into smoke.
    if t < FIREBALL_SECS {
        let k = t / FIREBALL_SECS;
        for i in 0..5u32 {
            let angle = noise(blast.seed, 40 + i) * std::f32::consts::TAU;
            let spread = radius * 0.2 * noise(blast.seed, 50 + i);
            let base = add(
                at(spread * angle.cos(), spread * angle.sin(), 0.0),
                frame.up,
                radius * 0.4 * ease_out(k) * (0.6 + noise(blast.seed, 60 + i)),
            );
            let size = radius * (0.25 + 0.3 * k) * (0.7 + 0.5 * noise(blast.seed, 70 + i));
            let colour = mix([255, 170, 60], [70, 60, 55], k * 1.3);
            puff(
                out,
                base,
                right,
                frame.up,
                size,
                size,
                rgba(colour, 0.85 * (1.0 - k)),
            );
        }
    }
    // Debris flung out on high arcs.
    if t < DEBRIS_SECS {
        let k = t / DEBRIS_SECS;
        for i in 0..DEBRIS {
            let angle = noise(blast.seed, 100 + i) * std::f32::consts::TAU;
            let speed = radius * (0.4 + 0.6 * noise(blast.seed, 120 + i));
            let lift = radius * (0.5 + 0.8 * noise(blast.seed, 140 + i));
            let along = speed * ease_out(k);
            let height = (lift * 4.0 * k * (1.0 - k)).max(0.0);
            let p = add(
                at(along * angle.cos(), along * angle.sin(), 0.1),
                frame.up,
                height,
            );
            let size = 0.25 + 0.35 * noise(blast.seed, 160 + i);
            let spin = t * 9.0 + i as f32;
            let side = add([0.0; 3], right, size * spin.cos());
            let rise = add([0.0; 3], frame.up, size * spin.sin().abs().max(0.3));
            out.tri(
                [add(p, side, -1.0), add(p, side, 1.0), add(p, rise, 1.0)],
                rgba([45, 35, 30], 1.0 - k * 0.5),
            );
        }
    }
}

fn draw_wreck(
    out: &mut OverlayGeometry,
    ground: &dyn Footing,
    right: [f32; 3],
    wreck: &HudWreck,
    t: f32,
    clock: f32,
) {
    let (face, cx, cy) = (wreck.face, wreck.x as f32 / 1000.0, wreck.y as f32 / 1000.0);
    let (hw, hh) = (
        f32::from(wreck.footprint.0.max(1)) / 2.0,
        f32::from(wreck.footprint.1.max(1)) / 2.0,
    );
    let full = (wreck.height as f32 / 1000.0).max(0.5);
    let at = |dx: f32, dy: f32, lift: f32| ground.point(face, cx, cy, (dx, dy), lift);
    // The model itself sinks into its foundations (the views cut it lower as it falls); what
    // stays is a ruin: rubble, a few broken walls, fire and smoke.
    let k = (t / COLLAPSE_SECS).clamp(0.0, 1.0);
    let height = full * 0.25;
    let tint = owner_color(wreck.owner);
    let char = |shade: f32| -> [u8; 4] {
        let base = [0.33, 0.29, 0.26];
        let rgb: [u8; 3] = std::array::from_fn(|i| {
            ((base[i] * 0.85 + tint[i] * 0.1) * shade * 255.0).min(255.0) as u8
        });
        rgba(rgb, k)
    };
    // Broken stretches of wall along the footprint's edges.
    for i in 0..6u32 {
        let side = (noise(wreck.seed, 70 + i) * 4.0) as u32;
        let along = noise(wreck.seed, 80 + i) * 1.6 - 0.8;
        let (x0, y0, x1, y1) = match side {
            0 => (along * hw - 0.6, -hh, along * hw + 0.6, -hh),
            1 => (hw, along * hh - 0.6, hw, along * hh + 0.6),
            2 => (along * hw - 0.6, hh, along * hw + 0.6, hh),
            _ => (-hw, along * hh - 0.6, -hw, along * hh + 0.6),
        };
        let rise = height * (0.5 + noise(wreck.seed, 90 + i));
        out.quad(
            [
                at(x0, y0, 0.0),
                at(x1, y1, 0.0),
                at(x1, y1, rise * 0.6),
                at(x0, y0, rise),
            ],
            char(0.7 + 0.3 * noise(wreck.seed, 95 + i)),
        );
    }
    // Rubble heaped over the footprint.
    for i in 0..8u32 {
        let dx = (noise(wreck.seed, 20 + i) - 0.5) * hw * 2.2;
        let dy = (noise(wreck.seed, 30 + i) - 0.5) * hh * 2.2;
        let size = 0.4 + 0.6 * noise(wreck.seed, 40 + i);
        let rise = (0.3 + 0.5 * noise(wreck.seed, 50 + i)) * k;
        out.tri(
            [
                at(dx - size, dy, 0.05),
                at(dx + size, dy - size * 0.5, 0.05),
                at(dx, dy + size * 0.3, rise),
            ],
            rgba([60, 52, 46], k),
        );
    }
    // A cloud of dust rolls out from the foot while it falls.
    if t < COLLAPSE_SECS + 1.5 {
        let d = t / (COLLAPSE_SECS + 1.5);
        let frame = ground.frame(face, cx, cy);
        for i in 0..8u32 {
            let angle = i as f32 / 8.0 * std::f32::consts::TAU + noise(wreck.seed, 60 + i);
            let reach = (hw.max(hh) + 1.0) * (0.6 + 0.8 * ease_out(d));
            let base = at(reach * angle.cos(), reach * angle.sin(), 0.0);
            let size = (hw.max(hh) * 0.9 + 0.5) * (0.6 + d);
            puff(
                out,
                base,
                right,
                frame.up,
                size * 1.4,
                size,
                rgba([150, 135, 115], 0.6 * (1.0 - d)),
            );
        }
    }
    if t < BURN_SECS {
        let blaze = Blaze {
            at: (face, cx, cy),
            half: (hw, hh),
            height: height * 0.6,
            strength: 1.0 - t / BURN_SECS,
            seed: wreck.seed,
        };
        fires(out, ground, right, &blaze, clock);
    }
}

/// Flames flickering over a footprint and smoke rising from it.
/// What burns: a footprint on the ground, how high the flames start and how strongly it burns.
#[derive(Clone, Copy)]
struct Blaze {
    /// Face and centre in tiles.
    at: (u8, f32, f32),
    /// Half the footprint in tiles.
    half: (f32, f32),
    /// Height the flames stand on, in tiles.
    height: f32,
    /// 0 to 1.
    strength: f32,
    /// Varies the flames and the smoke.
    seed: u32,
}

fn fires(
    out: &mut OverlayGeometry,
    ground: &dyn Footing,
    right: [f32; 3],
    blaze: &Blaze,
    clock: f32,
) {
    let Blaze {
        at: (face, cx, cy),
        half: (hw, hh),
        height,
        strength,
        seed,
    } = *blaze;
    let strength = strength.clamp(0.0, 1.0);
    if strength <= 0.0 {
        return;
    }
    let frame = ground.frame(face, cx, cy);
    let at = |dx: f32, dy: f32, lift: f32| ground.point(face, cx, cy, (dx, dy), lift);
    let flames = ((hw * hh).sqrt() * 1.5).clamp(2.0, 7.0) as u32;
    for i in 0..flames {
        let dx = (noise(seed, 400 + i) - 0.5) * hw * 1.6;
        let dy = (noise(seed, 420 + i) - 0.5) * hh * 1.6;
        let flicker = 0.75 + 0.25 * (clock * (9.0 + 3.0 * noise(seed, 440 + i)) + i as f32).sin();
        let size = (0.8 + 0.8 * noise(seed, 460 + i)) * strength * flicker;
        let base = at(dx, dy, height * noise(seed, 480 + i));
        flame(
            out,
            base,
            right,
            frame.up,
            size,
            size * 1.8,
            rgba([255, 120, 25], 0.85 * strength),
        );
        flame(
            out,
            base,
            right,
            frame.up,
            size * 0.5,
            size * 1.1,
            rgba([255, 230, 120], 0.95 * strength),
        );
    }
    // Smoke: puffs that rise, swell and thin, each on its own cycle.
    let column = (hw.max(hh) * 0.8).max(0.8);
    for i in 0..6u32 {
        let period = 4.0 + 2.0 * noise(seed, 500 + i);
        let phase = ((clock + noise(seed, 520 + i) * period) / period).fract();
        let drift = phase * column * 1.5;
        let base = add(
            at(
                (noise(seed, 540 + i) - 0.5) * hw + drift * 0.6,
                (noise(seed, 560 + i) - 0.5) * hh + drift * 0.3,
                height,
            ),
            frame.up,
            phase * column * 6.0,
        );
        let size = column * (0.8 + 1.6 * phase);
        puff(
            out,
            base,
            right,
            frame.up,
            size,
            size,
            rgba([45, 42, 40], 0.55 * strength * (1.0 - phase)),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::FlatGround;

    fn hud(blasts: Vec<HudBlast>, wrecks: Vec<HudWreck>) -> HudSnapshot {
        HudSnapshot {
            game_ticks: 400,
            blasts,
            wrecks,
            ..HudSnapshot::default()
        }
    }

    fn blast(age_ticks: u32) -> HudBlast {
        HudBlast {
            face: 0,
            x: 40_000,
            y: 40_000,
            radius: 12_000,
            age_ticks,
            seed: 7,
        }
    }

    fn drawn(hud: &HudSnapshot) -> OverlayGeometry {
        let flat = |_: f32, _: f32| 0.0;
        let mut out = OverlayGeometry::default();
        bombardment(&mut out, &FlatGround(&flat), [1.0, 0.0, 0.0], &[], hud);
        out
    }

    /// A fresh blast is drawn with its flash and shockwave; a minute later its scorch has faded
    /// and nothing is left of it.
    #[test]
    fn a_blast_flashes_spreads_and_fades() {
        let fresh = drawn(&hud(vec![blast(2)], Vec::new()));
        let late = drawn(&hud(vec![blast(400)], Vec::new()));
        let gone = drawn(&hud(vec![blast(1200)], Vec::new()));
        assert!(
            fresh.tris.len() > late.tris.len(),
            "{} {}",
            fresh.tris.len(),
            late.tris.len()
        );
        assert!(!late.tris.is_empty());
        assert!(gone.tris.is_empty());
        // The shockwave reaches out toward the edge of the blast.
        let reach = |o: &OverlayGeometry| {
            o.tris
                .iter()
                .flat_map(|t| t.v)
                .map(|p| ((p[0] - 40.0).powi(2) + (p[2] - 40.0).powi(2)).sqrt())
                .fold(0.0_f32, f32::max)
        };
        let wave = drawn(&hud(vec![blast(16)], Vec::new()));
        assert!(reach(&wave) > 8.0, "{}", reach(&wave));
    }

    /// A fallen building raises dust while it sinks, then its ruin stands and burns.
    #[test]
    fn a_wreck_collapses_and_burns() {
        let wreck = |age_ticks| HudWreck {
            face: 0,
            x: 10_000,
            y: 10_000,
            kind: 9,
            footprint: (4, 4),
            height: 3_000,
            owner: 2,
            age_ticks,
            seed: 3,
        };
        let falling = drawn(&hud(Vec::new(), vec![wreck(1)]));
        let fallen = drawn(&hud(Vec::new(), vec![wreck(200)]));
        let dust = |o: &OverlayGeometry| o.tris.iter().any(|t| t.colour[..3] == [150, 135, 115]);
        assert!(dust(&falling) && !dust(&fallen));
        // Broken walls, opaque once it has come down, and flames over them.
        assert!(
            fallen
                .tris
                .iter()
                .any(|t| t.colour[3] == 255 && t.colour[0] < 120)
        );
        assert!(
            fallen
                .tris
                .iter()
                .any(|t| t.colour[0] == 255 && t.colour[1] == 120)
        );
    }
}
