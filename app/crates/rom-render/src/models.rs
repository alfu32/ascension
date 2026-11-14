//! Composed models: buildings and units built from named parts (`mesh_part` rows), baked once
//! before play into a near and a far mesh. A part is one shape of the kit in [`crate::kit`],
//! placed in the model's frame (x east, y up, z south; tiles; origin on the ground at the
//! footprint centre; the front faces south). Shapes and knobs are renderer knowledge; the rows
//! name them. Nothing here touches raylib, and the same parts always bake the same vertices.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::kit::{self, Painter, Place, Surface};
use crate::looks::MaterialLook;

/// Parts whose far mesh spans less than this (tiles) are left out of the far version.
pub const FAR_MIN_EXTENT: f32 = 0.3;
/// Deepest nesting of includes followed when baking.
const MAX_INCLUDE_DEPTH: usize = 8;
/// Vertex alpha bit of faces that wear the owner's colour; the low bits are the program id.
pub const TEAM_BIT: u8 = 0x80;
/// Height above the ground (tiles) over which the ground contact shade fades out.
const GROUND_SHADE_HEIGHT: f32 = 0.45;
/// Shade at the foot of a part standing on the ground.
const GROUND_SHADE: f32 = 0.62;
/// Height above a part's base (tiles) over which the shade where it meets another part fades.
const MEET_SHADE_HEIGHT: f32 = 0.25;
/// Shade at the base of a part standing on another part.
const MEET_SHADE: f32 = 0.8;

/// A shape of the part kit, named by `mesh_part.shape`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Shape {
    /// A block (optionally tapered, gabled, with door and windows).
    Box,
    /// A stepped base.
    Plinth,
    /// Two roof slopes meeting at a ridge along x.
    GableRoof,
    /// Four roof slopes.
    HipRoof,
    /// A flat roof slab with an optional parapet.
    FlatRoof,
    /// A half dome.
    Dome,
    /// An upright cylinder (optionally tapered).
    Cylinder,
    /// An upright cone.
    Cone,
    /// A row of columns along x.
    ColumnRow,
    /// A wall along x, or four walls round a yard, with door, windows and a gate gap.
    Wall,
    /// Merlons along the top of a wall or round a yard.
    Crenellation,
    /// Posts and rails.
    Fence,
    /// Pointed stakes.
    Palisade,
    /// A pole with a hanging cloth.
    Banner,
    /// A sloping cloth sheet on posts.
    Awning,
    /// A wall pierced by a round-headed opening: a gateway or one bay of an arcade.
    Arch,
    /// An open boat's hull along x, pointed at both ends.
    Hull,
    /// A standing person about one tile tall, with sockets for what it carries.
    Figure,
    /// Another composed mesh placed as a sub-assembly.
    Include,
}

/// Knobs every part understands: position, turn, lean (`roll` tips the top towards +x, then
/// `pitch` towards +z, degrees, about the part's base centre; on a roof `pitch` is its slope),
/// repeat (`repeat` copies, each moved by `dx`, `dz`), and `far` (0: left out of the far version).
pub const COMMON_KNOBS: &[&str] = &[
    "x", "y", "z", "yaw", "pitch", "roll", "repeat", "dx", "dz", "far",
];
/// Colour knobs every part understands.
pub const COMMON_COLOURS: &[&str] = &["tint"];

impl Shape {
    /// Every shape, in a fixed order.
    pub const ALL: [Shape; 19] = [
        Shape::Box,
        Shape::Plinth,
        Shape::GableRoof,
        Shape::HipRoof,
        Shape::FlatRoof,
        Shape::Dome,
        Shape::Cylinder,
        Shape::Cone,
        Shape::ColumnRow,
        Shape::Wall,
        Shape::Crenellation,
        Shape::Fence,
        Shape::Palisade,
        Shape::Banner,
        Shape::Awning,
        Shape::Arch,
        Shape::Hull,
        Shape::Figure,
        Shape::Include,
    ];

