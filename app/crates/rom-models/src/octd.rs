//! Meshcraft project files (`.octd`, schema 17 JSON) as far as models need them: prototypes
//! (named objects) with triangle faces and edges, instances placed by an origin and three axes,
//! a material catalogue (`meshcraftMaterials`, faces naming one by `meshcraftMaterialId`), and
//! custom properties (Meshcraft's property sets, `ifcProperties`) carrying the game settings of
//! each part in the set [`SET`].

use std::collections::BTreeMap;

use rom_kit::kit::Place;
use serde_json::{Map, Value, json};

/// Name of the property set holding a part's game settings.
pub const SET: &str = "Rise of Man";
/// Schema version of the files written.
pub const VERSION: u64 = 17;
/// Root field naming the model a file was exported from.
pub const MODEL_FIELD: &str = "romModel";

/// An affine frame: three axes (columns, with scale) and an origin.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    /// Images of x, y and z.
    pub axes: [[f64; 3]; 3],
    /// Image of the origin.
    pub origin: [f64; 3],
}

impl Frame {
    /// No change.
    pub const IDENTITY: Frame = Frame {
        axes: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        origin: [0.0; 3],
    };

    /// The frame of a kit place.
    pub fn of_place(place: &Place) -> Self {
        let axis = |e: [f32; 3]| place.turn(e).map(f64::from);
        Self {
            axes: [
                axis([1.0, 0.0, 0.0]),
                axis([0.0, 1.0, 0.0]),
                axis([0.0, 0.0, 1.0]),
            ],
            origin: place.at.map(f64::from),
        }
    }

    /// The frame a part's placement knobs give, in its anchor: a lean (`roll` then `pitch`,
    /// degrees), a turn `yaw` about y, uniform `scale`, then a move to `at`.
    pub fn of_knobs(at: [f64; 3], yaw: f64, pitch: f64, roll: f64, scale: f64) -> Self {
        let (sy, cy) = yaw.to_radians().sin_cos();
        let (sp, cp) = pitch.to_radians().sin_cos();
        let (sr, cr) = roll.to_radians().sin_cos();
        let yaw_m = [[cy, 0.0, sy], [0.0, 1.0, 0.0], [-sy, 0.0, cy]];
        let pitch_m = [[1.0, 0.0, 0.0], [0.0, cp, -sp], [0.0, sp, cp]];
        let roll_m = [[cr, sr, 0.0], [-sr, cr, 0.0], [0.0, 0.0, 1.0]];
        let m = mat_mul(&yaw_m, &mat_mul(&pitch_m, &roll_m));
        Self {
            axes: std::array::from_fn(|j| std::array::from_fn(|i| m[i][j] * scale)),
            origin: at,
        }
    }

    /// The point `p` of the inner frame in the outer one.
    pub fn apply(&self, p: [f64; 3]) -> [f64; 3] {
        std::array::from_fn(|i| {
            self.origin[i]
                + self.axes[0][i] * p[0]
                + self.axes[1][i] * p[1]
                + self.axes[2][i] * p[2]
        })
    }

    /// The direction `v` of the inner frame in the outer one.
    pub fn turn(&self, v: [f64; 3]) -> [f64; 3] {
        std::array::from_fn(|i| {
            self.axes[0][i] * v[0] + self.axes[1][i] * v[1] + self.axes[2][i] * v[2]
        })
    }

    /// `inner` placed inside this frame.
    pub fn then(&self, inner: &Frame) -> Frame {
        Frame {
            axes: inner.axes.map(|a| self.turn(a)),
            origin: self.apply(inner.origin),
        }
    }

    /// The inverse frame, if the axes are independent.
    pub fn inverse(&self) -> Option<Frame> {
        let [a, b, c] = self.axes;
        // Rows of the inverse are the cross products over the determinant.
        let det = dot(a, cross(b, c));
        if det.abs() < 1e-12 {
            return None;
        }
        let rows = [cross(b, c), cross(c, a), cross(a, b)].map(|r| r.map(|v| v / det));
        let axes = std::array::from_fn(|j| std::array::from_fn(|i| rows[i][j]));
        let origin = std::array::from_fn(|i| -dot(rows[i], self.origin));
        Some(Frame { axes, origin })
    }

