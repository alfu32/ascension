//! The zoom levels beyond a system (task 162): the cluster view, the quadrant view and the
//! galaxy view, and where each system stands in them. Pure maths: no window, no GPU.
//!
//! Every frame is the star-map frame (map units, `y` up). The **cluster view** shows the systems
//! of one star cluster at their true 3D offsets from the cluster's centre, spread so that the
//! cluster's closest pair stands [`system_view_clearance`] apart: from anywhere in the system
//! view no other system's star or planets show. Every cluster of a match is spread to the same
//! radius ([`GalaxyLayout::cluster_radius`]), so one zoom frames any of them. The **disk** (the
//! quadrant and galaxy views) is the galactic plane, `y = 0`: a cluster sits at its place on the
//! disk (`star_cluster.x, y`, galactic longitude and distance from the Sun with the latitude
//! dropped), [`DISK_UNITS`] map units per catalogue unit, drawn as its stars with their offsets
//! flattened onto the plane and shrunk to [`DISK_SHRINK`]. Between the two the anchored cluster
//! flattens onto the disk as the camera moves out ([`GalaxyLayout::positions`]). Seen from the
//! Sun (F2) every system stands at its true direction and distance ([`GalaxyLayout::sky`]).

use crate::hud_minimap::FOV_DEGREES;
use crate::layers::SYSTEM_DETAIL_GONE;
use crate::universe::{SYSTEM_RADIUS, UniverseScene};
use crate::zoom::{MAP_MIN_DISTANCE, SYSTEM_MAX_DISTANCE};

/// Map units per catalogue unit on the galactic disk.
pub const DISK_UNITS: f32 = 2000.0;
/// The share of its true (flattened) size a cluster is drawn on the disk: `DISK_SHRINK_PCT` in
/// `db/content/stars.py`, whose footprints keep the clusters apart at this size.
pub const DISK_SHRINK: f32 = 0.2;
/// Room about a cluster's stars on the disk, catalogue units: `DISK_ROOM` in `stars.py`.
pub const DISK_ROOM: f32 = 25.0;
/// Camera distance over the radius it frames in the cluster view: a system on the near side of
/// the cluster stays in the picture.
pub const CLUSTER_FIT: f32 = 4.0;
/// Camera distance over the radius it frames in the quadrant and galaxy views (looking down on
/// the disk from [`DISK_ELEVATION`]).
pub const DISK_FIT: f32 = 2.6;
/// Log-distance over which the anchored cluster flattens onto the disk past the cluster view.
pub const MORPH_SPAN: f32 = 1.2;
/// Least log-distance between the widest zooms of two levels.
pub const LEVEL_GAP: f32 = 0.7;
/// How far out a route leaving the cluster ends in the cluster view, in cluster radii: the
/// route's exit, toward the other cluster, where that cluster is named.
pub const EXIT_REACH: f32 = 1.3;
/// The widest picture the system view is measured for (width over height).
pub const WIDEST_ASPECT: f32 = 2.4;
/// Elevation in degrees the space camera eases to over the disk, unless it is steeper already:
/// the galaxy is seen from above, the quadrants as sectors.
pub const DISK_ELEVATION: f32 = 55.0;

/// The least distance, in map units, between two systems of the cluster view: from any camera of
/// the system view (up to the hand-over, [`MAP_MIN_DISTANCE`] from a focus panned anywhere within
/// twice the system's radius of its star) another system's own detail (planets, rings) is gone
/// ([`SYSTEM_DETAIL_GONE`]), and in the plane of the focus it lies beyond the corners of the
/// widest picture ([`WIDEST_ASPECT`]); five percent to spare.
pub fn system_view_clearance() -> f32 {
    let k = MAP_MIN_DISTANCE / SYSTEM_MAX_DISTANCE;
    let disc = SYSTEM_RADIUS * k;
    let reach = 2.0 * SYSTEM_RADIUS * k;
    let eye = MAP_MIN_DISTANCE + reach;
    let depth = disc / SYSTEM_DETAIL_GONE + eye;
    let tan = (FOV_DEGREES.to_radians() / 2.0).tan();
    let corner = MAP_MIN_DISTANCE * tan * (1.0 + WIDEST_ASPECT * WIDEST_ASPECT).sqrt();
    let lateral = corner + reach + disc;
    depth.max(lateral) * 1.05
}

