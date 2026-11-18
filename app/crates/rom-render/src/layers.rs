//! One camera, nested frames.
//!
//! The continuous zoom has one perspective camera: one eye, one direction, one up. It is
//! expressed in each frame it draws in — the loaded planet's frame (tiles, centred on the
//! planet), the anchored system's frame (system units, the star at the origin) and the star map
//! (map units) — through the similarity transforms that nest the frames: the planet sits at its
//! place in its system, turned by its day; the system sits at its place on the map, scaled. The
//! transforms are composed in doubles when a frame is drawn, so every frame keeps coordinates
//! near its own origin and 32-bit floats stay precise from a building to the whole map. The
//! frames are drawn into one picture, far to near, each in its own depth range; no frame is
//! swapped, no projection changes. Pure maths: no window, no GPU.

use std::collections::BTreeMap;

use crate::hud_minimap::FOV_DEGREES;
use crate::pick::View;
use crate::sun::PlanetFrame;

fn to64(v: [f32; 3]) -> [f64; 3] {
    v.map(f64::from)
}

fn to32(v: [f64; 3]) -> [f32; 3] {
    v.map(|c| c as f32)
}

fn length(v: [f32; 3]) -> f32 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// Smooth step of `x` from 0 to 1 over `0..=1`.
pub fn smoothstep(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// A camera in one frame: the eye, the point looked at and the up vector, in the frame's units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameView {
    /// Eye position.
    pub eye: [f32; 3],
    /// Point looked at.
    pub target: [f32; 3],
    /// Unit up vector.
    pub up: [f32; 3],
}

impl FrameView {
    /// The pick view of this camera on a canvas of `size` pixels.
    pub fn view(&self, size: (f32, f32), centre_y: f32) -> View {
        View::looking(self.eye, self.target, self.up, size, centre_y)
    }

    /// Whether a sphere of `radius` at `centre` can reach the picture of a canvas with the
    /// width over height `aspect`: a cone test against the picture's diagonal, with a margin for
    /// the picture's off-centre projection. Never false for a sphere the eye is inside.
    pub fn sees_sphere(&self, centre: [f32; 3], radius: f32, aspect: f32) -> bool {
        const MARGIN: f32 = 0.25;
        let to = sub(centre, self.eye);
        let distance = length(to);
        if distance <= radius * 1.001 {
            return true;
        }
        let ahead = sub(self.target, self.eye);
        let ahead_length = length(ahead).max(1e-9);
        let cos = ((0..3).map(|i| to[i] * ahead[i]).sum::<f32>() / (distance * ahead_length))
            .clamp(-1.0, 1.0);
        let tan = (FOV_DEGREES.to_radians() / 2.0).tan();
        let half_diagonal = (tan * (1.0 + aspect * aspect).sqrt()).atan();
        let reach = half_diagonal + (radius / distance).asin() + MARGIN;
        cos.acos() <= reach
    }

    /// Distance from the eye to `point`.
    pub fn distance_to(&self, point: [f32; 3]) -> f32 {
        length(sub(point, self.eye))
    }

    /// Distance from the eye to the point looked at.
    pub fn distance(&self) -> f32 {
        self.distance_to(self.target)
    }

    /// Frame units per pixel at `point`, on a canvas `height` pixels high.
    pub fn per_pixel(&self, point: [f32; 3], height: f32) -> f32 {
        let tan = (FOV_DEGREES.to_radians() / 2.0).tan();
        self.distance_to(point) * 2.0 * tan / height.max(1.0)
    }

    /// Radius on screen in pixels of a sphere of `radius` at `centre`, on a canvas `height`
    /// pixels high: its angular radius over the half field of view.
    pub fn sphere_pixels(&self, centre: [f32; 3], radius: f32, height: f32) -> f32 {
        let d = self.distance_to(centre).max(radius * 1.000_01).max(1e-9);
        let angle = (radius / d).clamp(-1.0, 1.0).asin();
        let tan = (FOV_DEGREES.to_radians() / 2.0).tan();
        angle.tan() / tan * height / 2.0
    }
}

