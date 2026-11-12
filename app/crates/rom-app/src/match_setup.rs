//! Builds the simulation world from the rules and a generated map.
//!
//! Types are chosen by their component rows (a territory marked as a city, a builder
//! component), never by key or name.

use std::collections::BTreeMap;
use std::sync::Arc;

use rom_mapgen::GeneratedMap;
use rom_rules::{DefId, EntityType, ResourceKind, Rules};
use rom_sim::{EntityId, Player, PlayerId, Pos, World};

/// Builders each player starts with.
pub const BUILDERS_PER_PLAYER: usize = 5;
/// Amount of every base resource each player starts with.
pub const START_RESOURCES: i64 = 500;
/// The neutral owner of map nodes.
pub const NEUTRAL: PlayerId = PlayerId(0);

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

/// Builds the world: one player per map start, each with a city and builders, and the map's
/// nodes owned by the neutral player 0.
pub fn build_match(rules: &Arc<Rules>, map: &GeneratedMap, seed: u64) -> Result<Match, SetupError> {
    let city = city_type(rules).ok_or(SetupError::NoCity)?;
    let builder = builder_type(rules).ok_or(SetupError::NoBuilder)?;
    let (city_id, builder_id) = (city.id, builder.id);
    let first_age = *rules
        .ages
        .values()
        .min_by_key(|age| (age.ordinal, age.id))
        .map(|age| &age.id)
        .ok_or(SetupError::NoFaction)?;
    let mut factions: Vec<DefId> = rules
        .factions
        .values()
        .filter(|f| {
            f.playable && rules.is_available(city_id, f.id) && rules.is_available(builder_id, f.id)
        })
        .map(|f| f.id)
        .collect();
    if factions.is_empty() {
        factions = rules.factions.keys().copied().collect();
    }
    if factions.is_empty() {
        return Err(SetupError::NoFaction);
    }
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
    let mut players = vec![player(0, factions[0])];
    for index in 0..map.starts.len() {
        players.push(player(index as u32 + 1, factions[index % factions.len()]));
    }
    let mut world = World::new(Arc::clone(rules), map.grid.clone(), seed, players)?;

    let mut starts = Vec::new();
    for (index, start) in map.starts.iter().enumerate() {
        let owner = PlayerId(index as u32 + 1);
        let city_pos = Pos {
            x: snap(start.x, city.footprint_w),
            y: snap(start.y, city.footprint_h),
        };
        let city_entity = world.spawn(city_id, owner, city_pos)?;
        let reach = (city.footprint_w.max(city.footprint_h) * 500 + 1500) as f64;
        let mut builders = Vec::new();
        for n in 0..BUILDERS_PER_PLAYER {
            let angle = n as f64 * std::f64::consts::TAU / BUILDERS_PER_PLAYER as f64 + 0.3;
            let at = ring_position(&world, city_pos, angle, reach);
            builders.push(world.spawn(builder_id, owner, at)?);
        }
        starts.push(PlayerStart {
            player: owner,
            city: city_entity,
            city_pos,
            builders,
        });
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

/// A point on the ring around `centre` at `angle`; moves outwards until the ground is buildable
/// (a stand-in for "dry land") and stays on the map.
fn ring_position(world: &World, centre: Pos, angle: f64, reach: f64) -> Pos {
    let (sin, cos) = angle.sin_cos();
    for step in 0..6 {
        let distance = reach + f64::from(step) * 700.0;
        let at = Pos {
            x: centre.x + (cos * distance) as i32,
            y: centre.y + (sin * distance) as i32,
        };
        if let Some(terrain) = world.map().terrain_at_pos(at)
            && world.rules().terrains[&terrain].buildable
        {
            return at;
        }
    }
    centre
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::Options;
    use crate::terrain::generate_content;

    fn content(players: u32) -> crate::terrain::Content {
        generate_content(&Options {
            players,
            map_size: (128, 128),
            ..Options::default()
        })
        .expect("content")
    }

    #[test]
    fn match_setup_spawns_city_and_builders_per_player() {
        let content = content(3);
        let setup = build_match(&content.rules, &content.map, 1).expect("match");
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
}
