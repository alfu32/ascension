//! Minimap helpers: terrain colours, the map image, and the camera footprint on the ground.
//! Pure functions; the render thread uploads and draws the results.

use std::collections::BTreeMap;

use crate::camera::Camera;
use crate::hud_layout::Rect;
use crate::terrain::{TerrainScene, TileKind};

/// Largest side of the minimap image in pixels; larger maps are sampled.
pub const MAX_MINIMAP_SIDE: u32 = 512;
/// Vertical field of view of the world camera, in degrees.
pub const FOV_DEGREES: f32 = 45.0;

/// Base colour of a tile kind before height shading.
fn base_colour(kind: TileKind, features: &BTreeMap<u16, [u8; 3]>) -> [f32; 3] {
    match kind {
        TileKind::Land => [92.0, 128.0, 66.0],
        TileKind::ShallowSea => [74.0, 142.0, 178.0],
        TileKind::DeepSea => [26.0, 62.0, 110.0],
        TileKind::Space => [4.0, 6.0, 12.0],
        TileKind::Feature(id) => features
            .get(&id)
            .map_or([128.0, 128.0, 128.0], |c| c.map(f32::from)),
    }
}

/// Minimap colour of a tile: the kind's colour, lighter on high ground and darker on low.
pub fn tile_colour(kind: TileKind, height: u8, features: &BTreeMap<u16, [u8; 3]>) -> [u8; 3] {
    let shade = match kind {
        TileKind::Space => 1.0,
        TileKind::ShallowSea | TileKind::DeepSea => 0.92 + 0.16 * f32::from(height) / 255.0,
        _ => 0.72 + 0.56 * f32::from(height) / 255.0,
    };
    base_colour(kind, features).map(|channel| (channel * shade).round().clamp(0.0, 255.0) as u8)
}

/// The map as RGBA pixels, at most [`MAX_MINIMAP_SIDE`] on a side: `(width, height, pixels)`.
pub fn terrain_pixels(scene: &TerrainScene) -> (u32, u32, Vec<u8>) {
    let step = scene
        .width
        .max(scene.height)
        .div_ceil(MAX_MINIMAP_SIDE)
        .max(1);
    let (width, height) = ((scene.width / step).max(1), (scene.height / step).max(1));
    let mut pixels = Vec::with_capacity(width as usize * height as usize * 4);
    for y in 0..height {
        for x in 0..width {
            let index = (y * step) as usize * scene.width as usize + (x * step) as usize;
            let colour = match (scene.kinds.get(index), scene.heights.get(index)) {
                (Some(kind), Some(height)) => tile_colour(*kind, *height, &scene.feature_colors),
                _ => [0, 0, 0],
            };
            pixels.extend_from_slice(&[colour[0], colour[1], colour[2], 255]);
        }
    }
    (width, height, pixels)
}

/// The tile each pixel of [`terrain_pixels`] samples, as a row-major tile index.
pub fn terrain_pixel_tiles(scene: &TerrainScene) -> Vec<u32> {
    let step = scene
        .width
        .max(scene.height)
        .div_ceil(MAX_MINIMAP_SIDE)
        .max(1);
    let (width, height) = ((scene.width / step).max(1), (scene.height / step).max(1));
    let mut tiles = Vec::with_capacity(width as usize * height as usize);
    for y in 0..height {
        for x in 0..width {
            tiles.push(((y * step) as usize * scene.width as usize + (x * step) as usize) as u32);
        }
    }
    tiles
}

/// The largest rectangle with the map's aspect ratio that fits in `view`, centred in it.
pub fn fit_map(map_width: u32, map_height: u32, view: Rect) -> Rect {
    let (mw, mh) = (map_width.max(1) as f32, map_height.max(1) as f32);
    let scale = (view.w / mw).min(view.h / mh);
    let (w, h) = (mw * scale, mh * scale);
    Rect::new(
        view.x + (view.w - w) / 2.0,
        view.y + (view.h - h) / 2.0,
        w,
        h,
    )
}

/// Screen position on the minimap of the tile position `(x, y)`.
pub fn tile_to_minimap(x: f32, y: f32, map_size: (u32, u32), area: Rect) -> (f32, f32) {
    (
        area.x + x / map_size.0.max(1) as f32 * area.w,
        area.y + y / map_size.1.max(1) as f32 * area.h,
    )
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

fn normalised(v: [f32; 3]) -> [f32; 3] {
    let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-6);
    v.map(|c| c / length)
}

/// NDC y of the centre of `area` in a frame of `height` pixels (up is positive).
pub fn ndc_centre_y(area: Rect, height: f32) -> f32 {
    1.0 - 2.0 * area.centre().1 / height.max(1.0)
}

