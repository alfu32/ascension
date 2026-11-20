//! Grass blades near the camera (task 83): Meshcraft's parametric grass program
//! (`meshcraft.grass.v1`, generation "blades") grown on the ground triangles around the eye,
//! with the parameter values of the owner's material (density 480, height 0.25, width 0.1,
//! curvature 0.3, wind 0.15, colour variation 0.25, seed 1).
//!
//! As in Meshcraft, the CPU only lists source triangles ("domains"); every blade is placed,
//! shaped and coloured on the GPU from the triangle's seed and its candidate number, every
//! frame (`assets/shaders/grass.vs`, `grass.fs`). Nothing here is stored or reaches the
//! simulation.
//!
//! Kept from Meshcraft: the blade generator (seeded root in the triangle, random facing, three
//! tapering segments bent by curvature and wind, the fractional last candidate), its hash and
//! random numbers, the seeded brightness variation and darker base, the flat blade normal seen
//! from either side, the candidate scheme (one instanced draw, the domain advancing every
//! "blades per triangle" instances) and its LOD falloff. What the game's frame changes:
//! - the domains are the land triangles within the far edge of the [`Reach`] that wear grass,
//!   in view, off building footprints and stains, rebuilt as the eye moves;
//! - the LOD is applied per triangle instead of per placed object, with a game distance
//!   ([`Reach::lod`] instead of 3500), and multiplied by the far fade; past [`Reach::coarse`]
//!   the blades thin out faster and grow wider (task 108), so at the higher [`Detail`] levels the
//!   patches show farther for few more blades; one triangle's
//!   candidates are split over four draws of 64, 16, 4 and 1 candidates per domain entry, so no
//!   draw carries many discarded candidates;
//! - the candidate budget ([`Reach::budget`]) bounds the candidates of the whole frame instead of one
//!   material's (5500); when the view needs more, the density is scaled down evenly;
//! - the base colour is the grass material of the ground under each blade (the terrain's own
//!   material slot and map shade), so the material-colour uniform is dropped; the lighting is
//!   the terrain's (the light that represents the sun, night and the shadow map), so Meshcraft's
//!   ambient, specular and PBR terms, base texture and depth pass (blades cast no shadow) are
//!   dropped, as are its other generators (foliage, trees, surfaces).
//!
//! Nothing here touches raylib.

use std::collections::{BTreeSet, VecDeque};

use crate::globe::{ChunkBounds, CullView, Globe};
use crate::hud_model::HudStain;
use crate::looks::Program;
use crate::materials::{MAX_SLOTS, SurfaceSlots};
use crate::terrain::TileKind;
use rom_geo::{SurfacePos, Tile};

/// Blades per square tile at full detail (Meshcraft parameter "Density").
pub const DENSITY: f32 = 480.0;
/// Blade length in tiles ("Blade height").
pub const HEIGHT: f32 = 0.25;
/// Blade base width in tiles ("Blade width").
pub const WIDTH: f32 = 0.1;
/// Tip bend relative to the height ("Curvature").
pub const CURVATURE: f32 = 0.3;
/// Animated tip displacement relative to the height ("Wind strength").
pub const WIND: f32 = 0.15;
/// Seeded variation in blade brightness ("Color variation").
pub const VARIATION: f32 = 0.25;
/// Reproducible distribution ("Seed").
pub const SEED: u32 = 1;
/// How far the blades reach and how many are drawn: the "rendering detail" setting's part for
/// the grass (task 108). All distances are in tiles from the eye.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reach {
    /// Meshcraft's LOD distance: the density halves there.
    pub lod: f32,
    /// Blades keep their full height up to here.
    pub full: f32,
    /// The last blades have thinned and shrunk away here.
    pub far: f32,
    /// Past here the blades are coarser: the density falls by the square of the distance's
    /// ratio to it and each blade is that ratio wider.
    pub coarse: f32,
    /// Most candidates (blades before thinning) drawn in one frame within `coarse`.
    pub budget: usize,
    /// Most candidates drawn in one frame past `coarse`, besides `budget`: the far blades never
    /// thin the near ones.
    pub far_budget: usize,
}

impl Reach {
    /// Meshcraft's LOD falloff times the coarse thinning and the far fade: the share of
    /// [`DENSITY`] kept `distance` tiles from the eye.
    pub fn lod(&self, distance: f32) -> f32 {
        let d = distance / self.lod;
        let wider = self.widen(distance);
        1.0 / (1.0 + d * d) / (wider * wider) * self.height_fade(distance)
    }

    /// How many times wider than near ones the blades are `distance` tiles from the eye.
    pub fn widen(&self, distance: f32) -> f32 {
        (distance / self.coarse).max(1.0)
    }

    /// The share of the blade height kept `distance` tiles from the eye.
    pub fn height_fade(&self, distance: f32) -> f32 {
        1.0 - smoothstep(self.full, self.far, distance)
    }
}

/// The "rendering detail" setting (owner, 2026-11-20): one level for the distances and budgets
/// of what is drawn near the camera, so that performance can be judged level by level. It sets
/// the grass's [`Reach`]; [`Detail::Medium`], the default, is the reach before the setting.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Detail {
    /// Shorter than before, for weak GPUs.
    Min,
    /// The reach before task 108.
    #[default]
    Medium,
    /// About 1.5 times as far, with coarser far blades.
    Max,
    /// About twice as far.
    Ultra,
}

impl Detail {
    /// Every level, lowest first.
    pub const ALL: [Detail; 4] = [Detail::Min, Detail::Medium, Detail::Max, Detail::Ultra];

