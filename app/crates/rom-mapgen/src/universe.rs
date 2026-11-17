//! A universe: star systems on a star map, the bodies on their orbits and the
//! routes between systems. The systems are a connected region of the real star
//! catalogue (`rom_rules::Star`), in 3D. Integer arithmetic only; the same request
//! always yields the same universe.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use rom_geo::SurfacePos;
use rom_rules::{
    BodyKind, DefId, FeaturePattern, MAX_STAR_MAP_RADIUS, MapStyle, MapStyleFeature,
    PLANET_N_RANGE, RegionKind, Rules, Star,
};
use rom_sim::SimRng;

use crate::{MapGenError, SpherePlanet, SphereRequest, expand_planet, generate_sphere, mix64};

/// Golden angle in millidegrees: successive orbits never line up.
const GOLDEN_MDEG: i64 = 137_508;
/// Full turn in millidegrees.
const TURN_MDEG: i64 = 360_000;
/// Star map layouts tried before the most balanced one is kept.
const ATTEMPTS: u64 = 64;
/// Routes counted from a home system when its resource-body value is measured.
pub const FAIR_HOPS: u32 = 2;
/// The poorest home system must reach this share of the richest one's value.
pub const FAIR_VALUE_PCT: i64 = 80;
/// Most neutral systems in one universe (with at most 64 home systems, every
/// system still gets a distinct name).
const MAX_NEUTRAL_SYSTEMS: i64 = 192;
/// Most bodies orbiting one star.
const MAX_ORBITS: i64 = 64;
/// Home-system candidates are stars whose local density is within this band of the region's median,
/// in percent, so no player starts in an outlier.
const HOME_BAND_PCT: (i64, i64) = (70, 135);
/// Placements of the home systems tried; the fairest, then the widest apart, is kept.
const HOME_TRIALS: usize = 24;
/// The two shortest routes of the poorest home system must reach this share of the richest one's.
pub const FAIR_NEAR_PCT: i64 = 70;
/// A route passes at least this far (map units) from every other system.
const ROUTE_CLEARANCE: i64 = 40;

/// What to generate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UniverseRequest {
    /// The `map_style` definition to follow.
    pub style: DefId,
    /// Seed; the same request always yields the same universe.
    pub seed: u64,
    /// The faction of each player, in player order.
    pub player_factions: Vec<DefId>,
}

/// One star system on the star map (`star_system` row).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StarSystem {
    /// Id, 1-based; equals the index in [`Universe::systems`] plus one.
    pub id: u32,
    /// The star's name from the catalogue.
    pub name: String,
    /// Star map position, abstract units: equatorial frame with the Sun at 0,0,0.
    pub x: i32,
    /// Star map position, abstract units.
    pub y: i32,
    /// Star map position, abstract units; toward the north celestial pole.
    pub z: i32,
    /// The catalogue star (`star.id`) this system is.
    pub star: u32,
    /// The star's constellation, for labels.
    pub constellation: String,
}

/// A star, habitable planet or resource body (`body` row).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Body {
    /// Id, 1-based; equals the index in [`Universe::bodies`] plus one.
    pub id: u32,
    /// The system the body belongs to.
    pub system_id: u32,
    /// Star, habitable or resource.
    pub kind: BodyKind,
    /// Name: the star's, "<star> II" for a planet, "<star> belt" or "<planet> moon" for a
    /// resource body.
    pub name: String,
    /// Distance from the star in system units; 0 for the star.
    pub orbit_radius: i32,
    /// Position on the orbit, 0..360000 millidegrees.
    pub orbit_angle_mdeg: i32,
    /// Cube face size; habitable planets only.
    pub n: Option<u32>,
    /// The race whose players start here; home planets only.
    pub home_race: Option<DefId>,
    /// The node a resource body yields; resource bodies only.
    pub node_entity_type: Option<DefId>,
}

/// A route between two systems (`route` row), `system_a < system_b`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Route {
    /// Lower system id.
    pub system_a: u32,
    /// Higher system id.
    pub system_b: u32,
    /// Integer star map distance.
    pub length: i32,
}

/// A generated universe.
///
/// Home planets are generated eagerly (they hold the starts); every other
/// habitable planet is kept as a recipe and generated on demand by
/// [`Universe::expand`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Universe {
    /// The map style the universe follows.
    pub style: DefId,
    /// Star systems in id order.
    pub systems: Vec<StarSystem>,
    /// Bodies in id order, grouped by system: the star, then bodies by orbit.
    pub bodies: Vec<Body>,
    /// Routes ordered by `(system_a, system_b)`.
    pub routes: Vec<Route>,
    /// Constellation stick-figure lines between systems of this match, `a < b`, in order.
    pub figure: Vec<(u32, u32)>,
    /// The sphere request of every habitable body, keyed by body id.
    pub recipes: BTreeMap<u32, SphereRequest>,
    /// Generated planets keyed by body id: the home planets.
    pub planets: BTreeMap<u32, SpherePlanet>,
    /// One start per player, in player order: home planet body id and tile centre.
    pub starts: Vec<(u32, SurfacePos)>,
}

/// One home system's share in the fairness measure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HomeShare {
    /// The race at home in this system.
    pub race: DefId,
    /// The home system.
    pub system: u32,
    /// Habitable planets in the home system.
    pub habitable: u32,
    /// Summed `et_node.income` of resource bodies within [`FAIR_HOPS`] routes,
    /// not counting other races' home systems.
    pub value: i64,
    /// Summed length of the home system's two shortest routes, map units.
    pub near: i64,
}

