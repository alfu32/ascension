//! Builds the simulation world from the rules and a generated map, planet or universe.
//!
//! Types are chosen by their component rows (a territory marked as a city, a builder
//! component), never by key or name.

use std::collections::BTreeMap;
use std::sync::Arc;

use rom_mapgen::{GeneratedMap, SpherePlanet, Universe};
use rom_rules::{DefId, EntityType, ResourceKind, Rules};
use rom_sim::{
    BodyId, BodyInfo, EntityId, PlanetSurface, Player, PlayerId, Pos, RouteInfo, SystemInfo,
    UniverseSetup, World,
};

use crate::script_setup::ScriptSetup;

/// Builders each player starts with.
pub const BUILDERS_PER_PLAYER: usize = 5;
/// Amount of every base resource each player starts with.
pub const START_RESOURCES: i64 = 500;
/// The neutral owner of map nodes.
pub const NEUTRAL: PlayerId = PlayerId(0);
/// Player one: the first start, whom test setups (grants, placements, colonies) refer to.
pub const PLAYER_ONE: PlayerId = PlayerId(1);

/// What was placed for one player.
#[derive(Clone, Debug)]
pub struct PlayerStart {
    /// The player.
    pub player: PlayerId,
    /// The starting city.
    pub city: EntityId,
    /// Centre of the city in milli-tiles.
    pub city_pos: Pos,
    /// The starting builders.
    pub builders: Vec<EntityId>,
}

/// A world ready to run, with the facts the demo driver needs.
#[derive(Debug)]
pub struct Match {
    /// The simulation state.
    pub world: World,
    /// One entry per player start, in player id order.
    pub starts: Vec<PlayerStart>,
    /// The entity type used for the starting builders.
    pub builder_type: DefId,
}

