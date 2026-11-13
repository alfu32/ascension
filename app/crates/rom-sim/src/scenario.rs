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

/// Order-driven content audit on a map generated before the first tick.
#[derive(Debug)]
pub struct Playthrough {
    world: World,
    constructed: std::collections::BTreeSet<DefId>,
    trained: std::collections::BTreeSet<DefId>,
    placement_attempts: BTreeMap<DefId, (usize, usize)>,
    ages: Vec<(DefId, u64)>,
}

impl Playthrough {
    /// Starts with a city, a builder, neutral map nodes and one million of each base resource.
    pub fn new(
        rules: Arc<Rules>,
        map: MapGrid,
        start: Pos,
        nodes: &[(DefId, Pos)],
        seed: u64,
    ) -> Result<Self, SimError> {
        let age = rules
            .ages
            .values()
            .min_by_key(|a| (a.ordinal, a.id))
            .ok_or(SimError::ScenarioUnavailable("starting age"))?
            .id;
        let faction = rules
            .playable_start_factions(age)
            .into_iter()
            .next()
            .ok_or(SimError::ScenarioUnavailable("starting faction"))?;
        let neutral = rules
            .factions
            .values()
            .find(|f| f.kind == rom_rules::FactionKind::Npc)
            .ok_or(SimError::ScenarioUnavailable("neutral faction"))?
            .id;
        let resources = rules
            .resources
            .values()
            .filter(|r| r.kind == rom_rules::ResourceKind::Base)
            .map(|r| (r.id, 1_000_000))
            .collect();
        let players = vec![
            Player {
                id: PlayerId(0),
                team: 0,
                faction: neutral,
                age,
                resources: BTreeMap::new(),
                techs: BTreeMap::new(),
            },
            Player {
                id: PlayerId(1),
                team: 1,
                faction,
                age,
                resources,
                techs: BTreeMap::new(),
            },
        ];
        let builder = rules
            .entity_types
            .values()
            .find(|e| e.builder.is_some() && e.upgrades_from_id.is_none())
            .ok_or(SimError::ScenarioUnavailable("builder type"))?
            .id;
        let mut world =
            World::new(rules, map, seed, players)?.with_match_options(crate::MatchOptions {
                victories: Default::default(),
                ..crate::MatchOptions::default()
            })?;
        let city = world
            .city_type(PlayerId(1))
            .ok_or(SimError::ScenarioUnavailable("city type"))?;
        let city_pos = world
            .find_city_site(PlayerId(1), start.body, start)
            .ok_or(SimError::ScenarioUnavailable("starting city site"))?;
        world.place_for_setup(city, PlayerId(1), city_pos)?;
        world.spawn(builder, PlayerId(1), world.offset(city_pos, 5000, 0))?;
        for (type_id, pos) in nodes {
            world.spawn(*type_id, PlayerId(0), *pos)?;
        }
        Ok(Self {
            world,
            constructed: [city].into_iter().collect(),
            trained: Default::default(),
            placement_attempts: BTreeMap::new(),
            ages: vec![(age, 0)],
        })
    }

    /// Immutable state for progress, statistics and the deterministic state hash.
    pub fn world(&self) -> &World {
        &self.world
    }

    /// Stable hash of the match and the bot's production history and placement cache.
    pub fn hash(&self) -> u64 {
        let mut hash = self.world.hash();
        let mut add = |value: u64| {
            for byte in value.to_le_bytes() {
                hash = (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3);
            }
        };
        for set in [&self.constructed, &self.trained] {
            add(set.len() as u64);
            for id in set {
                add(u64::from(id.0));
            }
        }
        add(self.placement_attempts.len() as u64);
        for (id, (entities, techs)) in &self.placement_attempts {
            add(u64::from(id.0));
            add(*entities as u64);
            add(*techs as u64);
        }
        add(self.ages.len() as u64);
        for (id, tick) in &self.ages {
            add(u64::from(id.0));
            add(*tick);
        }
        hash
    }

    /// Ages reached, in order, including the starting age at tick zero.
    pub fn ages(&self) -> &[(DefId, u64)] {
        &self.ages
    }

