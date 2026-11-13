//! Latitude and longitude lines of a sphere planet: the equator, the prime meridian, the tropics
//! and the polar circles, on the globe and on the equirectangular minimap. Pure maths.

use std::f32::consts::{PI, TAU};

use crate::globe::{Globe, to_render};
use crate::hud_layout::Rect;
use crate::orbit::{dir_from_lon_lat, tangent_frame};

/// Latitude of the tropics, in degrees.
pub const TROPIC_DEG: f32 = 23.44;
/// Latitude of the polar circles, in degrees.
pub const POLAR_CIRCLE_DEG: f32 = 66.56;
/// Orbit distance in tiles below which the lines are gone (street level).
pub const FADE_NEAR: f32 = 14.0;
/// Orbit distance in tiles from which the lines are fully drawn.
pub const FADE_FAR: f32 = 40.0;

/// Which line of the graticule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineKind {
    /// Latitude 0.
    Equator,
    /// Longitude 0, pole to pole.
    PrimeMeridian,
    /// Latitude ±23.44°.
    Tropic,
    /// Latitude ±66.56°.
    PolarCircle,
}

impl LineKind {
    /// Colour of the line; the equator and the prime meridian are a little stronger.
    pub fn colour(self) -> [u8; 4] {
        match self {
            Self::Equator => [255, 214, 120, 200],
            Self::PrimeMeridian => [255, 176, 120, 200],
            Self::Tropic => [150, 214, 255, 140],
            Self::PolarCircle => [200, 236, 255, 140],
        }
    }
}

/// One line: its kind, its geographic latitude (none for the meridian) and its points.
#[derive(Clone, Debug, PartialEq)]
pub struct GraticuleLine {
    /// Which line.
    pub kind: LineKind,
    /// Geographic latitude in degrees (north positive); `None` for the prime meridian.
    pub latitude_deg: Option<f32>,
    /// Points along the line, in order; circles are closed (the last point repeats the first).
    pub points: Vec<[f32; 3]>,
}

/// The lines in kind order, as unit directions in the render frame: circles of latitude with
/// `segments` pieces, the meridian with half as many.
pub fn graticule_dirs(segments: u32) -> Vec<GraticuleLine> {
    let segments = segments.max(8);
    let circle = |kind, latitude_deg: f32| {
        // Geographic north is −y in the render frame.
        let render_lat = -latitude_deg.to_radians();
        let points = (0..=segments)
            .map(|i| dir_from_lon_lat(i as f32 / segments as f32 * TAU - PI, render_lat))
            .collect();
        GraticuleLine {
            kind,
            latitude_deg: Some(latitude_deg),
            points,
        }
    };
    let half = segments / 2;
    let meridian = GraticuleLine {
        kind: LineKind::PrimeMeridian,
        latitude_deg: None,
        points: (0..=half)
            .map(|i| dir_from_lon_lat(0.0, i as f32 / half as f32 * PI - PI / 2.0))
            .collect(),
    };
    vec![
        circle(LineKind::Equator, 0.0),
        meridian,
        circle(LineKind::Tropic, TROPIC_DEG),
        circle(LineKind::Tropic, -TROPIC_DEG),
        circle(LineKind::PolarCircle, POLAR_CIRCLE_DEG),
        circle(LineKind::PolarCircle, -POLAR_CIRCLE_DEG),
    ]
}

/// The lines laid on `globe`: one point per tile along each line, `lift` tiles above the highest
/// ground (or sea surface) near it, so the line stays above the terrain mesh between points.
pub fn graticule_on(globe: &Globe, lift: f32) -> Vec<GraticuleLine> {
    let segments = 4 * globe.n();
    let reach = 0.75 / globe.radius().max(1.0);
    let mut lines = graticule_dirs(segments);
    for line in &mut lines {
        for point in &mut line.points {
            let dir = *point;
            let (north, east) = tangent_frame(dir);
            let around = [
                dir,
                add(dir, scale(north, reach)),
                add(dir, scale(north, -reach)),
                add(dir, scale(east, reach)),
                add(dir, scale(east, -reach)),
            ];
            let height = around
                .iter()
                .map(|&d| globe.ground_height(d))
                .fold(0.0, f32::max);
            *point = scale(dir, globe.radius() + height + lift);
        }
    }
    lines
}

/// Share of each line's alpha drawn at orbit distance `distance`: 0 at street level, 1 from
/// [`FADE_FAR`].
pub fn graticule_fade(distance: f32) -> f32 {
    ((distance - FADE_NEAR) / (FADE_FAR - FADE_NEAR)).clamp(0.0, 1.0)
}

/// A line on the equirectangular minimap drawn into `area`: its kind and end points.
pub type MinimapLine = (LineKind, (f32, f32), (f32, f32));

/// The graticule on the equirectangular minimap in `area` (see
/// [`crate::hud_minimap::dir_to_minimap`]): rows for the circles of latitude, a column for the
/// prime meridian.
pub fn minimap_graticule(area: Rect) -> Vec<MinimapLine> {
    let mut out = Vec::new();
    for line in graticule_dirs(16) {
        match line.latitude_deg {
            Some(latitude) => {
                // The minimap puts render latitude +90° at the top; geographic latitude is mirrored.
                let render_lat = -latitude.to_radians();
                let y = area.y + (0.5 - render_lat / PI) * area.h;
                out.push((line.kind, (area.x, y), (area.right(), y)));
            }
            None => {
                let x = area.x + 0.5 * area.w;
                out.push((line.kind, (x, area.y), (x, area.bottom())));
            }
        }
    }
    out
}

