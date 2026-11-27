//! Sea transports (task 167): land units board a ship from the shore beside it, sail across
//! water they cannot walk over and step ashore on another coast; a full ship refuses more, ships
//! and aircraft never board, and the units aboard are lost with a sunk ship. Every scenario runs
//! twice and compares state hashes.

use std::sync::Arc;

use rom_rules::{DefId, Layer, Rules, build_rules_db};
use rom_sim::{AlertKind, Command, EntityId, MapGrid, Player, PlayerId, Pos, SimError, World};

const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);
/// Columns of water between the two shores: land west of `WEST_SHORE`, water up to `EAST_SHORE`.
const WEST_SHORE: i32 = 16;
const EAST_SHORE: i32 = 40;

fn rules() -> Arc<Rules> {
    let conn = build_rules_db(
        include_str!("../../../db/schema.sql"),
        include_str!("../../../db/seed.sql"),
    )
    .unwrap();
    Arc::new(Rules::load(&conn).unwrap())
}

fn player(rules: &Rules, n: u32) -> Player {
    Player {
        id: PlayerId(n),
        team: n,
        faction: rules.id_of("nation.romans").unwrap(),
        age: rules.id_of("age.medieval").unwrap(),
        resources: rules.resources.keys().map(|r| (*r, 1_000_000)).collect(),
        techs: Default::default(),
    }
}

fn id(w: &World, key: &str) -> DefId {
    w.rules().id_of(key).unwrap()
}

fn command(player: PlayerId, executor: EntityId, kind: &str) -> Command {
    Command {
        player,
        executor,
        kind: kind.into(),
        subject: None,
        target_entity: None,
        target_pos: None,
        queue: false,
        text: None,
        facing: None,
    }
}

/// A 64 x 64 flat map: plain west of column 16 and east of column 40, shallow water between,
/// so nothing on foot or wheels crosses from one shore to the other.
fn world() -> World {
    let rules = rules();
    let (plain, coast) = (
        rules.id_of("ter.plain").unwrap(),
        rules.id_of("ter.coast").unwrap(),
    );
    let terrain = (0..64 * 64)
        .map(|i| {
            let x = i % 64;
            if (WEST_SHORE..EAST_SHORE).contains(&x) {
                coast
            } else {
                plain
            }
        })
        .collect();
    let map = MapGrid::new(64, 64, terrain).unwrap();
    let players = vec![player(&rules, 1), player(&rules, 2)];
    World::new(rules, map, 11, players).unwrap()
}

fn at(x: i32, y: i32) -> Pos {
    Pos {
        face: 0,
        ..Pos::flat(x, y)
    }
}

fn is_land(w: &World, pos: Pos) -> bool {
    let terrain = w.terrain_at(pos).expect("on the map");
    w.rules().terrains[&terrain].layer == Layer::Ground
}

/// A cog off the west shore and `units` of `key` standing inland on the west shore.
fn loaded_scene(w: &mut World, key: &str, units: usize) -> (EntityId, Vec<EntityId>) {
    let cog = w.spawn(id(w, "unit.cog"), P1, at(17_500, 32_500)).unwrap();
    let unit = id(w, key);
    let troops = (0..units)
        .map(|i| {
            let (x, y) = (
                8_500 + (i as i32 % 4) * 1000,
                28_500 + (i as i32 / 4) * 1000,
            );
            w.spawn(unit, P1, at(x, y)).unwrap()
        })
        .collect();
    (cog, troops)
}

fn failures(w: &World) -> Vec<String> {
    w.order_failures(P1)
        .map(|f| format!("{:?} {:?} {}", f.executor, f.kind, f.reason))
        .collect()
}

fn board_all(w: &mut World, cog: EntityId, troops: &[EntityId]) {
    for &t in troops {
        w.submit(Command {
            target_entity: Some(cog),
            ..command(P1, t, "enter")
        })
        .unwrap();
    }
    for _ in 0..2000 {
        w.step();
        if troops
            .iter()
            .all(|t| w.entity(*t).unwrap().container_id == Some(cog))
        {
            return;
        }
    }
    panic!("the troops never boarded: {:?}", failures(w));
}

