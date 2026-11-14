//! The part kit: one generator per [`Shape`], each painting flat-shaded faces of its part in
//! the part's own frame (base centre on the origin, x along its length, y up, front towards +z).
//! Surface detail comes from the material programs, not from geometry: a shape emits only its
//! silhouette, a few insets (openings, sills) and, for the far version, even less.

use std::f32::consts::{FRAC_PI_2, PI, TAU};

use crate::models::{ModelMesh, PartSpec, Shape, TEAM_BIT};
use crate::rig::{Activity, AttackStyle, BeastStyle, FigureStyle, Joint, Motion, Rig, WorkStyle};

/// Program id of faces drawn in their plain colour (openings, skin, hair).
const PLAIN: u8 = 0;
/// Program ids of the material library (see `materials.glsl`).
const TIMBER: u8 = 12;
const METAL: u8 = 15;

/// A rotation as the rows of a 3 by 3 matrix.
pub type Rotation = [[f32; 3]; 3];

fn mat_vec(m: &Rotation, v: [f32; 3]) -> [f32; 3] {
    m.map(|row| dot(row, v))
}

fn mat_mul(a: &Rotation, b: &Rotation) -> Rotation {
    std::array::from_fn(|i| std::array::from_fn(|j| (0..3).map(|k| a[i][k] * b[k][j]).sum()))
}

/// The turn by `yaw` radians about y as a matrix (the turn of [`Place::turn`]).
fn yaw_matrix(yaw: f32) -> Rotation {
    let (s, c) = yaw.sin_cos();
    [[c, 0.0, s], [0.0, 1.0, 0.0], [-s, 0.0, c]]
}

/// A lean: roll tips the top towards +x, then pitch tips it towards +z (degrees); `None` when
/// both are zero, so upright parts take exactly the path they took before leaning existed.
pub fn tilt(pitch_deg: f32, roll_deg: f32) -> Option<Rotation> {
    if pitch_deg == 0.0 && roll_deg == 0.0 {
        return None;
    }
    let (sp, cp) = pitch_deg.to_radians().sin_cos();
    let (sr, cr) = roll_deg.to_radians().sin_cos();
    let pitch = [[1.0, 0.0, 0.0], [0.0, cp, -sp], [0.0, sp, cp]];
    let roll = [[cr, sr, 0.0], [-sr, cr, 0.0], [0.0, 0.0, 1.0]];
    Some(mat_mul(&pitch, &roll))
}

/// A lean (`tilt`), then a turn about y (radians, counter-clockwise seen from above), uniform
/// scale, then a move.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Place {
    /// Turn about the vertical axis, in radians.
    pub yaw: f32,
    /// Uniform scale.
    pub scale: f32,
    /// Where the local origin lands.
    pub at: [f32; 3],
    /// Rotation applied before the turn; `None` keeps the part upright.
    pub tilt: Option<Rotation>,
}

impl Place {
    /// No change.
    pub const IDENTITY: Place = Place {
        yaw: 0.0,
        scale: 1.0,
        at: [0.0; 3],
        tilt: None,
    };

    /// A move by `at` with no turn.
    pub fn at(at: [f32; 3]) -> Self {
        Self {
            at,
            ..Self::IDENTITY
        }
    }

    /// A move by `at`, then the turn `yaw` (radians) about the new origin.
    pub fn turned(at: [f32; 3], yaw: f32) -> Self {
        Self {
            yaw,
            ..Self::at(at)
        }
    }

    /// `v` turned (and scaled) as this place turns local directions.
    pub fn turn(&self, v: [f32; 3]) -> [f32; 3] {
        let v = match &self.tilt {
            Some(m) => mat_vec(m, v),
            None => v,
        };
        let (s, c) = self.yaw.sin_cos();
        [
            (v[0] * c + v[2] * s) * self.scale,
            v[1] * self.scale,
            (-v[0] * s + v[2] * c) * self.scale,
        ]
    }

    /// The point `p` of the local frame in the outer frame.
    pub fn apply(&self, p: [f32; 3]) -> [f32; 3] {
        let t = self.turn(p);
        [t[0] + self.at[0], t[1] + self.at[1], t[2] + self.at[2]]
    }

    /// `child` placed inside this place.
    pub fn then(&self, child: &Place) -> Place {
        // Upright parents add turns; a leaning parent folds the child's turn into its lean.
        let (yaw, tilt) = match &self.tilt {
            None => (self.yaw + child.yaw, child.tilt),
            Some(lean) => {
                let inner = mat_mul(lean, &yaw_matrix(child.yaw));
                let tilt = match &child.tilt {
                    Some(c) => mat_mul(&inner, c),
                    None => inner,
                };
                (self.yaw, Some(tilt))
            }
        };
        Place {
            yaw,
            scale: self.scale * child.scale,
            at: self.apply(child.at),
            tilt,
        }
    }
}

/// What the faces being painted look like.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Surface {
    /// Main colour, RGB 0..1.
    pub colour: [f32; 3],
    /// Second colour, RGB 0..1.
    pub alt: [f32; 3],
    /// Program id.
    pub program: u8,
    /// Wears the owner's colour.
    pub team: bool,
    /// Pattern size in tiles.
    pub scale: f32,
}

impl Surface {
    /// A plain surface of one colour.
    pub fn plain(colour: [f32; 3]) -> Self {
        Self {
            colour,
            alt: colour.map(|c| c * 0.8),
            program: PLAIN,
            team: false,
            scale: 1.0,
        }
    }

    /// A surface of `program` in `colour`, its second tone darker.
    fn of(program: u8, colour: [f32; 3], scale: f32) -> Self {
        Self {
            colour,
            alt: colour.map(|c| c * 0.62),
            program,
            team: false,
            scale,
        }
    }
}

fn byte(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let l = dot(v, v).sqrt();
    if l < 1e-12 {
        [0.0, 1.0, 0.0]
    } else {
        v.map(|c| c / l)
    }
}

/// Paints faces into a model mesh through a place.
pub struct Painter<'a> {
    mesh: &'a mut ModelMesh,
    /// Where the part's frame sits in the model.
    pub place: Place,
    /// The look of the faces painted next.
    pub surface: Surface,
    /// The joint the faces painted next move with; after a shape is painted, the joint its
    /// attached parts follow.
    pub joint: u8,
    rig: Option<&'a mut Rig>,
}

/// A named point of a painted part other parts can attach to.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Socket {
    /// The socket's name.
    pub name: &'static str,
    /// Its frame, in the part's frame.
    pub place: Place,
    /// The joint it moves with.
    pub joint: u8,
    /// The motion of what is held there (a hand), if it turns in the socket.
    pub grip: Option<Motion>,
}

impl<'a> Painter<'a> {
    /// A painter adding to `mesh` at `place` with `surface`.
    pub fn new(mesh: &'a mut ModelMesh, place: Place, surface: Surface) -> Self {
        Self {
            mesh,
            place,
            surface,
            joint: 0,
            rig: None,
        }
    }

    /// A painter that also records the joints of what it paints into `rig`, its faces moving
    /// with `joint` until a shape says otherwise.
    pub fn rigged(
        mesh: &'a mut ModelMesh,
        place: Place,
        surface: Surface,
        rig: &'a mut Rig,
        joint: u8,
    ) -> Self {
        Self {
            mesh,
            place,
            surface,
            joint,
            rig: Some(rig),
        }
    }