    /// The level's name on the command line and in the settings.
    pub fn name(self) -> &'static str {
        match self {
            Detail::Min => "min",
            Detail::Medium => "medium",
            Detail::Max => "max",
            Detail::Ultra => "ultra",
        }
    }

    /// The level named `name` (any case).
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|level| level.name().eq_ignore_ascii_case(name.trim()))
    }

    /// The level's index in [`Self::ALL`].
    pub fn index(self) -> u8 {
        self as u8
    }

    /// The level of index `index` in [`Self::ALL`]; the default for any other.
    pub fn from_index(index: u8) -> Self {
        Self::ALL
            .get(usize::from(index))
            .copied()
            .unwrap_or_default()
    }

    /// How far the grass reaches at this level.
    pub fn reach(self) -> Reach {
        match self {
            Detail::Min => Reach {
                lod: 12.0,
                full: 20.0,
                far: 28.0,
                coarse: 28.0,
                budget: 15_000,
                far_budget: 0,
            },
            Detail::Medium => Reach {
                lod: 16.0,
                full: 32.0,
                far: 42.0,
                coarse: 42.0,
                budget: 30_000,
                far_budget: 0,
            },
            Detail::Max => Reach {
                lod: 16.0,
                full: 48.0,
                far: 64.0,
                coarse: 32.0,
                budget: 30_000,
                far_budget: 12_000,
            },
            Detail::Ultra => Reach {
                lod: 16.0,
                full: 64.0,
                far: 84.0,
                coarse: 32.0,
                budget: 30_000,
                far_budget: 20_000,
            },
        }
    }
}

/// Candidates per domain entry of each draw, largest first.
pub const SLICES: [u32; 4] = [64, 16, 4, 1];
/// Eye movement, in tiles, before the domains are rebuilt; also the margin the candidate counts
/// keep for it.
pub const REBUILD_STEP: f32 = 0.5;
/// A triangle expecting fewer blades than this gets none (it would cost a whole candidate).
const MIN_EXPECTED: f32 = 0.04;
/// Map shades are packed per channel as `shade / SHADE_RANGE · 255`.
pub const SHADE_RANGE: f32 = 2.0;

/// One source triangle of the grass program with the first candidate of its entry, laid out as
/// the vertex attributes of `grass.vs` (locations 0 to 8).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Domain {
    /// Corners, outward winding.
    pub a: [f32; 3],
    pub b: [f32; 3],
    pub c: [f32; 3],
    /// Seed low and high 16 bits, first candidate, grass material slot.
    pub seed: [f32; 4],
    /// Grass share of the ground blend at each corner, 0 to 1.
    pub share: [f32; 3],
    /// Map shade (fog of war, borders) at each corner, packed `r·65536 + g·256 + b`.
    pub shade: [f32; 3],
    /// Ground normals at the corners, as the land mesh carries them.
    pub na: [f32; 3],
    pub nb: [f32; 3],
    pub nc: [f32; 3],
}

/// Number of `f32` in a [`Domain`].
pub const DOMAIN_FLOATS: usize = std::mem::size_of::<Domain>() / 4;

/// Attribute layout of [`Domain`]: location, components and float offset.
pub const DOMAIN_ATTRIBUTES: [(u32, i32, usize); 9] = [
    (0, 3, 0),
    (1, 3, 3),
    (2, 3, 6),
    (3, 4, 9),
    (4, 3, 13),
    (5, 3, 16),
    (6, 3, 19),
    (7, 3, 22),
    (8, 3, 25),
];

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The domains of one frame, by draw, and the density they were counted with.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GrassField {
    /// Domain entries of each draw: entry `e` of draw `k` carries candidates
    /// `e.seed[2] .. e.seed[2] + SLICES[k]` of its triangle.
    pub bands: [Vec<Domain>; 4],
    /// Blades per square tile after the budget.
    pub density: f32,
}

impl GrassField {
    /// Candidates drawn: what the GPU's vertex stage works through.
    pub fn candidates(&self) -> usize {
        self.bands
            .iter()
            .zip(SLICES)
            .map(|(band, per)| band.len() * per as usize)
            .sum()
    }

    /// True when nothing is drawn.
    pub fn is_empty(&self) -> bool {
        self.bands.iter().all(Vec::is_empty)
    }

    /// Draws nothing.
    pub fn clear(&mut self) {
        for band in &mut self.bands {
            band.clear();
        }
    }
}

/// What the ground under the blades wears.
pub(crate) struct GroundLook<'a> {
    /// The planet.
    pub globe: &'a Globe,
    /// The land material slots and the one each tile wears.
    pub slots: &'a SurfaceSlots,
    /// Map shade per tile; empty while none is published.
    pub shades: &'a [[f32; 3]],
    /// Tiles that grow no blades: building footprints and stains.
    pub blocked: &'a BTreeSet<usize>,
    /// How far the blades reach.
    pub reach: Reach,
}

/// What the ground wears at one corner of the land mesh.
struct Corner {
    /// Share of the tiles around it that wear a grass slot.
    share: f32,
    /// One of those grass slots.
    slot: Option<u8>,
    /// Their mean map shade, packed.
    shade: f32,
}

/// One tile of the walk: where it is and its four displaced corners (00, 10, 01, 11).
#[derive(Clone, Copy)]
struct TileGround<'a> {
    globe: &'a Globe,
    tile: Tile,
    index: usize,
    corners: [[f32; 3]; 4],
}

/// A triangle on its way into the field.
struct Candidate {
    domain: Domain,
    expected: f32,
    /// Past [`Reach::coarse`]: counted against [`Reach::far_budget`].
    far: bool,
}

