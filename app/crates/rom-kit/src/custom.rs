//! Free-form part geometry (`shape = 'custom'`): triangles modelled outside the kit, kept in a
//! small text file per part (`app/assets/models/<model id>/<part>.mesh`) and baked like any
//! other part.
//!
//! The format, one record per line (`#` starts a comment):
//!
//! ```text
//! rom-mesh 1
//! material 0 mat.limestone      # slot, then a material key, a #rrggbb colour or - (the part's)
//! lod near                      # near version, then the far one
//! v 0.5 0 -0.5                  # position, tiles, in the part's frame (x east, y up, z south)
//! n 0 1 0                       # normal of the vertex of the same row
//! f 0 1 2 0 1                   # three vertex rows (0-based), material slot, team (0 or 1)
//! lod far
//! ...
//! ```
//!
//! Faces are counter-clockwise seen from outside; the bake keeps them flat-shaded.

use std::fmt::Write as _;

use crate::material::MaterialLook;

/// One triangle of a free-form part.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CustomFace {
    /// Vertex rows of its corners.
    pub corners: [u32; 3],
    /// Material slot.
    pub material: u32,
    /// Wears the owner's colour.
    pub team: bool,
}

/// One version (near or far) of a free-form part.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CustomLod {
    /// Vertex positions in the part's frame, tiles.
    pub positions: Vec<[f32; 3]>,
    /// Vertex normals, one per position.
    pub normals: Vec<[f32; 3]>,
    /// Triangles.
    pub faces: Vec<CustomFace>,
}

impl CustomLod {
    /// Smallest box holding every vertex: `[min, max]`; zero when empty.
    pub fn bounds(&self) -> [[f32; 3]; 2] {
        if self.positions.is_empty() {
            return [[0.0; 3]; 2];
        }
        let mut lo = [f32::MAX; 3];
        let mut hi = [f32::MIN; 3];
        for p in &self.positions {
            for k in 0..3 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
        [lo, hi]
    }
}

/// The geometry of a free-form part: its material slots and near and far versions.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CustomMesh {
    /// Material of each slot: a material key, `#rrggbb`, or `-` for the part's own material.
    pub materials: Vec<String>,
    /// Full detail.
    pub near: CustomLod,
    /// The version drawn from afar.
    pub far: CustomLod,
}

/// What the faces of one material slot look like once the slot is resolved.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FaceLook {
    /// The part's own material.
    Part,
    /// A material of the rules.
    Material(MaterialLook),
    /// A plain colour, RGB 0..1.
    Plain([f32; 3]),
}

/// A free-form part ready to bake: its geometry and the look of each material slot.
#[derive(Clone, Debug, PartialEq)]
pub struct CustomPaint {
    /// The geometry.
    pub mesh: CustomMesh,
    /// The look of each slot of `mesh.materials`.
    pub looks: Vec<FaceLook>,
}

