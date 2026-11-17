use crate::{Command, EntityId, MapGrid, Player, PlayerId, Pos, Stain, World};
use rom_rules::{Rules, build_rules_db};
use std::{collections::BTreeMap, sync::Arc};

fn rules(sql: &str) -> Arc<Rules> {
    let db = build_rules_db(
        include_str!("../../../../db/schema.sql"),
        include_str!("../../../../db/seed.sql"),
    )
    .unwrap();
    db.execute_batch(sql).unwrap();
    Arc::new(Rules::load(&db).unwrap())
}

/// A flat plain; players 1 and 3 are allies, player 2 their enemy.
fn world(sql: &str) -> World {
    let rules = rules(sql);
    let players = (0..=3)
        .map(|id| Player {
            id: PlayerId(id),
            team: if id == 3 { 1 } else { id },
            faction: rules.id_of("nation.romans").unwrap(),
            age: rules.id_of("age.classical").unwrap(),
            resources: rules.resources.keys().map(|r| (*r, 100_000)).collect(),
            techs: BTreeMap::new(),
        })
        .collect();
    let terrain = rules.id_of("ter.plain").unwrap();
    World::new(
        rules,
        MapGrid::new(96, 96, vec![terrain; 96 * 96]).unwrap(),
        7,
        players,
    )
    .unwrap()
}

fn spawn(w: &mut World, key: &str, owner: u32, x: i32, y: i32) -> EntityId {
    w.spawn(
        w.rules.id_of(key).unwrap(),
        PlayerId(owner),
        Pos::flat(x, y),
    )
    .unwrap()
}

fn order(w: &mut World, id: EntityId, kind: &str, target: Option<EntityId>) {
    w.submit(Command {
        player: w.entity(id).unwrap().owner,
        executor: id,
        kind: kind.into(),
        subject: None,
        target_entity: target,
        target_pos: None,
        queue: false,
    })
    .unwrap();
}

fn stains(w: &World) -> Vec<Stain> {
    w.stains(crate::BodyId(0)).copied().collect()
}

fn run_until(w: &mut World, limit: u64, done: impl Fn(&World) -> bool) {
    for _ in 0..limit {
        if done(w) {
            return;
        }
        w.step();
    }
    assert!(done(w), "not reached in {limit} ticks");
}

/// A barracks' stain: 0.6 × the diagonal of its 4 × 4 footprint.
const BARRACKS: u32 = 5656 * 600 / 1000;
/// A soldier's: 2.5 × its 250 milli-tile collision radius.
const SOLDIER: u32 = 625;

/// A building and a soldier struck down by enemies each leave one stain where they stood, sized
/// by their stature; the attacker that survives leaves none.
#[test]
fn destroyed_buildings_and_units_leave_stains_by_stature() {
    let play = || {
        let mut w = world("");
        let barracks = spawn(&mut w, "bld.barracks", 2, 20_000, 20_000);
        let soldier = spawn(&mut w, "unit.hoplite", 2, 40_000, 20_000);
        let a = spawn(&mut w, "unit.hoplite", 1, 23_000, 20_000);
        let b = spawn(&mut w, "unit.hoplite", 1, 41_000, 20_000);
        for id in [barracks, soldier] {
            w.entities[id.0 as usize].as_mut().unwrap().hp = 1;
        }
        let at_barracks = w.entity(barracks).unwrap().pos;
        order(&mut w, a, "attack", Some(barracks));
        order(&mut w, b, "attack", Some(soldier));
        // The soldier may step aside as it fights: its stain is where it fell, at most a step
        // from where it was last seen.
        let mut at_soldier = w.entity(soldier).unwrap().pos;
        for _ in 0..600 {
            if let Some(e) = w.entity(soldier) {
                at_soldier = e.pos;
            }
            if w.entity(barracks).is_none() && w.entity(soldier).is_none() {
                break;
            }
            w.step();
        }
        let left = stains(&w);
        assert_eq!(left.len(), 2, "{left:?}");
        let of = |id| *left.iter().find(|s| s.id == id).unwrap();
        let (b, s) = (of(barracks), of(soldier));
        assert_eq!(
            (b.pos, b.radius, b.owner),
            (at_barracks, BARRACKS, PlayerId(2))
        );
        assert_eq!(s.radius, SOLDIER);
        assert!(s.pos.x.abs_diff(at_soldier.x) + s.pos.y.abs_diff(at_soldier.y) <= 500);
        assert_eq!(b.type_id, w.rules.id_of("bld.barracks").unwrap());
        assert!(left.windows(2).all(|p| p[0].tick <= p[1].tick));
        w.hash()
    };
    assert_eq!(play(), play());
}

