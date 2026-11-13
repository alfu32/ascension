//! The sky seen from near the surface: blue by day, a short dusk, then navy and black with the
//! stars, and the blend into black space as the camera climbs. Pure maths: no window, no GPU.
//!
//! Everything follows the height of the light that represents the sun above the horizon of the
//! point the camera looks at, from the same direction as the lighting.

/// Colours of the sky dome and how strongly the stars show.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sky {
    /// Colour straight overhead.
    pub zenith: [f32; 3],
    /// Colour at the horizon.
    pub horizon: [f32; 3],
    /// Colour of the dusk or dawn glow near the horizon on the sun's side.
    pub glow: [f32; 3],
    /// Strength of that glow, 0 to 1.
    pub glow_strength: f32,
    /// Opacity of the star field, 0 (day) to 1 (night or space).
    pub stars: f32,
}

const DAY_ZENITH: [f32; 3] = [0.16, 0.38, 0.78];
const DAY_HORIZON: [f32; 3] = [0.60, 0.76, 0.92];
const DUSK_GLOW: [f32; 3] = [0.98, 0.52, 0.34];
const DUSK_HORIZON: [f32; 3] = [0.86, 0.56, 0.52];
const NAVY_ZENITH: [f32; 3] = [0.02, 0.04, 0.12];
const NAVY_HORIZON: [f32; 3] = [0.05, 0.08, 0.20];
/// The black of space, as the clear colour draws it.
pub const SPACE_COLOUR: [f32; 3] = [1.0 / 255.0, 2.0 / 255.0, 5.0 / 255.0];

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    // Exactly `b` at the end, so a full blend into space is plain space.
    if t >= 1.0 {
        return b;
    }
    [0, 1, 2].map(|i| a[i] + (b[i] - a[i]) * t)
}

/// The sky where the sun stands `sun_height` above the horizon (the sine of its elevation: 1 at
/// noon overhead, 0 at sunset, −1 at midnight below the feet). Dawn mirrors dusk.
pub fn local_sky(sun_height: f32) -> Sky {
    let h = sun_height.clamp(-1.0, 1.0);
    // Day: from just above the horizon up.
    let day = smoothstep(-0.04, 0.22, h);
    // Twilight: navy after sunset, black once the sun is well down.
    let twilight = smoothstep(-0.30, -0.04, h);
    // Dusk glow: strongest with the sun at the horizon, gone a little above and below it.
    let glow_strength = (-(h - 0.01) * (h - 0.01) / (0.09 * 0.09)).exp();
    let night_zenith = mix(SPACE_COLOUR, NAVY_ZENITH, twilight);
    let night_horizon = mix(SPACE_COLOUR, NAVY_HORIZON, twilight);
    let dusk_horizon = mix(night_horizon, DUSK_HORIZON, glow_strength);
    Sky {
        zenith: mix(night_zenith, DAY_ZENITH, day),
        horizon: mix(dusk_horizon, DAY_HORIZON, day),
        glow: DUSK_GLOW,
        glow_strength,
        stars: 1.0 - smoothstep(-0.20, 0.0, h),
    }
}

/// Share of black space in the backdrop for a camera `altitude` tiles above a planet of
/// `radius` tiles: 0 near the ground, where the horizon is in view, 1 from about half a radius
/// up (globe zoom), smooth in between.
pub fn space_share(altitude: f32, radius: f32) -> f32 {
    smoothstep(0.12, 0.6, altitude / radius.max(1.0))
}

/// `sky` blended toward black space with stars by `share` (see [`space_share`]).
pub fn toward_space(sky: Sky, share: f32) -> Sky {
    let s = share.clamp(0.0, 1.0);
    Sky {
        zenith: mix(sky.zenith, SPACE_COLOUR, s),
        horizon: mix(sky.horizon, SPACE_COLOUR, s),
        glow: sky.glow,
        glow_strength: sky.glow_strength * (1.0 - s),
        stars: sky.stars + (1.0 - sky.stars) * s,
    }
}

/// The backdrop of a planet view: the local sky at the point looked at, blended to space by the
/// camera's altitude.
pub fn backdrop(sun_height: f32, altitude: f32, radius: f32) -> Sky {
    toward_space(local_sky(sun_height), space_share(altitude, radius))
}

/// Sine of the angle the visible horizon dips below horizontal at `altitude`.
pub fn horizon_dip(altitude: f32, radius: f32) -> f32 {
    let below = radius / (radius + altitude.max(0.0));
    (1.0 - below * below).max(0.0).sqrt()
}