impl CustomMesh {
    /// Reads the text format; errors name the line.
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut mesh = CustomMesh::default();
        let mut lod: Option<bool> = None;
        let mut header = false;
        for (n, raw) in text.lines().enumerate() {
            // `#` starts a comment, except in a `#rrggbb` colour.
            let words: Vec<&str> = raw
                .split_whitespace()
                .take_while(|w| !w.starts_with('#') || is_colour(w))
                .collect();
            if words.is_empty() {
                continue;
            }
            let fail = |what: &str| format!("line {}: {what}: {raw:?}", n + 1);
            let numbers = |count: usize| -> Result<Vec<f32>, String> {
                if words.len() != count + 1 {
                    return Err(fail(&format!("expected {count} numbers")));
                }
                words[1..]
                    .iter()
                    .map(|w| {
                        w.parse::<f32>()
                            .ok()
                            .filter(|v| v.is_finite())
                            .ok_or_else(|| fail("not a number"))
                    })
                    .collect()
            };
            match words[0] {
                "rom-mesh" if words.get(1) == Some(&"1") && words.len() == 2 => header = true,
                "rom-mesh" => return Err(fail("unsupported version")),
                _ if !header => return Err(fail("the file must start with `rom-mesh 1`")),
                "material" => {
                    let slot: usize = words
                        .get(1)
                        .and_then(|w| w.parse().ok())
                        .ok_or_else(|| fail("material slot"))?;
                    if slot != mesh.materials.len() || words.len() != 3 {
                        return Err(fail("material slots are numbered 0, 1, 2 … in order"));
                    }
                    mesh.materials.push(words[2].to_owned());
                }
                "lod" => {
                    lod = match words.get(1) {
                        Some(&"near") if words.len() == 2 => Some(true),
                        Some(&"far") if words.len() == 2 => Some(false),
                        _ => return Err(fail("expected `lod near` or `lod far`")),
                    }
                }
                tag @ ("v" | "n" | "f") => {
                    let Some(near) = lod else {
                        return Err(fail(
                            "vertices and faces come after `lod near` or `lod far`",
                        ));
                    };
                    let target = if near { &mut mesh.near } else { &mut mesh.far };
                    match tag {
                        "v" => {
                            let v = numbers(3)?;
                            target.positions.push([v[0], v[1], v[2]]);
                        }
                        "n" => {
                            let v = numbers(3)?;
                            target.normals.push([v[0], v[1], v[2]]);
                        }
                        _ => {
                            if words.len() != 6 {
                                return Err(fail(
                                    "a face is three vertex rows, a material slot and a team flag",
                                ));
                            }
                            let int = |w: &str| w.parse::<u32>().map_err(|_| fail("not an index"));
                            let team = match words[5] {
                                "0" => false,
                                "1" => true,
                                _ => return Err(fail("team is 0 or 1")),
                            };
                            target.faces.push(CustomFace {
                                corners: [int(words[1])?, int(words[2])?, int(words[3])?],
                                material: int(words[4])?,
                                team,
                            });
                        }
                    }
                }
                other => return Err(fail(&format!("unknown record {other:?}"))),
            }
        }
        if !header {
            return Err("empty mesh file: it must start with `rom-mesh 1`".into());
        }
        for (name, lod) in [("near", &mesh.near), ("far", &mesh.far)] {
            if lod.positions.len() != lod.normals.len() {
                return Err(format!(
                    "{name}: {} positions but {} normals",
                    lod.positions.len(),
                    lod.normals.len()
                ));
            }
            for face in &lod.faces {
                if face
                    .corners
                    .iter()
                    .any(|&c| c as usize >= lod.positions.len())
                {
                    return Err(format!(
                        "{name}: a face names a vertex row that does not exist"
                    ));
                }
                if face.material as usize >= mesh.materials.len() {
                    return Err(format!(
                        "{name}: a face names a material slot that does not exist"
                    ));
                }
            }
        }
        Ok(mesh)
    }

    /// The text format of this mesh (what [`CustomMesh::parse`] reads back unchanged).
    pub fn to_text(&self) -> String {
        let mut out = String::from(
            "# Rise of Man part mesh: tiles, the part's own frame (x east, y up, z south).\n\
             # Format: app/docs/models-in-meshcraft.md\n\
             rom-mesh 1\n",
        );
        for (slot, material) in self.materials.iter().enumerate() {
            let _ = writeln!(out, "material {slot} {material}");
        }
        for (name, lod) in [("near", &self.near), ("far", &self.far)] {
            let _ = writeln!(out, "lod {name}");
            for p in &lod.positions {
                let _ = writeln!(out, "v {} {} {}", p[0], p[1], p[2]);
            }
            for v in &lod.normals {
                let _ = writeln!(out, "n {} {} {}", v[0], v[1], v[2]);
            }
            for f in &lod.faces {
                let _ = writeln!(
                    out,
                    "f {} {} {} {} {}",
                    f.corners[0],
                    f.corners[1],
                    f.corners[2],
                    f.material,
                    u8::from(f.team)
                );
            }
        }
        out
    }
}

