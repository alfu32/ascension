//! Day and night on a sphere planet: the directional light that represents the sun orbits the
//! planet orb once every `day_ticks` simulation ticks, and the shading rule that keeps the night
//! side readable. Pure maths: no window, no GPU.
//!
//! Directions are in the render frame of the planet (see [`crate::globe::to_render`]). The light
//! stays in the equatorial plane tilted by the axial tilt; its longitude falls by a full turn per
//! day, so every place on the planet gets day and night in turn.

use std::f32::consts::TAU;

use crate::orbit::{dir_from_lon_lat, lon_lat_of};

/// Default length of a day in simulation ticks (5 game minutes at 20 ticks per second).
pub const DEFAULT_DAY_TICKS: u64 = 6_000;

/// How the light that represents the sun moves over a planet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SunSettings {
    /// Ticks for one full orbit of the light around the planet orb (`world.day_ticks`).
    pub day_ticks: u64,
    /// Latitude the light stays over, in milli-degrees (`world.axial_tilt_mdeg`).
    pub axial_tilt_mdeg: i32,
    /// Keep the light fixed over the point the camera first looks at instead (`--sun fixed`;
    /// without a first view, over the home start).
    pub fixed: bool,
    /// With `fixed`, put the light on the far side of the planet instead, so that point is at
    /// midnight (`--sun night`).
    pub midnight: bool,
}

impl Default for SunSettings {
    fn default() -> Self {
        Self {
            day_ticks: DEFAULT_DAY_TICKS,
            axial_tilt_mdeg: 0,
            fixed: false,
            midnight: false,
        }
    }
}

/// Unit vector from the planet centre toward the light for a fixed sun over `home`: above it,
/// a little to the east and the north, so the relief at home still casts a shade.
pub fn fixed_toward(home: [f32; 3]) -> [f32; 3] {
    let n = unit(home);
    if dot(n, n) < 0.5 {
        return unit(FIXED_FALLBACK);
    }
    // East is around the polar axis; at a pole any tangent will do.
    let east = if n[0].hypot(n[2]) < 1e-3 {
        [1.0, 0.0, 0.0]
    } else {
        unit(cross([0.0, 1.0, 0.0], n))
    };
    let north = cross(n, east);
    unit([0, 1, 2].map(|i| n[i] * 0.82 + east[i] * 0.42 + north[i] * 0.3))
}

/// Toward-light direction used when a planet has no home position.
const FIXED_FALLBACK: [f32; 3] = [0.5, 0.3, 0.8];

/// The light that represents the sun over one planet, as a function of the simulation tick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SunClock {
    settings: SunSettings,
    /// Longitude of the light at tick 0, in radians.
    phase: f32,
    /// Latitude of the light in the render frame, in radians.
    latitude: f32,
    /// Direction toward the light when it is fixed.
    fixed: [f32; 3],
}

impl SunClock {
    /// The clock of a planet whose first player starts at the unit direction `home`: at tick 0
    /// the light stands over the home start's longitude. Without a home it starts over
    /// longitude 0.
    pub fn new(settings: SunSettings, home: Option<[f32; 3]>) -> Self {
        let phase = home.map_or(0.0, |h| lon_lat_of(h).0);
        // The render frame mirrors y: a northern geographic latitude points to −y.
        let latitude =
            -(settings.axial_tilt_mdeg.clamp(-90_000, 90_000) as f32 / 1000.0).to_radians();
        Self {
            settings,
            phase,
            latitude,
            fixed: unit(FIXED_FALLBACK),
        }
        .fixed_over(Some(home.unwrap_or(FIXED_FALLBACK)))
    }

    /// Places a fixed light over the unit direction `point` (the camera's first view), or on the
    /// far side of it with `midnight`; without a point, over the home start. The orbiting light
    /// is unaffected.
    pub fn fixed_over(mut self, point: Option<[f32; 3]>) -> Self {
        let Some(point) = point else {
            return self;
        };
        self.fixed = fixed_toward(point);
        if self.settings.midnight {
            self.fixed = self.fixed.map(|c| -c);
        }
        self
    }

