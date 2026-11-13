//! Seeded headless scenarios for the command-line runner and benchmarks.

use crate::{
    Command, MapGrid, Player, PlayerId, Pos, SimError, SimRng, Topology, World, geometry::Geometry,
};
use rom_geo::CubeSurface;
use rom_rules::{DefId, Rules};
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
        let (type_id, class, terrain) = mover(&rules, false)?;
        let rows = rules.movement_class_terrain.get(&class);
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
        let players = scenario_players(&rules, 1)?;
        let mut world = Self::new(rules, MapGrid::new(128, 128, tiles)?, seed, players)?;
        for _ in 0..units {
            let pos = loop {
                let pos = Pos::flat(
                    world.rng.range(0, 128_000) as i32,
                    world.rng.range(0, 128_000) as i32,
                );
                if world.surfaces[0].pathing.speed_pct(class, pos) > 0 {
                    break pos;
                }
            };
            let id = world.spawn(type_id, PlayerId(1), pos)?;
            let target = Pos::flat(
                world.rng.range(0, 128_000) as i32,
                world.rng.range(0, 128_000) as i32,
            );
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
        Self::battle_scenario_swapped(rules, seed, per_side, false)
    }

    /// Builds the battle with player ids exchanged when `swap` is true; positions
    /// and entity allocation stay the same so ownership is the only difference.
    pub fn battle_scenario_swapped(
        rules: Arc<Rules>,
        seed: u64,
        per_side: u32,
        swap: bool,
    ) -> Result<Self, SimError> {
        let (type_id, _, terrain) = mover(&rules, true)?;
        let players = scenario_players(&rules, 2)?;
        let mut world = Self::new(
            rules,
            MapGrid::new(96, 96, vec![terrain; 96 * 96])?,
            seed,
            players,
        )?;
        for side in 1..=2 {
            let owner = PlayerId(if swap { 3 - side } else { side });
            let (x, target) = if side == 1 {
                (33000, 63000)
            } else {
                (63000, 33000)
            };
            for n in 0..per_side {
                let y = (1000 + u64::from(n) * 94000 / u64::from(per_side.max(1))) as i32;
                let id = world.spawn(type_id, owner, Pos::flat(x, y))?;
                world.submit(Command {
                    player: owner,
                    executor: id,
                    kind: "attack_move".into(),
                    subject: None,
                    target_entity: None,
                    target_pos: Some(Pos::flat(target, y)),
                    queue: false,
                })?;
            }
        }
        Ok(world)
    }
}

/// The first moving type (armed when `armed`), its movement class, and the lowest-id
/// terrain it can cross.
fn mover(rules: &Rules, armed: bool) -> Result<(DefId, u32, DefId), SimError> {
    let et = rules
        .entity_types
        .values()
        .find(|e| e.movement.is_some() && (!armed || !e.weapons.is_empty()))
        .ok_or(SimError::ScenarioUnavailable(if armed {
            "armed moving entity type"
        } else {
            "moving entity type"
        }))?;
    let class = et.movement.as_ref().expect("moving type").movement_class_id;
    let terrain = rules
        .movement_class_terrain
        .get(&class)
        .into_iter()
        .flatten()
        .filter(|r| r.speed_pct > 0)
        .map(|r| r.terrain_id)
        .min()
        .ok_or(SimError::ScenarioUnavailable(if armed {
            "crossable terrain"
        } else {
            "passable terrain"
        }))?;
    Ok((et.id, class, terrain))
}

/// Players 1..=count on teams of their own, with the first faction and age.
fn scenario_players(rules: &Rules, count: u32) -> Result<Vec<Player>, SimError> {
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
    Ok((1..=count)
        .map(|n| Player {
            id: PlayerId(n),
            team: n,
            faction,
            age,
            resources: BTreeMap::new(),
            techs: BTreeMap::new(),
        })
        .collect())
}

/// Largest displacement along each axis for a sphere movement order, in milli-tiles.
const SPHERE_MOVE_SPAN: i64 = 64_000;
/// Distance of each battle line from the face edge between them, in milli-tiles.
const LINE_DEPTH: i64 = 15_000;
/// Length of each battle line, as in the flat battle.
const LINE_LENGTH: i64 = 94_000;
/// Radius around the edge midpoint levelled to crossable ground for a battle.
const BATTLEFIELD_RADIUS: i64 = 60_000;
/// The battle's home face and the edge it fights across: face 1's top edge meets face 4
/// with a quarter turn, so the lines also exercise frame rotation.
const BATTLE_FACE: u8 = 1;