    /// Whether every available production item has been exercised and completed.
    pub fn is_complete(&self) -> bool {
        use rom_rules::ProductionMode;
        let player = &self.world.players[&PlayerId(1)];
        self.world.rules.production.values().flatten().all(|row| {
            if !self
                .world
                .rules
                .is_available(row.product_def_id, player.faction)
            {
                return true;
            }
            match row.mode {
                ProductionMode::Construct => self.constructed.contains(&row.product_def_id),
                ProductionMode::Train => self.trained.contains(&row.product_def_id),
                ProductionMode::Research => player.techs.contains_key(&row.product_def_id),
                ProductionMode::Morph => true,
            }
        }) && !self
            .world
            .all_orders()
            .any(|o| matches!(o.kind.as_str(), "construct" | "train" | "research"))
    }

    /// Advances one tick, issuing only ordinary construct, research and train orders.
    pub fn step(&mut self) {
        self.plan();
        self.world.step();
        let age = self.world.players[&PlayerId(1)].age;
        if self
            .ages
            .last()
            .is_none_or(|(previous, _)| *previous != age)
        {
            self.ages.push((age, self.world.tick()));
        }
    }

    fn plan(&mut self) {
        use rom_rules::{ProductionMode, TechKind};
        let player = PlayerId(1);
        let rules = self.world.rules.clone();
        self.constructed.extend(
            self.world
                .entities()
                .filter(|e| {
                    e.owner == player
                        && e.build_progress.is_none()
                        && rules.entity_types[&e.type_id].class == rom_rules::EntityClass::Building
                })
                .map(|e| e.type_id),
        );
        let owner = &self.world.players[&player];
        let active = owner.active_sources();
        let owned = owner.owned_types(&self.world);
        let faction = owner.faction;
        let researched: std::collections::BTreeSet<_> = owner.techs.keys().copied().collect();
        let queued: std::collections::BTreeSet<_> = self
            .world
            .all_orders()
            .filter(|o| o.player == player)
            .filter_map(|o| o.subject)
            .collect();
        let executors: Vec<_> = self
            .world
            .entities()
            .filter(|e| {
                e.owner == player
                    && e.build_progress.is_none()
                    && e.orders.iter().all(|o| o.kind == "harvest")
            })
            .map(|e| (e.id, e.type_id))
            .collect();
        let signature = (
            self.world
                .entities()
                .filter(|e| e.build_progress.is_none())
                .count(),
            researched.len(),
        );
        for (executor, type_id) in executors {
            let products = rules
                .production
                .get(&type_id)
                .map_or(&[][..], Vec::as_slice);
            let mut choices: Vec<_> = products
                .iter()
                .filter(|p| {
                    !queued.contains(&p.product_def_id)
                        && rules.is_available(p.product_def_id, faction)
                        && rules.is_unlocked(p.product_def_id, &active, &owned)
                })
                .collect();
            // Train each step before researching its successor. Library ages come
            // first, then the lowest unresearched level of each track, then projects.
            choices.sort_by_key(|p| {
                let tech = rules.techs.get(&p.product_def_id);
                (
                    match p.mode {
                        ProductionMode::Train => 0,
                        ProductionMode::Research
                            if tech.is_some_and(|t| t.kind == TechKind::Age) =>
                        {
                            1
                        }
                        ProductionMode::Research
                            if tech.is_some_and(|t| t.kind == TechKind::Track) =>
                        {
                            2
                        }
                        ProductionMode::Research => 3,
                        _ => 4,
                    },
                    tech.map_or(0, |t| t.level),
                    p.product_def_id,
                )
            });
            for p in choices {
                let subject = p.product_def_id;
                let mut pos = None;
                match p.mode {
                    ProductionMode::Construct => {
                        let city = self.world.is_city_type(subject);
                        if !city
                            && (self.constructed.contains(&subject) || owned.contains(&subject))
                        {
                            continue;
                        }
                        if city && self.world.city_count(player) >= self.world.city_limit(player) {
                            continue;
                        }
                        if self.placement_attempts.get(&subject) == Some(&signature) {
                            continue;
                        }
                        pos = self.building_site(subject);
                        if pos.is_none() {
                            self.placement_attempts.insert(subject, signature);
                            continue;
                        }
                    }
                    ProductionMode::Train => {
                        if self.trained.contains(&subject)
                            || rules.is_obsolete(subject, &active, &owned)
                        {
                            continue;
                        }
                    }
                    ProductionMode::Research => {
                        if researched.contains(&subject) {
                            continue;
                        }
                        let tech = &rules.techs[&subject];
                        if tech.kind == TechKind::Track
                            && rules.techs.values().any(|earlier| {
                                earlier.kind == TechKind::Track
                                    && earlier.track == tech.track
                                    && earlier.level < tech.level
                                    && !researched.contains(&earlier.id)
                            })
                        {
                            continue;
                        }
                    }
                    ProductionMode::Morph => continue,
                }
                let kind = match p.mode {
                    ProductionMode::Construct => "construct",
                    ProductionMode::Train => "train",
                    ProductionMode::Research => "research",
                    ProductionMode::Morph => unreachable!(),
                };
                if self
                    .world
                    .submit(Command {
                        player,
                        executor,
                        kind: kind.into(),
                        subject: Some(subject),
                        target_entity: None,
                        target_pos: pos,
                        queue: false,
                    })
                    .is_ok()
                {
                    match p.mode {
                        ProductionMode::Construct => {
                            self.constructed.insert(subject);
                        }
                        ProductionMode::Train => {
                            self.trained.insert(subject);
                        }
                        _ => {}
                    }
                    break;
                }
            }
        }
    }