type Point = (i64, i64, i64);
/// How good a home placement is, smaller is better: spread of local reach in tens of percent,
/// then the gap between homes, widest first.
type HomeKey = (i64, std::cmp::Reverse<i64>);

fn pick(rng: &mut SimRng, lo: i64, hi: i64) -> i64 {
    lo + i64::from(rng.range(0, (hi - lo + 1) as u32))
}

fn dist2(a: Point, b: Point) -> i64 {
    (a.0 - b.0).pow(2) + (a.1 - b.1).pow(2) + (a.2 - b.2).pow(2)
}

fn isqrt(v: i64) -> i64 {
    (v.max(0) as u64).isqrt() as i64
}

/// Squared distance from `c` to the segment `ab`.
fn segment_dist2(a: Point, b: Point, c: Point) -> i64 {
    let len2 = dist2(a, b);
    let (ab, ac) = (
        [b.0 - a.0, b.1 - a.1, b.2 - a.2],
        [c.0 - a.0, c.1 - a.1, c.2 - a.2],
    );
    let t = ab[0] * ac[0] + ab[1] * ac[1] + ab[2] * ac[2];
    if len2 == 0 || t <= 0 {
        dist2(a, c)
    } else if t >= len2 {
        dist2(b, c)
    } else {
        let x = [
            i128::from(ab[1]) * i128::from(ac[2]) - i128::from(ab[2]) * i128::from(ac[1]),
            i128::from(ab[2]) * i128::from(ac[0]) - i128::from(ab[0]) * i128::from(ac[2]),
            i128::from(ab[0]) * i128::from(ac[1]) - i128::from(ab[1]) * i128::from(ac[0]),
        ];
        ((x[0] * x[0] + x[1] * x[1] + x[2] * x[2]) / i128::from(len2)) as i64
    }
}

fn knobs_ok(style: &MapStyle) -> bool {
    let pairs = [
        (style.neutral_systems_min, style.neutral_systems_max),
        (
            style.habitable_per_system_min,
            style.habitable_per_system_max,
        ),
        (
            style.resource_bodies_per_system_min,
            style.resource_bodies_per_system_max,
        ),
    ];
    pairs.iter().all(|&(lo, hi)| 0 <= lo && lo <= hi)
        && style.habitable_per_system_min >= 1
        && style.habitable_per_system_max + style.resource_bodies_per_system_max <= MAX_ORBITS
        && style.neutral_systems_max <= MAX_NEUTRAL_SYSTEMS
        && style.planet_n_min <= style.planet_n_median
        && style.planet_n_median <= style.planet_n_max
        && PLANET_N_RANGE.contains(&style.planet_n_min)
        && PLANET_N_RANGE.contains(&style.planet_n_max)
        && style.route_extra_pct >= 0
        && (1..=MAX_STAR_MAP_RADIUS).contains(&style.star_map_radius)
}

fn star_point(s: &Star) -> Point {
    (s.x, s.y, s.z)
}

/// A connected region of the catalogue: `count` stars nearest a seeded centre star (ties by
/// id), none farther from the centre than `radius`. `None` when the catalogue cannot supply it.
fn choose_region<'a>(
    rng: &mut SimRng,
    rules: &'a Rules,
    count: usize,
    radius: i64,
) -> Option<Vec<&'a Star>> {
    let all: Vec<&Star> = rules.stars.values().collect();
    if all.len() < count || count == 0 {
        return None;
    }
    let centre = star_point(all[rng.range(0, all.len() as u32) as usize]);
    let mut ranked: Vec<(i64, u32, &Star)> = all
        .iter()
        .map(|&s| (dist2(star_point(s), centre), s.id, s))
        .collect();
    ranked.sort_unstable_by_key(|&(d, id, _)| (d, id));
    ranked.truncate(count);
    (ranked[count - 1].0 <= radius * radius).then(|| ranked.into_iter().map(|r| r.2).collect())
}

/// Sum of the distances from each point to its three nearest neighbours: how crowded its
/// surroundings are.
fn local_reach(pts: &[Point]) -> Vec<i64> {
    pts.iter()
        .enumerate()
        .map(|(i, &p)| {
            let mut d: Vec<i64> = pts
                .iter()
                .enumerate()
                .filter(|&(j, _)| j != i)
                .map(|(_, &q)| isqrt(dist2(p, q)))
                .collect();
            d.sort_unstable();
            d.iter().take(3).sum()
        })
        .collect()
}

/// Home systems among the region's stars so every player starts with a comparable
/// neighbourhood: candidates are stars whose local reach is near the region's median, and of
/// several farthest-point placements the one with the smallest spread of reach, then the widest
/// gap between homes, wins. Returns region indices in increasing order.
fn place_homes(rng: &mut SimRng, pts: &[Point], homes: usize) -> Vec<usize> {
    let reach = local_reach(pts);
    let mut sorted = reach.clone();
    sorted.sort_unstable();
    let median = sorted[sorted.len() / 2];
    let mut pool: Vec<usize> = (0..pts.len())
        .filter(|&i| {
            reach[i] * 100 >= median * HOME_BAND_PCT.0 && reach[i] * 100 <= median * HOME_BAND_PCT.1
        })
        .collect();
    if pool.len() < homes {
        pool = (0..pts.len()).collect();
    }
    let mut best: Option<(HomeKey, Vec<usize>)> = None;
    for _ in 0..HOME_TRIALS {
        let mut chosen = vec![pool[rng.range(0, pool.len() as u32) as usize]];
        while chosen.len() < homes {
            let next = pool
                .iter()
                .copied()
                .filter(|i| !chosen.contains(i))
                .max_by_key(|&i| {
                    let near = chosen
                        .iter()
                        .map(|&c| dist2(pts[c], pts[i]))
                        .min()
                        .unwrap_or(0);
                    (near, std::cmp::Reverse(i))
                })
                .expect("the pool holds at least as many stars as homes");
            chosen.push(next);
        }
        chosen.sort_unstable();
        let low = chosen.iter().map(|&i| reach[i]).min().unwrap_or(0);
        let high = chosen.iter().map(|&i| reach[i]).max().unwrap_or(0);
        let spread = (high - low) * 100 / high.max(1);
        let gap = chosen
            .iter()
            .enumerate()
            .flat_map(|(a, &i)| chosen[a + 1..].iter().map(move |&j| (i, j)))
            .map(|(i, j)| dist2(pts[i], pts[j]))
            .min()
            .unwrap_or(0);
        let key = (spread / 10, std::cmp::Reverse(gap));
        if best.as_ref().is_none_or(|(k, _)| key < *k) {
            best = Some((key, chosen));
        }
    }
    best.map(|(_, c)| c).unwrap_or_default()
}