/// The galactic frame's axes (toward longitude 0, toward 90 degrees, to the north galactic pole)
/// in the catalogue's equatorial frame: `galactic_basis()` in `db/content/stars.py`.
fn galactic_basis() -> [[f64; 3]; 3] {
    let (ra, dec) = (192.859_f64.to_radians(), 27.128_f64.to_radians());
    let z = [dec.cos() * ra.cos(), dec.cos() * ra.sin(), dec.sin()];
    let mut p = [-z[2] * z[0], -z[2] * z[1], 1.0 - z[2] * z[2]];
    let norm = p.iter().map(|c| c * c).sum::<f64>().sqrt();
    p = p.map(|c| c / norm);
    let q = cross64(z, p);
    let t = 122.932_f64.to_radians();
    let x = [0, 1, 2].map(|k| t.cos() * p[k] - t.sin() * q[k]);
    [x, cross64(z, x), z]
}

fn cross64(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// A catalogue vector in the star-map frame's axes: galactic longitude 0 along `x`, the north
/// galactic pole up (`y`), longitude 90 degrees along `-z` (right-handed, as the sky's frame).
fn to_frame(basis: &[[f64; 3]; 3], v: [f64; 3]) -> [f64; 3] {
    let g = basis.map(|axis| (0..3).map(|k| v[k] * axis[k]).sum::<f64>());
    [g[0], g[2], -g[1]]
}

/// The sky's own axes, for systems without a catalogue cluster (test scenes): the north
/// celestial pole up, right ascension 6h along `-z`, as before the galactic disk (task 59).
const SKY_AXES: [[f64; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

fn length(v: [f32; 3]) -> f32 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn lerp(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [0, 1, 2].map(|i| a[i] + (b[i] - a[i]) * t)
}

/// A cluster of the match: the systems the cluster view shows together. Systems without a
/// cluster (a test scene) are one; the celestial home is one of its own.
#[derive(Clone, Debug, PartialEq)]
pub struct ClusterGroup {
    /// Shown at the quadrant and galaxy levels.
    pub name: String,
    /// Its centre in the star-map frame, on the disk; the cluster view spreads its systems about
    /// it and the disk draws them about it.
    pub centre: [f32; 3],
    /// Its systems, indices of the scene's systems in order.
    pub members: Vec<usize>,
    /// Radius of its footprint on the disk, map units: its flattened stars plus the room.
    pub room: f32,
    /// The galactic quadrant it lies in, 0 for I to 3 for IV.
    pub quadrant: u8,
    /// The celestial home's own group, which is no catalogue cluster.
    pub celestial: bool,
}

/// How far the zoom reaches beyond a system: the camera distances, in map units, at which the
/// whole cluster, the widest quadrant and the whole galaxy fit. A level that would show a single
/// item has none.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ladder {
    /// Distance that frames a whole cluster; `None` for a match of one system, which has the
    /// planet and the system levels only.
    pub cluster: Option<f32>,
    /// Distance that frames the widest quadrant; `None` when the match lies in one quadrant or
    /// one cluster.
    pub quadrant: Option<f32>,
    /// Distance that frames every cluster; `None` for a match of one cluster.
    pub galaxy: Option<f32>,
}

impl Default for Ladder {
    fn default() -> Self {
        Self {
            cluster: Some(MAP_MIN_DISTANCE * 40.0),
            quadrant: None,
            galaxy: None,
        }
    }
}

/// Where every system of a universe stands at the levels beyond a system.
#[derive(Clone, Debug, PartialEq)]
pub struct GalaxyLayout {
    /// The group of each system (an index of [`Self::groups`]), indexed like the scene's systems.
    pub group: Vec<usize>,
    /// The clusters of the match, catalogue clusters by id, then the systems without one, then
    /// the celestial home.
    pub groups: Vec<ClusterGroup>,
    /// Each system in the cluster view.
    pub cluster: Vec<[f32; 3]>,
    /// Each system on the disk.
    pub disk: Vec<[f32; 3]>,
    /// Each system at its true place, seen from the Sun.
    pub sky: Vec<[f32; 3]>,
    /// The Sun, where "Sky from the Sun" puts the eye.
    pub sun: [f32; 3],
    /// The radius of every cluster in the cluster view, map units.
    pub cluster_radius: f32,
    /// The widest zoom of each level.
    pub ladder: Ladder,
}

impl GalaxyLayout {
    /// The layout of `scene`'s systems.
    pub fn new(scene: &UniverseScene) -> Self {
        let basis = galactic_basis();
        let g = f64::from(DISK_UNITS);
        let n = scene.systems.len();
        // Groups in a fixed order: catalogue clusters by id, then systems without one, then the
        // celestial home.
        let mut keys: std::collections::BTreeMap<(u8, u64), Vec<usize>> = Default::default();
        for (i, system) in scene.systems.iter().enumerate() {
            let key = match (system.celestial, system.cluster) {
                (true, _) => (2, i as u64),
                (false, Some(id)) => (0, u64::from(id)),
                (false, None) => (1, 0),
            };
            keys.entry(key).or_default().push(i);
        }
        let sky_of = |i: usize| {
            let s = &scene.systems[i];
            [f64::from(s.x), f64::from(s.y), f64::from(s.z)]
        };
        let clearance = system_view_clearance();
        struct Raw {
            key: (u8, u64),
            members: Vec<usize>,
            centre: [f64; 3],
            offsets: Vec<[f64; 3]>,
            radius: f64,
            spacing: f64,
        }
        let mut raws = Vec::new();
        for (key, members) in keys {
            let count = members.len() as f64;
            let mean =
                [0, 1, 2].map(|k| members.iter().map(|&i| sky_of(i)[k]).sum::<f64>() / count);
            let axes = if key.0 == 1 { &SKY_AXES } else { &basis };
            let offsets: Vec<[f64; 3]> = members
                .iter()
                .map(|&i| to_frame(axes, [0, 1, 2].map(|k| sky_of(i)[k] - mean[k])))
                .collect();
            let radius = offsets
                .iter()
                .map(|o| (o[0] * o[0] + o[1] * o[1] + o[2] * o[2]).sqrt())
                .fold(0.0, f64::max);
            let mut spacing = f64::INFINITY;
            for (a, &i) in members.iter().enumerate() {
                for &j in &members[a + 1..] {
                    let (p, q) = (sky_of(i), sky_of(j));
                    let d = (0..3).map(|k| (p[k] - q[k]).powi(2)).sum::<f64>().sqrt();
                    spacing = spacing.min(d);
                }
            }
            let cluster = scene
                .clusters
                .iter()
                .find(|c| key.0 == 0 && u64::from(c.id) == key.1);
            let centre = match (key.0, cluster) {
                (2, _) => to_frame(&basis, sky_of(members[0])).map(|c| c * g),
                (_, Some(c)) => [f64::from(c.x) * g, 0.0, -f64::from(c.y) * g],
                _ => {
                    // Placed as the generator places a cluster: its centre's longitude at its
                    // distance from the Sun, the latitude dropped.
                    let f = to_frame(axes, mean);
                    let r = (mean[0] * mean[0] + mean[1] * mean[1] + mean[2] * mean[2]).sqrt();
                    let along = f[0].hypot(f[2]);
                    if along > 1e-9 {
                        [f[0] / along * r * g, 0.0, f[2] / along * r * g]
                    } else {
                        [r * g, 0.0, 0.0]
                    }
                }
            };
            raws.push(Raw {
                key,
                members,
                centre,
                offsets,
                radius,
                spacing,
            });
        }
        // One radius for every cluster: wide enough that the most crowded one keeps its closest
        // pair the clearance apart.
        let cluster_radius = raws
            .iter()
            .filter(|r| r.key.0 != 2 && r.members.len() > 1 && r.spacing > 1e-6)
            .map(|r| f64::from(clearance) * r.radius / r.spacing)
            .fold(0.0_f64, f64::max)
            .max(f64::from(clearance)) as f32;
        // The frame's origin is the middle of the clusters, so coordinates stay small.
        let material: Vec<&Raw> = raws.iter().filter(|r| r.key.0 != 2).collect();
        let origin = if material.is_empty() {
            [0.0; 3]
        } else {
            let count = material.len() as f64;
            [0, 1, 2].map(|k| material.iter().map(|r| r.centre[k]).sum::<f64>() / count)
        };
        let shift = |p: [f64; 3]| [0, 1, 2].map(|k| (p[k] - origin[k]) as f32);
        let sun = shift([0.0; 3]);
        let mut group = vec![0; n];
        let mut groups = Vec::new();
        let (mut cluster, mut disk, mut sky) = (vec![[0.0; 3]; n], vec![[0.0; 3]; n], vec![]);
        for i in 0..n {
            sky.push(shift(to_frame(&basis, sky_of(i)).map(|c| c * g)));
        }
        for (index, raw) in raws.iter().enumerate() {
            let centre = shift(raw.centre);
            let spread = if raw.radius > 1e-9 {
                f64::from(cluster_radius) / raw.radius
            } else {
                0.0
            };
            let mut flat_most = 0.0_f64;
            for (&i, o) in raw.members.iter().zip(&raw.offsets) {
                group[i] = index;
                if raw.key.0 == 2 {
                    cluster[i] = centre;
                    disk[i] = centre;
                    continue;
                }
                cluster[i] = [0, 1, 2].map(|k| centre[k] + (o[k] * spread) as f32);
                let flat = [o[0], 0.0, o[2]];
                flat_most = flat_most.max(o[0].hypot(o[2]));
                let s = f64::from(DISK_SHRINK) * g;
                disk[i] = [0, 1, 2].map(|k| centre[k] + (flat[k] * s) as f32);
            }
            let from_sun = sub(centre, sun);
            let degrees = (-from_sun[2])
                .atan2(from_sun[0])
                .to_degrees()
                .rem_euclid(360.0);
            let name = match raw.key {
                (2, _) => scene.systems[raw.members[0]].name.clone(),
                (0, id) => scene
                    .clusters
                    .iter()
                    .find(|c| u64::from(c.id) == id)
                    .map_or_else(|| format!("Cluster {id}"), |c| c.name.clone()),
                _ => "Local group".to_owned(),
            };
            groups.push(ClusterGroup {
                name,
                centre,
                members: raw.members.clone(),
                room: ((f64::from(DISK_SHRINK) * flat_most + f64::from(DISK_ROOM)) * g) as f32,
                quadrant: ((degrees / 90.0) as u8).min(3),
                celestial: raw.key.0 == 2,
            });
        }
        let ladder = ladder(scene, &groups, cluster_radius);
        Self {
            group,
            groups,
            cluster,
            disk,
            sky,
            sun,
            cluster_radius,
            ladder,
        }
    }

    /// The systems where the zoom shows them, anchored on group `anchored`, `disk` of the way
    /// (0 to 1) from the cluster view to the disk: the anchored cluster flattens from its 3D
    /// spread onto its place on the disk; a system of another cluster joined to it by a route
    /// stands at that route's exit ([`GalaxyLayout::exit`]) and moves to its place on the disk;
    /// every other system stands on the disk.
    pub fn positions(&self, scene: &UniverseScene, anchored: usize, disk: f32) -> Vec<[f32; 3]> {
        let exits = self.exits(scene, anchored);
        (0..self.group.len())
            .map(|i| {
                let from = if self.group[i] == anchored {
                    self.cluster[i]
                } else {
                    exits
                        .iter()
                        .find(|e| e.0 == i)
                        .map_or(self.disk[i], |e| e.1)
                };
                lerp(from, self.disk[i], disk)
            })
            .collect()
    }

    /// The exits of group `anchored`: each system of another group joined to one of its systems
    /// by a route, with the point where that route leaves the cluster view (toward the other
    /// system's place on the disk, [`EXIT_REACH`] cluster radii from the centre), in system order.
    pub fn exits(&self, scene: &UniverseScene, anchored: usize) -> Vec<(usize, [f32; 3])> {
        let Some(home) = self.groups.get(anchored) else {
            return Vec::new();
        };
        let mut out: Vec<(usize, [f32; 3])> = Vec::new();
        for &(a, b) in &scene.routes {
            let (Some(a), Some(b)) = (scene.system_index(a), scene.system_index(b)) else {
                continue;
            };
            for (inside, outside) in [(a, b), (b, a)] {
                if self.group[inside] == anchored
                    && self.group[outside] != anchored
                    && !out.iter().any(|e| e.0 == outside)
                {
                    out.push((outside, self.exit(home.centre, self.disk[outside])));
                }
            }
        }
        out.sort_by_key(|e| e.0);
        out
    }

    /// The point `EXIT_REACH` cluster radii from `centre` toward `toward`, in the disk's plane.
    fn exit(&self, centre: [f32; 3], toward: [f32; 3]) -> [f32; 3] {
        let way = [toward[0] - centre[0], 0.0, toward[2] - centre[2]];
        let reach = self.cluster_radius * EXIT_REACH;
        let along = way[0].hypot(way[2]);
        if along < 1e-3 {
            return [centre[0] + reach, centre[1], centre[2]];
        }
        [
            centre[0] + way[0] / along * reach,
            centre[1],
            centre[2] + way[2] / along * reach,
        ]
    }

    /// The group whose footprint on the disk lies nearest `point` (its distance to the centre
    /// less its room, so a point inside a footprint picks that one); material clusters only.
    pub fn nearest_group(&self, point: [f32; 3]) -> Option<usize> {
        self.groups
            .iter()
            .enumerate()
            .filter(|(_, g)| !g.celestial)
            .map(|(i, g)| {
                let d = [point[0] - g.centre[0], 0.0, point[2] - g.centre[2]];
                (i, length(d) - g.room)
            })
            .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))
            .map(|(i, _)| i)
    }

    /// The groups that are catalogue clusters (or the systems without one), the celestial home
    /// left out.
    pub fn material(&self) -> impl Iterator<Item = (usize, &ClusterGroup)> {
        self.groups.iter().enumerate().filter(|(_, g)| !g.celestial)
    }

    /// The middle of the disk and the radius that holds every cluster's footprint.
    pub fn disk_extent(&self) -> ([f32; 3], f32) {
        let material: Vec<&ClusterGroup> = self.material().map(|(_, g)| g).collect();
        extent(material.iter().copied())
    }
}

