//! Bombardment effects (task 75), built into the overlay geometry from the HUD snapshot: the
//! flash, light column, secondary bursts, shockwave, fireball and debris of a blast, a scorch
//! that hands over to the stains of what it destroyed (task 80), buildings that collapse when
//! they fall instead of vanishing, and fire and smoke over ruins and badly damaged buildings.
//! What stays on the ground afterwards, soot and a building's rubble, is the stain's
//! (`crate::stains`).
//!
//! Task 169: drop pods falling from orbit like ballistic missiles, with a re-entry trail of fire
//! and smoke, and their impact: a hard flash, a ring of dust, clods thrown up, a scorch that
//! fades, and the pod opening on the ground.
//!
//! Task 170: the exhaust of the lift engines under a building in the air.

use super::{LIFT, OverlayGeometry};
use crate::entities::{EntityView, Footing, building_height};
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
/// Seconds the scorched ground of a blast takes to fade; the stains of what it destroyed stay.
const SCORCH_SECS: f32 = 8.0;
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
        let t = blast.age_ticks as f32 / tps;
        if blast.pod {
            draw_pod_impact(out, ground, right, blast, t);
        } else {
            draw_blast(out, ground, right, blast, t);
        }
    }
    // Drop pods on their way down (task 169).
    for view in views.iter().filter(|v| v.pod > 0.0) {
        draw_pod_fall(out, ground, right, view, clock);
    }
    // The exhaust of buildings in the air (task 170).
    for view in views.iter().filter(|v| v.engines && v.building) {
        draw_lift_exhaust(out, ground, right, view, clock);
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
        v.building
            && v.altitude == 0.0
            && v.lift <= 0.0
            && v.under_construction.is_none()
            && v.hp_frac < 0.35
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
    // Scorched ground: a dark disc, ragged at the rim, fading while the stains take over.
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

/// Seconds an opened drop pod stays on the ground, the last of them fading (task 169).
const POD_SECS: f32 = 6.0;
/// Seconds the hard flash of a pod's impact lasts.
const POD_FLASH_SECS: f32 = 0.35;
/// Seconds the ring of dust takes to roll out.
const POD_DUST_SECS: f32 = 1.6;
/// Seconds the pod takes to open, after it struck.
const POD_OPEN: (f32, f32) = (0.5, 1.3);
/// Tiles of fire behind a falling pod at the end of its fall, and of smoke above that.
const POD_FIRE: f32 = 8.0;
const POD_SMOKE: f32 = 14.0;

/// A drop pod on its way down (task 169), `view.lift` tiles above its landing point and
/// `view.pod` of its fall done: the nose glowing white-hot, a tongue of fire behind it and a
/// column of smoke above, all growing as it falls faster.
fn draw_pod_fall(
    out: &mut OverlayGeometry,
    ground: &dyn Footing,
    right: [f32; 3],
    view: &EntityView,
    clock: f32,
) {
    let frame = ground.frame(view.face, view.x, view.y);
    let nose = ground.point(view.face, view.x, view.y, (0.0, 0.0), view.lift);
    let speed = view.pod.clamp(0.0, 1.0);
    let seed = view.id.wrapping_mul(2_654_435_761);
    let flicker = 0.85 + 0.15 * (clock * 23.0 + noise(seed, 1) * 6.0).sin();
    // Smoke first, so the fire is drawn over it: puffs widening and fading up the column.
    let smoke = POD_SMOKE * (0.4 + 0.6 * speed);
    for k in 0..9u32 {
        let along = POD_FIRE * 0.6 + smoke * k as f32 / 9.0;
        let wobble = (noise(seed, 10 + k) - 0.5) * 0.3 * (1.0 + k as f32 * 0.2);
        let base = add(add(nose, frame.up, along), right, wobble);
        let size = 0.9 + 0.25 * k as f32;
        puff(
            out,
            base,
            right,
            frame.up,
            size,
            size * 1.3,
            rgba(
                mix([120, 110, 100], [70, 68, 66], k as f32 / 9.0),
                0.55 * (1.0 - k as f32 / 10.0),
            ),
        );
    }
    // The fire of re-entry: an outer orange tongue and a yellow-white core.
    let fire = POD_FIRE * (0.35 + 0.65 * speed) * flicker;
    flame_down(
        out,
        nose,
        right,
        frame.up,
        1.6,
        fire,
        rgba([255, 120, 30], 0.75),
    );
    flame_down(
        out,
        nose,
        right,
        frame.up,
        0.8,
        fire * 0.6,
        rgba([255, 230, 150], 0.9),
    );
    // The nose, white-hot.
    let glow = 0.8 + 1.0 * speed;
    puff(
        out,
        add(nose, frame.up, -glow * 0.5),
        right,
        frame.up,
        glow * 1.6,
        glow,
        rgba([255, 200, 120], 0.8 * flicker),
    );
    puff(
        out,
        add(nose, frame.up, -glow * 0.3),
        right,
        frame.up,
        glow * 0.8,
        glow * 0.6,
        rgba([255, 252, 235], 0.95),
    );
}

/// The exhaust of a building's lift engines (task 170): a flickering plume under each nozzle,
/// a hot core in it, and near the ground a ring of dust blown out under the building.
fn draw_lift_exhaust(
    out: &mut OverlayGeometry,
    ground: &dyn Footing,
    right: [f32; 3],
    view: &EntityView,
    clock: f32,
) {
    let frame = crate::entities::entity_frame(view, ground);
    let (spots, size) = crate::entities::lift_engine_spots(view, &frame);
    let seed = view.id.wrapping_mul(2_654_435_761);
    let down = frame.up.map(|v| -v);
    for (k, spot) in spots.iter().enumerate() {
        let flicker = 0.8 + 0.2 * (clock * 19.0 + noise(seed, k as u32) * 6.0).sin();
        let nozzle = add(*spot, frame.up, -rom_kit::engines::LIFT_ENGINE_DEPTH * size);
        let length = size * 1.8 * flicker;
        flame_down(
            out,
            nozzle,
            right,
            down,
            size * 0.9,
            length,
            rgba([255, 140, 50], 0.7),
        );
        flame_down(
            out,
            nozzle,
            right,
            down,
            size * 0.45,
            length * 0.6,
            rgba([255, 236, 190], 0.9),
        );
    }
    // Low over the ground the blast raises dust.
    let low = (1.0 - view.lift / 4.0).clamp(0.0, 1.0);
    if low > 0.0 {
        let under = ground.point(view.face, view.x, view.y, (0.0, 0.0), LIFT);
        let width = f32::from(view.footprint.0.max(view.footprint.1)) * (1.1 + 0.15 * clock.sin());
        puff(
            out,
            add(under, frame.up, -0.2),
            right,
            frame.up,
            width,
            0.9 * low,
            rgba([168, 150, 122], 0.45 * low),
        );
    }
}

/// A tongue of flame `width` across trailing `length` up from `tip` (the way a falling pod
/// came).
fn flame_down(
    out: &mut OverlayGeometry,
    tip: [f32; 3],
    right: [f32; 3],
    up: [f32; 3],
    width: f32,
    length: f32,
    colour: [u8; 4],
) {
    let shoulder = add(tip, up, length * 0.25);
    let left = add(shoulder, right, -width / 2.0);
    let rightmost = add(shoulder, right, width / 2.0);
    let end = add(tip, up, length);
    let faint = [colour[0], colour[1], colour[2], colour[3] / 3];
    out.tri([tip, rightmost, left], colour);
    out.tri([left, rightmost, end], faint);
}

/// A drop pod's impact (task 169), `t` seconds after it struck: a hard flash, a bright ring and
/// a ring of dust rolling out, clods thrown up, a scorch fading, and the pod opening its four
/// petals around the soldiers stepping out.
fn draw_pod_impact(
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
    // The scorch: a dark ragged disc fading over the scorch time.
    if t < SCORCH_SECS {
        let fade = 1.0 - t / SCORCH_SECS;
        let centre = at(0.0, 0.0, LIFT * 0.6);
        let rim = |i: usize| {
            let angle = i as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
            let r = radius * 0.9 * (0.75 + 0.35 * noise(blast.seed, 200 + i as u32));
            at(r * angle.cos(), r * angle.sin(), LIFT * 0.6)
        };
        for i in 0..SEGMENTS {
            out.tri(
                [centre, rim(i), rim((i + 1) % SEGMENTS)],
                rgba([22, 17, 14], 0.8 * fade),
            );
        }
    }
    // The hard flash.
    if t < POD_FLASH_SECS {
        let k = t / POD_FLASH_SECS;
        let size = radius * (0.8 + 1.2 * ease_out(k));
        puff(
            out,
            add(frame.origin, frame.up, -size * 0.2),
            right,
            frame.up,
            size * 2.0,
            size * 1.5,
            rgba([255, 210, 140], 0.8 * (1.0 - k)),
        );
        puff(
            out,
            frame.origin,
            right,
            frame.up,
            size,
            size * 0.9,
            rgba([255, 255, 245], 1.0 - k),
        );
        billboard(
            out,
            frame.origin,
            right,
            frame.up,
            radius * 0.3,
            12.0,
            rgba([255, 240, 210], 0.6 * (1.0 - k)),
        );
    }
    // A bright shock ring, then the dust rolling out behind it.
    if t < POD_DUST_SECS {
        let k = t / POD_DUST_SECS;
        let ring = ease_out((k * 2.5).min(1.0)) * radius * 1.8;
        if k < 0.4 {
            let width = 0.25;
            let alpha = 0.9 * (1.0 - k / 0.4);
            for i in 0..SEGMENTS {
                let a0 = i as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
                let a1 = (i + 1) as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
                let point = |a: f32, r: f32, lift: f32| at(r * a.cos(), r * a.sin(), lift);
                out.quad(
                    [
                        point(a0, ring - width, LIFT * 2.0),
                        point(a0, ring, LIFT * 2.0 + 0.15),
                        point(a1, ring, LIFT * 2.0 + 0.15),
                        point(a1, ring - width, LIFT * 2.0),
                    ],
                    rgba([255, 235, 190], alpha),
                );
            }
        }
        let reach = ease_out(k) * radius * 2.4;
        for i in 0..10u32 {
            let angle = i as f32 / 10.0 * std::f32::consts::TAU + noise(blast.seed, 400 + i);
            let base = at(reach * angle.cos(), reach * angle.sin(), 0.0);
            let size = radius * (0.5 + 0.9 * k) * (0.8 + 0.4 * noise(blast.seed, 420 + i));
            puff(
                out,
                base,
                right,
                frame.up,
                size * 1.4,
                size,
                rgba([150, 125, 95], 0.7 * (1.0 - k)),
            );
        }
    }
    // Clods thrown up and falling back.
    if t < 1.2 {
        let k = t / 1.2;
        for i in 0..10u32 {
            let angle = noise(blast.seed, 100 + i) * std::f32::consts::TAU;
            let along = radius * (0.6 + 1.2 * noise(blast.seed, 120 + i)) * ease_out(k);
            let height = radius * (1.0 + 1.5 * noise(blast.seed, 140 + i)) * 4.0 * k * (1.0 - k);
            let p = add(
                at(along * angle.cos(), along * angle.sin(), 0.1),
                frame.up,
                height,
            );
            let size = 0.12 + 0.15 * noise(blast.seed, 160 + i);
            out.tri(
                [
                    add(p, right, -size),
                    add(p, right, size),
                    add(p, frame.up, size * 1.5),
                ],
                rgba([60, 48, 36], 1.0 - k * 0.5),
            );
        }
    }
    // The pod: a squat hull whose four petals swing open, hot at first, cooling to grey.
    if t < POD_SECS {
        let fade = ((POD_SECS - t) / 1.0).clamp(0.0, 1.0);
        let open = ((t - POD_OPEN.0) / (POD_OPEN.1 - POD_OPEN.0)).clamp(0.0, 1.0);
        let angle = ease_out(open) * 1.1;
        let hot = (1.0 - t / 2.5).clamp(0.0, 1.0);
        let colour = mix([78, 80, 88], [255, 130, 50], hot * 0.8);
        let half = 0.4;
        let length = 0.9;
        let sides = [
            (frame.ex, frame.ey),
            (frame.ey, frame.ex),
            (frame.ex.map(|v| -v), frame.ey),
            (frame.ey.map(|v| -v), frame.ex),
        ];
        for (n, side) in sides {
            let base = add(frame.origin, n, half);
            let (a, b) = (add(base, side, -half), add(base, side, half));
            let reach: [f32; 3] =
                std::array::from_fn(|i| frame.up[i] * angle.cos() + n[i] * angle.sin());
            let tip = |p: [f32; 3]| add(add(p, reach, length), frame.up, 0.02);
            out.quad([a, b, tip(b), tip(a)], rgba(colour, fade));
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
    // The model itself sinks into its foundations (the views cut it lower as it falls) in a
    // cloud of dust, then its ruin burns; the rubble lying there is its stain's (task 80).
    let height = full * 0.25;
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
            pod: false,
        }
    }

    fn drawn(hud: &HudSnapshot) -> OverlayGeometry {
        let flat = |_: f32, _: f32| 0.0;
        let mut out = OverlayGeometry::default();
        bombardment(&mut out, &FlatGround(&flat), [1.0, 0.0, 0.0], &[], hud);
        out
    }

    /// A fresh blast is drawn with its flash and shockwave; its scorch fades within seconds (the
    /// stains of what it destroyed stay) and a minute later nothing is left of it.
    #[test]
    fn a_blast_flashes_spreads_and_fades() {
        let fresh = drawn(&hud(vec![blast(2)], Vec::new()));
        let late = drawn(&hud(vec![blast(100)], Vec::new()));
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

    /// Task 169: a drop pod's impact flashes and throws dust, its open petals stay a few
    /// seconds and nothing is left after the scorch; no fireball rises like a blast's.
    #[test]
    fn a_pod_strikes_opens_and_fades() {
        let pod = |age_ticks| HudBlast {
            radius: 1_500,
            pod: true,
            ..blast(age_ticks)
        };
        let fresh = drawn(&hud(vec![pod(1)], Vec::new()));
        let opened = drawn(&hud(vec![pod(40)], Vec::new()));
        let gone = drawn(&hud(vec![pod(400)], Vec::new()));
        assert!(fresh.tris.len() > opened.tris.len());
        // The petals stand out of the ground once open.
        let top = |o: &OverlayGeometry| {
            o.tris
                .iter()
                .flat_map(|t| t.v)
                .map(|p| p[1])
                .fold(0.0, f32::max)
        };
        assert!(top(&opened) > 0.3, "{}", top(&opened));
        assert!(gone.tris.is_empty());
        // Smaller than a bombardment: nothing reaches past four tiles once the flash is over.
        let reach = |o: &OverlayGeometry| {
            o.tris
                .iter()
                .flat_map(|t| t.v)
                .map(|p| ((p[0] - 40.0).powi(2) + (p[2] - 40.0).powi(2)).sqrt())
                .fold(0.0_f32, f32::max)
        };
        assert!(reach(&opened) < 4.0, "{}", reach(&opened));
    }

    /// A fallen building raises dust while it sinks, then its ruin burns.
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
        // No rubble stands on the ground any more (it lies there as the stain's debris); flames
        // burn over the ruin.
        assert!(
            !fallen
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
