//! Task 169: jetpacks. A jump needs the research and waits for its cooldown; it crosses a wall
//! but not a standing shield field; anti-air hits a unit in the air. Every scenario runs twice
//! and compares state hashes.

use crate::{Command, EntityId, MapGrid, Player, PlayerId, Pos, SimError, World};
use rom_rules::{DefId, Rules, build_rules_db};
use std::sync::Arc;

const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);
const EMITTER: &str = "bld.shield_emitter";
const WALL: &str = "bld.defensive_line";

/// A plain of 40 × 40 tiles in the Spacefaring age; player one has researched `techs`. Every
/// player starts with 5 000 of each resource.
fn world(techs: &[&str]) -> World {
    let db = build_rules_db(
        include_str!("../../../db/schema.sql"),
        include_str!("../../../db/seed.sql"),
    )
    .unwrap();
    let rules = Arc::new(Rules::load(&db).unwrap());
    let players = (0..=2)
        .map(|n| Player {
            id: PlayerId(n),
            team: n,
            faction: rules.id_of("nation.romans").unwrap(),
            age: rules.id_of("age.starfaring").unwrap(),
            resources: rules.resources.keys().map(|r| (*r, 5_000)).collect(),
            techs: if n == 1 {
                techs.iter().map(|k| (rules.id_of(k).unwrap(), 1)).collect()
            } else {
                Default::default()
            },
        })
        .collect();
    let ground = rules.id_of("ter.plain").unwrap();
    World::new(
        rules,
        MapGrid::new(40, 40, vec![ground; 40 * 40]).unwrap(),
        1,
        players,
    )
    .unwrap()
}
fn id(w: &World, key: &str) -> DefId {
    w.rules.id_of(key).unwrap()
}
fn at(x: i32, y: i32) -> Pos {
    Pos::flat(x * 1000 + 500, y * 1000 + 500)
}
fn spawn(w: &mut World, key: &str, owner: PlayerId, x: i32, y: i32) -> EntityId {
    w.place_for_setup(id(w, key), owner, at(x, y)).unwrap()
}
fn ticks(w: &mut World, n: u64) {
    for _ in 0..n {
        w.step();
    }
}
fn order(w: &World, executor: EntityId, kind: &str, to: Pos, queue: bool) -> Command {
    Command {
        player: w.entity(executor).unwrap().owner,
        executor,
        kind: kind.into(),
        subject: None,
        target_entity: None,
        target_pos: Some(to),
        queue,
        text: None,
        facing: None,
    }
}
fn jump(w: &mut World, unit: EntityId, to: Pos) -> Result<(), SimError> {
    let command = order(w, unit, "jump", to, false);
    w.submit(command).map(|_| ())
}
/// Lays `segment`'s line for `owner` along row `y` from column `x0` to `x1`, finished.
fn line(w: &mut World, owner: PlayerId, segment: &str, x0: i32, x1: i32, y: i32) {
    let sites = w
        .line_sites(owner, id(w, segment), &[at(x0, y), at(x1, y)], &[])
        .unwrap();
    for s in sites {
        w.place_turned_for_setup(s.subject, owner, s.pos, s.facing)
            .unwrap();
    }
}

/// Before the research a marine cannot jump; after it, it jumps four tiles, lands where it
/// was sent, and cannot jump again until its cooldown has run out.
fn jump_and_cool(techs: &[&str]) -> (u64, Vec<Result<(), SimError>>, Pos) {
    let mut w = world(techs);
    let marine = spawn(&mut w, "unit.marine", P1, 10, 10);
    let mut out = vec![jump(&mut w, marine, at(14, 10))];
    ticks(&mut w, 2);
    assert_eq!(w.leaping(marine), out[0].is_ok());
    // In the air, and once down while the jetpack cools.
    out.push(jump(&mut w, marine, at(18, 10)));
    ticks(&mut w, 40);
    assert!(!w.leaping(marine));
    out.push(jump(&mut w, marine, at(18, 10)));
    let landed = w.entity(marine).unwrap().pos;
    // The cooldown is 400 ticks from the take-off.
    ticks(&mut w, 360);
    out.push(jump(&mut w, marine, at(18, 10)));
    ticks(&mut w, 40);
    (w.hash(), out, landed)
}

#[test]
fn a_jump_needs_jetpacks_and_waits_for_its_cooldown() {
    let (_, before, _) = jump_and_cool(&[]);
    assert_eq!(before[0], Err(SimError::CannotJump));
    let (first, after, landed) = jump_and_cool(&["tech.jetpacks"]);
    assert_eq!(
        after,
        vec![
            Ok(()),
            Err(SimError::JumpCooling),
            Err(SimError::JumpCooling),
            Ok(())
        ]
    );
    assert_eq!(landed, at(14, 10), "it lands where it was sent");
    let (second, _, _) = jump_and_cool(&["tech.jetpacks"]);
    assert_eq!(first, second);
}