/// The middle of `groups`' centres and the radius about it that holds their footprints.
fn extent<'a>(groups: impl Iterator<Item = &'a ClusterGroup> + Clone) -> ([f32; 3], f32) {
    let count = groups.clone().count().max(1) as f32;
    let middle = [0, 1, 2].map(|k| groups.clone().map(|g| g.centre[k]).sum::<f32>() / count);
    let reach = groups
        .map(|g| length(sub(g.centre, middle)) + g.room)
        .fold(0.0, f32::max);
    (middle, reach)
}

/// The widest zoom of each level for the groups of a scene: a level that would show one item is
/// skipped, and each level spans at least [`LEVEL_GAP`] (the disk [`MORPH_SPAN`] more).
fn ladder(scene: &UniverseScene, groups: &[ClusterGroup], cluster_radius: f32) -> Ladder {
    let systems = scene.systems.iter().filter(|s| !s.celestial).count();
    if systems <= 1 {
        return Ladder {
            cluster: None,
            quadrant: None,
            galaxy: None,
        };
    }
    let cluster = (cluster_radius * CLUSTER_FIT).max(MAP_MIN_DISTANCE * LEVEL_GAP.exp() * 2.0);
    let material: Vec<&ClusterGroup> = groups.iter().filter(|g| !g.celestial).collect();
    if material.len() <= 1 {
        return Ladder {
            cluster: Some(cluster),
            quadrant: None,
            galaxy: None,
        };
    }
    let past_cluster = cluster * (MORPH_SPAN + LEVEL_GAP).exp();
    let (_, reach) = extent(material.iter().copied());
    let mut galaxy = (reach * DISK_FIT).max(past_cluster);
    let mut quadrants: Vec<u8> = material.iter().map(|g| g.quadrant).collect();
    quadrants.sort_unstable();
    quadrants.dedup();
    let quadrant = (quadrants.len() > 1).then(|| {
        // A quadrant view frames the bulk of its quadrant (nine clusters in ten); the few far out
        // are seen by panning, and the galaxy view holds them all.
        let widest = quadrants
            .iter()
            .map(|&q| {
                let members: Vec<&ClusterGroup> = material
                    .iter()
                    .copied()
                    .filter(|g| g.quadrant == q)
                    .collect();
                let (middle, _) = extent(members.iter().copied());
                let mut reach: Vec<f32> = members
                    .iter()
                    .map(|g| length(sub(g.centre, middle)) + g.room)
                    .collect();
                reach.sort_by(f32::total_cmp);
                reach[(reach.len() * 9 / 10).min(reach.len() - 1)]
            })
            .fold(0.0, f32::max);
        let fit = (widest * DISK_FIT).max(past_cluster);
        galaxy = galaxy.max(fit * LEVEL_GAP.exp());
        fit
    });
    Ladder {
        cluster: Some(cluster),
        quadrant,
        galaxy: Some(galaxy),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::universe::{ClusterView, SystemView};

    fn system(id: u32, at: [i32; 3], cluster: Option<u32>) -> SystemView {
        SystemView {
            id,
            name: format!("S{id}"),
            x: at[0],
            y: at[1],
            z: at[2],
            constellation: String::new(),
            star_colour: [255, 255, 255],
            celestial: false,
            cluster,
        }
    }

    #[test]
    fn the_clearance_keeps_other_systems_out_of_the_system_view() {
        let c = system_view_clearance();
        let k = MAP_MIN_DISTANCE / SYSTEM_MAX_DISTANCE;
        // From the farthest eye of the system view, another system's disc of orbits looks smaller
        // than the angle at which its own detail is gone.
        let eye = MAP_MIN_DISTANCE + 2.0 * SYSTEM_RADIUS * k;
        assert!(SYSTEM_RADIUS * k / (c - eye) < SYSTEM_DETAIL_GONE);
        assert!(c > 60_000.0 && c < 90_000.0, "{c}");
    }

    #[test]
    fn a_cluster_view_keeps_its_closest_pair_the_clearance_apart() {
        let scene = UniverseScene {
            systems: vec![
                system(1, [0, 0, 0], Some(1)),
                system(2, [60, 0, 0], Some(1)),
                system(3, [0, 300, 0], Some(1)),
                system(4, [900, 900, 0], Some(2)),
                system(5, [960, 980, 40], Some(2)),
                system(6, [1000, 800, 0], Some(2)),
            ],
            clusters: vec![
                ClusterView {
                    id: 1,
                    name: "Near group".into(),
                    x: 100,
                    y: 0,
                },
                ClusterView {
                    id: 2,
                    name: "Far group".into(),
                    x: 1300,
                    y: 200,
                },
            ],
            ..UniverseScene::default()
        };
        let layout = GalaxyLayout::new(&scene);
        assert_eq!(layout.groups.len(), 2);
        let c = system_view_clearance();
        for group in &layout.groups {
            for (a, &i) in group.members.iter().enumerate() {
                for &j in &group.members[a + 1..] {
                    let d = length(sub(layout.cluster[i], layout.cluster[j]));
                    assert!(d >= c * 0.999, "{} {i} {j}: {d}", group.name);
                }
                // Every system lies within the common radius, one of each cluster on it.
                let from = length(sub(layout.cluster[i], group.centre));
                assert!(from <= layout.cluster_radius * 1.001);
            }
            // On the disk the stars lie flat about the centre, inside the footprint.
            for &i in &group.members {
                assert_eq!(layout.disk[i][1], group.centre[1]);
                assert!(length(sub(layout.disk[i], group.centre)) < group.room);
            }
        }
        // Two clusters: the disk levels exist; both lie in one quadrant, so no quadrant level.
        let ladder = layout.ladder;
        assert!(ladder.galaxy.unwrap() > ladder.cluster.unwrap() * MORPH_SPAN.exp());
        assert_eq!(ladder.quadrant, None);
    }

    #[test]
    fn levels_that_show_one_item_are_skipped() {
        let one = |systems: Vec<SystemView>| {
            GalaxyLayout::new(&UniverseScene {
                systems,
                ..UniverseScene::default()
            })
            .ladder
        };
        // One system: planet and system only.
        let solar = one(vec![system(1, [5, 5, 5], Some(3))]);
        assert_eq!((solar.cluster, solar.galaxy), (None, None));
        // One cluster: the cluster view is the end of the zoom.
        let single = one(vec![
            system(1, [5, 5, 5], Some(3)),
            system(2, [95, 5, 5], Some(3)),
        ]);
        assert!(single.cluster.is_some());
        assert_eq!((single.quadrant, single.galaxy), (None, None));
    }
}
