//! Task 165: a gate put into a standing line takes the place of the segments under it.

use crate::geometry::Tile;
use crate::{Command, EntityId, MapGrid, Player, PlayerId, Pos, SimError, World};
use rom_rules::{DefId, Rules, build_rules_db};
use std::sync::Arc;

const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);

/// A plain of `w` × `h` tiles; players 1 and 2 are enemies, each starting in `age` with 5 000 of
/// every resource. Buildings do not shoot; a town claims sixteen tiles round it.
fn world_sized(w: u32, h: u32, age: &str) -> World {
    let db = build_rules_db(
        include_str!("../../../db/schema.sql"),
        include_str!("../../../db/seed.sql"),
    )
    .unwrap();
    db.execute_batch(
        "UPDATE et_weapon SET shots=0 WHERE entity_type_id IN (SELECT id FROM entity_type WHERE class='building');
         UPDATE et_territory SET border_radius=16000, economic_radius=16000 WHERE is_city=1;",
    )
    .unwrap();
    let rules = Arc::new(Rules::load(&db).unwrap());
    let players = (0..=2)
        .map(|n| Player {
            id: PlayerId(n),
            team: n,
            faction: rules.id_of("nation.romans").unwrap(),
            age: rules.id_of(age).unwrap(),
            resources: rules.resources.keys().map(|r| (*r, 5_000)).collect(),
            techs: Default::default(),
        })
        .collect();
    let ground = rules.id_of("ter.plain").unwrap();
    World::new(
        rules,
        MapGrid::new(w, h, vec![ground; (w * h) as usize]).unwrap(),
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
fn tile(w: &World, unit: EntityId) -> (i32, i32) {
    let p = w.entity(unit).unwrap().pos;
    (p.x.div_euclid(1000), p.y.div_euclid(1000))
}
fn spawn(w: &mut World, key: &str, owner: PlayerId, x: i32, y: i32) -> EntityId {
    w.place_for_setup(id(w, key), owner, at(x, y)).unwrap()
}
fn ticks(w: &mut World, n: u64) {
    for _ in 0..n {
        w.step();
    }
}
fn order(
    w: &mut World,
    executor: EntityId,
    kind: &str,
    subject: Option<DefId>,
    target_pos: Option<Pos>,
    facing: Option<u16>,
) -> Result<(), SimError> {
    let player = w.entity(executor).unwrap().owner;
    w.submit(Command {
        player,
        executor,
        kind: kind.into(),
        subject,
        target_entity: None,
        target_pos,
        queue: false,
        text: None,
        facing,
    })
    .map(|_| ())
}
/// A finished line of `segment` for `owner` along `points`, as setup places it.
fn lay(w: &mut World, owner: PlayerId, segment: &str, points: &[Pos]) -> Vec<EntityId> {
    let sites = w.line_sites(owner, id(w, segment), points, &[]).unwrap();
    sites
        .iter()
        .map(|s| {
            w.place_turned_for_setup(s.subject, owner, s.pos, s.facing)
                .unwrap()
        })
        .collect()
}
/// The tiles of the standing entities of type `key`, finished or not, with their facings.
fn standing(w: &World, key: &str) -> Vec<((i32, i32), u16)> {
    let def = id(w, key);
    w.entities()
        .filter(|e| e.type_id == def)
        .map(|e| {
            (
                (e.pos.x.div_euclid(1000), e.pos.y.div_euclid(1000)),
                e.facing,
            )
        })
        .collect()
}
fn stock(w: &World, player: PlayerId) -> Vec<(DefId, i64)> {
    w.players[&player]
        .resources
        .iter()
        .map(|(r, n)| (*r, *n))
        .collect()
}
/// Runs until no site of type `key` is left, at most `limit` ticks.
fn finish(w: &mut World, key: &str, limit: u64) {
    let def = id(w, key);
    for _ in 0..limit {
        if !w
            .entities()
            .any(|e| e.type_id == def && e.build_progress.is_some())
        {
            return;
        }
        w.step();
    }
    panic!("{key} not finished in {limit} ticks");
}

#[test]
fn a_gate_snaps_onto_a_straight_wall_and_takes_exactly_its_three_segments() {
    let mut w = world_sized(40, 40, "age.classical");
    spawn(&mut w, "bld.city", P1, 20, 14);
    lay(&mut w, P1, "bld.stone_wall", &[at(10, 26), at(30, 26)]);
    let gate = id(&w, "bld.wall_gate");
    // Anywhere on a segment's tile: the gate centres on that tile, along the wall.
    let pointer = Pos::flat(20_150, 26_850);
    assert_eq!(w.gate_in_line(P1, gate, pointer), Some((at(20, 26), 0)));
    assert_eq!(
        w.gate_in_line(P1, gate, at(20, 27)),
        None,
        "beside the wall"
    );
    assert_eq!(
        w.gate_in_line(P2, gate, pointer),
        None,
        "not the enemy's wall"
    );
    assert_eq!(
        w.gate_in_line(P1, id(&w, "bld.palisade_gate"), pointer),
        None,
        "another line's gate"
    );
    assert_eq!(
        w.check_placement_facing(P1, gate, at(20, 26), 0),
        Ok(at(20, 26))
    );
    let builder = spawn(&mut w, "unit.citizen", P1, 20, 22);
    let before = stock(&w, P1);
    order(
        &mut w,
        builder,
        "construct",
        Some(gate),
        Some(at(20, 26)),
        Some(0),
    )
    .unwrap();
    let paid = stock(&w, P1);
    let price = w.price(P1, gate);
    for (resource, amount) in &price {
        let was = before.iter().find(|(r, _)| r == resource).unwrap().1;
        let now = paid.iter().find(|(r, _)| r == resource).unwrap().1;
        assert_eq!(was - now, *amount, "the gate's price only");
    }
    ticks(&mut w, 1);
    // The three segments under it are gone at once; no refund comes back for them.
    let walls = standing(&w, "bld.stone_wall");
    assert_eq!(walls.len(), 21 - 3);
    for x in 19..=21 {
        assert!(walls.iter().all(|(t, _)| *t != (x, 26)), "segment {x} gone");
    }
    assert!(walls.iter().any(|(t, _)| *t == (18, 26)));
    assert!(walls.iter().any(|(t, _)| *t == (22, 26)));
    let after = stock(&w, P1);
    for (resource, amount) in &price {
        let now = after.iter().find(|(r, _)| r == resource).unwrap().1;
        let then = paid.iter().find(|(r, _)| r == resource).unwrap().1;
        assert_eq!(now, then, "no refund of {resource:?} ({amount} paid)");
    }
    assert_eq!(standing(&w, "bld.wall_gate"), vec![((20, 26), 0)]);
    finish(&mut w, "bld.wall_gate", 3000);
    let built = w
        .entities()
        .find(|e| e.type_id == gate)
        .map(|e| (e.build_progress, e.facing));
    assert_eq!(built, Some((None, 0)));
}

#[test]
fn a_gate_goes_into_a_diagonal_wall_along_it() {
    let mut w = world_sized(48, 48, "age.classical");
    spawn(&mut w, "bld.city", P1, 36, 10);
    let line = lay(&mut w, P1, "bld.stone_wall", &[at(22, 10), at(34, 22)]);
    assert_eq!(line.len(), 13);
    let gate = id(&w, "bld.wall_gate");
    assert_eq!(w.gate_in_line(P1, gate, at(28, 16)), Some((at(28, 16), 45)));
    // Across the diagonal it would stand off the line.
    assert_eq!(
        w.check_placement_facing(P1, gate, at(28, 16), 0),
        Err(SimError::GateOffLine)
    );
    let builder = spawn(&mut w, "unit.citizen", P1, 30, 12);
    order(
        &mut w,
        builder,
        "construct",
        Some(gate),
        Some(at(28, 16)),
        Some(45),
    )
    .unwrap();
    ticks(&mut w, 1);
    let walls = standing(&w, "bld.stone_wall");
    assert_eq!(walls.len(), 13 - 3);
    for t in [(27, 15), (28, 16), (29, 17)] {
        assert!(walls.iter().all(|(w, _)| *w != t), "segment {t:?} gone");
    }
    assert_eq!(standing(&w, "bld.wall_gate"), vec![((28, 16), 45)]);
}

#[test]
fn a_gate_over_a_bend_or_a_foreign_wall_is_refused() {
    let mut w = world_sized(40, 40, "age.classical");
    spawn(&mut w, "bld.city", P1, 20, 21);
    // An L: east along row 26, then south down column 20.
    lay(
        &mut w,
        P1,
        "bld.stone_wall",
        &[at(10, 26), at(20, 26), at(20, 34)],
    );
    let gate = id(&w, "bld.wall_gate");
    let (corner, facing) = w.gate_in_line(P1, gate, at(20, 26)).unwrap();
    assert_eq!((corner, facing), (at(20, 26), 90));
    for f in [0, 90, 45] {
        assert_eq!(
            w.check_placement_facing(P1, gate, corner, f),
            Err(SimError::GateOffLine),
            "over the bend facing {f}"
        );
    }
    // Inside either leg it fits.
    assert!(w.check_placement_facing(P1, gate, at(15, 26), 0).is_ok());
    assert!(w.check_placement_facing(P1, gate, at(20, 30), 90).is_ok());
    // At the leg's end it would stand half off the wall.
    assert_eq!(
        w.check_placement_facing(P1, gate, at(10, 26), 0),
        Err(SimError::GateOffLine)
    );
    // The enemy's wall is no place for an own gate.
    lay(&mut w, P2, "bld.stone_wall", &[at(8, 20), at(14, 20)]);
    assert_eq!(w.gate_in_line(P1, gate, at(11, 20)), None);
    assert_eq!(
        w.check_placement_facing(P1, gate, at(11, 20), 0),
        Err(SimError::PlacementOverlap)
    );
    // Nor is an own diagonal wall with another building on the gate's square.
    lay(&mut w, P1, "bld.stone_wall", &[at(22, 30), at(30, 38)]);
    assert!(w.check_placement_facing(P1, gate, at(26, 34), 45).is_ok());
    spawn(&mut w, "bld.minefield", P1, 27, 34);
    let builder = spawn(&mut w, "unit.citizen", P1, 4, 4);
    let refused = order(
        &mut w,
        builder,
        "construct",
        Some(gate),
        Some(at(26, 34)),
        Some(45),
    );
    assert_eq!(refused, Err(SimError::PlacementOverlap));
}

#[test]
fn own_units_pass_the_new_gate_and_enemies_do_not() {
    // Priests fight no one: only the wall stands in their way.
    let mut w = world_sized(24, 40, "age.classical");
    spawn(&mut w, "bld.city", P1, 12, 10);
    lay(&mut w, P1, "bld.stone_wall", &[at(0, 20), at(23, 20)]);
    let gate = id(&w, "bld.wall_gate");
    let builders: Vec<EntityId> = (0..3)
        .map(|n| spawn(&mut w, "unit.citizen", P1, 10 + n, 16))
        .collect();
    order(
        &mut w,
        builders[0],
        "construct",
        Some(gate),
        Some(at(4, 20)),
        Some(0),
    )
    .unwrap();
    ticks(&mut w, 1);
    let site = w.entities().find(|e| e.type_id == gate).unwrap().id;
    for b in &builders[1..] {
        let player = P1;
        w.submit(Command {
            player,
            executor: *b,
            kind: "repair".into(),
            subject: None,
            target_entity: Some(site),
            target_pos: None,
            queue: false,
            text: None,
            facing: None,
        })
        .unwrap();
    }
    finish(&mut w, "bld.wall_gate", 4000);
    let own = spawn(&mut w, "unit.priest", P1, 12, 14);
    let enemy = spawn(&mut w, "unit.priest", P2, 20, 34);
    order(&mut w, own, "move", None, Some(at(12, 34)), None).unwrap();
    order(&mut w, enemy, "move", None, Some(at(20, 4)), None).unwrap();
    ticks(&mut w, 1600);
    assert!(
        tile(&w, own).1 > 20,
        "own unit through the gate: {:?}",
        tile(&w, own)
    );
    assert!(
        tile(&w, enemy).1 > 20,
        "the enemy stays out: {:?}",
        tile(&w, enemy)
    );
}

#[test]
fn a_turned_barracks_keeps_its_facing_and_turned_footprint() {
    let mut w = world_sized(40, 40, "age.classical");
    spawn(&mut w, "bld.city", P1, 20, 14);
    let barracks = id(&w, "bld.barracks");
    w.complete_research(P1, id(&w, "tech.military.1"));
    let builder = spawn(&mut w, "unit.citizen", P1, 12, 20);
    order(
        &mut w,
        builder,
        "construct",
        Some(barracks),
        Some(at(13, 22)),
        Some(50),
    )
    .unwrap();
    ticks(&mut w, 1);
    let site = w.entities().find(|e| e.type_id == barracks).unwrap();
    assert_eq!(site.facing, 45, "the order's facing, snapped");
    let et = &w.rules.entity_types[&barracks];
    let (across, down) = crate::turned_footprint(et.footprint_w, et.footprint_h, site.facing);
    let (x0, y0) = (
        i64::from(site.pos.x) - across * 500,
        i64::from(site.pos.y) - down * 500,
    );
    // Every tile of the turned footprint is taken; the ring round it is free.
    let palisade = id(&w, "bld.stone_wall");
    for ty in -1..=down {
        for tx in -1..=across {
            let x = (x0.div_euclid(1000) + tx) as i32;
            let y = (y0.div_euclid(1000) + ty) as i32;
            let inside = (0..across).contains(&tx) && (0..down).contains(&ty);
            let free = w.placement(P1, palisade, at(x, y), true).is_ok();
            assert_eq!(free, !inside, "tile {:?}", (x, y));
        }
    }
    // A gate is the building whose turn shows: a quarter turn stands it three tiles down.
    let gate = id(&w, "bld.wall_gate");
    order(
        &mut w,
        builder,
        "construct",
        Some(gate),
        Some(at(26, 22)),
        Some(90),
    )
    .unwrap();
    ticks(&mut w, 1);
    for y in 21..=23 {
        assert!(w.placement(P1, palisade, at(26, y), true).is_err());
    }
    assert!(w.placement(P1, palisade, at(25, 22), true).is_ok());
    assert!(w.placement(P1, palisade, at(27, 22), true).is_ok());
}

/// The tiles of row `y` from `x0` to `x1` that a unit of team 2 could step on.
fn open_tiles(w: &World, x0: i32, x1: i32, y: i32) -> Vec<i32> {
    let marine = id(w, "unit.marine");
    let class = w.rules.entity_types[&marine]
        .movement
        .as_ref()
        .unwrap()
        .movement_class_id;
    (x0..=x1)
        .filter(|&x| {
            w.surfaces[0]
                .pathing
                .passable_as(class, Some(2), Tile::new(0, x, y), None)
        })
        .collect()
}

#[test]
fn a_gate_put_into_a_shield_line_keeps_the_field_joined_on_both_sides() {
    let mut w = world_sized(40, 40, "age.starfaring");
    spawn(&mut w, "bld.city", P1, 16, 10);
    let emitters = lay(&mut w, P1, "bld.shield_emitter", &[at(4, 20), at(28, 20)]);
    assert_eq!(emitters.len(), 9, "an emitter every third tile");
    ticks(&mut w, 2);
    assert_eq!(
        open_tiles(&w, 4, 28, 20),
        Vec::<i32>::new(),
        "the field stands"
    );
    let gate = id(&w, "bld.shield_gate");
    // Over an emitter or over its field, the gate snaps along the line.
    assert_eq!(w.gate_in_line(P1, gate, at(16, 20)), Some((at(16, 20), 0)));
    assert_eq!(w.gate_in_line(P1, gate, at(17, 20)), Some((at(17, 20), 0)));
    assert_eq!(w.gate_in_line(P1, gate, at(17, 21)), None);
    assert!(w.check_placement_facing(P1, gate, at(17, 20), 0).is_ok());
    let builder = spawn(&mut w, "unit.citizen", P1, 16, 16);
    order(
        &mut w,
        builder,
        "construct",
        Some(gate),
        Some(at(16, 20)),
        Some(0),
    )
    .unwrap();
    // While the gate is built the line never opens: its site holds the field on both sides.
    let def = id(&w, "bld.shield_gate");
    let mut built = false;
    for tick in 0..4000 {
        w.step();
        assert_eq!(
            open_tiles(&w, 4, 28, 20),
            Vec::<i32>::new(),
            "tick {tick} of the gate's building"
        );
        if tick == 0 {
            assert_eq!(standing(&w, "bld.shield_emitter").len(), 8);
            assert!(w.field_at(at(14, 20)) && w.field_at(at(18, 20)));
        }
        if w.entities()
            .any(|e| e.type_id == def && e.build_progress.is_none())
        {
            built = true;
            break;
        }
    }
    assert!(built, "the gate is finished");
    ticks(&mut w, 2);
    assert_eq!(open_tiles(&w, 4, 28, 20), Vec::<i32>::new());
    assert!(w.field_at(at(14, 20)) && w.field_at(at(18, 20)));
    let gate_entity = w.entities().find(|e| e.type_id == def).unwrap().id;
    assert!(w.field_powered(gate_entity));
}

/// A walled town puts a gate into its wall while an enemy column marches on it; the builders'
/// own soldiers go out through it.
fn scenario(save_at: Option<u64>) -> u64 {
    let mut w = world_sized(48, 48, "age.classical");
    spawn(&mut w, "bld.city", P1, 24, 18);
    lay(
        &mut w,
        P1,
        "bld.stone_wall",
        &[at(8, 28), at(24, 28), at(36, 40)],
    );
    let gate = id(&w, "bld.wall_gate");
    let builders: Vec<EntityId> = (0..2)
        .map(|n| spawn(&mut w, "unit.citizen", P1, 20 + n, 25))
        .collect();
    order(
        &mut w,
        builders[0],
        "construct",
        Some(gate),
        Some(at(16, 28)),
        Some(0),
    )
    .unwrap();
    order(
        &mut w,
        builders[1],
        "construct",
        Some(gate),
        Some(at(27, 31)),
        Some(45),
    )
    .unwrap();
    let soldiers: Vec<EntityId> = (0..3)
        .map(|n| spawn(&mut w, "unit.spearman", P1, 18 + n, 24))
        .collect();
    let raiders: Vec<EntityId> = (0..4)
        .map(|n| spawn(&mut w, "unit.spearman", P2, 14 + n * 3, 44))
        .collect();
    for r in &raiders {
        order(&mut w, *r, "attack_move", None, Some(at(24, 18)), None).unwrap();
    }
    for tick in 0..2400 {
        if tick == 900 {
            for s in &soldiers {
                order(&mut w, *s, "attack_move", None, Some(at(16, 40)), None).unwrap();
            }
        }
        if save_at == Some(tick) {
            w = World::restore(w.rules().clone(), w.snapshot()).unwrap();
        }
        w.step();
    }
    assert_eq!(standing(&w, "bld.wall_gate").len(), 2);
    w.hash()
}

#[test]
fn putting_gates_into_a_wall_runs_alike_twice_and_across_a_save() {
    let first = scenario(None);
    assert_eq!(first, scenario(None));
    assert_eq!(first, scenario(Some(40)), "a save half way changes nothing");
}