/// Where the loaded planet sits in its system: its centre, system units per tile and the turn
/// of its day that carries planet directions into the system frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlanetPlace {
    /// Centre of the planet in the system frame.
    pub centre: [f32; 3],
    /// System units per tile.
    pub per_tile: f32,
    /// The planet's turn from its own frame into the system frame.
    pub frame: PlanetFrame,
}

impl PlanetPlace {
    /// Where the planet frame's point `p` (tiles) lies in the system frame.
    pub fn to_system_point(&self, p: [f32; 3]) -> [f32; 3] {
        let turned = to64(self.frame.carry(p));
        let (centre, k) = (to64(self.centre), f64::from(self.per_tile));
        to32([0, 1, 2].map(|i| centre[i] + turned[i] * k))
    }

    /// Where the system frame's point `p` lies in the planet frame (tiles).
    pub fn to_planet_point(&self, p: [f32; 3]) -> [f32; 3] {
        let (p, centre, k) = (to64(p), to64(self.centre), f64::from(self.per_tile));
        let local = to32([0, 1, 2].map(|i| (p[i] - centre[i]) / k));
        self.frame.uncarry(local)
    }

    /// A planet-frame camera seen in the system frame.
    pub fn to_system(&self, view: &FrameView) -> FrameView {
        FrameView {
            eye: self.to_system_point(view.eye),
            target: self.to_system_point(view.target),
            up: self.frame.carry(view.up),
        }
    }

    /// A system-frame camera seen in the planet frame.
    pub fn to_planet(&self, view: &FrameView) -> FrameView {
        FrameView {
            eye: self.to_planet_point(view.eye),
            target: self.to_planet_point(view.target),
            up: self.frame.uncarry(view.up),
        }
    }
}

/// Where the anchored system sits on the star map: its place and map units per system unit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SystemPlace {
    /// The system's place on the map.
    pub here: [f32; 3],
    /// Map units per system unit.
    pub per_unit: f32,
}

impl SystemPlace {
    /// Where the system frame's point `p` lies on the star map.
    pub fn to_map_point(&self, p: [f32; 3]) -> [f32; 3] {
        let (p, here, k) = (to64(p), to64(self.here), f64::from(self.per_unit));
        to32([0, 1, 2].map(|i| here[i] + p[i] * k))
    }

    /// A system-frame camera seen on the star map.
    pub fn to_map(&self, view: &FrameView) -> FrameView {
        FrameView {
            eye: self.to_map_point(view.eye),
            target: self.to_map_point(view.target),
            up: view.up,
        }
    }

    /// Where the star map's point `p` lies in this system's frame.
    pub fn from_map_point(&self, p: [f32; 3]) -> [f32; 3] {
        let (p, here, k) = (to64(p), to64(self.here), f64::from(self.per_unit));
        to32([0, 1, 2].map(|i| (p[i] - here[i]) / k))
    }

    /// A star-map camera seen in this system's frame.
    pub fn from_map(&self, view: &FrameView) -> FrameView {
        FrameView {
            eye: self.from_map_point(view.eye),
            target: self.from_map_point(view.target),
            up: view.up,
        }
    }
}

/// How much of each layer's detail shows at the camera's distance; they change smoothly with
/// the distance, so detail fades rather than pops.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fades {
    /// Space around the loaded planet (orbit rings, ships on the system plane, names): 0 while
    /// the planet camera decides the picture, 1 once the plane's own pose has taken over.
    pub space: f32,
    /// The star map's own detail (routes, names, ship marks): 0 up to the whole system, 1 out
    /// on the map. Each system's own detail goes with how wide it looks: [`system_detail`].
    pub map: f32,
}

/// Angular radius of a system's disc of orbits, in radians, down to which all of its own
/// detail shows (a quarter of the whole-system view's), and at which none is left.
pub const SYSTEM_DETAIL_FULL: f32 = 0.06;
pub const SYSTEM_DETAIL_GONE: f32 = 0.02;