#[test]
fn troops_board_sail_and_step_ashore_on_another_coast() {
    let run = || {
        let mut w = world();
        let (cog, troops) = loaded_scene(&mut w, "unit.spearman", 3);
        // A siege engine on wheels rides too.
        let catapult = w
            .spawn(id(&w, "unit.catapult"), P1, at(10_500, 34_500))
            .unwrap();
        let troops: Vec<EntityId> = troops.into_iter().chain([catapult]).collect();
        let (west, east) = (at(8_500, 28_500), at(56_500, 32_500));
        for &t in &troops {
            let ty = w.entity(t).unwrap().type_id;
            assert_eq!(w.reachable(ty, west, east), Some(false), "no way on land");
        }
        board_all(&mut w, cog, &troops);
        assert_eq!(w.passengers(cog), troops);
        let load: i64 = troops
            .iter()
            .map(|t| w.rules().entity_types[&w.entity(*t).unwrap().type_id].pop_cost)
            .sum();
        assert_eq!(w.carried_load(cog), load);
        assert_eq!(w.carry_capacity(cog), 10);
        // Aboard and far from land, a passenger cannot act.
        w.submit(Command {
            target_pos: Some(at(17_500, 40_500)),
            ..command(P1, cog, "move")
        })
        .unwrap();
        for _ in 0..400 {
            w.step();
        }
        // Sent to a point inland on the east shore: it sails over and they step ashore.
        w.submit(Command {
            target_pos: Some(east),
            ..command(P1, cog, "unload")
        })
        .unwrap();
        for _ in 0..4000 {
            w.step();
            if w.passengers(cog).is_empty() {
                break;
            }
        }
        assert!(w.passengers(cog).is_empty(), "{:?}", failures(&w));
        let ship = w.entity(cog).unwrap().pos;
        assert!(ship.x >= 30_000, "the cog crossed: {ship:?}");
        for &t in &troops {
            let e = w.entity(t).unwrap();
            assert_eq!(e.container_id, None);
            assert!(is_land(&w, e.pos), "{t:?} stands on land: {:?}", e.pos);
            assert!(e.pos.x >= EAST_SHORE * 1000, "{t:?} on the east shore");
        }
        // They walk on to the point.
        for _ in 0..600 {
            w.step();
        }
        for &t in &troops {
            let pos = w.entity(t).unwrap().pos;
            let (dx, dy) = (i64::from(pos.x - east.x), i64::from(pos.y - east.y));
            assert!(dx * dx + dy * dy <= 4000 * 4000, "{t:?} at {pos:?}");
        }
        w.hash()
    };
    assert_eq!(run(), run());
}

#[test]
fn a_full_ship_refuses_more_and_a_late_boarder_is_turned_away() {
    let run = || {
        let mut w = world();
        let (cog, troops) = loaded_scene(&mut w, "unit.spearman", 12);
        let pop = w.rules().entity_types[&id(&w, "unit.spearman")].pop_cost;
        let room = (w.carry_capacity(cog) / pop) as usize;
        assert_eq!(room, 10);
        // Eleven walk up together; the ship takes ten, the last to arrive is turned away.
        for &t in &troops[..=room] {
            w.submit(Command {
                target_entity: Some(cog),
                ..command(P1, t, "enter")
            })
            .unwrap();
        }
        for _ in 0..1500 {
            w.step();
        }
        assert_eq!(w.passengers(cog).len(), room);
        assert_eq!(w.carried_load(cog), w.carry_capacity(cog));
        let left: Vec<_> = troops[..=room]
            .iter()
            .filter(|t| w.entity(**t).unwrap().container_id.is_none())
            .collect();
        assert_eq!(left.len(), 1, "one left ashore");
        assert!(
            w.order_failures(P1).any(|f| f.reason == "no_room"),
            "{:?}",
            failures(&w)
        );
        // Full: the next is refused when ordered.
        assert_eq!(
            w.submit(Command {
                target_entity: Some(cog),
                ..command(P1, troops[room + 1], "enter")
            }),
            Err(SimError::NoRoomAboard)
        );
        w.hash()
    };
    assert_eq!(run(), run());
}

