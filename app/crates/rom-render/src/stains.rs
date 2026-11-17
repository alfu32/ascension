//! Stains on the ground where buildings and units were destroyed (task 80), as vertex data for
//! the two stain meshes made before play: soot, ash and churned earth laid over the land tiles
//! under a stain (copies of the terrain's own triangles, so they follow the relief exactly; the
//! shader draws the ragged mark and its fading), and a building's debris lying on the ground in
//! its colours. Nothing here touches raylib.

use crate::HudStain;
use crate::entities::Footing;
use crate::globe::Globe;
use crate::terrain::TileKind;
use rom_geo::SurfacePos;

/// Vertices the soot mesh holds: 43 690 triangles.
pub(crate) const SOOT_CAPACITY: usize = 3 * 43_690;
/// Vertices the debris mesh holds: 21 845 triangles.
pub(crate) const DEBRIS_CAPACITY: usize = 3 * 21_845;
/// `StainVertex::local[0]` of a debris vertex is above this; a soot vertex's is a few radii at
/// most.
pub(crate) const DEBRIS_MARK: f32 = 8.0;
/// How far past its radius a stain's ragged rim may reach, in radii.
const RIM_REACH: f32 = 1.15;
/// Share of a building stain's radius its debris is scattered over.
const DEBRIS_SPREAD: f32 = 0.8;

/// One vertex of the stain meshes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct StainVertex {
    /// Position in planet space.
    pub pos: [f32; 3],
    /// Ground normal (soot) or face normal (debris).
    pub normal: [f32; 3],
    /// Debris colour; soot ignores the colour channels.
    pub colour: [u8; 4],
    /// Game seconds the stain was left at, and its seed in 0..1.
    pub stamp: [f32; 2],
    /// Soot: offset from the stain's centre in its radii, along the face's x and y. Debris:
    /// [`DEBRIS_MARK`] + 1 and the vertex's height above the ground, in tiles (it settles into
    /// the ground as the stain fades).
    pub local: [f32; 2],
}

/// How strongly a stain shows at `age`, a share of its life: it fades smoothly, most of it in
/// the last third (the stain shader computes the same).
pub(crate) fn fade(age: f32) -> f32 {
    let a = age.clamp(0.0, 1.0);
    let t = ((a - 0.6) / 0.4).clamp(0.0, 1.0);
    (1.0 - 0.45 * a) * (1.0 - t * t * (3.0 - 2.0 * t))
}

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

/// The seed that varies a stain's look, from the id of what left it.
pub(crate) fn seed_of(stain: &HudStain) -> u32 {
    stain.id.wrapping_mul(2_654_435_761) ^ 0x5bd1_e995
}

fn is_land(kind: TileKind) -> bool {
    matches!(kind, TileKind::Land | TileKind::Feature(_))
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
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
        [0.0, 1.0, 0.0]
    } else {
        v.map(|c| c / length)
    }
}

fn add(a: [f32; 3], b: [f32; 3], s: f32) -> [f32; 3] {
    [a[0] + b[0] * s, a[1] + b[1] * s, a[2] + b[2] * s]
}

/// The tile index under the point `delta` tiles from `from`, if it is on land.
fn land_under(globe: &Globe, kinds: &[TileKind], from: SurfacePos, delta: (f32, f32)) -> bool {
    let at = globe.step_on(
        from,
        (delta.0 * 1000.0).round() as i64,
        (delta.1 * 1000.0).round() as i64,
    );
    kinds
        .get(globe.surface().index(at.tile()))
        .is_some_and(|k| is_land(*k))
}

