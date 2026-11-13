//! The universe a world belongs to: star systems, bodies and routes, passed in as plain
//! data, and the habitable surfaces generated so far. Surfaces of bodies that are not
//! generated yet are added during the match with [`World::add_surface`].

use crate::{EntityId, MapGrid, Player, PlayerId, SimError, Topology, World, surface::Surface};
use rom_geo::{CubeSurface, SurfacePos};
use rom_rules::{BodyKind, DefId, Rules};
use std::{collections::BTreeMap, sync::Arc};

/// A body of the universe; its surface, if any, is keyed by this id. A world with a
/// single surface (a flat map or one planet) uses body 0.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BodyId(pub u32);

/// A star system on the star map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemInfo {
    /// Universe system id.
    pub id: u32,
    /// Display name.
    pub name: String,
    /// Star map position, abstract units.
    pub x: i32,
    /// Star map position, abstract units.
    pub y: i32,
}

/// A star, habitable planet or resource body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BodyInfo {
    /// Universe body id.
    pub id: BodyId,
    /// The system it belongs to.
    pub system: u32,
    /// Star, habitable or resource.
    pub kind: BodyKind,
    /// Display name.
    pub name: String,
    /// Distance from the star in system units.
    pub orbit_radius: i32,
    /// Position on the orbit, 0..360000 millidegrees.
    pub orbit_angle_mdeg: i32,
    /// Cube face size of a habitable planet.
    pub n: Option<u32>,
    /// The race whose players start here.
    pub home_race: Option<DefId>,
    /// The node a resource body yields.
    pub node_type: Option<DefId>,
}

/// A route between two systems.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct RouteInfo {
    /// Lower system id.
    pub system_a: u32,
    /// Higher system id.
    pub system_b: u32,
    /// Integer star map distance.
    pub length: i32,
}

/// A generated planet surface: shape, terrain in the surface's dense tile order, and
/// its map nodes (type and tile position), spawned for the neutral player 0.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanetSurface {
    /// The cube's size.
    pub surface: CubeSurface,
    /// One terrain definition per tile.
    pub terrain: Vec<DefId>,
    /// Node types and positions.
    pub nodes: Vec<(DefId, SurfacePos)>,
}

/// Everything a world needs to know about its universe, as plain data.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UniverseSetup {
    /// Star systems.
    pub systems: Vec<SystemInfo>,
    /// Every body, including stars and resource bodies.
    pub bodies: Vec<BodyInfo>,
    /// Routes between systems.
    pub routes: Vec<RouteInfo>,
    /// The planets generated before the match, by body.
    pub planets: BTreeMap<BodyId, PlanetSurface>,
}

/// Stored universe facts, kept for later tasks (orbit, system space, travel).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct UniverseData {
    pub systems: Vec<SystemInfo>,
    /// Bodies in id order.
    pub bodies: Vec<BodyInfo>,
    pub routes: Vec<RouteInfo>,
}

impl World {
    /// Builds a world holding every planet in `setup.planets`, each with its own
    /// surface, and the universe's systems, bodies and routes. `homes` gives each player
    /// its home body, which must have a surface. Nodes are spawned for the neutral
    /// player 0 in body order; nothing is revealed (see `reveal_terrain`).
    pub fn from_universe(
        rules: Arc<Rules>,
        setup: UniverseSetup,
        seed: u64,
        players: Vec<Player>,
        homes: BTreeMap<PlayerId, BodyId>,
    ) -> Result<Self, SimError> {
        let UniverseSetup {
            systems,
            mut bodies,
            routes,
            planets,
        } = setup;
        bodies.sort_by_key(|b| b.id);
        if planets.is_empty() {
            return Err(SimError::InvalidUniverse("no planet surface"));
        }
        if bodies.windows(2).any(|w| w[0].id == w[1].id)
            || bodies
                .iter()
                .any(|b| !systems.iter().any(|s| s.id == b.system))
            || routes.iter().any(|r| {
                [r.system_a, r.system_b]
                    .iter()
                    .any(|id| !systems.iter().any(|s| s.id == *id))
            })
        {
            return Err(SimError::InvalidUniverse("unknown or repeated ids"));
        }
        let mut world = Self::empty(rules, seed, players)?;
        world.universe = UniverseData {
            systems,
            bodies,
            routes,
        };
        for (body, planet) in &planets {
            world.check_planet(*body, planet)?;
        }
        for (body, planet) in &planets {
            world.insert_surface(*body, planet.surface, planet.terrain.clone())?;
        }
        for (player, body) in &homes {
            if !world.players.contains_key(player) {
                return Err(SimError::UnknownPlayer(*player));
            }
            if world.slot_of(*body).is_none() {
                return Err(SimError::UnknownBody(*body));
            }
        }
        world.homes = homes;
        for (body, planet) in &planets {
            world.spawn_nodes_on(*body, &planet.nodes)?;
        }
        Ok(world)
    }

