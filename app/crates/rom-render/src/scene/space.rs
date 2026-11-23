//! Drawing the continuous zoom as one picture through one camera: the sky, the star map, the
//! anchored system and the loaded globe, far to near, each in its own depth range.
//!
//! Level of detail by distance, never by a swap of frames: the loaded globe is drawn as itself
//! while it is more than a few pixels wide, then as a small sphere textured with the planet
//! image; seen from the ground the star and the planets are small discs in the sky that grow to
//! their places as the camera leaves the planet; out on the star map the systems are additive
//! sprites, and the anchored system's star swells into its halo as its detail fades.

use raylib::consts::{BlendMode, MaterialMapIndex};
use raylib::prelude::{Rectangle, Texture2D};

use super::*;
use crate::entities::{Hull, owner_color};
use crate::globe::PlanetScene;
use crate::hud_minimap::planet_pixels;
use crate::layers::{
    CAPITAL_CLASS, FLOOR_BAND, FrameView, OneView, PlanetPlace, ShipScales, SystemPlace,
    apparent_share, morph_size, plane_unit, ship_on_screen, ship_true_length, system_detail,
};
use crate::space::{
    FIGURE_DASH, MapLineKind, SpaceHover, SpaceSnapshot, annulus, arc, capital_rings, chevron,
    chevron3, claim_rings, class_marker, colony_rings, diamond, facing, free_frame_pos,
    map_colony_rings, map_mark_scale, map_ship_marks, ship_frame_pos, star_map_lines,
    sticky_groups, transit_at_rim,
};
use crate::universe::{
    ACCRETION_RGB, BodyViewKind, CELESTIAL_RGB, MAP_RADIUS, PlacedBody, SYSTEM_RADIUS,
    UniverseScene, drawn_pixels, least_pixels, planet_colour, ring_points, rock_colour,
    shown_pixels, uv_sphere,
};
use crate::zoom::{Target, ZoomCamera};

const BODY_VS: &str = r#"#version 330
in vec3 vertexPosition;
in vec3 vertexNormal;
in vec2 vertexTexCoord;
uniform mat4 mvp;
uniform mat4 matModel;
out vec3 fragNormal;
out vec3 fragWorld;
out vec2 fragUv;
void main() {
    fragUv = vertexTexCoord;
    fragNormal = mat3(matModel) * vertexNormal;
    fragWorld = (matModel * vec4(vertexPosition, 1.0)).xyz;
    gl_Position = mvp * vec4(vertexPosition, 1.0);
}
"#;

// The star sits at the origin of the system frame and lights every body from there.
fn body_fs() -> String {
    format!(
        r#"#version 330
in vec3 fragNormal;
in vec3 fragWorld;
in vec2 fragUv;
uniform sampler2D texture0;
uniform vec3 tint;
uniform float emissive;
uniform float alpha;
out vec4 finalColor;
{}
void main() {{
    vec3 base = texture(texture0, fragUv).rgb * tint;
    vec3 normal = normalize(fragNormal);
    vec3 toStar = normalize(-fragWorld);
    float height = dot(normal, toStar);
    vec3 lit = base * terrainFactor(height, height, 1.0);
    finalColor = vec4(mix(lit, base * 1.3, emissive), alpha);
}}
"#,
        sun::glsl_lighting()
    )
}

/// Rings, planets and sprites are drawn in this many segments per orbit.
const RING_SEGMENTS: u32 = 128;
/// Diameter of a system's halo on the star map, in map units.
const HALO_SIZE: f32 = 2500.0;
/// Diameter of the bright core of a system on the star map, in map units.
const CORE_SIZE: f32 = 650.0;
/// Diameter of the star's glow in the system frame, in system units.
const STAR_GLOW: f32 = 2600.0;
const GLOW_SIDE: u32 = 128;
const ORBIT_RING: Color = Color::new(120, 140, 185, 58);
/// The radius on screen below which a body also wears a soft halo so it can still be seen,
/// hovered and double-clicked.
const HALO_BELOW_PIXELS: f32 = 6.0;
/// Diameter of a body's halo on screen, in pixels.
const HALO_PIXELS: f32 = 22.0;
/// The ring that fills while a ship charges its jump.
const CHARGE_RING: [u8; 3] = [255, 196, 64];
/// A jump inhibitor holding a system: its mark on the star map and its boundary in the system.
const HOLD_ENEMY: [u8; 3] = [255, 92, 72];
/// Widest a system's halo and core are drawn on screen, in pixels: a star the camera flies
/// close by does not swell over the picture.
const MAP_HALO_MAX_PIXELS: f32 = 70.0;
/// Share of its brightness that a system keeps at the far edge of the map, and of the depth range
/// (map radii either side of the focus) it fades over.
const FAR_FADE: f32 = 0.32;
/// Diameters in pixels of a system's halo and core seen from the Sun.
const SKY_HALO_PIXELS: f32 = 20.0;
const SKY_CORE_PIXELS: f32 = 8.0;
/// Widest the star's glow is drawn on screen, in pixels.
const STAR_GLOW_PIXELS: f32 = 140.0;
/// Radius in pixels, seen from the ground, of the star and of the other bodies in the sky.
const SKY_STAR_DISC: f32 = 7.0;
const SKY_PLANET_DISC: f32 = 3.5;
const SKY_ROCK_DISC: f32 = 2.5;
/// Diameter in pixels of another system's point of light seen from inside a system.
const SKY_STAR_PIXELS: f32 = 3.0;
/// Light from above the system's plane on ships drawn as their models, out in space (none in
/// the planet's orbit, where the globe lights them).
const STAR_FILL: f32 = 0.35;

/// What is in space and which ships are selected, kept for the next draw.
#[derive(Default)]
pub(super) struct SpaceState {
    snapshot: Arc<SpaceSnapshot>,
    pub(super) selected: Vec<u32>,
    hover: Option<SpaceHover>,
    /// The ships above the loaded globe as it draws them, in the planet frame.
    orbital: Vec<OrbitalShip>,
    /// How large each ship type is drawn.
    scales: Arc<ShipScales>,
}

/// A ship above the loaded globe as the globe draws it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct OrbitalShip {
    /// Entity id.
    pub(crate) id: u32,
    /// Its hull in the planet frame.
    pub(crate) hull: Hull,
}

/// One system drawn through the one camera.
#[derive(Clone, Copy, Debug, PartialEq)]
struct SystemPass {
    /// Index of the system in the universe.
    index: usize,
    /// The one camera in the system's frame.
    view: FrameView,
    /// Distance from the eye to the system's star.
    distance: f32,
    /// How much of its own detail shows.
    detail: f32,
    /// How much of the space around the loaded planet shows (1 away from it).
    space: f32,
    /// Whether the player knows its bodies.
    known: bool,
    /// Whether the loaded globe draws itself in this system.
    globe: bool,
    /// Where the loaded planet sits, when it lies in this system.
    place: Option<PlanetPlace>,
}

/// Which part of the system layer a pass draws, by distance from the eye: the loaded globe is
/// drawn between the part behind its centre and the part in front of it.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Part {
    /// Everything (no globe is drawn).
    All,
    /// What lies at least this far from the eye.
    Behind(f32),
    /// What lies nearer than this.
    Before(f32),
}

impl Part {
    fn keeps(self, distance: f32) -> bool {
        match self {
            Part::All => true,
            Part::Behind(d) => distance >= d,
            Part::Before(d) => distance < d,
        }
    }

    /// Whether this is the pass that also places the names.
    fn first(self) -> bool {
        !matches!(self, Part::Before(_))
    }
}

/// The radius in pixels a body of true radius `radius` is drawn with, `per_pixel` units a pixel
/// away, before the floors of [`shown_pixels`]: seen from the ground (`space` = 0) a small disc
/// in the sky, growing to its true size as the camera leaves the planet.
fn sky_pixels(kind: BodyViewKind, radius: f32, per_pixel: f32, space: f32) -> f32 {
    let sky = match kind {
        BodyViewKind::Star => SKY_STAR_DISC,
        BodyViewKind::Habitable | BodyViewKind::Celestial => SKY_PLANET_DISC,
        BodyViewKind::Resource => SKY_ROCK_DISC,
    };
    let per_pixel = per_pixel.max(1e-9);
    let pixels = radius / per_pixel;
    if pixels > sky {
        morph_size(sky, pixels, space)
    } else {
        pixels
    }
}

/// A ship's hull on its way from above the loaded globe (`above`, planet frame) to its place on
/// the system plane (`plane`), `t` of the way.
fn morph_hull(above: &Hull, place: &PlanetPlace, plane: &Hull, t: f32) -> Hull {
    let lerp = |a: [f32; 3], b: [f32; 3]| [0, 1, 2].map(|i| a[i] + (b[i] - a[i]) * t);
    let length = above.length * place.per_tile;
    Hull {
        origin: lerp(place.to_system_point(above.origin), plane.origin),
        forward: unit_vector(lerp(place.frame.carry(above.forward), plane.forward)),
        up: unit_vector(lerp(place.frame.carry(above.up), plane.up)),
        length: morph_size(length, plane.length, t),
        height: 0.0,
    }
}

/// `colour` with its alpha scaled by `by`.
fn fade(colour: Color, by: f32) -> Color {
    Color::new(
        colour.r,
        colour.g,
        colour.b,
        (f32::from(colour.a) * by.clamp(0.0, 1.0)) as u8,
    )
}

/// Scales the alpha of every line and triangle of `overlay` by `by`.
fn fade_overlay(overlay: &mut OverlayGeometry, by: f32) {
    let by = by.clamp(0.0, 1.0);
    let scaled = |a: u8| (f32::from(a) * by) as u8;
    for line in &mut overlay.lines {
        line.colour[3] = scaled(line.colour[3]);
    }
    for tri in &mut overlay.tris {
        tri.colour[3] = scaled(tri.colour[3]);
    }
}

fn centroid(tri: &crate::overlay::Tri3) -> [f32; 3] {
    [0, 1, 2].map(|i| (tri.v[0][i] + tri.v[1][i] + tri.v[2][i]) / 3.0)
}

fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// A colour from owner palette values.
fn owner_rgb(owner: u8) -> [u8; 3] {
    owner_color(owner).map(|c| (c * 255.0).round() as u8)
}

fn lerp_colour(colour: [u8; 3], toward: f32) -> [f32; 3] {
    colour.map(|c| (f32::from(c) / 255.0) * (1.0 - toward) + toward)
}

/// A name to draw next to a system on the star map, in pixels of the render target.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SpaceLabel {
    pub(crate) text: String,
    /// An HUD atlas icon drawn before the text (alone when the text is empty).
    pub(crate) icon: Option<String>,
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) colour: [u8; 4],
    /// Lower comes first when labels compete for room.
    pub(crate) priority: f32,
}

