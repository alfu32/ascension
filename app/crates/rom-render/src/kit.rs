//! The part kit: one generator per [`Shape`], each painting flat-shaded faces of its part in
//! the part's own frame (base centre on the origin, x along its length, y up, front towards +z).
//! Surface detail comes from the material programs, not from geometry: a shape emits only its
//! silhouette, a few insets (openings, sills) and, for the far version, even less.

use std::f32::consts::{FRAC_PI_2, TAU};

use crate::models::{ModelMesh, PartSpec, Shape, TEAM_BIT};

/// Program id of faces drawn in their plain colour (openings, skin, hair).
const PLAIN: u8 = 0;
/// Program ids of the material library (see `materials.glsl`).
const TIMBER: u8 = 12;
const METAL: u8 = 15;

/// Turn about y (radians, counter-clockwise seen from above), uniform scale, then a move.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Place {
    /// Turn about the vertical axis, in radians.
    pub yaw: f32,
    /// Uniform scale.
    pub scale: f32,
    /// Where the local origin lands.
    pub at: [f32; 3],
}

impl Place {
    /// No change.
    pub const IDENTITY: Place = Place {
        yaw: 0.0,
        scale: 1.0,
        at: [0.0; 3],
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
        Place {
            yaw: self.yaw + child.yaw,
            scale: self.scale * child.scale,
            at: self.apply(child.at),
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
}

impl<'a> Painter<'a> {
    /// A painter adding to `mesh` at `place` with `surface`.
    pub fn new(mesh: &'a mut ModelMesh, place: Place, surface: Surface) -> Self {
        Self {
            mesh,
            place,
            surface,
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
pub fn paint(part: &PartSpec, p: &mut Painter<'_>, fine: bool) -> Vec<(&'static str, Place)> {
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
            p.cylinder(
                [0.0; 3],
                (r, r * (1.0 - taper)),
                k.size("h", 1.0),
                segments(&k, 10, fine, 4),
                true,
            );
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
        Shape::Figure => return figure(&k, p, fine),
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

/// `figure`: a standing person `h` tall facing +z, tunic in the part's surface, with sockets
/// at the head, both hands and the back.
fn figure(k: &Knobs<'_>, p: &mut Painter<'_>, fine: bool) -> Vec<(&'static str, Place)> {
    let h = k.size("h", 1.0);
    let b = k.size("build", 1.0);
    let tunic = p.surface;
    let skin = Surface::plain(k.colour("skin", [0.78, 0.58, 0.42]));
    let hair = Surface::plain(k.colour("hair", [0.18, 0.12, 0.08]));
    let legs = Surface::plain(k.colour("legs", k.colour("skin", [0.78, 0.58, 0.42])));
    let belt = Surface::of(TIMBER, [0.32, 0.22, 0.12], 0.05);
    p.within(
        Place {
            scale: h,
            ..Place::IDENTITY
        },
        None,
        |p| {
            if fine {
                p.surface = legs;
                for s in [-1.0, 1.0] {
                    p.cuboid([s * 0.06 * b, 0.0, 0.0], [0.1 * b, 0.4, 0.12], 1.1, false);
                }
                p.surface = hair;
                for s in [-1.0, 1.0] {
                    p.cuboid(
                        [s * 0.06 * b, 0.0, 0.025],
                        [0.11 * b, 0.05, 0.17],
                        1.0,
                        false,
                    );
                }
                p.surface = tunic;
                p.cuboid([0.0, 0.3, 0.0], [0.36 * b, 0.22, 0.24], 0.82, false);
                p.cuboid([0.0, 0.5, 0.0], [0.3 * b, 0.26, 0.2], 1.12, false);
                p.surface = belt;
                p.cuboid([0.0, 0.48, 0.0], [0.32 * b, 0.045, 0.215], 1.0, false);
                for s in [-1.0, 1.0] {
                    let x = s * 0.205 * b;
                    p.surface = tunic;
                    p.cuboid([x, 0.62, 0.0], [0.085 * b, 0.14, 0.1], 1.0, true);
                    p.surface = skin;
                    p.cuboid([x, 0.43, 0.01], [0.07 * b, 0.19, 0.08], 1.0, true);
                }
                p.cuboid([0.0, 0.76, 0.0], [0.07, 0.05, 0.07], 1.0, false);
                p.sphere_band([0.0, 0.87, 0.0], 0.1, 0.11, (-FRAC_PI_2, FRAC_PI_2), 4, 8);
                p.surface = hair;
                p.sphere_band([0.0, 0.878, -0.015], 0.108, 0.108, (0.05, FRAC_PI_2), 2, 8);
            } else {
                p.surface = legs;
                p.cuboid([0.0; 3], [0.22 * b, 0.4, 0.12], 1.0, false);
                p.surface = tunic;
                p.cuboid([0.0, 0.3, 0.0], [0.34 * b, 0.46, 0.22], 1.0, false);
                p.surface = skin;
                for s in [-1.0, 1.0] {
                    p.cuboid(
                        [s * 0.205 * b, 0.43, 0.0],
                        [0.08 * b, 0.33, 0.09],
                        1.0,
                        false,
                    );
                }
                p.cuboid([0.0, 0.77, 0.0], [0.19, 0.2, 0.19], 1.0, false);
            }
        },
    );
    p.surface = tunic;
    let at = |x: f32, y: f32, z: f32| Place::at([x * h, y * h, z * h]);
    vec![
        ("head", at(0.0, 0.98, 0.0)),
        // Facing +z with y up, the figure's right hand is on its −x side.
        ("right_hand", at(-0.205 * b, 0.45, 0.03)),
        ("left_hand", at(0.205 * b, 0.45, 0.03)),
        ("back", at(0.0, 0.62, -0.13)),
    ]
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
