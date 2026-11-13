use crate::{
    Command, EntityId, MapGrid, Player, PlayerId, Pos, SimError, SimRng, World,
    vision::{Sighting, bit},
};
use rom_rules::{DefId, Rules, build_rules_db};
use std::{collections::BTreeMap, sync::Arc};

const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);
const P3: PlayerId = PlayerId(3);

/// A plain square world for players 1 and 3 (one team) and 2.
fn world(sql: &str, side: u32) -> World {
    let db = build_rules_db(
        include_str!("../../../../db/schema.sql"),
        include_str!("../../../../db/seed.sql"),
    )
    .unwrap();
    db.execute_batch(sql).unwrap();
    let rules = Arc::new(Rules::load(&db).unwrap());
    let players = (1..=3)
        .map(|n| Player {
            id: PlayerId(n),
            team: if n == 3 { 1 } else { n },
            faction: rules.id_of("nation.romans").unwrap(),
            age: rules.id_of("age.ancient").unwrap(),
            resources: BTreeMap::new(),
            techs: BTreeMap::new(),
        })
        .collect();
    let plain = rules.id_of("ter.plain").unwrap();
    let map = MapGrid::new(side, side, vec![plain; (side * side) as usize]).unwrap();
    World::new(rules, map, 7, players).unwrap()
}

fn id(w: &World, key: &str) -> DefId {
    w.rules.id_of(key).unwrap()
}

fn at(x: i32, y: i32) -> Pos {
    Pos::flat(x, y)
}

fn spawn(w: &mut World, key: &str, owner: PlayerId, pos: Pos) -> EntityId {
    w.spawn(id(w, key), owner, pos).unwrap()
}

fn order(
    w: &World,
    executor: EntityId,
    kind: &str,
    target: Option<EntityId>,
    pos: Option<Pos>,
) -> Command {
    Command {
        player: w.entity(executor).unwrap().owner,
        executor,
        kind: kind.into(),
        subject: None,
        target_entity: target,
        target_pos: pos,
        queue: false,
    }
}

fn walk_to(w: &mut World, unit: EntityId, pos: Pos) {
    w.submit(order(w, unit, "move", None, Some(pos))).unwrap();
    for _ in 0..2000 {
        w.step();
        if w.entity(unit).unwrap().orders().is_empty() {
            return;
        }
    }
    panic!("unit did not arrive");
}

/// Viewer counts equal a full recount, and every visible tile is explored.
fn check(w: &World) {
    let full = w.full_counts();
    for (player, vision) in &w.vision.players {
        let expected = full
            .get(player)
            .cloned()
            .unwrap_or_else(|| vec![0; w.map.terrain.len()]);
        assert_eq!(vision.count, expected, "player {player:?} tick {}", w.tick);
        for index in 0..w.map.terrain.len() {
            if w.visible_index(*player, index) {
                assert!(bit(&vision.explored, index));
            }
        }
    }
}

#[test]
fn unit_reveals_circle_and_explored_persists_after_it_leaves() {
    let mut w = world("", 64);
    let unit = spawn(&mut w, "unit.citizen", P1, at(10_500, 10_500));
    assert_eq!(w.entity(unit).unwrap().los, 2000);
    for (x, y, seen) in [
        (10, 10, true),
        (12, 10, true),
        (10, 8, true),
        (11, 11, true),
        (12, 11, false),
        (13, 10, false),
    ] {
        assert_eq!(w.is_visible(P1, x, y), seen, "({x}, {y})");
        assert_eq!(w.is_explored(P1, x, y), seen, "({x}, {y})");
    }
    walk_to(&mut w, unit, at(40_500, 40_500));
    assert!(!w.is_visible(P1, 10, 10));
    assert!(w.is_explored(P1, 10, 10));
    assert!(w.is_visible(P1, 40, 40));
    assert!(!w.is_explored(P1, 60, 60));
    assert!(!w.is_explored(P2, 10, 10));
    check(&w);
}