fn weighted_kind(rng: &mut SimRng, rows: &[&MapStyleFeature]) -> DefId {
    let total: i64 = rows.iter().map(|r| r.chance_pct.max(0)).sum();
    let mut roll = i64::from(rng.range(0, total as u32));
    for row in rows {
        let weight = row.chance_pct.max(0);
        if roll < weight {
            return row.feature_def_id;
        }
        roll -= weight;
    }
    rows[rows.len() - 1].feature_def_id
}

/// Node types of one system's resource bodies; a home system first receives
/// `clumps_min` of every `per_player` kind (never more than the style's maximum).
fn resource_kinds(
    rng: &mut SimRng,
    style: &MapStyle,
    rows: &[&MapStyleFeature],
    home: bool,
) -> Vec<DefId> {
    let mut count = pick(
        rng,
        style.resource_bodies_per_system_min,
        style.resource_bodies_per_system_max,
    ) as usize;
    let mut kinds = Vec::new();
    if home {
        for row in rows
            .iter()
            .filter(|r| r.pattern == FeaturePattern::PerPlayer)
        {
            for _ in 0..row.clumps_min.max(0) {
                kinds.push(row.feature_def_id);
            }
        }
        kinds.truncate(style.resource_bodies_per_system_max as usize);
        count = count.max(kinds.len());
    }
    kinds.truncate(count);
    while kinds.len() < count {
        kinds.push(weighted_kind(rng, rows));
    }
    kinds
}

/// Share of resource bodies that circle a habitable planet as moons, in percent.
const MOON_PCT: u32 = 40;
/// Most moons of one planet.
const MOONS_PER_PLANET: u8 = 2;
/// Tiles of the system plane from a planet to its first moon (the simulation lays a system out
/// within [`PLANE_TILES`] tiles), plus up to [`MOON_SPREAD_TILES`] more.
const MOON_TILES: i64 = 10;
const MOON_SPREAD_TILES: u32 = 3;
/// Tiles from the star to the outermost orbit in the simulation's layout.
const PLANE_TILES: i64 = 240;
/// Innermost orbit radius in system units, and its seeded spread.
const INNER_ORBIT: i64 = 1500;
const INNER_SPREAD: u32 = 500;
/// Each orbit is this many percent of the previous one, plus a seeded spread (140..=190).
const RATIO_PCT: i64 = 140;
const RATIO_SPREAD: u32 = 51;
/// Angle between a moon and its planet in millidegrees; less than the clearance of any two
/// other bodies, so a moon is told from its neighbours by position alone.
pub const MOON_ANGLE_MDEG: i64 = 600;
/// Largest angular clearance bodies other than moons keep from one another.
const CLEARANCE_MDEG: i64 = 5000;

/// What orbits one star and how it is arranged, before any radius is chosen: planets by size
/// outward (small and rocky near the star), a belt of resource bodies in one orbit slot, and
/// resource bodies that circle a planet as moons.
#[derive(Clone)]
struct Arrangement {
    /// Habitable planets from the star outward: face size and whether it is the home planet.
    planets: Vec<(u32, bool)>,
    /// Orbit slot (counted from the star) the belt takes, when any body belongs to it.
    belt_slot: Option<usize>,
    /// Resource bodies and the planet each circles; `None` for belt bodies.
    resources: Vec<(DefId, Option<usize>)>,
}

impl Arrangement {
    fn new(rng: &mut SimRng, mut planets: Vec<(u32, bool)>, kinds: Vec<DefId>) -> Self {
        planets.sort_by_key(|&(n, home)| (n, !home));
        let mut moons = vec![0u8; planets.len()];
        let mut resources = Vec::new();
        for kind in kinds {
            let open: Vec<usize> = (0..planets.len())
                .filter(|&p| moons[p] < MOONS_PER_PLANET)
                .collect();
            let parent = (rng.range(0, 100) < MOON_PCT && !open.is_empty())
                .then(|| open[rng.range(0, open.len() as u32) as usize]);
            if let Some(p) = parent {
                moons[p] += 1;
            }
            resources.push((kind, parent));
        }
        let belt_slot = resources
            .iter()
            .any(|r| r.1.is_none())
            .then(|| rng.range(0, planets.len() as u32 + 1) as usize);
        Self {
            planets,
            belt_slot,
            resources,
        }
    }