impl World {
    /// The movement scenario on a sphere planet: `units` of the first moving type on
    /// random passable tiles, each ordered to a random point up to 64 tiles away along
    /// each local axis, crossing face edges where that leads.
    pub fn sphere_movement_scenario(
        rules: Arc<Rules>,
        surface: CubeSurface,
        terrain: Vec<DefId>,
        seed: u64,
        units: u32,
    ) -> Result<Self, SimError> {
        let (type_id, class, _) = mover(&rules, false)?;
        let players = scenario_players(&rules, 1)?;
        let mut world = Self::from_sphere(rules, surface, terrain, &[], seed, players)?;
        let geo = world.surfaces[0].geo.clone();
        if (0..geo.tiles()).all(|i| {
            world.surfaces[0]
                .pathing
                .speed_pct(class, geo.index_centre(i))
                == 0
        }) {
            return Err(SimError::ScenarioUnavailable("passable terrain"));
        }
        let side = surface.n * 1000;
        let span = SPHERE_MOVE_SPAN.min(i64::from(side) / 2);
        for _ in 0..units {
            let pos = loop {
                let pos = Pos {
                    face: world.rng.range(0, 6) as u8,
                    x: world.rng.range(0, side) as i32,
                    y: world.rng.range(0, side) as i32,
                    ..Pos::default()
                };
                if world.surfaces[0].pathing.speed_pct(class, pos) > 0 {
                    break pos;
                }
            };
            let id = world.spawn(type_id, PlayerId(1), pos)?;
            let dx = i64::from(world.rng.range(0, 2 * span as u32 + 1)) - span;
            let dy = i64::from(world.rng.range(0, 2 * span as u32 + 1)) - span;
            let target = geo.offset(pos, dx, dy).0;
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

    /// The battle on a sphere planet: the two lines of the flat battle, thirty tiles
    /// apart, on opposite sides of a face edge so the fight crosses it. Ground around
    /// the edge is levelled to the crossable terrain first, as the flat battle's map is
    /// uniform. Needs at least 128 tiles per face edge.
    pub fn sphere_battle_scenario(
        rules: Arc<Rules>,
        surface: CubeSurface,
        mut terrain: Vec<DefId>,
        seed: u64,
        per_side: u32,
        swap: bool,
    ) -> Result<Self, SimError> {
        if surface.n < 128 {
            return Err(SimError::ScenarioUnavailable(
                "sphere of at least 128 tiles per face",
            ));
        }
        let (type_id, _, ground) = mover(&rules, true)?;
        let geo = Geometry::new(Topology::Cube(surface)).ok_or(SimError::InvalidMap)?;
        if terrain.len() != geo.tiles() {
            return Err(SimError::InvalidMap);
        }
        let side = i64::from(surface.n) * 1000;
        let middle = Pos {
            face: BATTLE_FACE,
            x: (side / 2) as i32,
            y: 0,
            ..Pos::default()
        };
        for index in geo.disc(middle, BATTLEFIELD_RADIUS).flatten() {
            terrain[index] = ground;
        }
        let players = scenario_players(&rules, 2)?;
        let mut world = Self::from_sphere(rules, surface, terrain, &[], seed, players)?;
        let line = |n: u32| {
            let along = side / 2 - LINE_LENGTH / 2
                + (1000 + i64::from(n) * LINE_LENGTH / i64::from(per_side.max(1)));
            let near = Pos {
                face: BATTLE_FACE,
                x: along as i32,
                y: LINE_DEPTH as i32,
                ..Pos::default()
            };
            (near, geo.offset(near, 0, -2 * LINE_DEPTH).0)
        };
        for side in 1..=2 {
            let owner = PlayerId(if swap { 3 - side } else { side });
            for n in 0..per_side {
                let (near, far) = line(n);
                let (start, target) = if side == 1 { (near, far) } else { (far, near) };
                let id = world.spawn(type_id, owner, start)?;
                world.submit(Command {
                    player: owner,
                    executor: id,
                    kind: "attack_move".into(),
                    subject: None,
                    target_entity: None,
                    target_pos: Some(target),
                    queue: false,
                })?;
            }
        }
        Ok(world)
    }
}

impl World {
    /// Two cities and forty armed infantry attacking the second capital, on a flat
    /// 96×96 map or a uniform cube sphere. Types are selected by their components.
    pub fn siege_scenario(
        rules: Arc<Rules>,
        seed: u64,
        sphere: Option<CubeSurface>,
    ) -> Result<Self, SimError> {
        let infantry = rules
            .entity_types
            .values()
            .find(|e| {
                e.class == rom_rules::EntityClass::Unit
                    && !e.weapons.is_empty()
                    && e.movement.is_some()
                    && e.builder.is_none()
                    && e.harvester.is_empty()
            })
            .ok_or(SimError::ScenarioUnavailable("armed infantry"))?;
        let unit = infantry.id;
        let class = infantry
            .movement
            .as_ref()
            .expect("infantry movement")
            .movement_class_id;
        let ground = rules
            .movement_class_terrain
            .get(&class)
            .into_iter()
            .flatten()
            .filter(|r| {
                r.speed_pct > 0 && rules.terrains[&r.terrain_id].layer == rom_rules::Layer::Ground
            })
            .map(|r| r.terrain_id)
            .min()
            .ok_or(SimError::ScenarioUnavailable("infantry terrain"))?;
        let players = scenario_players(&rules, 2)?;
        let mut world = if let Some(surface) = sphere {
            if !(96..=4096).contains(&surface.n) {
                return Err(SimError::ScenarioUnavailable(
                    "siege sphere of at least 96 tiles",
                ));
            }
            Self::from_sphere(
                rules,
                surface,
                vec![ground; surface.len()],
                &[],
                seed,
                players,
            )?
        } else {
            Self::new(
                rules,
                MapGrid::new(96, 96, vec![ground; 96 * 96])?,
                seed,
                players,
            )?
        };
        let city = world
            .city_type(PlayerId(1))
            .ok_or(SimError::ScenarioUnavailable("city"))?;
        world.place_for_setup(city, PlayerId(1), Pos::flat(20_500, 20_500))?;
        let target = world.place_for_setup(city, PlayerId(2), Pos::flat(68_500, 68_500))?;
        for n in 0..40 {
            let pos = Pos::flat(64_000 - n % 5 * 800, 67_000 + n / 5 * 600);
            let id = world.spawn(unit, PlayerId(1), pos)?;
            world.submit(Command {
                player: PlayerId(1),
                executor: id,
                kind: "attack".into(),
                subject: None,
                target_entity: Some(target),
                target_pos: None,
                queue: false,
            })?;
        }
        Ok(world)
    }
}