/// Meshes, shader and sprites of the system and star-map frames.
pub(super) struct SpaceGpu {
    universe: Arc<UniverseScene>,
    home_body: Option<usize>,
    map_positions: Vec<[f32; 3]>,
    /// The median distance from a system to its nearest neighbour, map units: halos never grow
    /// past it, so a crowded map stays readable.
    gap: f32,
    /// The bodies of every system placed in its frame, indexed like the systems.
    placed: Vec<Vec<PlacedBody>>,
    tints: Vec<Option<[u8; 3]>>,
    shader: Shader,
    /// The shader's `tint`, `emissive` and `alpha` uniform locations, looked up once.
    uniforms: [i32; 3],
    material: WeakMaterial,
    lod_material: WeakMaterial,
    sphere: Mesh,
    lod: Mesh,
    glow: Texture2D,
    planet_texture: Option<Texture2D>,
    labels: Vec<SpaceLabel>,
    /// Which group each counted ship belonged to last frame, ship id to its leader's id, so a
    /// group does not break up and re-form as ships drift in and out of overlap (task 105).
    ship_groups: std::collections::BTreeMap<u32, u32>,
}

/// Uploads a unit sphere with normals and texture coordinates.
pub(super) fn upload_sphere(thread: &RaylibThread, rings: u32, slices: u32) -> Mesh {
    let sphere = uv_sphere(rings, slices);
    let positions: Vec<Vector3> = sphere
        .positions
        .iter()
        .map(|p| Vector3::new(p[0], p[1], p[2]))
        .collect();
    let normals = positions.clone();
    let texcoords: Vec<Vector2> = sphere
        .texcoords
        .iter()
        .map(|t| Vector2::new(t[0], t[1]))
        .collect();
    let mut builder = Mesh::gen_mesh(&positions, &texcoords);
    builder.normals(&normals);
    builder.indices(&sphere.indices);
    builder
        .build(thread)
        .unwrap_or_else(|_| Mesh::gen_mesh_sphere(thread, 1.0, rings as i32, slices as i32))
}

/// A soft round sprite: white, alpha falling off from the centre.
fn glow_pixels() -> Vec<u8> {
    let mut pixels = Vec::with_capacity((GLOW_SIDE * GLOW_SIDE * 4) as usize);
    for y in 0..GLOW_SIDE {
        for x in 0..GLOW_SIDE {
            let dx = (x as f32 + 0.5) / GLOW_SIDE as f32 * 2.0 - 1.0;
            let dy = (y as f32 + 0.5) / GLOW_SIDE as f32 * 2.0 - 1.0;
            let r = dx.hypot(dy).min(1.0);
            let a = (1.0 - r).powf(1.6);
            pixels.extend_from_slice(&[255, 255, 255, (a * 255.0) as u8]);
        }
    }
    pixels
}

/// A faint circle of `radius` about `centre` in the plane of the system; only the segments
/// whose middle `keeps` accepts.
fn draw_ring(
    world: &mut impl RaylibDraw3D,
    centre: [f32; 3],
    radius: f32,
    colour: Color,
    keeps: &dyn Fn([f32; 3]) -> bool,
) {
    if colour.a == 0 {
        return;
    }
    let ring = ring_points(radius, RING_SEGMENTS);
    for pair in ring.windows(2) {
        let at = |p: [f32; 3]| [p[0] + centre[0], p[1], p[2] + centre[2]];
        let (a, b) = (at(pair[0]), at(pair[1]));
        if keeps([0, 1, 2].map(|i| (a[i] + b[i]) / 2.0)) {
            world.draw_line3D(vec3(a), vec3(b), colour);
        }
    }
}

/// A sphere of `radius` at `pos`, with the planet frame's three unit axes.
fn ball(pos: [f32; 3], radius: f32, axes: [[f32; 3]; 3]) -> Matrix {
    let [x, y, z] = axes.map(|axis| axis.map(|c| c * radius));
    Matrix {
        m0: x[0],
        m1: x[1],
        m2: x[2],
        m4: y[0],
        m5: y[1],
        m6: y[2],
        m8: z[0],
        m9: z[1],
        m10: z[2],
        m12: pos[0],
        m13: pos[1],
        m14: pos[2],
        ..Matrix::identity()
    }
}

fn vec3(v: [f32; 3]) -> Vector3 {
    Vector3::new(v[0], v[1], v[2])
}

fn colour3(c: [u8; 3]) -> [f32; 3] {
    c.map(|v| f32::from(v) / 255.0)
}

impl Scene {
    /// Builds the meshes and sprites of the system and star-map frames for `universe`, with the
    /// home planet's image as the texture of its low-detail sphere.
    pub(crate) fn set_universe(
        &mut self,
        rl: &mut RaylibHandle,
        thread: &RaylibThread,
        universe: &Arc<UniverseScene>,
        home_body: Option<u32>,
        planet: Option<&PlanetScene>,
    ) {
        let mut shader = rl.load_shader_from_memory(thread, Some(BODY_VS), Some(&body_fs()));
        let mut material = rl.load_material_default(thread);
        material.set_shader(&shader);
        let mut lod_material = rl.load_material_default(thread);
        lod_material.set_shader(&shader);
        let tint = shader.get_shader_location("tint");
        shader.set_shader_value(tint, [1.0_f32, 1.0, 1.0]);
        let planet_texture = planet.and_then(|planet| {
            let (w, h, rgba) = planet_pixels(planet);
            crate::hud_gpu::upload(rl, thread, w, h, &rgba).ok()
        });
        if let Some(texture) = &planet_texture {
            lod_material.set_material_texture(MaterialMapIndex::MATERIAL_MAP_ALBEDO, texture);
        }
        let glow = crate::hud_gpu::upload(rl, thread, GLOW_SIDE, GLOW_SIDE, &glow_pixels())
            .expect("a small RGBA texture uploads");
        let tints = universe
            .systems
            .iter()
            .map(|s| {
                universe
                    .bodies
                    .iter()
                    .find(|b| b.system == s.id && b.home_race_colour.is_some())
                    .and_then(|b| b.home_race_colour)
            })
            .collect();
        self.space = Some(SpaceGpu {
            universe: Arc::clone(universe),
            home_body: home_body.and_then(|id| universe.body_index(id)),
            map_positions: universe.place_systems(),
            gap: nearest_gap(&universe.place_systems()),
            placed: universe
                .systems
                .iter()
                .map(|s| universe.place_system(s.id))
                .collect(),
            tints,
            uniforms: ["tint", "emissive", "alpha"].map(|n| shader.get_shader_location(n)),
            shader,
            material,
            lod_material,
            sphere: upload_sphere(thread, 20, 40),
            lod: upload_sphere(thread, 12, 24),
            glow,
            planet_texture,
            labels: Vec::new(),
            ship_groups: std::collections::BTreeMap::new(),
        });
    }

    /// Takes the universe the zoom camera now shows after bodies changed (a moon brought to
    /// life, task 77): the same systems and positions, bodies placed again.
    pub(crate) fn replace_universe(&mut self, universe: &Arc<UniverseScene>) {
        if let Some(space) = &mut self.space {
            space.placed = universe
                .systems
                .iter()
                .map(|s| universe.place_system(s.id))
                .collect();
            space.universe = Arc::clone(universe);
        }
    }

    /// Keeps what is in space and the selected entity ids for the next draw.
    pub(crate) fn set_space_state(&mut self, snapshot: &Arc<SpaceSnapshot>, selected: &[u32]) {
        if !Arc::ptr_eq(&self.space_state.snapshot, snapshot) {
            self.space_state.snapshot = Arc::clone(snapshot);
        }
        if self.space_state.selected != selected {
            self.space_state.selected = selected.to_vec();
        }
    }

    /// What the pointer rests on in space, highlighted in the next draw.
    pub(crate) fn set_space_hover(&mut self, hover: Option<SpaceHover>) {
        self.space_state.hover = hover;
    }

    /// The ships above the loaded globe for the next draw, where the globe draws them.
    pub(crate) fn set_orbital_ships(&mut self, ships: impl IntoIterator<Item = OrbitalShip>) {
        self.space_state.orbital.clear();
        self.space_state.orbital.extend(ships);
    }

    /// How large each ship type is drawn in the space views.
    pub(crate) fn set_ship_scales(&mut self, scales: &Arc<ShipScales>) {
        if !Arc::ptr_eq(&self.space_state.scales, scales) {
            self.space_state.scales = Arc::clone(scales);
        }
    }

    /// Adds a name to draw over the frame just drawn, in render-target pixels.
    pub(crate) fn push_space_label(&mut self, label: SpaceLabel) {
        if let Some(space) = &mut self.space {
            space.labels.push(label);
        }
    }

    /// The names to draw over the last frame, in render-target pixels.
    pub(crate) fn space_labels(&self) -> &[SpaceLabel] {
        self.space.as_ref().map_or(&[], |s| &s.labels)
    }

    /// Whether the system and star-map frames exist.
    pub(crate) fn has_space(&self) -> bool {
        self.space.is_some()
    }
    /// Draws what the one camera of the zoom sees into one picture: the sky, the star map, the
    /// anchored system and the loaded globe, far to near, each layer in its own depth range and
    /// with the same camera. Nothing is cross-faded: detail fades with the distance.
    pub(crate) fn draw_zoomed(
        &mut self,
        rl: &mut RaylibHandle,
        thread: &RaylibThread,
        target: &mut RenderTexture2D,
        zoom: &ZoomCamera,
        orbit: &OrbitCamera,
    ) {
        let one = zoom.one_view(orbit);
        if let Some(space) = &mut self.space {
            space.labels.clear();
        }
        self.screen_bars.clear();
        self.screen_emblems.clear();
        let height = target.texture.height as f32;
        // The loaded globe is drawn as itself while it is more than a few pixels wide; the
        // system draws it as a small sphere below that.
        let globe = match (&one.planet, &one.place) {
            (Some(view), Some(place)) if self.globe.is_some() => {
                let pixels = view.sphere_pixels([0.0; 3], orbit.radius, height);
                // Below a planet's floor the system's sphere, drawn at the floor, takes over.
                let floor = least_pixels(BodyViewKind::Habitable, 0.0, one.fades.space);
                (pixels >= floor).then(|| {
                    let eye = match &one.planet_camera {
                        Some(camera) => GlobeEye::of_orbit(camera),
                        None => GlobeEye::of_view(view, orbit.radius),
                    };
                    (eye, one.system.distance_to(place.centre))
                })
            }
            _ => None,
        };
        let frame = match &globe {
            Some((eye, _)) => self.globe_backdrop(rl, thread, target, orbit, eye),
            None => None,
        };
        if frame.is_none() {
            // From the Sun only the real stars show: the backdrop's made-up ones would mislead.
            self.space_backdrop(rl, thread, target, &one.system, !zoom.sky_active());
        }
        let stars = frame.as_ref().map_or(1.0, |f| f.stars);
        self.draw_map_layer(rl, thread, target, zoom, orbit, &one, stars);
        let drawn = globe.is_some() && frame.is_some();
        let mut passes = if zoom.sky_active() {
            Vec::new()
        } else {
            self.system_passes(
                zoom,
                &one,
                drawn,
                target.texture.width as f32 / height.max(1.0),
            )
        };
        // The farthest system first; the anchored one last, around the globe it holds.
        passes.sort_by(|a, b| {
            (a.index == zoom.system())
                .cmp(&(b.index == zoom.system()))
                .then(b.distance.total_cmp(&a.distance))
        });
        let mut globe_left = drawn;
        for pass in passes {
            match (&globe, &frame) {
                (Some((eye, split)), Some(frame)) if pass.index == zoom.system() => {
                    let behind = Part::Behind(*split);
                    self.draw_system_layer(rl, thread, target, zoom, &pass, behind);
                    self.globe_world(rl, thread, target, orbit, eye, frame);
                    globe_left = false;
                    let before = Part::Before(*split);
                    self.draw_system_layer(rl, thread, target, zoom, &pass, before);
                }
                _ => self.draw_system_layer(rl, thread, target, zoom, &pass, Part::All),
            }
        }
        if let (true, Some((eye, _)), Some(frame)) = (globe_left, &globe, &frame) {
            self.globe_world(rl, thread, target, orbit, eye, frame);
        }
    }