    /// Adds a joint below the current one, turning about `pivot` (in the current place) in the
    /// current place's axes; without a rig, or with the rig full, the current joint.
    pub fn add_joint(&mut self, pivot: [f32; 3], motion: Motion) -> u8 {
        let place = self.place;
        let parent = self.joint;
        let Some(rig) = self.rig.as_deref_mut() else {
            return parent;
        };
        let scale = place.scale.max(1e-6);
        let axes = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]].map(|e| {
            let v = place.turn(e);
            [v[0] / scale, v[1] / scale, v[2] / scale]
        });
        let frame = std::array::from_fn(|i| std::array::from_fn(|j| axes[j][i]));
        rig.add(Joint {
            parent,
            pivot: place.apply(pivot),
            frame,
            scale,
            motion,
        })
        .unwrap_or(parent)
    }

    /// Sets the cycle of `activity` (seconds) unless an earlier part has.
    pub fn suggest_period(&mut self, activity: Activity, seconds: f32) {
        if let Some(rig) = self.rig.as_deref_mut() {
            rig.suggest_period(activity, seconds);
        }
    }

    /// Paints the convex polygon `points` (local frame) facing along `outward`.
    pub fn face(&mut self, points: &[[f32; 3]], outward: [f32; 3]) {
        let mut pts: Vec<[f32; 3]> = Vec::with_capacity(points.len());
        for p in points.iter().map(|p| self.place.apply(*p)) {
            if pts
                .last()
                .is_none_or(|q| dot(sub(p, *q), sub(p, *q)) > 1e-12)
            {
                pts.push(p);
            }
        }
        while pts.len() > 1
            && dot(
                sub(pts[0], pts[pts.len() - 1]),
                sub(pts[0], pts[pts.len() - 1]),
            ) <= 1e-12
        {
            pts.pop();
        }
        if pts.len() < 3 || self.mesh.positions.len() + pts.len() > usize::from(u16::MAX) {
            return;
        }
        // Newell's normal of the polygon, turned to agree with `outward`.
        let mut n = [0.0_f32; 3];
        for i in 0..pts.len() {
            let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
            n[0] += (a[1] - b[1]) * (a[2] + b[2]);
            n[1] += (a[2] - b[2]) * (a[0] + b[0]);
            n[2] += (a[0] - b[0]) * (a[1] + b[1]);
        }
        if dot(n, n) < 1e-14 {
            return;
        }
        if dot(n, self.place.turn(outward)) < 0.0 {
            pts.reverse();
            n = n.map(|c| -c);
        }
        let normal = normalize(n);
        let s = self.surface;
        let tag = s.program | if s.team { TEAM_BIT } else { 0 };
        let colour = [byte(s.colour[0]), byte(s.colour[1]), byte(s.colour[2]), tag];
        let alt = [byte(s.alt[0]), byte(s.alt[1]), byte(s.alt[2]), 255];
        let first = self.mesh.positions.len() as u16;
        for p in &pts {
            self.mesh.positions.push(*p);
            self.mesh.normals.push(normal);
            self.mesh.colors.push(colour);
            self.mesh.alts.push(alt);
            self.mesh.scales.push(s.scale);
            self.mesh.joints.push(self.joint);
        }
        for i in 1..pts.len() as u16 - 1 {
            self.mesh
                .indices
                .extend_from_slice(&[first, first + i, first + i + 1]);
        }
    }

    /// Paints with `surface` and `local` (inside the current place) for the duration of `f`.
    fn within(&mut self, local: Place, surface: Option<Surface>, f: impl FnOnce(&mut Self)) {
        let (place, look) = (self.place, self.surface);
        self.place = place.then(&local);
        if let Some(s) = surface {
            self.surface = s;
        }
        f(self);
        self.place = place;
        self.surface = look;
    }

    /// A block standing on `base` (bottom centre) of `size` [w, h, d]; its top is `top` times
    /// as wide and deep. The bottom face is left out unless `bottom`.
    pub fn cuboid(&mut self, base: [f32; 3], size: [f32; 3], top: f32, bottom: bool) {
        let [w, _, d] = size.map(|v| v / 2.0);
        let [x, y, z] = base;
        let (tw, td) = (w * top, d * top);
        let b = [
            [x - w, y, z - d],
            [x + w, y, z - d],
            [x + w, y, z + d],
            [x - w, y, z + d],
        ];
        let t = [
            [x - tw, y + size[1], z - td],
            [x + tw, y + size[1], z - td],
            [x + tw, y + size[1], z + td],
            [x - tw, y + size[1], z + td],
        ];
        self.face(&[b[3], b[2], t[2], t[3]], [0.0, 0.0, 1.0]);
        self.face(&[b[1], b[0], t[0], t[1]], [0.0, 0.0, -1.0]);
        self.face(&[b[2], b[1], t[1], t[2]], [1.0, 0.0, 0.0]);
        self.face(&[b[0], b[3], t[3], t[0]], [-1.0, 0.0, 0.0]);
        self.face(&t, [0.0, 1.0, 0.0]);
        if bottom {
            self.face(&b, [0.0, -1.0, 0.0]);
        }
    }

    /// An upright cylinder of `segments` sides, radius `r` at the bottom and `r_top` at the top,
    /// standing on `base`; the top is capped when `cap`.
    pub fn cylinder(
        &mut self,
        base: [f32; 3],
        (r, r_top): (f32, f32),
        h: f32,
        segments: usize,
        cap: bool,
    ) {
        let ring = |radius: f32, y: f32| -> Vec<[f32; 3]> {
            (0..segments)
                .map(|i| {
                    let a = i as f32 / segments as f32 * TAU;
                    [
                        base[0] + radius * a.cos(),
                        base[1] + y,
                        base[2] + radius * a.sin(),
                    ]
                })
                .collect()
        };
        let (lo, hi) = (ring(r, 0.0), ring(r_top, h));
        for i in 0..segments {
            let j = (i + 1) % segments;
            let mid = (i as f32 + 0.5) / segments as f32 * TAU;
            self.face(&[lo[i], lo[j], hi[j], hi[i]], [mid.cos(), 0.0, mid.sin()]);
        }
        if cap && r_top > 1e-4 {
            self.face(&hi, [0.0, 1.0, 0.0]);
        }
    }

    /// A cone of `segments` sides on `base`.
    pub fn cone(&mut self, base: [f32; 3], r: f32, h: f32, segments: usize) {
        let apex = [base[0], base[1] + h, base[2]];
        for i in 0..segments {
            let (a, b) = (
                i as f32 / segments as f32 * TAU,
                (i + 1) as f32 / segments as f32 * TAU,
            );
            let p = |t: f32| [base[0] + r * t.cos(), base[1], base[2] + r * t.sin()];
            let mid = (a + b) / 2.0;
            self.face(&[p(a), p(b), apex], [mid.cos(), r / h.max(1e-3), mid.sin()]);
        }
    }

    /// Part of an ellipsoid round `centre` with radii `r` (horizontal) and `ry`, between
    /// latitudes `from` and `to` (radians), in `rings` bands of `segments` sides.
    pub fn sphere_band(
        &mut self,
        centre: [f32; 3],
        r: f32,
        ry: f32,
        (from, to): (f32, f32),
        rings: usize,
        segments: usize,
    ) {
        let point = |lat: f32, lon: f32| {
            [
                centre[0] + r * lat.cos() * lon.cos(),
                centre[1] + ry * lat.sin(),
                centre[2] + r * lat.cos() * lon.sin(),
            ]
        };
        for k in 0..rings {
            let (a, b) = (
                from + (to - from) * k as f32 / rings as f32,
                from + (to - from) * (k + 1) as f32 / rings as f32,
            );
            for i in 0..segments {
                let (l0, l1) = (
                    i as f32 / segments as f32 * TAU,
                    (i + 1) as f32 / segments as f32 * TAU,
                );
                let quad = [point(a, l0), point(a, l1), point(b, l1), point(b, l0)];
                let mid = (a + b) / 2.0;
                let lm = (l0 + l1) / 2.0;
                let out = [mid.cos() * lm.cos(), mid.sin(), mid.cos() * lm.sin()];
                self.face(&quad, out);
            }
        }
    }
}

/// Reads a part's knobs with defaults.
struct Knobs<'a>(&'a PartSpec);

impl Knobs<'_> {
    fn get(&self, key: &str, default: f32) -> f32 {
        self.0
            .knobs
            .get(key)
            .copied()
            .filter(|v| v.is_finite())
            .unwrap_or(default)
    }

    /// A size knob: positive, else `default`.
    fn size(&self, key: &str, default: f32) -> f32 {
        Some(self.get(key, default))
            .filter(|v| *v > 0.0)
            .unwrap_or(default)
    }

    fn count(&self, key: &str, default: usize, max: usize) -> usize {
        self.get(key, default as f32).round().clamp(0.0, max as f32) as usize
    }

    fn colour(&self, key: &str, default: [f32; 3]) -> [f32; 3] {
        self.0.colours.get(key).copied().unwrap_or(default)
    }
}

/// Segments of a round shape: the knob, halved (at least `min`) for the far version.
fn segments(k: &Knobs<'_>, default: usize, fine: bool, min: usize) -> usize {
    let n = k.count("segments", default, 48).max(3);
    if fine { n } else { (n / 2).max(min) }
}

/// Paints `part` with the painter's place and surface; returns its sockets in the part frame.
/// Shapes that move record their joints first, the same in the near and far versions, and
/// leave [`Painter::joint`] at the joint their attached parts follow.
pub fn paint(part: &PartSpec, p: &mut Painter<'_>, fine: bool) -> Vec<Socket> {
    let k = Knobs(part);
    match part.shape {
        Shape::Box => block(&k, p, fine),
        Shape::Plinth => plinth(&k, p, fine),
        Shape::GableRoof => gable_roof(&k, p, fine),
        Shape::HipRoof => hip_roof(&k, p, fine),
        Shape::FlatRoof => flat_roof(&k, p),
        Shape::Dome => {
            let r = k.size("w", 1.0) / 2.0;
            let rings = if fine {
                (segments(&k, 12, true, 6) / 4).max(2)
            } else {
                2
            };
            p.sphere_band(
                [0.0; 3],
                r,
                k.size("h", r),
                (0.0, FRAC_PI_2),
                rings,
                segments(&k, 12, fine, 6),
            );
        }
        Shape::Cylinder => {
            let r = k.size("w", 1.0) / 2.0;
            let taper = k.get("taper", 0.0).clamp(0.0, 1.0);
            let n = segments(&k, 10, fine, 4);
            p.cylinder([0.0; 3], (r, r * (1.0 - taper)), k.size("h", 1.0), n, true);
            // A leaning cylinder (a log, a wheel) shows its foot unless told otherwise.
            let leaning = if p.place.tilt.is_some() { 1.0 } else { 0.0 };
            if k.get("bottom", leaning) > 0.5 {
                let ring: Vec<[f32; 3]> = (0..n)
                    .map(|i| {
                        let a = i as f32 / n as f32 * TAU;
                        [r * a.cos(), 0.0, r * a.sin()]
                    })
                    .collect();
                p.face(&ring, [0.0, -1.0, 0.0]);
            }
        }
        Shape::Cone => {
            p.cone(
                [0.0; 3],
                k.size("w", 1.0) / 2.0,
                k.size("h", 1.0),
                segments(&k, 10, fine, 4),
            );
        }
        Shape::ColumnRow => column_row(&k, p, fine),
        Shape::Wall => wall(&k, p, fine),
        Shape::Crenellation => crenellation(&k, p, fine),
        Shape::Fence => fence(&k, p, fine),
        Shape::Palisade => palisade(&k, p, fine),
        Shape::Banner => banner(&k, p, fine),
        Shape::Awning => awning(&k, p),
        Shape::Arch => arch(&k, p, fine),
        Shape::Hull => hull(&k, p, fine),
        Shape::Figure => return figure(&k, p, fine),
        Shape::Quadruped => return quadruped(&k, p, fine),
        Shape::Wheel => wheel(&k, p, fine),
        Shape::OarBank => oar_bank(&k, p, fine),
        Shape::Sail => sail(&k, p, fine),
        Shape::Shield => shield(&k, p, fine),
        Shape::Bow => bow(&k, p, fine),
        Shape::Include => {}
    }
    Vec::new()
}

/// `box`: a block, tapered or gabled, with openings.
fn block(k: &Knobs<'_>, p: &mut Painter<'_>, fine: bool) {
    let (w, d, h) = (k.size("w", 1.0), k.size("d", 1.0), k.size("h", 1.0));
    let gable = k.get("gable", 0.0).clamp(0.0, 75.0);
    if gable > 0.0 {
        let ridge = h + d / 2.0 * gable.to_radians().tan();
        let (x, z) = (w / 2.0, d / 2.0);
        p.face(
            &[[-x, 0.0, z], [x, 0.0, z], [x, h, z], [-x, h, z]],
            [0.0, 0.0, 1.0],
        );
        p.face(
            &[[x, 0.0, -z], [-x, 0.0, -z], [-x, h, -z], [x, h, -z]],
            [0.0, 0.0, -1.0],
        );
        for s in [1.0, -1.0] {
            p.face(
                &[
                    [s * x, 0.0, -z],
                    [s * x, 0.0, z],
                    [s * x, h, z],
                    [s * x, ridge, 0.0],
                    [s * x, h, -z],
                ],
                [s, 0.0, 0.0],
            );
            // The slopes lie under a roof; the far version leaves them out.
            if fine {
                p.face(
                    &[
                        [-x, h, s * z],
                        [x, h, s * z],
                        [x, ridge, 0.0],
                        [-x, ridge, 0.0],
                    ],
                    [0.0, 1.0, s],
                );
            }
        }
    } else {
        let top = 1.0 - k.get("taper", 0.0).clamp(0.0, 0.95);
        p.cuboid([0.0; 3], [w, h, d], top, false);
    }
    if fine {
        openings(k, p, (w, d, h));
    }
}