#[test]
fn allies_share_visibility_enemies_do_not() {
    let mut w = world("", 64);
    spawn(&mut w, "unit.citizen", P1, at(10_500, 10_500));
    spawn(&mut w, "unit.citizen", P2, at(40_500, 40_500));
    assert!(w.is_visible(P3, 10, 10));
    assert!(w.is_explored(P3, 10, 10));
    assert!(!w.is_visible(P2, 10, 10));
    assert!(!w.is_explored(P2, 10, 10));
    assert!(!w.is_visible(P1, 40, 40));
    assert!(!w.is_visible(P3, 40, 40));
    let known = |player| {
        w.visible_entities(player)
            .iter()
            .filter_map(|s| match s {
                Sighting::Live(e) => Some(e.owner),
                Sighting::Remembered(_) => None,
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(known(P3), [P1]);
    assert_eq!(known(P2), [P2]);
}

fn fog_scenario() -> u64 {
    let mut w = world(
        "INSERT INTO modifier(source_def_id,target_tag_id,stat_key,op,value)
         SELECT d.id,t.id,'los','pct',50 FROM def d,tag t WHERE d.key='tech.religion' AND t.key='biological';",
        32,
    );
    let mut rng = SimRng::new(5);
    let kinds = ["unit.citizen", "unit.spearman", "unit.priest"];
    for n in 0..20 {
        let kind = kinds[n % kinds.len()];
        let pos = at(rng.range(0, 32_000) as i32, rng.range(0, 32_000) as i32);
        spawn(&mut w, kind, PlayerId(1 + (n % 3) as u32), pos);
    }
    check(&w);
    for tick in 0..500 {
        let live: Vec<EntityId> = w.entities().map(|e| e.id).collect();
        assert!(!live.is_empty());
        for _ in 0..3 {
            let unit = live[rng.range(0, live.len() as u32) as usize];
            let pos = at(rng.range(0, 32_000) as i32, rng.range(0, 32_000) as i32);
            w.submit(order(&w, unit, "move", None, Some(pos))).unwrap();
        }
        if tick % 50 == 25 {
            w.despawn(live[rng.range(0, live.len() as u32) as usize])
                .unwrap();
            let pos = at(rng.range(0, 32_000) as i32, rng.range(0, 32_000) as i32);
            spawn(&mut w, "unit.citizen", PlayerId(rng.range(1, 4)), pos);
        }
        if tick == 250 {
            w.complete_research(P1, id(&w, "tech.religion"));
        }
        w.step();
        check(&w);
    }
    assert!(w.entities().any(|e| e.owner == P1 && e.los == 3000));
    w.hash()
}

#[test]
fn incremental_visibility_equals_full_recompute() {
    // Run twice: the fog of war is part of the deterministic state.
    assert_eq!(fog_scenario(), fog_scenario());
}

#[test]
fn hidden_enemy_is_not_acquired_and_cannot_be_ordered_attacked() {
    let mut w = world("", 64);
    let spear = spawn(&mut w, "unit.spearman", P1, at(10_500, 10_500));
    // Eight tiles away: within the response range, beyond the 5-tile line of sight.
    let enemy = spawn(&mut w, "unit.citizen", P2, at(18_500, 10_500));
    for _ in 0..30 {
        w.step();
    }
    assert!(w.entity(spear).unwrap().orders().is_empty());
    let hash = w.hash();
    assert_eq!(
        w.submit(order(&w, spear, "attack", Some(enemy), None)),
        Err(SimError::TargetHidden)
    );
    assert_eq!(w.hash(), hash);
    // An ally's scout reveals the enemy: now it is both acquired and attackable.
    spawn(&mut w, "unit.citizen", P3, at(17_500, 10_500));
    w.submit(order(&w, spear, "attack", Some(enemy), None))
        .unwrap();
    w.submit(order(&w, spear, "stop", None, None)).unwrap();
    for _ in 0..20 {
        w.step();
    }
    assert_eq!(
        w.entity(spear).unwrap().orders()[0].target_entity,
        Some(enemy)
    );
}

#[test]
fn remembered_building_persists_in_fog_until_seen_again() {
    let mut w = world("", 64);
    let farm = spawn(&mut w, "bld.farm", P2, at(30_000, 10_000));
    let scout = spawn(&mut w, "unit.spearman", P1, at(26_500, 10_500));
    let memory = |w: &World| -> Vec<(EntityId, bool)> {
        w.visible_entities(P1)
            .iter()
            .filter_map(|s| match s {
                Sighting::Live(e) if e.id == farm => Some((e.id, true)),
                Sighting::Remembered(b) => Some((b.id, false)),
                _ => None,
            })
            .collect()
    };
    w.step();
    assert_eq!(memory(&w), [(farm, true)]);
    walk_to(&mut w, scout, at(5500, 10_500));
    assert_eq!(memory(&w), [(farm, false)]);
    let snapshot = *w.remembered_buildings(P1).next().unwrap();
    assert_eq!(
        (snapshot.type_id, snapshot.owner, snapshot.pos),
        (id(&w, "bld.farm"), P2, at(30_000, 10_000))
    );
    // Destroyed out of sight: still remembered.
    w.despawn(farm).unwrap();
    w.step();
    assert_eq!(memory(&w), [(farm, false)]);
    assert!(w.remembered_buildings(P2).next().is_none());
    walk_to(&mut w, scout, at(26_500, 10_500));
    assert!(memory(&w).is_empty());
    assert!(w.remembered_buildings(P1).next().is_none());
}

#[test]
fn reveal_terrain_explores_every_tile_but_shows_no_enemy() {
    // 37 × 37 tiles: the tile count is not a multiple of 64, so the last word is partial.
    let mut w = world("", 37);
    let foe = spawn(&mut w, "unit.spearman", P2, at(30_500, 30_500));
    w.reveal_terrain(P1);
    for y in 0..37 {
        for x in 0..37 {
            assert!(w.is_explored(P1, x, y));
            assert!(!w.is_visible(P1, x, y));
        }
    }
    assert!(!w.is_explored(P2, 0, 0));
    assert!(
        w.visible_entities(P1)
            .iter()
            .all(|s| !matches!(s, Sighting::Live(e) if e.id == foe))
    );
    let hash = w.hash();
    w.reveal_terrain(P1);
    assert_eq!(w.hash(), hash, "revealing twice changes nothing");
}