    /// The systems whose own detail shows through the one camera: the anchored one in its
    /// frame, any other seen through the star map in its own frame; `globe` when the loaded
    /// globe draws itself.
    fn system_passes(
        &self,
        zoom: &ZoomCamera,
        one: &OneView,
        globe: bool,
        aspect: f32,
    ) -> Vec<SystemPass> {
        let Some(space) = &self.space else {
            return Vec::new();
        };
        let k = one.system_place.per_unit;
        let mut passes = Vec::new();
        for (index, (system, &here)) in space
            .universe
            .systems
            .iter()
            .zip(&space.map_positions)
            .enumerate()
        {
            let anchored = index == zoom.system();
            let view = if anchored {
                one.system
            } else {
                SystemPlace { here, per_unit: k }.from_map(&one.map)
            };
            let distance = view.distance_to([0.0; 3]).max(1e-6);
            let detail = system_detail(SYSTEM_RADIUS / distance);
            if detail <= PASS_DETAIL_MIN {
                continue;
            }
            // A system off the picture draws nothing: its disc lies outside the view cone.
            if !anchored && !view.sees_sphere([0.0; 3], SYSTEM_RADIUS, aspect) {
                continue;
            }
            passes.push(SystemPass {
                index,
                view,
                distance,
                detail,
                space: if anchored { one.fades.space } else { 1.0 },
                known: self.space_state.snapshot.knows(system.id),
                globe: anchored && globe,
                place: one.place.filter(|_| anchored),
            });
        }
        passes
    }

    /// Clears `target` to space and draws the star field, turned with the camera `view`.
    fn space_backdrop(
        &mut self,
        rl: &mut RaylibHandle,
        thread: &RaylibThread,
        target: &mut RenderTexture2D,
        view: &FrameView,
        stars_on: bool,
    ) {
        self.set_star_fade(if stars_on { 1.0 } else { 0.0 });
        let mut drawing = rl.begin_texture_mode(thread, target);
        drawing.clear_background(SPACE);
        if let Some(stars) = &self.star_mesh {
            let forward = vec3(sub3(view.target, view.eye));
            let sky =
                Camera3D::perspective(Vector3::new(0.0, 0.0, 0.0), forward, vec3(view.up), 45.0);
            let mut backdrop = drawing.begin_mode3D(sky);
            shift_projection(self.projection_centre_y);
            backdrop.draw_mesh(stars, self.star_material.clone(), Matrix::identity());
        }
    }