/// Geographic latitude and longitude in degrees of a render-frame direction, by the integer
/// conversions of `rom-geo`.
pub fn geo_lat_lon_deg(dir: [f32; 3]) -> (f32, f32) {
    let length = dot(dir, dir).sqrt().max(1e-9);
    let scale = rom_geo::SPHERE_SCALE as f32 / length;
    let point = to_render(dir).map(|c| (c * scale).round() as i32);
    (
        rom_geo::latitude_mdeg(point) as f32 / 1000.0,
        rom_geo::longitude_mdeg(point) as f32 / 1000.0,
    )
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn scale(a: [f32; 3], s: f32) -> [f32; 3] {
    a.map(|c| c * s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::globe::PlanetScene;
    use crate::terrain::TileKind;
    use std::collections::BTreeMap;

    fn hilly(n: u32) -> PlanetScene {
        let tiles = 6 * (n * n) as usize;
        PlanetScene {
            n,
            kinds: (0..tiles)
                .map(|i| {
                    if i % 7 < 3 {
                        TileKind::DeepSea
                    } else {
                        TileKind::Land
                    }
                })
                .collect(),
            heights: (0..tiles).map(|i| (i * 37 % 256) as u8).collect(),
            feature_colors: BTreeMap::new(),
            starts: Vec::new(),
            nodes: Vec::new(),
            sun: Default::default(),
        }
    }

    #[test]
    fn graticule_lines_lie_on_their_latitudes() {
        let globe = Globe::new(&hilly(48));
        let lines = graticule_on(&globe, 0.2);
        assert_eq!(lines.len(), 6);
        for line in &lines {
            assert!(line.points.len() > 90);
            for &p in &line.points {
                let (lat, lon) = geo_lat_lon_deg(p);
                match line.latitude_deg {
                    Some(expected) => {
                        assert!((lat - expected).abs() <= 0.1, "{:?}: {lat}", line.kind)
                    }
                    None => {
                        if lat.abs() < 89.9 {
                            assert!(lon.abs() <= 0.1, "meridian at {lon}");
                        }
                    }
                }
                // Above the ground: never below the sea-level sphere plus the lift.
                assert!(dot(p, p).sqrt() >= globe.radius() + 0.2 - 1e-3);
            }
        }
        let meridian = &lines[1];
        let (first, _) = geo_lat_lon_deg(meridian.points[0]);
        let (last, _) = geo_lat_lon_deg(*meridian.points.last().unwrap());
        assert!((first.abs() - 90.0).abs() < 0.1 && (last.abs() - 90.0).abs() < 0.1);
        assert!(first * last < 0.0, "the meridian runs pole to pole");
        let tropics: Vec<f32> = lines
            .iter()
            .filter(|l| l.kind == LineKind::Tropic)
            .filter_map(|l| l.latitude_deg)
            .collect();
        assert_eq!(tropics, vec![TROPIC_DEG, -TROPIC_DEG]);
    }

    #[test]
    fn minimap_graticule_rows_match_latitudes() {
        let area = Rect::new(10.0, 20.0, 512.0, 256.0);
        let lines = minimap_graticule(area);
        assert_eq!(lines.len(), 6);
        for (line, (kind, a, b)) in graticule_dirs(64).iter().zip(&lines) {
            assert_eq!(line.kind, *kind);
            // Every point of the line lands on the drawn row or column of the minimap.
            for &p in &line.points {
                let (x, y) = crate::hud_minimap::dir_to_minimap(p, area);
                if line.latitude_deg.is_some() {
                    assert!((y - a.1).abs() < 0.05 && (y - b.1).abs() < 0.05, "{kind:?}");
                    assert!((a.0, b.0) == (area.x, area.right()));
                } else if y > area.y + 0.5 && y < area.bottom() - 0.5 {
                    assert!((x - a.0).abs() < 0.05 && a.0 == b.0, "{x} vs {}", a.0);
                }
            }
        }
        // The equator halves the picture; the tropics and polar circles pair around it.
        let row = |i: usize| lines[i].1.1;
        let middle = area.y + area.h / 2.0;
        assert!((row(0) - middle).abs() < 1e-3);
        assert!((row(2) + row(3) - 2.0 * middle).abs() < 1e-3);
        assert!((row(4) + row(5) - 2.0 * middle).abs() < 1e-3);
        let expected = TROPIC_DEG / 180.0 * area.h;
        assert!(((row(2) - middle).abs() - expected).abs() < 1e-3);
    }

    #[test]
    fn lines_fade_out_at_street_level() {
        assert_eq!(graticule_fade(8.0), 0.0);
        assert_eq!(graticule_fade(FADE_FAR), 1.0);
        assert!(graticule_fade(25.0) > 0.0 && graticule_fade(25.0) < 1.0);
    }
}
