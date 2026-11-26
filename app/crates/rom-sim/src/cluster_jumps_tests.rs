//! Task 163: jumps between star clusters open with the galactic map-scale tier (rank 3). Before
//! it a ship jumps only between the systems of its own cluster; the paths keep to it and a route
//! plan into another cluster stops at the cluster's edge. Each scenario runs twice with equal
//! state hashes.

use crate::{
    BodyId, BodyInfo, Command, EntityId, PlanetSurface, Player, PlayerId, Pos, RouteInfo, SimError,
    SystemInfo, UniverseSetup, World,
};
use rom_geo::CubeSurface;
use rom_rules::{BodyKind, Rules, build_rules_db};
use std::{collections::BTreeMap, sync::Arc};

const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);

fn rules() -> Arc<Rules> {
    let db = build_rules_db(
        include_str!("../../../db/schema.sql"),
        include_str!("../../../db/seed.sql"),
    )
    .unwrap();
    Arc::new(Rules::load(&db).unwrap())
}

/// The first age of the rules at map-scale `rank` or beyond, found by rank.
fn age_of_rank(rules: &Rules, rank: i64) -> rom_rules::DefId {
    rules.first_age_of_rank(rank).unwrap().id
}

/// Five systems in two star clusters. Cluster 10 holds systems 1, 2 and 3, cluster 20 systems
/// 4 and 5. Routes: 1 – 2 (22) and 2 – 3 (100) inside cluster 10; 2 – 4 (10) and 4 – 3 (10)
/// between the clusters, a short cut from 2 to 3; 4 – 5 (22) inside cluster 20. Planet 2 of
/// system 1 is both players' home. Both players are at map-scale `rank`.
fn world(rank: i64) -> World {
    let rules = rules();
    let age = age_of_rank(&rules, rank);
    let star = |n: u32, system: u32| BodyInfo {
        id: BodyId(n),
        system,
        kind: BodyKind::Star,
        name: format!("Star {n}"),
        orbit_radius: 0,
        orbit_angle_mdeg: 0,
        n: None,
        home_race: None,
        node_type: None,
    };
    let system = |id: u32, x: i32, cluster: u32| SystemInfo {
        id,
        name: format!("System {id}"),
        x,
        y: 0,
        z: 0,
        constellation: String::new(),
        cluster: Some(cluster),
    };
    let route = |system_a, system_b, length| RouteInfo {
        system_a,
        system_b,
        length,
    };
    let plain = rules.id_of("ter.plain").unwrap();
    let setup = UniverseSetup {
        systems: vec![
            system(1, 0, 10),
            system(2, 100, 10),
            system(3, 200, 10),
            system(4, 300, 20),
            system(5, 400, 20),
        ],
        bodies: vec![
            star(1, 1),
            BodyInfo {
                id: BodyId(2),
                system: 1,
                kind: BodyKind::Habitable,
                name: "Home".into(),
                orbit_radius: 1500,
                orbit_angle_mdeg: 0,
                n: Some(24),
                home_race: None,
                node_type: None,
            },
            star(10, 2),
            star(20, 3),
            star(30, 4),
            star(40, 5),
        ],
        routes: vec![
            route(1, 2, 22),
            route(2, 3, 100),
            route(2, 4, 10),
            route(3, 4, 10),
            route(4, 5, 22),
        ],
        planets: BTreeMap::from([(
            BodyId(2),
            PlanetSurface {
                surface: CubeSurface { n: 24 },
                terrain: vec![plain; 6 * 24 * 24],
                nodes: Vec::new(),
            },
        )]),
        figure: Vec::new(),
        dormant: BTreeMap::new(),
    };
    let players = (1..=2)
        .map(|n| Player {
            id: PlayerId(n),
            team: n,
            faction: rules.id_of("nation.romans").unwrap(),
            age,
            resources: BTreeMap::new(),
            techs: BTreeMap::new(),
        })
        .collect();
    let homes = BTreeMap::from([(P1, BodyId(2)), (P2, BodyId(2))]);
    World::from_universe(rules, setup, 9, players, homes).unwrap()
}

fn star_pos(w: &World, system: u32) -> Pos {
    w.body_position(w.space_of(system).unwrap()).unwrap()
}

/// A corvette of `owner` in system `system`'s space, beside its star.
fn corvette(w: &mut World, owner: PlayerId, system: u32) -> EntityId {
    let at = star_pos(w, system);
    let at = Pos {
        x: at.x + 9000,
        ..at
    };
    w.spawn(w.rules.id_of("unit.corvette").unwrap(), owner, at)
        .unwrap()
}

fn travel(w: &mut World, ship: EntityId, system: u32, queue: bool) -> Result<(), SimError> {
    let to = star_pos(w, system);
    w.submit(Command {
        player: w.entity(ship).unwrap().owner,
        executor: ship,
        kind: "travel".into(),
        subject: None,
        target_entity: None,
        target_pos: Some(to),
        queue,
        text: None,
        facing: None,
    })
    .map(|_| ())
}

/// The system whose space the ship flies in, once it has arrived somewhere.
fn system_of(w: &World, ship: EntityId) -> Option<u32> {
    w.entity(ship).and_then(|e| w.system_space(e.pos.body))
}