    fn building_site(&self, subject: DefId) -> Option<Pos> {
        let player = PlayerId(1);
        // Square rings give a stable outward spiral around each city. Both the
        // placement preview and the per-city limit must pass before submission.
        for city in self.world.cities(player) {
            let centre = self.world.entity(city)?.pos;
            for radius in 0_i32..=64 {
                let ring: Vec<_> = if radius == 0 {
                    vec![(0, 0)]
                } else {
                    (-radius..radius)
                        .flat_map(|n| [(n, -radius), (radius, n), (-n, radius), (-radius, -n)])
                        .collect()
                };
                for (dx, dy) in ring {
                    let pos = self
                        .world
                        .offset(centre, i64::from(dx) * 1000, i64::from(dy) * 1000);
                    if self.world.territory_owner(pos.x / 1000, pos.y / 1000) != Some(player) {
                        continue;
                    }
                    if self.world.rules.entity_types[&subject]
                        .placement_rules
                        .iter()
                        .any(|rule| {
                            rule.rule == rom_rules::PlacementKind::OnTerrain
                                && self.world.terrain_at(pos) != rule.ref_def_id
                        })
                    {
                        continue;
                    }
                    let candidate = crate::construction::footprint(
                        &self.world.rules.entity_types[&subject],
                        pos,
                    );
                    // Leave a tile-wide lane between footprints so completed
                    // buildings cannot enclose a builder or block later sites.
                    let blocked = self
                        .world
                        .entities()
                        .filter(|e| self.world.is_building(e.type_id))
                        .any(|e| {
                            let other = crate::construction::footprint(
                                &self.world.rules.entity_types[&e.type_id],
                                e.pos,
                            );
                            candidate[0] - 1000 < other[2]
                                && other[0] < candidate[2] + 1000
                                && candidate[1] - 1000 < other[3]
                                && other[1] < candidate[3] + 1000
                        });
                    if blocked {
                        continue;
                    }
                    if let Ok(pos) = self.world.check_placement(player, subject, pos)
                        && self.world.check_city_limit(player, subject, pos).is_ok()
                    {
                        return Some(pos);
                    }
                }
            }
        }
        None
    }
}