/// Builds [`GrassField`]s, keeping its scratch memory between frames.
#[derive(Default)]
pub(crate) struct GrassBuilder {
    stamp: Vec<u32>,
    generation: u32,
    queue: VecDeque<usize>,
    triangles: Vec<Candidate>,
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

fn length(v: [f32; 3]) -> f32 {
    dot(v, v).sqrt()
}

fn is_ground(kind: TileKind) -> bool {
    matches!(kind, TileKind::Land | TileKind::Feature(_))
}

/// Packs a map shade (each channel 0 to [`SHADE_RANGE`]) into one exact float.
pub fn pack_shade(shade: [f32; 3]) -> f32 {
    let byte = |c: f32| (c / SHADE_RANGE * 255.0).round().clamp(0.0, 255.0);
    byte(shade[0]) * 65_536.0 + byte(shade[1]) * 256.0 + byte(shade[2])
}

/// The seed of triangle `half` (0 or 1) of tile `tile`.
pub fn triangle_seed(tile: usize, half: usize) -> u32 {
    hash((tile * 2 + half) as u32 ^ 0x6772_6173)
}

/// `grass.vs`'s integer hash (after Meshcraft's `procedural.vs`).
pub fn hash(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^ (x >> 16)
}

/// `grass.vs`'s uniform random number in 0..1 from a key.
pub fn random(x: u32) -> f32 {
    (hash(x) & 0x00ff_ffff) as f32 / 16_777_216.0
}

impl GrassBuilder {
    /// Fills `out` with the grass triangles of `ground` within reach of `eye` that `cull` may
    /// see (the pyramid and the horizon), counted for the budget.
    pub(crate) fn build(
        &mut self,
        ground: &GroundLook,
        eye: [f32; 3],
        cull: &CullView,
        out: &mut GrassField,
    ) {
        out.clear();
        out.density = DENSITY;
        let globe = ground.globe;
        let surface = globe.surface();
        let tiles = surface.len();
        let grass: [bool; MAX_SLOTS] = std::array::from_fn(|i| {
            ground
                .slots
                .slots
                .get(i)
                .is_some_and(|m| m.program == Program::Grass)
        });
        if !grass.contains(&true) || ground.slots.tile_slot.len() != tiles {
            return;
        }
        let radius = globe.radius();
        let reach = ground.reach;
        if length(eye) - radius - crate::terrain::RELIEF > reach.far {
            return;
        }
        if self.stamp.len() != tiles {
            self.stamp = vec![0; tiles];
            self.generation = 0;
        }
        self.generation = self.generation.wrapping_add(1);
        if self.generation == 0 {
            self.stamp.fill(0);
            self.generation = 1;
        }
        let generation = self.generation;
        self.triangles.clear();
        self.queue.clear();
        let start = surface.index(globe.tile_of(eye));
        self.stamp[start] = generation;
        self.queue.push_back(start);
        let n = globe.n();
        let shaded = ground.shades.len() == tiles;
        // The grass share, a grass slot and the packed map shade at a corner of the land mesh.
        let corner = |face: u8, vx: u32, vy: u32| -> Corner {
            let (around, count) = if (1..n).contains(&vx) && (1..n).contains(&vy) {
                // Inside a face the four tiles around the corner are plain neighbours.
                let first = surface.index(Tile {
                    face,
                    x: vx - 1,
                    y: vy - 1,
                });
                let row = n as usize;
                ([first, first + 1, first + row, first + row + 1], 4)
            } else {
                globe.corner_tiles(face, vx, vy)
            };
            let mut with = 0;
            let mut slot = None;
            let mut sum = [0.0; 3];
            for &tile in &around[..count] {
                let s = ground.slots.tile_slot[tile];
                if s > 0 && grass[usize::from(s - 1)] {
                    with += 1;
                    slot.get_or_insert(s - 1);
                }
                if shaded {
                    for (s, v) in sum.iter_mut().zip(ground.shades[tile]) {
                        *s += v;
                    }
                }
            }
            let share = 1.0 / count.max(1) as f32;
            Corner {
                share: with as f32 * share,
                slot,
                shade: pack_shade(if shaded {
                    sum.map(|s| s * share)
                } else {
                    [1.0; 3]
                }),
            }
        };
        while let Some(index) = self.queue.pop_front() {
            let tile = surface.tile(index);
            // One corner decides the walk: the tile lies within a diagonal of it.
            let first = globe.vertex(tile.face, tile.x, tile.y);
            let offset = sub(first, eye);
            // Beyond reach, or behind the eye (the half-disk ahead holds every tile in view and
            // stays connected to the tile under the eye, which the view looks down on).
            if length(offset) - 1.5 > reach.far || dot(offset, cull.forward) < -2.0 {
                continue;
            }
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                if let Some(next) = surface.neighbor(tile, dx, dy) {
                    let next = surface.index(next);
                    if self.stamp[next] != generation {
                        self.stamp[next] = generation;
                        self.queue.push_back(next);
                    }
                }
            }
            if !is_ground(globe.kinds()[index]) || ground.blocked.contains(&index) {
                continue;
            }
            let corners = [(0, 0), (1, 0), (0, 1), (1, 1)]
                .map(|(dx, dy)| globe.vertex(tile.face, tile.x + dx, tile.y + dy));
            let centre = [0, 1, 2].map(|k| corners.iter().map(|c| c[k]).sum::<f32>() / 4.0);
            if length(sub(centre, eye)) - 0.75 > reach.far {
                continue;
            }
            let bounds = ChunkBounds {
                centre,
                radius: 0.75 + HEIGHT * 1.5,
                top: length(centre) + HEIGHT * 1.5,
            };
            if !bounds.may_be_seen(radius, cull) {
                continue;
            }
            let ground_tile = TileGround {
                globe,
                tile,
                index,
                corners,
            };
            self.add_tile(&ground_tile, eye, &reach, &corner);
        }
        self.allocate(&reach, out);
    }