/// Door on the front, windows on the front and back and on the sides, of a block of `w`, `d`,
/// `h` centred on the origin.
fn openings(k: &Knobs<'_>, p: &mut Painter<'_>, (w, d, h): (f32, f32, f32)) {
    let dark = Surface::plain(k.colour("opening", [0.10, 0.09, 0.085]));
    let door_look = Surface::of(TIMBER, k.colour("door_colour", [0.34, 0.22, 0.13]), 0.1);
    let wall = p.surface;
    // A face: where its bottom centre is, its horizontal axis and its outward normal.
    let faces = [
        ([0.0, 0.0, d / 2.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0], w),
        ([0.0, 0.0, -d / 2.0], [-1.0, 0.0, 0.0], [0.0, 0.0, -1.0], w),
        ([w / 2.0, 0.0, 0.0], [0.0, 0.0, -1.0], [1.0, 0.0, 0.0], d),
        ([-w / 2.0, 0.0, 0.0], [0.0, 0.0, 1.0], [-1.0, 0.0, 0.0], d),
    ];
    let door = k.get("door", 0.0).clamp(0.0, w * 0.9);
    let door_h = k.size("door_h", (h * 0.6).min(0.8)).min(h * 0.95);
    let door_x = k.get("door_x", 0.0);
    let (ww, wh) = (k.size("window_w", 0.22), k.size("window_h", 0.28));
    let window_y = k.get("window_y", h * 0.5);
    let rows = k.count("window_rows", 1, 8).max(1);
    let row_gap = k.size("row_gap", wh + 0.3);
    let panel = |p: &mut Painter<'_>,
                 (o, r, n): ([f32; 3], [f32; 3], [f32; 3]),
                 u: f32,
                 y0: f32,
                 pw: f32,
                 ph: f32| {
        let at = |du: f32, y: f32| {
            [
                o[0] + r[0] * du + n[0] * 0.012,
                y,
                o[2] + r[2] * du + n[2] * 0.012,
            ]
        };
        p.face(
            &[
                at(u - pw / 2.0, y0),
                at(u + pw / 2.0, y0),
                at(u + pw / 2.0, y0 + ph),
                at(u - pw / 2.0, y0 + ph),
            ],
            n,
        );
    };
    let ledge = |p: &mut Painter<'_>,
                 (o, r, _): ([f32; 3], [f32; 3], [f32; 3]),
                 u: f32,
                 y: f32,
                 lw: f32| {
        let yaw = (-r[2]).atan2(r[0]);
        let centre = [o[0] + r[0] * u, y, o[2] + r[2] * u];
        p.within(Place::turned(centre, yaw), None, |p| {
            p.cuboid([0.0; 3], [lw, 0.05, 0.12], 1.0, false);
        });
    };
    for (index, &(o, r, n, span)) in faces.iter().enumerate() {
        let frame = (o, r, n);
        if index == 0 && door > 0.0 {
            p.surface = door_look;
            panel(p, frame, door_x, 0.0, door, door_h);
            p.surface = Surface::of(TIMBER, [0.40, 0.28, 0.17], 0.1);
            ledge(p, frame, door_x, door_h, door + 0.14);
            p.surface = wall;
        }
        let count = if index < 2 {
            k.count("windows", 0, 16)
        } else {
            k.count("side_windows", 0, 16)
        };
        for row in 0..rows {
            let y = window_y + row as f32 * row_gap;
            if y + wh > h {
                break;
            }
            for i in 0..count {
                let u = ((i as f32 + 0.5) / count as f32 - 0.5) * span;
                if index == 0
                    && door > 0.0
                    && y < door_h
                    && (u - door_x).abs() < (door + ww) / 2.0 + 0.05
                {
                    continue;
                }
                p.surface = dark;
                panel(p, frame, u, y, ww, wh);
                p.surface = wall;
                ledge(p, frame, u, y - 0.05, ww + 0.08);
            }
        }
    }
    p.surface = wall;
}

/// `plinth`: steps, each inset from the one below.
fn plinth(k: &Knobs<'_>, p: &mut Painter<'_>, fine: bool) {
    let (w, d, h) = (k.size("w", 2.0), k.size("d", 2.0), k.size("h", 0.4));
    let steps = if fine {
        k.count("steps", 3, 12).max(1)
    } else {
        1
    };
    let inset = k.get("inset", 0.15).max(0.0);
    let step = h / steps as f32;
    for i in 0..steps {
        let shrink = 2.0 * inset * i as f32;
        p.cuboid(
            [0.0, step * i as f32, 0.0],
            [(w - shrink).max(0.05), step, (d - shrink).max(0.05)],
            1.0,
            false,
        );
    }
}

/// `gable_roof`: two slabs meeting at a ridge along x over a `w` by `d` block.
fn gable_roof(k: &Knobs<'_>, p: &mut Painter<'_>, fine: bool) {
    let (w, d) = (k.size("w", 2.0), k.size("d", 2.0));
    let pitch = k.get("pitch", 35.0).clamp(5.0, 75.0).to_radians();
    let over = k.get("overhang", 0.15).max(0.0);
    let t = k.size("t", 0.08);
    let tan = pitch.tan();
    let ridge = d / 2.0 * tan;
    let lift = t / pitch.cos();
    let ex = w / 2.0 + over;
    let eave_y = -over * tan;
    for s in [1.0_f32, -1.0] {
        let eave_z = s * (d / 2.0 + over);
        let under = [
            [-ex, eave_y, eave_z],
            [ex, eave_y, eave_z],
            [ex, ridge, 0.0],
            [-ex, ridge, 0.0],
        ];
        let top = under.map(|v| [v[0], v[1] + lift, v[2]]);
        p.face(&top, [0.0, 1.0, s * tan]);
        if !fine {
            continue;
        }
        p.face(&under, [0.0, -1.0, -s * tan]);
        p.face(&[under[0], under[1], top[1], top[0]], [0.0, 0.0, s]);
        for e in [1.0_f32, -1.0] {
            let i = if e > 0.0 { 1 } else { 0 };
            let j = if e > 0.0 { 2 } else { 3 };
            p.face(&[under[i], under[j], top[j], top[i]], [e, 0.0, 0.0]);
        }
    }
}

/// `hip_roof`: four slopes over a `w` by `d` block, a fascia and a flat underside.
fn hip_roof(k: &Knobs<'_>, p: &mut Painter<'_>, fine: bool) {
    let (w, d) = (k.size("w", 2.0), k.size("d", 2.0));
    let pitch = k.get("pitch", 30.0).clamp(5.0, 75.0).to_radians();
    let over = k.get("overhang", 0.15).max(0.0);
    let t = k.size("t", 0.08);
    let tan = pitch.tan();
    let (hw, hd) = (w / 2.0 + over, d / 2.0 + over);
    let eave = -over * tan;
    let top = eave + hw.min(hd) * tan;
    let corners = [
        [-hw, eave, -hd],
        [hw, eave, -hd],
        [hw, eave, hd],
        [-hw, eave, hd],
    ];
    let [a, b, c, dd] = corners;
    if hw >= hd {
        let rx = hw - hd;
        let (r1, r2) = ([-rx, top, 0.0], [rx, top, 0.0]);
        p.face(&[dd, c, r2, r1], [0.0, 1.0, tan]);
        p.face(&[b, a, r1, r2], [0.0, 1.0, -tan]);
        p.face(&[c, b, r2], [tan, 1.0, 0.0]);
        p.face(&[a, dd, r1], [-tan, 1.0, 0.0]);
    } else {
        let rz = hd - hw;
        let (r1, r2) = ([0.0, top, -rz], [0.0, top, rz]);
        p.face(&[c, b, r1, r2], [tan, 1.0, 0.0]);
        p.face(&[a, dd, r2, r1], [-tan, 1.0, 0.0]);
        p.face(&[dd, c, r2], [0.0, 1.0, tan]);
        p.face(&[b, a, r1], [0.0, 1.0, -tan]);
    }
    let low = corners.map(|v| [v[0], v[1] - t, v[2]]);
    p.face(&low, [0.0, -1.0, 0.0]);
    if fine {
        for i in 0..4 {
            let j = (i + 1) % 4;
            let mid = [
                (corners[i][0] + corners[j][0]) / 2.0,
                0.0,
                (corners[i][2] + corners[j][2]) / 2.0,
            ];
            p.face(&[corners[i], corners[j], low[j], low[i]], mid);
        }
    }
}

/// `flat_roof`: a slab with an optional parapet round its edge.
fn flat_roof(k: &Knobs<'_>, p: &mut Painter<'_>) {
    let over = k.get("overhang", 0.05).max(0.0);
    let (w, d) = (k.size("w", 2.0) + 2.0 * over, k.size("d", 2.0) + 2.0 * over);
    let t = k.size("t", 0.12);
    p.cuboid([0.0; 3], [w, t, d], 1.0, true);
    let parapet = k.get("parapet", 0.0);
    if parapet > 0.0 {
        let pt = k.size("parapet_t", 0.1);
        for run in ring_runs(w, d, pt, 0.0) {
            run.paint(p, |p, length| {
                p.cuboid([0.0, t, 0.0], [length, parapet, pt], 1.0, false)
            });
        }
    }
}

/// A straight run between two points of the ground plan (x, z).
#[derive(Clone, Copy, Debug)]
struct Run {
    from: [f32; 2],
    to: [f32; 2],
}

impl Run {
    fn length(&self) -> f32 {
        (self.to[0] - self.from[0]).hypot(self.to[1] - self.from[1])
    }

    /// Paints `f(painter, length)` in the run's frame: origin at its middle, x along it.
    fn paint(&self, p: &mut Painter<'_>, f: impl FnOnce(&mut Painter<'_>, f32)) {
        let length = self.length();
        if length < 1e-3 {
            return;
        }
        let dir = [
            (self.to[0] - self.from[0]) / length,
            (self.to[1] - self.from[1]) / length,
        ];
        let mid = [
            (self.from[0] + self.to[0]) / 2.0,
            0.0,
            (self.from[1] + self.to[1]) / 2.0,
        ];
        let yaw = (-dir[1]).atan2(dir[0]);
        p.within(Place::turned(mid, yaw), None, |p| f(p, length));
    }
}

/// The runs of a straight piece along x (`ring` 0) or of four sides round a `w` by `d` yard
/// whose centre lines are inset by half of `t`, with a `gap` in the middle of the front side.
fn ring_runs(w: f32, d: f32, t: f32, gap: f32) -> Vec<Run> {
    let (x, z) = (w / 2.0 - t / 2.0, d / 2.0 - t / 2.0);
    let mut runs = vec![
        Run {
            from: [-w / 2.0, -z],
            to: [w / 2.0, -z],
        },
        Run {
            from: [x, -z + t / 2.0],
            to: [x, z - t / 2.0],
        },
        Run {
            from: [-x, z - t / 2.0],
            to: [-x, -z + t / 2.0],
        },
    ];
    let gap = gap.clamp(0.0, w - 0.1);
    if gap > 0.0 {
        runs.push(Run {
            from: [-w / 2.0, z],
            to: [-gap / 2.0, z],
        });
        runs.push(Run {
            from: [gap / 2.0, z],
            to: [w / 2.0, z],
        });
    } else {
        runs.push(Run {
            from: [-w / 2.0, z],
            to: [w / 2.0, z],
        });
    }
    runs
}

/// The runs a linear shape follows: one along x, or a ring.
fn runs_of(k: &Knobs<'_>, w: f32, t: f32) -> Vec<Run> {
    if k.get("ring", 0.0) > 0.5 {
        ring_runs(w, k.size("d", w), t, k.get("gap", 0.0))
    } else {
        vec![Run {
            from: [-w / 2.0, 0.0],
            to: [w / 2.0, 0.0],
        }]
    }
}

/// `column_row`: columns along x with a base and a capital each.
fn column_row(k: &Knobs<'_>, p: &mut Painter<'_>, fine: bool) {
    let count = k.count("count", 4, 32).max(1);
    let r = k.size("radius", 0.12);
    let spacing = k.size("spacing", 0.6);
    let h = k.size("h", 1.6);
    let capital = k.get("capital", 1.0) > 0.5;
    let seg = segments(k, 8, fine, 4);
    for i in 0..count {
        let x = (i as f32 - (count - 1) as f32 / 2.0) * spacing;
        if fine && capital {
            p.cuboid([x, 0.0, 0.0], [2.6 * r, 0.08, 2.6 * r], 1.0, false);
            p.cylinder([x, 0.08, 0.0], (r, r * 0.86), h - 0.18, seg, false);
            p.cuboid([x, h - 0.1, 0.0], [2.8 * r, 0.1, 2.8 * r], 1.0, false);
        } else {
            // A shaft under an entablature: its top is never seen.
            p.cylinder([x, 0.0, 0.0], (r, r * 0.86), h, seg, fine);
        }
    }
}