    /// The bodies of the system with radii growing geometrically from the star, ids and names
    /// left for the caller.
    fn orbits(&self, rng: &mut SimRng, race: Option<DefId>) -> Vec<Body> {
        let slots = self.planets.len() + usize::from(self.belt_slot.is_some());
        let mut radii = Vec::with_capacity(slots);
        let mut radius = INNER_ORBIT + i64::from(rng.range(0, INNER_SPREAD));
        for _ in 0..slots {
            radii.push(radius);
            radius = radius * (RATIO_PCT + i64::from(rng.range(0, RATIO_SPREAD))) / 100;
        }
        let mut planet_radii = Vec::new();
        let mut belt_radius = 0;
        let mut next = radii.iter();
        for slot in 0..slots {
            let r = *next.next().expect("one radius per slot");
            if Some(slot) == self.belt_slot {
                belt_radius = r;
            } else {
                planet_radii.push(r);
            }
        }
        let count = (self.planets.len() + self.resources.len()) as i64;
        let clearance = CLEARANCE_MDEG.min(TURN_MDEG / (4 * count.max(1)));
        let clear = |angles: &[i64], angle: i64| {
            angles.iter().all(|&a| {
                let d = (angle - a).rem_euclid(TURN_MDEG);
                d.min(TURN_MDEG - d) >= clearance
            })
        };
        let mut angles: Vec<i64> = Vec::new();
        let spot = |angles: &mut Vec<i64>, want: i64| {
            let mut angle = want.rem_euclid(TURN_MDEG);
            while !clear(angles, angle) {
                angle = (angle + clearance) % TURN_MDEG;
            }
            angles.push(angle);
            angle
        };
        let start = i64::from(rng.range(0, TURN_MDEG as u32));
        let mut out = Vec::new();
        let mut planet_angle = Vec::new();
        for (i, &(n, home)) in self.planets.iter().enumerate() {
            let jitter = i64::from(rng.range(0, 6001)) - 3000;
            let angle = spot(&mut angles, start + i as i64 * GOLDEN_MDEG + jitter);
            planet_angle.push(angle);
            out.push(Body {
                id: 0,
                system_id: 0,
                kind: BodyKind::Habitable,
                name: String::new(),
                orbit_radius: planet_radii[i] as i32,
                orbit_angle_mdeg: angle as i32,
                n: Some(n),
                home_race: if home { race } else { None },
                node_entity_type: None,
            });
        }
        let belt_start = i64::from(rng.range(0, TURN_MDEG as u32));
        let width = belt_radius / 6;
        let mut seen: BTreeSet<i64> = planet_radii.iter().copied().collect();
        let mut belt_n = 0;
        let mut moon_n = vec![0usize; self.planets.len()];
        let plane = |radii: &[i64]| radii.iter().copied().max().unwrap_or(0);
        // The outermost orbit decides the simulation's tile scale; moons never extend it.
        let mut belt_radii = Vec::new();
        for &(_, parent) in &self.resources {
            if parent.is_none() {
                let mut r = belt_radius - width / 2 + i64::from(rng.range(0, width.max(1) as u32));
                while !seen.insert(r) {
                    r += 1;
                }
                belt_radii.push(r);
            }
        }
        let outer = plane(&planet_radii).max(plane(&belt_radii)).max(1);
        let scale = ((outer + PLANE_TILES - 1) / PLANE_TILES).max(1);
        let outermost_planet = planet_radii
            .iter()
            .position(|&r| r == plane(&planet_radii))
            .filter(|_| plane(&planet_radii) >= plane(&belt_radii));
        for &(kind, parent) in &self.resources {
            let (radius, angle) = match parent {
                None => {
                    let r = belt_radii[belt_n];
                    let angle = spot(&mut angles, belt_start + belt_n as i64 * GOLDEN_MDEG);
                    belt_n += 1;
                    (r, angle)
                }
                Some(p) => {
                    let m = moon_n[p];
                    moon_n[p] += 1;
                    let reach = (MOON_TILES + i64::from(rng.range(0, MOON_SPREAD_TILES))) * scale;
                    // The outermost planet's moons lie inside its orbit, the others' outside
                    // and then inside, so a moon never extends the system.
                    let offset = match (outermost_planet == Some(p), m) {
                        (true, m) => -reach * (m as i64 + 1),
                        (false, 0) => reach,
                        (false, _) => -reach,
                    };
                    let sign = if m == 0 { 1 } else { -1 };
                    let mut r = planet_radii[p] + offset;
                    while !seen.insert(r) {
                        r += 1;
                    }
                    (
                        r,
                        (planet_angle[p] + sign * MOON_ANGLE_MDEG).rem_euclid(TURN_MDEG),
                    )
                }
            };
            out.push(Body {
                id: 0,
                system_id: 0,
                kind: BodyKind::Resource,
                name: String::new(),
                orbit_radius: radius as i32,
                orbit_angle_mdeg: angle as i32,
                n: None,
                home_race: None,
                node_entity_type: Some(kind),
            });
        }
        out.sort_by_key(|b| (b.orbit_radius, b.orbit_angle_mdeg));
        out
    }
}

struct Layout {
    systems: Vec<StarSystem>,
    bodies: Vec<Body>,
    routes: Vec<Route>,
    figure: Vec<(u32, u32)>,
}

/// May the route `a`-`b` join the map? It passes clear of every other system and, unless
/// `lune` is false, no system lies nearer to both ends than they are to each other (the relative
/// neighbourhood rule: only real neighbours are joined).
fn can_add(
    edges: &BTreeSet<(u32, u32)>,
    pos: &[Point],
    a: u32,
    b: u32,
    clearance: i64,
    lune: bool,
) -> bool {
    let (pa, pb) = (pos[a as usize - 1], pos[b as usize - 1]);
    let ab = dist2(pa, pb);
    !edges.contains(&(a.min(b), a.max(b)))
        && pos.iter().enumerate().all(|(i, &p)| {
            let id = i as u32 + 1;
            id == a
                || id == b
                || (segment_dist2(pa, pb, p) >= clearance * clearance
                    && !(lune && dist2(pa, p).max(dist2(pb, p)) < ab))
        })
}