    /// Adds the two triangles of a tile (as the land mesh splits it) where they wear grass.
    fn add_tile(
        &mut self,
        ground: &TileGround,
        eye: [f32; 3],
        reach: &Reach,
        corner: &dyn Fn(u8, u32, u32) -> Corner,
    ) {
        let TileGround {
            globe,
            tile,
            index,
            corners,
        } = *ground;
        let at = [(0, 0), (1, 0), (0, 1), (1, 1)].map(|(dx, dy)| (tile.x + dx, tile.y + dy));
        let looks = at.map(|(vx, vy)| corner(tile.face, vx, vy));
        let Some(slot) = looks.iter().find_map(|c| c.slot) else {
            return;
        };
        let normals = at.map(|(vx, vy)| globe.vertex_normal(tile.face, vx, vy));
        // Corner order 00, 10, 01, 11; the land mesh's triangles are (00, 01, 10), (10, 01, 11).
        for (half, [i, j, k]) in [[0, 2, 1], [1, 2, 3]].into_iter().enumerate() {
            let [a, b, c] = [corners[i], corners[j], corners[k]];
            let area = length(cross(sub(b, a), sub(c, a))) * 0.5;
            let centroid = [0, 1, 2].map(|n| (a[n] + b[n] + c[n]) / 3.0);
            let distance = (length(sub(centroid, eye)) - REBUILD_STEP).max(0.0);
            let corner_shares = [looks[i].share, looks[j].share, looks[k].share];
            let most = corner_shares.iter().copied().fold(0.0, f32::max);
            if most <= 0.0 {
                continue;
            }
            // As `grass.vs` counts: the shader then keeps a blade by its root's share of `most`.
            let expected = area * DENSITY * reach.lod(distance) * most;
            let seed = triangle_seed(index, half);
            let domain = Domain {
                a,
                b,
                c,
                seed: [
                    (seed & 0xffff) as f32,
                    (seed >> 16) as f32,
                    0.0,
                    f32::from(slot),
                ],
                share: corner_shares,
                shade: [looks[i].shade, looks[j].shade, looks[k].shade],
                na: normals[i],
                nb: normals[j],
                nc: normals[k],
            };
            self.triangles.push(Candidate {
                domain,
                expected,
                far: distance > reach.coarse,
            });
        }
    }

    /// Splits each triangle's candidates over the draws, within the budgets: the near
    /// triangles within the reach's budget, the far ones within its far budget and never denser
    /// than the near ones.
    fn allocate(&mut self, reach: &Reach, out: &mut GrassField) {
        let count = |far: bool, scale: f32| -> usize {
            self.triangles
                .iter()
                .filter(|t| t.far == far)
                .map(|t| t.expected * scale)
                .filter(|e| *e >= MIN_EXPECTED)
                .map(|e| e.ceil() as usize)
                .sum()
        };
        let fit = |far: bool, budget: usize, mut scale: f32| {
            for _ in 0..12 {
                let needed = count(far, scale);
                if needed <= budget {
                    break;
                }
                scale *= budget as f32 / needed as f32 * 0.97;
            }
            scale
        };
        let near_scale = fit(false, reach.budget, 1.0);
        let far_scale = fit(true, reach.far_budget, near_scale);
        out.density = DENSITY * near_scale;
        let mut totals = [0, 0];
        // The near ones stop at the first that would overrun their budget, as before task 108.
        let mut near_full = false;
        for triangle in &self.triangles {
            let (scale, budget, total) = if triangle.far {
                (far_scale, reach.far_budget, &mut totals[1])
            } else {
                (near_scale, reach.budget, &mut totals[0])
            };
            let expected = triangle.expected * scale;
            if expected < MIN_EXPECTED || (near_full && !triangle.far) {
                continue;
            }
            let n = expected.ceil() as u32;
            if *total + n as usize > budget {
                near_full |= !triangle.far;
                continue;
            }
            *total += n as usize;
            let mut domain = triangle.domain;
            if triangle.far {
                // The field's density is the near one; a far triangle's blades are thinned by
                // its grass share, which scales the count alone (`grass.vs` keeps a blade by
                // its root's share of the largest).
                let thin = far_scale / near_scale;
                domain.share = domain.share.map(|s| s * thin);
            }
            let mut first = 0;
            for (band, per) in out.bands.iter_mut().zip(SLICES) {
                while n - first >= per {
                    let mut entry = domain;
                    entry.seed[2] = first as f32;
                    band.push(entry);
                    first += per;
                }
            }
        }
    }
}

/// Cosine of the turn of the view (about 2°) after which the domains are rebuilt.
pub const TURN_COSINE: f32 = 0.9994;

/// The blades from frame to frame: what the ground wears, the tiles that grow none, the current
/// field and when it is rebuilt. The GPU side uploads the field whenever [`Self::prepare`] says
/// it changed.
pub(crate) struct GrassState {
    builder: GrassBuilder,
    field: GrassField,
    /// Eye and forward vector the field was built for.
    built: Option<([f32; 3], [f32; 3])>,
    /// Something besides the eye changed: shades, slots, buildings or stains.
    dirty: bool,
    slots: SurfaceSlots,
    buildings: Vec<(u8, i32, i32, u8, u8)>,
    stains: Vec<(u32, u8, i32, i32, i32)>,
    blocked_buildings: BTreeSet<usize>,
    blocked_stains: BTreeSet<usize>,
    blocked: BTreeSet<usize>,
    reach: Reach,
}