/// `wall`: one wall along x (with openings on both faces) or four round a yard.
fn wall(k: &Knobs<'_>, p: &mut Painter<'_>, fine: bool) {
    let (w, h, t) = (k.size("w", 2.0), k.size("h", 1.5), k.size("t", 0.2));
    if k.get("ring", 0.0) > 0.5 {
        for run in runs_of(k, w, t) {
            run.paint(p, |p, length| {
                p.cuboid([0.0; 3], [length, h, t], 1.0, false)
            });
        }
    } else {
        p.cuboid([0.0; 3], [w, h, t], 1.0, false);
        if fine {
            openings(k, p, (w, t, h));
        }
    }
}

/// Positions along a run of `length` of `n` evenly spaced items `size` wide, end to end.
fn spread(length: f32, size: f32, n: usize) -> impl Iterator<Item = f32> {
    let span = (length - size).max(0.0);
    (0..n).map(move |i| {
        if n == 1 {
            0.0
        } else {
            -span / 2.0 + span * i as f32 / (n - 1) as f32
        }
    })
}

/// `crenellation`: merlons along a run or round a yard.
fn crenellation(k: &Knobs<'_>, p: &mut Painter<'_>, fine: bool) {
    let (w, h, t) = (k.size("w", 2.0), k.size("h", 0.25), k.size("t", 0.2));
    let merlon = k.size("merlon", 0.22) * if fine { 1.0 } else { 2.0 };
    for run in runs_of(k, w, t) {
        run.paint(p, |p, length| {
            let n = ((length + merlon) / (2.0 * merlon)).floor().max(1.0) as usize;
            for x in spread(length, merlon, n) {
                p.cuboid([x, 0.0, 0.0], [merlon, h, t], 1.0, false);
            }
        });
    }
}

/// `fence`: posts and rails along a run or round a yard.
fn fence(k: &Knobs<'_>, p: &mut Painter<'_>, fine: bool) {
    let (w, h) = (k.size("w", 2.0), k.size("h", 0.5));
    let spacing = k.size("spacing", 0.5) * if fine { 1.0 } else { 2.0 };
    let rails = k.count("rails", 2, 6);
    for run in runs_of(k, w, 0.06) {
        run.paint(p, |p, length| {
            let n = (length / spacing).ceil() as usize + 1;
            for x in spread(length, 0.07, n) {
                p.cuboid([x, 0.0, 0.0], [0.07, h, 0.07], 1.0, false);
            }
            for r in 0..rails {
                let y = h * (0.35 + 0.5 * r as f32 / rails.max(2).saturating_sub(1).max(1) as f32);
                p.cuboid([0.0, y - 0.025, 0.0], [length, 0.05, 0.04], 1.0, true);
            }
        });
    }
}

/// `palisade`: a wall of pointed stakes along a run or round a yard.
fn palisade(k: &Knobs<'_>, p: &mut Painter<'_>, fine: bool) {
    let (w, h, t) = (k.size("w", 2.0), k.size("h", 1.0), k.size("t", 0.18));
    let spacing = k.size("spacing", 0.24);
    for run in runs_of(k, w, t) {
        run.paint(p, |p, length| {
            if !fine {
                p.cuboid([0.0; 3], [length, h, t], 1.0, false);
                return;
            }
            let n = (length / spacing).round().max(1.0) as usize;
            let stake = length / n as f32;
            let tooth = (stake * 0.9).min(h * 0.4);
            let body = h - tooth;
            p.cuboid([0.0; 3], [length, body, t], 1.0, false);
            for i in 0..n {
                let x = -length / 2.0 + stake * (i as f32 + 0.5);
                let (l, r, z) = (x - stake / 2.0, x + stake / 2.0, t / 2.0);
                p.face(
                    &[[l, body, z], [r, body, z], [x, h, z * 0.3]],
                    [0.0, 0.3, 1.0],
                );
                p.face(
                    &[[r, body, -z], [l, body, -z], [x, h, -z * 0.3]],
                    [0.0, 0.3, -1.0],
                );
                p.face(
                    &[
                        [r, body, z],
                        [r, body, -z],
                        [x, h, -z * 0.3],
                        [x, h, z * 0.3],
                    ],
                    [1.0, 0.6, 0.0],
                );
                p.face(
                    &[
                        [l, body, -z],
                        [l, body, z],
                        [x, h, z * 0.3],
                        [x, h, -z * 0.3],
                    ],
                    [-1.0, 0.6, 0.0],
                );
            }
        });
    }
}

/// `banner`: a pole with a crossbar and a cloth hanging from it, notched at the bottom.
fn banner(k: &Knobs<'_>, p: &mut Painter<'_>, fine: bool) {
    let h = k.size("h", 2.6);
    let (fw, fh) = (k.size("fw", 0.6), k.size("fh", 1.0).min(h * 0.9));
    let pole = k.size("pole", 0.07);
    let notch = k.get("notch", 0.18).clamp(0.0, fh * 0.6);
    let cloth = p.surface;
    let wood = Surface::of(TIMBER, k.colour("pole_colour", [0.36, 0.25, 0.15]), 0.06);
    p.surface = wood;
    p.cuboid([0.0; 3], [pole, h, pole], 1.0, false);
    let bar = h - 0.12;
    if fine {
        p.cuboid([0.0, bar, 0.05], [fw + 0.12, 0.05, 0.05], 1.0, true);
        p.surface = Surface::of(METAL, [0.78, 0.62, 0.3], 0.2);
        p.cone([0.0, h, 0.0], 0.07, 0.16, 5);
    }
    p.surface = cloth;
    let strips = if fine { 4 } else { 1 };
    let column = |j: usize| {
        let u = j as f32 / strips as f32;
        let x = -fw / 2.0 + fw * u;
        let bottom = bar - fh + notch * (1.0 - (2.0 * u - 1.0).abs());
        let z = 0.09
            + if fine {
                0.035 * (j as f32 * 1.9).sin()
            } else {
                0.0
            };
        (x, bottom, z)
    };
    for j in 0..strips {
        let (x0, b0, z0) = column(j);
        let (x1, b1, z1) = column(j + 1);
        p.face(
            &[[x0, b0, z0], [x1, b1, z1], [x1, bar, z1], [x0, bar, z0]],
            [0.0, 0.0, 1.0],
        );
        let back = |z: f32| z - 0.025;
        p.face(
            &[
                [x1, b1, back(z1)],
                [x0, b0, back(z0)],
                [x0, bar, back(z0)],
                [x1, bar, back(z1)],
            ],
            [0.0, 0.0, -1.0],
        );
    }
    if fine {
        for (j, s) in [(0, -1.0), (strips, 1.0)] {
            let (x, b, z) = column(j);
            p.face(
                &[
                    [x, b, z],
                    [x, b, z - 0.025],
                    [x, bar, z - 0.025],
                    [x, bar, z],
                ],
                [s, 0.0, 0.0],
            );
        }
    }
}

/// `awning`: a cloth sheet sloping up to the back, on four posts.
fn awning(k: &Knobs<'_>, p: &mut Painter<'_>) {
    let (w, d, h) = (k.size("w", 2.0), k.size("d", 1.0), k.size("h", 1.0));
    let rise = k.get("rise", 0.35);
    let (x, z, t) = (w / 2.0, d / 2.0, 0.04);
    let top = [
        [-x, h + t, z],
        [x, h + t, z],
        [x, h + rise + t, -z],
        [-x, h + rise + t, -z],
    ];
    let low = top.map(|v| [v[0], v[1] - t, v[2]]);
    p.face(&top, [0.0, 1.0, rise / d]);
    p.face(&low, [0.0, -1.0, -rise / d]);
    p.face(&[low[0], low[1], top[1], top[0]], [0.0, 0.0, 1.0]);
    let cloth = p.surface;
    if k.get("posts", 1.0) > 0.5 {
        p.surface = Surface::of(TIMBER, k.colour("post_colour", [0.42, 0.30, 0.18]), 0.06);
        for (px, pz, ph) in [
            (x - 0.05, z - 0.05, h),
            (-x + 0.05, z - 0.05, h),
            (x - 0.05, -z + 0.05, h + rise),
            (-x + 0.05, -z + 0.05, h + rise),
        ] {
            p.cuboid([px, 0.0, pz], [0.06, ph, 0.06], 1.0, false);
        }
    }
    p.surface = cloth;
}

/// `arch`: a wall `w` wide, `h` high and `t` thick along x, pierced in the middle by an opening
/// `span` wide whose round head springs at `spring`.
fn arch(k: &Knobs<'_>, p: &mut Painter<'_>, fine: bool) {
    let (w, h, t) = (k.size("w", 2.0), k.size("h", 2.0), k.size("t", 0.4));
    let span = k.size("span", w * 0.5).min(w - 0.1).max(0.05);
    let r = span / 2.0;
    let spring = k
        .get("spring", h * 0.45)
        .clamp(0.0, (h - r - 0.05).max(0.0));
    let n = segments(k, 8, fine, 3);
    let (x, z) = (w / 2.0, t / 2.0);
    let arc: Vec<[f32; 2]> = (0..=n)
        .map(|i| {
            let a = PI * (1.0 - i as f32 / n as f32);
            [r * a.cos(), spring + r * a.sin()]
        })
        .collect();
    for s in [1.0_f32, -1.0] {
        let (zf, out) = (s * z, [0.0, 0.0, s]);
        p.face(
            &[[-x, 0.0, zf], [-r, 0.0, zf], [-r, h, zf], [-x, h, zf]],
            out,
        );
        p.face(&[[r, 0.0, zf], [x, 0.0, zf], [x, h, zf], [r, h, zf]], out);
        for pair in arc.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            p.face(
                &[
                    [a[0], a[1], zf],
                    [b[0], b[1], zf],
                    [b[0], h, zf],
                    [a[0], h, zf],
                ],
                out,
            );
        }
        // The outer end and the jamb of the opening on this side of x.
        p.face(
            &[
                [s * x, 0.0, -z],
                [s * x, 0.0, z],
                [s * x, h, z],
                [s * x, h, -z],
            ],
            [s, 0.0, 0.0],
        );
        p.face(
            &[
                [s * r, 0.0, -z],
                [s * r, 0.0, z],
                [s * r, spring, z],
                [s * r, spring, -z],
            ],
            [-s, 0.0, 0.0],
        );
    }
    for (i, pair) in arc.windows(2).enumerate() {
        let (a, b) = (pair[0], pair[1]);
        let mid = PI * (1.0 - (i as f32 + 0.5) / n as f32);
        p.face(
            &[
                [a[0], a[1], z],
                [b[0], b[1], z],
                [b[0], b[1], -z],
                [a[0], a[1], -z],
            ],
            [-mid.cos(), -mid.sin(), 0.0],
        );
    }
    p.face(
        &[[-x, h, -z], [x, h, -z], [x, h, z], [-x, h, z]],
        [0.0, 1.0, 0.0],
    );
}

