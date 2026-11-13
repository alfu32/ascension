//! Drawing the system frame and the star-map frame, and the cross-fade between frames.
//!
//! Level of detail: at system level the home globe is a sphere mesh of a few hundred triangles
//! textured with the planet image; at star-map level systems are additive sprites.

use raylib::consts::{BlendMode, MaterialMapIndex};
use raylib::prelude::{Rectangle, Texture2D};

use super::*;
use crate::entities::{frame_box, owner_color};
use crate::globe::PlanetScene;
use crate::hud_minimap::planet_pixels;
use crate::space::{SpaceSnapshot, annulus, claim_rings, ship_frame_pos, transit_point};
use crate::universe::{
    BodyViewKind, UniverseScene, planet_colour, ring_points, rock_colour, uv_sphere,
};
use crate::zoom::ZoomCamera;

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
const BODY_FS: &str = r#"#version 330
in vec3 fragNormal;
in vec3 fragWorld;
in vec2 fragUv;
uniform sampler2D texture0;
uniform vec3 tint;
uniform float emissive;
out vec4 finalColor;
void main() {
    vec3 base = texture(texture0, fragUv).rgb * tint;
    vec3 normal = normalize(fragNormal);
    vec3 toStar = normalize(-fragWorld);
    float diffuse = max(dot(normal, toStar), 0.0);
    vec3 lit = base * (0.12 + 1.05 * diffuse);
    finalColor = vec4(mix(lit, base * 1.3, emissive), 1.0);
}
"#;

/// Rings, planets and sprites are drawn in this many segments per orbit.
const RING_SEGMENTS: u32 = 128;
/// Diameter of a system's halo on the star map, in map units.
const HALO_SIZE: f32 = 2500.0;
/// Diameter of the bright core of a system on the star map, in map units.
const CORE_SIZE: f32 = 650.0;
/// Diameter of the star's glow in the system frame, in system units.
const STAR_GLOW: f32 = 2600.0;
const GLOW_SIDE: u32 = 128;
const ORBIT_RING: Color = Color::new(120, 140, 185, 120);
const ROUTE_LINE: Color = Color::new(110, 140, 200, 110);

/// What is in space and which ships are selected, kept for the next draw.
#[derive(Default)]
pub(super) struct SpaceState {
    snapshot: Arc<SpaceSnapshot>,
    selected: Vec<u32>,
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
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) colour: [u8; 4],
    /// Lower comes first when labels compete for room.
    pub(crate) priority: f32,
}

/// Which frame to draw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Frame {
    Globe,
    System,
    Map,
}

/// Meshes, shader and sprites of the system and star-map frames.
pub(super) struct SpaceGpu {
    universe: Arc<UniverseScene>,
    home_body: Option<usize>,
    map_positions: Vec<[f32; 3]>,
    tints: Vec<Option<[u8; 3]>>,
    shader: Shader,
    material: WeakMaterial,
    lod_material: WeakMaterial,
    sphere: Mesh,
    lod: Mesh,
    glow: Texture2D,
    planet_texture: Option<Texture2D>,
    aux: Option<RenderTexture2D>,
    labels: Vec<SpaceLabel>,
}