    /// Splits this frame into a move, a turn (yaw, pitch, roll in degrees, as
    /// [`Frame::of_knobs`] composes them) and a scale along each own axis.
    pub fn decompose(&self) -> Decomposed {
        let scale = self.axes.map(|a| dot(a, a).sqrt());
        let r: [[f64; 3]; 3] =
            std::array::from_fn(|i| std::array::from_fn(|j| self.axes[j][i] / scale[j].max(1e-12)));
        let sp = (-r[1][2]).clamp(-1.0, 1.0);
        let pitch = sp.asin();
        let (yaw, roll) = if pitch.cos().abs() > 1e-6 {
            (r[0][2].atan2(r[2][2]), (-r[1][0]).atan2(r[1][1]))
        } else {
            ((-r[2][0]).atan2(r[0][0]), 0.0)
        };
        Decomposed {
            at: self.origin,
            yaw: yaw.to_degrees(),
            pitch: pitch.to_degrees(),
            roll: roll.to_degrees(),
            scale,
        }
    }

    /// Whether two frames agree to `tolerance` in every axis and origin component.
    pub fn close(&self, other: &Frame, tolerance: f64) -> bool {
        (0..3).all(|i| {
            (self.origin[i] - other.origin[i]).abs() <= tolerance
                && (0..3).all(|j| (self.axes[i][j] - other.axes[i][j]).abs() <= tolerance)
        })
    }
}

/// A frame split into its move, turn and scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Decomposed {
    /// The move.
    pub at: [f64; 3],
    /// Turn about y, degrees.
    pub yaw: f64,
    /// Lean towards +z, degrees.
    pub pitch: f64,
    /// Lean towards +x, degrees.
    pub roll: f64,
    /// Scale along the frame's own x, y and z.
    pub scale: [f64; 3],
}

fn mat_mul(a: &[[f64; 3]; 3], b: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    std::array::from_fn(|i| std::array::from_fn(|j| (0..3).map(|k| a[i][k] * b[k][j]).sum()))
}

/// Dot product.
pub fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Cross product.
pub fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// A number rounded to 1/100000, for file output.
fn round(v: f64) -> f64 {
    let r = (v * 100_000.0).round() / 100_000.0;
    if r == 0.0 { 0.0 } else { r }
}

/// `{"x":…,"y":…,"z":…}`.
pub fn vec3(v: [f64; 3]) -> Value {
    json!({"x": round(v[0]), "y": round(v[1]), "z": round(v[2])})
}

/// Reads `{"x":…,"y":…,"z":…}` (missing components are 0).
pub fn read_vec3(value: Option<&Value>) -> Option<[f64; 3]> {
    let object = value?.as_object()?;
    let c = |k: &str| object.get(k).and_then(Value::as_f64).unwrap_or(0.0);
    Some([c("x"), c("y"), c("z")])
}

/// Writes `frame` as `<prefix>Origin` and `<prefix>AxisU/V/W` fields.
pub fn put_frame(object: &mut Map<String, Value>, prefix: &str, frame: &Frame) {
    object.insert(format!("{prefix}Origin"), vec3(frame.origin));
    for (axis, name) in frame.axes.iter().zip(["U", "V", "W"]) {
        object.insert(format!("{prefix}Axis{name}"), vec3(*axis));
    }
}

/// Reads the frame written by [`put_frame`]; missing axes default to the unit axes.
pub fn read_frame(object: &Map<String, Value>, prefix: &str) -> Frame {
    let mut frame = Frame::IDENTITY;
    if let Some(origin) = read_vec3(object.get(&format!("{prefix}Origin"))) {
        frame.origin = origin;
    }
    for (k, name) in ["U", "V", "W"].iter().enumerate() {
        if let Some(axis) = read_vec3(object.get(&format!("{prefix}Axis{name}"))) {
            frame.axes[k] = axis;
        }
    }
    frame
}

/// One custom property value.
#[derive(Clone, Debug, PartialEq)]
pub enum Prop {
    /// Text (`IfcLabel`).
    Text(String),
    /// A number (`IfcReal`).
    Number(f64),
    /// A flag (`IfcBoolean`).
    Flag(bool),
}

/// The property sets value of `props` in set [`SET`].
pub fn props_value(props: &[(String, Prop)]) -> Value {
    let mut set = Map::new();
    for (name, prop) in props {
        let (data_type, value) = match prop {
            Prop::Text(s) => ("IfcLabel", Value::from(s.clone())),
            Prop::Number(n) => ("IfcReal", json!(round(*n))),
            Prop::Flag(b) => ("IfcBoolean", Value::from(*b)),
        };
        set.insert(
            name.clone(),
            json!({"kind": "single", "dataType": data_type, "value": value, "unit": null, "description": ""}),
        );
    }
    json!({ SET: set })
}

