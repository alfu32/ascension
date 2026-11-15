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
    FrameView, OneView, PlanetPlace, ShipScales, SystemPlace, morph_size, plane_unit,
    system_detail, system_ship_length,
};
use crate::space::{
    SpaceHover, SpaceSnapshot, annulus, chevron, claim_rings, colony_rings, diamond,
    map_colony_rings, map_mark_scale, map_ship_marks, ship_frame_pos, transit_at_rim,
};
use crate::universe::{
    BodyViewKind, MAP_RADIUS, PlacedBody, SYSTEM_RADIUS, UniverseScene, planet_colour, ring_points,
    rock_colour, uv_sphere,
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
/// Smallest radius of a body on screen, in pixels, and the radius below which it also wears a
/// soft halo so it can still be seen, hovered and double-clicked.
const MIN_BODY_PIXELS: f32 = 2.5;
const HALO_BELOW_PIXELS: f32 = 6.0;
/// Diameter of a body's halo on screen, in pixels.
const HALO_PIXELS: f32 = 22.0;
const ROUTE_LINE: Color = Color::new(110, 140, 200, 110);
/// Widest the star's glow is drawn on screen, in pixels.
const STAR_GLOW_PIXELS: f32 = 140.0;
/// Radius in pixels, seen from the ground, of the star and of the other bodies in the sky.
const SKY_STAR_DISC: f32 = 7.0;
const SKY_PLANET_DISC: f32 = 3.5;
const SKY_ROCK_DISC: f32 = 2.5;
/// Diameter in pixels of another system's point of light seen from inside a system.
const SKY_STAR_PIXELS: f32 = 3.0;

/// What is in space and which ships are selected, kept for the next draw.
#[derive(Default)]
pub(super) struct SpaceState {
    snapshot: Arc<SpaceSnapshot>,
    selected: Vec<u32>,
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

/// The radius a body of true radius `radius` is drawn with, `per_pixel` units a pixel away:
/// seen from the ground (`space` = 0) a small disc in the sky, growing to its true size as the
/// camera leaves the planet, and never under a few pixels.
fn shown_radius(kind: BodyViewKind, radius: f32, per_pixel: f32, space: f32) -> f32 {
    let sky = match kind {
        BodyViewKind::Star => SKY_STAR_DISC,
        BodyViewKind::Habitable => SKY_PLANET_DISC,
        BodyViewKind::Resource => SKY_ROCK_DISC,
    };
    let per_pixel = per_pixel.max(1e-9);
    let pixels = radius / per_pixel;
    let shown = if pixels > sky {
        morph_size(sky, pixels, space)
    } else {
        pixels
    };
    shown.max(MIN_BODY_PIXELS) * per_pixel
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
    /// The bodies of every system placed in its frame, indexed like the systems.
    placed: Vec<Vec<PlacedBody>>,
    tints: Vec<Option<[u8; 3]>>,
    shader: Shader,
    material: WeakMaterial,
    lod_material: WeakMaterial,
    sphere: Mesh,
    lod: Mesh,
    glow: Texture2D,
    planet_texture: Option<Texture2D>,
    labels: Vec<SpaceLabel>,
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
            placed: universe
                .systems
                .iter()
                .map(|s| universe.place_system(s.id))
                .collect(),
            tints,
            shader,
            material,
            lod_material,
            sphere: upload_sphere(thread, 20, 40),
            lod: upload_sphere(thread, 12, 24),
            glow,
            planet_texture,
            labels: Vec::new(),
        });
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
        let height = target.texture.height as f32;
        // The loaded globe is drawn as itself while it is more than a few pixels wide; the
        // system draws it as a small sphere below that.
        let globe = match (&one.planet, &one.place) {
            (Some(view), Some(place)) if self.globe.is_some() => {
                let pixels = view.sphere_pixels([0.0; 3], orbit.radius, height);
                (pixels >= MIN_BODY_PIXELS).then(|| {
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
            self.space_backdrop(rl, thread, target, &one.system);
        }
        let stars = frame.as_ref().map_or(1.0, |f| f.stars);
        self.draw_map_layer(rl, thread, target, zoom, orbit, &one, stars);
        let drawn = globe.is_some() && frame.is_some();
        let mut passes = self.system_passes(zoom, &one, drawn);
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
    fn system_passes(&self, zoom: &ZoomCamera, one: &OneView, globe: bool) -> Vec<SystemPass> {
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
            if detail <= 0.004 {
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
    ) {
        self.set_star_fade(1.0);
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
        let tint_at = space.shader.get_shader_location("tint");
        let emissive_at = space.shader.get_shader_location("emissive");
        let alpha_at = space.shader.get_shader_location("alpha");
        let (width, height) = (target.texture.width as f32, target.texture.height as f32);
        let mut rim_labels: Vec<([f32; 3], u32, u32, f32)> = Vec::new();
        let mut icons: Vec<([f32; 3], f32, String, f32)> = Vec::new();
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
                        (BodyViewKind::Habitable, _) => {
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
            let mut halos: Vec<([f32; 3], f32)> = Vec::new();
            for placed in &bodies_placed {
                let body = &universe.bodies[placed.body];
                if body.kind != BodyViewKind::Star && !known {
                    continue;
                }
                if (pass.globe && Some(placed.body) == loaded) || !keeps(placed.pos) {
                    continue;
                }
                let held = state.snapshot.resource_body(body.id);
                let (colour, emissive) = match body.kind {
                    BodyViewKind::Star => (star_colour, 1.0),
                    BodyViewKind::Habitable => (planet_colour(body.id), 0.0),
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
                let radius = shown_radius(body.kind, placed.radius, per_pixel, pass.space);
                world.draw_mesh(mesh, material.clone(), ball(placed.pos, radius, axes));
                // Never smaller than a few pixels, and a soft halo below that.
                if body.kind != BodyViewKind::Star && radius < per_pixel * HALO_BELOW_PIXELS {
                    let faint = 1.0 - radius / (per_pixel * HALO_BELOW_PIXELS);
                    sprite(
                        &mut world,
                        camera,
                        &space.glow,
                        placed.pos,
                        per_pixel * HALO_PIXELS,
                        colour,
                        (((120.0 * faint) + 30.0) * pass.detail) as u8,
                    );
                }
                if body.kind == BodyViewKind::Habitable {
                    halos.push((placed.pos, radius));
                }
                if let Some(r) = held {
                    icons.push((placed.pos, radius, r.icon.clone(), per_pixel));
                }
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
            let system_id = universe.systems.get(pass.index).map(|s| s.id);
            if let Some(system_id) = system_id {
                let outermost = universe.system_outermost(system_id);
                let distance = view.distance();
                let unit = plane_unit(distance);
                let rings = claim_rings(&universe, &bodies_placed, &state.snapshot.claims);
                let thickness = (distance * 0.004).max(5.0);
                let mut overlay = OverlayGeometry::default();
                for ring in &rings {
                    let [r, g, b] = ring.colour;
                    overlay.tris.extend(annulus(
                        [ring.centre[0], 0.4, ring.centre[2]],
                        ring.radius,
                        thickness,
                        [r, g, b, (235.0 * detail) as u8],
                        96,
                    ));
                }
                // Colonies: a ring in the owner's colour around each planet with its cities,
                // never smaller on screen than a few pixels beyond the planet's least size.
                if known {
                    for ring in colony_rings(
                        &universe,
                        &bodies_placed,
                        &state.snapshot.colonies,
                        system_id,
                    ) {
                        let [r, g, b] = ring.colour;
                        let least = view.per_pixel(ring.centre, height) * (MIN_BODY_PIXELS + 6.0);
                        overlay.tris.extend(annulus(
                            [ring.centre[0], 0.4, ring.centre[2]],
                            ring.radius.max(least),
                            thickness * 0.8,
                            [r, g, b, (240.0 * detail) as u8],
                            96,
                        ));
                    }
                }
                let forward = unit_vector(sub3(view.target, view.eye));
                let right = unit_vector(cross3(forward, [0.0, 1.0, 0.0]));
                let cam_up = cross3(right, forward);
                let mut boxes: Vec<[f32; 16]> = Vec::with_capacity(state.snapshot.ships.len() * 8);
                let loaded_id = loaded.map(|b| universe.bodies[b].id);
                for ship in state
                    .snapshot
                    .ships
                    .iter()
                    .filter(|s| s.system == system_id)
                {
                    let scale = state.scales.of(ship.kind);
                    let at = ship_frame_pos(&universe, &bodies_placed, outermost, ship);
                    let theta = ship.facing_deg.to_radians();
                    let plane = Hull {
                        origin: [at[0], at[1] + unit * 0.4, at[2]],
                        forward: [theta.cos(), 0.0, theta.sin()],
                        up: [0.0, 1.0, 0.0],
                        length: system_ship_length(scale, distance),
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
                        Some((o, place)) => morph_hull(&o.hull, &place, &plane, pass.space),
                        None => plane,
                    };
                    let hull = Hull {
                        length: hull.length * pass.detail,
                        height: Hull::height_of(hull.length * pass.detail),
                        ..hull
                    };
                    if !keeps(hull.origin) || hull.length <= 0.0 {
                        continue;
                    }
                    hull.push(owner_color(ship.owner), &mut boxes);
                    let selected = state.selected.binary_search(&ship.id).is_ok();
                    let mark = (255.0 * pass.detail * pass.space) as u8;
                    if selected {
                        let ring = annulus(
                            [at[0], 0.6, at[2]],
                            unit * 2.4 * scale.system.sqrt(),
                            (unit * 0.22).max(thickness * 0.7),
                            [63, 208, 255, mark],
                            28,
                        );
                        overlay.tris.extend(ring);
                    }
                    if selected || ship.hp_frac < 0.999 {
                        let width = unit * 3.0 * scale.system.sqrt();
                        let base = [at[0], at[1] + unit * 2.2, at[2]];
                        let corner = |along: f32, up: f32| {
                            [
                                base[0] + right[0] * along + cam_up[0] * up,
                                base[1] + right[1] * along + cam_up[1] * up,
                                base[2] + right[2] * along + cam_up[2] * up,
                            ]
                        };
                        let h = unit * 0.32;
                        let filled = -width / 2.0 + width * ship.hp_frac.clamp(0.0, 1.0);
                        let quad = |from: f32, to: f32, colour: [u8; 4]| {
                            [
                                crate::overlay::Tri3 {
                                    v: [corner(from, 0.0), corner(to, 0.0), corner(to, h)],
                                    colour,
                                },
                                crate::overlay::Tri3 {
                                    v: [corner(from, 0.0), corner(to, h), corner(from, h)],
                                    colour,
                                },
                            ]
                        };
                        overlay
                            .tris
                            .extend(quad(-width / 2.0, width / 2.0, [30, 30, 30, mark]));
                        overlay
                            .tris
                            .extend(quad(-width / 2.0, filled, [70, 230, 90, mark]));
                    }
                }
                if !boxes.is_empty() {
                    let matrices: Vec<Matrix> = boxes.iter().map(matrix_of).collect();
                    world.draw_mesh_instanced(
                        &self.box_mesh,
                        self.entity_material.clone(),
                        &matrices,
                    );
                }
                // The body under the pointer wears a ring.
                if let Some(SpaceHover::Target(Target::Body(index))) = state.hover
                    && let Some(at) = bodies_placed.iter().find(|p| p.body == index)
                {
                    overlay.tris.extend(annulus(
                        [at.pos[0], 0.5, at.pos[2]],
                        at.radius * 1.35 + thickness * 2.0,
                        thickness * 1.2,
                        [255, 230, 160, (230.0 * pass.detail) as u8],
                        64,
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
                sprite(
                    &mut world,
                    camera,
                    &space.glow,
                    star.pos,
                    glow,
                    star_colour,
                    a(150.0),
                );
                sprite(
                    &mut world,
                    camera,
                    &space.glow,
                    star.pos,
                    glow * 0.4,
                    [255, 255, 255],
                    a(200.0),
                );
            }
        }
        set_clip_planes(DEFAULT_NEAR, DEFAULT_FAR);
        drop(drawing);
        if !part.first() {
            return;
        }
        let look = view.view((width, height), self.projection_centre_y);
        let here = universe.systems.get(pass.index).map(|s| s.id);
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
            if detail > 0.004 {
                for &(a, b) in &universe.routes {
                    if let (Some(a), Some(b)) = (universe.system_index(a), universe.system_index(b))
                    {
                        world.draw_line3D(
                            vec3(space.map_positions[a]),
                            vec3(space.map_positions[b]),
                            fade(ROUTE_LINE, detail),
                        );
                    }
                }
            }
            let grow = view.distance() * 0.1;
            let k = one.system_place.per_unit;
            for (i, system) in universe.systems.iter().enumerate() {
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
                let (halo_size, halo_alpha) = if known {
                    (HALO_SIZE.max(grow), 230.0)
                } else {
                    (HALO_SIZE.max(grow) * 0.5, 90.0)
                };
                let core_size = CORE_SIZE.max(grow * 0.3);
                // From inside a system the others are points of light in the sky, as bright
                // as the sky lets them be; out on the map they are halos.
                let point = view.per_pixel(pos, height) * SKY_STAR_PIXELS;
                let sky = (stars * 0.75).clamp(0.0, 1.0);
                let seen = sky + (1.0 - sky) * detail;
                let star = STAR_GLOW * k;
                let halo_size = morph_size(star, morph_size(point * 2.5, halo_size, detail), swell);
                let core_size = morph_size(star * 0.4, morph_size(point, core_size, detail), swell);
                let halo_alpha = halo_alpha * seen * swell;
                let core_alpha = 255.0 * seen * swell;
                if halo_alpha < 1.0 {
                    continue;
                }
                sprite(
                    &mut world,
                    camera,
                    &space.glow,
                    pos,
                    halo_size,
                    halo,
                    halo_alpha as u8,
                );
                sprite(
                    &mut world,
                    camera,
                    &space.glow,
                    pos,
                    core_size,
                    [255, 255, 255],
                    core_alpha as u8,
                );
            }
            // Ships: a mark beside each system that has ships, an arrowhead on the lane for each
            // ship between systems, its lane ahead lit and its destination ringed.
            let mut overlay = OverlayGeometry::default();
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
                        overlay.tris.extend(annulus(
                            goal,
                            icon * 2.6,
                            icon * 0.35,
                            [r, g, b, 235],
                            40,
                        ));
                        overlay
                            .tris
                            .extend(chevron(at, dir, icon * 2.9, [255, 255, 255, 255]));
                        overlay
                            .tris
                            .extend(chevron(at, dir, icon * 2.2, [r, g, b, 255]));
                    }
                    _ => {
                        overlay
                            .tris
                            .extend(diamond(at, icon * 1.3, [255, 255, 255, 255]));
                        overlay
                            .tris
                            .extend(diamond(at, icon * 0.95, [r, g, b, 255]));
                    }
                }
                if chosen {
                    overlay.tris.extend(annulus(
                        at,
                        icon * 2.0,
                        icon * 0.3,
                        [63, 208, 255, 255],
                        28,
                    ));
                }
            }
            if detail > 0.004 {
                // Colonies: a ring in each owner's colour around every system with its cities.
                for (index, owner, rank) in
                    map_colony_rings(&universe, &self.space_state.snapshot.colonies)
                {
                    let [r, g, b] = owner_rgb(owner);
                    overlay.tris.extend(annulus(
                        space.map_positions[index],
                        icon * (3.3 + 0.6 * f32::from(rank)),
                        icon * 0.26,
                        [r, g, b, 230],
                        40,
                    ));
                }
                if let Some(SpaceHover::Target(Target::System(index))) = self.space_state.hover
                    && let Some(&at) = space.map_positions.get(index)
                {
                    overlay.tris.extend(annulus(
                        at,
                        icon * 4.0,
                        icon * 0.4,
                        [255, 230, 160, 230],
                        48,
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
            let rgb = match (home, known) {
                (_, false) => [130, 140, 155],
                (Some(tint), true) => tint,
                (None, true) => [200, 215, 235],
            };
            space.labels.push(SpaceLabel {
                icon: None,
                text: if known {
                    system.name.clone()
                } else {
                    format!("{} (unexplored)", system.name)
                },
                x: x + halo_pixels.max(8.0) * 0.5,
                y: y + halo_pixels.max(8.0) * 0.4,
                colour: [rgb[0], rgb[1], rgb[2], (235.0 * detail) as u8],
                priority: if home.is_some() {
                    -1.0
                } else {
                    (x - width / 2.0).hypot(y - height / 2.0) / width
                },
            });
        }
        for mark in &marks {
            let Some((x, y, _)) = look.project(mark.at) else {
                continue;
            };
            let text = match mark.bound_for {
                Some(dest) => format!("to {}", universe.systems[dest].name),
                None if mark.ids.len() > 1 => format!("{} ships", mark.ids.len()),
                None => "1 ship".to_owned(),
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
    // SAFETY: the render thread has an active GL context; these calls only flip the depth mask
    // and blend mode around a billboard draw.
    unsafe {
        raylib::ffi::rlDrawRenderBatchActive();
        raylib::ffi::rlDisableDepthMask();
        raylib::ffi::BeginBlendMode(BlendMode::BLEND_ADDITIVE as i32);
    }
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
    // SAFETY: as above; the batch is flushed so the sprite is drawn with the settings above.
    unsafe {
        raylib::ffi::rlDrawRenderBatchActive();
        raylib::ffi::EndBlendMode();
        raylib::ffi::rlEnableDepthMask();
    }
}
