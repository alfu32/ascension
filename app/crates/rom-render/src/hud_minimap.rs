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
        let rgb = crate::owner_color(e.owner).map(|c| (c * 255.0).round() as u8);
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