    /// Draws the part `part` of one system through the one camera: bodies (the loaded planet
    /// only when the globe does not draw itself), rings, ships and marks. Seen from the ground
    /// the star and the planets are small discs in the sky; they grow to their places as the
    /// camera leaves the planet.
    fn draw_system_layer(
        &mut self,
        rl: &mut RaylibHandle,
        thread: &RaylibThread,
        target: &mut RenderTexture2D,
        zoom: &ZoomCamera,
        pass: &SystemPass,
        part: Part,
    ) {
        if pass.detail < 0.004 {
            return;
        }
        self.set_lighting(
            crate::sun::unit_or(super::SHIP_LIGHT.map(|c| -c)),
            false,
            None,
        );
        // Ships drawn as their models move as they do on the planet, even with no globe drawn.
        let time = self
            .anim_time
            .unwrap_or(self.clock.elapsed().as_secs_f32() % 3_600.0);
        if let Some(models) = &mut self.models {
            models.time = time;
        }
        let Some(space) = &mut self.space else {
            return;
        };
        let view = pass.view;
        let camera = Camera3D::perspective(vec3(view.eye), vec3(view.target), vec3(view.up), 45.0);
        let universe = Arc::clone(&space.universe);
        let bodies_placed = space.placed.get(pass.index).cloned().unwrap_or_default();
        let star_colour = universe
            .systems
            .get(pass.index)
            .map_or([255, 244, 214], |s| s.star_colour);
        // The celestial home (task 76): a black hole in a glowing disc, and the residences of
        // the ascended players on its mythical planet.
        let celestial = universe
            .systems
            .get(pass.index)
            .is_some_and(|s| s.celestial);
        let mut residence_labels: Vec<([f32; 3], String, [u8; 3])> = Vec::new();
        let [tint_at, emissive_at, alpha_at] = space.uniforms;
        let (width, height) = (target.texture.width as f32, target.texture.height as f32);
        let mut rim_labels: Vec<([f32; 3], u32, u32, f32)> = Vec::new();
        // Groups of overlapping ship markers: where their count goes, in pixels, and the owner.
        let mut group_counts: Vec<(f32, f32, usize, u8)> = Vec::new();
        // Jump inhibitors' blows on arriving ships (task 104): where, how large, how bright.
        let mut strike_flashes: Vec<([f32; 3], f32, f32)> = Vec::new();
        let bar_look = view.view((width, height), self.projection_centre_y);
        let mut ship_bars: Vec<crate::bars::ScreenBar> = Vec::new();
        let mut icons: Vec<([f32; 3], f32, String, f32)> = Vec::new();
        // Where a planet with known capitals stands, its drawn radius in pixels and the owners.
        let mut capital_spots: Vec<([f32; 3], f32, Vec<u8>)> = Vec::new();
        let keeps = |p: [f32; 3]| part.keeps(view.distance_to(p));
        let loaded = zoom.surface_body();
        // The loaded globe hides the stretch of a ring that runs through it.
        let globe_ball = bodies_placed
            .iter()
            .find(|b| pass.globe && Some(b.body) == loaded)
            .map(|b| (b.pos, b.radius));
        let ring_keeps = |p: [f32; 3]| {
            keeps(p)
                && globe_ball.is_none_or(|(centre, radius)| {
                    (0..3).map(|i| (p[i] - centre[i]).powi(2)).sum::<f32>() > radius * radius
                })
        };
        // Rings, claims and names belong to space: they fade in as the camera leaves the planet.
        let detail = pass.detail * pass.space;
        let mut drawing = rl.begin_texture_mode(thread, target);
        clear_depth();
        let far = view.distance_to([0.0; 3]) + SYSTEM_RADIUS * 2.5;
        let near = (view.distance() * 0.05).clamp(0.05, far * 0.01);
        set_clip_planes(f64::from(near), f64::from(far));
        {
            let mut world = drawing.begin_mode3D(camera);
            shift_projection(self.projection_centre_y);
            // An unexplored system shows its star only.
            let known = pass.known;
            if known && detail > 0.004 {
                // Planets keep their rings, moons a small one about their planet and the belt
                // one faint ring through its middle.
                let colour = fade(ORBIT_RING, detail);
                let mut belt = Vec::new();
                for placed in &bodies_placed {
                    match (universe.bodies[placed.body].kind, placed.moon_of) {
                        (BodyViewKind::Habitable | BodyViewKind::Celestial, _) => {
                            draw_ring(&mut world, [0.0; 3], placed.orbit, colour, &ring_keeps)
                        }
                        (BodyViewKind::Resource, Some(parent)) => {
                            if let Some(at) = bodies_placed.iter().find(|p| p.body == parent) {
                                draw_ring(&mut world, at.pos, placed.orbit, colour, &ring_keeps);
                            }
                        }
                        (BodyViewKind::Resource, None) => belt.push(placed.orbit),
                        _ => {}
                    }
                }
                if !belt.is_empty() {
                    let mean = belt.iter().sum::<f32>() / belt.len() as f32;
                    draw_ring(&mut world, [0.0; 3], mean, colour, &ring_keeps);
                }
            }
            let state = &self.space_state;
            // Every body's radius as drawn, its kind's floor on screen included.
            let per_pixels: Vec<f32> = bodies_placed
                .iter()
                .map(|p| view.per_pixel(p.pos, height).max(1e-9))
                .collect();
            let kinds: Vec<BodyViewKind> = bodies_placed
                .iter()
                .map(|p| universe.bodies[p.body].kind)
                .collect();
            let pixels: Vec<f32> = bodies_placed
                .iter()
                .zip(&kinds)
                .zip(&per_pixels)
                .map(|((p, &k), &per)| sky_pixels(k, p.radius, per, pass.space))
                .collect();
            let drawn: Vec<f32> = shown_pixels(&kinds, &pixels, pass.space)
                .into_iter()
                .zip(&per_pixels)
                .zip(&bodies_placed)
                .map(|((px, &per), p)| drawn_pixels(px, p.room, per) * per)
                .collect();
            // Planets holding a known capital (task 131): the emblems go above them.
            if known && part.first() {
                for ((placed, &radius), &per) in bodies_placed.iter().zip(&drawn).zip(&per_pixels) {
                    let body = &universe.bodies[placed.body];
                    let owners = crate::capitals::planet_owners(&state.snapshot.capitals, body.id);
                    if !owners.is_empty() {
                        capital_spots.push((placed.pos, radius / per, owners));
                    }
                }
            }
            // How much larger than true a body is drawn, for the rings about it.
            let grown = |body: u32| -> f32 {
                universe
                    .body_index(body)
                    .and_then(|index| bodies_placed.iter().position(|p| p.body == index))
                    .map_or(1.0, |i| {
                        (drawn[i] / bodies_placed[i].radius.max(1e-6)).max(1.0)
                    })
            };
            let mut halos: Vec<([f32; 3], f32)> = Vec::new();
            // The soft halos of bodies too small to read, drawn together after the bodies.
            let mut small_halos: Vec<([f32; 3], f32, [u8; 3], u8)> = Vec::new();
            let mut flashes: Vec<([f32; 3], f32, f32)> = Vec::new();
            for (placed, &radius) in bodies_placed.iter().zip(&drawn) {
                let body = &universe.bodies[placed.body];
                if body.kind != BodyViewKind::Star && !known {
                    continue;
                }
                // Blasts on the planet flash on its globe (task 75), the loaded globe included;
                // drawn over the atmosphere.
                // The loaded planet's globe may be drawn by a pass of its own: its flashes are
                // drawn here whichever part holds it, on the side facing the camera only.
                let globe_here = Some(placed.body) == loaded;
                if known && (keeps(placed.pos) || globe_here) {
                    let per_pixel = view.per_pixel(placed.pos, height);
                    let mut axes = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
                    if Some(placed.body) == loaded {
                        axes = axes.map(|axis| zoom.planet_to_system(axis));
                    }
                    for flash in state.snapshot.flashes.iter().filter(|f| f.body == body.id) {
                        let k = flash.age_ticks as f32 / crate::space::FLASH_TICKS as f32;
                        if k >= 1.0 {
                            continue;
                        }
                        let d = flash.dir;
                        let dir: [f32; 3] = std::array::from_fn(|i| {
                            axes[0][i] * d[0] + axes[1][i] * d[1] + axes[2][i] * d[2]
                        });
                        let at: [f32; 3] =
                            std::array::from_fn(|i| placed.pos[i] + dir[i] * radius * 1.03);
                        let toward: f32 = (0..3).map(|i| dir[i] * (view.eye[i] - at[i])).sum();
                        if globe_here && toward <= 0.0 {
                            continue;
                        }
                        let size = (radius * 0.5 * (0.5 + k)).max(per_pixel * 90.0);
                        flashes.push((at, size, 1.0 - k));
                    }
                }
                if (pass.globe && Some(placed.body) == loaded) || !keeps(placed.pos) {
                    continue;
                }
                let held = state.snapshot.resource_body(body.id);
                let (colour, emissive) = match body.kind {
                    BodyViewKind::Star if celestial => ([4, 3, 3], 1.0),
                    BodyViewKind::Star => (star_colour, 1.0),
                    BodyViewKind::Habitable => (planet_colour(body.id), 0.0),
                    BodyViewKind::Celestial => (CELESTIAL_RGB, 0.35),
                    BodyViewKind::Resource => {
                        // A resource body wears its resource's colour over the grey of the rock.
                        let rock = rock_colour(body.id);
                        let rgb = held.map_or(rock, |r| {
                            [0, 1, 2].map(|i| {
                                ((u16::from(rock[i]) + 3 * u16::from(r.colour[i])) / 4) as u8
                            })
                        });
                        (rgb, 0.0)
                    }
                };
                let is_home =
                    Some(placed.body) == space.home_body && space.planet_texture.is_some();
                let (mesh, material, tint) = if is_home {
                    (&space.lod, &space.lod_material, [1.0, 1.0, 1.0])
                } else {
                    (&space.sphere, &space.material, colour3(colour))
                };
                // The loaded planet turns with the day, so its day side faces the star at the
                // tick the globe shows.
                let mut axes = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
                if Some(placed.body) == loaded {
                    axes = axes.map(|axis| zoom.planet_to_system(axis));
                }
                space.shader.set_shader_value(tint_at, tint);
                space.shader.set_shader_value(emissive_at, emissive);
                space.shader.set_shader_value(alpha_at, pass.detail);
                let per_pixel = view.per_pixel(placed.pos, height);
                world.draw_mesh(mesh, material.clone(), ball(placed.pos, radius, axes));
                // Never smaller than a few pixels, and a soft halo below that.
                if body.kind != BodyViewKind::Star && radius < per_pixel * HALO_BELOW_PIXELS {
                    let faint = 1.0 - radius / (per_pixel * HALO_BELOW_PIXELS);
                    small_halos.push((
                        placed.pos,
                        per_pixel * HALO_PIXELS,
                        colour,
                        (((120.0 * faint) + 30.0) * pass.detail) as u8,
                    ));
                }
                if matches!(body.kind, BodyViewKind::Habitable | BodyViewKind::Celestial) {
                    halos.push((placed.pos, radius));
                }
                // Each ascended player's residence: a small dome in its colour on the planet.
                if body.kind == BodyViewKind::Celestial {
                    let homes = &state.snapshot.residences;
                    let toward = unit_vector(sub3(view.eye, placed.pos));
                    for (k, home) in homes.iter().enumerate() {
                        let at = residence_point(placed.pos, radius, toward, k, homes.len());
                        let [r, g, b] = owner_rgb(home.owner);
                        space.shader.set_shader_value(tint_at, colour3([r, g, b]));
                        space.shader.set_shader_value(emissive_at, 0.55_f32);
                        world.draw_mesh(
                            &space.sphere,
                            space.material.clone(),
                            ball(
                                at,
                                radius * 0.08,
                                [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                            ),
                        );
                        residence_labels.push((at, home.name.clone(), [r, g, b]));
                    }
                }
                if let Some(r) = held {
                    icons.push((placed.pos, radius, r.icon.clone(), per_pixel));
                }
            }
            if !small_halos.is_empty() {
                begin_sprite_run();
                for (pos, size, colour, alpha) in small_halos {
                    sprite_in_run(&mut world, camera, &space.glow, pos, size, colour, alpha);
                }
                end_sprite_run();
            }
            // Every habitable planet wears an atmosphere halo, lit from the star at the origin.
            if known && pass.detail > 0.5 {
                for (pos, radius) in halos {
                    let toward = unit_vector(pos.map(|c| -c));
                    let thickness = super::sky::SYSTEM_HALO_THICKNESS;
                    self.sky
                        .set_halo(pos, radius, thickness, view.eye, toward, pass.detail);
                    self.sky.draw_halo(&mut world, pos, radius, thickness);
                }
            }
            for (at, size, strength) in flashes {
                for (scale, colour, alpha) in
                    [(1.0, [255, 160, 70], 220.0), (0.45, [255, 245, 220], 255.0)]
                {
                    sprite(
                        &mut world,
                        camera,
                        &space.glow,
                        at,
                        size * scale,
                        colour,
                        (strength * alpha * pass.detail) as u8,
                    );
                }
            }
            let system_id = universe.systems.get(pass.index).map(|s| s.id);
            if let Some(system_id) = system_id {
                let outermost = universe.system_outermost(system_id);
                let distance = view.distance();
                let unit = plane_unit(distance);
                let rings = claim_rings(&universe, &bodies_placed, &state.snapshot.claims);
                let thickness = (distance * 0.004).max(5.0);
                let mut overlay = OverlayGeometry::default();
                // The accretion disc: hot and white at its inner edge, orange, then a dim red
                // rim, in the plane of the system about the black hole.
                if celestial
                    && let Some(hole) = bodies_placed
                        .iter()
                        .zip(&drawn)
                        .find(|(p, _)| universe.bodies[p.body].kind == BodyViewKind::Star)
                {
                    let core = *hole.1;
                    let steps = 18;
                    for step in 0..steps {
                        let t = step as f32 / steps as f32;
                        let width = core * 2.6 / steps as f32;
                        let radius = core * 1.35 + core * 2.6 * t + width * 0.5;
                        let (a, b, u) = if t < 0.3 {
                            (ACCRETION_RGB[0], ACCRETION_RGB[1], t / 0.3)
                        } else {
                            (ACCRETION_RGB[1], ACCRETION_RGB[2], (t - 0.3) / 0.7)
                        };
                        let rgb = [0, 1, 2].map(|i| {
                            (f32::from(a[i]) + (f32::from(b[i]) - f32::from(a[i])) * u) as u8
                        });
                        let alpha = 245.0 * (1.0 - t * t) * pass.detail;
                        overlay.tris.extend(annulus(
                            [0.0, 0.2, 0.0],
                            radius,
                            width * 1.05,
                            [rgb[0], rgb[1], rgb[2], alpha as u8],
                            120,
                        ));
                    }
                }
                for ring in &rings {
                    let [r, g, b] = ring.colour;
                    overlay.tris.extend(annulus(
                        [ring.centre[0], 0.4, ring.centre[2]],
                        ring.radius * grown(ring.body),
                        thickness,
                        [r, g, b, (235.0 * detail) as u8],
                        96,
                    ));
                }
                // Colonies: a ring in the owner's colour around each planet with its cities,
                // as much wider than true as the planet is drawn.
                if known {
                    for ring in colony_rings(
                        &universe,
                        &bodies_placed,
                        &state.snapshot.colonies,
                        system_id,
                    ) {
                        let [r, g, b] = ring.colour;
                        overlay.tris.extend(annulus(
                            [ring.centre[0], 0.4, ring.centre[2]],
                            ring.radius * grown(ring.body),
                            thickness * 0.8,
                            [r, g, b, (240.0 * detail) as u8],
                            96,
                        ));
                    }
                }
                // Capitals (task 131): a thin ring in the owner's colour round the planet,
                // inside its colony rings.
                if known {
                    for ring in capital_rings(
                        &universe,
                        &bodies_placed,
                        &state.snapshot.capitals,
                        system_id,
                    ) {
                        let [r, g, b] = ring.colour;
                        overlay.tris.extend(annulus(
                            [ring.centre[0], 0.4, ring.centre[2]],
                            ring.radius * grown(ring.body),
                            thickness * 0.4,
                            [r, g, b, (240.0 * detail) as u8],
                            96,
                        ));
                    }
                }
                // A system a jump inhibitor holds: a faint boundary at its rim.
                if let Some(hold) = state.snapshot.hold_of(system_id) {
                    let [r, g, b] = if hold.enemy {
                        HOLD_ENEMY
                    } else {
                        owner_rgb(hold.owner)
                    };
                    for (radius, alpha) in [(1.06, 120.0), (1.09, 60.0)] {
                        overlay.tris.extend(annulus(
                            [0.0, 0.3, 0.0],
                            SYSTEM_RADIUS * radius,
                            thickness * 1.6,
                            [r, g, b, (alpha * detail) as u8],
                            180,
                        ));
                    }
                }
                let mut boxes: Vec<[f32; 16]> = Vec::with_capacity(state.snapshot.ships.len() * 8);
                let mut model_ships: Vec<(u16, bool, Matrix)> = Vec::new();
                let models = self.models.as_ref();
                let loaded_id = loaded.map(|b| universe.bodies[b].id);
                let per_tile = zoom.scale().system_per_tile();
                // The eye's own axes: markers face it.
                let eye_toward = unit_vector(sub3(view.eye, view.target));
                let eye_right = unit_vector(cross3(view.up, eye_toward));
                let eye_up = cross3(eye_toward, eye_right);
                // Ships far enough out to be class markers, for the counts of overlapping ones.
                // Ships held at their floor are counted too, while they overlap (task 105).
                let mut marked: Vec<(u32, u8, [f32; 3], f32)> = Vec::new();
                for ship in state
                    .snapshot
                    .ships
                    .iter()
                    .filter(|s| s.system == system_id)
                {
                    let scale = state.scales.of(ship.kind);
                    let class = state.scales.class(ship.kind);
                    let at = ship_frame_pos(&universe, &bodies_placed, outermost, ship);
                    let theta = ship.facing_deg.to_radians();
                    let heading = [theta.cos(), 0.0, theta.sin()];
                    // A ship whose model was uploaded is drawn as it; any other keeps the hull.
                    let model = ship
                        .model
                        .filter(|m| models.is_some_and(|g| g.has(m.index)));
                    // True size near, its floor far, its class marker farther still (task 96).
                    // The floor holds for the length it shows the eye, nose on or side on.
                    let per_pixel = view.per_pixel(at, height).max(1e-9);
                    let true_length = ship_true_length(ship.model, scale, per_tile);
                    let share =
                        apparent_share(ship.model, heading, unit_vector(sub3(view.eye, at)));
                    let screen = ship_on_screen(true_length * share, per_pixel, class);
                    let lift = unit * 0.4;
                    let plane = Hull {
                        origin: [at[0], at[1] + lift, at[2]],
                        forward: heading,
                        up: [0.0, 1.0, 0.0],
                        length: screen.model / share,
                        height: 0.0,
                    };
                    // A ship in orbit of the loaded globe leaves its place above the ground
                    // for its place on the plane as the camera leaves the planet.
                    let above = state
                        .orbital
                        .iter()
                        .find(|o| o.id == ship.id)
                        .filter(|_| ship.body.is_some() && ship.body == loaded_id)
                        .zip(pass.place);
                    let hull = match above {
                        Some(_) if pass.space <= 0.0 => continue,
                        // The globe draws a model and a hull alike at its planet scale and its
                        // hover height: the morph starts from what the globe drew.
                        Some((o, place)) => morph_hull(&o.hull, &place, &plane, pass.space),
                        None => plane,
                    };
                    let hull = Hull {
                        length: hull.length * pass.detail,
                        height: Hull::height_of(hull.length * pass.detail),
                        ..hull
                    };
                    if !keeps(hull.origin) {
                        continue;
                    }
                    let instance = model.filter(|_| hull.length > 0.0).and_then(|m| {
                        let (transform, near) = models?.instance_at(
                            m.index,
                            hull.origin,
                            hull.forward,
                            hull.up,
                            hull.length,
                            owner_color(ship.owner),
                            super::models::phase_of(ship.id),
                            view.eye,
                        )?;
                        Some((m.index, near, transform))
                    });
                    match instance {
                        Some(instance) => model_ships.push(instance),
                        None if hull.length > 0.0 => hull.push(owner_color(ship.owner), &mut boxes),
                        None => {}
                    }
                    let mark = (255.0 * pass.detail * pass.space) as u8;
                    // The class marker: a chevron, a diamond for a capital ship, in the owner's
                    // colour on a white rim, facing the ship's heading; it grows in as the model
                    // shrinks away.
                    if screen.marker > 0.004 {
                        let [r, g, b] = owner_rgb(ship.owner);
                        let size = screen.marker_pixels * per_pixel * (0.6 + 0.4 * screen.marker);
                        let a = (f32::from(mark) * screen.marker) as u8;
                        let lifted = [at[0], at[1] + lift, at[2]];
                        // Turned toward the eye, its nose along the heading as the screen shows
                        // it, so it reads at any tilt of the camera.
                        let along = screen_heading(heading, eye_right, eye_up);
                        let across = cross3(along, eye_toward);
                        let capital = class >= CAPITAL_CLASS;
                        let behind: [f32; 3] =
                            std::array::from_fn(|i| lifted[i] - eye_toward[i] * per_pixel);
                        overlay.tris.extend(class_marker(
                            behind,
                            along,
                            across,
                            size * 0.62,
                            capital,
                            [255, 255, 255, a],
                        ));
                        overlay.tris.extend(class_marker(
                            lifted,
                            along,
                            across,
                            size * 0.5,
                            capital,
                            [r, g, b, a],
                        ));
                    }
                    // A class marker, or a model still at its floor (a little past it for a
                    // ship already counted), is one to count with the overlapping.
                    let held = space.ship_groups.contains_key(&ship.id);
                    let band = if held { FLOOR_BAND * 1.25 } else { FLOOR_BAND };
                    if screen.marker >= 0.5 || screen.pixels <= screen.marker_pixels * band {
                        let size = screen.pixels.max(screen.marker_pixels);
                        marked.push((
                            ship.id,
                            ship.owner,
                            [at[0], at[1] + lift, at[2]],
                            size * (1.0 - screen.marker) + screen.marker_pixels * screen.marker,
                        ));
                    }
                    // Rings and bars follow the ship as drawn.
                    let shown = screen.pixels.max(screen.marker_pixels) * per_pixel;
                    let ring_radius = shown * 0.62 + per_pixel * 3.0;
                    let selected = state.selected.binary_search(&ship.id).is_ok();
                    // A ship charging its jump wears a ring that fills.
                    if let Some(fraction) = state.snapshot.charge_of(ship.id) {
                        let [r, g, b] = CHARGE_RING;
                        let radius = ring_radius + per_pixel * 6.0;
                        let width = (per_pixel * 4.0).max(thickness * 0.8);
                        overlay.tris.extend(annulus(
                            [at[0], 0.7, at[2]],
                            radius,
                            width,
                            [r, g, b, mark / 4],
                            40,
                        ));
                        overlay.tris.extend(arc(
                            [at[0], 0.8, at[2]],
                            radius,
                            width * 1.5,
                            fraction,
                            [r, g, b, mark],
                            40,
                        ));
                    }
                    if selected {
                        let ring = annulus(
                            [at[0], 0.6, at[2]],
                            ring_radius,
                            (per_pixel * 3.0).max(thickness * 0.7),
                            [63, 208, 255, mark],
                            28,
                        );
                        overlay.tris.extend(ring);
                    }
                    if selected || ship.hp_frac < 0.999 {
                        let anchor = BarAnchor {
                            id: ship.id,
                            at: [
                                hull.origin[0],
                                hull.origin[1] + shown * 0.3 + per_pixel * 10.0,
                                hull.origin[2],
                            ],
                            width: shown.max(per_pixel * 30.0),
                            frac: ship.hp_frac.clamp(0.0, 1.0),
                            ground_radius: None,
                        };
                        // A planet between the eye and the ship hides its bar, as it hides it.
                        let hidden = bodies_placed.iter().any(|b| {
                            crate::bars::behind_sphere(view.eye, anchor.at, b.pos, b.radius * 0.98)
                        });
                        if !hidden {
                            let first = ship_bars.len();
                            crate::bars::project(
                                &[anchor],
                                &|_| true,
                                &bar_look,
                                self.bar_scale,
                                &mut ship_bars,
                            );
                            for bar in &mut ship_bars[first..] {
                                bar.alpha = mark;
                            }
                        }
                    }
                }
                // Ships that overlap on screen are one group: it shows how many it holds, beside
                // the group so the label covers none of them.
                let mut next_groups = std::collections::BTreeMap::new();
                if !marked.is_empty() {
                    let screen: Vec<(u32, u8, f32, f32, f32)> = marked
                        .iter()
                        .filter_map(|&(id, owner, at, px)| {
                            let (x, y, _) = bar_look.project(at)?;
                            Some((id, owner, x, y, px * 0.5))
                        })
                        .collect();
                    for group in sticky_groups(&screen, &space.ship_groups) {
                        let leader = screen[group[0]].0;
                        if group.len() > 1 {
                            let right = group
                                .iter()
                                .map(|&i| screen[i].2 + screen[i].4)
                                .fold(f32::MIN, f32::max);
                            let top = group
                                .iter()
                                .map(|&i| screen[i].3 - screen[i].4)
                                .fold(f32::MAX, f32::min);
                            group_counts.push((
                                right + 4.0,
                                top - 6.0,
                                group.len(),
                                screen[group[0]].1,
                            ));
                            for &i in &group {
                                next_groups.insert(screen[i].0, leader);
                            }
                        }
                    }
                }
                // Only this system's ships change their entry; the others keep theirs.
                for ship in state
                    .snapshot
                    .ships
                    .iter()
                    .filter(|s| s.system == system_id)
                {
                    space.ship_groups.remove(&ship.id);
                }
                space.ship_groups.extend(next_groups);
                if !boxes.is_empty() {
                    let matrices: Vec<Matrix> = boxes.iter().map(matrix_of).collect();
                    world.draw_instances(&self.box_mesh, self.entity_material.clone(), &matrices);
                }
                if let (Some(models), false) = (&self.models, model_ships.is_empty()) {
                    // Lit by the star from its direction; the loaded planet shades what lies
                    // in its shadow, as its night side does on the globe.
                    let shade = bodies_placed
                        .iter()
                        .find(|b| Some(b.body) == loaded)
                        .map_or([0.0; 4], |b| [b.pos[0], b.pos[1], b.pos[2], b.radius]);
                    let shader = &mut self.model_shader;
                    let at = shader.get_shader_location("starLight");
                    shader.set_shader_value(at, 1_i32);
                    let at = shader.get_shader_location("starShade");
                    shader.set_shader_value(at, shade);
                    let at = shader.get_shader_location("starFill");
                    shader.set_shader_value(at, STAR_FILL * pass.space);
                    let at = shader.get_shader_location("time");
                    shader.set_shader_value(at, time);
                    super::materials::bind_noise(self.noise.as_ref().map(|t| t.id));
                    let mut scratch = Vec::new();
                    models.draw_instances(
                        &mut world,
                        shader,
                        &self.model_material,
                        &mut model_ships,
                        &mut scratch,
                    );
                    super::materials::bind_noise(None);
                    let at = shader.get_shader_location("starLight");
                    shader.set_shader_value(at, 0_i32);
                }
                // The body under the pointer wears a ring.
                if let Some(SpaceHover::Target(Target::Body(index))) = state.hover
                    && let Some(i) = bodies_placed.iter().position(|p| p.body == index)
                {
                    let at = &bodies_placed[i];
                    overlay.tris.extend(annulus(
                        [at.pos[0], 0.5, at.pos[2]],
                        drawn[i] * 1.35 + thickness * 2.0,
                        thickness * 1.2,
                        [255, 230, 160, (230.0 * pass.detail) as u8],
                        64,
                    ));
                }
                // A jump inhibitor's blow on a ship that jumped in flashes where it arrived, as a
                // shell's impact does (task 95), larger where the blow destroyed it.
                for strike in state
                    .snapshot
                    .strikes
                    .iter()
                    .filter(|s| s.system == system_id)
                {
                    let k = strike.age_ticks as f32 / crate::space::STRIKE_FLASH_TICKS as f32;
                    if k >= 1.0 {
                        continue;
                    }
                    let at = free_frame_pos(strike.x, strike.y, outermost);
                    let at = [at[0], at[1] + unit * 0.4, at[2]];
                    if !keeps(at) {
                        continue;
                    }
                    let pixels = if strike.destroyed { 150.0 } else { 100.0 };
                    let size = view.per_pixel(at, height) * pixels * (0.5 + k);
                    strike_flashes.push((at, size, (1.0 - k) * pass.detail * pass.space));
                }
                // Alerts (task 118): a pulsing ring at the planet attacked or where foreign
                // ships jumped in.
                let pulse = crate::space::alert_pulse();
                for alert in state
                    .snapshot
                    .alerts
                    .iter()
                    .filter(|a| a.system == system_id)
                {
                    let at = match alert.body.and_then(|b| universe.body_index(b)) {
                        Some(body) => bodies_placed.iter().find(|p| p.body == body).map(|p| p.pos),
                        None => Some(free_frame_pos(alert.x, alert.y, outermost)),
                    };
                    let Some(at) = at else { continue };
                    let at = [at[0], at[1] + unit * 0.5, at[2]];
                    if !keeps(at) {
                        continue;
                    }
                    let per_pixel = view.per_pixel(at, height);
                    let [r, g, b, _] = alert.kind.colour();
                    let strength = alert.strength() * detail;
                    let alpha = |a: f32| (a * strength).clamp(0.0, 255.0) as u8;
                    overlay.tris.extend(annulus(
                        at,
                        per_pixel * (18.0 + 30.0 * pulse),
                        per_pixel * 2.5,
                        [r, g, b, alpha(255.0 * (1.0 - pulse))],
                        48,
                    ));
                    overlay.tris.extend(annulus(
                        at,
                        per_pixel * 22.0,
                        per_pixel * 2.0,
                        [r, g, b, alpha(220.0)],
                        48,
                    ));
                }
                // Ships between systems wait at the rim, toward the other end of their lane.
                let size = (distance * 0.03).max(60.0);
                for transit in &state.snapshot.transits {
                    let Some((at, dir)) =
                        transit_at_rim(&universe, &space.map_positions, pass.index, transit)
                    else {
                        continue;
                    };
                    let [r, g, b] = owner_rgb(transit.owner);
                    let a = (255.0 * detail) as u8;
                    let lifted = [at[0], 2.0, at[2]];
                    overlay
                        .tris
                        .extend(chevron(lifted, dir, size * 1.3, [255, 255, 255, a]));
                    overlay
                        .tris
                        .extend(chevron([at[0], 3.0, at[2]], dir, size, [r, g, b, a]));
                    if state.selected.binary_search(&transit.id).is_ok() {
                        overlay.tris.extend(annulus(
                            lifted,
                            size * 1.2,
                            size * 0.15,
                            [63, 208, 255, a],
                            28,
                        ));
                    }
                    rim_labels.push((lifted, transit.from, transit.to, transit.progress));
                }
                overlay
                    .tris
                    .retain(|t| t.colour[3] > 0 && keeps(centroid(t)));
                super::draw_overlay(&mut world, &overlay);
            }
            // The strikes' flashes: an orange burst and a white core, additive.
            if !strike_flashes.is_empty() {
                begin_sprite_run();
                for &(at, size, strength) in &strike_flashes {
                    for (scale, colour, alpha) in
                        [(1.0, [255, 180, 80], 230.0), (0.4, [255, 245, 210], 255.0)]
                    {
                        sprite_in_run(
                            &mut world,
                            camera,
                            &space.glow,
                            at,
                            size * scale,
                            colour,
                            (strength * alpha).clamp(0.0, 255.0) as u8,
                        );
                    }
                }
                end_sprite_run();
            }
            // The glow of the star: additive, without writing depth; never wider on screen than
            // a soft disc, so seen from a planet it is a sun, not a sheet of light.
            if let Some(star) = bodies_placed
                .iter()
                .find(|p| universe.bodies[p.body].kind == BodyViewKind::Star)
                .filter(|star| keeps(star.pos))
            {
                let per_pixel = view.per_pixel(star.pos, height);
                let glow = STAR_GLOW.min(per_pixel * STAR_GLOW_PIXELS);
                let a = |v: f32| (v * pass.detail) as u8;
                // A black hole gives no light of its own: the glow is its disc's, wide and
                // orange, with no white core.
                let (glow, core) = if celestial {
                    (glow * 1.6, 0.0)
                } else {
                    (glow, 200.0)
                };
                begin_sprite_run();
                sprite_in_run(
                    &mut world,
                    camera,
                    &space.glow,
                    star.pos,
                    glow,
                    star_colour,
                    a(150.0),
                );
                sprite_in_run(
                    &mut world,
                    camera,
                    &space.glow,
                    star.pos,
                    glow * 0.4,
                    [255, 255, 255],
                    a(core),
                );
                end_sprite_run();
            }
        }
        set_clip_planes(DEFAULT_NEAR, DEFAULT_FAR);
        drop(drawing);
        self.screen_bars.extend(ship_bars);
        for (x, y, count, owner) in group_counts {
            let lighter = lerp_colour(owner_rgb(owner), 0.75).map(|c| (c * 255.0) as u8);
            space.labels.push(SpaceLabel {
                icon: None,
                text: format!("\u{00d7}{count}"),
                x,
                y: y + 5.0,
                colour: [
                    lighter[0],
                    lighter[1],
                    lighter[2],
                    (245.0 * pass.detail) as u8,
                ],
                priority: -2.5,
            });
        }
        if !part.first() {
            return;
        }
        let look = view.view((width, height), self.projection_centre_y);
        let here = universe.systems.get(pass.index).map(|s| s.id);
        // The emblem of each capital on a planet, side by side above it.
        if detail > 0.02 {
            let size = crate::capitals::SPACE_EMBLEM * self.emblem_scale;
            let gap = crate::capitals::EMBLEM_GAP * self.emblem_scale;
            for (at, radius, owners) in capital_spots {
                let Some((x, y, _)) = look.project(at) else {
                    continue;
                };
                let above = y - radius - 4.0 * self.emblem_scale - size / 2.0;
                self.screen_emblems.extend(crate::capitals::row(
                    &owners,
                    (x, above),
                    size,
                    gap,
                    detail,
                ));
            }
        }
        // The icon of a resource body's resource stands beside it at every zoom of the system.
        for (at, radius, icon, per_pixel) in icons {
            let Some((x, y, _)) = look.project(at) else {
                continue;
            };
            let beside = radius / per_pixel.max(1e-6) + 5.0;
            space.labels.push(SpaceLabel {
                text: String::new(),
                icon: Some(icon),
                x: x + beside,
                y: y + 4.0,
                colour: [255, 255, 255, (255.0 * detail) as u8],
                priority: -1.0,
            });
        }
        for (at, name, [r, g, b]) in residence_labels {
            let Some((x, y, _)) = look.project(at) else {
                continue;
            };
            let lighter = lerp_colour([r, g, b], 0.45).map(|c| (c * 255.0) as u8);
            space.labels.push(SpaceLabel {
                icon: None,
                text: name,
                x: x + 10.0,
                y: y - 6.0,
                colour: [lighter[0], lighter[1], lighter[2], (240.0 * detail) as u8],
                priority: -3.0,
            });
        }
        for (at, from, to, progress) in rim_labels {
            let Some((x, y, _)) = look.project(at) else {
                continue;
            };
            let name = |id: u32| {
                universe
                    .systems
                    .iter()
                    .find(|s| s.id == id)
                    .map_or("?", |s| s.name.as_str())
            };
            let text = if Some(from) == here {
                format!("in transit to {} {:.0}%", name(to), progress * 100.0)
            } else {
                format!("in transit from {} {:.0}%", name(from), progress * 100.0)
            };
            space.labels.push(SpaceLabel {
                icon: None,
                text,
                x: x + 14.0,
                y: y + 5.0,
                colour: [200, 225, 255, (235.0 * detail) as u8],
                priority: -2.0,
            });
        }
    }

    /// Draws the star map through the one camera: from inside a system the other systems are a
    /// faint field of stars in their true directions (`stars`: how much the sky lets them
    /// through); out on the map they grow into halos with routes, names and ship marks, and the
    /// anchored system's star swells into its halo as its own detail fades.
    #[allow(clippy::too_many_arguments)]
    fn draw_map_layer(
        &mut self,
        rl: &mut RaylibHandle,
        thread: &RaylibThread,
        target: &mut RenderTexture2D,
        zoom: &ZoomCamera,
        orbit: &OrbitCamera,
        one: &OneView,
        stars: f32,
    ) {
        let fades = one.fades;
        let Some(space) = &mut self.space else {
            return;
        };
        let view = one.map;
        let camera = Camera3D::perspective(vec3(view.eye), vec3(view.target), vec3(view.up), 45.0);
        let universe = Arc::clone(&space.universe);
        let (width, height) = (target.texture.width as f32, target.texture.height as f32);
        let detail = fades.map;
        // Marks keep their size on screen.
        let marks_view = zoom.map_view(orbit);
        let (icon, beside) = map_mark_scale(&marks_view, view.distance());
        let marks = if detail > 0.004 {
            map_ship_marks(
                &universe,
                &space.map_positions,
                &self.space_state.snapshot,
                beside,
            )
        } else {
            Vec::new()
        };
        let mut drawing = rl.begin_texture_mode(thread, target);
        let far = view.distance_to([0.0; 3]) + MAP_RADIUS * 3.0;
        let near = (view.distance() * 0.01).clamp(1e-4, far * 1e-3);
        set_clip_planes(f64::from(near), f64::from(far));
        {
            let mut world = drawing.begin_mode3D(camera);
            shift_projection(self.projection_centre_y);
            // SAFETY: the render thread has an active GL context; the map's marks, lines and
            // glows are drawn in order over the sky rather than depth-tested.
            unsafe {
                raylib::ffi::rlDrawRenderBatchActive();
                raylib::ffi::rlDisableDepthTest();
            }
            let from_sun = zoom.sky_active();
            let depth = |p: [f32; 3]| map_depth(&view, p, from_sun);
            let mut overlay = OverlayGeometry::default();
            // The camera's own axes: the map's marks face it, as there is no plane to lay them in.
            let ahead = unit_vector(sub3(view.target, view.eye));
            let screen_right = unit_vector(cross3(ahead, view.up));
            let screen_up = cross3(screen_right, ahead);
            let face = |tris: Vec<crate::overlay::Tri3>, at: [f32; 3]| {
                facing(tris, at, screen_right, screen_up)
            };
            if detail > 0.004 {
                // The figures first, dashed, then the routes over them: the only solid lines
                // between systems are routes.
                for line in star_map_lines(
                    &universe,
                    &space.map_positions,
                    &self.space_state.snapshot,
                    zoom.constellations(),
                    from_sun,
                    // From the Sun the figures lie about the middle of the map, not at a focus.
                    FIGURE_DASH
                        * if from_sun {
                            view.distance_to([0.0; 3])
                        } else {
                            view.distance()
                        },
                ) {
                    let colour = line.kind.colour(from_sun);
                    let shown = detail
                        * match line.kind {
                            MapLineKind::Figure => depth(line.at),
                            MapLineKind::Route | MapLineKind::Stub => 0.35 + 0.65 * depth(line.at),
                        };
                    let colour = [
                        colour[0],
                        colour[1],
                        colour[2],
                        (f32::from(colour[3]) * shown) as u8,
                    ];
                    world.draw_line3D(
                        vec3(line.a),
                        vec3(line.b),
                        Color::new(colour[0], colour[1], colour[2], colour[3]),
                    );
                }
            }
            let grow = view.distance() * 0.1;
            let k = one.system_place.per_unit;
            // Far systems first, so the near ones are drawn over them.
            let mut order: Vec<usize> = (0..universe.systems.len()).collect();
            order.sort_by(|&a, &b| {
                let distance = |i: usize| view.distance_to(space.map_positions[i]);
                distance(b).total_cmp(&distance(a)).then(a.cmp(&b))
            });
            begin_sprite_run();
            for i in order {
                let system = &universe.systems[i];
                let pos = space.map_positions[i];
                // While a system's own detail shows, its star stands for it; as the detail
                // fades the star's glow swells into the map's halo.
                let own = system_detail(SYSTEM_RADIUS * k / view.distance_to(pos).max(1e-6));
                let swell = 1.0 - own;
                let tint = space.tints[i].unwrap_or(system.star_colour);
                // From the star's own colour to the colour of the race at home there.
                let halo = [0, 1, 2].map(|c| {
                    let (from, to) = (f32::from(system.star_colour[c]), f32::from(tint[c]));
                    (from + (to - from) * swell).round() as u8
                });
                let known = self.space_state.snapshot.knows(system.id);
                // An unexplored system is a faint marker.
                let (halo_size, halo_alpha) = if known || from_sun {
                    (HALO_SIZE.max(grow), 230.0)
                } else {
                    (HALO_SIZE.max(grow) * 0.5, 90.0)
                };
                // Perspective makes far systems smaller; a system the camera flies close by
                // stops growing at a size that still leaves the picture readable.
                let widest = view.per_pixel(pos, height) * MAP_HALO_MAX_PIXELS;
                let halo_size = halo_size.min(space.gap * 0.8).min(widest);
                let core_size = CORE_SIZE
                    .max(grow * 0.3)
                    .min(space.gap * 0.25)
                    .min(widest * 0.3);
                // From inside a system the others are points of light in the sky, as bright
                // as the sky lets them be; out on the map they are halos.
                let point = view.per_pixel(pos, height) * SKY_STAR_PIXELS;
                let sky = (stars * 0.75).clamp(0.0, 1.0);
                let seen = sky + (1.0 - sky) * detail;
                let star = STAR_GLOW * k;
                let (halo_size, core_size, seen) = if from_sun {
                    // From the Sun every star is a point of light of the same size on screen.
                    let pixel = view.per_pixel(pos, height);
                    (pixel * SKY_HALO_PIXELS, pixel * SKY_CORE_PIXELS, 1.0)
                } else {
                    (
                        morph_size(star, morph_size(point * 2.5, halo_size, detail), swell),
                        morph_size(star * 0.4, morph_size(point, core_size, detail), swell),
                        seen,
                    )
                };
                let near = depth(pos);
                let halo_alpha = halo_alpha * seen * swell * near;
                let core_alpha = 255.0 * seen * swell * (0.5 + 0.5 * near);
                if halo_alpha < 1.0 {
                    continue;
                }
                sprite_in_run(
                    &mut world,
                    camera,
                    &space.glow,
                    pos,
                    halo_size,
                    halo,
                    halo_alpha as u8,
                );
                sprite_in_run(
                    &mut world,
                    camera,
                    &space.glow,
                    pos,
                    core_size,
                    [255, 255, 255],
                    core_alpha as u8,
                );
            }
            end_sprite_run();
            // Ships: a mark beside each system that has ships, an arrowhead on the lane for each
            // ship between systems, its lane ahead lit and its destination ringed.
            for mark in &marks {
                let [r, g, b] = owner_rgb(mark.owner);
                let chosen = mark
                    .ids
                    .iter()
                    .any(|id| self.space_state.selected.binary_search(id).is_ok());
                let at = mark.at;
                match (mark.heading, mark.bound_for) {
                    (Some(dir), Some(dest)) => {
                        let goal = space.map_positions[dest];
                        overlay.lines.push(crate::overlay::Line3 {
                            a: at,
                            b: goal,
                            colour: [r, g, b, 255],
                        });
                        overlay.tris.extend(face(
                            annulus(goal, icon * 2.6, icon * 0.35, [r, g, b, 235], 40),
                            goal,
                        ));
                        overlay
                            .tris
                            .extend(chevron3(at, dir, icon * 2.9, [255, 255, 255, 255]));
                        overlay
                            .tris
                            .extend(chevron3(at, dir, icon * 2.2, [r, g, b, 255]));
                    }
                    _ => {
                        overlay
                            .tris
                            .extend(face(diamond(at, icon * 1.3, [255, 255, 255, 255]), at));
                        overlay
                            .tris
                            .extend(face(diamond(at, icon * 0.95, [r, g, b, 255]), at));
                    }
                }
                if chosen {
                    overlay.tris.extend(face(
                        annulus(at, icon * 2.0, icon * 0.3, [63, 208, 255, 255], 28),
                        at,
                    ));
                }
                // Ships charging a jump: a ring that fills as the least charged of them does.
                if let Some(fraction) = mark.charge {
                    let [r, g, b] = CHARGE_RING;
                    overlay.tris.extend(face(
                        annulus(at, icon * 2.6, icon * 0.32, [r, g, b, 70], 32),
                        at,
                    ));
                    overlay.tris.extend(face(
                        arc(at, icon * 2.6, icon * 0.45, fraction, [r, g, b, 255], 32),
                        at,
                    ));
                }
            }
            if detail > 0.004 {
                // Colonies: a ring in each owner's colour around every system with its cities.
                for (index, owner, rank) in
                    map_colony_rings(&universe, &self.space_state.snapshot.colonies)
                {
                    let [r, g, b] = owner_rgb(owner);
                    let at = space.map_positions[index];
                    let radius = icon * (3.3 + 0.6 * f32::from(rank));
                    overlay.tris.extend(face(
                        annulus(at, radius, icon * 0.26, [r, g, b, 230], 40),
                        at,
                    ));
                }
                // Jump inhibitors: a six-sided ring round the system they hold, red for an
                // enemy's, the owner's colour for one's own.
                for hold in &self.space_state.snapshot.holds {
                    let Some(index) = universe.system_index(hold.system) else {
                        continue;
                    };
                    let [r, g, b] = if hold.enemy {
                        HOLD_ENEMY
                    } else {
                        owner_rgb(hold.owner)
                    };
                    let at = space.map_positions[index];
                    overlay.tris.extend(face(
                        annulus(at, icon * 5.6, icon * 0.55, [r, g, b, 235], 6),
                        at,
                    ));
                    overlay.tris.extend(face(
                        annulus(at, icon * 6.3, icon * 0.2, [r, g, b, 150], 6),
                        at,
                    ));
                }
                // Alerts (task 118): a pulsing ring round the system where a city is under
                // attack or foreign ships jumped in.
                let pulse = crate::space::alert_pulse();
                for alert in &self.space_state.snapshot.alerts {
                    let Some(index) = universe.system_index(alert.system) else {
                        continue;
                    };
                    let [r, g, b, _] = alert.kind.colour();
                    let strength = alert.strength();
                    let alpha = |a: f32| (a * strength).clamp(0.0, 255.0) as u8;
                    let at = space.map_positions[index];
                    overlay.tris.extend(face(
                        annulus(
                            at,
                            icon * (4.0 + 5.0 * pulse),
                            icon * 0.45,
                            [r, g, b, alpha(255.0 * (1.0 - pulse))],
                            40,
                        ),
                        at,
                    ));
                    overlay.tris.extend(face(
                        annulus(at, icon * 4.6, icon * 0.35, [r, g, b, alpha(230.0)], 40),
                        at,
                    ));
                }
                // The celestial home: its disc as a tilted orange ring about the point of light.
                for (index, system) in universe.systems.iter().enumerate() {
                    if !system.celestial {
                        continue;
                    }
                    let at = space.map_positions[index];
                    for (radius, width, rgba) in [
                        (icon * 2.6, icon * 0.9, [255, 170, 70, 210]),
                        (icon * 3.6, icon * 0.6, [200, 80, 35, 150]),
                    ] {
                        overlay
                            .tris
                            .extend(face(annulus(at, radius, width, rgba, 48), at));
                    }
                }
                if let Some(SpaceHover::Target(Target::System(index))) = self.space_state.hover
                    && let Some(&at) = space.map_positions.get(index)
                {
                    overlay.tris.extend(face(
                        annulus(at, icon * 4.0, icon * 0.4, [255, 230, 160, 230], 48),
                        at,
                    ));
                }
            }
            fade_overlay(&mut overlay, detail);
            super::draw_overlay(&mut world, &overlay);
            // SAFETY: as above; the batch is flushed before depth testing is restored.
            unsafe {
                raylib::ffi::rlDrawRenderBatchActive();
                raylib::ffi::rlEnableDepthTest();
            }
        }
        set_clip_planes(DEFAULT_NEAR, DEFAULT_FAR);
        drop(drawing);
        if detail <= 0.004 {
            return;
        }
        // Names, nearest the centre of the picture first.
        let centre_y = self.projection_centre_y;
        let look = view.view((width, height), centre_y);
        for (i, system) in universe.systems.iter().enumerate() {
            let Some((x, y, depth)) = look.project(space.map_positions[i]) else {
                continue;
            };
            if x < 0.0 || y < 0.0 || x > width || y > height {
                continue;
            }
            let halo_pixels =
                HALO_SIZE.max(view.distance() * 0.1) / 2.0 / depth * height / (2.0 * look.tan());
            let home = space.tints[i];
            let known = self.space_state.snapshot.knows(system.id);
            // Capitals known there (task 131): their emblems in a row left of the star, clear
            // of its colony rings, its name and its ships on the right.
            let owners =
                crate::capitals::system_owners(&self.space_state.snapshot.capitals, system.id);
            if !owners.is_empty() {
                let scale = self.emblem_scale;
                let per_pixel = view.per_pixel(space.map_positions[i], height).max(1e-9);
                let rings = icon * 3.9 / per_pixel;
                let clear = (halo_pixels * 0.5).max(rings) + 3.0 * scale;
                let opacity = detail * map_depth(&view, space.map_positions[i], zoom.sky_active());
                self.screen_emblems.extend(crate::capitals::row_leftward(
                    &owners,
                    (x - clear, y),
                    crate::capitals::SPACE_EMBLEM * scale,
                    crate::capitals::EMBLEM_GAP * scale,
                    opacity,
                ));
            }
            let rgb = match (home, known) {
                (_, false) => [130, 140, 155],
                (Some(tint), true) => tint,
                (None, true) => [200, 215, 235],
            };
            // An ascension under way there (task 76): everyone sees its countdown.
            for (row, countdown) in self
                .space_state
                .snapshot
                .countdowns
                .iter()
                .filter(|c| c.system == system.id)
                .enumerate()
            {
                let [r, g, b] = owner_rgb(countdown.owner);
                let lighter = lerp_colour([r, g, b], 0.5).map(|c| (c * 255.0) as u8);
                space.labels.push(SpaceLabel {
                    icon: None,
                    text: countdown.text.clone(),
                    x: x + halo_pixels.max(8.0) * 0.5,
                    y: y + halo_pixels.max(8.0) * 0.4 + 17.0 * (row + 1) as f32,
                    colour: [lighter[0], lighter[1], lighter[2], (250.0 * detail) as u8],
                    priority: -4.0,
                });
            }
            space.labels.push(SpaceLabel {
                icon: None,
                text: if known || zoom.sky_active() || system.celestial {
                    system.name.clone()
                } else {
                    format!("{} (unexplored)", system.name)
                },
                x: x + halo_pixels.max(8.0) * 0.5,
                y: y + halo_pixels.max(8.0) * 0.4,
                // Names fade with the distance, as the stars do.
                colour: [
                    rgb[0],
                    rgb[1],
                    rgb[2],
                    (235.0 * detail * map_depth(&view, space.map_positions[i], zoom.sky_active()))
                        as u8,
                ],
                priority: if home.is_some() {
                    -1.0
                } else {
                    (x - width / 2.0).hypot(y - height / 2.0) / width
                },
            });
        }
        // Each constellation once, faintly, over the middle of its member systems.
        let mut members: std::collections::BTreeMap<&str, Vec<[f32; 3]>> = Default::default();
        for (i, system) in universe.systems.iter().enumerate() {
            if !system.constellation.is_empty() {
                members
                    .entry(system.constellation.as_str())
                    .or_default()
                    .push(space.map_positions[i]);
            }
        }
        for (name, at) in members {
            let mid = [0, 1, 2].map(|i| at.iter().map(|p| p[i]).sum::<f32>() / at.len() as f32);
            let Some((x, y, _)) = look.project(mid) else {
                continue;
            };
            if x < 0.0 || y < 0.0 || x > width || y > height {
                continue;
            }
            let text = name.to_uppercase();
            space.labels.push(SpaceLabel {
                x: x - text.len() as f32 * 3.4,
                y: y - 22.0,
                text,
                icon: None,
                colour: [140, 175, 235, (130.0 * detail) as u8],
                priority: 3.0,
            });
        }
        space.labels.push(SpaceLabel {
            icon: None,
            text: if zoom.sky_active() {
                "Sky from the Sun   F2 back   F3 figures   arrows or drag to look".to_owned()
            } else {
                "F2 sky from the Sun   F3 constellation figures   F4 level the view".to_owned()
            },
            x: 16.0,
            y: 112.0,
            colour: [150, 165, 195, (170.0 * detail) as u8],
            priority: -2.0,
        });
        for mark in &marks {
            let Some((x, y, _)) = look.project(mark.at) else {
                continue;
            };
            // Ships travelling together are one mark that counts them.
            let many = if mark.ids.len() > 1 {
                format!("{} ", plural_ships(mark.ids.len()))
            } else {
                String::new()
            };
            let text = match mark.bound_for {
                Some(dest) if mark.charting => {
                    format!("{many}to {} (charting)", universe.systems[dest].name)
                }
                Some(dest) => format!("{many}to {}", universe.systems[dest].name),
                None if mark.charge.is_some() => {
                    format!("{} charging a jump", plural_ships(mark.ids.len()))
                }
                None => plural_ships(mark.ids.len()),
            };
            let [r, g, b] = owner_rgb(mark.owner);
            let lighter = lerp_colour([r, g, b], 0.5).map(|c| (c * 255.0) as u8);
            space.labels.push(SpaceLabel {
                icon: None,
                text,
                x: x + 12.0,
                y: y + 16.0,
                colour: [lighter[0], lighter[1], lighter[2], (235.0 * detail) as u8],
                priority: 0.9,
            });
        }
    }
}

/// Share of a star-map mark's brightness that its depth leaves it, seen through `view`: 1 at
/// the near side of the map, down to `FAR_FADE` at the far side, so near systems read as near
/// (1 everywhere from the Sun).
fn map_depth(view: &FrameView, p: [f32; 3], from_sun: bool) -> f32 {
    let t = ((view.distance_to(p) - view.distance()) / (2.0 * MAP_RADIUS) + 0.5).clamp(0.0, 1.0);
    if from_sun {
        1.0
    } else {
        1.0 - (1.0 - FAR_FADE) * t
    }
}

/// "1 ship" or "N ships".
fn plural_ships(n: usize) -> String {
    if n == 1 {
        "1 ship".to_owned()
    } else {
        format!("{n} ships")
    }
}

/// The unit direction in the eye's plane (axes `right` and `up`) that `heading` points on
/// screen; straight up the screen for a heading along the line of sight.
fn screen_heading(heading: [f32; 3], right: [f32; 3], up: [f32; 3]) -> [f32; 3] {
    let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let (x, y) = (dot(heading, right), dot(heading, up));
    let length = x.hypot(y);
    if length < 1e-4 {
        return up;
    }
    [0, 1, 2].map(|i| (right[i] * x + up[i] * y) / length)
}

fn cross3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn unit_vector(v: [f32; 3]) -> [f32; 3] {
    let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-6);
    v.map(|c| c / length)
}