/// `hull`: an open boat `w` long along x, `d` across and `h` deep amidships, its ends rising by
/// `sheer` to a point; planked inside and out in the near version, outside only far out. With
/// `float` it rides the water, carrying what is attached to it.
fn hull(k: &Knobs<'_>, p: &mut Painter<'_>, fine: bool) {
    let (w, d, h) = (k.size("w", 2.0), k.size("d", 0.7), k.size("h", 0.35));
    if k.get("float", 0.0) > 0.5 {
        p.joint = p.add_joint([0.0; 3], Motion::Float);
    }
    let sheer = k.get("sheer", h * 0.6).max(0.0);
    let n = segments(k, 8, fine, 4).max(2);
    // Keel, chine and gunwale of one side at each station along the hull.
    let station = |i: usize, s: f32| {
        let u = -1.0 + 2.0 * i as f32 / n as f32;
        let x = u * w / 2.0;
        let beam = d / 2.0 * (1.0 - u * u).max(0.0).sqrt();
        let top = h + sheer * u * u;
        let keel = (h + sheer) * u.abs().powi(3);
        let chine = keel + (top - keel) * 0.45;
        [
            [x, keel, 0.0],
            [x, chine, s * beam * 0.82],
            [x, top, s * beam],
        ]
    };
    for s in [1.0_f32, -1.0] {
        for i in 0..n {
            let (a, b) = (station(i, s), station(i + 1, s));
            for strake in 0..2 {
                let quad = [a[strake], b[strake], b[strake + 1], a[strake + 1]];
                let c = quad.iter().fold([0.0; 3], |m, q| {
                    [m[0] + q[0] / 4.0, m[1] + q[1] / 4.0, m[2] + q[2] / 4.0]
                });
                let out = [c[0], c[1] - h * 0.7, c[2]];
                p.face(&quad, out);
                if fine {
                    p.face(&quad, out.map(|v| -v));
                }
            }
        }
    }
}

/// The motion knobs of a figure.
fn figure_style(k: &Knobs<'_>, h: f32) -> FigureStyle {
    let attack = match k.count("attack", 0, 4) {
        1 => AttackStyle::Thrust,
        2 => AttackStyle::Throw,
        3 => AttackStyle::Draw,
        4 => AttackStyle::Whirl,
        _ => AttackStyle::Strike,
    };
    let work = match k.count("work", 0, 1) {
        1 => WorkStyle::Raise,
        _ => WorkStyle::Tool,
    };
    FigureStyle {
        height: h,
        attack,
        work,
        hold: [
            k.get("hold_right", 0.0).clamp(-90.0, 120.0).to_radians(),
            k.get("hold_left", 0.0).clamp(-90.0, 120.0).to_radians(),
        ],
        stride: k.get("stride", 28.0).clamp(0.0, 60.0).to_radians(),
        seated: k.get("seat", 0.0) > 0.5,
        robe: k.get("robe", 0.0) > 0.5,
    }
}

/// `figure`: a standing person `h` tall facing +z, tunic in the part's surface, with sockets
/// at the head, both hands and the back. `robe` gives a long robe to the ground, `seat` sits
/// astride a mount; `attack`, `work`, `hold_left`, `hold_right`, `stride` and `pace` shape its
/// motion. Joints: the body (from the hips), both legs and both arms; what a hand holds turns
/// in the hand.
fn figure(k: &Knobs<'_>, p: &mut Painter<'_>, fine: bool) -> Vec<Socket> {
    let h = k.size("h", 1.0);
    let b = k.size("build", 1.0);
    let style = figure_style(k, h);
    let tunic = p.surface;
    let skin = Surface::plain(k.colour("skin", [0.78, 0.58, 0.42]));
    let hair = Surface::plain(k.colour("hair", [0.18, 0.12, 0.08]));
    let legs = Surface::plain(k.colour("legs", k.colour("skin", [0.78, 0.58, 0.42])));
    let belt = Surface::of(TIMBER, [0.32, 0.22, 0.12], 0.05);
    let attack_period = match style.attack {
        AttackStyle::Strike | AttackStyle::Thrust => 1.2,
        AttackStyle::Throw | AttackStyle::Whirl => 1.6,
        AttackStyle::Draw => 2.0,
    };
    p.suggest_period(Activity::Move, k.size("pace", 1.0));
    p.suggest_period(Activity::Attack, attack_period);
    p.suggest_period(Activity::Work, 1.3);
    // Joints, right side first (the figure's right is its −x side).
    let body = p.add_joint([0.0, 0.4 * h, 0.0], Motion::Body(style));
    p.joint = body;
    let leg = [0, 1].map(|side| {
        let s = if side == 0 { -1.0 } else { 1.0 };
        p.add_joint([s * 0.06 * b * h, 0.4 * h, 0.0], Motion::Leg(style, side))
    });
    let arm = [0, 1].map(|side| {
        let s = if side == 0 { -1.0 } else { 1.0 };
        p.add_joint([s * 0.205 * b * h, 0.72 * h, 0.0], Motion::Arm(style, side))
    });
    let side_of = |s: f32| usize::from(s > 0.0);
    // A seated leg hangs from the hip, splayed to straddle the mount and bent forward.
    let seat = |s: f32| Place {
        at: [s * 0.06 * b, 0.4, 0.0],
        tilt: tilt(-28.0, -s * 26.0),
        ..Place::IDENTITY
    };
    p.within(
        Place {
            scale: h,
            ..Place::IDENTITY
        },
        None,
        |p| {
            if fine {
                for s in [-1.0, 1.0] {
                    p.joint = leg[side_of(s)];
                    if style.seated {
                        p.surface = legs;
                        p.within(seat(s), None, |p| {
                            p.cuboid([0.0, -0.4, 0.0], [0.1 * b, 0.4, 0.12], 1.1, false);
                            p.surface = hair;
                            p.cuboid([0.0, -0.4, 0.025], [0.11 * b, 0.05, 0.17], 1.0, false);
                        });
                    } else if !style.robe {
                        p.surface = legs;
                        p.cuboid([s * 0.06 * b, 0.0, 0.0], [0.1 * b, 0.4, 0.12], 1.1, false);
                    }
                }
                if !style.seated {
                    p.surface = hair;
                    for s in [-1.0, 1.0] {
                        p.joint = leg[side_of(s)];
                        let z = if style.robe { 0.07 } else { 0.025 };
                        p.cuboid([s * 0.06 * b, 0.0, z], [0.11 * b, 0.05, 0.17], 1.0, false);
                    }
                }
                p.joint = body;
                p.surface = tunic;
                if style.robe {
                    p.cuboid([0.0; 3], [0.42 * b, 0.56, 0.28], 0.72, false);
                } else {
                    p.cuboid([0.0, 0.3, 0.0], [0.36 * b, 0.22, 0.24], 0.82, false);
                }
                p.cuboid([0.0, 0.5, 0.0], [0.3 * b, 0.26, 0.2], 1.12, false);
                p.surface = belt;
                p.cuboid([0.0, 0.48, 0.0], [0.32 * b, 0.045, 0.215], 1.0, false);
                for s in [-1.0, 1.0] {
                    let x = s * 0.205 * b;
                    p.joint = arm[side_of(s)];
                    p.surface = tunic;
                    p.cuboid([x, 0.62, 0.0], [0.085 * b, 0.14, 0.1], 1.0, true);
                    p.surface = skin;
                    p.cuboid([x, 0.43, 0.01], [0.07 * b, 0.19, 0.08], 1.0, true);
                }
                p.joint = body;
                p.cuboid([0.0, 0.76, 0.0], [0.07, 0.05, 0.07], 1.0, false);
                p.sphere_band([0.0, 0.87, 0.0], 0.1, 0.11, (-FRAC_PI_2, FRAC_PI_2), 4, 8);
                p.surface = hair;
                p.sphere_band([0.0, 0.878, -0.015], 0.108, 0.108, (0.05, FRAC_PI_2), 2, 8);
            } else {
                p.surface = legs;
                if style.seated {
                    for s in [-1.0, 1.0] {
                        p.joint = leg[side_of(s)];
                        p.within(seat(s), None, |p| {
                            p.cuboid([0.0, -0.4, 0.0], [0.1 * b, 0.4, 0.12], 1.0, false);
                        });
                    }
                    p.joint = body;
                    p.surface = tunic;
                    p.cuboid([0.0, 0.3, 0.0], [0.34 * b, 0.46, 0.22], 1.0, false);
                } else if style.robe {
                    p.joint = body;
                    p.surface = tunic;
                    p.cuboid([0.0; 3], [0.4 * b, 0.76, 0.26], 0.75, false);
                } else {
                    p.joint = body;
                    p.cuboid([0.0; 3], [0.22 * b, 0.4, 0.12], 1.0, false);
                    p.surface = tunic;
                    p.cuboid([0.0, 0.3, 0.0], [0.34 * b, 0.46, 0.22], 1.0, false);
                }
                p.surface = skin;
                for s in [-1.0, 1.0] {
                    p.joint = arm[side_of(s)];
                    p.cuboid(
                        [s * 0.205 * b, 0.43, 0.0],
                        [0.08 * b, 0.33, 0.09],
                        1.0,
                        false,
                    );
                }
                p.joint = body;
                p.cuboid([0.0, 0.77, 0.0], [0.19, 0.2, 0.19], 1.0, false);
            }
        },
    );
    p.surface = tunic;
    // Parts attached to the figure itself move with its body.
    p.joint = body;
    let at = |x: f32, y: f32, z: f32| Place::at([x * h, y * h, z * h]);
    let socket = |name, place, joint, grip| Socket {
        name,
        place,
        joint,
        grip,
    };
    vec![
        socket("head", at(0.0, 0.98, 0.0), body, None),
        // Facing +z with y up, the figure's right hand is on its −x side.
        socket(
            "right_hand",
            at(-0.205 * b, 0.45, 0.03),
            arm[0],
            Some(Motion::Grip(style, 0)),
        ),
        socket(
            "left_hand",
            at(0.205 * b, 0.45, 0.03),
            arm[1],
            Some(Motion::Grip(style, 1)),
        ),
        socket("back", at(0.0, 0.62, -0.13), body, None),
    ]
}

