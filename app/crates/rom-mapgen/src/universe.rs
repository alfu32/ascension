//! A universe: star systems on a star map, the bodies on their orbits and the
//! routes between systems. Integer arithmetic only; the same request always
//! yields the same universe.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use rom_geo::SurfacePos;
use rom_rules::{
    BodyKind, DefId, FeaturePattern, MAX_STAR_MAP_RADIUS, MapStyle, MapStyleFeature,
    PLANET_N_RANGE, RegionKind, Rules,
};
use rom_sim::SimRng;

use crate::{MapGenError, SpherePlanet, SphereRequest, expand_planet, generate_sphere, mix64};

/// System units between successive orbits around a star.
const ORBIT_STEP: i32 = 1000;
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
/// Invented syllables for system names: an onset and a coda make a name.
const ONSETS: [&str; 22] = [
    "Bra", "Cel", "Dov", "Ern", "Fol", "Grim", "Hal", "Isk", "Jun", "Kor", "Lun", "Mav", "Nor",
    "Ost", "Pel", "Quil", "Rud", "Teg", "Tam", "Ulv", "Vor", "Wyn",
];
const CODAS: [&str; 15] = [
    "ad", "ber", "dis", "en", "gar", "holm", "ik", "mar", "nel", "ost", "rin", "sk", "tal", "ven",
    "wick",
];

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
    /// Invented name.
    pub name: String,
    /// Star map position, abstract units; the map is a disc centred on 0,0.
    pub x: i32,
    /// Star map position, abstract units.
    pub y: i32,
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
    /// Invented name.
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
}

type Point = (i64, i64);

fn pick(rng: &mut SimRng, lo: i64, hi: i64) -> i64 {
    lo + i64::from(rng.range(0, (hi - lo + 1) as u32))
}

fn dist2(a: Point, b: Point) -> i64 {
    (a.0 - b.0).pow(2) + (a.1 - b.1).pow(2)
}

fn isqrt(v: i64) -> i64 {
    (v.max(0) as u64).isqrt() as i64
}

fn cross(o: Point, a: Point, b: Point) -> i128 {
    i128::from(a.0 - o.0) * i128::from(b.1 - o.1) - i128::from(a.1 - o.1) * i128::from(b.0 - o.0)
}

fn on_segment(a: Point, b: Point, c: Point) -> bool {
    c.0 >= a.0.min(b.0) && c.0 <= a.0.max(b.0) && c.1 >= a.1.min(b.1) && c.1 <= a.1.max(b.1)
}

/// True when segments `ab` and `cd` meet anywhere except at a shared endpoint.
pub(crate) fn segments_cross(a: Point, b: Point, c: Point, d: Point) -> bool {
    let shared = [(a, b, c, d), (a, b, d, c), (b, a, c, d), (b, a, d, c)]
        .into_iter()
        .find(|(p, _, s, _)| p == s);
    if let Some((p, q, _, r)) = shared {
        // Two segments from one point overlap only when they leave it in the same direction.
        let dot = (q.0 - p.0) * (r.0 - p.0) + (q.1 - p.1) * (r.1 - p.1);
        return cross(p, q, r) == 0 && dot > 0;
    }
    let (o1, o2) = (cross(a, b, c).signum(), cross(a, b, d).signum());
    let (o3, o4) = (cross(c, d, a).signum(), cross(c, d, b).signum());
    if o1 * o2 < 0 && o3 * o4 < 0 {
        return true;
    }
    (o1 == 0 && on_segment(a, b, c))
        || (o2 == 0 && on_segment(a, b, d))
        || (o3 == 0 && on_segment(c, d, a))
        || (o4 == 0 && on_segment(c, d, b))
}