/// A far version of `near` with fewer triangles: vertices closer than a twelfth of the part's
/// largest extent merge, and triangles that collapse are dropped. Small meshes stay as they are.
pub fn decimate(near: &CustomLod) -> CustomLod {
    const KEEP_BELOW: usize = 80;
    if near.faces.len() <= KEEP_BELOW {
        return near.clone();
    }
    let [lo, hi] = near.bounds();
    let extent = (0..3).map(|k| hi[k] - lo[k]).fold(0.0_f32, f32::max);
    let cell = (extent / 12.0).max(1e-4);
    let key = |p: [f32; 3]| p.map(|c| (c / cell).round() as i64);
    let mut cells: Vec<([i64; 3], [f32; 3])> = Vec::new();
    let mut out = CustomLod::default();
    for f in &near.faces {
        let mut corners = [0u32; 3];
        for (k, &c) in f.corners.iter().enumerate() {
            let p = near.positions[c as usize];
            let id = key(p);
            let at = match cells.iter().position(|(cell, _)| *cell == id) {
                Some(at) => at,
                None => {
                    cells.push((id, p));
                    cells.len() - 1
                }
            };
            corners[k] = at as u32;
        }
        if corners[0] == corners[1] || corners[1] == corners[2] || corners[0] == corners[2] {
            continue;
        }
        let [a, b, c] = corners.map(|i| cells[i as usize].1);
        let n = cross(sub(b, a), sub(c, a));
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        if len < 1e-9 {
            continue;
        }
        let base = out.positions.len() as u32;
        for p in [a, b, c] {
            out.positions.push(p);
            out.normals.push(n.map(|v| v / len));
        }
        out.faces.push(CustomFace {
            corners: [base, base + 1, base + 2],
            ..*f
        });
    }
    out
}

/// Whether `word` is a `#rrggbb` colour.
fn is_colour(word: &str) -> bool {
    word.len() == 7 && word[1..].chars().all(|c| c.is_ascii_hexdigit())
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

#[cfg(test)]
mod tests {
    use super::*;

    const TRIANGLE: &str = "rom-mesh 1\nmaterial 0 mat.limestone\nmaterial 1 #ff8000\nlod near\n\
        v 0 0 0\nv 1 0 0\nv 0 1 0\nn 0 0 1\nn 0 0 1\nn 0 0 1\nf 0 1 2 1 1\nlod far\n";

    #[test]
    fn the_text_format_reads_back_what_it_writes() {
        let mesh = CustomMesh::parse(TRIANGLE).unwrap();
        assert_eq!(mesh.materials, ["mat.limestone", "#ff8000"]);
        assert_eq!(mesh.near.faces.len(), 1);
        assert!(mesh.near.faces[0].team);
        assert!(mesh.far.faces.is_empty());
        assert_eq!(CustomMesh::parse(&mesh.to_text()).unwrap(), mesh);
    }

    #[test]
    fn broken_files_are_refused_with_the_line() {
        for (text, needle) in [
            ("", "rom-mesh 1"),
            ("rom-mesh 2", "version"),
            ("rom-mesh 1\nv 0 0 0", "lod"),
            ("rom-mesh 1\nlod near\nv 0 0", "3 numbers"),
            (
                "rom-mesh 1\nmaterial 0 -\nlod near\nv 0 0 0\nn 0 1 0\nf 0 0 5 0 0",
                "vertex row",
            ),
            (
                "rom-mesh 1\nlod near\nv 0 0 0\nn 0 1 0\nf 0 0 0 3 0",
                "material slot",
            ),
            ("rom-mesh 1\nlod near\nv 0 0 0", "normals"),
        ] {
            let error = CustomMesh::parse(text).unwrap_err();
            assert!(error.contains(needle), "{text:?}: {error}");
        }
    }

    #[test]
    fn decimation_keeps_small_meshes_and_thins_large_ones() {
        let small = CustomMesh::parse(TRIANGLE).unwrap().near;
        assert_eq!(decimate(&small), small);
        // A finely split strip: 200 slivers along x collapse to a few cells.
        let mut big = CustomLod::default();
        for i in 0..200 {
            let x = i as f32 * 0.005;
            let base = big.positions.len() as u32;
            for p in [[x, 0.0, 0.0], [x + 0.005, 0.0, 0.0], [x, 1.0, 0.0]] {
                big.positions.push(p);
                big.normals.push([0.0, 0.0, 1.0]);
            }
            big.faces.push(CustomFace {
                corners: [base, base + 1, base + 2],
                material: 0,
                team: false,
            });
        }
        let far = decimate(&big);
        assert!(far.faces.len() < big.faces.len() / 4, "{}", far.faces.len());
    }
}