/// The properties of set [`SET`] in a property sets value, by name.
pub fn read_props(value: Option<&Value>) -> BTreeMap<String, Prop> {
    let mut out = BTreeMap::new();
    let Some(set) = value.and_then(|v| v.get(SET)).and_then(Value::as_object) else {
        return out;
    };
    for (name, property) in set {
        let value = property.get("value").unwrap_or(property);
        let prop = match value {
            Value::String(s) => Prop::Text(s.clone()),
            Value::Number(n) => Prop::Number(n.as_f64().unwrap_or(0.0)),
            Value::Bool(b) => Prop::Flag(*b),
            Value::Null => Prop::Text(String::new()),
            other => Prop::Text(other.to_string()),
        };
        out.insert(name.clone(), prop);
    }
    out
}

/// A colour of 0..1 channels as Meshcraft's face colour.
pub fn face_colour(rgb: [u8; 3]) -> Value {
    let c = |v: u8| round(f64::from(v) / 255.0);
    json!({"r": c(rgb[0]), "g": c(rgb[1]), "b": c(rgb[2]), "a": 1.0})
}

/// A catalogue material named `name` in `rgb`.
pub fn material(name: &str, rgb: [u8; 3]) -> Value {
    json!({
        "id": name,
        "name": name,
        "color": {"red": rgb[0], "green": rgb[1], "blue": rgb[2], "alpha": 255},
        "shadingType": "flat",
        "texturePath": null,
        "textureScaleU": 1.0,
        "textureScaleV": 1.0
    })
}

/// A triangle in a prototype's frame with its catalogue material.
#[derive(Clone, Debug, PartialEq)]
pub struct Tri {
    /// Corners, counter-clockwise seen from outside.
    pub points: [[f64; 3]; 3],
    /// Face colour.
    pub rgb: [u8; 3],
    /// Catalogue material id.
    pub material: String,
}

/// The faces and edges of a prototype: every triangle, and the edges that are not between two
/// triangles of one flat face (so Meshcraft shows outlines, not triangulation).
pub fn faces_and_edges(tris: &[Tri]) -> (Vec<Value>, Vec<Value>) {
    type Key = [i64; 3];
    let key = |p: [f64; 3]| p.map(|c| (c * 100_000.0).round() as i64);
    let normal = |t: &Tri| {
        let [a, b, c] = t.points;
        let n = cross(
            [b[0] - a[0], b[1] - a[1], b[2] - a[2]],
            [c[0] - a[0], c[1] - a[1], c[2] - a[2]],
        );
        let l = dot(n, n).sqrt().max(1e-12);
        n.map(|v| v / l)
    };
    // Per edge: faces met, its ends, the first face's normal, and whether it is a crease.
    type Edge = (usize, [f64; 3], [f64; 3], [f64; 3], bool);
    let mut edges: BTreeMap<(Key, Key), Edge> = BTreeMap::new();
    let mut order = Vec::new();
    let mut faces = Vec::with_capacity(tris.len());
    for (i, t) in tris.iter().enumerate() {
        faces.push(json!({
            "id": format!("face-{i}"),
            "a": vec3(t.points[0]),
            "b": vec3(t.points[1]),
            "c": vec3(t.points[2]),
            "color": face_colour(t.rgb),
            "meshcraftMaterialId": t.material,
        }));
        let n = normal(t);
        for k in 0..3 {
            let (p, q) = (t.points[k], t.points[(k + 1) % 3]);
            let (kp, kq) = (key(p), key(q));
            let id = if kp <= kq { (kp, kq) } else { (kq, kp) };
            match edges.get_mut(&id) {
                Some(edge) => {
                    edge.0 += 1;
                    // Two triangles of one flat face share this edge: not an outline.
                    if dot(edge.3, n) < 0.9999 {
                        edge.4 = true;
                    }
                }
                None => {
                    edges.insert(id, (1, p, q, n, false));
                    order.push(id);
                }
            }
        }
    }
    let segments = order
        .iter()
        .filter_map(|id| {
            let (count, p, q, _, crease) = edges[id];
            (count != 2 || crease).then_some((p, q))
        })
        .enumerate()
        .map(|(i, (p, q))| json!({"id": format!("edge-{i}"), "start": vec3(p), "end": vec3(q)}))
        .collect();
    (faces, segments)
}