/// Appends the soot of `stain`: the terrain's triangles of every land tile its rim may reach;
/// water tiles are left out, so a stain stops at the shore.
pub(crate) fn soot(
    stain: &HudStain,
    born: f32,
    globe: &Globe,
    kinds: &[TileKind],
    out: &mut Vec<StainVertex>,
) {
    let radius = (stain.radius as f32 / 1000.0).max(0.1);
    let (cx, cy) = (stain.x as f32 / 1000.0, stain.y as f32 / 1000.0);
    let centre = SurfacePos {
        face: stain.face,
        x: i64::from(stain.x),
        y: i64::from(stain.y),
    };
    let frame = globe.ground_frame(stain.face, cx, cy);
    let reach = radius * RIM_REACH;
    let steps = reach.ceil() as i32 + 1;
    let surface = globe.surface();
    let mut tiles = Vec::new();
    // Cell centres on the tile grid of the stain's face, stepped across face edges.
    let (ox, oy) = (cx.floor() + 0.5 - cx, cy.floor() + 0.5 - cy);
    for j in -steps..=steps {
        for i in -steps..=steps {
            let (dx, dy) = (ox + i as f32, oy + j as f32);
            // The nearest point of the cell must lie within the rim.
            let (nx, ny) = ((dx.abs() - 0.5).max(0.0), (dy.abs() - 0.5).max(0.0));
            if nx * nx + ny * ny > reach * reach {
                continue;
            }
            let at = globe.step_on(
                centre,
                (dx * 1000.0).round() as i64,
                (dy * 1000.0).round() as i64,
            );
            let index = surface.index(at.tile());
            if kinds.get(index).is_some_and(|k| is_land(*k)) {
                tiles.push(index);
            }
        }
    }
    tiles.sort_unstable();
    tiles.dedup();
    let seed = seed_of(stain) as f32 / u32::MAX as f32;
    let vertex = |face: u8, vx: u32, vy: u32| {
        let pos = globe.vertex(face, vx, vy);
        let d = sub(pos, frame.origin);
        StainVertex {
            pos,
            normal: globe.vertex_normal(face, vx, vy),
            colour: [0, 0, 0, 255],
            stamp: [born, seed],
            local: [dot(d, frame.ex) / radius, dot(d, frame.ey) / radius],
        }
    };
    for index in tiles {
        let tile = surface.tile(index);
        let (f, x, y) = (tile.face, tile.x, tile.y);
        // The two triangles of the land meshes, split along the same diagonal.
        let (v00, v10, v01, v11) = (
            vertex(f, x, y),
            vertex(f, x + 1, y),
            vertex(f, x, y + 1),
            vertex(f, x + 1, y + 1),
        );
        out.extend_from_slice(&[v00, v01, v10, v10, v01, v11]);
    }
}

/// Colours a building's debris is made of when its model is not known: stone, timber, plaster.
pub(crate) const PLAIN_DEBRIS: [[u8; 3]; 3] = [[128, 122, 112], [112, 82, 56], [186, 176, 158]];