/// The median distance from a point to its nearest neighbour; the whole map's size for fewer
/// than two points.
fn nearest_gap(points: &[[f32; 3]]) -> f32 {
    if points.len() < 2 {
        return MAP_RADIUS * 2.0;
    }
    let mut nearest: Vec<f32> = points
        .iter()
        .enumerate()
        .map(|(i, p)| {
            points
                .iter()
                .enumerate()
                .filter(|&(j, _)| j != i)
                .map(|(_, q)| (0..3).map(|k| (p[k] - q[k]).powi(2)).sum::<f32>().sqrt())
                .fold(f32::MAX, f32::min)
        })
        .collect();
    nearest.sort_by(f32::total_cmp);
    nearest[nearest.len() / 2]
}

/// Least detail at which another system gets a pass of its own: below it every layer of the
/// pass is barely visible, and the star map's halo stands for the system.
const PASS_DETAIL_MIN: f32 = 0.004;

/// Flushes what is queued and switches to additive blending without depth writes, for a run of
/// [`sprite_in_run`] draws that [`end_sprite_run`] closes: one draw call for the whole run.
fn begin_sprite_run() {
    // SAFETY: the render thread has an active GL context; these calls only flip the depth mask
    // and blend mode around billboard draws.
    unsafe {
        raylib::ffi::rlDrawRenderBatchActive();
        raylib::ffi::rlDisableDepthMask();
        raylib::ffi::BeginBlendMode(BlendMode::BLEND_ADDITIVE as i32);
    }
}

