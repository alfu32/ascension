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

/// Builds the world: one player per map start with the given faction, each with a city and
/// builders, and the map's nodes owned by the neutral player 0. Every player is on its own team.
pub fn build_match(
    rules: &Arc<Rules>,
    map: &GeneratedMap,
    factions: &[DefId],
    seed: u64,
) -> Result<Match, SetupError> {
    let city = city_type(rules).ok_or(SetupError::NoCity)?;
    let builder = builder_type(rules).ok_or(SetupError::NoBuilder)?;
    let (city_id, builder_id) = (city.id, builder.id);
    let first_age = start_age(rules).ok_or(SetupError::NoFaction)?;
    if factions.is_empty() || factions.len() != map.starts.len() {
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
    for (index, faction) in factions.iter().enumerate() {
        players.push(player(index as u32 + 1, *faction));
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

    fn races_of(content: &crate::terrain::Content, factions: &[DefId]) -> Vec<DefId> {
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
}