/// How much of a star system's own detail (bodies, rings, ships) shows when its disc of orbits
/// looks `angle` radians in radius: the system shrinks into its place on the star map, then
/// its detail fades into the map's halo. Every system alike, anchored or not, so changing the
/// anchor never changes the picture.
pub fn system_detail(angle: f32) -> f32 {
    smoothstep((angle - SYSTEM_DETAIL_GONE) / (SYSTEM_DETAIL_FULL - SYSTEM_DETAIL_GONE))
}

/// The one camera in every frame it draws in, with the fades of the layers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OneView {
    /// In the anchored system's frame.
    pub system: FrameView,
    /// On the star map.
    pub map: FrameView,
    /// In the loaded planet's frame, when that planet lies in the anchored system.
    pub planet: Option<FrameView>,
    /// The planet camera as drawn, while it is the one camera.
    pub planet_camera: Option<crate::orbit::OrbitCamera>,
    /// Where the loaded planet sits in the anchored system.
    pub place: Option<PlanetPlace>,
    /// Where the anchored system sits on the star map.
    pub system_place: SystemPlace,
    /// How much detail each layer shows.
    pub fades: Fades,
}

/// How large a ship type is drawn in the space views, from its `et_view_scale` row.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShipScale {
    /// Above a planet: times its true size.
    pub planet: f32,
    /// On a system's plane: times the readable reference size.
    pub system: f32,
    /// Standing on a planet's ground: times its true size (a starship keeps its planet size
    /// when it lands; an orbital craft comes down to its true size).
    pub landed: f32,
}

impl Default for ShipScale {
    fn default() -> Self {
        Self {
            planet: 1.0,
            system: 1.0,
            landed: 1.0,
        }
    }
}

impl ShipScale {
    /// The scale of a `et_view_scale` row's percentages; drawn at its planet size on the ground
    /// as well.
    pub fn from_pct(planet_pct: i64, system_pct: i64) -> Self {
        let planet = planet_pct.max(1) as f32 / 100.0;
        Self {
            planet,
            system: system_pct.max(1) as f32 / 100.0,
            landed: planet,
        }
    }

    /// The same scale, drawn at its true size once on the ground.
    pub fn true_size_on_the_ground(self) -> Self {
        Self {
            landed: 1.0,
            ..self
        }
    }
}

/// Ship scales by entity type (`EntityView::kind`, `ShipView::kind`); a type without a row is
/// drawn at the default scale.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ShipScales(pub BTreeMap<u16, ShipScale>);

impl ShipScales {
    /// The scale of entity type `kind`.
    pub fn of(&self, kind: u16) -> ShipScale {
        self.0.get(&kind).copied().unwrap_or_default()
    }
}

/// Height of an orbit above the ground in the planet view, in tiles, as the snapshots give it:
/// a ship's altitude there is this times how far up it is (0 on the ground, 1 in orbit). The
/// drawn height is [`hover_altitude`].
pub const ORBIT_ALTITUDE: f32 = 8.0;
/// Length of a ship on the system plane at scale 1, in units of [`plane_unit`].
pub const SHIP_LENGTH_UNITS: f32 = 3.2;
/// How much higher a ship holds over a planet per tile of its drawn length (task 72).
pub const HOVER_PER_TILE: f32 = 0.6;

/// The size factor of a ship `altitude` tiles above the ground (of [`ORBIT_ALTITUDE`]): its
/// planet scale in orbit, easing to its scale on the ground as it comes down to land.
pub fn planet_factor(scale: ShipScale, altitude: f32) -> f32 {
    scale.landed + (scale.planet - scale.landed) * smoothstep(altitude / ORBIT_ALTITUDE)
}

/// The height above sea level, in tiles, at which a ship drawn `length` tiles long holds over a
/// planet (owner, 2026-11-17: bigger ships higher, smaller ones lower, so a mixed fleet over
/// one point stacks in height): the orbit of the snapshots plus [`HOVER_PER_TILE`] a tile.
pub fn hover_altitude(length: f32) -> f32 {
    ORBIT_ALTITUDE + HOVER_PER_TILE * length.max(0.0)
}