/// A spanning tree of the nearest pairs, then routes so every home system has two (with more
/// than two systems), then the shortest extra routes between real neighbours.
fn connect(pos: &[Point], homes: usize, extra_pct: i64, clearance: i64) -> Option<Vec<Route>> {
    let k = pos.len() as u32;
    let mut candidates: Vec<(i64, u32, u32)> = Vec::new();
    for a in 1..=k {
        for b in a + 1..=k {
            candidates.push((dist2(pos[a as usize - 1], pos[b as usize - 1]), a, b));
        }
    }
    candidates.sort_unstable();
    let mut parent: Vec<u32> = (0..=k).collect();
    fn root(parent: &mut [u32], mut v: u32) -> u32 {
        while parent[v as usize] != v {
            parent[v as usize] = parent[parent[v as usize] as usize];
            v = parent[v as usize];
        }
        v
    }
    let mut edges = BTreeSet::new();
    for &(_, a, b) in &candidates {
        let (ra, rb) = (root(&mut parent, a), root(&mut parent, b));
        if ra != rb {
            parent[ra as usize] = rb;
            edges.insert((a, b));
        }
    }
    let tree = edges.len();
    let degree = |edges: &BTreeSet<(u32, u32)>, s: u32| {
        edges.iter().filter(|&&(a, b)| a == s || b == s).count()
    };
    if k > 2 {
        for home in 1..=homes as u32 {
            while degree(&edges, home) < 2 {
                let next = candidates
                    .iter()
                    .filter(|&&(_, a, b)| a == home || b == home)
                    .find(|&&(_, a, b)| can_add(&edges, pos, a, b, clearance, false))?;
                edges.insert((next.1, next.2));
            }
        }
    }
    let target = (tree as i64 * extra_pct + 50) / 100;
    for &(_, a, b) in &candidates {
        if (edges.len() - tree) as i64 >= target {
            break;
        }
        if can_add(&edges, pos, a, b, clearance, true) {
            edges.insert((a, b));
        }
    }
    Some(
        edges
            .into_iter()
            .map(|(a, b)| Route {
                system_a: a,
                system_b: b,
                length: isqrt(dist2(pos[a as usize - 1], pos[b as usize - 1])).max(1) as i32,
            })
            .collect(),
    )
}

/// Roman numeral of a small positive number: the planets of a system in orbit order.
fn roman(mut n: u32) -> String {
    const DIGITS: [(u32, &str); 13] = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut out = String::new();
    for (value, text) in DIGITS {
        while n >= value {
            out.push_str(text);
            n -= value;
        }
    }
    out
}

/// Names the bodies of one system, in orbit order: planets "<star> II", resource bodies
/// "<star> belt" or, beside a planet, "<planet> moon"; a second of a kind gets a letter.
fn name_bodies(star: &str, bodies: &mut [Body]) {
    let mut planets: Vec<(i32, i32, String)> = Vec::new();
    let mut numeral = 0;
    for b in bodies.iter_mut().filter(|b| b.kind == BodyKind::Habitable) {
        numeral += 1;
        b.name = format!("{star} {}", roman(numeral));
        planets.push((b.orbit_radius, b.orbit_angle_mdeg, b.name.clone()));
    }
    let mut counts: BTreeMap<String, u32> = BTreeMap::new();
    for b in bodies.iter_mut().filter(|b| b.kind == BodyKind::Resource) {
        let host = planets.iter().find(|(_, angle, _)| {
            let d = (b.orbit_angle_mdeg - angle).rem_euclid(TURN_MDEG as i32);
            d.min(TURN_MDEG as i32 - d) <= MOON_ANGLE_MDEG as i32
        });
        let base = match host {
            Some((_, _, planet)) => format!("{planet} moon"),
            None => format!("{star} belt"),
        };
        let seen = counts.entry(base.clone()).or_insert(0);
        b.name = match *seen {
            0 => base,
            n => format!("{base} {}", char::from(b'a' + n as u8)),
        };
        *seen += 1;
    }
}