    /// The shape a `mesh_part.shape` value names, if the renderer has it.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.name() == name)
    }

    /// The name rows use for this shape.
    pub fn name(self) -> &'static str {
        match self {
            Shape::Box => "box",
            Shape::Plinth => "plinth",
            Shape::GableRoof => "gable_roof",
            Shape::HipRoof => "hip_roof",
            Shape::FlatRoof => "flat_roof",
            Shape::Dome => "dome",
            Shape::Cylinder => "cylinder",
            Shape::Cone => "cone",
            Shape::ColumnRow => "column_row",
            Shape::Wall => "wall",
            Shape::Crenellation => "crenellation",
            Shape::Fence => "fence",
            Shape::Palisade => "palisade",
            Shape::Banner => "banner",
            Shape::Awning => "awning",
            Shape::Arch => "arch",
            Shape::Hull => "hull",
            Shape::Figure => "figure",
            Shape::Include => "include",
        }
    }

    /// The numeric knobs of this shape besides [`COMMON_KNOBS`].
    pub fn knobs(self) -> &'static [&'static str] {
        match self {
            Shape::Box => &[
                "w",
                "d",
                "h",
                "taper",
                "gable",
                "door",
                "door_h",
                "door_x",
                "windows",
                "window_w",
                "window_h",
                "window_y",
                "window_rows",
                "row_gap",
                "side_windows",
            ],
            Shape::Plinth => &["w", "d", "h", "steps", "inset"],
            Shape::GableRoof => &["w", "d", "pitch", "overhang", "t"],
            Shape::HipRoof => &["w", "d", "pitch", "overhang", "t"],
            Shape::FlatRoof => &["w", "d", "t", "overhang", "parapet", "parapet_t"],
            Shape::Dome => &["w", "h", "segments"],
            Shape::Cylinder => &["w", "h", "segments", "taper", "bottom"],
            Shape::Cone => &["w", "h", "segments"],
            Shape::ColumnRow => &["count", "radius", "spacing", "h", "segments", "capital"],
            Shape::Wall => &[
                "w",
                "d",
                "h",
                "t",
                "ring",
                "gap",
                "door",
                "door_h",
                "door_x",
                "windows",
                "window_w",
                "window_h",
                "window_y",
                "window_rows",
                "row_gap",
                "side_windows",
            ],
            Shape::Crenellation => &["w", "d", "h", "t", "merlon", "ring", "gap"],
            Shape::Fence => &["w", "d", "h", "ring", "gap", "spacing", "rails"],
            Shape::Palisade => &["w", "d", "h", "t", "ring", "gap", "spacing"],
            Shape::Banner => &["h", "fw", "fh", "pole", "notch"],
            Shape::Awning => &["w", "d", "h", "rise", "posts"],
            Shape::Arch => &["w", "h", "t", "span", "spring", "segments"],
            Shape::Hull => &["w", "d", "h", "sheer", "segments"],
            Shape::Figure => &["h", "build"],
            Shape::Include => &["scale"],
        }
    }

    /// The colour knobs of this shape besides [`COMMON_COLOURS`].
    pub fn colour_knobs(self) -> &'static [&'static str] {
        match self {
            Shape::Box | Shape::Wall => &["opening", "door_colour"],
            Shape::Banner => &["pole_colour"],
            Shape::Awning => &["post_colour"],
            Shape::Figure => &["skin", "hair", "legs"],
            _ => &[],
        }
    }

    /// Text knobs (resolved by the caller): only an include's `mesh`.
    pub fn text_knobs(self) -> &'static [&'static str] {
        match self {
            Shape::Include => &["mesh"],
            _ => &[],
        }
    }

    /// Named points of this shape other parts can attach to.
    pub fn sockets(self) -> &'static [&'static str] {
        match self {
            Shape::Figure => &["head", "right_hand", "left_hand", "back"],
            _ => &[],
        }
    }
}

/// One part of a model as the renderer bakes it: its shape, material and knobs.
#[derive(Clone, Debug, PartialEq)]
pub struct PartSpec {
    /// Reference name, unique in its model.
    pub name: String,
    /// The shape.
    pub shape: Shape,
    /// The material its main surface wears.
    pub material: MaterialLook,
    /// Whether the main surface wears the owner's colour.
    pub team: bool,
    /// The part or socket it moves with (`""`: the model; `part.socket` or a socket name).
    pub attach: String,
    /// Numeric knobs.
    pub knobs: BTreeMap<String, f32>,
    /// Colour knobs, RGB 0..1.
    pub colours: BTreeMap<String, [f32; 3]>,
    /// The sub-assembly of an include.
    pub include: Option<Arc<ModelSpec>>,
}

impl PartSpec {
    /// A part of `shape` named `name` wearing `material`, with no knobs.
    pub fn new(name: &str, shape: Shape, material: MaterialLook) -> Self {
        Self {
            name: name.into(),
            shape,
            material,
            team: false,
            attach: String::new(),
            knobs: BTreeMap::new(),
            colours: BTreeMap::new(),
            include: None,
        }
    }