impl Default for GrassState {
    fn default() -> Self {
        Self {
            builder: GrassBuilder::default(),
            field: GrassField::default(),
            built: None,
            dirty: true,
            slots: SurfaceSlots::default(),
            buildings: Vec::new(),
            stains: Vec::new(),
            blocked_buildings: BTreeSet::new(),
            blocked_stains: BTreeSet::new(),
            blocked: BTreeSet::new(),
            reach: Detail::default().reach(),
        }
    }
}

impl GrassState {
    /// The field to draw; empty when nothing is.
    pub(crate) fn field(&self) -> &GrassField {
        &self.field
    }

    /// Takes the land slots of a new planet or of newly concealed terrain; true when they
    /// changed.
    pub(crate) fn set_slots(&mut self, slots: &SurfaceSlots) -> bool {
        if self.slots == *slots {
            return false;
        }
        self.slots = slots.clone();
        self.dirty = true;
        true
    }

    /// Takes the reach of a rendering detail level; true when it changed (the next frame
    /// rebuilds with it).
    pub(crate) fn set_reach(&mut self, reach: Reach) -> bool {
        if self.reach == reach {
            return false;
        }
        self.reach = reach;
        self.dirty = true;
        true
    }

    /// The map shades changed: the next frame rebuilds with them.
    pub(crate) fn shades_changed(&mut self) {
        self.dirty = true;
    }

    /// Forgets the planet: no blades until the next one.
    pub(crate) fn clear(&mut self) {
        *self = Self {
            builder: std::mem::take(&mut self.builder),
            reach: self.reach,
            ..Self::default()
        };
    }

    /// Takes the buildings among `views`; their footprints grow no blades.
    pub(crate) fn set_buildings(&mut self, globe: &Globe, views: &[crate::entities::EntityView]) {
        let key: Vec<(u8, i32, i32, u8, u8)> = views
            .iter()
            .filter(|v| v.building && v.altitude <= 0.0)
            .map(|v| {
                (
                    v.face,
                    (v.x * 1000.0).round() as i32,
                    (v.y * 1000.0).round() as i32,
                    v.footprint.0,
                    v.footprint.1,
                )
            })
            .collect();
        if key != self.buildings {
            self.buildings = key;
            self.blocked_buildings.clear();
            block_buildings(globe, views, &mut self.blocked_buildings);
            self.merge_blocked();
        }
    }

    /// Takes the stains on the ground; they grow no blades.
    pub(crate) fn set_stains(&mut self, globe: &Globe, stains: &[HudStain]) {
        let key: Vec<(u32, u8, i32, i32, i32)> = stains
            .iter()
            .map(|s| (s.id, s.face, s.x, s.y, s.radius))
            .collect();
        if key != self.stains {
            self.stains = key;
            self.blocked_stains.clear();
            block_stains(globe, stains, &mut self.blocked_stains);
            self.merge_blocked();
        }
    }

    fn merge_blocked(&mut self) {
        self.blocked = self
            .blocked_buildings
            .union(&self.blocked_stains)
            .copied()
            .collect();
        self.dirty = true;
    }

    /// Rebuilds the field when the eye moved or turned, or the ground changed; with `shown`
    /// false (the setting off) the field is empty. True when the field changed.
    pub(crate) fn prepare(
        &mut self,
        globe: &Globe,
        shades: &[[f32; 3]],
        cull: &CullView,
        shown: bool,
    ) -> bool {
        if !shown {
            self.built = None;
            let had = !self.field.is_empty();
            self.field.clear();
            return had;
        }
        let stale = self.dirty
            || self.built.is_none_or(|(eye, forward)| {
                let moved = length(sub(eye, cull.eye));
                moved >= REBUILD_STEP || dot(forward, cull.forward) < TURN_COSINE
            });
        if !stale {
            return false;
        }
        self.dirty = false;
        self.built = Some((cull.eye, cull.forward));
        let ground = GroundLook {
            globe,
            slots: &self.slots,
            shades,
            blocked: &self.blocked,
            reach: self.reach,
        };
        self.builder.build(&ground, cull.eye, cull, &mut self.field);
        true
    }
}

/// Where `grass.vs` roots candidate `candidate` of `domain` for an eye at `eye`, the field
/// density `density` and the reach `reach`, or `None` when it draws no blade for it (a reference
/// for tests).
pub fn blade_root(
    domain: &Domain,
    candidate: u32,
    eye: [f32; 3],
    density: f32,
    reach: &Reach,
) -> Option<[f32; 3]> {
    let seed = domain.seed[0] as u32 | ((domain.seed[1] as u32) << 16);
    let key = hash(seed) ^ hash(candidate) ^ SEED;
    let (a, b, c) = (domain.a, domain.b, domain.c);
    let area = length(cross(sub(b, a), sub(c, a))) * 0.5;
    let centroid = [0, 1, 2].map(|n| (a[n] + b[n] + c[n]) / 3.0);
    let most = domain.share.iter().copied().fold(0.0, f32::max);
    let count = area * density * reach.lod(length(sub(centroid, eye))) * most;
    if candidate as f32 + random(key.wrapping_add(3)) >= count {
        return None;
    }
    let r = random(key.wrapping_add(1)).sqrt();
    let s = random(key.wrapping_add(2));
    let weights = [1.0 - r, r * (1.0 - s), r * s];
    let root = [0, 1, 2].map(|n| a[n] * weights[0] + b[n] * weights[1] + c[n] * weights[2]);
    let share = dot(weights, domain.share);
    if random(key.wrapping_add(7)) * most >= share {
        return None;
    }
    if reach.height_fade(length(sub(root, eye))) <= 0.0 {
        return None;
    }
    Some(root)
}