#[test]
fn ships_and_aircraft_never_board_and_a_ship_far_out_takes_no_one() {
    let mut w = world();
    let (cog, troops) = loaded_scene(&mut w, "unit.spearman", 1);
    let galley = w
        .spawn(id(&w, "unit.war_galley"), P1, at(18_500, 34_500))
        .unwrap();
    let biplane = w
        .spawn(id(&w, "unit.biplane"), P1, at(10_500, 30_500))
        .unwrap();
    let other = w.spawn(id(&w, "unit.cog"), P1, at(19_500, 30_500)).unwrap();
    let cog_type = id(&w, "unit.cog");
    for (unit, what) in [
        (galley, "a warship"),
        (biplane, "an aircraft"),
        (other, "a ship"),
    ] {
        let ty = w.entity(unit).unwrap().type_id;
        assert!(!w.may_ride(cog_type, ty), "{what} may not ride");
        assert_eq!(
            w.submit(Command {
                target_entity: Some(cog),
                ..command(P1, unit, "enter")
            }),
            Err(SimError::CannotBoard),
            "{what} is refused"
        );
    }
    assert!(w.may_ride(cog_type, id(&w, "unit.citizen")));
    assert!(w.may_ride(cog_type, id(&w, "unit.knight")));
    // Far out at sea nobody boards and nobody steps ashore.
    let far = w.spawn(id(&w, "unit.cog"), P1, at(28_500, 32_500)).unwrap();
    assert_eq!(
        w.submit(Command {
            target_entity: Some(far),
            ..command(P1, troops[0], "enter")
        }),
        Err(SimError::OffShore)
    );
    board_all(&mut w, cog, &troops);
    // Aboard near the shore, an order steps the unit ashore at once.
    w.submit(Command {
        target_pos: Some(at(6_500, 32_500)),
        ..command(P1, troops[0], "move")
    })
    .unwrap();
    w.step();
    let e = w.entity(troops[0]).unwrap();
    assert_eq!(e.container_id, None);
    assert!(is_land(&w, e.pos));
    board_all(&mut w, cog, &troops);
    w.submit(Command {
        target_pos: Some(at(28_500, 36_500)),
        ..command(P1, cog, "move")
    })
    .unwrap();
    for _ in 0..600 {
        w.step();
    }
    assert!(!w.shore_in_reach(cog));
    assert_eq!(
        w.submit(Command {
            target_entity: Some(cog),
            ..command(P1, cog, "unload")
        }),
        Err(SimError::OffShore)
    );
    assert_eq!(
        w.submit(Command {
            target_pos: Some(at(6_500, 32_500)),
            ..command(P1, troops[0], "move")
        }),
        Err(SimError::Aboard)
    );
}

#[test]
fn the_units_aboard_are_lost_with_a_sunk_ship() {
    let run = || {
        let mut w = world();
        let (cog, troops) = loaded_scene(&mut w, "unit.spearman", 5);
        board_all(&mut w, cog, &troops);
        w.submit(Command {
            target_pos: Some(at(28_500, 32_500)),
            ..command(P1, cog, "move")
        })
        .unwrap();
        for _ in 0..400 {
            w.step();
        }
        // A war galley of the same age answers the transport: it sinks it.
        let galley = w
            .spawn(id(&w, "unit.war_galley"), P2, at(34_500, 32_500))
            .unwrap();
        w.submit(Command {
            target_entity: Some(cog),
            ..command(P2, galley, "attack")
        })
        .unwrap();
        let losses = w.stats().losses.get(&P1).copied().unwrap_or(0);
        for _ in 0..4000 {
            w.step();
            if w.entity(cog).is_none() {
                break;
            }
        }
        assert!(w.entity(cog).is_none(), "the cog sank");
        assert!(w.entity(galley).is_some(), "the galley lived");
        for t in &troops {
            assert!(w.entity(*t).is_none(), "{t:?} went down with the ship");
        }
        assert_eq!(w.stats().losses[&P1], losses + 6, "ship and five aboard");
        let sunk: Vec<_> = w
            .alerts_of(P1)
            .filter_map(|a| match a.kind {
                AlertKind::Sunk { ship, lost, .. } => Some((ship, lost)),
                _ => None,
            })
            .collect();
        assert_eq!(sunk, vec![(cog, 5)]);
        w.hash()
    };
    assert_eq!(run(), run());
}