/// Appends the debris of a destroyed building: low broken pieces lying on the ground over most
/// of its stain, in the colours of its model (`palette`) and a few in its owner's `team`
/// colour, some charred; none on water.
pub(crate) fn debris(
    stain: &HudStain,
    born: f32,
    globe: &Globe,
    kinds: &[TileKind],
    palette: &[[u8; 3]; 3],
    team: [u8; 3],
    out: &mut Vec<StainVertex>,
) {
    let radius = (stain.radius as f32 / 1000.0).max(0.1);
    let (cx, cy) = (stain.x as f32 / 1000.0, stain.y as f32 / 1000.0);
    let centre = SurfacePos {
        face: stain.face,
        x: i64::from(stain.x),
        y: i64::from(stain.y),
    };
    let seed = seed_of(stain);
    let stamp = [born, seed as f32 / u32::MAX as f32];
    let spread = radius * DEBRIS_SPREAD;
    let count = (radius * radius * 1.8).clamp(6.0, 90.0) as u32;
    // Larger buildings break into larger pieces.
    let scale = 0.7 + 0.3 * (radius / 4.0).min(1.5);
    let up = globe.ground_frame(stain.face, cx, cy).up;
    for i in 0..count {
        let n = |k: u32| noise(seed, i * 16 + k);
        // More pieces near the middle, where the building stood.
        let angle = n(0) * std::f32::consts::TAU;
        let distance = spread * n(1).sqrt() * (0.35 + 0.65 * n(2));
        let (px, py) = (distance * angle.cos(), distance * angle.sin());
        if !land_under(globe, kinds, centre, (px, py)) {
            continue;
        }
        let beam = n(3) < 0.22;
        let size = (0.14 + 0.26 * n(4)) * scale;
        let height = size * (0.25 + 0.35 * n(5));
        let turn = n(6) * std::f32::consts::TAU;
        let (c, s) = (turn.cos(), turn.sin());
        // The outline on the ground: a slab or a broken lump of three to five corners.
        let outline: Vec<(f32, f32)> = if beam {
            let (l, w) = (size * (1.6 + 1.4 * n(7)), size * 0.28);
            vec![(-l, -w), (l, -w), (l, w), (-l, w)]
        } else {
            let corners = 3 + (n(7) * 3.0) as u32;
            (0..corners)
                .map(|k| {
                    let a = (k as f32 + 0.35 * n(8 + k)) / corners as f32 * std::f32::consts::TAU;
                    let r = size * (0.65 + 0.5 * n(12 + k % 4));
                    (r * a.cos(), r * a.sin())
                })
                .collect()
        };
        let outline: Vec<(f32, f32)> = outline
            .iter()
            .map(|(x, y)| (px + x * c - y * s, py + x * s + y * c))
            .collect();
        let rgb = {
            let pick = n(9);
            let base = if pick < 0.12 {
                team
            } else {
                palette[((pick - 0.12) / 0.88 * 3.0).min(2.0) as usize]
            };
            let charred = n(10) < 0.3;
            let shade = 0.55 + 0.4 * n(11);
            base.map(|v| {
                let v = f32::from(v) * shade;
                (if charred { v * 0.35 + 12.0 } else { v }).min(255.0) as u8
            })
        };
        let colour = [rgb[0], rgb[1], rgb[2], 255];
        let base: Vec<[f32; 3]> = outline
            .iter()
            .map(|d| globe.point(stain.face, cx, cy, *d, -0.02))
            .collect();
        // The top leans: one side higher than the other, and narrower than the base.
        let tilt = (n(13) - 0.5) * 0.8;
        let top: Vec<([f32; 3], f32)> = outline
            .iter()
            .map(|(x, y)| {
                let (tx, ty) = (px + (x - px) * 0.78, py + (y - py) * 0.78);
                let lean = 1.0 + tilt * ((x - px) * c + (y - py) * s) / size.max(0.01);
                let h = (height * lean).max(0.02);
                (
                    add(globe.point(stain.face, cx, cy, (tx, ty), 0.0), up, h),
                    h,
                )
            })
            .collect();
        let middle = globe.point(stain.face, cx, cy, (px, py), height * 0.5);
        let mut tri = |a: ([f32; 3], f32), b: ([f32; 3], f32), c: ([f32; 3], f32)| {
            let mut normal = unit(cross(sub(b.0, a.0), sub(c.0, a.0)));
            let centroid = [0, 1, 2].map(|k| (a.0[k] + b.0[k] + c.0[k]) / 3.0);
            if dot(normal, sub(centroid, middle)) < 0.0 {
                normal = normal.map(|v| -v);
            }
            for (p, h) in [a, b, c] {
                out.push(StainVertex {
                    pos: p,
                    normal,
                    colour,
                    stamp,
                    local: [DEBRIS_MARK + 1.0, h],
                });
            }
        };
        let k = outline.len();
        for m in 1..k - 1 {
            tri(top[0], top[m], top[m + 1]);
        }
        for m in 0..k {
            let next = (m + 1) % k;
            let (b0, b1) = ((base[m], 0.0), (base[next], 0.0));
            tri(b0, b1, top[next]);
            tri(b0, top[next], top[m]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::globe::PlanetScene;
    use std::collections::BTreeMap;

    /// Level land on every face, with a strip of sea across face 0 from x = 20.
    fn globe() -> (Globe, Vec<TileKind>) {
        let n = 32;
        let tiles = 6 * (n * n) as usize;
        let kinds: Vec<TileKind> = (0..tiles)
            .map(|i| {
                let (face, x) = (i / (n * n) as usize, i % n as usize);
                if face == 0 && x >= 20 {
                    TileKind::ShallowSea
                } else {
                    TileKind::Land
                }
            })
            .collect();
        let planet = PlanetScene {
            n,
            kinds: kinds.clone(),
            heights: vec![140; tiles],
            feature_colors: BTreeMap::new(),
            starts: Vec::new(),
            nodes: Vec::new(),
            sun: Default::default(),
        };
        (Globe::new(&planet), kinds)
    }

    fn stain(x: i32, radius: i32, building: bool) -> HudStain {
        HudStain {
            id: 41,
            face: 0,
            x,
            y: 10_000,
            radius,
            tick: 200,
            building,
            owner: 2,
            model: None,
        }
    }

    /// A larger stain covers more ground; every soot vertex is a terrain vertex, and the mark
    /// is centred where the thing stood.
    #[test]
    fn soot_covers_the_land_under_the_stain() {
        let (globe, kinds) = globe();
        let (mut small, mut large) = (Vec::new(), Vec::new());
        soot(&stain(8_000, 625, false), 10.0, &globe, &kinds, &mut small);
        soot(&stain(8_000, 3_400, true), 10.0, &globe, &kinds, &mut large);
        assert!(!small.is_empty() && small.len() % 6 == 0);
        assert!(
            large.len() > 8 * small.len(),
            "{} {}",
            large.len(),
            small.len()
        );
        let inside = large
            .iter()
            .filter(|v| v.local[0].hypot(v.local[1]) <= 1.0)
            .count();
        assert!(inside > 0);
        let reach = large
            .iter()
            .map(|v| v.local[0].hypot(v.local[1]))
            .fold(0.0_f32, f32::max);
        assert!(reach < RIM_REACH + 0.6, "{reach}");
        assert!(
            large
                .iter()
                .all(|v| v.stamp[0] == 10.0 && v.local[0] < DEBRIS_MARK)
        );
    }

    /// A stain on the shore stops at the water: no soot over sea tiles, no debris in the sea.
    #[test]
    fn a_stain_at_the_shore_stops_at_the_water() {
        let (globe, kinds) = globe();
        let shore = stain(19_000, 4_000, true);
        let mut out = Vec::new();
        soot(&shore, 0.0, &globe, &kinds, &mut out);
        // The middle of every soot triangle, and of every debris piece's base, is over land.
        let over_land = |p: [f32; 3]| {
            let tile = globe.pos_of(p).tile();
            is_land(kinds[globe.surface().index(tile)])
        };
        assert!(out.len() > 60);
        assert!(
            out.chunks(3).all(|t| over_land(
                [0, 1, 2].map(|k| (t[0].pos[k] + t[1].pos[k] + t[2].pos[k]) / 3.0)
            )),
            "soot over the sea"
        );
        // Tiles beside the shore are stained.
        assert!(out.iter().any(|v| globe.pos_of(v.pos).x >= 19_500));
        let mut pieces = Vec::new();
        debris(
            &shore,
            0.0,
            &globe,
            &kinds,
            &PLAIN_DEBRIS,
            [220, 60, 60],
            &mut pieces,
        );
        assert!(!pieces.is_empty());
        assert!(
            pieces
                .iter()
                .filter(|v| v.local[1] == 0.0)
                .all(|v| globe.pos_of(v.pos).x < 21_000)
        );
    }

    /// Debris lies low on the ground, in the building's colours, the same every time.
    #[test]
    fn debris_lies_low_and_is_stable() {
        let (globe, kinds) = globe();
        let barracks = stain(8_000, 3_393, true);
        let mut a = Vec::new();
        debris(
            &barracks,
            3.0,
            &globe,
            &kinds,
            &PLAIN_DEBRIS,
            [60, 110, 230],
            &mut a,
        );
        let mut b = Vec::new();
        debris(
            &barracks,
            3.0,
            &globe,
            &kinds,
            &PLAIN_DEBRIS,
            [60, 110, 230],
            &mut b,
        );
        assert_eq!(a, b);
        assert!(a.len() % 3 == 0 && a.len() > 100);
        assert!(
            a.iter()
                .all(|v| v.local[0] > DEBRIS_MARK && v.local[1] < 0.5)
        );
        // Stone, timber and plaster, and a few pieces in the owner's blue.
        assert!(a.iter().any(|v| v.colour[0] > v.colour[2]));
        assert!(a.iter().any(|v| v.colour[2] > v.colour[0]));
    }

    /// A stain keeps most of its strength for two thirds of its life, then fades out.
    #[test]
    fn stains_fade_mostly_in_the_last_third() {
        assert_eq!(fade(0.0), 1.0);
        assert!(fade(0.33) > 0.8);
        assert!(fade(0.66) > 0.6);
        assert!(fade(0.85) < 0.3);
        assert_eq!(fade(1.0), 0.0);
        let mut last = 1.0;
        for i in 0..=100 {
            let f = fade(i as f32 / 100.0);
            assert!(f <= last + 1e-6);
            last = f;
        }
    }
}