/// `quadruped`: a horse-like beast standing along z (its head to +z), `h` its size factor (1:
/// a horse, its back 0.8 tiles up), `l` and `w` the length and width of its body; hide in the
/// part's surface, colours `mane` and `hoof`. `pace` and `stride` shape its gait. Sockets:
/// `rider` (the saddle), `back` (behind it, for loads), `head`, `chest`. Joints: the body, four
/// legs, the neck with the head, the tail.
fn quadruped(k: &Knobs<'_>, p: &mut Painter<'_>, fine: bool) -> Vec<Socket> {
    let s = k.size("h", 1.0);
    let (l, w) = (k.size("l", 0.95), k.size("w", 0.3));
    let style = BeastStyle {
        height: s,
        stride: k.get("stride", 30.0).clamp(0.0, 60.0).to_radians(),
    };
    p.suggest_period(Activity::Move, k.size("pace", 0.7));
    let hide = p.surface;
    let mane = Surface::plain(k.colour("mane", [0.24, 0.17, 0.12]));
    let hoof = Surface::plain(k.colour("hoof", [0.16, 0.13, 0.11]));
    let (leg_h, body_h) = (0.52, 0.3);
    let (lx, lz) = (w / 2.0 - 0.055, l / 2.0 - 0.11);
    let neck_at = [0.0, leg_h + body_h * 0.55, l / 2.0 - 0.1];
    let tail_at = [0.0, leg_h + body_h * 0.92, -l / 2.0 + 0.02];
    let scaled = |v: [f32; 3]| v.map(|c| c * s);
    let body = p.add_joint(
        scaled([0.0, leg_h + body_h / 2.0, 0.0]),
        Motion::Beast(style),
    );
    p.joint = body;
    // Legs: front right, front left, back right, back left.
    let legs = [(true, 0_usize), (true, 1), (false, 0), (false, 1)].map(|(front, side)| {
        let x = if side == 0 { -lx } else { lx };
        let z = if front { lz } else { -lz };
        let joint = p.add_joint(
            scaled([x, leg_h + 0.04, z]),
            Motion::BeastLeg(style, front, side),
        );
        (joint, x, z)
    });
    let neck = p.add_joint(scaled(neck_at), Motion::Neck);
    let tail = p.add_joint(scaled(tail_at), Motion::Tail);
    let neck_place = Place {
        at: neck_at,
        tilt: tilt(38.0, 0.0),
        ..Place::IDENTITY
    };
    let head_place = Place {
        at: [0.0, 0.44, 0.02],
        tilt: tilt(80.0, 0.0),
        ..Place::IDENTITY
    };
    p.within(
        Place {
            scale: s,
            ..Place::IDENTITY
        },
        None,
        |p| {
            p.joint = body;
            p.cuboid([0.0, leg_h, 0.0], [w, body_h, l], 0.9, true);
            p.joint = neck;
            p.within(neck_place, None, |p| {
                p.cuboid([0.0; 3], [w * 0.5, 0.5, 0.22], 0.75, false);
                p.within(head_place, None, |p| {
                    p.cuboid([0.0, -0.06, 0.0], [w * 0.48, 0.38, 0.17], 0.7, true);
                    if fine {
                        // Ears stand up from the poll (the head's −z side).
                        for e in [-1.0, 1.0] {
                            p.within(
                                Place {
                                    at: [e * 0.04, 0.0, -0.07],
                                    tilt: tilt(-90.0, 0.0),
                                    ..Place::IDENTITY
                                },
                                None,
                                |p| p.cone([0.0; 3], 0.025, 0.09, 4),
                            );
                        }
                    }
                });
                if fine {
                    p.surface = mane;
                    p.cuboid([0.0, 0.04, -0.11], [0.04, 0.46, 0.05], 1.0, false);
                }
            });
            p.surface = mane;
            p.joint = tail;
            p.within(
                Place {
                    at: tail_at,
                    tilt: tilt(-150.0, 0.0),
                    ..Place::IDENTITY
                },
                None,
                |p| p.cuboid([0.0; 3], [0.07, 0.46, 0.08], 0.55, true),
            );
            p.surface = hide;
            if fine {
                for &(joint, x, z) in &legs {
                    p.joint = joint;
                    p.surface = hide;
                    p.cuboid([x, 0.06, z], [0.075, leg_h, 0.09], 1.35, false);
                    p.surface = hoof;
                    p.cuboid([x, 0.0, z], [0.085, 0.07, 0.1], 0.95, false);
                }
            } else {
                p.joint = body;
                for z in [lz, -lz] {
                    p.cuboid(
                        [0.0, 0.0, z],
                        [2.0 * lx + 0.08, leg_h + 0.04, 0.09],
                        1.0,
                        false,
                    );
                }
            }
        },
    );
    p.surface = hide;
    p.joint = body;
    let at = |v: [f32; 3]| Place::at(scaled(v));
    let socket = |name, place, joint| Socket {
        name,
        place,
        joint,
        grip: None,
    };
    vec![
        socket("rider", at([0.0, leg_h + body_h, 0.04]), body),
        socket("back", at([0.0, leg_h + body_h, -0.24]), body),
        socket("head", at([0.0, 1.1, l / 2.0 + 0.32]), neck),
        socket("chest", at([0.0, leg_h + body_h / 2.0, l / 2.0]), body),
    ]
}

/// `wheel`: an upright wheel of diameter `w` and thickness `t` on an axle along x, resting on
/// the ground; `spokes` bars cross its outer faces. It turns as its unit moves.
fn wheel(k: &Knobs<'_>, p: &mut Painter<'_>, fine: bool) {
    let r = k.size("w", 0.6) / 2.0;
    let t = k.size("t", 0.08);
    let spokes = k.count("spokes", 2, 6);
    let n = segments(k, 10, fine, 5);
    p.joint = p.add_joint([0.0, r, 0.0], Motion::Wheel);
    let rim = |x: f32| -> Vec<[f32; 3]> {
        (0..n)
            .map(|i| {
                let a = i as f32 / n as f32 * TAU;
                [x, r + r * a.cos(), r * a.sin()]
            })
            .collect()
    };
    let (outer, inner) = (rim(t / 2.0), rim(-t / 2.0));
    p.face(&outer, [1.0, 0.0, 0.0]);
    p.face(&inner, [-1.0, 0.0, 0.0]);
    for i in 0..n {
        let j = (i + 1) % n;
        let mid = (i as f32 + 0.5) / n as f32 * TAU;
        p.face(
            &[outer[i], outer[j], inner[j], inner[i]],
            [0.0, mid.cos(), mid.sin()],
        );
    }
    if fine {
        let wheel = p.surface;
        p.surface.colour = wheel.colour.map(|c| c * 0.62);
        p.surface.alt = wheel.alt.map(|c| c * 0.62);
        for i in 0..spokes {
            let turn = 180.0 * i as f32 / spokes.max(1) as f32;
            p.within(
                Place {
                    at: [0.0, r, 0.0],
                    tilt: tilt(turn, 0.0),
                    ..Place::IDENTITY
                },
                None,
                |p| p.cuboid([0.0, -r * 0.88, 0.0], [t + 0.03, r * 1.76, 0.05], 1.0, true),
            );
        }
        p.surface = wheel;
    }
}

/// `oar_bank`: `count` oars a side along `w` (x), their locks `beam` apart across z, each
/// `length` long and sloping `drop` degrees down into the water. Each oar sweeps on its own
/// joint while its unit moves; far out each side is one plank.
fn oar_bank(k: &Knobs<'_>, p: &mut Painter<'_>, fine: bool) {
    let w = k.size("w", 2.4);
    let count = k.count("count", 6, 12).max(1);
    let beam = k.size("beam", 0.8);
    let length = k.size("length", 1.0);
    let drop = k.get("drop", 25.0).clamp(0.0, 80.0);
    p.suggest_period(Activity::Move, 2.2);
    p.suggest_period(Activity::Attack, 1.6);
    let base = p.joint;
    for side in [1.0_f32, -1.0] {
        let lean = side * (90.0 + drop);
        if !fine {
            p.joint = base;
            p.within(
                Place {
                    at: [0.0, 0.0, side * beam / 2.0],
                    tilt: tilt(lean, 0.0),
                    ..Place::IDENTITY
                },
                None,
                |p| p.cuboid([0.0; 3], [w, length * 0.85, 0.03], 1.0, true),
            );
        }
        for i in 0..count {
            let x = -w / 2.0 + w * (i as f32 + 0.5) / count as f32;
            let lock = [x, 0.0, side * beam / 2.0];
            let joint = {
                p.joint = base;
                p.add_joint(lock, Motion::Oar(side))
            };
            if fine {
                p.joint = joint;
                p.within(
                    Place {
                        at: lock,
                        tilt: tilt(lean, 0.0),
                        ..Place::IDENTITY
                    },
                    None,
                    |p| {
                        p.cuboid([0.0, -0.12, 0.0], [0.035, length, 0.035], 1.0, false);
                        p.cuboid([0.0, length - 0.42, 0.0], [0.12, 0.3, 0.02], 1.0, true);
                    },
                );
            }
        }
    }
    p.joint = base;
}

/// `sail`: a square sail `w` wide and `h` high above its foot, bellied `billow` towards +z,
/// hanging from a yard (colour `yard_colour`); the cloth is the part's surface.
fn sail(k: &Knobs<'_>, p: &mut Painter<'_>, fine: bool) {
    let (w, h) = (k.size("w", 1.4), k.size("h", 1.1));
    let billow = k.get("billow", 0.15);
    let cells = if fine { 2 } else { 1 };
    let point = |i: usize, j: usize, back: f32| {
        let (u, v) = (i as f32 / cells as f32, j as f32 / cells as f32);
        let x = (u - 0.5) * w;
        let bulge = billow * (1.0 - (2.0 * u - 1.0).powi(2)) * (0.55 + 0.45 * (PI * v).sin());
        [x, v * h, if fine { bulge } else { billow * 0.6 } - back]
    };
    for i in 0..cells {
        for j in 0..cells {
            let front = [
                point(i, j, 0.0),
                point(i + 1, j, 0.0),
                point(i + 1, j + 1, 0.0),
                point(i, j + 1, 0.0),
            ];
            p.face(&front, [0.0, 0.0, 1.0]);
            let back = front.map(|v| [v[0], v[1], v[2] - 0.02]);
            p.face(&back, [0.0, 0.0, -1.0]);
        }
    }
    if fine {
        let cloth = p.surface;
        p.surface = Surface::of(TIMBER, k.colour("yard_colour", [0.42, 0.29, 0.17]), 0.08);
        p.cuboid([0.0, h - 0.03, 0.0], [w + 0.24, 0.07, 0.07], 1.0, true);
        p.surface = cloth;
    }
}

/// `shield`: a round shield `w` across (or, with `h`, a rectangle `w` by `h`) and `t` thick,
/// centred on its origin and facing +z; the face is the part's surface, the rim and `boss` are
/// colour `rim`.
fn shield(k: &Knobs<'_>, p: &mut Painter<'_>, fine: bool) {
    let w = k.size("w", 0.5);
    let tall = k.get("h", 0.0);
    let t = k.size("t", 0.04);
    let boss = k.get("boss", w * 0.22).max(0.0);
    let face = p.surface;
    let rim = Surface::of(METAL, k.colour("rim", [0.72, 0.55, 0.3]), 0.2);
    let outline: Vec<[f32; 2]> = if tall > 0.0 {
        let (x, y) = (w / 2.0, tall / 2.0);
        vec![[-x, -y], [x, -y], [x, y], [-x, y]]
    } else {
        let n = segments(k, 10, fine, 5);
        (0..n)
            .map(|i| {
                let a = i as f32 / n as f32 * TAU;
                [w / 2.0 * a.cos(), w / 2.0 * a.sin()]
            })
            .collect()
    };
    let at = |v: [f32; 2], z: f32| [v[0], v[1], z];
    let front: Vec<[f32; 3]> = outline.iter().map(|v| at(*v, t / 2.0)).collect();
    let back: Vec<[f32; 3]> = outline.iter().map(|v| at(*v, -t / 2.0)).collect();
    p.face(&front, [0.0, 0.0, 1.0]);
    p.surface = rim;
    p.face(&back, [0.0, 0.0, -1.0]);
    if fine {
        let n = outline.len();
        for i in 0..n {
            let j = (i + 1) % n;
            let out = [
                outline[i][0] + outline[j][0],
                outline[i][1] + outline[j][1],
                0.0,
            ];
            p.face(&[front[i], back[i], back[j], front[j]], out);
        }
        if boss > 0.0 {
            p.within(
                Place {
                    at: [0.0, 0.0, t / 2.0],
                    tilt: tilt(90.0, 0.0),
                    ..Place::IDENTITY
                },
                None,
                |p| p.cone([0.0; 3], boss / 2.0, boss * 0.45, 6),
            );
        }
    }
    p.surface = face;
}