/// A prototype object with the given faces, edges and children.
pub fn prototype(id: &str, name: &str, faces: Vec<Value>, segments: Vec<Value>) -> Value {
    let mut object = Map::new();
    object.insert("id".into(), id.into());
    object.insert("name".into(), name.into());
    object.insert("kind".into(), "MESH".into());
    put_frame(&mut object, "definition", &Frame::IDENTITY);
    object.insert("segments".into(), Value::Array(segments));
    object.insert("faces".into(), Value::Array(faces));
    object.insert("externalReferenceEnabled".into(), false.into());
    object.insert("externalReferencePath".into(), "".into());
    object.insert("gluedToSurface".into(), false.into());
    Value::Object(object)
}

/// An instance of `prototype` at `frame` with `children` and custom properties.
pub fn instance(
    id: &str,
    prototype: &str,
    frame: &Frame,
    children: Vec<Value>,
    props: Value,
) -> Value {
    let mut object = Map::new();
    object.insert("id".into(), id.into());
    object.insert("prototypeId".into(), prototype.into());
    put_frame(&mut object, "instance", frame);
    object.insert("children".into(), Value::Array(children));
    object.insert("ifcProperties".into(), props);
    Value::Object(object)
}

/// A camera looking at a model of `size` (across, high, deep) from the south-east, above.
pub fn camera(size: [f64; 3]) -> Value {
    let reach = size[0].max(size[1]).max(size[2]).max(1.0) * 1.8;
    let target = [0.0, size[1] * 0.4, 0.0];
    let position = [reach * 0.8, reach * 0.75, reach];
    let d = [
        target[0] - position[0],
        target[1] - position[1],
        target[2] - position[2],
    ];
    let l = dot(d, d).sqrt();
    let direction = d.map(|v| v / l);
    let side = cross(direction, [0.0, 1.0, 0.0]);
    let up = cross(side, direction);
    let ul = dot(up, up).sqrt();
    json!({
        "position": vec3(position),
        "direction": vec3(direction),
        "up": vec3(up.map(|v| v / ul)),
        "target": vec3(target),
        "near": 0.05,
        "far": 500.0,
        "fieldOfView": 45.0
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_decompose_into_the_knobs_that_made_them() {
        for (yaw, pitch, roll) in [
            (0.0, 0.0, 0.0),
            (90.0, 0.0, 0.0),
            (-135.0, 20.0, 0.0),
            (30.0, -15.0, 70.0),
            (179.0, 5.0, -5.0),
        ] {
            let frame = Frame::of_knobs([1.0, 2.0, -3.0], yaw, pitch, roll, 1.0);
            let d = frame.decompose();
            let again = Frame::of_knobs(d.at, d.yaw, d.pitch, d.roll, 1.0);
            assert!(frame.close(&again, 1e-9), "{yaw} {pitch} {roll}: {d:?}");
            assert!(d.scale.iter().all(|s| (s - 1.0).abs() < 1e-9));
        }
    }

    #[test]
    fn frames_match_kit_places() {
        let place = Place {
            yaw: 0.7,
            scale: 1.5,
            at: [1.0, 0.5, -2.0],
            tilt: rom_kit::kit::tilt(15.0, -25.0),
        };
        let frame = Frame::of_place(&place);
        let knobs = Frame::of_knobs([1.0, 0.5, -2.0], 0.7_f64.to_degrees(), 15.0, -25.0, 1.5);
        assert!(frame.close(&knobs, 1e-5), "{frame:?} {knobs:?}");
        let p = [0.3_f32, 1.2, -0.4];
        let by_place = place.apply(p).map(f64::from);
        let by_frame = frame.apply(p.map(f64::from));
        assert!((0..3).all(|i| (by_place[i] - by_frame[i]).abs() < 1e-5));
        let inverse = frame.inverse().unwrap();
        let back = inverse.apply(by_frame);
        assert!((0..3).all(|i| (back[i] - f64::from(p[i])).abs() < 1e-5));
    }

    #[test]
    fn flat_faces_show_their_outline_only() {
        let square = |m: &str| {
            let (a, b, c, d) = (
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 0.0, -1.0],
                [0.0, 0.0, -1.0],
            );
            vec![
                Tri {
                    points: [a, b, c],
                    rgb: [1, 2, 3],
                    material: m.into(),
                },
                Tri {
                    points: [a, c, d],
                    rgb: [1, 2, 3],
                    material: m.into(),
                },
            ]
        };
        let (faces, edges) = faces_and_edges(&square("mat.x"));
        assert_eq!(faces.len(), 2);
        assert_eq!(edges.len(), 4);
    }
}