/// The unit that keeps marks readable on the system plane at camera distance `distance`
/// (system units): it grows with the distance, so marks keep their size on screen.
pub fn plane_unit(distance: f32) -> f32 {
    (distance * 0.016).max(24.0)
}

/// Length of a ship on the system plane at camera distance `distance`, in system units.
pub fn system_ship_length(scale: ShipScale, distance: f32) -> f32 {
    SHIP_LENGTH_UNITS * plane_unit(distance) * scale.system
}

/// A size between the planet view's (`t` = 0) and the system plane's (`t` = 1), blended in
/// log space so the change reads the same at every step.
pub fn morph_size(planet: f32, system: f32, t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    planet.max(1e-6).powf(1.0 - t) * system.max(1e-6).powf(t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_nest_and_round_trip() {
        let frame = PlanetFrame::aligned([0.3, 0.4, 0.866], [5000.0, 0.0, -2000.0], [0.0; 3]);
        let place = PlanetPlace {
            centre: [5000.0, 0.0, -2000.0],
            per_tile: 2.5,
            frame,
        };
        let p = [12.0, 130.0, -40.0];
        let back = place.to_planet_point(place.to_system_point(p));
        assert!((0..3).all(|i| (back[i] - p[i]).abs() < 1e-3), "{back:?}");
        let map = SystemPlace {
            here: [30_000.0, 0.0, -9_000.0],
            per_unit: 0.1,
        };
        assert_eq!(map.to_map_point([0.0; 3]), map.here);
        let q = map.from_map_point(map.to_map_point(p));
        assert!((0..3).all(|i| (q[i] - p[i]).abs() < 1e-2), "{q:?}");
        // Distances scale by the frame factors, so sizes on screen agree in every frame.
        let view = FrameView {
            eye: [0.0, 0.0, 300.0],
            target: [0.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
        };
        let in_system = place.to_system(&view);
        let in_map = map.to_map(&in_system);
        let planet_px = view.sphere_pixels([0.0; 3], 120.0, 800.0);
        let system_px = in_system.sphere_pixels(place.centre, 300.0, 800.0);
        let map_px = in_map.sphere_pixels(map.to_map_point(place.centre), 30.0, 800.0);
        assert!(
            (planet_px - system_px).abs() < 1e-2,
            "{planet_px} {system_px}"
        );
        assert!((planet_px - map_px).abs() < 1e-2, "{planet_px} {map_px}");
    }

    #[test]
    fn ship_sizes_change_smoothly() {
        // A starship keeps its planet size down to the ground; a craft eases to its own.
        let ship = ShipScale::from_pct(900, 40);
        assert_eq!(planet_factor(ship, ORBIT_ALTITUDE), 9.0);
        assert_eq!(planet_factor(ship, 0.0), 9.0);
        let scale = ShipScale::from_pct(400, 40).true_size_on_the_ground();
        assert_eq!(planet_factor(scale, ORBIT_ALTITUDE), 4.0);
        assert_eq!(planet_factor(scale, 0.0), 1.0);
        let mut last = planet_factor(scale, 0.0);
        for i in 1..=100 {
            let now = planet_factor(scale, ORBIT_ALTITUDE * i as f32 / 100.0);
            assert!(now >= last && now - last < 0.1);
            last = now;
        }
        assert_eq!(morph_size(2.0, 50.0, 0.0), 2.0);
        assert!((morph_size(2.0, 50.0, 1.0) - 50.0).abs() < 1e-3);
        assert!(
            system_ship_length(scale, 48_000.0)
                < system_ship_length(ShipScale::default(), 48_000.0) * 0.5
        );
    }

    #[test]
    fn bigger_ships_hover_higher() {
        assert_eq!(hover_altitude(0.0), ORBIT_ALTITUDE);
        let mut last = 0.0;
        for length in [2.0, 14.0, 20.0, 30.0, 54.0] {
            let height = hover_altitude(length);
            assert!(height > last, "{length}: {height}");
            last = height;
        }
    }
}