fn layout(
    rng: &mut SimRng,
    rules: &Rules,
    style: &MapStyle,
    races: &[DefId],
    rows: &[&MapStyleFeature],
) -> Option<Layout> {
    let homes = races.len();
    let neutral = pick(rng, style.neutral_systems_min, style.neutral_systems_max) as usize;
    let k = homes + neutral;
    let region = choose_region(rng, rules, k, style.star_map_radius)?;
    let pts: Vec<Point> = region.iter().map(|s| star_point(s)).collect();
    let chosen = place_homes(rng, &pts, homes);
    if chosen.len() != homes {
        return None;
    }
    // Homes first, then the other stars, each group by catalogue id.
    let mut order: Vec<usize> = chosen.clone();
    order.sort_by_key(|&i| region[i].id);
    let mut rest: Vec<usize> = (0..k).filter(|i| !chosen.contains(i)).collect();
    rest.sort_by_key(|&i| region[i].id);
    order.extend(rest);
    let stars: Vec<&Star> = order.iter().map(|&i| region[i]).collect();
    let pos: Vec<Point> = stars.iter().map(|s| star_point(s)).collect();
    let routes = connect(&pos, homes, style.route_extra_pct, ROUTE_CLEARANCE)?;
    let systems: Vec<StarSystem> = stars
        .iter()
        .enumerate()
        .map(|(i, s)| StarSystem {
            id: i as u32 + 1,
            name: s.name.clone(),
            x: s.x as i32,
            y: s.y as i32,
            z: s.z as i32,
            star: s.id,
            constellation: rules
                .constellations
                .get(&s.constellation_id)
                .map(|c| c.name.clone())
                .unwrap_or_default(),
        })
        .collect();
    let system_of: BTreeMap<u32, u32> = systems.iter().map(|s| (s.star, s.id)).collect();
    let figure: Vec<(u32, u32)> = rules
        .constellation_lines
        .iter()
        .filter_map(|l| {
            let (a, b) = (system_of.get(&l.star_a)?, system_of.get(&l.star_b)?);
            Some((*a.min(b), *a.max(b)))
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    // Every home system gets the same contents and the same arrangement (planets by size,
    // belt slot, moons); only the orbit radii and angles differ.
    let n_pick = |rng: &mut SimRng| pick(rng, style.planet_n_min, style.planet_n_max) as u32;
    let home_habitable = pick(
        rng,
        style.habitable_per_system_min,
        style.habitable_per_system_max,
    ) as usize;
    let mut home_planets: Vec<(u32, bool)> = vec![(style.planet_n_median as u32, true)];
    home_planets.extend((1..home_habitable).map(|_| (n_pick(rng), false)));
    let home_resources = resource_kinds(rng, style, rows, true);
    let home_plan = Arrangement::new(rng, home_planets, home_resources);

    let mut bodies = Vec::new();
    for system in &systems {
        let (plan, race) = if let Some(&race) = races.get(system.id as usize - 1) {
            (home_plan.clone(), Some(race))
        } else {
            let habitable = pick(
                rng,
                style.habitable_per_system_min,
                style.habitable_per_system_max,
            );
            let planets = (0..habitable).map(|_| (n_pick(rng), false)).collect();
            let kinds = resource_kinds(rng, style, rows, false);
            (Arrangement::new(rng, planets, kinds), None)
        };
        bodies.push(Body {
            id: bodies.len() as u32 + 1,
            system_id: system.id,
            kind: BodyKind::Star,
            name: system.name.clone(),
            orbit_radius: 0,
            orbit_angle_mdeg: 0,
            n: None,
            home_race: None,
            node_entity_type: None,
        });
        let mut orbits = plan.orbits(rng, race);
        name_bodies(&system.name, &mut orbits);
        for mut body in orbits {
            body.id = bodies.len() as u32 + 1;
            body.system_id = system.id;
            bodies.push(body);
        }
    }
    Some(Layout {
        systems,
        bodies,
        routes,
        figure,
    })
}

struct Checked<'a> {
    rules: &'a Rules,
    style: &'a MapStyle,
    races: Vec<DefId>,
    player_races: Vec<DefId>,
    rows: Vec<&'a MapStyleFeature>,
}

fn check_request<'a>(
    rules: &'a Rules,
    request: &UniverseRequest,
) -> Result<Checked<'a>, MapGenError> {
    let style = rules
        .map_styles
        .get(&request.style)
        .ok_or(MapGenError::UnknownStyle(request.style))?;
    if !(1..=64).contains(&request.player_factions.len()) {
        return Err(MapGenError::InvalidRequest("players must be 1..=64"));
    }
    if request
        .player_factions
        .iter()
        .any(|f| !rules.factions.contains_key(f))
    {
        return Err(MapGenError::InvalidRequest("unknown faction"));
    }
    if !knobs_ok(style) {
        return Err(MapGenError::InvalidRequest("universe knobs out of range"));
    }
    let player_races: Vec<DefId> = request
        .player_factions
        .iter()
        .map(|&f| rules.race_of(f))
        .collect();
    let races: Vec<DefId> = player_races
        .iter()
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let mut rows: Vec<&MapStyleFeature> = rules
        .map_style_features
        .get(&request.style)
        .into_iter()
        .flatten()
        .filter(|r| r.region_kind == RegionKind::ResourceBody)
        .collect();
    rows.sort_by_key(|r| r.id);
    if let Some(row) = rows.iter().find(|r| {
        rules
            .entity_types
            .get(&r.feature_def_id)
            .is_none_or(|e| e.node.is_none())
    }) {
        return Err(MapGenError::UnsupportedFeature(row.feature_def_id));
    }
    rows.retain(|r| r.chance_pct > 0);
    if rows.is_empty() && style.resource_bodies_per_system_max > 0 {
        return Err(MapGenError::InvalidRequest(
            "resource bodies wanted but no resource_body feature has a chance",
        ));
    }
    Ok(Checked {
        rules,
        style,
        races,
        player_races,
        rows,
    })
}

/// Lays out the star map, bodies and routes and records a recipe for every
/// habitable planet, without generating any planet surface.
pub fn plan_universe(rules: &Rules, request: &UniverseRequest) -> Result<Universe, MapGenError> {
    let checked = check_request(rules, request)?;
    plan(&checked, request)
}