    /// This part with knob `key` set to `value`.
    pub fn knob(mut self, key: &str, value: f32) -> Self {
        self.knobs.insert(key.into(), value);
        self
    }
}

/// The parts of one model, in row order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ModelSpec {
    /// The parts.
    pub parts: Vec<PartSpec>,
}

/// A model the renderer bakes: its review id, parts and size factor.
#[derive(Clone, Debug, PartialEq)]
pub struct ModelLook {
    /// The model id, `<entity key>@<age>`.
    pub id: String,
    /// The parts.
    pub spec: Arc<ModelSpec>,
    /// Uniform size factor (`entity_visual.scale_pct` / 100).
    pub scale: f32,
}

/// Baked vertex data of one model version, in the model's frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ModelMesh {
    /// Vertex positions.
    pub positions: Vec<[f32; 3]>,
    /// Faceted normals.
    pub normals: Vec<[f32; 3]>,
    /// Material colour (RGB) and program id, plus [`TEAM_BIT`] on owner-coloured faces.
    pub colors: Vec<[u8; 4]>,
    /// Second material colour (RGB) and the contact shade (alpha, 255 = unshaded).
    pub alts: Vec<[u8; 4]>,
    /// Pattern size of the material, in tiles.
    pub scales: Vec<f32>,
    /// Triangle list, counter-clockwise seen from outside.
    pub indices: Vec<u16>,
}

impl ModelMesh {
    /// Number of triangles.
    pub fn triangles(&self) -> usize {
        self.indices.len() / 3
    }

    /// Smallest box holding every vertex: `[min, max]`; zero for an empty mesh.
    pub fn bounds(&self) -> [[f32; 3]; 2] {
        bounds_of(&self.positions)
    }

    fn truncate(&mut self, vertices: usize, indices: usize) {
        self.positions.truncate(vertices);
        self.normals.truncate(vertices);
        self.colors.truncate(vertices);
        self.alts.truncate(vertices);
        self.scales.truncate(vertices);
        self.indices.truncate(indices);
    }
}

fn bounds_of(points: &[[f32; 3]]) -> [[f32; 3]; 2] {
    if points.is_empty() {
        return [[0.0; 3]; 2];
    }
    let mut lo = [f32::MAX; 3];
    let mut hi = [f32::MIN; 3];
    for p in points {
        for k in 0..3 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    [lo, hi]
}

/// One placed part of a baked model, for labels, outlines and the manifest.
#[derive(Clone, Debug, PartialEq)]
pub struct PartInfo {
    /// The part's name; parts of an include are `<include>/<part>`.
    pub path: String,
    /// The shape.
    pub shape: Shape,
    /// Its box in the model frame: `[min, max]`.
    pub bounds: [[f32; 3]; 2],
    /// Triangles in the near version.
    pub triangles: usize,
}

/// A model baked into its near and far versions.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BakedModel {
    /// Full detail.
    pub near: ModelMesh,
    /// Fewer segments, no openings, small parts left out.
    pub far: ModelMesh,
    /// Every placed part of the near version, in bake order.
    pub parts: Vec<PartInfo>,
    /// Problems met while baking (unknown attach targets), one line each.
    pub warnings: Vec<String>,
}

impl BakedModel {
    /// Box of the near version in the model frame: `[min, max]`.
    pub fn bounds(&self) -> [[f32; 3]; 2] {
        self.near.bounds()
    }

    /// Stable hash of the near and far vertex data, quantised to 1/1024 tile.
    pub fn fingerprint(&self) -> u64 {
        let mut hash = Fnv::default();
        for mesh in [&self.near, &self.far] {
            hash.word(mesh.positions.len() as u64);
            for p in mesh.positions.iter().chain(&mesh.normals) {
                for c in p {
                    hash.word(quantise(*c) as u64);
                }
            }
            for c in mesh.colors.iter().chain(&mesh.alts) {
                hash.bytes(c);
            }
            for s in &mesh.scales {
                hash.word(quantise(*s) as u64);
            }
            for i in &mesh.indices {
                hash.word(u64::from(*i));
            }
        }
        hash.0
    }
}

/// `value` in 1/1024 units, rounded.
fn quantise(value: f32) -> i64 {
    (f64::from(value) * 1024.0).round() as i64
}