    /// The settings the clock runs on.
    pub fn settings(&self) -> SunSettings {
        self.settings
    }

    /// Fraction of a day completed at `tick`, in `0..1`; zero when the light is fixed.
    pub fn day_fraction(&self, tick: u64) -> f32 {
        if self.settings.fixed || self.settings.day_ticks == 0 {
            return 0.0;
        }
        let day = self.settings.day_ticks;
        ((tick % day) as f64 / day as f64) as f32
    }

    /// Unit vector from the planet centre toward the light at `tick`.
    pub fn toward(&self, tick: u64) -> [f32; 3] {
        if self.settings.fixed || self.settings.day_ticks == 0 {
            return self.fixed;
        }
        let longitude = self.phase - TAU * self.day_fraction(tick);
        dir_from_lon_lat(longitude, self.latitude)
    }

    /// The direction the light travels at `tick` (the opposite of [`Self::toward`]).
    pub fn light(&self, tick: u64) -> [f32; 3] {
        self.toward(tick).map(|c| -c)
    }
}

/// Angle about the planet axis (+y) that turns the planet frame into the system frame so the
/// light toward `toward` in the planet frame comes from the star: the planet's rotation phase,
/// defined relative to the direction from `planet` to `star` in the system frame.
pub fn planet_spin(toward: [f32; 3], planet: [f32; 3], star: [f32; 3]) -> f32 {
    let to_star = sub(star, planet);
    let star_longitude = to_star[0].atan2(to_star[2]);
    let sun_longitude = toward[0].atan2(toward[2]);
    (star_longitude - sun_longitude).rem_euclid(TAU)
}

/// `v` turned by `angle` radians about +y; a vector at longitude λ ends up at λ + `angle`.
pub fn rotate_y(v: [f32; 3], angle: f32) -> [f32; 3] {
    let (sin, cos) = angle.sin_cos();
    [v[0] * cos + v[2] * sin, v[1], -v[0] * sin + v[2] * cos]
}

/// Half-width of the soft terminator band, as the sine of the angle from the terminator.
pub const TERMINATOR: f32 = 0.12;
/// Light on the day side that does not depend on the slope.
pub const DAY_AMBIENT: f32 = 0.42;
/// Strength of the direct light on ground facing the sun.
pub const DIRECT: f32 = 0.70;
/// Light on the night side: a cool floor high enough to keep terrain and borders readable.
pub const NIGHT_AMBIENT: [f32; 3] = [0.30, 0.35, 0.50];
/// How much of the direct light a shadow takes away.
pub const SHADOW_STRENGTH: f32 = 0.55;

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// How much daylight reaches a point whose radial direction makes `sun_height` (the cosine of
/// the angle to the light) with the light: 1 by day, 0 by night, a soft band between.
pub fn daylight(sun_height: f32) -> f32 {
    smoothstep(-TERMINATOR, TERMINATOR, sun_height)
}

/// The light factor (per colour channel) the terrain shader applies; the shader computes the
/// same expression. `sun_height` is as in [`daylight`], `diffuse` the cosine between the ground
/// normal and the light, `visibility` the share of the light not in shadow.
pub fn terrain_light(sun_height: f32, diffuse: f32, visibility: f32) -> [f32; 3] {
    let day = DAY_AMBIENT + DIRECT * diffuse.max(0.0) * visibility;
    let t = daylight(sun_height);
    NIGHT_AMBIENT.map(|night| night + (day - night) * t)
}