/// Why a match could not be set up.
#[derive(Debug, thiserror::Error)]
pub enum SetupError {
    /// The rules contain no city building.
    #[error("the rules define no city (an entity type whose territory is a city)")]
    NoCity,
    /// The rules contain no builder.
    #[error("the rules define no builder")]
    NoBuilder,
    /// The rules contain no faction or age to start from.
    #[error("the rules define no faction or age")]
    NoFaction,
    /// A setup request that the rules or the universe cannot satisfy.
    #[error("{0}")]
    Setup(String),
    /// The simulation rejected the setup.
    #[error("simulation setup failed: {0}")]
    Sim(#[from] rom_sim::SimError),
}

/// First entity type, in id order, that is a city.
pub fn city_type(rules: &Rules) -> Option<&EntityType> {
    rules
        .entity_types
        .values()
        .find(|et| et.territory.as_ref().is_some_and(|t| t.is_city))
}

/// First entity type, in id order, with a builder component.
pub fn builder_type(rules: &Rules) -> Option<&EntityType> {
    rules.entity_types.values().find(|et| et.builder.is_some())
}

/// A coordinate that puts a footprint of `size` tiles on whole tiles, nearest to `coordinate`.
fn snap(coordinate: i32, size: i64) -> i32 {
    ((i64::from(coordinate) - size * 500 + 500).div_euclid(1000) * 1000 + size * 500) as i32
}

/// The age matches start in: the first by ordinal.
pub fn start_age(rules: &Rules) -> Option<DefId> {
    rules
        .ages
        .values()
        .min_by_key(|age| (age.ordinal, age.id))
        .map(|age| age.id)
}

/// The factions of `players` default players: races in id order take turns, each giving its
/// start factions in id order, so with three races four players are two groups of the first
/// race and one group of each other. Chosen through the start-age window and the race
/// ancestry, never by key or name.
pub fn default_factions(rules: &Rules, players: u32) -> Vec<DefId> {
    let Some(age) = start_age(rules) else {
        return Vec::new();
    };
    let mut by_race: BTreeMap<DefId, Vec<DefId>> = BTreeMap::new();
    for faction in rules.playable_start_factions(age) {
        by_race
            .entry(rules.race_of(faction))
            .or_default()
            .push(faction);
    }
    let races: Vec<&Vec<DefId>> = by_race.values().collect();
    (0..players as usize)
        .map(|i| {
            let groups = races[i % races.len()];
            groups[(i / races.len()) % groups.len()]
        })
        .collect()
}

/// Puts every non-neutral player into the age with this ordinal, when one is given.
fn set_age_by_ordinal(
    rules: &Rules,
    players: &mut [Player],
    ordinal: Option<i64>,
) -> Result<(), SetupError> {
    let Some(ordinal) = ordinal else {
        return Ok(());
    };
    let age = rules
        .ages
        .values()
        .find(|a| a.ordinal == ordinal)
        .ok_or_else(|| SetupError::Setup(format!("no age with ordinal {ordinal}")))?
        .id;
    for player in players.iter_mut().filter(|p| p.id != NEUTRAL) {
        player.age = age;
    }
    Ok(())
}

/// The players of a match: neutral player 0 and one per faction, each on its own team.
fn make_players(rules: &Rules, factions: &[DefId]) -> Result<Vec<Player>, SetupError> {
    let first_age = start_age(rules).ok_or(SetupError::NoFaction)?;
    let resources: BTreeMap<DefId, i64> = rules
        .resources
        .values()
        .filter(|r| r.kind == ResourceKind::Base)
        .map(|r| (r.id, START_RESOURCES))
        .collect();
    let player = |id: u32, faction: DefId| Player {
        id: PlayerId(id),
        team: id,
        faction,
        age: rules
            .factions
            .get(&faction)
            .and_then(|f| f.first_age_id)
            .filter(|age| rules.ages.contains_key(age))
            .unwrap_or(first_age),
        resources: if id == 0 {
            BTreeMap::new()
        } else {
            resources.clone()
        },
        techs: BTreeMap::new(),
    };
    let mut players = vec![player(0, *factions.first().ok_or(SetupError::NoFaction)?)];
    for (index, faction) in factions.iter().enumerate() {
        players.push(player(index as u32 + 1, *faction));
    }
    Ok(players)
}

/// Builds the world: one player per map start with the given faction, each with a city and
/// builders, and the map's nodes owned by the neutral player 0. Every player is on its own team.
pub fn build_match(
    rules: &Arc<Rules>,
    map: &GeneratedMap,
    factions: &[DefId],
    seed: u64,
) -> Result<Match, SetupError> {
    build_match_in_age(rules, map, factions, seed, None)
}

/// [`build_match`] with every player starting in the age of the given ordinal.
pub fn build_match_in_age(
    rules: &Arc<Rules>,
    map: &GeneratedMap,
    factions: &[DefId],
    seed: u64,
    age_ordinal: Option<i64>,
) -> Result<Match, SetupError> {
    let city = city_type(rules).ok_or(SetupError::NoCity)?;
    let builder = builder_type(rules).ok_or(SetupError::NoBuilder)?;
    let builder_id = builder.id;
    if factions.is_empty() || factions.len() != map.starts.len() {
        return Err(SetupError::NoFaction);
    }
    let mut players = make_players(rules, factions)?;
    set_age_by_ordinal(rules, &mut players, age_ordinal)?;
    let mut world = World::new(Arc::clone(rules), map.grid.clone(), seed, players)?;

    let mut starts = Vec::new();
    for (index, start) in map.starts.iter().enumerate() {
        let city_pos = Pos {
            x: snap(start.x, city.footprint_w),
            y: snap(start.y, city.footprint_h),
            face: 0,
            ..Pos::default()
        };
        starts.push(place_start(
            &mut world,
            PlayerId(index as u32 + 1),
            city,
            builder_id,
            city_pos,
        )?);
    }
    for node in &map.nodes {
        world.spawn(node.entity_type, NEUTRAL, node.pos)?;
    }
    Ok(Match {
        world,
        starts,
        builder_type: builder_id,
    })
}

/// Builds the world on a cube-sphere planet: terrain and nodes from the planet, players as in
/// [`build_match`]. A start too close to a face edge for the city footprint moves inward the
/// least that keeps the footprint on one face; builders are placed with `World::offset`, so a
/// ring that crosses a face edge continues on the next face.
pub fn build_sphere_match(
    rules: &Arc<Rules>,
    planet: &SpherePlanet,
    factions: &[DefId],
    seed: u64,
) -> Result<Match, SetupError> {
    build_sphere_match_in_age(rules, planet, factions, seed, None)
}

/// [`build_sphere_match`] with every player starting in the age of the given ordinal.
pub fn build_sphere_match_in_age(
    rules: &Arc<Rules>,
    planet: &SpherePlanet,
    factions: &[DefId],
    seed: u64,
    age_ordinal: Option<i64>,
) -> Result<Match, SetupError> {
    let city = city_type(rules).ok_or(SetupError::NoCity)?;
    let builder = builder_type(rules).ok_or(SetupError::NoBuilder)?;
    let builder_id = builder.id;
    if factions.is_empty() || factions.len() != planet.starts.len() {
        return Err(SetupError::NoFaction);
    }
    let mut players = make_players(rules, factions)?;
    set_age_by_ordinal(rules, &mut players, age_ordinal)?;
    let mut world = World::from_sphere(
        Arc::clone(rules),
        planet.surface,
        planet.terrain.clone(),
        &planet.nodes,
        seed,
        players,
    )?;
    let side = i64::from(planet.surface.n) * 1000;
    let inward = |coordinate: i64, size: i64| -> i32 {
        let snapped = i64::from(snap(coordinate.clamp(0, i64::from(i32::MAX)) as i32, size));
        snapped.clamp(size * 500, side - size * 500) as i32
    };
    let mut starts = Vec::new();
    for (index, start) in planet.starts.iter().enumerate() {
        // The nearest spot to the start whose whole footprint is buildable ground.
        let spot = |dx: i64, dy: i64| Pos {
            x: inward(start.x + dx * 1000, city.footprint_w),
            y: inward(start.y + dy * 1000, city.footprint_h),
            face: start.face,
            ..Pos::default()
        };
        let city_pos = (0..=12_i64)
            .flat_map(|r| {
                (-r..=r).flat_map(move |dx| {
                    (-r..=r)
                        .filter(move |dy| dx.abs().max(dy.abs()) == r)
                        .map(move |dy| (dx, dy))
                })
            })
            .map(|(dx, dy)| spot(dx, dy))
            .find(|pos| footprint_buildable(&world, city, *pos))
            .unwrap_or_else(|| spot(0, 0));
        starts.push(place_start(
            &mut world,
            PlayerId(index as u32 + 1),
            city,
            builder_id,
            city_pos,
        )?);
    }
    // The world model: before the space age each player knows the terrain of its home planet.
    // Units and buildings stay hidden until they are seen.
    for &player in &starts_players(&world) {
        let home = world.home_body(player).unwrap_or_default();
        world.reveal_terrain(player, home);
    }
    Ok(Match {
        world,
        starts,
        builder_type: builder_id,
    })
}

/// The generated universe as plain data for the simulation, with its generated (home) planets.
pub fn universe_setup(universe: &Universe) -> UniverseSetup {
    UniverseSetup {
        systems: universe
            .systems
            .iter()
            .map(|s| SystemInfo {
                id: s.id,
                name: s.name.clone(),
                x: s.x,
                y: s.y,
                z: s.z,
                constellation: s.constellation.clone(),
            })
            .collect(),
        bodies: universe
            .bodies
            .iter()
            .map(|b| BodyInfo {
                id: BodyId(b.id),
                system: b.system_id,
                kind: b.kind,
                name: b.name.clone(),
                orbit_radius: b.orbit_radius,
                orbit_angle_mdeg: b.orbit_angle_mdeg,
                n: b.n,
                home_race: b.home_race,
                node_type: b.node_entity_type,
            })
            .collect(),
        figure: universe.figure.clone(),
        dormant: universe
            .dormant
            .iter()
            .map(|(body, surface)| (BodyId(*body), *surface))
            .collect(),
        routes: universe
            .routes
            .iter()
            .map(|r| RouteInfo {
                system_a: r.system_a,
                system_b: r.system_b,
                length: r.length,
            })
            .collect(),
        planets: universe
            .planets
            .iter()
            .map(|(body, planet)| (BodyId(*body), planet_surface(planet)))
            .collect(),
    }
}

/// A generated planet as the simulation takes it.
pub fn planet_surface(planet: &SpherePlanet) -> PlanetSurface {
    PlanetSurface {
        surface: planet.surface,
        terrain: planet.terrain.clone(),
        nodes: planet.nodes.clone(),
    }
}

/// Builds one world for the whole universe: every home planet is a surface, each player starts
/// on the home planet of its race with a city (placed by the simulation's site search) and
/// builders, and the other habitable planets are added later with `World::add_surface`.
/// `setup` holds the test-only age and resources a script asked for.
pub fn build_universe_match(
    rules: &Arc<Rules>,
    universe: &Universe,
    factions: &[DefId],
    seed: u64,
    age_ordinal: Option<i64>,
    setup: &ScriptSetup,
) -> Result<Match, SetupError> {
    let builder = builder_type(rules).ok_or(SetupError::NoBuilder)?;
    let builder_id = builder.id;
    if factions.is_empty() || factions.len() != universe.starts.len() {
        return Err(SetupError::NoFaction);
    }
    let mut players = make_players(rules, factions)?;
    if let Some(key) = &setup.age {
        let age = rules
            .id_of(key)
            .filter(|id| rules.ages.contains_key(id))
            .ok_or_else(|| SetupError::Setup(format!("unknown age {key:?}")))?;
        for player in players.iter_mut().filter(|p| p.id != NEUTRAL) {
            player.age = age;
        }
    }
    set_age_by_ordinal(rules, &mut players, age_ordinal)?;
    for key in &setup.techs {
        let tech = rules
            .id_of(key)
            .filter(|id| rules.techs.contains_key(id))
            .ok_or_else(|| SetupError::Setup(format!("unknown technology {key:?}")))?;
        if let Some(player) = players.iter_mut().find(|p| p.id == PLAYER_ONE) {
            player.techs.insert(tech, 1);
        }
    }
    // Task 76: everything at the library, and the price of the ascension.
    for owner in &setup.ascension_ready {
        if let Some(player) = players.iter_mut().find(|p| p.id == PlayerId(*owner)) {
            for tech in rules.ascension_requirements() {
                player.techs.insert(tech, 1);
            }
            for tech in rules.techs.keys().filter(|t| rules.ascends(**t)) {
                for cost in rules.costs.get(tech).into_iter().flatten() {
                    *player.resources.entry(cost.resource_id).or_default() += cost.amount;
                }
            }
        }
    }
    let homes: BTreeMap<PlayerId, BodyId> = universe
        .starts
        .iter()
        .enumerate()
        .map(|(i, (body, _))| (PlayerId(i as u32 + 1), BodyId(*body)))
        .collect();
    let mut world = World::from_universe(
        Arc::clone(rules),
        universe_setup(universe),
        seed,
        players,
        homes,
    )?;
    let mut starts = Vec::new();
    for (index, (body, start)) in universe.starts.iter().enumerate() {
        let (player, body) = (PlayerId(index as u32 + 1), BodyId(*body));
        world.reveal_terrain(player, body);
        let mut near = World::pos_of(*start)?;
        near.body = body;
        let city_id = world.city_type(player).ok_or(SetupError::NoCity)?;
        let site = world
            .find_city_site(player, body, near)
            .ok_or_else(|| SetupError::Setup("no room for a starting city".into()))?;
        let city = rules.entity_types[&city_id].clone();
        starts.push(place_start(&mut world, player, &city, builder_id, site)?);
    }
    // Task 110: as on a single planet (`ScriptSetup::apply_grants`): research with the starting
    // city standing, then the grants.
    for owner in &setup.research_all {
        world.complete_research_for_setup(PlayerId(*owner))?;
    }
    for (key, amount) in &setup.grants {
        let resource = rules
            .id_of(key)
            .filter(|id| rules.resources.contains_key(id))
            .ok_or_else(|| SetupError::Setup(format!("unknown resource {key:?}")))?;
        world.grant(PLAYER_ONE, resource, *amount)?;
    }
    setup
        .apply_placements(&mut world, starts[0].city_pos)
        .map_err(SetupError::Setup)?;
    setup
        .apply_space_placements(&mut world)
        .map_err(SetupError::Setup)?;
    for owner in &setup.colonies {
        found_colony(rules, universe, &mut world, PlayerId(*owner), false)?;
    }
    for owner in &setup.far_colonies {
        found_colony(rules, universe, &mut world, PlayerId(*owner), true)?;
    }
    for owner in &setup.auto_colonies {
        let colony = found_colony(rules, universe, &mut world, PlayerId(*owner), false)?;
        // The same place, as an automated colony (the type with a colony component).
        let pos = world.entity(colony).map(|e| e.pos).expect("founded");
        world.despawn(colony)?;
        let automated = rules
            .entity_types
            .values()
            .find(|et| et.colony.is_some())
            .ok_or_else(|| SetupError::Setup("auto-colony: no colony type".into()))?
            .id;
        world.place_for_setup(automated, PlayerId(*owner), pos)?;
    }
    Ok(Match {
        world,
        starts,
        builder_type: builder_id,
    })
}

/// Test setup (`colony`): a finished city of `owner` on the habitable planet of player one's
/// home system nearest it, the planet generated first. With `far` (`colony-far`), on the
/// habitable planet nearest its star of the system one route from home whose star is nearest.
fn found_colony(
    rules: &Arc<Rules>,
    universe: &Universe,
    world: &mut World,
    owner: PlayerId,
    far: bool,
) -> Result<rom_sim::EntityId, SetupError> {
    let fail = |what: &str| SetupError::Setup(format!("colony: {what}"));
    let home = world.home_body(PLAYER_ONE).ok_or_else(|| fail("no home"))?;
    let (system, at) = world
        .body(home)
        .map(|b| b.system)
        .zip(world.body_position(home))
        .ok_or_else(|| fail("home planet not in a system"))?;
    let (system, at) = if far {
        let neighbour = world
            .routes()
            .iter()
            .filter_map(|r| {
                let other = if r.system_a == system {
                    r.system_b
                } else if r.system_b == system {
                    r.system_a
                } else {
                    return None;
                };
                world
                    .bodies()
                    .iter()
                    .any(|b| {
                        b.system == other
                            && b.kind == rom_rules::BodyKind::Habitable
                            && b.home_race.is_none()
                    })
                    .then_some((r.length, other))
            })
            .min()
            .map(|(_, other)| other)
            .ok_or_else(|| fail("no neighbouring system with a planet"))?;
        let star = world
            .space_of(neighbour)
            .and_then(|s| world.body_position(s))
            .ok_or_else(|| fail("no star"))?;
        (neighbour, star)
    } else {
        (system, at)
    };
    let body = world
        .bodies()
        .iter()
        .filter(|b| b.system == system && b.kind == rom_rules::BodyKind::Habitable && b.id != home)
        .filter(|b| !far || b.home_race.is_none())
        .filter_map(|b| Some((at.distance(world.body_position(b.id)?), b.id)))
        .min()
        .map(|(_, id)| id)
        .ok_or_else(|| fail("no other planet"))?;
    if world.surface(body).is_none() {
        let planet = universe
            .expand(rules, body.0)
            .map_err(|e| fail(&e.to_string()))?;
        world
            .add_surface(body, &planet_surface(&planet))
            .map_err(|e| fail(&e.to_string()))?;
    }
    world.reveal_terrain(PLAYER_ONE, body);
    let n = world.body(body).and_then(|b| b.n).unwrap_or(192) as i32;
    let city = world.city_type(owner).ok_or(SetupError::NoCity)?;
    // Near the middle of the first face (of the four around the equator) with room for it.
    let site = [0, 1, 2, 3, 4, 5]
        .into_iter()
        .find_map(|face| world.find_city_site(owner, body, Pos::on(body, face, n * 500, n * 500)))
        .ok_or_else(|| fail("no room for a city"))?;
    Ok(world.place_for_setup(city, owner, site)?)
}

/// Whether every tile under a building of type `et` centred at `pos` is buildable.
fn footprint_buildable(world: &World, et: &EntityType, pos: Pos) -> bool {
    let (hw, hh) = (et.footprint_w * 500, et.footprint_h * 500);
    let (x, y) = (i64::from(pos.x), i64::from(pos.y));
    ((x - hw).div_euclid(1000)..(x + hw).div_euclid(1000)).all(|tx| {
        ((y - hh).div_euclid(1000)..(y + hh).div_euclid(1000)).all(|ty| {
            let at = Pos {
                x: (tx * 1000 + 500) as i32,
                y: (ty * 1000 + 500) as i32,
                face: pos.face,
                ..Pos::default()
            };
            world
                .terrain_at(at)
                .is_some_and(|t| world.rules().terrains[&t].buildable)
        })
    })
}

/// Sine and cosine, as bit patterns, of the angle of each starting builder around its city:
/// `n · τ / BUILDERS_PER_PLAYER + 0.3` radians for `n` in `0..5`. The values `f64::sin_cos` gave
/// on x86_64 (glibc), fixed so every machine places the builders alike: the platform's sine may
/// differ in the last bit, and a leader and its followers must build the same world.
const RING: [(u64, u64); BUILDERS_PER_PLAYER] = [
    (0x3fd2_e9cd_95ba_ba33, 0x3fee_921d_d42f_09ba),
    (0x3fef_ff2d_c7ab_ef3d, 0x3f8c_ff48_ed1d_ad89),
    (0x3fd4_a30d_0001_35bb, 0xbfee_4a6e_759c_4b34),
    (0xbfe9_9ea0_58c3_7162, 0xbfe3_2c7d_b527_40de),
    (0xbfea_26fa_b9c6_75d0, 0x3fe2_70d1_32e0_0b9e),
];

/// Places a player's city at `city_pos` and the builders on a ring around it.
fn place_start(
    world: &mut World,
    owner: PlayerId,
    city: &EntityType,
    builder_id: DefId,
    city_pos: Pos,
) -> Result<PlayerStart, SetupError> {
    let city_entity = world.place_for_setup(city.id, owner, city_pos)?;
    let reach = (city.footprint_w.max(city.footprint_h) * 500 + 1500) as f64;
    let mut builders = Vec::new();
    for (sin, cos) in RING {
        let at = ring_position(
            world,
            city_pos,
            (f64::from_bits(sin), f64::from_bits(cos)),
            reach,
        );
        builders.push(world.spawn(builder_id, owner, at)?);
    }
    Ok(PlayerStart {
        player: owner,
        city: city_entity,
        city_pos,
        builders,
    })
}

/// A point on the ring around `centre` in the direction `(sin, cos)`; moves outwards until the
/// ground is buildable (a stand-in for "dry land") and stays on the map. On a sphere the ring
/// continues across face edges.
fn ring_position(world: &World, centre: Pos, (sin, cos): (f64, f64), reach: f64) -> Pos {
    for step in 0..6 {
        let distance = reach + f64::from(step) * 700.0;
        let at = world.offset(centre, (cos * distance) as i64, (sin * distance) as i64);
        if let Some(terrain) = world.terrain_at(at)
            && world.rules().terrains[&terrain].buildable
        {
            return at;
        }
    }
    centre
}

/// Every non-neutral player of the world, in id order.
fn starts_players(world: &World) -> Vec<PlayerId> {
    world
        .players()
        .map(|p| p.id)
        .filter(|id| *id != NEUTRAL)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{
        Content, ContentRequest, generate_content, generate_sphere_content,
        generate_universe_content,
    };

    fn content(players: u32) -> Content {
        generate_content(&ContentRequest {
            players,
            map_size: (128, 128),
            ..ContentRequest::default()
        })
        .expect("content")
    }

    fn universe(style: Option<&str>, factions: Option<Vec<String>>) -> ContentRequest {
        ContentRequest {
            style: style.map(str::to_owned),
            factions,
            seed: 1,
            ..ContentRequest::default()
        }
    }

    /// The sine and cosine table is what `sin_cos` gives for the ring's angles here.
    #[cfg(target_arch = "x86_64")]
    #[test]
    fn the_ring_table_is_the_ring_angles() {
        for (n, (sin, cos)) in RING.iter().enumerate() {
            let angle = n as f64 * std::f64::consts::TAU / BUILDERS_PER_PLAYER as f64 + 0.3;
            let (s, c) = angle.sin_cos();
            assert_eq!((s.to_bits(), c.to_bits()), (*sin, *cos), "builder {n}");
        }
    }

    #[test]
    fn match_setup_spawns_city_and_builders_per_player() {
        let content = content(3);
        let setup = build_match(&content.rules, &content.map, &content.factions, 1).expect("match");
        let world = &setup.world;
        assert_eq!(setup.starts.len(), 3);
        let city = city_type(&content.rules).expect("a city type");
        assert!(city.territory.as_ref().is_some_and(|t| t.is_city));
        let builder = &content.rules.entity_types[&setup.builder_type];
        assert!(builder.builder.is_some());
        // Chosen by property: the first such type in id order.
        assert!(
            content
                .rules
                .entity_types
                .values()
                .take_while(|et| et.id != city.id)
                .all(|et| et.territory.as_ref().is_none_or(|t| !t.is_city))
        );
        let base_resources: Vec<DefId> = content
            .rules
            .resources
            .values()
            .filter(|r| r.kind == ResourceKind::Base)
            .map(|r| r.id)
            .collect();
        assert!(!base_resources.is_empty());
        let mut teams = Vec::new();
        for start in &setup.starts {
            let cities = world
                .entities()
                .filter(|e| e.owner == start.player && e.type_id == city.id)
                .count();
            let builders = world
                .entities()
                .filter(|e| e.owner == start.player && e.type_id == setup.builder_type)
                .count();
            assert_eq!((cities, builders), (1, BUILDERS_PER_PLAYER));
            assert_eq!(start.builders.len(), BUILDERS_PER_PLAYER);
            let buildings = world
                .entities()
                .filter(|e| {
                    e.owner == start.player
                        && content.rules.entity_types[&e.type_id].class
                            == rom_rules::EntityClass::Building
                })
                .count();
            assert_eq!(buildings, 1);
            let player = world
                .players()
                .find(|p| p.id == start.player)
                .expect("player");
            for resource in &base_resources {
                assert_eq!(player.resources[resource], START_RESOURCES);
            }
            teams.push(player.team);
            let city_entity = world.entity(start.city).expect("city entity");
            assert_eq!(city_entity.pos, start.city_pos);
        }
        teams.sort_unstable();
        teams.dedup();
        assert_eq!(teams.len(), 3, "teams are all different");
        let neutral: Vec<_> = world.entities().filter(|e| e.owner == NEUTRAL).collect();
        assert_eq!(neutral.len(), content.map.nodes.len());
        assert!(!neutral.is_empty());
        assert_eq!(
            world.players().find(|p| p.id == NEUTRAL).map(|p| p.team),
            Some(0)
        );
    }

    fn races_of(content: &Content, factions: &[DefId]) -> Vec<DefId> {
        factions.iter().map(|f| content.rules.race_of(*f)).collect()
    }

    #[test]
    fn default_factions_cover_every_race() {
        let content = content(4);
        let rules = &content.rules;
        assert_eq!(content.factions.len(), 4);
        let races = races_of(&content, &content.factions);
        let every_race: std::collections::BTreeSet<DefId> = rules
            .factions
            .values()
            .filter(|f| f.kind == rom_rules::FactionKind::Race && f.playable)
            .map(|f| f.id)
            .collect();
        assert_eq!(
            races
                .iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>(),
            every_race
        );
        // Two groups of the first race, one of each other race, all different factions.
        assert_eq!(races.iter().filter(|r| **r == races[0]).count(), 2);
        let distinct: std::collections::BTreeSet<_> = content.factions.iter().collect();
        assert_eq!(distinct.len(), 4);
        let at_start = rules.playable_start_factions(start_age(rules).unwrap());
        assert!(content.factions.iter().all(|f| at_start.contains(f)));
    }

    #[test]
    fn players_receive_their_faction_and_distinct_teams() {
        let content = content(4);
        let setup = build_match(&content.rules, &content.map, &content.factions, 1).expect("match");
        let mut teams = Vec::new();
        for (index, faction) in content.factions.iter().enumerate() {
            let player = setup
                .world
                .players()
                .find(|p| p.id == PlayerId(index as u32 + 1))
                .expect("player");
            assert_eq!(player.faction, *faction);
            teams.push(player.team);
        }
        teams.sort_unstable();
        teams.dedup();
        assert_eq!(teams.len(), 4);
        assert!(build_match(&content.rules, &content.map, &content.factions[..2], 1).is_err());
    }

    #[test]
    fn match_setup_places_each_race_on_its_home_planet() {
        let content = content(4);
        let setup = build_match(&content.rules, &content.map, &content.factions, 1).expect("match");
        let races = races_of(&content, &content.factions);
        for (index, start) in setup.starts.iter().enumerate() {
            let entity = setup.world.entity(start.city).expect("city");
            let (x, y) = (entity.pos.x / 1000, entity.pos.y / 1000);
            let at = (y as usize) * content.map.grid.width as usize + x as usize;
            let region = content.map.regions[content.map.region[at] as usize];
            assert_eq!(region.home_race, Some(races[index]), "player {index}");
        }
        let homes: std::collections::BTreeSet<_> = (0..4)
            .map(|i| content.map.start_region(i).unwrap())
            .collect();
        assert_eq!(homes.len(), 3);
    }

    #[test]
    fn universe_world_puts_each_race_on_its_home_planet() {
        let content = generate_universe_content(&universe(None, None)).expect("universe");
        let game = build_universe_match(
            &content.rules,
            &content.universe,
            &content.factions,
            1,
            None,
            &ScriptSetup::default(),
        )
        .expect("match");
        let world = &game.world;
        assert_eq!(game.starts.len(), content.factions.len());
        let mut surfaces = 0;
        for (start, faction) in game.starts.iter().zip(&content.factions) {
            let race = content.rules.race_of(*faction);
            let city = world.entity(start.city).expect("city");
            let home = world.body(city.pos.body).expect("a body");
            assert_eq!(home.home_race, Some(race), "player {:?}", start.player);
            assert_eq!(world.home_body(start.player), Some(home.id));
            assert!(world.surface(home.id).is_some());
            assert_eq!(start.builders.len(), BUILDERS_PER_PLAYER);
            for b in &start.builders {
                assert_eq!(world.entity(*b).unwrap().pos.body, home.id);
            }
            surfaces += 1;
        }
        assert!(surfaces > 1);
        // Other habitable planets are not generated yet.
        assert!(!world.ungenerated_bodies().is_empty());
    }

    #[test]
    fn sphere_setup_places_city_and_builders_on_valid_tiles() {
        let request = ContentRequest {
            players: 4,
            ..ContentRequest::default()
        };
        let content = generate_sphere_content(&request, 128).expect("planet");
        let setup = build_sphere_match(&content.rules, &content.planet, &content.factions, 1)
            .expect("match");
        let world = &setup.world;
        let rules = &content.rules;
        let city = city_type(rules).expect("a city type");
        let side = 128 * 1000;
        assert_eq!(setup.starts.len(), 4);
        // Every player knows the home planet's terrain from the start.
        for start in &setup.starts {
            assert!(world.is_explored_on(start.player, 3, 0, 0));
            assert!(world.is_explored_on(start.player, 5, 10, 10));
        }
        for start in &setup.starts {
            let entity = world.entity(start.city).expect("city");
            assert_eq!(entity.pos, start.city_pos);
            assert!(start.city_pos.face < 6);
            // The footprint lies on one face and every tile under it is buildable.
            let (hw, hh) = (city.footprint_w * 500, city.footprint_h * 500);
            let (x, y) = (i64::from(start.city_pos.x), i64::from(start.city_pos.y));
            assert!(x - hw >= 0 && x + hw <= side && y - hh >= 0 && y + hh <= side);
            for tx in (x - hw) / 1000..(x + hw) / 1000 {
                for ty in (y - hh) / 1000..(y + hh) / 1000 {
                    let at = Pos {
                        x: (tx * 1000 + 500) as i32,
                        y: (ty * 1000 + 500) as i32,
                        face: start.city_pos.face,
                        ..Pos::default()
                    };
                    let terrain = world.terrain_at(at).expect("on the planet");
                    assert!(
                        rules.terrains[&terrain].buildable,
                        "player {:?} tile {tx},{ty} city {:?} start {:?} terrain {}",
                        start.player,
                        start.city_pos,
                        content.planet.starts[start.player.0 as usize - 1],
                        rules.def(terrain).name
                    );
                }
            }
            assert_eq!(start.builders.len(), BUILDERS_PER_PLAYER);
            for id in &start.builders {
                let builder = world.entity(*id).expect("builder");
                assert_eq!(builder.owner, start.player);
                let terrain = world.terrain_at(builder.pos).expect("on the planet");
                assert!(rules.terrains[&terrain].buildable);
                let distance = world.distance(builder.pos, start.city_pos).expect("near");
                assert!(
                    (1_000..12_000).contains(&distance),
                    "builder {distance} away"
                );
            }
        }
        // Nodes were placed by the planet, owned by the neutral player.
        let nodes = world.entities().filter(|e| e.owner == NEUTRAL).count();
        assert_eq!(nodes, content.planet.nodes.len());
    }

    /// Task 84: the last-age rich start leaves player one ready to ascend, in that age, rich;
    /// every age's script scans.
    #[test]
    fn the_last_age_rich_start_is_ready_to_ascend() {
        for age in [
            "00-ancient",
            "01-classical",
            "02-medieval",
            "03-gunpowder",
            "04-enlightenment",
            "05-industrial",
            "06-modern",
            "07-information",
            "08-automation",
            "09-fusion",
            "10-starfaring",
            "11-expansion",
            "12-abundance",
        ] {
            let path = format!(
                "{}/../../docs/tasks/rich-start-{age}-script.txt",
                env!("CARGO_MANIFEST_DIR")
            );
            let text = std::fs::read_to_string(&path).expect("script");
            let setup = ScriptSetup::scan(&text).expect("scan");
            assert_eq!(setup.research_all, vec![1], "{path}");
        }
        let setup = ScriptSetup::scan(include_str!(
            "../../../docs/tasks/rich-start-13-galactic-script.txt"
        ))
        .expect("scan");
        let content = generate_universe_content(&universe(None, None)).expect("universe");
        let game = build_universe_match(
            &content.rules,
            &content.universe,
            &content.factions,
            1,
            None,
            &setup,
        )
        .expect("match");
        let me = PLAYER_ONE;
        // Task 110: the universe match applies `research-all` and the grants itself.
        assert!(game.world.ascension_ready(me));
        let p = game
            .world
            .players()
            .find(|p| p.id == me)
            .expect("player one");
        assert_eq!(Some(p.age), content.rules.id_of("age.galactic"));
        let food = content.rules.id_of("res.food").unwrap();
        assert!(p.resources[&food] >= 100_000);
    }

    /// The technologies `research-all` must leave a player in `age` with, by the simulation's own
    /// rule (`rom-sim` `research_for_setup_completes_everything_up_to_the_age_and_no_further`):
    /// panics on one beyond the age or one left out; returns how many it owns.
    fn assert_researched_up_to(world: &World, player: PlayerId, age: DefId) -> usize {
        let rules = world.rules();
        let p = world.players().find(|p| p.id == player).expect("player");
        assert_eq!(p.age, age, "the age is kept");
        let reached = rules.ages[&age].ordinal;
        let sources = p.active_sources();
        let age_of = |a: &DefId| rules.ages.get(a).map(|a| a.ordinal);
        for (id, tech) in &rules.techs {
            let needs_later_age = rules
                .prerequisites
                .get(id)
                .into_iter()
                .flatten()
                .filter_map(|r| age_of(&r.requires_def_id))
                .any(|o| o > reached);
            let advance_beyond = rules.tech_effects.get(id).into_iter().flatten().any(|e| {
                e.effect == rom_rules::TechEffectKind::SetAge
                    && e.target_def_id.and_then(|a| age_of(&a)) > Some(reached)
            });
            let has = p.techs.get(id).is_some_and(|n| *n > 0);
            if needs_later_age || advance_beyond || rules.ascends(*id) {
                assert!(!has, "{} is beyond the age", rules.key(*id));
            } else if tech.kind != rom_rules::TechKind::Handicap
                && rules.is_unlocked(*id, &sources, &Default::default())
            {
                assert!(has, "{} was left out", rules.key(*id));
            }
        }
        p.techs.values().filter(|n| **n > 0).count()
    }

    /// Task 110: `research-all` applies in a universe match as on a single planet: everything
    /// up to the start age is researched before the first tick, and the age is kept.
    #[test]
    fn research_all_applies_in_universe_and_single_planet_matches() {
        let setup = ScriptSetup::scan("0 research-all 1\n").expect("scan");
        let universe = generate_universe_content(&self::universe(None, None)).expect("universe");
        let rules = &universe.rules;
        let industrial = rules.id_of("age.industrial").expect("industrial");
        let ordinal = Some(rules.ages[&industrial].ordinal);
        let build = |setup: &ScriptSetup| {
            build_universe_match(
                rules,
                &universe.universe,
                &universe.factions,
                1,
                ordinal,
                setup,
            )
            .expect("match")
        };
        let ages_only = build(&ScriptSetup::default());
        let me = PLAYER_ONE;
        let before = ages_only
            .world
            .players()
            .find(|p| p.id == me)
            .unwrap()
            .techs
            .len();
        let game = build(&setup);
        let owned = assert_researched_up_to(&game.world, me, industrial);
        assert!(
            owned > before,
            "{owned} technologies, {before} with the ages only"
        );
        assert_eq!(game.world.tick(), 0);

        let content = content(2);
        let mut flat =
            build_match_in_age(&content.rules, &content.map, &content.factions, 1, ordinal)
                .expect("match");
        setup.apply_grants(&mut flat.world).expect("grants");
        let industrial = content.rules.id_of("age.industrial").unwrap();
        assert_eq!(assert_researched_up_to(&flat.world, me, industrial), owned);
    }

    /// Task 109: in the Planet style every player starts on the one planet of the one system,
    /// and from the space age ships and orbital craft reach that planet's orbit.
    #[test]
    fn planet_style_reaches_orbit() {
        let content = generate_universe_content(&universe(
            Some("map.one_world"),
            Some(vec!["nation.romans".into(), "nation.ashscale".into()]),
        ))
        .expect("universe");
        let stars = content
            .universe
            .systems
            .iter()
            .filter(|s| s.star.is_some())
            .count();
        assert_eq!(stars, 1);
        assert!(
            content
                .universe
                .starts
                .iter()
                .all(|s| s.0 == content.universe.starts[0].0)
        );
        let setup = ScriptSetup::scan(
            "0 grant-age age.starfaring\n0 place-orbit 1 unit.corvette @0,-30\n\
             0 place-orbit 1 unit.orbital_fighter @10,-30\n",
        )
        .expect("scan");
        let mut game = build_universe_match(
            &content.rules,
            &content.universe,
            &content.factions,
            1,
            None,
            &setup,
        )
        .expect("match");
        let home = game.starts[0].city_pos.body;
        assert!(game.starts.iter().all(|s| s.city_pos.body == home));
        let orbiting = |world: &World| {
            world
                .entities()
                .filter(|e| world.orbit(e.id).is_some_and(|o| o.body == home))
                .count()
        };
        for _ in 0..4000 {
            if orbiting(&game.world) == 2 {
                break;
            }
            game.world.step();
        }
        assert_eq!(
            orbiting(&game.world),
            2,
            "the corvette and the fighters in orbit"
        );
    }
}