    /// Adds the surface of a habitable body generated during the match, with its nodes
    /// for the neutral player 0. Every player starts with nothing of it explored.
    pub fn add_surface(
        &mut self,
        body: BodyId,
        planet: &PlanetSurface,
    ) -> Result<Vec<EntityId>, SimError> {
        self.check_planet(body, planet)?;
        if !planet.nodes.is_empty() && !self.players.contains_key(&PlayerId(0)) {
            return Err(SimError::UnknownPlayer(PlayerId(0)));
        }
        for (type_id, at) in &planet.nodes {
            if self.rules.entity_type(*type_id).is_none() {
                return Err(SimError::UnknownType(*type_id));
            }
            let side = i64::from(planet.surface.n) * 1000;
            if at.face >= 6 || !(0..side).contains(&at.x) || !(0..side).contains(&at.y) {
                return Err(SimError::OutsideMap);
            }
        }
        self.insert_surface(body, planet.surface, planet.terrain.clone())?;
        self.spawn_nodes_on(body, &planet.nodes)
    }

    /// A planet may be given only for a habitable body without a surface, at its size.
    fn check_planet(&self, body: BodyId, planet: &PlanetSurface) -> Result<(), SimError> {
        let info = self.body(body).ok_or(SimError::UnknownBody(body))?;
        if info.kind != BodyKind::Habitable || self.slot_of(body).is_some() {
            return Err(SimError::UnknownBody(body));
        }
        if info.n.is_some_and(|n| n != planet.surface.n) {
            return Err(SimError::InvalidMap);
        }
        Ok(())
    }

    /// Creates a cube surface for `body` and gives every player its vision there.
    pub(crate) fn insert_surface(
        &mut self,
        body: BodyId,
        surface: CubeSurface,
        terrain: Vec<DefId>,
    ) -> Result<(), SimError> {
        if !(4..=4096).contains(&surface.n) {
            return Err(SimError::InvalidMap);
        }
        let map = MapGrid::new(surface.n, surface.n * 6, terrain)?;
        self.insert_map(body, map, Topology::Cube(surface))
    }

    /// Creates a surface of any topology for `body`, keeping surfaces in body order.
    pub(crate) fn insert_map(
        &mut self,
        body: BodyId,
        map: MapGrid,
        topology: Topology,
    ) -> Result<(), SimError> {
        if self.slot_of(body).is_some() {
            return Err(SimError::UnknownBody(body));
        }
        let surface = Surface::new(&self.rules, body, map, topology)?;
        let at = self.surfaces.partition_point(|s| s.body() < body);
        self.surfaces.insert(at, surface);
        let size = self
            .surfaces
            .iter()
            .map(|s| s.body().0 as usize + 1)
            .max()
            .unwrap_or(0);
        self.slots = vec![u32::MAX; size];
        for (slot, s) in self.surfaces.iter().enumerate() {
            self.slots[s.body().0 as usize] = slot as u32;
        }
        let ids: Vec<PlayerId> = self.players.keys().copied().collect();
        for id in ids {
            self.ensure_vision_on(at, id);
        }
        Ok(())
    }

    fn spawn_nodes_on(
        &mut self,
        body: BodyId,
        nodes: &[(DefId, SurfacePos)],
    ) -> Result<Vec<EntityId>, SimError> {
        nodes
            .iter()
            .map(|(type_id, at)| {
                let mut pos = Self::pos_of(*at)?;
                pos.body = body;
                self.spawn(*type_id, PlayerId(0), pos)
            })
            .collect()
    }

    /// The universe's bodies in id order; empty for a world built from one map.
    pub fn bodies(&self) -> &[BodyInfo] {
        &self.universe.bodies
    }

    /// One body of the universe.
    pub fn body(&self, id: BodyId) -> Option<&BodyInfo> {
        self.universe
            .bodies
            .binary_search_by_key(&id, |b| b.id)
            .ok()
            .map(|i| &self.universe.bodies[i])
    }

    /// The universe's star systems.
    pub fn systems(&self) -> &[SystemInfo] {
        &self.universe.systems
    }

    /// The universe's routes.
    pub fn routes(&self) -> &[RouteInfo] {
        &self.universe.routes
    }

    /// Every surface in body order.
    pub fn surfaces(&self) -> &[Surface] {
        &self.surfaces
    }

    /// The surface of a body, if it has been generated.
    pub fn surface(&self, body: BodyId) -> Option<&Surface> {
        self.slot_of(body).map(|slot| &self.surfaces[slot])
    }

    /// Habitable bodies whose surface has not been generated yet, in id order.
    pub fn ungenerated_bodies(&self) -> Vec<BodyId> {
        self.universe
            .bodies
            .iter()
            .filter(|b| b.kind == BodyKind::Habitable && self.slot_of(b.id).is_none())
            .map(|b| b.id)
            .collect()
    }

