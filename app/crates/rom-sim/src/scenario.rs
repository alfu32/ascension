//! Seeded headless scenarios for the command-line runner and benchmarks.

use crate::{Command, MapGrid, Player, PlayerId, Pos, SimError, SimRng, World};
use rom_rules::Rules;
use std::{collections::BTreeMap, sync::Arc};

/// Mixed into the seed so obstacle placement does not disturb unit placement.
const OBSTACLE_STREAM: u64 = 0x6f62_7374_6163_6c65;

impl World {
    /// Builds the CLI's seeded 128×128 movement scenario using only components and ids.
    pub fn movement_scenario(rules: Arc<Rules>, seed: u64, units: u32) -> Result<Self, SimError> {
        Self::obstacle_scenario(rules, seed, units, 0)
    }

    /// The movement scenario with `obstacle_pct` percent of tiles set to a terrain the
    /// moving type cannot cross; units start on passable tiles, targets may be blocked.
    pub fn obstacle_scenario(
        rules: Arc<Rules>,
        seed: u64,
        units: u32,
        obstacle_pct: u32,
    ) -> Result<Self, SimError> {
        let entity_type = rules
            .entity_types
            .values()
            .find(|et| et.movement.is_some())
            .ok_or(SimError::ScenarioUnavailable("moving entity type"))?;
        let type_id = entity_type.id;
        let class = entity_type
            .movement
            .as_ref()
            .expect("selected moving type")
            .movement_class_id;
        let rows = rules.movement_class_terrain.get(&class);
        let terrain = rows
            .into_iter()
            .flatten()
            .filter(|row| row.speed_pct > 0)
            .map(|row| row.terrain_id)
            .min()
            .ok_or(SimError::ScenarioUnavailable("passable terrain"))?;
        let mut tiles = vec![terrain; 128 * 128];
        if obstacle_pct > 0 {
            if obstacle_pct >= 100 {
                return Err(SimError::ScenarioUnavailable("obstacle share below 100 %"));
            }
            let flies = rules.movement_classes.get(&class).is_some_and(|c| c.flies);
            let blocked = rules
                .terrains
                .keys()
                .copied()
                .find(|t| {
                    !flies
                        && !rows
                            .into_iter()
                            .flatten()
                            .any(|row| row.terrain_id == *t && row.speed_pct > 0)
                })
                .ok_or(SimError::ScenarioUnavailable("impassable terrain"))?;
            let mut rng = SimRng::new(seed ^ OBSTACLE_STREAM);
            for tile in &mut tiles {
                if rng.range(0, 100) < obstacle_pct {
                    *tile = blocked;
                }
            }
        }
        let player = Player {
            id: PlayerId(1),
            team: 1,
            faction: *rules
                .factions
                .keys()
                .next()
                .ok_or(SimError::ScenarioUnavailable("faction"))?,
            resources: BTreeMap::new(),
            techs: BTreeMap::new(),
            age: *rules
                .ages
                .keys()
                .next()
                .ok_or(SimError::ScenarioUnavailable("age"))?,
        };
        let mut world = Self::new(rules, MapGrid::new(128, 128, tiles)?, seed, vec![player])?;
        for _ in 0..units {
            let pos = loop {
                let pos = Pos {
                    x: world.rng.range(0, 128_000) as i32,
                    y: world.rng.range(0, 128_000) as i32,
                };
                if world.pathing.speed_pct(class, pos) > 0 {
                    break pos;
                }
            };
            let id = world.spawn(type_id, PlayerId(1), pos)?;
            let target = Pos {
                x: world.rng.range(0, 128_000) as i32,
                y: world.rng.range(0, 128_000) as i32,
            };
            world.submit(Command {
                player: PlayerId(1),
                executor: id,
                kind: "move".into(),
                subject: None,
                target_entity: None,
                target_pos: Some(target),
                queue: false,
            })?;
        }
        Ok(world)
    }
}

impl World {
    /// Builds two opposing lines, thirty tiles apart, on a crossable 96×96 map.
    pub fn battle_scenario(rules: Arc<Rules>, seed: u64, per_side: u32) -> Result<Self, SimError> {
        let et = rules
            .entity_types
            .values()
            .find(|e| e.movement.is_some() && !e.weapons.is_empty())
            .ok_or(SimError::ScenarioUnavailable("armed moving entity type"))?;
        let type_id = et.id;
        let class = et.movement.as_ref().expect("moving type").movement_class_id;
        let terrain = rules
            .movement_class_terrain
            .get(&class)
            .into_iter()
            .flatten()
            .filter(|r| r.speed_pct > 0)
            .map(|r| r.terrain_id)
            .min()
            .ok_or(SimError::ScenarioUnavailable("crossable terrain"))?;
        let faction = *rules
            .factions
            .keys()
            .next()
            .ok_or(SimError::ScenarioUnavailable("faction"))?;
        let age = *rules
            .ages
            .keys()
            .next()
            .ok_or(SimError::ScenarioUnavailable("age"))?;
        let players = (1..=2)
            .map(|n| Player {
                id: PlayerId(n),
                team: n,
                faction,
                age,
                resources: BTreeMap::new(),
                techs: BTreeMap::new(),
            })
            .collect();
        let mut world = Self::new(
            rules,
            MapGrid::new(96, 96, vec![terrain; 96 * 96])?,
            seed,
            players,
        )?;
        for side in 1..=2 {
            let (x, target) = if side == 1 {
                (33000, 63000)
            } else {
                (63000, 33000)
            };
            for n in 0..per_side {
                let y = (1000 + u64::from(n) * 94000 / u64::from(per_side.max(1))) as i32;
                let id = world.spawn(type_id, PlayerId(side), Pos { x, y })?;
                world.submit(Command {
                    player: PlayerId(side),
                    executor: id,
                    kind: "attack_move".into(),
                    subject: None,
                    target_entity: None,
                    target_pos: Some(Pos { x: target, y }),
                    queue: false,
                })?;
            }
        }
        Ok(world)
    }
}