#[test]
fn a_jump_needs_free_land_and_a_unit_that_jumps() {
    let mut w = world(&["tech.jetpacks"]);
    let marine = spawn(&mut w, "unit.marine", P1, 10, 10);
    // Cavalry and wheeled infantry do not jump.
    let robot = spawn(&mut w, "unit.combat_robot", P1, 12, 12);
    assert_eq!(jump(&mut w, robot, at(14, 12)), Err(SimError::CannotJump));
    assert_eq!(w.jump_range(marine), 6_000);
    // A building's footprint, with nothing free within a tile and a half, is no landing.
    spawn(&mut w, "bld.barracks", P2, 24, 24);
    let barracks = w.jump_landing(w.entity(marine).unwrap().type_id, at(24, 24));
    assert!(barracks.is_none_or(|p| p != at(24, 24)), "{barracks:?}");
}

/// A marine of player one jumps across player two's wall, then is sent across a shield field
/// of player two: refused; a jump queued behind a walk that ends in front of the field fails
/// there. Optionally saved and restored mid-air.
fn wall_and_field(save_at: Option<u64>) -> (u64, World, EntityId, Result<(), SimError>) {
    let mut w = world(&["tech.jetpacks"]);
    line(&mut w, P2, WALL, 0, 39, 15);
    line(&mut w, P2, EMITTER, 0, 39, 25);
    let marine = spawn(&mut w, "unit.marine", P1, 12, 12);
    ticks(&mut w, 2);
    assert!(w.field_at(at(13, 25)), "the field stands");
    jump(&mut w, marine, at(12, 18)).unwrap();
    for tick in 0..30 {
        if save_at == Some(tick) {
            let copy = World::restore(w.rules().clone(), w.snapshot()).unwrap();
            assert_eq!(copy.hash(), w.hash());
            w = copy;
        }
        w.step();
    }
    assert_eq!(w.entity(marine).unwrap().pos, at(12, 18), "over the wall");
    let walk = order(&w, marine, "move", at(12, 22), false);
    w.submit(walk).unwrap();
    ticks(&mut w, 400);
    assert_eq!(w.entity(marine).unwrap().pos, at(12, 22));
    // The field blocks the air above its line, and above its emitters.
    let field = jump(&mut w, marine, at(13, 27));
    assert_eq!(
        jump(&mut w, marine, at(12, 27)),
        Err(SimError::FieldInTheWay)
    );
    // Queued behind a walk back, it walks into range again and fails at the field.
    let walk = order(&w, marine, "move", at(12, 19), false);
    w.submit(walk).unwrap();
    let queued = order(&w, marine, "jump", at(13, 27), true);
    w.submit(queued).unwrap();
    ticks(&mut w, 400);
    assert!(
        w.entity(marine).unwrap().pos.y < 25_000,
        "still north of the field"
    );
    assert!(
        w.order_failures(P1)
            .any(|f| f.kind == "jump" && f.reason == "field"),
        "the queued jump failed at the field"
    );
    (w.hash(), w, marine, field)
}

#[test]
fn a_jump_crosses_a_wall_but_not_a_shield_field() {
    let (first, _, _, field) = wall_and_field(None);
    assert_eq!(field, Err(SimError::FieldInTheWay));
    let (second, _, _, _) = wall_and_field(None);
    assert_eq!(first, second);
    let (restored, _, _, _) = wall_and_field(Some(10));
    assert_eq!(first, restored, "a save in mid-air changes nothing");
}

/// A marine jumps past an enemy flak gun, which can reach it only while it is in the air.
fn past_the_flak() -> (u64, bool, bool, i64, i64) {
    let mut w = world(&["tech.jetpacks"]);
    let marine = spawn(&mut w, "unit.marine", P1, 6, 20);
    let flak = spawn(&mut w, "unit.flak_gun", P2, 9, 24);
    let full = w.entity(marine).unwrap().hp;
    assert!(
        !w.can_target(flak, marine),
        "anti-air cannot hit it on the ground"
    );
    jump(&mut w, marine, at(12, 20)).unwrap();
    ticks(&mut w, 2);
    let aloft = w.can_target(flak, marine);
    assert_eq!(
        w.target_layer(marine),
        rom_rules::Layer::Air,
        "a jumper is in the air"
    );
    ticks(&mut w, 40);
    let down = w.can_target(flak, marine);
    // Walking on, it is out of the gun's reach on the ground.
    let walk = order(&w, marine, "move", at(30, 20), false);
    w.submit(walk).unwrap();
    ticks(&mut w, 200);
    (w.hash(), aloft, down, full, w.entity(marine).unwrap().hp)
}

#[test]
fn anti_air_hits_a_unit_in_flight() {
    let (first, aloft, down, full, hp) = past_the_flak();
    assert!(aloft && !down, "in reach of anti-air only in the air");
    assert!(hp < full, "the flak hit it in the air: {hp} of {full}");
    let (second, ..) = past_the_flak();
    assert_eq!(first, second);
}