/// A site half built and struck down leaves a stain half as wide as a finished building's.
#[test]
fn a_half_built_site_leaves_a_smaller_stain() {
    let mut w = world("");
    let kind = w.rules.id_of("bld.barracks").unwrap();
    let site = w
        .spawn_entity(kind, PlayerId(1), Pos::flat(20_000, 20_000), true)
        .unwrap();
    let total = w
        .rules
        .build_ticks(kind, &w.players[&PlayerId(1)].active_sources());
    let e = w.entities[site.0 as usize].as_mut().unwrap();
    e.build_progress = Some((total / 2) as u32);
    e.hp = 0;
    w.step();
    let left = stains(&w);
    assert_eq!(left.len(), 1);
    assert!(left[0].radius < BARRACKS, "{}", left[0].radius);
    assert!(
        left[0].radius.abs_diff(BARRACKS / 2) <= 2,
        "{}",
        left[0].radius
    );
}

/// A building its owner pulls down and a tower upgraded in place leave no stain.
#[test]
fn pulled_down_and_upgraded_buildings_leave_none() {
    let mut w = world("");
    let farm = spawn(&mut w, "bld.farm", 1, 20_000, 20_000);
    let tower = spawn(&mut w, "bld.tower", 1, 40_000, 40_000);
    order(&mut w, farm, "raze", None);
    w.step();
    assert!(w.entity(farm).is_none());
    let bastion = w.rules.id_of("bld.bastion").unwrap();
    w.complete_research(PlayerId(1), w.rules.id_of("tech.up.bastion").unwrap());
    w.step();
    assert_eq!(w.entity(tower).unwrap().type_id, bastion);
    assert!(stains(&w).is_empty());
    assert!(w.razed.is_empty());
}

/// Stains last `world.stain_days × world.day_ticks` ticks; past `world.stain_cap` the oldest
/// goes first.
#[test]
fn stains_fade_after_their_days_and_the_cap_drops_the_oldest() {
    let mut w = world(
        "UPDATE game_constant SET value=10 WHERE key='world.day_ticks';
         UPDATE game_constant SET value=3 WHERE key='world.stain_days';
         UPDATE game_constant SET value=3 WHERE key='world.stain_cap';",
    );
    assert_eq!(w.stain_life(), 30);
    let mut dead = Vec::new();
    for n in 0..4 {
        let id = spawn(&mut w, "unit.hoplite", 1, 10_000 + n * 3000, 10_000);
        w.entities[id.0 as usize].as_mut().unwrap().hp = 0;
        w.step();
        dead.push(id);
    }
    // The first one went when the fourth came.
    let ids: Vec<_> = stains(&w).iter().map(|s| s.id).collect();
    assert_eq!(ids, dead[1..]);
    // Left at tick `last`, the last one is seen for 30 ticks after it.
    let last = stains(&w)[2].tick;
    while w.tick() < last + 30 {
        w.step();
    }
    assert_eq!(stains(&w).len(), 1);
    w.step();
    assert!(stains(&w).is_empty());
}

/// A unit aboard a carrier and a ship in orbit leave no stain; without days or room nothing is
/// kept.
#[test]
fn nothing_aboard_and_no_days_leave_no_stain() {
    let mut w = world("UPDATE game_constant SET value=0 WHERE key='world.stain_days';");
    let soldier = spawn(&mut w, "unit.hoplite", 1, 10_000, 10_000);
    assert_eq!(w.stain_radius(soldier), Some(SOLDIER));
    w.entities[soldier.0 as usize].as_mut().unwrap().hp = 0;
    w.step();
    assert!(stains(&w).is_empty());
    let mut w = world("");
    let soldier = spawn(&mut w, "unit.hoplite", 1, 10_000, 10_000);
    let carrier = spawn(&mut w, "unit.hoplite", 1, 12_000, 10_000);
    w.entities[soldier.0 as usize]
        .as_mut()
        .unwrap()
        .container_id = Some(carrier);
    assert_eq!(w.stain_radius(soldier), None);
}

/// The same battle twice leaves the same stains and hashes.
#[test]
fn a_battle_leaves_the_same_stains_twice() {
    let play = || {
        let mut w = World::battle_scenario(rules(""), 3, 24).unwrap();
        run_until(&mut w, 2000, |w| w.stains(crate::BodyId(0)).count() >= 10);
        let hash = w.hash();
        for _ in 0..50 {
            w.step();
        }
        (stains(&w), hash, w.hash())
    };
    let first = play();
    assert!(first.0.len() >= 10);
    assert_eq!(first, play());
}