fn plan(c: &Checked, request: &UniverseRequest) -> Result<Universe, MapGenError> {
    let mut best: Option<(i64, Universe)> = None;
    for attempt in 0..ATTEMPTS {
        let mut rng = SimRng::new(mix64(
            request.seed ^ attempt.wrapping_mul(0xd1b5_4a32_d192_ed03),
        ));
        let Some(layout) = layout(&mut rng, c.rules, c.style, &c.races, &c.rows) else {
            continue;
        };
        let mut universe = Universe {
            style: request.style,
            systems: layout.systems,
            bodies: layout.bodies,
            routes: layout.routes,
            figure: layout.figure,
            recipes: BTreeMap::new(),
            planets: BTreeMap::new(),
            starts: Vec::new(),
        };
        for body in &universe.bodies {
            let Some(n) = body.n else { continue };
            let player_factions = request
                .player_factions
                .iter()
                .zip(&c.player_races)
                .filter(|&(_, race)| Some(*race) == body.home_race)
                .map(|(&f, _)| f)
                .collect();
            let seed = mix64(request.seed ^ u64::from(body.id).wrapping_mul(0x9e37_79b9_7f4a_7c15));
            universe.recipes.insert(
                body.id,
                SphereRequest {
                    style: request.style,
                    seed,
                    n,
                    player_factions,
                },
            );
        }
        let shares = universe.home_shares(c.rules);
        let low = shares.iter().map(|s| s.value).min().unwrap_or(0);
        let high = shares.iter().map(|s| s.value).max().unwrap_or(0);
        let score = if high == 0 { 1000 } else { low * 1000 / high };
        let fair = universe.layout_problems(c.rules).is_empty();
        if fair {
            return Ok(universe);
        }
        if best.as_ref().is_none_or(|(s, _)| score > *s) {
            best = Some((score, universe));
        }
    }
    best.map(|(_, u)| u).ok_or(MapGenError::StarMap(
        c.races.len() as u32 + c.style.neutral_systems_max as u32,
    ))
}

/// Generates a universe: the star map, bodies and routes, then every home
/// planet (median size, shared by all players of its race) with its starts.
pub fn generate_universe(
    rules: &Rules,
    request: &UniverseRequest,
) -> Result<Universe, MapGenError> {
    let checked = check_request(rules, request)?;
    let mut universe = plan(&checked, request)?;
    let mut starts = vec![None; request.player_factions.len()];
    for body in universe.bodies.iter().filter(|b| b.home_race.is_some()) {
        let recipe = &universe.recipes[&body.id];
        let planet = generate_sphere(rules, recipe)?;
        let players = checked
            .player_races
            .iter()
            .enumerate()
            .filter(|&(_, race)| Some(*race) == body.home_race)
            .map(|(p, _)| p);
        for (p, &pos) in players.zip(&planet.starts) {
            starts[p] = Some((body.id, pos));
        }
        universe.planets.insert(body.id, planet);
    }
    universe.starts = starts
        .into_iter()
        .map(|s| s.ok_or(MapGenError::NoRoomForPlayers(1)))
        .collect::<Result<_, _>>()?;
    Ok(universe)
}

impl Universe {
    /// The planet of a habitable body: the generated one, or its recipe expanded now.
    pub fn expand(&self, rules: &Rules, body: u32) -> Result<SpherePlanet, MapGenError> {
        if let Some(planet) = self.planets.get(&body) {
            return Ok(planet.clone());
        }
        let recipe = self
            .recipes
            .get(&body)
            .ok_or(MapGenError::InvalidRequest("not a habitable body"))?;
        expand_planet(rules, recipe)
    }

    /// Home systems as `(race, system id)`, ordered by race.
    pub fn home_systems(&self) -> Vec<(DefId, u32)> {
        let mut homes: Vec<_> = self
            .bodies
            .iter()
            .filter_map(|b| b.home_race.map(|race| (race, b.system_id)))
            .collect();
        homes.sort_unstable();
        homes
    }

    /// Routes needed to reach every system from `from`, indexed by system id − 1.
    pub fn hops_from(&self, from: u32) -> Vec<Option<u32>> {
        let mut hops = vec![None; self.systems.len()];
        let mut queue = VecDeque::from([from]);
        hops[from as usize - 1] = Some(0);
        while let Some(s) = queue.pop_front() {
            let here = hops[s as usize - 1].unwrap_or(0);
            for r in &self.routes {
                let next = if r.system_a == s {
                    r.system_b
                } else if r.system_b == s {
                    r.system_a
                } else {
                    continue;
                };
                if hops[next as usize - 1].is_none() {
                    hops[next as usize - 1] = Some(here + 1);
                    queue.push_back(next);
                }
            }
        }
        hops
    }

    /// Number of routes touching a system.
    pub fn degree(&self, system: u32) -> usize {
        self.routes
            .iter()
            .filter(|r| r.system_a == system || r.system_b == system)
            .count()
    }

    /// The fairness measure of every home system, ordered by race.
    pub fn home_shares(&self, rules: &Rules) -> Vec<HomeShare> {
        let homes = self.home_systems();
        let home_ids: BTreeSet<u32> = homes.iter().map(|&(_, s)| s).collect();
        homes
            .iter()
            .map(|&(race, system)| {
                let hops = self.hops_from(system);
                let habitable = self
                    .bodies
                    .iter()
                    .filter(|b| b.system_id == system && b.kind == BodyKind::Habitable)
                    .count() as u32;
                let value = self
                    .bodies
                    .iter()
                    .filter(|b| {
                        hops[b.system_id as usize - 1].is_some_and(|h| h <= FAIR_HOPS)
                            && (b.system_id == system || !home_ids.contains(&b.system_id))
                    })
                    .filter_map(|b| b.node_entity_type)
                    .filter_map(|d| rules.entity_types.get(&d)?.node.as_ref())
                    .map(|node| node.income)
                    .sum();
                let mut lengths: Vec<i64> = self
                    .routes
                    .iter()
                    .filter(|r| r.system_a == system || r.system_b == system)
                    .map(|r| i64::from(r.length))
                    .collect();
                lengths.sort_unstable();
                let near = lengths.iter().take(2).sum();
                HomeShare {
                    race,
                    system,
                    habitable,
                    value,
                    near,
                }
            })
            .collect()
    }