/// The GLSL of [`daylight`], [`terrain_light`] and the shadow lookup, shared by the lit shaders.
/// Expects the uniforms `toSun`, `planet`, `lightVP`, `shadowMap`, `shadows`, `depthBias` and
/// `normalBias`.
pub(crate) fn glsl_lighting() -> String {
    format!(
        r#"
uniform vec3 toSun;
uniform int planet;
uniform mat4 lightVP;
uniform sampler2D shadowMap;
uniform int shadows;
uniform float depthBias;
uniform float normalBias;
const float TERMINATOR = {TERMINATOR:.4};
const float DAY_AMBIENT = {DAY_AMBIENT:.4};
const float DIRECT = {DIRECT:.4};
const vec3 NIGHT_AMBIENT = vec3({:.4}, {:.4}, {:.4});
const float SHADOW_STRENGTH = {SHADOW_STRENGTH:.4};
float daylight(vec3 world) {{
    if (planet == 0) return 1.0;
    return smoothstep(-TERMINATOR, TERMINATOR, dot(normalize(world), toSun));
}}
// Depth is packed into RGB (24 bits) by the shadow pass; 3 x 3 percentage-closer filtering.
float visibility(vec3 world, vec3 normal) {{
    if (shadows == 0) return 1.0;
    vec4 projected = lightVP * vec4(world + normal * normalBias, 1.0);
    vec3 p = projected.xyz / projected.w * 0.5 + 0.5;
    if (any(lessThan(p, vec3(0.0))) || any(greaterThan(p, vec3(1.0)))) return 1.0;
    vec2 texel = 1.0 / vec2(textureSize(shadowMap, 0));
    float bias = depthBias * (1.0 + 3.0 * (1.0 - max(dot(normal, toSun), 0.0)));
    float occlusion = 0.0;
    for (int x = -1; x <= 1; ++x) for (int y = -1; y <= 1; ++y) {{
        vec3 stored = texture(shadowMap, p.xy + vec2(x, y) * texel).rgb;
        float depth = dot(stored, vec3(65536.0, 256.0, 1.0)) / 65793.0;
        occlusion += p.z - bias > depth ? 1.0 : 0.0;
    }}
    return 1.0 - SHADOW_STRENGTH * occlusion / 9.0;
}}
vec3 terrainLight(vec3 world, vec3 normal) {{
    float diffuse = max(dot(normal, toSun), 0.0);
    float lit = diffuse > 0.0 ? visibility(world, normal) : 1.0;
    vec3 day = vec3(DAY_AMBIENT + DIRECT * diffuse * lit);
    return mix(NIGHT_AMBIENT, day, daylight(world));
}}
"#,
        NIGHT_AMBIENT[0], NIGHT_AMBIENT[1], NIGHT_AMBIENT[2],
    )
}