/// FNV-1a, 64 bits.
struct Fnv(u64);

impl Default for Fnv {
    fn default() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }
}

impl Fnv {
    fn bytes(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.0 ^= u64::from(*b);
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    fn word(&mut self, word: u64) {
        self.bytes(&word.to_le_bytes());
    }
}

/// Bakes `spec` into its near and far versions.
pub fn bake_model(spec: &ModelSpec) -> BakedModel {
    let mut near = Bake::new(true);
    near.emit(spec, Place::IDENTITY, "", 0);
    let mut far = Bake::new(false);
    far.emit(spec, Place::IDENTITY, "", 0);
    BakedModel {
        near: near.mesh,
        far: far.mesh,
        parts: near.parts,
        warnings: near.warnings,
    }
}

/// One version of a model being baked.
struct Bake {
    /// The near version (else the far one).
    fine: bool,
    mesh: ModelMesh,
    parts: Vec<PartInfo>,
    /// Whether placed parts are recorded (not for the second and later copies of an include).
    record: bool,
    warnings: Vec<String>,
}

/// Where parts can attach: placed parts by name and their sockets.
#[derive(Default)]
struct Anchors {
    parts: BTreeMap<String, Place>,
    sockets: Vec<(String, String, Place)>,
}

impl Anchors {
    fn find(&self, attach: &str) -> Option<Place> {
        if let Some(place) = self.parts.get(attach) {
            return Some(*place);
        }
        if let Some((part, socket)) = attach.split_once('.') {
            return self
                .sockets
                .iter()
                .find(|(p, s, _)| p == part && s == socket)
                .map(|(_, _, place)| *place);
        }
        self.sockets
            .iter()
            .find(|(_, s, _)| s == attach)
            .map(|(_, _, place)| *place)
    }
}

impl Bake {
    fn new(fine: bool) -> Self {
        Self {
            fine,
            mesh: ModelMesh::default(),
            parts: Vec::new(),
            record: true,
            warnings: Vec::new(),
        }
    }