    fn layout_problems(&self, rules: &Rules) -> Vec<String> {
        let mut problems = Vec::new();
        if let Some(first) = self.systems.first()
            && self.hops_from(first.id).iter().any(Option::is_none)
        {
            problems.push("not every system is reachable".to_owned());
        }
        let shares = self.home_shares(rules);
        for s in &shares {
            if self.systems.len() > 2 && self.degree(s.system) < 2 {
                problems.push(format!(
                    "home system {} has fewer than two routes",
                    s.system
                ));
            }
        }
        if let (Some(a), Some(b)) = (
            shares.iter().map(|s| s.habitable).min(),
            shares.iter().map(|s| s.habitable).max(),
        ) && a != b
        {
            problems.push(format!("home systems hold {a} to {b} habitable planets"));
        }
        let low = shares.iter().map(|s| s.value).min().unwrap_or(0);
        let high = shares.iter().map(|s| s.value).max().unwrap_or(0);
        if low * 100 < high * FAIR_VALUE_PCT {
            problems.push(format!(
                "resource-body value within {FAIR_HOPS} routes ranges {low} to {high}"
            ));
        }
        let low = shares.iter().map(|s| s.near).min().unwrap_or(0);
        let high = shares.iter().map(|s| s.near).max().unwrap_or(0);
        if low * 100 < high * FAIR_NEAR_PCT {
            problems.push(format!(
                "the two shortest routes of a home system total {low} to {high}"
            ));
        }
        problems
    }

    /// Problems that make the universe unfair; empty means fair. Checks the
    /// star map (reachability, two routes per home system, equal habitable
    /// counts, comparable resource-body value within two routes) and every
    /// generated home planet.
    pub fn fairness(&self, rules: &Rules) -> Vec<String> {
        let mut problems = self.layout_problems(rules);
        for (body, planet) in &self.planets {
            for p in planet.fairness(rules) {
                problems.push(format!("planet {body}: {p}"));
            }
        }
        problems
    }

    /// FNV-1a 64 over systems, bodies, routes, recipes, generated planets and starts.
    pub fn hash(&self) -> u64 {
        let mut h = 0xcbf29ce484222325_u64;
        let mut feed = |bytes: &[u8]| {
            for &b in bytes {
                h = (h ^ u64::from(b)).wrapping_mul(0x100000001b3);
            }
        };
        let opt = |v: Option<u32>| {
            v.map_or([0; 5], |v| {
                let b = v.to_le_bytes();
                [1, b[0], b[1], b[2], b[3]]
            })
        };
        feed(&self.style.0.to_le_bytes());
        feed(&(self.systems.len() as u64).to_le_bytes());
        for s in &self.systems {
            feed(&s.id.to_le_bytes());
            feed(&(s.name.len() as u64).to_le_bytes());
            feed(s.name.as_bytes());
            feed(&s.x.to_le_bytes());
            feed(&s.y.to_le_bytes());
            feed(&s.z.to_le_bytes());
            feed(&s.star.to_le_bytes());
            feed(&(s.constellation.len() as u64).to_le_bytes());
            feed(s.constellation.as_bytes());
        }
        feed(&(self.bodies.len() as u64).to_le_bytes());
        for b in &self.bodies {
            feed(&b.id.to_le_bytes());
            feed(&b.system_id.to_le_bytes());
            feed(b.kind.key().as_bytes());
            feed(&(b.name.len() as u64).to_le_bytes());
            feed(b.name.as_bytes());
            feed(&b.orbit_radius.to_le_bytes());
            feed(&b.orbit_angle_mdeg.to_le_bytes());
            feed(&opt(b.n));
            feed(&opt(b.home_race.map(|d| d.0)));
            feed(&opt(b.node_entity_type.map(|d| d.0)));
        }
        feed(&(self.routes.len() as u64).to_le_bytes());
        for r in &self.routes {
            feed(&r.system_a.to_le_bytes());
            feed(&r.system_b.to_le_bytes());
            feed(&r.length.to_le_bytes());
        }
        feed(&(self.figure.len() as u64).to_le_bytes());
        for (a, b) in &self.figure {
            feed(&a.to_le_bytes());
            feed(&b.to_le_bytes());
        }
        for (id, r) in &self.recipes {
            feed(&id.to_le_bytes());
            feed(&r.seed.to_le_bytes());
            feed(&r.n.to_le_bytes());
            feed(&(r.player_factions.len() as u64).to_le_bytes());
            for f in &r.player_factions {
                feed(&f.0.to_le_bytes());
            }
        }
        for (id, p) in &self.planets {
            feed(&id.to_le_bytes());
            feed(&p.hash().to_le_bytes());
        }
        for (id, p) in &self.starts {
            feed(&id.to_le_bytes());
            feed(&[p.face]);
            feed(&p.x.to_le_bytes());
            feed(&p.y.to_le_bytes());
        }
        h
    }
}

#[cfg(test)]
mod tests {
    use super::{roman, segment_dist2};

    #[test]
    fn distance_to_a_segment_in_3d() {
        // Beside the middle, beyond either end, and on the segment.
        assert_eq!(segment_dist2((0, 0, 0), (10, 0, 0), (5, 3, 4)), 25);
        assert_eq!(segment_dist2((0, 0, 0), (10, 0, 0), (-3, 4, 0)), 25);
        assert_eq!(segment_dist2((0, 0, 0), (10, 0, 0), (13, 0, 4)), 25);
        assert_eq!(segment_dist2((0, 0, 0), (10, 0, 0), (4, 0, 0)), 0);
    }

    #[test]
    fn planets_take_roman_numerals() {
        let all: Vec<String> = [1, 2, 3, 4, 9, 14, 40].map(roman).into();
        assert_eq!(all, ["I", "II", "III", "IV", "IX", "XIV", "XL"]);
    }
}