/// `v` scaled to unit length; straight up (+y) for a zero vector.
pub(crate) fn unit_or(v: [f32; 3]) -> [f32; 3] {
    let u = unit(v);
    if u == [0.0; 3] { [0.0, 1.0, 0.0] } else { u }
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn unit(v: [f32; 3]) -> [f32; 3] {
    let length = dot(v, v).sqrt();
    if length < 1e-9 {
        return [0.0; 3];
    }
    v.map(|c| c / length)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: [f32; 3], b: [f32; 3], eps: f32) -> bool {
        (0..3).all(|i| (a[i] - b[i]).abs() < eps)
    }

    fn home() -> [f32; 3] {
        unit([0.4, -0.3, 0.8])
    }

    #[test]
    fn sun_direction_completes_one_rotation_per_day() {
        let clock = SunClock::new(SunSettings::default(), Some(home()));
        let day = DEFAULT_DAY_TICKS;
        for start in [0, 17, 2_999, 123_456] {
            assert!(close(clock.toward(start), clock.toward(start + day), 1e-5));
            assert!(close(
                clock.toward(start),
                clock.toward(start + 7 * day),
                1e-5
            ));
        }
        // Half a day later the light stands over the opposite meridian; a quarter turns it 90°.
        let (lon0, _) = lon_lat_of(clock.toward(0));
        let (lon_half, _) = lon_lat_of(clock.toward(day / 2));
        let turned = (lon0 - lon_half).rem_euclid(TAU);
        assert!((turned - TAU / 2.0).abs() < 1e-4, "{turned}");
        let quarter = dot(clock.toward(0), clock.toward(day / 4));
        assert!(quarter.abs() < 1e-5, "{quarter}");
        // The longitude falls steadily: the light moves west, the planet turns east.
        let (lon1, _) = lon_lat_of(clock.toward(day / 10));
        assert!(((lon0 - lon1).rem_euclid(TAU) - TAU / 10.0).abs() < 1e-4);
        for tick in (0..day).step_by(250) {
            let toward = clock.toward(tick);
            assert!((dot(toward, toward) - 1.0).abs() < 1e-5);
            assert!(
                toward[1].abs() < 1e-6,
                "no tilt keeps the light over the equator"
            );
        }
    }

    #[test]
    fn sun_starts_over_the_home_start() {
        for home in [home(), unit([-0.9, 0.2, -0.1]), unit([0.0, 0.5, -0.7])] {
            let clock = SunClock::new(SunSettings::default(), Some(home));
            let (home_lon, _) = lon_lat_of(home);
            let (sun_lon, sun_lat) = lon_lat_of(clock.toward(0));
            let gap = (home_lon - sun_lon).rem_euclid(TAU);
            assert!(gap < 1e-4 || TAU - gap < 1e-4, "{home:?}: {gap}");
            assert!(sun_lat.abs() < 1e-6);
            // So the home start is in full daylight at the first tick.
            assert!(daylight(dot(home, clock.toward(0))) > 0.99);
        }
        let tilted = SunClock::new(
            SunSettings {
                axial_tilt_mdeg: 23_440,
                ..SunSettings::default()
            },
            Some(home()),
        );
        let (_, lat) = lon_lat_of(tilted.toward(0));
        // Render latitude is the mirror of the geographic one.
        assert!((lat + 23.44_f32.to_radians()).abs() < 1e-5);
    }

    #[test]
    fn fixed_sun_option_keeps_the_direction_constant() {
        let settings = SunSettings {
            fixed: true,
            ..SunSettings::default()
        };
        let clock = SunClock::new(settings, Some(home()));
        let first = clock.toward(0);
        for tick in [1, 1_500, 3_000, 6_000, 1_000_003] {
            assert_eq!(clock.toward(tick), first);
        }
        assert_eq!(first, fixed_toward(home()));
        assert!(dot(first, home()) > 0.8, "the fixed light shines on home");
        assert_eq!(clock.light(5), first.map(|c| -c));
        // Over the first view when there is one; at midnight there with `midnight`.
        let view = unit([-0.6, 0.1, 0.5]);
        let over = SunClock::new(settings, Some(home())).fixed_over(Some(view));
        assert!(dot(over.toward(77), view) > 0.8);
        let night = SunClock::new(
            SunSettings {
                midnight: true,
                ..settings
            },
            Some(home()),
        )
        .fixed_over(Some(view));
        assert!(dot(night.toward(77), view) < -0.8);
        assert_eq!(night.toward(0), night.toward(9_999));
    }

    #[test]
    fn night_ambient_floor_keeps_terrain_visible() {
        let clock = SunClock::new(SunSettings::default(), Some(home()));
        let toward = clock.toward(0);
        let opposite = toward.map(|c| -c);
        // A point opposite the light, with ground facing straight away from it.
        let night = terrain_light(dot(opposite, toward), dot(opposite, toward), 1.0);
        let luma = |c: [f32; 3]| 0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2];
        assert!(luma(night) >= 0.3, "night too dark: {night:?}");
        assert!(night.iter().all(|&c| c >= 0.28));
        // Darker than noon, and the terminator is a band, not a hard line.
        let noon = terrain_light(1.0, 1.0, 1.0);
        assert!(luma(night) < 0.5 * luma(noon));
        let dusk = terrain_light(0.0, 0.0, 1.0);
        assert!(luma(dusk) > luma(night) - 0.05 && luma(dusk) < luma(terrain_light(0.3, 0.3, 1.0)));
        assert!(daylight(0.05) > 0.0 && daylight(0.05) < 1.0);
        assert!(daylight(-0.05) > 0.0 && daylight(-0.05) < 0.5);
    }

    #[test]
    fn spin_turns_the_light_onto_the_star() {
        let clock = SunClock::new(SunSettings::default(), Some(home()));
        let planet = [1200.0, 0.0, -800.0];
        for tick in [0, 1, 777, 4_321] {
            let toward = clock.toward(tick);
            let spin = planet_spin(toward, planet, [0.0; 3]);
            let in_system = rotate_y(toward, spin);
            assert!(close(in_system, unit(sub([0.0; 3], planet)), 1e-5));
        }
    }
}