/// Adds the tiles under the footprints of the buildings among `views` to `out`.
pub(crate) fn block_buildings(
    globe: &Globe,
    views: &[crate::entities::EntityView],
    out: &mut BTreeSet<usize>,
) {
    let surface = globe.surface();
    for view in views.iter().filter(|v| v.building && v.altitude <= 0.0) {
        let centre = SurfacePos {
            face: view.face,
            x: (view.x * 1000.0).round() as i64,
            y: (view.y * 1000.0).round() as i64,
        };
        let (w, h) = (view.footprint.0.max(1), view.footprint.1.max(1));
        for j in 0..h {
            for i in 0..w {
                let dx = f32::from(i) + 0.5 - f32::from(w) / 2.0;
                let dy = f32::from(j) + 0.5 - f32::from(h) / 2.0;
                let at = globe.step_on(
                    centre,
                    (dx * 1000.0).round() as i64,
                    (dy * 1000.0).round() as i64,
                );
                out.insert(surface.index(at.tile()));
            }
        }
    }
}

/// Adds the tiles under `stains` to `out`: those whose nearest point lies within a stain.
pub(crate) fn block_stains(globe: &Globe, stains: &[HudStain], out: &mut BTreeSet<usize>) {
    let surface = globe.surface();
    for stain in stains {
        let radius = (stain.radius as f32 / 1000.0).max(0.1);
        let (cx, cy) = (stain.x as f32 / 1000.0, stain.y as f32 / 1000.0);
        let centre = SurfacePos {
            face: stain.face,
            x: i64::from(stain.x),
            y: i64::from(stain.y),
        };
        let steps = radius.ceil() as i32 + 1;
        let (ox, oy) = (cx.floor() + 0.5 - cx, cy.floor() + 0.5 - cy);
        for j in -steps..=steps {
            for i in -steps..=steps {
                let (dx, dy) = (ox + i as f32, oy + j as f32);
                let (nx, ny) = ((dx.abs() - 0.5).max(0.0), (dy.abs() - 0.5).max(0.0));
                if nx * nx + ny * ny > radius * radius {
                    continue;
                }
                let at = globe.step_on(
                    centre,
                    (dx * 1000.0).round() as i64,
                    (dy * 1000.0).round() as i64,
                );
                out.insert(surface.index(at.tile()));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::globe::PlanetScene;
    use crate::looks::MaterialLook;
    use crate::orbit::OrbitCamera;
    use std::collections::BTreeMap;

    const N: u32 = 96;

    /// A planet of grass with a sea in the west of face 0 and a sandy strip beside it.
    fn planet() -> (Globe, SurfaceSlots) {
        let tiles = 6 * (N * N) as usize;
        let kinds: Vec<TileKind> = (0..tiles)
            .map(|i| {
                let (face, x) = (i / (N * N) as usize, i % N as usize);
                if face == 0 && x < 40 {
                    TileKind::ShallowSea
                } else if face == 0 && x < 44 {
                    TileKind::Feature(1)
                } else {
                    TileKind::Land
                }
            })
            .collect();
        let heights: Vec<u8> = (0..tiles).map(|i| 130 + (i * 7 % 13) as u8).collect();
        let globe = Globe::new(&PlanetScene {
            sea_level: None,
            n: N,
            kinds: kinds.clone(),
            heights,
            feature_colors: BTreeMap::new(),
            starts: Vec::new(),
            nodes: Vec::new(),
            sun: Default::default(),
        });
        let look = |program| MaterialLook {
            program,
            colour: [0.3, 0.5, 0.2],
            colour_alt: [0.4, 0.6, 0.2],
            scale: 1.0,
        };
        let slots = SurfaceSlots {
            slots: vec![look(Program::Grass), look(Program::Sand)],
            tile_slot: kinds
                .iter()
                .map(|k| match k {
                    TileKind::Land => 1,
                    TileKind::Feature(_) => 2,
                    _ => 0,
                })
                .collect(),
        };
        (globe, slots)
    }

    /// The reach the tests build with: the farthest-but-one level, coarse far blades included.
    fn reach() -> Reach {
        Detail::Max.reach()
    }

    /// A camera over face 0 (the sea at its west), `distance` tiles from its target.
    fn camera(globe: &Globe, distance: f32, x: f32, heading: f32) -> (OrbitCamera, CullView) {
        let surface = globe.surface();
        let tile = rom_geo::Tile {
            face: 0,
            x: x as u32,
            y: N / 2,
        };
        let point = globe.tile_point(tile);
        let mut orbit = OrbitCamera::new(globe.radius(), 3.0 * globe.radius());
        orbit.dir = crate::terrain::normalize(point);
        orbit.distance = distance;
        orbit.heading = heading;
        assert_eq!(
            surface.index(globe.tile_of(orbit.target())),
            surface.index(tile)
        );
        let cull = CullView::of_orbit(&orbit, 16.0 / 9.0, 45.0, 1.5);
        (orbit, cull)
    }

    fn build(
        globe: &Globe,
        slots: &SurfaceSlots,
        blocked: &BTreeSet<usize>,
        cull: &CullView,
    ) -> GrassField {
        let ground = GroundLook {
            globe,
            slots,
            shades: &[],
            blocked,
            reach: reach(),
        };
        let mut field = GrassField::default();
        GrassBuilder::default().build(&ground, cull.eye, cull, &mut field);
        field
    }

    fn roots(field: &GrassField, eye: [f32; 3]) -> Vec<[f32; 3]> {
        let mut out = Vec::new();
        for (band, per) in field.bands.iter().zip(SLICES) {
            for entry in band {
                for k in 0..per {
                    let candidate = entry.seed[2] as u32 + k;
                    if let Some(root) = blade_root(entry, candidate, eye, field.density, &reach()) {
                        out.push(root);
                    }
                }
            }
        }
        out
    }

    #[test]
    fn placement_is_the_same_for_the_same_seed_and_frame() {
        let (globe, slots) = planet();
        let (_, cull) = camera(&globe, 14.0, 60.0, 0.3);
        let none = BTreeSet::new();
        let first = build(&globe, &slots, &none, &cull);
        let second = build(&globe, &slots, &none, &cull);
        assert!(!first.is_empty());
        assert_eq!(first, second);
        let blades = roots(&first, cull.eye);
        assert!(blades.len() > 1000, "{}", blades.len());
        assert_eq!(blades, roots(&second, cull.eye));
        // Each candidate has its own place: no two blades share a root.
        let mut keys: Vec<[u32; 3]> = blades.iter().map(|r| r.map(f32::to_bits)).collect();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), blades.len());
        // A moved eye keeps the blades that stay: candidate numbers and seeds do not move.
        let (_, near) = camera(&globe, 13.5, 60.0, 0.3);
        let moved = roots(&build(&globe, &slots, &none, &near), near.eye);
        let kept = blades.iter().filter(|r| moved.contains(r)).count();
        assert!(kept * 10 > blades.len() * 8, "{kept} of {}", blades.len());
    }

    #[test]
    fn nothing_past_the_far_distance_or_without_grass() {
        let (globe, slots) = planet();
        for (distance, x) in [
            (9.0, 46.0),
            (20.0, 48.0),
            (reach().full, 50.0),
            (60.0, 50.0),
            (100.0, 50.0),
        ] {
            let (_, cull) = camera(&globe, distance, x, 4.0);
            let field = build(&globe, &slots, &BTreeSet::new(), &cull);
            for band in &field.bands {
                for entry in band {
                    assert!(entry.share.iter().any(|s| *s > 0.0));
                    let near = [entry.a, entry.b, entry.c]
                        .iter()
                        .map(|p| length(sub(*p, cull.eye)))
                        .fold(f32::MAX, f32::min);
                    assert!(near < reach().far + 1.0, "{near}");
                }
            }
            let surface = globe.surface();
            for root in roots(&field, cull.eye) {
                assert!(length(sub(root, cull.eye)) < reach().far);
                // On grass or the sand beside it, never on the sea; on the near side.
                let tile = surface.index(globe.tile_of(root));
                assert!(slots.tile_slot[tile] > 0, "blade on water");
                // Within the horizon of the eye over the sea-level sphere, hills allowed for.
                let r = globe.radius();
                let horizon =
                    (r / length(cull.eye)).acos() + (r / (r + crate::terrain::RELIEF)).acos();
                let apart = (dot(root, cull.eye) / (length(root) * length(cull.eye))).acos();
                assert!(apart < horizon, "blade beyond the horizon");
            }
            if distance > reach().far {
                assert!(field.is_empty());
            }
        }
        // Blades thin out across the sand strip (x 40 to 43) and stop at the sea.
        let (_, cull) = camera(&globe, 12.0, 46.0, 4.71);
        let field = build(&globe, &slots, &BTreeSet::new(), &cull);
        let surface = globe.surface();
        let column = |root: [f32; 3]| surface.tile(surface.index(globe.tile_of(root))).x;
        let blades = roots(&field, cull.eye);
        let on = |x: u32| blades.iter().filter(|r| column(**r) == x).count();
        // The sand tile beside the grass has half a share on its grass edge, the next none.
        assert!(0 < on(43) && on(43) < on(46), "{} {}", on(43), on(46));
        assert_eq!(on(41) + on(42), 0);
        assert_eq!(blades.iter().filter(|r| column(**r) < 40).count(), 0);
    }

    #[test]
    fn buildings_and_stains_grow_no_blades() {
        let (globe, slots) = planet();
        let (_, cull) = camera(&globe, 12.0, 60.0, 0.0);
        let mut blocked = BTreeSet::new();
        let farm = crate::entities::EntityView {
            id: 1,
            kind: 1,
            owner: 1,
            face: 0,
            x: 60.5,
            y: N as f32 / 2.0 + 0.5,
            altitude: 0.0,
            shot: None,
            spacecraft: false,
            facing_deg: 0.0,
            hp_frac: 1.0,
            building: true,
            footprint: (3, 3),
            radius: 1.5,
            carrying: false,
            kept: 0,
            capacity: 0,
            carry_node: false,
            under_construction: None,
            model: None,
            activity: Default::default(),
        };
        block_buildings(&globe, &[farm], &mut blocked);
        assert_eq!(blocked.len(), 9);
        let stain = HudStain {
            id: 2,
            face: 0,
            x: 66_500,
            y: (N as i32 / 2) * 1000 + 500,
            radius: 1_000,
            tick: 0,
            building: false,
            owner: 1,
            model: None,
        };
        block_stains(&globe, &[stain], &mut blocked);
        assert!(blocked.len() >= 9 + 5);
        let field = build(&globe, &slots, &blocked, &cull);
        let surface = globe.surface();
        let blades = roots(&field, cull.eye);
        assert!(!blades.is_empty());
        for root in blades {
            assert!(!blocked.contains(&surface.index(globe.tile_of(root))));
        }
    }

    #[test]
    fn the_work_stays_within_the_budget() {
        let (globe, slots) = planet();
        // The closest and the farthest views that still draw blades, every way round.
        for distance in [8.0, 14.0, 24.0, reach().full, reach().far] {
            for heading in [0.0, 1.6, 3.1, 4.7] {
                let (_, cull) = camera(&globe, distance, 64.0, heading);
                let field = build(&globe, &slots, &BTreeSet::new(), &cull);
                assert!(
                    field.candidates() <= reach().budget + reach().far_budget,
                    "{}",
                    field.candidates()
                );
                assert!(field.density <= DENSITY && field.density > 0.0);
            }
        }
    }

    #[test]
    fn far_blades_are_fewer_and_wider() {
        let r = reach();
        let falloff = |d: f32| 1.0 / (1.0 + (d / r.lod).powi(2));
        assert_eq!(r.widen(r.coarse), 1.0);
        assert_eq!(r.lod(r.coarse), falloff(r.coarse));
        let d = r.coarse * 1.25;
        assert!(d < r.full);
        assert!((r.widen(d) - 1.25).abs() < 1e-6);
        assert!((r.lod(d) - falloff(d) / 1.5625).abs() < 1e-6);
    }

    #[test]
    fn the_detail_levels_set_the_reach() {
        // Medium is the reach before the setting existed: no coarse blades, no far budget.
        let medium = Detail::default().reach();
        assert_eq!(Detail::default(), Detail::Medium);
        assert_eq!(
            (medium.lod, medium.full, medium.far, medium.budget),
            (16.0, 32.0, 42.0, 30_000)
        );
        for d in [0.0, 10.0, 30.0, 41.0] {
            assert_eq!(medium.widen(d), 1.0);
        }
        // Max and Ultra reach about 1.5 and 2 times as far; Min less far than Medium.
        let ratio = |level: Detail| level.reach().far / medium.far;
        assert!(ratio(Detail::Min) < 1.0);
        assert!((1.4..=1.6).contains(&ratio(Detail::Max)));
        assert!((1.9..=2.1).contains(&ratio(Detail::Ultra)));
        for level in Detail::ALL {
            assert_eq!(Detail::parse(level.name()), Some(level));
            assert_eq!(Detail::from_index(level.index()), level);
            let r = level.reach();
            assert!(r.lod < r.full && r.full < r.far && r.coarse <= r.far);
        }
        assert_eq!(Detail::parse(" ULTRA "), Some(Detail::Ultra));
        assert_eq!(Detail::parse("high"), None);
    }

    #[test]
    fn density_and_height_fade_out_smoothly() {
        for level in Detail::ALL {
            let r = level.reach();
            assert_eq!(r.height_fade(r.full), 1.0);
            assert_eq!(r.height_fade(r.far), 0.0);
            assert_eq!(r.lod(r.far), 0.0);
            assert!((r.lod(r.lod) - 0.5).abs() < 1e-6);
            let mut last = r.lod(0.0);
            for step in 1..=200 {
                let d = step as f32 * r.far / 200.0;
                let now = r.lod(d);
                assert!(
                    now <= last && last - now < 0.02,
                    "{level:?} {d}: {last} -> {now}"
                );
                last = now;
            }
        }
    }

    #[test]
    fn the_setting_off_draws_nothing() {
        let (globe, slots) = planet();
        let (_, cull) = camera(&globe, 14.0, 60.0, 0.3);
        let mut state = GrassState::default();
        assert!(state.set_slots(&slots) && !state.set_slots(&slots));
        assert!(state.prepare(&globe, &[], &cull, true));
        assert!(!state.field().is_empty());
        // The same view keeps the field; a small step does too.
        assert!(!state.prepare(&globe, &[], &cull, true));
        let (_, near) = camera(&globe, 13.9, 60.0, 0.3);
        assert!(!state.prepare(&globe, &[], &near, true));
        // Off: nothing to draw, whatever the view.
        assert!(state.prepare(&globe, &[], &cull, false));
        assert!(state.field().is_empty() && state.field().candidates() == 0);
        assert!(!state.prepare(&globe, &[], &near, false));
        assert!(state.field().is_empty());
        // On again: the blades come back as they were.
        let before = {
            let mut fresh = GrassState::default();
            fresh.set_slots(&slots);
            fresh.prepare(&globe, &[], &cull, true);
            fresh.field().clone()
        };
        assert!(state.prepare(&globe, &[], &cull, true));
        assert_eq!(*state.field(), before);
        // A new planet starts empty.
        state.clear();
        assert!(state.field().is_empty());
    }

    #[test]
    fn domains_match_the_shader_attributes() {
        use std::mem::offset_of;
        let floats = |bytes: usize| bytes / 4;
        let fields = [
            offset_of!(Domain, a),
            offset_of!(Domain, b),
            offset_of!(Domain, c),
            offset_of!(Domain, seed),
            offset_of!(Domain, share),
            offset_of!(Domain, shade),
            offset_of!(Domain, na),
            offset_of!(Domain, nb),
            offset_of!(Domain, nc),
        ];
        for ((location, size, offset), field) in DOMAIN_ATTRIBUTES.into_iter().zip(fields) {
            assert_eq!(offset, floats(field), "location {location}");
            assert!((1..=4).contains(&size));
        }
        assert_eq!(DOMAIN_FLOATS, 28);
        let source = include_str!("../../../assets/shaders/grass.vs");
        for (location, size, _) in DOMAIN_ATTRIBUTES {
            let kind = if size == 4 { "vec4" } else { "vec3" };
            assert!(source.contains(&format!("layout(location={location}) in {kind} ")));
        }
    }

    #[test]
    fn shades_pack_exactly() {
        let packed = pack_shade([1.0, 0.5, 2.0]);
        assert_eq!(packed, 128.0 * 65_536.0 + 64.0 * 256.0 + 255.0);
        assert_eq!(packed as u32 as f32, packed);
    }
}