/// Squared distance from `c` to the segment `ab`.
fn segment_dist2(a: Point, b: Point, c: Point) -> i64 {
    let len2 = dist2(a, b);
    let t = (c.0 - a.0) * (b.0 - a.0) + (c.1 - a.1) * (b.1 - a.1);
    if len2 == 0 || t <= 0 {
        dist2(a, c)
    } else if t >= len2 {
        dist2(b, c)
    } else {
        let x = cross(a, b, c);
        (x * x / i128::from(len2)) as i64
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

/// Uniform integer points in the star map disc.
fn sample_disc(rng: &mut SimRng, radius: i64, count: usize) -> Vec<Point> {
    let mut out = Vec::with_capacity(count);
    while out.len() < count {
        let x = pick(rng, -radius, radius);
        let y = pick(rng, -radius, radius);
        if x * x + y * y <= radius * radius {
            out.push((x, y));
        }
    }
    out
}

/// Home positions as far apart as practical: farthest-point selection from
/// several random first picks, keeping the layout with the widest closest pair.
fn place_homes(rng: &mut SimRng, pool: &[Point], homes: usize) -> Vec<Point> {
    if homes == 1 {
        return vec![pool[rng.range(0, pool.len() as u32) as usize]];
    }
    let mut best = Vec::new();
    let mut best_gap = -1;
    for _ in 0..16 {
        let mut chosen = vec![pool[rng.range(0, pool.len() as u32) as usize]];
        while chosen.len() < homes {
            let next = pool
                .iter()
                .copied()
                .enumerate()
                .max_by_key(|&(i, p)| {
                    let near = chosen.iter().map(|&c| dist2(c, p)).min().unwrap_or(0);
                    (near, std::cmp::Reverse(i))
                })
                .map(|(_, p)| p)
                .expect("pool is not empty");
            chosen.push(next);
        }
        let gap = chosen
            .iter()
            .enumerate()
            .flat_map(|(i, &a)| chosen[i + 1..].iter().map(move |&b| dist2(a, b)))
            .min()
            .unwrap_or(0);
        if gap > best_gap {
            best_gap = gap;
            best = chosen;
        }
    }
    best
}

/// Neutral positions keeping a minimum separation (Poisson-disc style dart
/// throwing over a shuffled pool), relaxing the separation when the pool runs out.
fn place_neutrals(
    rng: &mut SimRng,
    pool: &[Point],
    homes: &[Point],
    count: usize,
    sep: i64,
) -> Vec<Point> {
    let mut order: Vec<usize> = (0..pool.len()).collect();
    for i in (1..order.len()).rev() {
        order.swap(i, rng.range(0, i as u32 + 1) as usize);
    }
    let mut sep = sep;
    loop {
        let mut placed: Vec<Point> = Vec::with_capacity(count);
        for &i in &order {
            if placed.len() == count {
                break;
            }
            let p = pool[i];
            if homes
                .iter()
                .chain(&placed)
                .all(|&q| dist2(p, q) >= sep * sep)
            {
                placed.push(p);
            }
        }
        if placed.len() == count || sep <= 1 {
            return placed;
        }
        sep = sep * 9 / 10;
    }
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

enum Orbiting {
    Habitable(u32, Option<DefId>),
    Resource(DefId),
}

struct Layout {
    systems: Vec<StarSystem>,
    bodies: Vec<Body>,
    routes: Vec<Route>,
}

fn can_add(edges: &BTreeSet<(u32, u32)>, pos: &[Point], a: u32, b: u32, clearance: i64) -> bool {
    let (pa, pb) = (pos[a as usize - 1], pos[b as usize - 1]);
    !edges.contains(&(a.min(b), a.max(b)))
        && edges
            .iter()
            .all(|&(c, d)| !segments_cross(pa, pb, pos[c as usize - 1], pos[d as usize - 1]))
        && pos.iter().enumerate().all(|(i, &p)| {
            let id = i as u32 + 1;
            id == a || id == b || segment_dist2(pa, pb, p) >= clearance * clearance
        })
}

/// A spanning tree, then routes so every home system has two (with more than
/// two systems), then the shortest non-crossing extras.
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
                    .find(|&&(_, a, b)| can_add(&edges, pos, a, b, clearance))?;
                edges.insert((next.1, next.2));
            }
        }
    }
    let target = (tree as i64 * extra_pct + 50) / 100;
    for &(_, a, b) in &candidates {
        if (edges.len() - tree) as i64 >= target {
            break;
        }
        if can_add(&edges, pos, a, b, clearance) {
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

fn names(rng: &mut SimRng, count: usize) -> Vec<String> {
    // Joins that double a letter ("Ostost", "Isksk") read badly and are left out.
    let mut pool: Vec<String> = ONSETS
        .iter()
        .flat_map(|o| CODAS.iter().map(move |c| (o, c)))
        .filter(|(o, c)| !o.to_lowercase().ends_with(&c[..1]))
        .map(|(o, c)| format!("{o}{c}"))
        .collect();
    (0..count)
        .map(|_| pool.swap_remove(rng.range(0, pool.len() as u32) as usize))
        .collect()
}

fn layout(
    rng: &mut SimRng,
    style: &MapStyle,
    races: &[DefId],
    rows: &[&MapStyleFeature],
) -> Option<Layout> {
    let homes = races.len();
    let neutral = pick(rng, style.neutral_systems_min, style.neutral_systems_max) as usize;
    let k = homes + neutral;
    let radius = style.star_map_radius;
    let sep = (radius * 1000 / isqrt(k as i64 * 1_000_000)).max(1);
    let pool = sample_disc(rng, radius, 48 * k);
    let mut pos = place_homes(rng, &pool, homes);
    let neutrals = place_neutrals(rng, &pool, &pos, neutral, sep);
    if neutrals.len() < neutral {
        return None;
    }
    pos.extend(neutrals);
    let routes = connect(&pos, homes, style.route_extra_pct, sep / 4)?;
    let systems: Vec<StarSystem> = names(rng, k)
        .into_iter()
        .zip(&pos)
        .enumerate()
        .map(|(i, (name, &(x, y)))| StarSystem {
            id: i as u32 + 1,
            name,
            x: x as i32,
            y: y as i32,
        })
        .collect();

    // Every home system gets the same contents; only the orbits differ.
    let n_pick = |rng: &mut SimRng| pick(rng, style.planet_n_min, style.planet_n_max) as u32;
    let home_habitable = pick(
        rng,
        style.habitable_per_system_min,
        style.habitable_per_system_max,
    ) as usize;
    let home_sizes: Vec<u32> = (1..home_habitable).map(|_| n_pick(rng)).collect();
    let home_resources = resource_kinds(rng, style, rows, true);

    let mut bodies = Vec::new();
    for system in &systems {
        let mut orbiting = Vec::new();
        if let Some(&race) = races.get(system.id as usize - 1) {
            orbiting.push(Orbiting::Habitable(
                style.planet_n_median as u32,
                Some(race),
            ));
            orbiting.extend(home_sizes.iter().map(|&n| Orbiting::Habitable(n, None)));
            orbiting.extend(home_resources.iter().map(|&d| Orbiting::Resource(d)));
        } else {
            let habitable = pick(
                rng,
                style.habitable_per_system_min,
                style.habitable_per_system_max,
            );
            for _ in 0..habitable {
                orbiting.push(Orbiting::Habitable(n_pick(rng), None));
            }
            let kinds = resource_kinds(rng, style, rows, false);
            orbiting.extend(kinds.into_iter().map(Orbiting::Resource));
        }
        for i in (1..orbiting.len()).rev() {
            orbiting.swap(i, rng.range(0, i as u32 + 1) as usize);
        }
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
        let start = i64::from(rng.range(0, TURN_MDEG as u32));
        let mut angles = BTreeSet::new();
        for (i, body) in orbiting.into_iter().enumerate() {
            let radius =
                ORBIT_STEP * (i as i32 + 1) + rng.range(0, ORBIT_STEP as u32 * 2 / 5) as i32;
            let jitter = i64::from(rng.range(0, 6001)) - 3000;
            let mut angle = (start + i as i64 * GOLDEN_MDEG + jitter).rem_euclid(TURN_MDEG);
            while !angles.insert(angle) {
                angle = (angle + 1) % TURN_MDEG;
            }
            let (kind, n, home_race, node) = match body {
                Orbiting::Habitable(n, race) => (BodyKind::Habitable, Some(n), race, None),
                Orbiting::Resource(d) => (BodyKind::Resource, None, None, Some(d)),
            };
            bodies.push(Body {
                id: bodies.len() as u32 + 1,
                system_id: system.id,
                kind,
                name: format!("{} {}", system.name, i + 1),
                orbit_radius: radius,
                orbit_angle_mdeg: angle as i32,
                n,
                home_race,
                node_entity_type: node,
            });
        }
    }
    Some(Layout {
        systems,
        bodies,
        routes,
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
        let Some(layout) = layout(&mut rng, c.style, &c.races, &c.rows) else {
            continue;
        };
        let mut universe = Universe {
            style: request.style,
            systems: layout.systems,
            bodies: layout.bodies,
            routes: layout.routes,
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
                HomeShare {
                    race,
                    system,
                    habitable,
                    value,
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
    use super::segments_cross;

    #[test]
    fn crossing_rules() {
        assert!(segments_cross((0, 0), (10, 10), (0, 10), (10, 0)));
        assert!(!segments_cross((0, 0), (10, 0), (0, 5), (10, 5)));
        // Sharing an endpoint is not a crossing, unless the segments overlap.
        assert!(!segments_cross((0, 0), (10, 0), (0, 0), (0, 10)));
        assert!(segments_cross((0, 0), (10, 0), (0, 0), (5, 0)));
        // A touching endpoint in the middle of the other segment is a crossing.
        assert!(segments_cross((0, 0), (10, 0), (5, 0), (5, 5)));
    }
}