/// Sky colour along a unit view `ray`, with local vertical `up` and light direction `toward`.
/// The dome shader uses the same gradient and directional twilight glow.
pub fn dome_colour(sky: &Sky, ray: [f32; 3], up: [f32; 3], toward: [f32; 3], dip: f32) -> [f32; 3] {
    let dot = |a: [f32; 3], b: [f32; 3]| (0..3).map(|i| a[i] * b[i]).sum::<f32>();
    let vertical = dot(ray, up);
    let height = (vertical + dip) / (1.0 + dip);
    let colour = mix(
        sky.horizon,
        sky.zenith,
        1.0 - (1.0 - height.clamp(0.0, 1.0)).powi(4),
    );
    let across = [0, 1, 2].map(|i| ray[i] - up[i] * vertical);
    let sun_across = [0, 1, 2].map(|i| toward[i] - up[i] * dot(toward, up));
    let length = |v| dot(v, v).sqrt();
    let side = if length(across) > 1e-4 && length(sun_across) > 1e-4 {
        (dot(across, sun_across) / (length(across) * length(sun_across))).max(0.0)
    } else {
        0.0
    };
    let low = 1.0 - (height.abs() * 3.0).clamp(0.0, 1.0);
    mix(
        colour,
        sky.glow,
        sky.glow_strength * side * side * low * low * 0.85,
    )
}

/// GLSL counterpart of [`dome_colour`].
pub(crate) const DOME_FS: &str = r#"#version 330
in vec3 fragDir;
uniform vec3 up;
uniform vec3 toSun;
uniform vec3 zenith;
uniform vec3 horizon;
uniform vec3 glow;
uniform float glowStrength;
uniform float dip;
out vec4 finalColor;
void main() {
    vec3 d = normalize(fragDir);
    float vertical = dot(d, up);
    float h = (vertical + dip) / (1.0 + dip);
    vec3 colour = mix(horizon, zenith, 1.0 - pow(1.0 - clamp(h, 0.0, 1.0), 4.0));
    vec3 across = d - up * vertical;
    vec3 sunAcross = toSun - up * dot(toSun, up);
    float side = 0.0;
    if (length(across) > 1e-4 && length(sunAcross) > 1e-4) {
        side = max(dot(normalize(across), normalize(sunAcross)), 0.0);
    }
    float low = 1.0 - clamp(abs(h) * 3.0, 0.0, 1.0);
    colour = mix(colour, glow, glowStrength * side * side * low * low * 0.85);
    finalColor = vec4(colour, 1.0);
}
"#;

/// Thickness of the atmosphere halo around a habitable planet, as a fraction of its radius.
pub const HALO_THICKNESS: f32 = 0.06;
/// Opacity of the halo at its brightest.
const HALO_PEAK: f32 = 0.85;
/// Share of the day-side brightness the halo keeps on the night side.
const HALO_NIGHT: f32 = 0.12;

/// Colour of the atmosphere seen from outside: the street-level day sky, between its horizon
/// and zenith colours, so climbing out of the blue sky reads as the same air.
pub fn atmosphere_colour() -> [f32; 3] {
    mix(DAY_HORIZON, DAY_ZENITH, 0.35)
}

/// Opacity of the atmosphere halo at a point beyond the limb whose direction from the planet
/// centre makes `sun_dot` (cosine) with the light, `t` of the way out (0 at the planet's edge,
/// 1 where the halo ends): brightest on the day side, dim at night, fading outward.
pub fn halo(sun_dot: f32, t: f32) -> f32 {
    let rim = (1.0 - t.clamp(0.0, 1.0)).powf(2.5);
    let light = HALO_NIGHT + (1.0 - HALO_NIGHT) * smoothstep(-0.3, 0.35, sun_dot);
    HALO_PEAK * rim * light
}

/// Weight of the warm terminator tint in the halo colour at `sun_dot`.
pub fn halo_warmth(sun_dot: f32) -> f32 {
    0.55 * (-(sun_dot * sun_dot) / (0.16 * 0.16)).exp()
}