/// Uploads a unit sphere with normals and texture coordinates.
fn upload_sphere(thread: &RaylibThread, rings: u32, slices: u32) -> Mesh {
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

fn ball(pos: [f32; 3], radius: f32) -> Matrix {
    Matrix {
        m0: radius,
        m5: radius,
        m10: radius,
        m12: pos[0],
        m13: pos[1],
        m14: pos[2],
        m15: 1.0,
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
        let mut shader = rl.load_shader_from_memory(thread, Some(BODY_VS), Some(BODY_FS));
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
            tints,
            shader,
            material,
            lod_material,
            sphere: upload_sphere(thread, 20, 40),
            lod: upload_sphere(thread, 12, 24),
            glow,
            planet_texture,
            aux: None,
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

    /// The names to draw over the last frame, in render-target pixels.
    pub(crate) fn space_labels(&self) -> &[SpaceLabel] {
        self.space.as_ref().map_or(&[], |s| &s.labels)
    }

    /// Whether the system and star-map frames exist.
    pub(crate) fn has_space(&self) -> bool {
        self.space.is_some()
    }

    /// Draws the frames the zoom camera shows, cross-fading two of them at a hand-over.
    pub(crate) fn draw_zoomed(
        &mut self,
        rl: &mut RaylibHandle,
        thread: &RaylibThread,
        target: &mut RenderTexture2D,
        zoom: &ZoomCamera,
        orbit: &OrbitCamera,
    ) {
        let weights = zoom.weights();
        let frames: Vec<(Frame, f32)> = [
            (Frame::Globe, weights.globe),
            (Frame::System, weights.system),
            (Frame::Map, weights.map),
        ]
        .into_iter()
        .filter(|&(_, w)| w > 0.002)
        .collect();
        if let Some(space) = &mut self.space {
            space.labels.clear();
        }
        let Some(&(first, _)) = frames.first() else {
            return;
        };
        self.draw_frame(rl, thread, target, first, zoom, orbit);
        let Some(&(second, weight)) = frames.get(1) else {
            return;
        };
        let (width, height) = (target.texture.width, target.texture.height);
        let Some(space) = &mut self.space else {
            return;
        };
        let mut aux = match space.aux.take() {
            Some(aux) if aux.texture.width == width && aux.texture.height == height => aux,
            _ => match rl.load_render_texture(thread, width as u32, height as u32) {
                Ok(aux) => aux,
                Err(error) => {
                    eprintln!("rom-render: cross-fade target: {error}");
                    return;
                }
            },
        };
        self.draw_frame(rl, thread, &mut aux, second, zoom, orbit);
        let mut drawing = rl.begin_texture_mode(thread, target);
        // Render textures are stored upside down: a negative source height flips them back.
        drawing.draw_texture_pro(
            &aux,
            Rectangle::new(0.0, 0.0, width as f32, -(height as f32)),
            Rectangle::new(0.0, 0.0, width as f32, height as f32),
            Vector2::zero(),
            0.0,
            Color::new(
                255,
                255,
                255,
                (weight.clamp(0.0, 1.0) * 255.0).round() as u8,
            ),
        );
        drop(drawing);
        if let Some(space) = &mut self.space {
            space.aux = Some(aux);
        }
    }

    fn draw_frame(
        &mut self,
        rl: &mut RaylibHandle,
        thread: &RaylibThread,
        target: &mut RenderTexture2D,
        frame: Frame,
        zoom: &ZoomCamera,
        orbit: &OrbitCamera,
    ) {
        match frame {
            Frame::Globe => self.draw_globe(rl, thread, target, &zoom.draw_orbit(orbit)),
            Frame::System => self.draw_system(rl, thread, target, zoom, orbit),
            Frame::Map => self.draw_star_map(rl, thread, target, zoom),
        }
    }

    fn draw_system(
        &mut self,
        rl: &mut RaylibHandle,
        thread: &RaylibThread,
        target: &mut RenderTexture2D,
        zoom: &ZoomCamera,
        orbit: &OrbitCamera,
    ) {
        self.set_light(super::SHIP_LIGHT);
        let Some(space) = &mut self.space else {
            return;
        };
        let view = zoom.system_view(orbit);
        let camera = Camera3D::perspective(vec3(view.eye), vec3(view.target), vec3(view.up), 45.0);
        let universe = Arc::clone(&space.universe);
        let star_colour = universe
            .systems
            .get(zoom.system())
            .map_or([255, 244, 214], |s| s.star_colour);
        let tint_at = space.shader.get_shader_location("tint");
        let emissive_at = space.shader.get_shader_location("emissive");
        let mut drawing = rl.begin_texture_mode(thread, target);
        drawing.clear_background(SPACE);
        if let Some(stars) = &self.star_mesh {
            let forward = vec3([
                view.target[0] - view.eye[0],
                view.target[1] - view.eye[1],
                view.target[2] - view.eye[2],
            ]);
            let sky = Camera3D::perspective(
                Vector3::new(0.0, 0.0, 0.0),
                forward,
                Vector3::new(0.0, 1.0, 0.0),
                45.0,
            );
            let mut backdrop = drawing.begin_mode3D(sky);
            shift_projection(self.projection_centre_y);
            backdrop.draw_mesh(stars, self.star_material.clone(), Matrix::identity());
        }
        set_clip_planes(f64::from(view.near), f64::from(view.far));
        {
            let mut world = drawing.begin_mode3D(camera);
            shift_projection(self.projection_centre_y);
            for placed in zoom.placed() {
                let body = &universe.bodies[placed.body];
                if body.kind != BodyViewKind::Star {
                    let ring = ring_points(placed.orbit, RING_SEGMENTS);
                    for pair in ring.windows(2) {
                        world.draw_line3D(vec3(pair[0]), vec3(pair[1]), ORBIT_RING);
                    }
                }
            }
            for placed in zoom.placed() {
                let body = &universe.bodies[placed.body];
                let (colour, emissive) = match body.kind {
                    BodyViewKind::Star => (star_colour, 1.0),
                    BodyViewKind::Habitable => (planet_colour(body.id), 0.0),
                    BodyViewKind::Resource => (rock_colour(body.id), 0.0),
                };
                let is_home =
                    Some(placed.body) == space.home_body && space.planet_texture.is_some();
                let (mesh, material, tint) = if is_home {
                    (&space.lod, &space.lod_material, [1.0, 1.0, 1.0])
                } else {
                    (&space.sphere, &space.material, colour3(colour))
                };
                space.shader.set_shader_value(tint_at, tint);
                space.shader.set_shader_value(emissive_at, emissive);
                world.draw_mesh(mesh, material.clone(), ball(placed.pos, placed.radius));
            }
            let system_id = universe.systems.get(zoom.system()).map(|s| s.id);
            let state = &self.space_state;
            if state.snapshot.system == system_id && system_id.is_some() {
                let outermost = system_id.map_or(1.0, |id| universe.system_outermost(id));
                let distance = zoom.scale().system_distance(zoom.l());
                let unit = (distance * 0.016).max(24.0);
                let rings = claim_rings(&universe, zoom.placed(), &state.snapshot.claims);
                let thickness = (distance * 0.004).max(5.0);
                let mut overlay = OverlayGeometry::default();
                for ring in &rings {
                    let [r, g, b] = ring.colour;
                    overlay.tris.extend(annulus(
                        [ring.centre[0], 0.4, ring.centre[2]],
                        ring.radius,
                        thickness,
                        [r, g, b, 235],
                        96,
                    ));
                }
                let forward = unit_vector([
                    view.target[0] - view.eye[0],
                    view.target[1] - view.eye[1],
                    view.target[2] - view.eye[2],
                ]);
                let right = unit_vector(cross3(forward, [0.0, 1.0, 0.0]));
                let cam_up = cross3(right, forward);
                let mut matrices: Vec<Matrix> = Vec::with_capacity(state.snapshot.ships.len() * 2);
                for ship in &state.snapshot.ships {
                    let at = ship_frame_pos(&universe, zoom.placed(), outermost, ship);
                    let theta = ship.facing_deg.to_radians();
                    let f = [theta.cos(), 0.0, theta.sin()];
                    let side = [-theta.sin(), 0.0, theta.cos()];
                    let colour = owner_color(ship.owner);
                    let axes = [f, [0.0, 1.0, 0.0], side];
                    let lift = [at[0], at[1] + unit * 0.4, at[2]];
                    matrices.push(matrix_of(&frame_box(
                        lift,
                        axes,
                        [unit * 3.2, unit * 0.7, unit * 1.1],
                        colour,
                    )));
                    let cabin = [
                        lift[0] + f[0] * unit * 0.5,
                        lift[1] + unit * 0.6,
                        lift[2] + f[2] * unit * 0.5,
                    ];
                    matrices.push(matrix_of(&frame_box(
                        cabin,
                        axes,
                        [unit * 1.1, unit * 0.6, unit * 0.7],
                        lerp_colour(owner_rgb(ship.owner), 0.45),
                    )));
                    let selected = state.selected.binary_search(&ship.id).is_ok();
                    if selected {
                        let ring = annulus(
                            [at[0], 0.6, at[2]],
                            unit * 2.4,
                            (unit * 0.22).max(thickness * 0.7),
                            [63, 208, 255, 255],
                            28,
                        );
                        overlay.tris.extend(ring);
                    }
                    if selected || ship.hp_frac < 0.999 {
                        let width = unit * 3.0;
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
                            .extend(quad(-width / 2.0, width / 2.0, [30, 30, 30, 255]));
                        overlay
                            .tris
                            .extend(quad(-width / 2.0, filled, [70, 230, 90, 255]));
                    }
                }
                if !matrices.is_empty() {
                    world.draw_mesh_instanced(
                        &self.box_mesh,
                        self.entity_material.clone(),
                        &matrices,
                    );
                }
                super::draw_overlay(&mut world, &overlay);
            }
            // The glow of the star: additive, without writing depth.
            if let Some(star) = zoom
                .placed()
                .iter()
                .find(|p| universe.bodies[p.body].kind == BodyViewKind::Star)
            {
                sprite(
                    &mut world,
                    camera,
                    &space.glow,
                    star.pos,
                    STAR_GLOW,
                    star_colour,
                    150,
                );
                sprite(
                    &mut world,
                    camera,
                    &space.glow,
                    star.pos,
                    STAR_GLOW * 0.4,
                    [255, 255, 255],
                    200,
                );
            }
        }
        set_clip_planes(DEFAULT_NEAR, DEFAULT_FAR);
    }

    fn draw_star_map(
        &mut self,
        rl: &mut RaylibHandle,
        thread: &RaylibThread,
        target: &mut RenderTexture2D,
        zoom: &ZoomCamera,
    ) {
        let Some(space) = &mut self.space else {
            return;
        };
        let view = zoom.map_view();
        let camera = Camera3D::perspective(vec3(view.eye), vec3(view.target), vec3(view.up), 45.0);
        let universe = Arc::clone(&space.universe);
        let (width, height) = (target.texture.width as f32, target.texture.height as f32);
        let mut drawing = rl.begin_texture_mode(thread, target);
        drawing.clear_background(SPACE);
        if let Some(stars) = &self.star_mesh {
            let forward = vec3([
                view.target[0] - view.eye[0],
                view.target[1] - view.eye[1],
                view.target[2] - view.eye[2],
            ]);
            let sky = Camera3D::perspective(
                Vector3::new(0.0, 0.0, 0.0),
                forward,
                Vector3::new(0.0, 1.0, 0.0),
                45.0,
            );
            let mut backdrop = drawing.begin_mode3D(sky);
            shift_projection(self.projection_centre_y);
            backdrop.draw_mesh(stars, self.star_material.clone(), Matrix::identity());
        }
        set_clip_planes(f64::from(view.near), f64::from(view.far));
        {
            let mut world = drawing.begin_mode3D(camera);
            shift_projection(self.projection_centre_y);
            for &(a, b) in &universe.routes {
                if let (Some(a), Some(b)) = (universe.system_index(a), universe.system_index(b)) {
                    world.draw_line3D(
                        vec3(space.map_positions[a]),
                        vec3(space.map_positions[b]),
                        ROUTE_LINE,
                    );
                }
            }
            let grow_now = zoom.scale().map_distance(zoom.l()) * 0.1;
            for transit in &self.space_state.snapshot.transits {
                if let (Some(a), Some(b)) = (
                    universe.system_index(transit.from),
                    universe.system_index(transit.to),
                ) {
                    let [r, g, b_] = owner_rgb(transit.owner);
                    world.draw_line3D(
                        vec3(space.map_positions[a]),
                        vec3(space.map_positions[b]),
                        Color::new(r, g, b_, 190),
                    );
                    let at = transit_point(
                        space.map_positions[a],
                        space.map_positions[b],
                        transit.progress,
                    );
                    sprite(
                        &mut world,
                        camera,
                        &space.glow,
                        at,
                        (grow_now * 0.55).max(1300.0),
                        [r, g, b_],
                        255,
                    );
                    sprite(
                        &mut world,
                        camera,
                        &space.glow,
                        at,
                        (grow_now * 0.2).max(480.0),
                        [255, 255, 255],
                        255,
                    );
                }
            }
            for (i, system) in universe.systems.iter().enumerate() {
                let pos = space.map_positions[i];
                let halo = space.tints[i].unwrap_or(system.star_colour);
                let grow = zoom.scale().map_distance(zoom.l()) * 0.1;
                let (halo_size, core_size) = (HALO_SIZE.max(grow), CORE_SIZE.max(grow * 0.3));
                sprite(&mut world, camera, &space.glow, pos, halo_size, halo, 230);
                sprite(
                    &mut world,
                    camera,
                    &space.glow,
                    pos,
                    core_size,
                    [255, 255, 255],
                    255,
                );
            }
        }
        set_clip_planes(DEFAULT_NEAR, DEFAULT_FAR);
        drop(drawing);
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
                HALO_SIZE.max(zoom.scale().map_distance(zoom.l()) * 0.1) / 2.0 / depth * height
                    / (2.0 * look.tan());
            let home = space.tints[i];
            let rgb = home.unwrap_or([200, 215, 235]);
            space.labels.push(SpaceLabel {
                text: system.name.clone(),
                x: x + halo_pixels.max(8.0) * 0.5,
                y: y + halo_pixels.max(8.0) * 0.4,
                colour: [rgb[0], rgb[1], rgb[2], 235],
                priority: if home.is_some() {
                    -1.0
                } else {
                    (x - width / 2.0).hypot(y - height / 2.0) / width
                },
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
    world.draw_billboard(
        camera,
        texture,
        vec3(pos),
        size,
        Color::new(colour[0], colour[1], colour[2], alpha),
    );
    // SAFETY: as above; the batch is flushed so the sprite is drawn with the settings above.
    unsafe {
        raylib::ffi::rlDrawRenderBatchActive();
        raylib::ffi::EndBlendMode();
        raylib::ffi::rlEnableDepthMask();
    }
}
