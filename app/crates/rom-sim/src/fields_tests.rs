//! Task 156: shield emitters, their energy field and the shield gate.

use crate::{Command, EntityId, MapGrid, Player, PlayerId, Pos, SimError, World};
use rom_rules::{DefId, Rules, build_rules_db};
use std::sync::Arc;

const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);
/// Player 1's ally.
const P3: PlayerId = PlayerId(3);
const EMITTER: &str = "bld.shield_emitter";
const GATE: &str = "bld.shield_gate";

/// A plain of `w` × `h` tiles. Players 1 and 3 are one team, player 2 another; every player
/// starts in `age` with 5 000 of each resource. Buildings do not shoot.
fn world_sized(w: u32, h: u32, age: &str) -> World {
    let db = build_rules_db(
        include_str!("../../../db/schema.sql"),
        include_str!("../../../db/seed.sql"),
    )
    .unwrap();
    db.execute_batch(
        "UPDATE et_weapon SET shots=0 WHERE entity_type_id IN (SELECT id FROM entity_type WHERE class='building');",
    )
    .unwrap();
    let rules = Arc::new(Rules::load(&db).unwrap());
    let players = (0..=3)
        .map(|n| Player {
            id: PlayerId(n),
            team: if n == 3 { 1 } else { n },
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
fn walk(w: &mut World, executor: EntityId, x: i32, y: i32) {
    let player = w.entity(executor).unwrap().owner;
    w.submit(Command {
        player,
        executor,
        kind: "move".into(),
        subject: None,
        target_entity: None,
        target_pos: Some(at(x, y)),
        queue: false,
        text: None,
        facing: None,
    })
    .unwrap();
}
fn energy(w: &World, player: PlayerId) -> i64 {
    let energy = id(w, "res.energy");
    w.players[&player]
        .resources
        .get(&energy)
        .copied()
        .unwrap_or(0)
}
fn set_energy(w: &mut World, player: PlayerId, amount: i64) {
    let energy = id(w, "res.energy");
    w.players
        .get_mut(&player)
        .unwrap()
        .resources
        .insert(energy, amount);
}
/// Lays `segment`'s line across the map at row `y` (every tile from column 0 to `width` − 1),
/// with its gate centred at column `gate_x`, as the interface would: one building a site.
fn line_across(w: &mut World, segment: &str, width: i32, y: i32, gate_x: i32) -> Vec<EntityId> {
    let sites = w
        .line_sites(
            P1,
            id(w, segment),
            &[at(0, y), at(width - 1, y)],
            &[at(gate_x, y)],
        )
        .unwrap();
    sites
        .iter()
        .map(|s| {
            w.place_turned_for_setup(s.subject, P1, s.pos, s.facing)
                .unwrap()
        })
        .collect()
}
fn columns(w: &World, key: &str) -> Vec<i32> {
    let type_id = id(w, key);
    let mut out: Vec<i32> = w
        .entities()
        .filter(|e| e.type_id == type_id)
        .map(|e| e.pos.x.div_euclid(1000))
        .collect();
    out.sort_unstable();
    out
}

#[test]
fn a_line_of_emitters_keeps_one_every_third_tile_where_it_runs_straight() {
    let w = world_sized(40, 40, "age.starfaring");
    let (emitter, gate) = (id(&w, EMITTER), id(&w, GATE));
    // Straight across, a gate at column 12: an emitter every third tile, the stretch's ends
    // kept, each facing along the line.
    let sites = w
        .line_sites(P1, emitter, &[at(0, 20), at(23, 20)], &[at(12, 20)])
        .unwrap();
    let laid: Vec<(i32, bool, u16)> = sites
        .iter()
        .map(|s| (s.pos.x / 1000, s.subject == gate, s.facing))
        .collect();
    assert_eq!(
        laid,
        vec![
            (0, false, 0),
            (3, false, 0),
            (6, false, 0),
            (9, false, 0),
            (10, false, 0),
            (12, true, 0),
            (14, false, 0),
            (17, false, 0),
            (20, false, 0),
            (23, false, 0),
        ]
    );
    // A diagonal keeps every third tile too, facing along it.
    let sites = w
        .line_sites(P1, emitter, &[at(2, 2), at(11, 11)], &[])
        .unwrap();
    let laid: Vec<(i32, u16)> = sites.iter().map(|s| (s.pos.x / 1000, s.facing)).collect();
    assert_eq!(laid, vec![(2, 45), (5, 45), (8, 45), (11, 45)]);
    // A line that turns keeps the corner.
    let sites = w
        .line_sites(P1, emitter, &[at(2, 30), at(6, 30), at(6, 35)], &[])
        .unwrap();
    let laid: Vec<(i32, i32, u16)> = sites
        .iter()
        .map(|s| (s.pos.x / 1000, s.pos.y / 1000, s.facing))
        .collect();
    assert_eq!(
        laid,
        vec![
            (2, 30, 0),
            (5, 30, 0),
            (6, 30, 90),
            (6, 33, 90),
            (6, 35, 90)
        ]
    );
    // Other fortifications keep a segment on every tile.
    let wall = w
        .line_sites(
            P1,
            id(&w, "bld.defensive_line"),
            &[at(0, 20), at(9, 20)],
            &[],
        )
        .unwrap();
    assert_eq!(wall.len(), 10);
}

/// A shield line across the map with its gate; an own unit, an ally and an enemy walk across.
/// The field stands once its emitters are paid for. Optionally saved and restored on the way.
fn field_line(save_at: Option<u64>) -> (u64, World, [EntityId; 3]) {
    let mut w = world_sized(24, 40, "age.starfaring");
    line_across(&mut w, EMITTER, 24, 20, 12);
    let own = spawn(&mut w, "unit.priest", P1, 12, 8);
    let ally = spawn(&mut w, "unit.priest", P3, 14, 8);
    let enemy = spawn(&mut w, "unit.priest", P2, 4, 8);
    walk(&mut w, own, 12, 34);
    walk(&mut w, ally, 14, 34);
    walk(&mut w, enemy, 4, 34);
    for tick in 0..1600 {
        if save_at == Some(tick) {
            let copy = World::restore(w.rules().clone(), w.snapshot()).unwrap();
            assert_eq!(copy.hash(), w.hash());
            assert_eq!(copy.field_links(), w.field_links());
            w = copy;
        }
        w.step();
    }
    (w.hash(), w, [own, ally, enemy])
}

#[test]
fn a_field_blocks_enemies_and_the_shield_gate_lets_its_team_through() {
    let (first, w, [own, ally, enemy]) = field_line(None);
    assert_eq!(columns(&w, EMITTER), vec![0, 3, 6, 9, 10, 14, 17, 20, 23]);
    // Every stretch stands, the gaps between emitters are field, the emitters paid.
    let links = w.field_links();
    assert_eq!(links.len(), 9, "{links:?}");
    assert!(links.iter().all(|l| l.up && l.owner == P1));
    assert_eq!(links.iter().map(|l| u32::from(l.tiles)).sum::<u32>(), 12);
    for x in [1, 2, 4, 5, 7, 8, 15, 16, 18, 19, 21, 22] {
        assert!(w.field_at(at(x, 20)), "field at column {x}");
    }
    assert!(!w.field_at(at(12, 20)) && !w.field_at(at(4, 21)));
    assert!(
        tile(&w, own).1 > 20,
        "own unit through the gate: {:?}",
        tile(&w, own)
    );
    assert!(
        tile(&w, ally).1 > 20,
        "ally through the gate: {:?}",
        tile(&w, ally)
    );
    assert!(
        tile(&w, enemy).1 < 20,
        "the field stops the enemy: {:?}",
        tile(&w, enemy)
    );
    // Upkeep: nine emitters at 2 and the gate at 6, three periods paid (ticks 1, 601, 1201).
    assert_eq!(energy(&w, P1), 5_000 - 3 * (9 * 2 + 6));
    let (second, _, _) = field_line(None);
    assert_eq!(first, second);
    let (restored, _, _) = field_line(Some(700));
    assert_eq!(first, restored, "a save half way changes nothing");
}

#[test]
fn the_field_drops_without_energy_and_only_the_emitters_block() {
    let run = || {
        let mut w = world_sized(24, 40, "age.starfaring");
        set_energy(&mut w, P1, 0);
        line_across(&mut w, EMITTER, 24, 20, 12);
        ticks(&mut w, 1);
        let links = w.field_links().to_vec();
        assert_eq!(links.len(), 9);
        assert!(links.iter().all(|l| !l.up), "no energy, no field");
        assert!(!w.field_at(at(4, 20)));
        let enemy = spawn(&mut w, "unit.priest", P2, 4, 8);
        walk(&mut w, enemy, 4, 34);
        ticks(&mut w, 1200);
        assert!(
            tile(&w, enemy).1 > 20,
            "between the emitters: {:?}",
            tile(&w, enemy)
        );
        // Energy for one period: every emitter pays and the field stands, then drops when the
        // period is over and nothing is left to pay with.
        set_energy(&mut w, P1, 9 * 2 + 6);
        ticks(&mut w, 1);
        assert_eq!(energy(&w, P1), 0);
        assert!(w.field_links().iter().all(|l| l.up));
        assert!(w.field_at(at(4, 20)));
        ticks(&mut w, 599);
        assert!(w.field_at(at(4, 20)), "paid for a period");
        ticks(&mut w, 1);
        assert!(!w.field_at(at(4, 20)), "the period is over");
        assert!(w.field_links().iter().all(|l| !l.up));
        // A store short of the whole upkeep powers the emitters in id order: a stretch stands
        // only when both its emitters are paid.
        set_energy(&mut w, P1, 2 * 2);
        ticks(&mut w, 1);
        let up: Vec<bool> = w.field_links().iter().map(|l| l.up).collect();
        assert_eq!(up.iter().filter(|u| **u).count(), 1, "{up:?}");
        w.hash()
    };
    assert_eq!(run(), run());
}

#[test]
fn defensive_lines_become_emitters_at_the_spacefaring_advance() {
    let run = || {
        let mut w = world_sized(24, 40, "age.fusion");
        let built = line_across(&mut w, "bld.defensive_line", 24, 20, 12);
        assert_eq!(built.len(), 22, "21 segments and the checkpoint");
        let facings: Vec<u16> = built.iter().map(|e| w.entity(*e).unwrap().facing).collect();
        let half = w.entity(built[0]).unwrap().max_hp / 2;
        w.entities[built[0].0 as usize].as_mut().unwrap().hp = half;
        ticks(&mut w, 1);
        assert!(w.field_links().is_empty(), "no emitters before the advance");
        w.complete_research(P1, id(&w, "tech.age.starfaring"));
        let emitter = id(&w, EMITTER);
        for (e, facing) in built.iter().zip(&facings) {
            let e = w.entity(*e).unwrap();
            assert!(
                [emitter, id(&w, GATE)].contains(&e.type_id),
                "{}",
                w.rules.key(e.type_id)
            );
            assert_eq!(e.facing, *facing, "the line keeps its facing");
        }
        let first = w.entity(built[0]).unwrap();
        assert_eq!(
            (first.hp, first.max_hp),
            (2_000, 4_000),
            "health keeps its share"
        );
        // The defensive line is out of the build menu.
        let citizen = spawn(&mut w, "unit.citizen", P1, 4, 30);
        let line = id(&w, "bld.defensive_line");
        assert_eq!(
            w.submit(Command {
                player: P1,
                executor: citizen,
                kind: "construct".into(),
                subject: Some(line),
                target_entity: None,
                target_pos: Some(at(4, 32)),
                queue: false,
                text: None,
                facing: None,
            })
            .map(|_| ()),
            Err(SimError::Superseded)
        );
        // Neighbours join: a stretch between each two, no tile left between them.
        ticks(&mut w, 1);
        let links = w.field_links();
        assert_eq!(links.len(), 21, "{links:?}");
        assert!(links.iter().all(|l| l.up && l.tiles == 0));
        // An enemy still finds the line shut; the owner's team passes the shield gate.
        let enemy = spawn(&mut w, "unit.priest", P2, 4, 8);
        let own = spawn(&mut w, "unit.priest", P1, 12, 8);
        walk(&mut w, enemy, 4, 34);
        walk(&mut w, own, 12, 34);
        ticks(&mut w, 1200);
        assert!(tile(&w, enemy).1 < 20 && tile(&w, own).1 > 20);
        w.hash()
    };
    assert_eq!(run(), run());
}

#[test]
fn artillery_breaks_an_emitter_and_the_field_beside_it_drops() {
    let run = || {
        let mut w = world_sized(40, 40, "age.starfaring");
        line_across(&mut w, EMITTER, 40, 20, 30);
        ticks(&mut w, 1);
        let target = w
            .entities()
            .find(|e| e.type_id == id(&w, EMITTER) && e.pos.x / 1000 == 6)
            .unwrap()
            .id;
        assert!(w.field_at(at(5, 20)) && w.field_at(at(7, 20)));
        let gun = spawn(&mut w, "unit.railgun_battery", P2, 6, 24);
        ticks(&mut w, 1);
        w.submit(Command {
            player: P2,
            executor: gun,
            kind: "attack".into(),
            subject: None,
            target_entity: Some(target),
            target_pos: None,
            queue: false,
            text: None,
            facing: None,
        })
        .unwrap();
        for _ in 0..4000 {
            w.step();
            if w.entity(target).is_none() {
                break;
            }
        }
        assert!(w.entity(target).is_none(), "the emitter falls");
        // The stretches either side of it are gone; the rest still stand.
        assert!(!w.field_at(at(5, 20)) && !w.field_at(at(7, 20)));
        assert!(w.field_at(at(1, 20)) && w.field_at(at(10, 20)));
        w.hash()
    };
    assert_eq!(run(), run());
}

#[test]
fn a_field_takes_its_team_from_its_emitters_and_never_joins_another_owner() {
    let mut w = world_sized(24, 24, "age.starfaring");
    let emitter = id(&w, EMITTER);
    // Two emitters of player 1 three tiles apart, facing each other's way, and one of player 2
    // in between their next pair.
    w.place_turned_for_setup(emitter, P1, at(2, 10), 0).unwrap();
    w.place_turned_for_setup(emitter, P1, at(5, 10), 0).unwrap();
    w.place_turned_for_setup(emitter, P1, at(10, 10), 0)
        .unwrap();
    w.place_turned_for_setup(emitter, P2, at(12, 10), 0)
        .unwrap();
    w.place_turned_for_setup(emitter, P1, at(13, 10), 0)
        .unwrap();
    ticks(&mut w, 1);
    let links: Vec<(i32, i32, u8)> = w
        .field_links()
        .iter()
        .map(|l| (l.start.x / 1000, l.end.x / 1000, l.tiles))
        .collect();
    // 2 → 5 over two tiles; 5 reaches nothing within its gap; 10 is cut off by player 2's.
    assert_eq!(links, vec![(2, 5, 2)]);
    assert!(w.field_at(at(3, 10)) && w.field_at(at(4, 10)));
    assert!(!w.field_at(at(11, 10)));
}