impl World {
    /// Two cities thirty tiles apart, two carried nodes, a keeper and two route traders.
    pub fn wealth_scenario(rules: Arc<Rules>, seed: u64, sphere: bool) -> Result<Self, SimError> {
        use rom_rules::{ClaimMode, EntityClass, ProductionMode};
        let find = |predicate: &dyn Fn(&rom_rules::EntityType) -> bool, label| {
            rules
                .entity_types
                .values()
                .find(|e| predicate(e))
                .map(|e| e.id)
                .ok_or(SimError::ScenarioUnavailable(label))
        };
        let node = find(
            &|e| {
                e.node
                    .as_ref()
                    .is_some_and(|n| n.claim_mode == ClaimMode::Carry)
            },
            "carry node",
        )?;
        let tag = rules.entity_types[&node]
            .node
            .as_ref()
            .and_then(|n| n.claim_tag_id)
            .ok_or(SimError::ScenarioUnavailable("claim tag"))?;
        let priest = find(
            &|e| e.class == EntityClass::Unit && e.movement.is_some() && rules.has_tag(e.id, tag),
            "carrier",
        )?;
        let keeper = find(&|e| e.keeper.is_some(), "keeper")?;
        let city = find(&|e| e.territory.as_ref().is_some_and(|t| t.is_city), "city")?;
        let mut traders: Vec<_> = rules
            .entity_types
            .values()
            .filter(|e| {
                e.trader.as_ref().is_some_and(|t| t.runs_routes) && e.upgrades_from_id.is_none()
            })
            .collect();
        traders.sort_by_key(|e| (e.trader.as_ref().expect("trader").route_income, e.id));
        let traders = traders
            .into_iter()
            .take(2)
            .map(|e| e.id)
            .collect::<Vec<_>>();
        if traders.len() != 2 {
            return Err(SimError::ScenarioUnavailable("two traders"));
        }
        let market = find(
            &|e| {
                e.class == EntityClass::Building
                    && rules.production.get(&e.id).is_some_and(|rows| {
                        rows.iter().any(|p| {
                            p.mode == ProductionMode::Train && traders.contains(&p.product_def_id)
                        })
                    })
            },
            "trader producer",
        )?;
        let terrain = rules
            .terrains
            .values()
            .find(|t| t.layer == rom_rules::Layer::Ground && t.buildable && !t.blocks_los)
            .ok_or(SimError::ScenarioUnavailable("ground terrain"))?
            .id;
        let players = scenario_players(&rules, 1)?;
        let mut w = if sphere {
            Self::from_sphere(
                rules,
                CubeSurface { n: 128 },
                vec![terrain; 6 * 128 * 128],
                &[],
                seed,
                players,
            )?
        } else {
            Self::new(
                rules,
                MapGrid::new(96, 96, vec![terrain; 96 * 96])?,
                seed,
                players,
            )?
        }
        .with_match_options(crate::MatchOptions {
            victories: Default::default(),
            ..Default::default()
        })?;
        let p = PlayerId(1);
        let home_pos = if sphere {
            Pos::flat(110_500, 64_500)
        } else {
            Pos::flat(16_500, 32_500)
        };
        w.spawn(city, p, home_pos)?;
        let target = w.spawn(city, p, w.offset(home_pos, 30_000, 0))?;
        w.spawn(keeper, p, w.offset(home_pos, 0, -8000))?;
        w.spawn(market, p, w.offset(home_pos, 0, 8000))?;
        for n in 0..2 {
            let pos = w.offset(home_pos, -7000 - n * 3000, -3000);
            let relic = w.spawn(node, p, pos)?;
            let carrier = w.spawn(priest, p, w.offset(pos, 0, 1000))?;
            w.submit(Command {
                player: p,
                executor: carrier,
                kind: "carry".into(),
                subject: None,
                target_entity: Some(relic),
                target_pos: None,
                queue: false,
            })?;
        }
        for (n, trader) in traders.into_iter().enumerate() {
            let executor = w.spawn(trader, p, w.offset(home_pos, -5000, n as i64 * 2000))?;
            w.submit(Command {
                player: p,
                executor,
                kind: "trade".into(),
                subject: None,
                target_entity: Some(target),
                target_pos: None,
                queue: false,
            })?;
        }
        Ok(w)
    }
}

#[cfg(test)]
mod content_tests {
    use super::*;
    use rom_rules::build_rules_db;

    fn rules() -> Arc<Rules> {
        let db = build_rules_db(
            include_str!("../../../db/schema.sql"),
            include_str!("../../../db/seed.sql"),
        )
        .unwrap();
        Arc::new(Rules::load(&db).unwrap())
    }

    fn audit() -> Playthrough {
        audit_at_pace(1)
    }

    fn audit_at_pace(pace: i64) -> Playthrough {
        let mut catalogue = (*rules()).clone();
        // Accelerate durations and travel in the debug progression tests; costs,
        // prerequisites, effects and the complete generated roster stay intact.
        for tech in catalogue.techs.values_mut() {
            tech.research_ticks = (tech.research_ticks / pace).max(1);
        }
        for entity in catalogue.entity_types.values_mut() {
            entity.build_ticks = (entity.build_ticks / pace).max(1);
            if let Some(movement) = &mut entity.movement {
                movement.speed *= pace;
            }
        }
        let r = Arc::new(catalogue);
        // Uniform land with a shore and deposits: the CLI separately audits the
        // generated one-world map, while these tests isolate content progression.
        let mut terrain = vec![r.id_of("ter.plain").unwrap(); 128 * 128];
        for y in 0..128 {
            for x in 0..128 {
                let key = if x < 12 {
                    Some("ter.coast")
                } else if (20..24).contains(&x) && (20..24).contains(&y) {
                    Some("ter.oil")
                } else if (50..54).contains(&x) && (20..24).contains(&y) {
                    Some("ter.forest")
                } else if (20..24).contains(&x) && (50..54).contains(&y) {
                    Some("ter.mountain")
                } else {
                    None
                };
                if let Some(key) = key {
                    terrain[y * 128 + x] = r.id_of(key).unwrap();
                }
            }
        }
        Playthrough::new(
            r,
            MapGrid::new(128, 128, terrain).unwrap(),
            Pos::flat(32_500, 32_500),
            &[],
            1,
        )
        .unwrap()
    }