    /// Number of habitable bodies of the universe (surfaces for a world without one).
    pub fn habitable_count(&self) -> usize {
        if self.universe.bodies.is_empty() {
            return self.surfaces.len();
        }
        self.universe
            .bodies
            .iter()
            .filter(|b| b.kind == BodyKind::Habitable)
            .count()
    }

    /// A player's home body.
    pub fn home_body(&self, player: PlayerId) -> Option<BodyId> {
        self.homes.get(&player).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rom_rules::build_rules_db;

    fn world() -> World {
        let conn = build_rules_db(
            include_str!("../../../db/schema.sql"),
            include_str!("../../../db/seed.sql"),
        )
        .unwrap();
        let rules = Arc::new(Rules::load(&conn).unwrap());
        let plain = rules.id_of("ter.plain").unwrap();
        let body = |id: u32, kind: BodyKind| BodyInfo {
            id: BodyId(id),
            system: 1,
            kind,
            name: format!("B{id}"),
            orbit_radius: 1000,
            orbit_angle_mdeg: 5,
            n: (kind == BodyKind::Habitable).then_some(8),
            home_race: None,
            node_type: None,
        };
        let planet = PlanetSurface {
            surface: CubeSurface { n: 8 },
            terrain: vec![plain; 6 * 64],
            nodes: Vec::new(),
        };
        let setup = UniverseSetup {
            systems: vec![
                SystemInfo {
                    id: 1,
                    name: "A".into(),
                    x: 0,
                    y: 0,
                },
                SystemInfo {
                    id: 2,
                    name: "B".into(),
                    x: 9,
                    y: 9,
                },
            ],
            bodies: vec![
                body(1, BodyKind::Star),
                body(2, BodyKind::Habitable),
                body(3, BodyKind::Habitable),
            ],
            routes: vec![RouteInfo {
                system_a: 1,
                system_b: 2,
                length: 12,
            }],
            planets: BTreeMap::from([(BodyId(2), planet.clone()), (BodyId(3), planet)]),
        };
        let player = Player {
            id: PlayerId(1),
            team: 1,
            faction: rules.id_of("nation.romans").unwrap(),
            age: rules.id_of("age.ancient").unwrap(),
            resources: BTreeMap::new(),
            techs: BTreeMap::new(),
        };
        let homes = BTreeMap::from([(PlayerId(1), BodyId(2))]);
        World::from_universe(rules, setup, 7, vec![player], homes).unwrap()
    }

    fn changes(world: &mut World, update: impl FnOnce(&mut World)) {
        let before = world.hash();
        update(world);
        assert_ne!(world.hash(), before);
    }

    #[test]
    fn hash_covers_bodies_systems_routes_homes_and_every_surface() {
        let mut w = world();
        changes(&mut w, |w| w.universe.systems[1].x += 1);
        changes(&mut w, |w| w.universe.systems[0].name.push('x'));
        changes(&mut w, |w| w.universe.bodies[1].orbit_angle_mdeg += 1);
        changes(&mut w, |w| w.universe.bodies[2].system = 2);
        changes(&mut w, |w| w.universe.bodies[0].node_type = Some(DefId(1)));
        changes(&mut w, |w| w.universe.routes[0].length += 1);
        changes(&mut w, |w| {
            w.homes.insert(PlayerId(1), BodyId(3));
        });
        for slot in 0..2 {
            changes(&mut w, |w| w.surfaces[slot].map.terrain[3].0 += 1);
            changes(&mut w, |w| w.surfaces[slot].territory.winner[3] += 1);
            changes(&mut w, |w| w.surfaces[slot].pathing.occupancy[3] += 1);
            changes(&mut w, |w| {
                w.surfaces[slot]
                    .vision
                    .players
                    .get_mut(&PlayerId(1))
                    .unwrap()
                    .count[3] += 1
            });
            changes(&mut w, |w| {
                w.surfaces[slot]
                    .vision
                    .players
                    .get_mut(&PlayerId(1))
                    .unwrap()
                    .explored[0] ^= 2
            });
            changes(&mut w, |w| w.surfaces[slot].vision.touched.push(4));
            changes(&mut w, |w| {
                w.surfaces[slot]
                    .vision
                    .buildings
                    .entry(4)
                    .or_default()
                    .insert(EntityId(9));
            });
        }
    }

    #[test]
    fn universe_setup_is_checked() {
        let w = world();
        assert_eq!(w.surfaces().len(), 2);
        assert_eq!(w.habitable_count(), 2);
        assert!(w.ungenerated_bodies().is_empty());
        assert_eq!(w.body(BodyId(1)).unwrap().kind, BodyKind::Star);
        assert_eq!(w.systems().len(), 2);
        assert_eq!(w.routes().len(), 1);
        assert_eq!(w.surface(BodyId(3)).unwrap().tiles(), 6 * 64);
        assert!(w.surface(BodyId(1)).is_none());
    }
}