/// Draws the queued sprites and restores blending and the depth mask.
fn end_sprite_run() {
    // SAFETY: as above; the batch is flushed so the sprites are drawn with the settings above.
    unsafe {
        raylib::ffi::rlDrawRenderBatchActive();
        raylib::ffi::EndBlendMode();
        raylib::ffi::rlEnableDepthMask();
    }
}

/// Queues an additive billboard of `size` units at `pos` inside a sprite run.
fn sprite_in_run(
    world: &mut impl RaylibDraw3D,
    camera: Camera3D,
    texture: &Texture2D,
    pos: [f32; 3],
    size: f32,
    colour: [u8; 3],
    alpha: u8,
) {
    // The quad faces the camera with the camera's own up, so it stays round seen from above.
    let side = texture.width as f32;
    world.draw_billboard_pro(
        camera,
        **texture,
        Rectangle::new(0.0, 0.0, side, texture.height as f32),
        vec3(pos),
        camera.up,
        Vector2::new(size, size),
        Vector2::new(size / 2.0, size / 2.0),
        0.0,
        Color::new(colour[0], colour[1], colour[2], alpha),
    );
}

/// An additive billboard of `size` units at `pos`; it does not write depth.
fn sprite(
    world: &mut impl RaylibDraw3D,
    camera: Camera3D,
    texture: &Texture2D,
    pos: [f32; 3],
    size: f32,
    colour: [u8; 3],
    alpha: u8,
) {
    begin_sprite_run();
    sprite_in_run(world, camera, texture, pos, size, colour, alpha);
    end_sprite_run();
}

/// Where the `k`-th of `n` residences stands on the celestial planet centred at `centre` with
/// drawn radius `radius`: on a ring about the point of the planet facing the eye (`toward`, a
/// unit vector), so every residence shows whichever way the planet is seen; one alone stands
/// in the middle.
fn residence_point(
    centre: [f32; 3],
    radius: f32,
    toward: [f32; 3],
    k: usize,
    n: usize,
) -> [f32; 3] {
    let side = if toward[1].abs() < 0.9 {
        [0.0, 1.0, 0.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    let u = unit_vector(cross3(toward, side));
    let v = cross3(toward, u);
    let tilt: f32 = if n > 1 { 0.6 } else { 0.0 };
    let turn = std::f32::consts::TAU * k as f32 / n.max(1) as f32 + 0.4;
    let r = radius * 1.01;
    std::array::from_fn(|i| {
        centre[i]
            + r * (tilt.cos() * toward[i] + tilt.sin() * (turn.cos() * u[i] + turn.sin() * v[i]))
    })
}