    fn reach(audit: &mut Playthrough, ordinal: i64) {
        while audit.world.rules.ages[&audit.world.players[&PlayerId(1)].age].ordinal < ordinal
            && audit.world.tick() < 100_000
        {
            audit.step();
        }
        assert_eq!(
            audit.world.rules.ages[&audit.world.players[&PlayerId(1)].age].ordinal,
            ordinal
        );
    }

    #[test]
    fn playthrough_reaches_industrial() {
        let mut audit = audit_at_pace(100);
        reach(&mut audit, 5);
        assert!(audit.world.tick() < 30_000);
        assert_eq!(audit.world.stats.orders_unsupported, 0);
    }

    #[test]
    #[ignore = "full fourteen-age content audit; also run by the release CLI gate"]
    fn playthrough_reaches_galactic() {
        let mut audit = audit();
        reach(&mut audit, 13);
        assert!(audit.world.tick() < 400_000);
    }

    #[test]
    fn playthrough_is_deterministic() {
        let mut first = audit_at_pace(100);
        let mut second = audit_at_pace(100);
        for _ in 0..120 {
            first.step();
            second.step();
        }
        assert!(first.ages.len() > 1);
        assert_eq!(first.world.hash(), second.world.hash());
        assert_eq!(first.hash(), second.hash());
        assert_eq!(first.ages, second.ages);
        assert_eq!(first.constructed, second.constructed);
        assert_eq!(first.trained, second.trained);
    }

    #[test]
    fn barracks_needs_military_1() {
        let mut a = audit();
        let r = a.world.rules.clone();
        let b = r.id_of("bld.barracks").unwrap();
        let builder = a
            .world
            .entities()
            .find(|e| r.entity_types[&e.type_id].builder.is_some())
            .unwrap()
            .id;
        let pos = a.building_site(b).unwrap();
        let order = Command {
            player: PlayerId(1),
            executor: builder,
            kind: "construct".into(),
            subject: Some(b),
            target_entity: None,
            target_pos: Some(pos),
            queue: true,
        };
        let before = a.world.hash();
        assert_eq!(a.world.submit(order.clone()), Err(SimError::Locked));
        assert_eq!(a.world.hash(), before);
        a.world
            .players
            .get_mut(&PlayerId(1))
            .unwrap()
            .techs
            .insert(r.id_of("tech.military.1").unwrap(), 1);
        a.world.submit(order).unwrap();
    }

    #[test]
    fn classical_needs_two_level_one_tracks() {
        let mut a = audit();
        let r = a.world.rules.clone();
        let library = a
            .world
            .spawn(
                r.id_of("bld.library").unwrap(),
                PlayerId(1),
                Pos::flat(60_000, 60_000),
            )
            .unwrap();
        let order = Command {
            player: PlayerId(1),
            executor: library,
            kind: "research".into(),
            subject: r.id_of("tech.age.classical"),
            target_entity: None,
            target_pos: None,
            queue: true,
        };
        assert_eq!(a.world.submit(order.clone()), Err(SimError::Locked));
        a.world
            .players
            .get_mut(&PlayerId(1))
            .unwrap()
            .techs
            .insert(r.id_of("tech.military.1").unwrap(), 1);
        assert_eq!(a.world.submit(order.clone()), Err(SimError::Locked));
        a.world
            .players
            .get_mut(&PlayerId(1))
            .unwrap()
            .techs
            .insert(r.id_of("tech.commerce.1").unwrap(), 1);
        a.world.submit(order).unwrap();
    }