/// The GLSL of [`halo`], [`halo_warmth`] and [`atmosphere_colour`] for the halo shader.
pub(crate) fn glsl_halo() -> String {
    let [r, g, b] = atmosphere_colour();
    let [wr, wg, wb] = DUSK_GLOW;
    format!(
        r#"
const vec3 ATMOSPHERE = vec3({r:.4}, {g:.4}, {b:.4});
const vec3 WARM = vec3({wr:.4}, {wg:.4}, {wb:.4});
float halo(float sunDot, float t) {{
    float rim = pow(1.0 - clamp(t, 0.0, 1.0), 2.5);
    float light = {HALO_NIGHT:.4} + (1.0 - {HALO_NIGHT:.4}) * smoothstep(-0.3, 0.35, sunDot);
    return {HALO_PEAK:.4} * rim * light;
}}
float haloWarmth(float sunDot) {{
    return 0.55 * exp(-(sunDot * sunDot) / (0.16 * 0.16));
}}
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn luma(c: [f32; 3]) -> f32 {
        0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2]
    }

    #[test]
    fn sky_is_blue_at_noon_and_black_at_midnight() {
        let noon = local_sky(1.0);
        for c in [noon.zenith, noon.horizon] {
            assert!(c[2] > c[0] && c[2] > c[1] && c[2] > 0.7, "{c:?}");
        }
        // Deeper blue overhead, paler near the horizon.
        assert!(luma(noon.horizon) > luma(noon.zenith));
        assert!(noon.glow_strength < 0.01 && noon.stars == 0.0);
        let midnight = local_sky(-1.0);
        for c in [midnight.zenith, midnight.horizon] {
            assert!(c.iter().all(|&v| v < 0.03), "{c:?}");
        }
        // At sunset the horizon warms: more red than blue, with the glow at full strength.
        let sunset = local_sky(0.0);
        assert!(
            sunset.horizon[0] > sunset.horizon[2],
            "{:?}",
            sunset.horizon
        );
        assert!(sunset.glow_strength > 0.9);
        // A little after sunset the sky is navy: dark, blue over red.
        let navy = local_sky(-0.15);
        assert!(luma(navy.zenith) < 0.1 && navy.zenith[2] > navy.zenith[0]);
        // Dusk and dawn are the same function of the sun's height, and darkening is monotonic.
        let mut last = f32::MAX;
        for step in 0..=40 {
            let h = 1.0 - step as f32 * 0.05;
            let zenith = luma(local_sky(h).zenith);
            assert!(zenith <= last + 1e-6, "{h}");
            last = zenith;
        }
    }

    #[test]
    fn halo_is_brighter_on_the_day_side() {
        for t in [0.0, 0.3, 0.7] {
            let day = halo(0.9, t);
            let night = halo(-0.9, t);
            assert!(day > 2.0 * night, "{t}: {day} vs {night}");
            assert!(night > 0.0, "the night side keeps a faint halo");
            // Dusk lies between.
            assert!(halo(0.0, t) < day && halo(0.0, t) > night);
        }
        // Brightest at the limb, nothing at the outer edge.
        assert!(halo(0.9, 0.0) > halo(0.9, 0.5));
        assert_eq!(halo(0.9, 1.0), 0.0);
        // Warm only near the terminator; the colour is the sky's blue.
        assert!(halo_warmth(0.0) > 0.5 && halo_warmth(0.8) < 0.01);
        let air = atmosphere_colour();
        assert!(air[2] > air[0] && air[2] > air[1]);
    }

    #[test]
    fn stars_fade_in_after_dusk() {
        assert_eq!(local_sky(0.5).stars, 0.0);
        assert_eq!(local_sky(0.01).stars, 0.0);
        assert!(local_sky(-0.1).stars > 0.0 && local_sky(-0.1).stars < 1.0);
        assert_eq!(local_sky(-0.3).stars, 1.0);
        let mut last = 0.0;
        for step in 0..=60 {
            let h = 0.3 - step as f32 * 0.01;
            let stars = local_sky(h).stars;
            assert!(stars >= last && stars - last < 0.1, "{h}: {stars}");
            last = stars;
        }
    }

    #[test]
    fn sky_blends_to_space_with_altitude() {
        let radius = 122.0;
        assert_eq!(space_share(2.0, radius), 0.0);
        assert_eq!(space_share(radius, radius), 1.0);
        let street = backdrop(1.0, 3.0, radius);
        assert_eq!(street, local_sky(1.0));
        let globe = backdrop(1.0, 2.0 * radius, radius);
        assert_eq!(globe.zenith, SPACE_COLOUR);
        assert_eq!(globe.horizon, SPACE_COLOUR);
        assert_eq!(globe.stars, 1.0);
        // No pop: small climbs change the backdrop a little.
        let mut last = backdrop(1.0, 0.0, radius);
        for step in 1..=400 {
            let now = backdrop(1.0, step as f32 * 0.5, radius);
            let jump = (0..3)
                .map(|i| (now.zenith[i] - last.zenith[i]).abs())
                .fold(now.stars - last.stars, f32::max);
            assert!(jump < 0.02, "altitude {}: {jump}", step as f32 * 0.5);
            last = now;
        }
    }

    #[test]
    fn dome_gradient_and_twilight_follow_the_local_horizon() {
        let up = [0.0, 1.0, 0.0];
        let toward = [0.0, 0.0, 1.0];
        let noon = local_sky(1.0);
        assert_eq!(dome_colour(&noon, up, up, toward, 0.0), noon.zenith);
        assert_eq!(dome_colour(&noon, toward, up, toward, 0.0), noon.horizon);
        let dusk = local_sky(0.0);
        let front = dome_colour(&dusk, toward, up, toward, 0.0);
        let back = dome_colour(&dusk, [0.0, 0.0, -1.0], up, toward, 0.0);
        assert!(front[0] > back[0] && front[2] < back[2]);
        // Overhead light has no horizontal glow direction, even with horizon dip.
        assert!(
            dome_colour(&dusk, up, up, up, 0.5)
                .iter()
                .all(|v| v.is_finite())
        );
        assert_eq!(horizon_dip(0.0, 122.0), 0.0);
        assert!(horizon_dip(10.0, 122.0) > 0.0);
        assert!(horizon_dip(10.0, 122.0) < horizon_dip(50.0, 122.0));
    }
}