/// Projects the four corners of `area` (screen pixels, in a `width` × `height` frame) onto the
/// ground plane `y = 0`, for a camera whose projection centre was moved to the middle of the
/// area. Returns tile positions `[x, z]` in the order top-left, top-right, bottom-right,
/// bottom-left. Corners whose ray misses the ground are pushed out to `far` tiles.
pub fn ground_footprint(camera: &Camera, frame: (f32, f32), area: Rect, far: f32) -> [[f32; 2]; 4] {
    let eye = camera.position();
    let forward = normalised(sub(camera.target, eye));
    let right = normalised(cross(forward, [0.0, 1.0, 0.0]));
    let up = cross(right, forward);
    let tan = (FOV_DEGREES.to_radians() / 2.0).tan();
    let aspect = frame.0 / frame.1.max(1.0);
    let centre_y = ndc_centre_y(area, frame.1);
    let corners = [
        (area.x, area.y),
        (area.right(), area.y),
        (area.right(), area.bottom()),
        (area.x, area.bottom()),
    ];
    corners.map(|(px, py)| {
        let nx = 2.0 * px / frame.0.max(1.0) - 1.0;
        let ny = 1.0 - 2.0 * py / frame.1.max(1.0);
        let scale_x = nx * aspect * tan;
        let scale_y = (ny - centre_y) * tan;
        let dir = [
            forward[0] + right[0] * scale_x + up[0] * scale_y,
            forward[1] + right[1] * scale_x + up[1] * scale_y,
            forward[2] + right[2] * scale_x + up[2] * scale_y,
        ];
        if dir[1] < -1e-3 {
            let s = (-eye[1] / dir[1]).min(far * 2.0);
            [eye[0] + dir[0] * s, eye[2] + dir[2] * s]
        } else {
            let horizontal = (dir[0] * dir[0] + dir[2] * dir[2]).sqrt().max(1e-6);
            [
                eye[0] + dir[0] / horizontal * far,
                eye[2] + dir[2] / horizontal * far,
            ]
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimap_night_follows_the_light() {
        let (width, height) = GLOBE_MINIMAP_SIZE;
        let dirs = planet_pixel_dirs();
        assert_eq!(dirs.len(), (width * height) as usize);
        let base = vec![200u8; dirs.len() * 4];
        let brightness_at = |pixels: &[u8], lon_deg: f32| {
            let x = ((lon_deg / 360.0 + 0.5) * width as f32) as usize;
            pixels[((height / 2) as usize * width as usize + x) * 4]
        };
        let mut out = Vec::new();
        let noon_at_zero = crate::orbit::dir_from_lon_lat(0.0, 0.0);
        night_pixels(&base, &dirs, noon_at_zero, &mut out);
        assert_eq!(brightness_at(&out, 0.0), 200);
        assert_eq!(brightness_at(&out, 180.0 - 1.0), 90);
        // The light moves a quarter turn east: the bright side follows it.
        let east = crate::orbit::dir_from_lon_lat(90f32.to_radians(), 0.0);
        night_pixels(&base, &dirs, east, &mut out);
        assert_eq!(brightness_at(&out, 90.0), 200);
        assert_eq!(brightness_at(&out, -90.0), 90);
        // On the terminator the brightness is half way between night and day.
        assert!((140..=155).contains(&brightness_at(&out, 0.0)));
        assert!(!light_moved(noon_at_zero, noon_at_zero));
        assert!(light_moved(noon_at_zero, east));
        let small = crate::orbit::dir_from_lon_lat(0.2f32.to_radians(), 0.0);
        assert!(!light_moved(noon_at_zero, small));
    }

    #[test]
    fn declutter_keeps_the_highest_priority_of_overlapping_labels() {
        let boxes = [
            (0.0, 0.0, 50.0, 10.0, 2.0),
            (10.0, 2.0, 50.0, 10.0, 1.0),
            (0.0, 30.0, 50.0, 10.0, 3.0),
        ];
        assert_eq!(declutter(&boxes), vec![1, 2]);
        assert!(declutter(&[]).is_empty());
    }

    #[test]
    fn minimap_colours_follow_tile_kinds() {
        let features = BTreeMap::from([(7_u16, [200, 10, 10])]);
        let land = tile_colour(TileKind::Land, 128, &features);
        let shallow = tile_colour(TileKind::ShallowSea, 128, &features);
        let deep = tile_colour(TileKind::DeepSea, 128, &features);
        let space = tile_colour(TileKind::Space, 128, &features);
        assert!(
            land[1] > land[0] && land[1] > land[2],
            "land is green: {land:?}"
        );
        assert!(
            shallow[2] > shallow[0] && shallow[2] > shallow[1],
            "shallow is blue"
        );
        assert!(deep[2] > deep[0] && deep[2] > deep[1], "deep is blue");
        let luma = |c: [u8; 3]| u32::from(c[0]) + u32::from(c[1]) + u32::from(c[2]);
        assert!(luma(deep) < luma(shallow));
        assert!(luma(space) < luma(deep));
        let feature = tile_colour(TileKind::Feature(7), 128, &features);
        assert!(feature[0] > feature[1] && feature[0] > feature[2]);
        // Unknown features are grey; height shading makes high ground lighter.
        let unknown = tile_colour(TileKind::Feature(99), 128, &features);
        assert!((i32::from(unknown[0]) - i32::from(unknown[2])).abs() <= 2);
        assert!(
            luma(tile_colour(TileKind::Land, 255, &features))
                > luma(tile_colour(TileKind::Land, 0, &features))
        );
    }

    #[test]
    fn terrain_pixels_cover_the_map_and_sample_large_maps() {
        let scene = TerrainScene {
            width: 1100,
            height: 600,
            heights: vec![100; 1100 * 600],
            kinds: vec![TileKind::Land; 1100 * 600],
            feature_colors: BTreeMap::new(),
            starts: Vec::new(),
            nodes: Vec::new(),
        };
        let (w, h, pixels) = terrain_pixels(&scene);
        assert!(w <= MAX_MINIMAP_SIDE && h <= MAX_MINIMAP_SIDE);
        assert_eq!(pixels.len(), (w * h * 4) as usize);
        assert!(pixels.as_chunks::<4>().0.iter().all(|p| p[3] == 255));
    }

    #[test]
    fn an_alert_pulses_at_its_place_and_fades_at_the_end() {
        let area = Rect::new(0.0, 0.0, 200.0, 100.0);
        let alert = |age| crate::HudAlert {
            face: 0,
            x: 50_000,
            y: 25_000,
            age_ticks: age,
            life_ticks: 200,
            kind: crate::AlertMark::Attack,
        };
        let rings = |marks: &[MapMark]| -> Vec<((f32, f32), f32, u8)> {
            marks
                .iter()
                .filter_map(|m| match *m {
                    MapMark::Ring(at, r, c) => Some((at, r, c[3])),
                    _ => None,
                })
                .collect()
        };
        let small = alert_marks(&[alert(0)], (100, 50), area, None, 1.0, 0.0);
        let large = alert_marks(&[alert(0)], (100, 50), area, None, 1.0, 0.5);
        // At the point's place on the map, a ring that grows with the pulse.
        assert!(rings(&small).iter().all(|(at, _, _)| *at == (100.0, 50.0)));
        assert!(rings(&large)[0].1 > rings(&small)[0].1);
        // Full strength while fresh, faded near the end, gone after it.
        let late = alert_marks(&[alert(190)], (100, 50), area, None, 1.0, 0.0);
        assert!(rings(&late)[0].2 < rings(&small)[0].2);
        assert!(alert_marks(&[alert(200)], (100, 50), area, None, 1.0, 0.0).is_empty());
    }

    #[test]
    fn map_fits_the_view_keeping_its_aspect() {
        let view = Rect::new(690.0, 690.0, 240.0, 190.0);
        let fitted = fit_map(256, 256, view);
        assert_eq!((fitted.w, fitted.h), (190.0, 190.0));
        assert!(view.contains_rect(&fitted));
        assert_eq!(fitted.centre(), view.centre());
        let (x, y) = tile_to_minimap(128.0, 64.0, (256, 256), fitted);
        assert!((x - fitted.centre().0).abs() < 1e-3);
        assert!((y - (fitted.y + fitted.h / 4.0)).abs() < 1e-3);
    }

    #[test]
    fn footprint_surrounds_the_target_and_is_symmetric() {
        let camera = Camera {
            target: [100.0, 0.0, 100.0],
            yaw: 0.0,
            pitch: 60_f32.to_radians(),
            distance: 40.0,
            ..Camera::default()
        };
        let frame = (1600.0, 900.0);
        let area = Rect::new(0.0, 130.0, 1600.0, 540.0);
        let footprint = ground_footprint(&camera, frame, area, 1000.0);
        // The centre of the area looks at the target, so the target lies inside the polygon.
        let xs: Vec<f32> = footprint.iter().map(|p| p[0]).collect();
        let zs: Vec<f32> = footprint.iter().map(|p| p[1]).collect();
        let (min_x, max_x) = (
            xs.iter().cloned().fold(f32::MAX, f32::min),
            xs.iter().cloned().fold(f32::MIN, f32::max),
        );
        let (min_z, max_z) = (
            zs.iter().cloned().fold(f32::MAX, f32::min),
            zs.iter().cloned().fold(f32::MIN, f32::max),
        );
        assert!(min_x < 100.0 && max_x > 100.0 && min_z < 100.0 && max_z > 100.0);
        // Looking along -z from +z: the far edge is wider than the near edge.
        let far_width = (footprint[1][0] - footprint[0][0]).abs();
        let near_width = (footprint[2][0] - footprint[3][0]).abs();
        assert!(far_width > near_width);
        // Left and right are mirror images around the target.
        assert!((footprint[0][0] + footprint[1][0] - 200.0).abs() < 0.1);
    }
}

/// An entity dot or camera outline segment, already in minimap screen coordinates.
#[derive(Clone, Copy, Debug)]
pub enum MapMark {
    /// Owner-coloured entity dot.
    Dot(Rect, [u8; 4]),
    /// Camera footprint edge.
    Line((f32, f32), (f32, f32)),
    /// A circle outline: centre, radius and colour.
    Ring((f32, f32), f32, [u8; 4]),
    /// A coloured line segment.
    Segment((f32, f32), (f32, f32), [u8; 4]),
}

/// Current entity dots and ground-plane camera outline; consumers clip to `area`.
pub fn marks(
    entities: &[crate::EntityView],
    camera: &Camera,
    layout: &crate::hud_layout::HudLayout,
    map_size: (u32, u32),
    area: Rect,
) -> Vec<MapMark> {
    let mut marks = Vec::with_capacity(entities.len() + 4);
    for e in entities {
        let (x, y) = tile_to_minimap(e.x, e.y, map_size, area);
        let side = (if e.building { 4.0 } else { 2.0 }) * layout.scale;
        let rgb = (if e.carry_node {
            crate::entities::NODE_COLOR
        } else {
            crate::owner_color(e.owner)
        })
        .map(|c| (c * 255.0).round() as u8);
        marks.push(MapMark::Dot(
            Rect::new(x - side / 2.0, y - side / 2.0, side, side),
            [rgb[0], rgb[1], rgb[2], 255],
        ));
    }
    let points = ground_footprint(
        camera,
        (layout.width, layout.height),
        layout.world_area(),
        map_size.0.max(map_size.1) as f32 * 4.0,
    )
    .map(|p| tile_to_minimap(p[0], p[1], map_size, area));
    for i in 0..4 {
        marks.push(MapMark::Line(points[i], points[(i + 1) % 4]));
    }
    marks
}

/// Size of the sphere minimap image: equirectangular, twice as wide as high.
pub const GLOBE_MINIMAP_SIZE: (u32, u32) = (512, 256);

/// The dense tile index each pixel of [`planet_pixels`] shows.
pub fn planet_pixel_tiles(n: u32) -> Vec<u32> {
    let (width, height) = GLOBE_MINIMAP_SIZE;
    let surface = rom_geo::CubeSurface { n };
    let scale = rom_geo::SPHERE_SCALE as f32;
    let mut tiles = Vec::with_capacity(width as usize * height as usize);
    for y in 0..height {
        let lat = (0.5 - (y as f32 + 0.5) / height as f32) * std::f32::consts::PI;
        for x in 0..width {
            let lon = ((x as f32 + 0.5) / width as f32 - 0.5) * std::f32::consts::TAU;
            let dir = crate::orbit::dir_from_lon_lat(lon, lat);
            let point = crate::globe::to_render(dir).map(|c| (c * scale).round() as i32);
            tiles.push(surface.index(surface.tile_at_point(point)) as u32);
        }
    }
    tiles
}

/// Brightness of the night side of the minimap, as a fraction of the day side.
pub const NIGHT_BRIGHTNESS: f32 = 0.45;
/// Sine of the sun's elevation over which the terminator fades, about four degrees.
const TERMINATOR_WIDTH: f32 = 0.07;

/// The unit direction each pixel of the equirectangular sphere minimap shows.
pub fn planet_pixel_dirs() -> Vec<[f32; 3]> {
    let (width, height) = GLOBE_MINIMAP_SIZE;
    let mut dirs = Vec::with_capacity(width as usize * height as usize);
    for y in 0..height {
        let lat = (0.5 - (y as f32 + 0.5) / height as f32) * std::f32::consts::PI;
        for x in 0..width {
            let lon = ((x as f32 + 0.5) / width as f32 - 0.5) * std::f32::consts::TAU;
            dirs.push(crate::orbit::dir_from_lon_lat(lon, lat));
        }
    }
    dirs
}

/// Brightness factor of a surface point whose normal makes `cosine` with the direction to the
/// light: 1 by day, [`NIGHT_BRIGHTNESS`] by night, smooth across the terminator.
pub fn night_factor(cosine: f32) -> f32 {
    let f = ((cosine + TERMINATOR_WIDTH) / (2.0 * TERMINATOR_WIDTH)).clamp(0.0, 1.0);
    let smooth = f * f * (3.0 - 2.0 * f);
    NIGHT_BRIGHTNESS + (1.0 - NIGHT_BRIGHTNESS) * smooth
}

/// Darkens the pixels of `src` (RGBA) whose direction faces away from the light `toward`.
pub fn night_pixels(src: &[u8], dirs: &[[f32; 3]], toward: [f32; 3], out: &mut Vec<u8>) {
    out.clear();
    for (pixel, dir) in src.as_chunks::<4>().0.iter().zip(dirs) {
        let cosine = dir[0] * toward[0] + dir[1] * toward[1] + dir[2] * toward[2];
        let k = night_factor(cosine);
        out.extend([
            (f32::from(pixel[0]) * k).round() as u8,
            (f32::from(pixel[1]) * k).round() as u8,
            (f32::from(pixel[2]) * k).round() as u8,
            pixel[3],
        ]);
    }
}

/// Whether the light moved far enough from `previous` to change a minimap pixel (about one
/// pixel of longitude).
pub fn light_moved(previous: [f32; 3], now: [f32; 3]) -> bool {
    let cosine = previous[0] * now[0] + previous[1] * now[1] + previous[2] * now[2];
    cosine < (std::f32::consts::TAU / GLOBE_MINIMAP_SIZE.0 as f32).cos()
}

/// Thin economic-radius circles, projected to either surface's minimap.
pub fn city_radius_marks(
    cities: &[crate::hud::HudCityRadius],
    size: (u32, u32),
    area: Rect,
    globe: Option<&crate::Globe>,
) -> Vec<MapMark> {
    use crate::Footing;
    let mut marks = Vec::new();
    for city in cities.iter().filter(|c| c.minimap && c.economic > 0) {
        let project = |i: usize| {
            let angle = i as f32 / 128.0 * std::f32::consts::TAU;
            let r = city.economic as f32 / 1000.0;
            let (x, y) = (city.x as f32 / 1000.0, city.y as f32 / 1000.0);
            let (dx, dy) = (r * angle.cos(), r * angle.sin());
            match globe {
                Some(g) => dir_to_minimap(g.point(city.face, x, y, (dx, dy), 0.0), area),
                None => tile_to_minimap(x + dx, y + dy, size, area),
            }
        };
        for i in 0..128 {
            let (a, b) = (project(i), project(i + 1));
            if (a.0 - b.0).abs() < area.w / 2.0 {
                marks.push(MapMark::Line(a, b));
            }
        }
    }
    marks
}

/// The capital emblems on either surface's minimap (task 131): at each capital's place, larger
/// than a building's dot, faded for one remembered under the fog.
pub(crate) fn capital_emblems(
    capitals: &[crate::HudCapital],
    size: (u32, u32),
    area: Rect,
    globe: Option<&crate::Globe>,
    scale: f32,
) -> Vec<crate::capitals::ScreenEmblem> {
    use crate::Footing;
    capitals
        .iter()
        .map(|c| {
            let (x, y) = (c.x as f32 / 1000.0, c.y as f32 / 1000.0);
            let at = match globe {
                Some(g) => dir_to_minimap(g.point(c.face, x, y, (0.0, 0.0), 0.0), area),
                None => tile_to_minimap(x, y, size, area),
            };
            let (rgb, alpha) = crate::capitals::emblem_colour(c.owner, c.remembered);
            crate::capitals::ScreenEmblem {
                x: at.0,
                y: at.1,
                size: crate::capitals::MINIMAP_EMBLEM * scale,
                rgb,
                alpha,
            }
        })
        .collect()
}

/// Pulsing rings at alerts on the shown surface (task 118): a bright dot and a ring that grows
/// once a second, in the alert's colour, fading over the marker's last quarter.
pub fn alert_marks(
    alerts: &[crate::HudAlert],
    size: (u32, u32),
    area: Rect,
    globe: Option<&crate::Globe>,
    scale: f32,
    pulse: f32,
) -> Vec<MapMark> {
    use crate::Footing;
    let mut marks = Vec::new();
    for alert in alerts {
        let left = alert.life_ticks.saturating_sub(alert.age_ticks) as f32;
        let strength = (left / (alert.life_ticks.max(4) as f32 / 4.0)).clamp(0.0, 1.0);
        if strength <= 0.0 {
            continue;
        }
        let (x, y) = (alert.x as f32 / 1000.0, alert.y as f32 / 1000.0);
        let at = match globe {
            Some(g) => dir_to_minimap(g.point(alert.face, x, y, (0.0, 0.0), 0.0), area),
            None => tile_to_minimap(x, y, size, area),
        };
        let [r, g, b, _] = alert.kind.colour();
        let fade = |a: f32| (a * strength).clamp(0.0, 255.0) as u8;
        let dot = 5.0 * scale;
        marks.push(MapMark::Dot(
            Rect::new(at.0 - dot / 2.0, at.1 - dot / 2.0, dot, dot),
            [r, g, b, fade(255.0)],
        ));
        // A ring that grows from the dot to three times the steady ring's size and fades as it
        // goes, and the steady ring, both two pixels wide.
        let grow = (6.0 + 22.0 * pulse) * scale;
        let alpha = fade(255.0 * (1.0 - pulse));
        let steady = 10.0 * scale;
        for step in [0.0, 1.0] {
            marks.push(MapMark::Ring(at, grow + step, [r, g, b, alpha]));
            marks.push(MapMark::Ring(at, steady + step, [r, g, b, fade(230.0)]));
        }
    }
    marks
}

/// The planet as an equirectangular RGBA image of [`GLOBE_MINIMAP_SIZE`]: longitude runs left to
/// right, latitude from the north pole at the top.
pub fn planet_pixels(planet: &crate::globe::PlanetScene) -> (u32, u32, Vec<u8>) {
    let (width, height) = GLOBE_MINIMAP_SIZE;
    let mut pixels = Vec::with_capacity(width as usize * height as usize * 4);
    for index in planet_pixel_tiles(planet.n) {
        let index = index as usize;
        let colour = tile_colour(
            planet.kinds[index],
            planet.heights[index],
            &planet.feature_colors,
        );
        pixels.extend_from_slice(&[colour[0], colour[1], colour[2], 255]);
    }
    (width, height, pixels)
}

/// Screen position on the equirectangular minimap of a direction from the planet centre.
pub fn dir_to_minimap(dir: [f32; 3], area: Rect) -> (f32, f32) {
    let (lon, lat) = crate::orbit::lon_lat_of(dir);
    (
        area.x + (lon / std::f32::consts::TAU + 0.5) * area.w,
        area.y + (0.5 - lat / std::f32::consts::PI) * area.h,
    )
}

/// A dot on the sphere minimap: direction, colour and side in design pixels.
pub type GlobeDot = ([f32; 3], [u8; 4], f32);

/// Entity dots and a cross at the camera direction, in minimap screen coordinates.
pub fn globe_marks(
    orbit: &crate::orbit::OrbitCamera,
    dots: &[GlobeDot],
    layout: &crate::hud_layout::HudLayout,
    area: Rect,
) -> Vec<MapMark> {
    let mut marks = Vec::with_capacity(dots.len() + 2);
    for &(dir, colour, side) in dots {
        let (x, y) = dir_to_minimap(dir, area);
        let side = side * layout.scale;
        marks.push(MapMark::Dot(
            Rect::new(x - side / 2.0, y - side / 2.0, side, side),
            colour,
        ));
    }
    let (x, y) = dir_to_minimap(orbit.dir, area);
    let arm = 6.0 * layout.scale;
    marks.push(MapMark::Line((x - arm, y), (x + arm, y)));
    marks.push(MapMark::Line((x, y - arm), (x, y + arm)));
    marks
}

/// Marks of the minimap above the planet range: a small diagram of the system (orbits, bodies,
/// ships, the part of the plane in view) or the star map (systems, routes, ships, the part in
/// view). Drawn into `area`, which is fitted to a square; north (`-z`) is up.
pub fn space_marks(
    zoom: &crate::zoom::ZoomCamera,
    orbit: &crate::orbit::OrbitCamera,
    space: &crate::space::SpaceSnapshot,
    layout: &crate::hud_layout::HudLayout,
    area: Rect,
) -> Vec<MapMark> {
    use crate::universe::{BodyViewKind, SYSTEM_RADIUS};
    use crate::zoom::Level;
    let side = area.w.min(area.h);
    let (cx, cy) = area.centre();
    let half = side / 2.0 * 0.92;
    let dot = |x: f32, y: f32, size: f32, colour: [u8; 4]| {
        let size = size * layout.scale;
        MapMark::Dot(
            Rect::new(x - size / 2.0, y - size / 2.0, size, size),
            colour,
        )
    };
    let owner = |o: u8| {
        let [r, g, b] = crate::entities::owner_color(o).map(|c| (c * 255.0).round() as u8);
        [r, g, b, 255]
    };
    let mut marks = Vec::new();
    let scene = zoom.scene();
    let map = zoom.level() >= Level::Cluster;
    let (middle, reach) = if map {
        zoom.minimap_frame()
    } else {
        ([0.0; 3], SYSTEM_RADIUS)
    };
    let scale = half / reach.max(1.0);
    let to_area = |p: [f32; 3]| {
        (
            cx + (p[0] - middle[0]) * scale,
            cy + (p[2] - middle[2]) * scale,
        )
    };
    if !map {
        for placed in zoom.placed() {
            let body = &scene.bodies[placed.body];
            let (x, y) = to_area(placed.pos);
            match body.kind {
                BodyViewKind::Star
                    if scene
                        .systems
                        .iter()
                        .any(|s| s.id == body.system && s.celestial) =>
                {
                    marks.push(dot(x, y, 9.0, [255, 150, 60, 255]));
                    marks.push(dot(x, y, 5.0, [0, 0, 0, 255]));
                }
                BodyViewKind::Star => marks.push(dot(x, y, 8.0, [255, 230, 150, 255])),
                BodyViewKind::Celestial => {
                    marks.push(MapMark::Ring(
                        (cx, cy),
                        placed.orbit * scale,
                        [150, 130, 90, 255],
                    ));
                    marks.push(dot(x, y, 7.0, crate::universe::CELESTIAL_RGBA));
                }
                BodyViewKind::Habitable => {
                    marks.push(MapMark::Ring(
                        (cx, cy),
                        placed.orbit * scale,
                        [90, 110, 150, 255],
                    ));
                    let rgb = body
                        .home_race_colour
                        .unwrap_or(crate::universe::planet_colour(body.id));
                    marks.push(dot(x, y, 6.5, [rgb[0], rgb[1], rgb[2], 255]));
                }
                BodyViewKind::Resource => {
                    // Moons and belt bodies carry no ring of their own; the belt gets one below.
                    // Each is a dot in the colour of its resource, by its drawn size and always
                    // smaller than a planet's.
                    let rgb = space
                        .resource_body(body.id)
                        .map_or([170, 160, 140], |r| r.colour);
                    let size = 3.0 + 2.5 * placed.radius / crate::universe::SMALLEST_PLANET_RADIUS;
                    marks.push(dot(x, y, size, [rgb[0], rgb[1], rgb[2], 255]));
                }
            }
        }
        let belt: Vec<f32> = zoom
            .placed()
            .iter()
            .filter(|p| scene.bodies[p.body].kind == BodyViewKind::Resource && p.moon_of.is_none())
            .map(|p| p.orbit)
            .collect();
        if !belt.is_empty() {
            let mean = belt.iter().sum::<f32>() / belt.len() as f32;
            marks.push(MapMark::Ring((cx, cy), mean * scale, [60, 72, 100, 255]));
        }
        if let Some(system) = zoom.system_id() {
            let outermost = scene.system_outermost(system);
            for ship in space.ships.iter().filter(|s| s.system == system) {
                let at = crate::space::ship_frame_pos(scene, zoom.placed(), outermost, ship);
                let (x, y) = to_area(at);
                marks.push(dot(x, y, 4.0, owner(ship.owner)));
            }
            for transit in &space.transits {
                if let Some((at, _)) = crate::space::transit_at_rim(
                    scene,
                    &zoom.map_positions(),
                    zoom.system(),
                    transit,
                ) {
                    let (x, y) = to_area(at);
                    marks.push(dot(x, y, 4.0, owner(transit.owner)));
                }
            }
        }
    } else {
        let positions = zoom.map_positions();
        let shown = zoom.map_shown();
        let on_disk = zoom.on_disk();
        // Charted routes whole; an uncharted one as a stub leaving each end the player has seen.
        // In the cluster view the routes of the cluster, those leaving it to their exits.
        for &(id_a, id_b) in &scene.routes {
            let (Some(a), Some(b)) = (scene.system_index(id_a), scene.system_index(id_b)) else {
                continue;
            };
            if shown[a].reach.min(shown[b].reach) < 0.5 {
                continue;
            }
            let (pa, pb) = (positions[a], positions[b]);
            let stub = |from: [f32; 3], to: [f32; 3]| {
                [0, 1, 2].map(|i| from[i] + (to[i] - from[i]) * crate::space::ROUTE_STUB_SHARE)
            };
            match crate::space::route_sight(space, id_a, id_b) {
                crate::space::RouteSight::Charted => {
                    marks.push(MapMark::Segment(
                        to_area(pa),
                        to_area(pb),
                        [70, 90, 130, 255],
                    ));
                }
                crate::space::RouteSight::Stub { from_a, from_b } => {
                    for (from, to, shown) in [(pa, pb, from_a), (pb, pa, from_b)] {
                        if shown {
                            marks.push(MapMark::Segment(
                                to_area(from),
                                to_area(stub(from, to)),
                                [60, 75, 105, 200],
                            ));
                        }
                    }
                }
                crate::space::RouteSight::Hidden => {}
            }
        }
        let tint_of = |system: u32| {
            scene
                .bodies
                .iter()
                .find(|b| b.system == system && b.home_race_colour.is_some())
                .and_then(|b| b.home_race_colour)
        };
        if on_disk {
            // On the disk each cluster is a dot, larger with more systems, in the colour of a
            // race at home there.
            for group in &zoom.layout().groups {
                let tint = group
                    .members
                    .iter()
                    .find_map(|&i| tint_of(scene.systems[i].id));
                let rgb = tint.unwrap_or(if group.celestial {
                    crate::universe::ACCRETION_RGB[1]
                } else {
                    [200, 215, 235]
                });
                let size =
                    2.5 + 0.6 * group.members.len() as f32 + if tint.is_some() { 1.5 } else { 0.0 };
                let (x, y) = to_area(group.centre);
                marks.push(dot(x, y, size, [rgb[0], rgb[1], rgb[2], 255]));
            }
        } else {
            for (i, system) in scene.systems.iter().enumerate() {
                if shown[i].opacity < 0.5 {
                    continue;
                }
                let tint = tint_of(system.id);
                let rgb = tint.unwrap_or(system.star_colour);
                let size = if tint.is_some() { 5.0 } else { 3.0 };
                let (x, y) = to_area(positions[i]);
                // Seen from above; a system's height shows as its brightness: lower is dimmer.
                let height = ((positions[i][1] - middle[1]) / reach.max(1.0)).clamp(-1.0, 1.0);
                let alpha = (255.0 * (0.72 + 0.28 * height)) as u8;
                marks.push(dot(x, y, size, [rgb[0], rgb[1], rgb[2], alpha]));
            }
        }
        let beside = [half * 0.05 / scale, 0.0, 0.0];
        for mark in crate::space::map_ship_marks(scene, &positions, &shown, space, beside) {
            let (x, y) = to_area(mark.at);
            marks.push(dot(x, y, 4.0, owner(mark.owner)));
        }
    }
    // Alerts (task 118): a pulsing ring at the system on the star map, at the planet or the
    // arrival point in the system.
    let pulse = crate::space::alert_pulse();
    for alert in &space.alerts {
        let at = if map {
            scene
                .system_index(alert.system)
                .map(|i| zoom.map_positions()[i])
        } else if zoom.system_id() == Some(alert.system) {
            match alert.body.and_then(|b| scene.body_index(b)) {
                Some(body) => zoom.placed().iter().find(|p| p.body == body).map(|p| p.pos),
                None => Some(crate::space::free_frame_pos(
                    alert.x,
                    alert.y,
                    scene.system_outermost(alert.system),
                )),
            }
        } else {
            None
        };
        let Some(at) = at else { continue };
        let (x, y) = to_area(at);
        let [r, g, b, _] = alert.kind.colour();
        let strength = alert.strength();
        let alpha = |a: f32| (a * strength).clamp(0.0, 255.0) as u8;
        let grow = (5.0 + 10.0 * pulse) * layout.scale;
        marks.push(MapMark::Ring(
            (x, y),
            grow,
            [r, g, b, alpha(255.0 * (1.0 - pulse))],
        ));
        marks.push(MapMark::Ring(
            (x, y),
            7.0 * layout.scale,
            [r, g, b, alpha(220.0)],
        ));
    }
    // The part of the plane in view, a trapezoid for a tilted camera (cut off where the view
    // reaches toward the horizon; none on the free star map, which has no plane), and the
    // focus.
    let canvas = (layout.width, layout.height);
    let world = layout.world_area();
    let centre_y = ndc_centre_y(world, layout.height);
    let view = zoom.space_view_at(orbit, canvas, centre_y);
    let corners = [
        (0.0, world.y),
        (canvas.0, world.y),
        (canvas.0, world.bottom()),
        (0.0, world.bottom()),
    ]
    .map(|(x, y)| zoom.plane_under(&view, x, y).map(to_area));
    if corners.iter().all(Option::is_some) {
        let corners = corners.map(|c| c.unwrap_or_default());
        for i in 0..4 {
            marks.push(MapMark::Line(corners[i], corners[(i + 1) % 4]));
        }
    }
    let target = if map {
        zoom.map_target(orbit)
    } else {
        zoom.system_target(orbit)
    };
    let (x, y) = to_area(target);
    let arm = 6.0 * layout.scale;
    marks.push(MapMark::Line((x - arm, y), (x + arm, y)));
    marks.push(MapMark::Line((x, y - arm), (x, y + arm)));
    marks
}

/// The point of the space shown in the minimap under canvas pixel `(x, y)`: in the anchored
/// system's frame, or on the star map (then `true`). `None` outside the fitted diagram.
pub fn space_minimap_point(
    zoom: &crate::zoom::ZoomCamera,
    layout: &crate::hud_layout::HudLayout,
    x: f32,
    y: f32,
) -> Option<([f32; 3], bool)> {
    use crate::universe::SYSTEM_RADIUS;
    let area = layout.rect(crate::hud_layout::Region::MinimapView);
    let side = area.w.min(area.h);
    let (cx, cy) = area.centre();
    if (x - cx).abs() > side / 2.0 || (y - cy).abs() > side / 2.0 {
        return None;
    }
    let half = side / 2.0 * 0.92;
    let map = zoom.level() >= crate::zoom::Level::Cluster;
    let (middle, reach) = if map {
        zoom.minimap_frame()
    } else {
        ([0.0; 3], SYSTEM_RADIUS)
    };
    let scale = half / reach.max(1.0);
    Some((
        [
            middle[0] + (x - cx) / scale,
            0.0,
            middle[2] + (y - cy) / scale,
        ],
        map,
    ))
}

/// Which labels fit: greedy by priority, a label that would overlap one already kept is dropped.
/// Each box is `(x, y, width, height)`; returns the indices kept, in priority order.
pub fn declutter(boxes: &[(f32, f32, f32, f32, f32)]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..boxes.len()).collect();
    order.sort_by(|&a, &b| boxes[a].4.total_cmp(&boxes[b].4));
    let mut kept: Vec<usize> = Vec::new();
    for i in order {
        let (x, y, w, h, _) = boxes[i];
        let clear = kept.iter().all(|&k| {
            let (kx, ky, kw, kh, _) = boxes[k];
            x + w + 2.0 <= kx || kx + kw + 2.0 <= x || y + h + 2.0 <= ky || ky + kh + 2.0 <= y
        });
        if clear {
            kept.push(i);
        }
    }
    kept
}