    #[test]
    fn automation_releases_workers_and_replaces_their_income() {
        let run = || {
            let mut a = audit();
            let r = a.world.rules.clone();
            a.world.players.get_mut(&PlayerId(1)).unwrap().age = r.id_of("age.automation").unwrap();
            let farm = a
                .world
                .place_for_setup(
                    r.id_of("bld.farm").unwrap(),
                    PlayerId(1),
                    Pos::flat(60_000, 60_000),
                )
                .unwrap();
            let worker = a
                .world
                .spawn(
                    r.id_of("unit.citizen").unwrap(),
                    PlayerId(1),
                    Pos::flat(57_500, 60_000),
                )
                .unwrap();
            a.world
                .submit(Command {
                    player: PlayerId(1),
                    executor: worker,
                    kind: "harvest".into(),
                    subject: None,
                    target_entity: Some(farm),
                    target_pos: None,
                    queue: false,
                })
                .unwrap();
            a.world.step();
            assert_eq!(a.world.entity(worker).unwrap().work_site, Some(farm));
            a.world
                .submit(Command {
                    player: PlayerId(1),
                    executor: farm,
                    kind: "research".into(),
                    subject: r.id_of("tech.up.auto_farm"),
                    target_entity: None,
                    target_pos: None,
                    queue: true,
                })
                .unwrap();
            for _ in 0..700 {
                a.world.step();
            }
            assert_eq!(
                a.world.entity(farm).unwrap().type_id,
                r.id_of("bld.auto_farm").unwrap()
            );
            assert_eq!(a.world.entity(farm).unwrap().gather_slots, 0);
            assert_eq!(a.world.entity(worker).unwrap().work_site, None);
            // Upgraded farms still count toward the original per-city limit.
            let city_pos = a.world.entity(a.world.cities(PlayerId(1))[0]).unwrap().pos;
            for n in 0..5 {
                a.world
                    .place_for_setup(
                        r.id_of("bld.auto_farm").unwrap(),
                        PlayerId(1),
                        a.world.offset(city_pos, 8000, n * 1000),
                    )
                    .unwrap();
            }
            assert_eq!(
                a.world
                    .check_city_limit(PlayerId(1), r.id_of("bld.farm").unwrap(), city_pos),
                Err(SimError::LimitReached)
            );
            assert_eq!(
                a.world
                    .income(PlayerId(1), r.id_of("res.food").unwrap())
                    .raw,
                60
            );
            a.world.hash()
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn training_an_obsolete_type_is_rejected() {
        let mut a = audit();
        let r = a.world.rules.clone();
        let owner = a.world.players.get_mut(&PlayerId(1)).unwrap();
        owner.age = r.id_of("age.classical").unwrap();
        owner.techs.insert(r.id_of("tech.up.hoplite").unwrap(), 1);
        owner.resources.clear(); // Obsolete must be checked before the price.
        let b = a
            .world
            .spawn(
                r.id_of("bld.barracks").unwrap(),
                PlayerId(1),
                Pos::flat(60_000, 60_000),
            )
            .unwrap();
        let before = a.world.hash();
        assert_eq!(
            a.world.submit(Command {
                player: PlayerId(1),
                executor: b,
                kind: "train".into(),
                subject: r.id_of("unit.spearman"),
                target_entity: None,
                target_pos: None,
                queue: true
            }),
            Err(SimError::Obsolete)
        );
        assert_eq!(before, a.world.hash());
    }

    #[test]
    fn upgrade_tech_converts_existing_units_and_obsoletes_the_old_step() {
        let run = || {
            let mut a = audit();
            let r = a.world.rules.clone();
            a.world.players.get_mut(&PlayerId(1)).unwrap().age = r.id_of("age.classical").unwrap();
            let b = a
                .world
                .spawn(
                    r.id_of("bld.barracks").unwrap(),
                    PlayerId(1),
                    Pos::flat(60_000, 60_000),
                )
                .unwrap();
            let unit = a
                .world
                .spawn(
                    r.id_of("unit.spearman").unwrap(),
                    PlayerId(1),
                    Pos::flat(60_000, 65_000),
                )
                .unwrap();
            let old_max = a.world.entity(unit).unwrap().max_hp;
            a.world.entities[unit.0 as usize].as_mut().unwrap().hp = old_max / 2;
            a.world
                .submit(Command {
                    player: PlayerId(1),
                    executor: b,
                    kind: "research".into(),
                    subject: r.id_of("tech.up.hoplite"),
                    target_entity: None,
                    target_pos: None,
                    queue: true,
                })
                .unwrap();
            for _ in 0..400 {
                a.world.step();
            }
            let e = a.world.entity(unit).unwrap();
            assert_eq!(e.type_id, r.id_of("unit.hoplite").unwrap());
            assert_eq!(e.hp, e.max_hp / 2);
            assert!(r.is_obsolete(
                r.id_of("unit.spearman").unwrap(),
                &a.world.players[&PlayerId(1)].active_sources(),
                &a.world.owned_types(PlayerId(1))
            ));
            a.world.hash()
        };
        assert_eq!(run(), run());
    }
}