/// `bow`: a stave `h` long and `w` thick held at its middle (the origin), bent `bend` towards
/// +z, its string straight between the tips (colour `string`).
fn bow(k: &Knobs<'_>, p: &mut Painter<'_>, fine: bool) {
    let h = k.size("h", 0.8);
    let bend = k.get("bend", 0.14);
    let t = k.size("w", 0.035) / 2.0;
    let n = if fine { 4 } else { 2 };
    let arc: Vec<[f32; 2]> = (0..=n)
        .map(|i| {
            let y = -h / 2.0 + h * i as f32 / n as f32;
            [y, bend * (1.0 - (2.0 * y / h).powi(2))]
        })
        .collect();
    for pair in arc.windows(2) {
        let ([y0, z0], [y1, z1]) = (pair[0], pair[1]);
        // Across the stave in the y–z plane, towards +z.
        let (dy, dz) = (y1 - y0, z1 - z0);
        let len = dy.hypot(dz).max(1e-6);
        let (ny, nz) = (-dz / len * t, dy / len * t);
        let c = |y: f32, z: f32, x: f32, s: f32| [x, y + s * ny, z + s * nz];
        for (sx, ss) in [(1.0_f32, 1.0_f32), (-1.0, -1.0)] {
            // The faces towards ±z (belly and back) and towards ±x.
            p.face(
                &[
                    c(y0, z0, -t, ss),
                    c(y0, z0, t, ss),
                    c(y1, z1, t, ss),
                    c(y1, z1, -t, ss),
                ],
                [0.0, ss * ny, ss * nz],
            );
            p.face(
                &[
                    c(y0, z0, sx * t, -1.0),
                    c(y0, z0, sx * t, 1.0),
                    c(y1, z1, sx * t, 1.0),
                    c(y1, z1, sx * t, -1.0),
                ],
                [sx, 0.0, 0.0],
            );
        }
    }
    if fine {
        let stave = p.surface;
        p.surface = Surface::plain(k.colour("string", [0.85, 0.82, 0.72]));
        let s = 0.006;
        let (lo, hi) = (-h / 2.0, h / 2.0);
        p.face(
            &[[-s, lo, 0.0], [s, lo, 0.0], [s, hi, 0.0], [-s, hi, 0.0]],
            [0.0, 0.0, 1.0],
        );
        p.face(
            &[[s, lo, 0.0], [-s, lo, 0.0], [-s, hi, 0.0], [s, hi, 0.0]],
            [0.0, 0.0, -1.0],
        );
        p.surface = stave;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::looks::{MaterialLook, Program};
    use crate::models::{ModelSpec, bake_model};

    fn part(shape: Shape, knobs: &[(&str, f32)]) -> PartSpec {
        let material = MaterialLook {
            program: Program::Plaster,
            colour: [0.7; 3],
            colour_alt: [0.5; 3],
            scale: 0.5,
        };
        let mut part = PartSpec::new("p", shape, material);
        for (k, v) in knobs {
            part = part.knob(k, *v);
        }
        part
    }

    fn close(a: [f32; 3], b: [f32; 3]) -> bool {
        (0..3).all(|i| (a[i] - b[i]).abs() < 1e-3)
    }

    /// Bakes one part and checks its near triangles and box.
    fn check(shape: Shape, knobs: &[(&str, f32)], triangles: usize, lo: [f32; 3], hi: [f32; 3]) {
        let baked = bake_model(&ModelSpec {
            parts: vec![part(shape, knobs)],
        });
        let [l, h] = baked.bounds();
        assert_eq!(baked.near.triangles(), triangles, "{shape:?} {knobs:?}");
        assert!(close(l, lo) && close(h, hi), "{shape:?}: {l:?} {h:?}");
        assert!(baked.far.triangles() <= baked.near.triangles(), "{shape:?}");
        assert!(baked.far.triangles() > 0, "{shape:?}");
        // Every normal is unit length and points away from the faces' winding.
        for t in baked.near.indices.chunks(3) {
            let [a, b, c] = [t[0], t[1], t[2]].map(|i| baked.near.positions[usize::from(i)]);
            let n = baked.near.normals[usize::from(t[0])];
            let cross = [
                (b[1] - a[1]) * (c[2] - a[2]) - (b[2] - a[2]) * (c[1] - a[1]),
                (b[2] - a[2]) * (c[0] - a[0]) - (b[0] - a[0]) * (c[2] - a[2]),
                (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]),
            ];
            assert!(dot(cross, n) > -1e-6, "{shape:?}: wound against its normal");
            assert!((dot(n, n) - 1.0).abs() < 1e-4);
        }
    }

    #[test]
    fn block_shapes_have_their_bounds_and_triangles() {
        check(Shape::Box, &[], 10, [-0.5, 0.0, -0.5], [0.5, 1.0, 0.5]);
        check(
            Shape::Box,
            &[("w", 2.0), ("gable", 45.0)],
            14,
            [-1.0, 0.0, -0.5],
            [1.0, 1.5, 0.5],
        );
        // A door with its lintel and four windows with sills; the sills stand out 6 cm.
        check(
            Shape::Box,
            &[("w", 2.0), ("h", 1.5), ("door", 0.4), ("windows", 2.0)],
            10 + 12 + 4 * 12,
            [-1.0, 0.0, -0.56],
            [1.0, 1.5, 0.56],
        );
        check(
            Shape::Plinth,
            &[("w", 3.0), ("d", 2.0), ("h", 0.3), ("steps", 3.0)],
            30,
            [-1.5, 0.0, -1.0],
            [1.5, 0.3, 1.0],
        );
        check(
            Shape::Wall,
            &[("w", 2.0), ("h", 1.5), ("t", 0.2)],
            10,
            [-1.0, 0.0, -0.1],
            [1.0, 1.5, 0.1],
        );
        check(
            Shape::Wall,
            &[
                ("w", 4.0),
                ("d", 3.0),
                ("h", 1.0),
                ("t", 0.2),
                ("ring", 1.0),
                ("gap", 1.0),
            ],
            50,
            [-2.0, 0.0, -1.5],
            [2.0, 1.0, 1.5],
        );
    }

    #[test]
    fn roof_shapes_have_their_bounds_and_triangles() {
        let tan = 45_f32.to_radians().tan();
        let lift = 0.1 / 45_f32.to_radians().cos();
        check(
            Shape::GableRoof,
            &[
                ("w", 2.0),
                ("d", 2.0),
                ("pitch", 45.0),
                ("overhang", 0.2),
                ("t", 0.1),
            ],
            20,
            [-1.2, -0.2 * tan, -1.2],
            [1.2, 1.0 + lift, 1.2],
        );
        check(
            Shape::HipRoof,
            &[
                ("w", 3.0),
                ("d", 2.0),
                ("pitch", 45.0),
                ("overhang", 0.0),
                ("t", 0.1),
            ],
            16,
            [-1.5, -0.1, -1.0],
            [1.5, 1.0, 1.0],
        );
        check(
            Shape::FlatRoof,
            &[("w", 2.0), ("d", 2.0), ("overhang", 0.0)],
            12,
            [-1.0, 0.0, -1.0],
            [1.0, 0.12, 1.0],
        );
        check(
            Shape::FlatRoof,
            &[("w", 2.0), ("d", 2.0), ("overhang", 0.0), ("parapet", 0.3)],
            12 + 40,
            [-1.0, 0.0, -1.0],
            [1.0, 0.42, 1.0],
        );
    }

    #[test]
    fn round_shapes_have_their_bounds_and_triangles() {
        check(
            Shape::Dome,
            &[("w", 2.0), ("segments", 12.0)],
            12 * 3 * 2 - 12,
            [-1.0, 0.0, -1.0],
            [1.0, 1.0, 1.0],
        );
        check(
            Shape::Cylinder,
            &[("w", 1.0), ("h", 2.0), ("segments", 4.0)],
            4 * 2 + 2,
            [-0.5, 0.0, -0.5],
            [0.5, 2.0, 0.5],
        );
        check(
            Shape::Cone,
            &[("w", 1.0), ("h", 2.0), ("segments", 4.0)],
            4,
            [-0.5, 0.0, -0.5],
            [0.5, 2.0, 0.5],
        );
        check(
            Shape::ColumnRow,
            &[
                ("count", 3.0),
                ("radius", 0.1),
                ("spacing", 1.0),
                ("h", 2.0),
                ("segments", 4.0),
            ],
            3 * (10 + 8 + 10),
            [-1.14, 0.0, -0.14],
            [1.14, 2.0, 0.14],
        );
    }

    #[test]
    fn linear_shapes_have_their_bounds_and_triangles() {
        // Merlons 0.25 wide along 2.25 tiles: five of them, both ends flush.
        check(
            Shape::Crenellation,
            &[("w", 2.25), ("h", 0.3), ("t", 0.2), ("merlon", 0.25)],
            50,
            [-1.125, 0.0, -0.1],
            [1.125, 0.3, 0.1],
        );
        // Five posts and two rails (closed below) along two tiles.
        check(
            Shape::Fence,
            &[("w", 2.0), ("h", 0.6)],
            5 * 10 + 2 * 12,
            [-1.0, 0.0, -0.035],
            [1.0, 0.6, 0.035],
        );
        // A body and eight stakes along two tiles.
        check(
            Shape::Palisade,
            &[("w", 2.0), ("h", 1.2), ("spacing", 0.25)],
            10 + 8 * 6,
            [-1.0, 0.0, -0.09],
            [1.0, 1.2, 0.09],
        );
    }

    #[test]
    fn dressing_shapes_have_their_bounds_and_triangles() {
        check(
            Shape::Banner,
            &[("h", 3.0), ("fw", 0.6), ("fh", 1.0)],
            10 + 12 + 5 + 4 * 4 + 4,
            [-0.36, 0.0, -0.0666],
            [0.36, 3.16, 0.1239],
        );
        check(
            Shape::Awning,
            &[("w", 2.0), ("d", 1.0), ("h", 1.0), ("rise", 0.4)],
            6 + 4 * 10,
            [-1.0, 0.0, -0.5],
            [1.0, 1.44, 0.5],
        );
    }

    #[test]
    fn arches_have_their_bounds_and_triangles() {
        // Two piers and eight spandrels on each face, eight intrados panels, two ends, two
        // jambs and the top.
        check(
            Shape::Arch,
            &[
                ("w", 3.0),
                ("h", 2.5),
                ("t", 0.5),
                ("span", 1.2),
                ("spring", 1.0),
            ],
            2 * (4 + 2 * 8) + 2 * 8 + 4 + 4 + 2,
            [-1.5, 0.0, -0.25],
            [1.5, 2.5, 0.25],
        );
        // The far version has half the segments; an arch springing from the ground has no jambs.
        let far = bake_model(&ModelSpec {
            parts: vec![part(Shape::Arch, &[("spring", 0.0), ("segments", 6.0)])],
        });
        assert_eq!(far.near.triangles(), 2 * (4 + 2 * 6) + 2 * 6 + 4 + 2);
        assert_eq!(far.far.triangles(), 2 * (4 + 2 * 3) + 2 * 3 + 4 + 2);
    }

    #[test]
    fn hulls_have_their_bounds_and_triangles() {
        // Eight stations: two strakes a side, inside and out, the end panels triangles.
        check(
            Shape::Hull,
            &[("w", 2.0), ("d", 0.6), ("h", 0.3), ("sheer", 0.2)],
            2 * 2 * 2 * (8 * 2 - 2),
            [-1.0, 0.0, -0.3],
            [1.0, 0.5, 0.3],
        );
        let far = bake_model(&ModelSpec {
            parts: vec![part(Shape::Hull, &[])],
        });
        assert_eq!(far.far.triangles(), 2 * 2 * (4 * 2 - 2));
    }

    #[test]
    fn leaning_cylinders_close_their_foot() {
        let log = |extra: &[(&str, f32)]| {
            let knobs: Vec<(&str, f32)> = [("w", 0.3), ("h", 1.5), ("segments", 6.0)]
                .iter()
                .chain(extra)
                .copied()
                .collect();
            bake_model(&ModelSpec {
                parts: vec![part(Shape::Cylinder, &knobs)],
            })
            .near
            .triangles()
        };
        assert_eq!(log(&[]), 12 + 4);
        assert_eq!(log(&[("roll", 90.0)]), 12 + 4 + 4);
        assert_eq!(log(&[("roll", 90.0), ("bottom", 0.0)]), 12 + 4);
        assert_eq!(log(&[("bottom", 1.0)]), 12 + 4 + 4);
    }

    #[test]
    fn pitch_and_roll_lean_a_part_about_its_base() {
        // A post two tiles long rolled flat lies along +x; pitched flat it lies along +z.
        let post = [("w", 0.4), ("h", 2.0), ("segments", 4.0)];
        let lying = |extra: &[(&str, f32)]| {
            let knobs: Vec<(&str, f32)> = post.iter().chain(extra).copied().collect();
            bake_model(&ModelSpec {
                parts: vec![part(Shape::Cylinder, &knobs)],
            })
        };
        let [lo, hi] = lying(&[("roll", 90.0), ("y", 0.2)]).bounds();
        assert!(
            close(lo, [0.0, 0.0, -0.2]) && close(hi, [2.0, 0.4, 0.2]),
            "{lo:?} {hi:?}"
        );
        let [lo, hi] = lying(&[("pitch", 90.0), ("y", 0.2)]).bounds();
        assert!(
            close(lo, [-0.2, 0.0, 0.0]) && close(hi, [0.2, 0.4, 2.0]),
            "{lo:?} {hi:?}"
        );
        // A lean of 30 degrees towards the front, then a quarter turn: the top ends up east.
        let [lo, hi] = lying(&[("pitch", 30.0), ("yaw", 90.0)]).bounds();
        let (s, c) = 30_f32.to_radians().sin_cos();
        assert!((hi[0] - (2.0 * s + 0.2 * c)).abs() < 1e-3, "{hi:?}");
        assert!(
            (hi[1] - (2.0 * c + 0.2 * s)).abs() < 1e-3 && lo[1] < 0.0,
            "{lo:?} {hi:?}"
        );
        // Leaning keeps every normal unit length and adds only the closed foot.
        let leaning = lying(&[("pitch", 25.0), ("roll", -40.0)]);
        assert_eq!(
            leaning.near.triangles(),
            lying(&[("bottom", 1.0)]).near.triangles()
        );
        assert!(
            leaning
                .near
                .normals
                .iter()
                .all(|n| (dot(*n, *n) - 1.0).abs() < 1e-4)
        );
    }

    #[test]
    fn zero_lean_bakes_exactly_the_upright_part() {
        let knobs = [
            ("w", 1.3),
            ("d", 0.9),
            ("h", 1.1),
            ("gable", 30.0),
            ("yaw", 37.0),
        ];
        let upright = bake_model(&ModelSpec {
            parts: vec![part(Shape::Box, &knobs)],
        });
        let zeroed: Vec<(&str, f32)> = knobs
            .iter()
            .copied()
            .chain([("pitch", 0.0), ("roll", 0.0)])
            .collect();
        let zero = bake_model(&ModelSpec {
            parts: vec![part(Shape::Box, &zeroed)],
        });
        assert_eq!(upright, zero);
        assert_eq!(tilt(0.0, 0.0), None);
    }

    #[test]
    fn leaning_parents_carry_their_lean_to_children() {
        // A child turned a quarter inside a parent rolled flat: same as one combined matrix.
        let parent = Place {
            tilt: tilt(0.0, 90.0),
            ..Place::at([1.0, 0.0, 0.0])
        };
        let child = Place::turned([0.0, 1.0, 0.0], FRAC_PI_2);
        let p = parent.then(&child).apply([0.0, 1.0, 1.0]);
        let direct = parent.apply(child.apply([0.0, 1.0, 1.0]));
        assert!(close(p, direct), "{p:?} {direct:?}");
        // Upright places still add their turns.
        let upright = Place::turned([0.0; 3], 0.3).then(&Place::turned([0.0; 3], 0.4));
        assert_eq!(upright.tilt, None);
        assert!((upright.yaw - 0.7).abs() < 1e-6);
    }

    #[test]
    fn figures_stand_one_tile_tall_within_budget() {
        let baked = bake_model(&ModelSpec {
            parts: vec![part(Shape::Figure, &[])],
        });
        let [lo, hi] = baked.bounds();
        assert!((hi[1] - 0.98).abs() < 0.01, "{hi:?}");
        assert!(lo[1].abs() < 1e-5);
        assert!(hi[0] - lo[0] < 0.6 && hi[2] - lo[2] < 0.35, "{lo:?} {hi:?}");
        assert!(baked.near.triangles() <= 300, "{}", baked.near.triangles());
        assert!(baked.far.triangles() <= 80, "{}", baked.far.triangles());
        let tall = bake_model(&ModelSpec {
            parts: vec![part(Shape::Figure, &[("h", 1.4)])],
        });
        assert!((tall.bounds()[1][1] - 0.98 * 1.4).abs() < 0.01);
    }

    #[test]
    fn unit_shapes_have_their_bounds_and_triangles() {
        // A disc of ten sides (two faces and the tread) and two bars across it.
        let s72 = 72_f32.to_radians().sin();
        check(
            Shape::Wheel,
            &[],
            2 * 8 + 2 * 10 + 2 * 12,
            [-0.055, 0.0, -0.3 * s72],
            [0.055, 0.6, 0.3 * s72],
        );
        // A round face, a back, the rim and a boss of six sides.
        check(
            Shape::Shield,
            &[],
            8 + 8 + 20 + 6,
            [-0.25, -0.25 * s72, -0.02],
            [0.25, 0.25 * s72, 0.02 + 0.11 * 0.45],
        );
        check(
            Shape::Shield,
            &[("h", 0.8)],
            2 + 2 + 8 + 6,
            [-0.25, -0.4, -0.02],
            [0.25, 0.4, 0.0695],
        );
        // Four segments of stave, four sides each, and the string.
        let bow = bake_model(&ModelSpec {
            parts: vec![part(Shape::Bow, &[])],
        });
        assert_eq!(bow.near.triangles(), 4 * 4 * 2 + 4);
        assert_eq!(bow.far.triangles(), 2 * 4 * 2);
        let [lo, hi] = bow.bounds();
        assert!(hi[2] > 0.14 && lo[2] > -0.02 && (hi[1] - 0.408).abs() < 0.01);
        // Two by two cells, both sides, and the yard.
        check(
            Shape::Sail,
            &[],
            2 * 2 * 2 * 2 + 12,
            [-0.82, 0.0, -0.035],
            [0.82, 1.14, 0.15],
        );
        // Two oars a side, each a shaft and a blade.
        let oars = bake_model(&ModelSpec {
            parts: vec![part(Shape::OarBank, &[("count", 2.0)])],
        });
        assert_eq!(oars.near.triangles(), 2 * 2 * (10 + 12));
        assert_eq!(oars.far.triangles(), 2 * 12);
        assert_eq!(oars.rig.joints.len(), 1 + 4);
        // Body, neck, head, ears, mane, tail and four legs with hooves.
        check(
            Shape::Quadruped,
            &[],
            12 + 10 + 12 + 8 + 10 + 12 + 4 * 20,
            [-0.15, 0.0, -0.70405],
            [0.15, 1.16212, 0.97213],
        );
    }

    #[test]
    fn moving_shapes_record_their_joints() {
        let joints = |shape: Shape, knobs: &[(&str, f32)]| {
            let baked = bake_model(&ModelSpec {
                parts: vec![part(shape, knobs)],
            });
            assert!(baked.warnings.is_empty(), "{shape:?}: {:?}", baked.warnings);
            baked.rig.joints.len()
        };
        // The model, the body, two legs and two arms.
        assert_eq!(joints(Shape::Figure, &[]), 6);
        // The model, the body, four legs, the neck and the tail.
        assert_eq!(joints(Shape::Quadruped, &[]), 8);
        assert_eq!(joints(Shape::Wheel, &[]), 2);
        assert_eq!(joints(Shape::Hull, &[]), 1);
        assert_eq!(joints(Shape::Hull, &[("float", 1.0)]), 2);
        assert_eq!(joints(Shape::Box, &[("strike", 30.0)]), 2);
        assert_eq!(joints(Shape::Box, &[]), 1);
    }

    #[test]
    fn robed_and_seated_figures_keep_their_size() {
        let robe = bake_model(&ModelSpec {
            parts: vec![part(Shape::Figure, &[("robe", 1.0)])],
        });
        let [lo, hi] = robe.bounds();
        assert!(lo[1].abs() < 1e-5 && (hi[1] - 0.986).abs() < 0.01);
        assert!(robe.near.triangles() <= 300 && robe.far.triangles() <= 80);
        // Seated legs splay out and forward from the hips; nothing reaches the ground.
        let seated = bake_model(&ModelSpec {
            parts: vec![part(Shape::Figure, &[("seat", 1.0)])],
        });
        let [lo, hi] = seated.bounds();
        assert!(
            lo[1] > 0.02 && hi[2] > 0.25 && hi[0] > 0.27,
            "{lo:?} {hi:?}"
        );
        assert!(seated.near.triangles() <= 300 && seated.far.triangles() <= 80);
    }

    #[test]
    fn places_compose_turns_scales_and_moves() {
        let outer = Place::turned([1.0, 0.0, 0.0], FRAC_PI_2);
        let inner = Place {
            scale: 2.0,
            ..Place::at([1.0, 0.0, 0.0])
        };
        let p = outer.then(&inner).apply([1.0, 1.0, 0.0]);
        // Inner: (1,1,0) → (3,2,0); outer turns +x towards −z, then moves by +x.
        assert!(close(p, [1.0, 2.0, -3.0]), "{p:?}");
    }
}