/// Steps until the ship is in `system`'s space with no order left, at most `limit` ticks.
fn arrive(w: &mut World, ship: EntityId, system: u32, limit: u64) {
    for _ in 0..limit {
        if system_of(w, ship) == Some(system) && w.entity(ship).unwrap().orders.is_empty() {
            return;
        }
        w.step();
    }
    panic!("the ship did not reach system {system} within {limit} ticks");
}

/// The jump from cluster 10 to cluster 20 is refused at rank 2 with the typed error and its
/// plan stops at the cluster's edge; a jump within the cluster goes; once the player reaches
/// rank 3 (its age advances) the same jump goes. Returns the final state hash.
fn refused_then_allowed() -> u64 {
    let mut w = world(rom_rules::JUMP_RANK);
    let ship = corvette(&mut w, P1, 2);
    assert_eq!(w.space_rank(P1), rom_rules::JUMP_RANK);
    assert!(!w.jump_allowed(P1, 2, 4));
    assert!(w.jump_allowed(P1, 2, 3));
    assert_eq!(travel(&mut w, ship, 4, false), Err(SimError::ClusterClosed));
    assert_eq!(travel(&mut w, ship, 5, false), Err(SimError::ClusterClosed));
    // Queued behind another order, it is refused the same way.
    assert_eq!(travel(&mut w, ship, 5, true), Err(SimError::ClusterClosed));
    assert_eq!(w.route_plan(P1, 2, 5), Some(vec![2]));
    assert!(w.entity(ship).unwrap().orders.is_empty());
    // Within its cluster it jumps, the long way round rather than through cluster 20.
    assert_eq!(w.travel_path(P1, 2, 3), Some(vec![2, 3]));
    travel(&mut w, ship, 3, false).unwrap();
    arrive(&mut w, ship, 3, 40_000);
    // The age advances to the galactic tier: the clusters open.
    let galactic = age_of_rank(&w.rules, rom_rules::CLUSTER_JUMP_RANK);
    w.players.get_mut(&P1).unwrap().age = galactic;
    assert_eq!(w.space_rank(P1), rom_rules::CLUSTER_JUMP_RANK);
    assert!(w.jump_allowed(P1, 3, 4));
    assert_eq!(w.route_plan(P1, 3, 5), Some(vec![3, 4, 5]));
    travel(&mut w, ship, 5, false).unwrap();
    arrive(&mut w, ship, 5, 40_000);
    // Player two, still at rank 2, stays refused.
    let other = corvette(&mut w, P2, 3);
    assert_eq!(
        travel(&mut w, other, 4, false),
        Err(SimError::ClusterClosed)
    );
    w.hash()
}

#[test]
fn a_jump_between_clusters_is_refused_at_rank_2_and_allowed_at_rank_3() {
    assert_eq!(refused_then_allowed(), refused_then_allowed());
}

/// A jump within a cluster goes at rank 2, also over several systems; the path keeps to the
/// cluster even where a shorter one leads through another. Returns the final state hash.
fn within_cluster() -> u64 {
    let mut w = world(rom_rules::JUMP_RANK);
    let ship = corvette(&mut w, P1, 1);
    assert_eq!(w.travel_path(P1, 1, 3), Some(vec![1, 2, 3]));
    travel(&mut w, ship, 3, false).unwrap();
    arrive(&mut w, ship, 3, 60_000);
    w.hash()
}

#[test]
fn a_jump_within_a_cluster_is_allowed_at_rank_2() {
    assert_eq!(within_cluster(), within_cluster());
    // At rank 3 the shorter path through cluster 20 is taken.
    let w = world(rom_rules::CLUSTER_JUMP_RANK);
    assert_eq!(w.travel_path(P1, 1, 3), Some(vec![1, 2, 4, 3]));
}

#[test]
fn a_route_plan_stops_at_the_cluster_edge() {
    let w = world(rom_rules::JUMP_RANK);
    // The shortest way from 1 to 5 runs 1 – 2 – 4 – 5; 2 is the last system of cluster 10 on it.
    assert_eq!(w.travel_path(P1, 1, 5), None);
    assert_eq!(w.route_plan(P1, 1, 5), Some(vec![1, 2]));
    assert_eq!(w.route_plan(P1, 1, 4), Some(vec![1, 2]));
    // A plan that may go there is the travel path itself.
    assert_eq!(w.route_plan(P1, 1, 3), Some(vec![1, 2, 3]));
    let w = world(rom_rules::CLUSTER_JUMP_RANK);
    assert_eq!(w.route_plan(P1, 1, 5), Some(vec![1, 2, 4, 5]));
    // Before the interstellar tier no jump goes at all.
    let w = world(1);
    assert!(!w.jump_allowed(P1, 1, 2));
    assert_eq!(w.route_plan(P1, 1, 5), Some(vec![1]));
}

/// A system without a cluster (the celestial home, flat and test worlds) counts as being in
/// every cluster.
#[test]
fn a_system_without_a_cluster_is_in_every_cluster() {
    let mut w = world(rom_rules::JUMP_RANK);
    w.universe.systems[3].cluster = None;
    assert!(w.same_cluster(2, 4));
    assert!(w.same_cluster(4, 5));
    assert!(!w.same_cluster(1, 5));
    assert_eq!(w.travel_path(P1, 1, 5), Some(vec![1, 2, 4, 5]));
}