    /// Bakes the parts of `spec` placed at `place`, their names prefixed with `prefix`.
    fn emit(&mut self, spec: &ModelSpec, place: Place, prefix: &str, depth: usize) {
        let fine = self.fine;
        let mut anchors = Anchors::default();
        for part in &spec.parts {
            let path = format!("{prefix}{}", part.name);
            let parent = if part.attach.is_empty() {
                place
            } else {
                match anchors.find(&part.attach) {
                    Some(anchor) => anchor,
                    None => {
                        self.warnings.push(format!(
                            "{path}: attaches to {:?}, which is not an earlier part or socket",
                            part.attach
                        ));
                        place
                    }
                }
            };
            let knob = |k: &str, default: f32| part.knobs.get(k).copied().unwrap_or(default);
            let copies = if !fine && knob("far", 1.0) < 0.5 {
                0
            } else {
                knob("repeat", 1.0).round().clamp(1.0, 64.0) as usize
            };
            let (dx, dz) = (knob("dx", 0.0), knob("dz", 0.0));
            // A roof's own `pitch` is its slope; such shapes lean by `roll` only.
            let lean_pitch = if part.shape.knobs().contains(&"pitch") {
                0.0
            } else {
                knob("pitch", 0.0)
            };
            let start = (self.mesh.positions.len(), self.mesh.indices.len());
            let place_of = |copy: usize| {
                parent.then(&Place {
                    yaw: knob("yaw", 0.0).to_radians(),
                    scale: if part.shape == Shape::Include {
                        knob("scale", 1.0).max(0.01)
                    } else {
                        1.0
                    },
                    at: [
                        knob("x", 0.0) + dx * copy as f32,
                        knob("y", 0.0),
                        knob("z", 0.0) + dz * copy as f32,
                    ],
                    tilt: kit::tilt(lean_pitch, knob("roll", 0.0)),
                })
            };
            // Later parts attach to the first copy, even when the far version leaves the part out.
            anchors.parts.insert(part.name.clone(), place_of(0));
            for copy in 0..copies {
                let at = place_of(copy);
                let copy_start = (self.mesh.positions.len(), self.mesh.indices.len());
                if part.shape == Shape::Include {
                    if let Some(inner) = &part.include
                        && depth < MAX_INCLUDE_DEPTH
                    {
                        let record = self.record;
                        self.record = record && copy == 0;
                        self.emit(inner, at, &format!("{path}/"), depth + 1);
                        self.record = record;
                    }
                    continue;
                }
                let tint = part.colours.get("tint").copied().unwrap_or([1.0; 3]);
                let colour = std::array::from_fn(|k| part.material.colour[k] * tint[k]);
                let alt = std::array::from_fn(|k| part.material.colour_alt[k] * tint[k]);
                let surface = Surface {
                    colour,
                    alt,
                    program: part.material.program.id() as u8,
                    team: part.team,
                    scale: part.material.scale,
                };
                let mesh = &mut self.mesh;
                let mut painter = Painter::new(mesh, at, surface);
                let sockets = kit::paint(part, &mut painter, fine);
                for (name, local) in sockets {
                    anchors
                        .sockets
                        .push((part.name.clone(), name.into(), at.then(&local)));
                }
                shade_contact(mesh, copy_start.0);
                if !fine {
                    let span = bounds_of(&mesh.positions[copy_start.0..]);
                    let extent = (0..3)
                        .map(|k| span[1][k] - span[0][k])
                        .fold(0.0_f32, f32::max);
                    if extent < FAR_MIN_EXTENT {
                        mesh.truncate(copy_start.0, copy_start.1);
                    }
                }
            }
            if part.shape != Shape::Include && self.record {
                self.parts.push(PartInfo {
                    path,
                    shape: part.shape,
                    bounds: bounds_of(&self.mesh.positions[start.0..]),
                    triangles: (self.mesh.indices.len() - start.1) / 3,
                });
            }
        }
    }
}

/// Darkens the vertices from `start` on near the ground and near the base of their part when
/// it stands on another part.
fn shade_contact(mesh: &mut ModelMesh, start: usize) {
    let positions = &mesh.positions[start..];
    let Some(base) = positions.iter().map(|p| p[1]).reduce(f32::min) else {
        return;
    };
    let smooth = |t: f32| {
        let t = t.clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    };
    for (i, p) in positions.iter().enumerate() {
        let ground = GROUND_SHADE + (1.0 - GROUND_SHADE) * smooth(p[1] / GROUND_SHADE_HEIGHT);
        let meet = if base > 0.03 {
            MEET_SHADE + (1.0 - MEET_SHADE) * smooth((p[1] - base) / MEET_SHADE_HEIGHT)
        } else {
            1.0
        };
        let shade = ground.min(meet);
        let alpha = &mut mesh.alts[start + i][3];
        *alpha = (f32::from(*alpha) * shade).round().clamp(0.0, 255.0) as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::looks::Program;

    pub(crate) fn plain() -> MaterialLook {
        MaterialLook {
            program: Program::Plaster,
            colour: [0.8, 0.7, 0.6],
            colour_alt: [0.6, 0.5, 0.4],
            scale: 0.5,
        }
    }

    fn one(part: PartSpec) -> BakedModel {
        bake_model(&ModelSpec { parts: vec![part] })
    }

    #[test]
    fn shapes_are_named_by_their_rows() {
        for shape in Shape::ALL {
            assert_eq!(Shape::from_name(shape.name()), Some(shape));
        }
        assert_eq!(Shape::from_name("blob"), None);
    }

    #[test]
    fn repeat_places_copies_along_the_offsets() {
        let part = PartSpec::new("post", Shape::Box, plain())
            .knob("w", 0.2)
            .knob("d", 0.2)
            .knob("h", 1.0)
            .knob("repeat", 3.0)
            .knob("dx", 1.0)
            .knob("x", -1.0);
        let baked = one(part);
        let [lo, hi] = baked.bounds();
        assert!(
            (lo[0] + 1.1).abs() < 1e-5 && (hi[0] - 1.1).abs() < 1e-5,
            "{lo:?} {hi:?}"
        );
        assert_eq!(baked.near.triangles(), 3 * 10);
        assert_eq!(baked.parts.len(), 1);
    }

    #[test]
    fn includes_place_their_parts_with_the_include() {
        let house = Arc::new(ModelSpec {
            parts: vec![
                PartSpec::new("body", Shape::Box, plain())
                    .knob("w", 1.0)
                    .knob("d", 1.0)
                    .knob("h", 1.0),
            ],
        });
        let mut include = PartSpec::new("east", Shape::Include, plain())
            .knob("x", 3.0)
            .knob("yaw", 90.0)
            .knob("scale", 2.0);
        include.include = Some(Arc::clone(&house));
        let baked = one(include);
        let [lo, hi] = baked.bounds();
        assert!(
            (lo[0] - 2.0).abs() < 1e-4 && (hi[0] - 4.0).abs() < 1e-4,
            "{lo:?} {hi:?}"
        );
        assert!((hi[1] - 2.0).abs() < 1e-4);
        assert_eq!(baked.parts[0].path, "east/body");
        assert_eq!(baked.near.triangles(), 10);
    }

    #[test]
    fn team_flags_mark_only_team_parts() {
        let mut flag = PartSpec::new("flag", Shape::Banner, plain());
        flag.team = true;
        let wall = PartSpec::new("wall", Shape::Box, plain());
        let baked = bake_model(&ModelSpec {
            parts: vec![flag, wall],
        });
        let team = |c: &[u8; 4]| c[3] & TEAM_BIT != 0;
        let flagged = baked.near.colors.iter().filter(|c| team(c)).count();
        assert!(flagged > 0);
        // The pole is not team coloured, nor is any vertex of the wall.
        assert!(flagged < baked.near.colors.len() / 2);
        let wall_start = baked.near.positions.len() - 20;
        assert!(baked.near.colors[wall_start..].iter().all(|c| !team(c)));
    }

    #[test]
    fn attach_follows_sockets_and_parts() {
        let figure = PartSpec::new("body", Shape::Figure, plain()).knob("x", 2.0);
        let mut tool = PartSpec::new("tool", Shape::Cylinder, plain())
            .knob("w", 0.05)
            .knob("h", 0.5);
        tool.attach = "right_hand".into();
        let mut lost = tool.clone();
        lost.name = "lost".into();
        lost.attach = "nowhere".into();
        let baked = bake_model(&ModelSpec {
            parts: vec![figure, tool, lost],
        });
        let hand = baked.parts[1].bounds;
        // The right hand of a figure facing south is on its west side, beside x = 2.
        assert!(hand[0][0] > 1.6 && hand[1][0] < 2.0, "{hand:?}");
        assert!(hand[0][1] > 0.3, "{hand:?}");
        assert_eq!(baked.warnings.len(), 1);
    }

    #[test]
    fn contact_shading_darkens_the_foot_of_a_part() {
        let baked = one(PartSpec::new("body", Shape::Box, plain()).knob("h", 2.0));
        let shade_at = |y: f32| {
            baked
                .near
                .positions
                .iter()
                .zip(&baked.near.alts)
                .find(|(p, _)| (p[1] - y).abs() < 1e-4)
                .map(|(_, a)| a[3])
                .unwrap()
        };
        assert!(shade_at(0.0) < 170);
        assert_eq!(shade_at(2.0), 255);
    }

    #[test]
    fn bakes_and_fingerprints_are_stable() {
        let part = PartSpec::new("dome", Shape::Dome, plain()).knob("w", 2.0);
        let (a, b) = (one(part.clone()), one(part.clone()));
        assert_eq!(a, b);
        assert_eq!(a.fingerprint(), b.fingerprint());
        let moved = one(part.knob("x", 0.01));
        assert_ne!(a.fingerprint(), moved.fingerprint());
    }

    #[test]
    fn a_roof_pitch_is_its_slope_not_a_lean() {
        let roof = |knobs: &[(&str, f32)]| {
            let mut part = PartSpec::new("roof", Shape::GableRoof, plain());
            for (k, v) in knobs {
                part = part.knob(k, *v);
            }
            one(part)
        };
        let steep = roof(&[("pitch", 50.0)]);
        // Leaning by roll still works; the slope does not lean the part.
        let [lo, hi] = steep.bounds();
        assert!((lo[0] + hi[0]).abs() < 1e-4 && (lo[2] + hi[2]).abs() < 1e-4);
        let rolled = roof(&[("pitch", 50.0), ("roll", 20.0)]);
        assert_ne!(rolled.bounds(), steep.bounds());
    }

    #[test]
    fn far_versions_drop_small_parts() {
        let big = PartSpec::new("big", Shape::Box, plain());
        let small = PartSpec::new("small", Shape::Box, plain())
            .knob("w", 0.1)
            .knob("d", 0.1)
            .knob("h", 0.1);
        let baked = bake_model(&ModelSpec {
            parts: vec![big, small],
        });
        assert_eq!(baked.near.triangles(), 20);
        assert_eq!(baked.far.triangles(), 10);
    }
}
