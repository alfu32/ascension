//! Task 147: fortification lines with gates, and minefields.

use crate::{Command, EntityId, MapGrid, Player, PlayerId, Pos, Sighting, SimError, World};
use rom_rules::{DefId, Rules, build_rules_db};
use std::sync::Arc;

const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);
/// Player 1's ally.
const P3: PlayerId = PlayerId(3);

/// A plain of `w` × `h` tiles. Players 1 and 3 are one team, player 2 another; every player
/// starts in `age` with `techs`. Buildings do not shoot.
fn world_sized(w: u32, h: u32, age: &str, techs: &[&str]) -> World {
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
    let players = (0..=3)
        .map(|n| Player {
            id: PlayerId(n),
            team: if n == 3 { 1 } else { n },
            faction: rules.id_of("nation.romans").unwrap(),
            age: rules.id_of(age).unwrap(),
            resources: rules.resources.keys().map(|r| (*r, 5_000)).collect(),
            techs: techs.iter().map(|t| (rules.id_of(t).unwrap(), 1)).collect(),
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
fn command(
    w: &mut World,
    executor: EntityId,
    kind: &str,
    subject: Option<DefId>,
    target_entity: Option<EntityId>,
    target_pos: Option<Pos>,
    queue: bool,
) -> Result<(), SimError> {
    let player = w.entity(executor).unwrap().owner;
    w.submit(Command {
        player,
        executor,
        kind: kind.into(),
        subject,
        target_entity,
        target_pos,
        queue,
        text: None,
        facing: None,
    })
    .map(|_| ())
}
fn walk(w: &mut World, executor: EntityId, x: i32, y: i32) {
    command(w, executor, "move", None, None, Some(at(x, y)), false).unwrap();
}
/// A palisade across the map at row `y`, with its three-tile gate centred at column `gate_x`.
fn wall_across(w: &mut World, owner: PlayerId, width: i32, y: i32, gate_x: Option<i32>) {
    for x in 0..width {
        match gate_x {
            Some(g) if x == g => {
                spawn(w, "bld.palisade_gate", owner, x, y);
            }
            Some(g) if (x - g).abs() == 1 => {}
            _ => {
                spawn(w, "bld.palisade", owner, x, y);
            }
        }
    }
}
fn sees(w: &World, player: PlayerId, target: EntityId) -> bool {
    w.visible_entities(player)
        .iter()
        .any(|s| matches!(s, Sighting::Live(e) if e.id == target))
}

#[test]
fn a_wall_stops_everyone_and_its_gate_lets_its_own_team_through() {
    // Priests fight no one: only the wall stands in their way.
    let mut w = world_sized(24, 40, "age.ancient", &[]);
    wall_across(&mut w, P1, 24, 20, Some(2));
    let own = spawn(&mut w, "unit.priest", P1, 2, 8);
    let ally = spawn(&mut w, "unit.priest", P3, 5, 8);
    let enemy = spawn(&mut w, "unit.priest", P2, 21, 4);
    walk(&mut w, own, 3, 34);
    walk(&mut w, ally, 6, 34);
    walk(&mut w, enemy, 21, 34);
    ticks(&mut w, 1600);
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
        "the enemy stays outside: {:?}",
        tile(&w, enemy)
    );
    // The enemy's own gate is the one that lets it through: give it the wall's gate.
    let gate = w
        .entities()
        .find(|e| e.type_id == id(&w, "bld.palisade_gate"))
        .unwrap()
        .id;
    w.change_owner(gate, P2);
    walk(&mut w, enemy, 21, 34);
    walk(&mut w, own, 3, 5);
    ticks(&mut w, 1600);
    assert!(
        tile(&w, enemy).1 > 20,
        "the captured gate lets its new team through"
    );
    assert!(
        tile(&w, own).1 > 20,
        "and shuts the old one out: {:?}",
        tile(&w, own)
    );
}

#[test]
fn a_diagonal_wall_leaves_no_gap_between_its_corners() {
    let mut w = world_sized(24, 24, "age.ancient", &[]);
    let sites = w
        .line_sites(P1, id(&w, "bld.palisade"), &[at(0, 0), at(23, 23)], &[])
        .unwrap();
    assert_eq!(sites.len(), 24, "one segment a tile of the diagonal");
    for site in &sites {
        w.place_for_setup(site.subject, P1, site.pos).unwrap();
    }
    let enemy = spawn(&mut w, "unit.spearman", P2, 18, 4);
    walk(&mut w, enemy, 4, 18);
    ticks(&mut w, 1200);
    let (x, y) = tile(&w, enemy);
    assert!(x > y, "the enemy stays on its side: {:?}", (x, y));
}

#[test]
fn line_sites_follow_the_drag_with_the_gate_nearest_its_point() {
    let mut w = world_sized(96, 96, "age.ancient", &["tech.military.1"]);
    spawn(&mut w, "bld.city", P1, 25, 25);
    let palisade = id(&w, "bld.palisade");
    let gate = id(&w, "bld.palisade_gate");
    let bare = w
        .line_sites(P1, palisade, &[at(20, 32), at(30, 35), at(30, 40)], &[])
        .unwrap();
    // 11 tiles along the first stretch, then 5 more down the second (the corner once).
    assert_eq!(bare.len(), 16);
    for pair in bare.windows(2) {
        let (a, b) = (pair[0].pos, pair[1].pos);
        assert!((a.x - b.x).abs() <= 1000 && (a.y - b.y).abs() <= 1000 && a != b);
    }
    assert_eq!(bare[0].pos, at(20, 32));
    assert_eq!(bare[10].pos, at(30, 35));
    assert_eq!(bare[15].pos, at(30, 40));
    // The second stretch runs down: its segments face along +y (90°).
    assert_eq!(bare[12].facing, 90);
    assert_eq!(bare[0].facing, crate::math::facing_of(1000, 0));
    // A gate is three tiles wide along the line (task 147b): it takes the place of the three
    // segments nearest its point, centred where the line runs straight, and faces the line.
    let sites = w
        .line_sites(
            P1,
            palisade,
            &[at(20, 32), at(30, 35), at(30, 40)],
            &[at(30, 38)],
        )
        .unwrap();
    assert_eq!(sites.len(), 14);
    assert_eq!(sites[0].pos, at(20, 32));
    let gates: Vec<&crate::LineSite> = sites.iter().filter(|s| s.subject == gate).collect();
    assert_eq!(gates.len(), 1);
    assert_eq!((gates[0].pos, gates[0].facing), (at(30, 38), 90));
    for y in 37..=39 {
        assert!(
            !sites
                .iter()
                .any(|s| s.subject == palisade && s.pos == at(30, y)),
            "no segment under the gate at row {y}"
        );
    }
    assert!(sites.iter().all(|s| s.check.is_ok()), "inside the borders");
    // The second stretch runs down: its segments face along +y (90°).
    assert_eq!(sites[12].facing, 90);
    assert_eq!(sites[0].facing, crate::math::facing_of(1000, 0));
    // Outside the borders, on another segment, or of a type that is no segment, it says so.
    let far = w
        .line_sites(P1, palisade, &[at(70, 70), at(72, 70)], &[])
        .unwrap();
    assert!(
        far.iter()
            .all(|s| s.check == Err(SimError::NotInOwnTerritory))
    );
    w.place_for_setup(palisade, P1, at(20, 32)).unwrap();
    let again = w
        .line_sites(P1, palisade, &[at(20, 32), at(21, 32)], &[])
        .unwrap();
    assert_eq!(again[0].check, Err(SimError::PlacementOverlap));
    assert_eq!(again[1].check, Ok(()));
    assert_eq!(
        w.line_sites(P1, gate, &[at(20, 32)], &[]),
        Err(SimError::NotALine)
    );
    assert_eq!(
        w.line_sites(
            P1,
            palisade,
            &[at(0, 0), at(95, 95), at(0, 95), at(95, 0)],
            &[]
        ),
        Err(SimError::LineTooLong)
    );
}

#[test]
fn a_line_is_built_by_construct_orders_and_its_builder_is_not_shut_in() {
    let mut w = world_sized(96, 96, "age.ancient", &["tech.military.1"]);
    spawn(&mut w, "bld.city", P1, 25, 25);
    let citizen = spawn(&mut w, "unit.citizen", P1, 24, 34);
    let sites = w
        .line_sites(
            P1,
            id(&w, "bld.palisade"),
            &[at(18, 33), at(30, 33)],
            &[at(24, 33)],
        )
        .unwrap();
    let timber = id(&w, "res.timber");
    let before = w.players[&P1].resources[&timber];
    for (n, site) in sites.iter().enumerate() {
        command(
            &mut w,
            citizen,
            "construct",
            Some(site.subject),
            None,
            Some(site.pos),
            n > 0,
        )
        .unwrap();
    }
    // Ten segments of 10 timber and a three-tile gate of 30 in their place, without a ramp.
    assert_eq!(sites.len(), 11);
    assert_eq!(before - w.players[&P1].resources[&timber], 10 * 10 + 30);
    ticks(&mut w, 1);
    // Every site is laid at once (the citizen was standing on one and stepped aside).
    let laid = |w: &World| {
        sites
            .iter()
            .filter(|s| {
                w.entities()
                    .any(|e| e.pos == s.pos && e.type_id == s.subject)
            })
            .count()
    };
    assert_eq!(laid(&w), sites.len());
    ticks(&mut w, 3000);
    let done = sites
        .iter()
        .filter(|s| {
            w.entities()
                .any(|e| e.pos == s.pos && e.type_id == s.subject && e.build_progress.is_none())
        })
        .count();
    assert_eq!(done, sites.len(), "the whole line stands");
    walk(&mut w, citizen, 24, 45);
    ticks(&mut w, 400);
    assert_eq!(tile(&w, citizen), (24, 45), "its builder walks away");
}

#[test]
fn walls_gates_and_mines_do_not_widen_a_town() {
    let mut w = world_sized(96, 96, "age.industrial", &[]);
    let city = spawn(&mut w, "bld.city", P1, 25, 25);
    let radius = w.city_economic_radius(city);
    for x in 20..30 {
        spawn(&mut w, "bld.defensive_line", P1, x, 33);
        spawn(&mut w, "bld.minefield", P1, x, 35);
    }
    spawn(&mut w, "bld.checkpoint", P1, 31, 33);
    assert_eq!(w.city_economic_radius(city), radius);
    spawn(&mut w, "bld.barracks", P1, 30, 20);
    assert!(
        w.city_economic_radius(city) > radius,
        "a barracks still does"
    );
}

#[test]
fn walls_and_gates_change_with_the_age_and_grow_with_its_later_ages() {
    let mut w = world_sized(48, 48, "age.ancient", &["tech.military.1"]);
    spawn(&mut w, "bld.city", P1, 25, 25);
    let wall = spawn(&mut w, "bld.palisade", P1, 20, 33);
    let gate = spawn(&mut w, "bld.palisade_gate", P1, 21, 33);
    w.entities[wall.0 as usize].as_mut().unwrap().hp = 250;
    w.complete_research(P1, id(&w, "tech.age.classical"));
    let e = w.entity(wall).unwrap();
    assert_eq!(w.rules.key(e.type_id), "bld.stone_wall");
    assert_eq!(
        (e.hp, e.max_hp),
        (750, 1500),
        "half its health before and after"
    );
    assert_eq!(
        w.rules.key(w.entity(gate).unwrap().type_id),
        "bld.wall_gate"
    );
    // The palisade is out of the build menu now.
    let citizen = spawn(&mut w, "unit.citizen", P1, 24, 34);
    let palisade = id(&w, "bld.palisade");
    assert_eq!(
        command(
            &mut w,
            citizen,
            "construct",
            Some(palisade),
            None,
            Some(at(20, 35)),
            false
        ),
        Err(SimError::Superseded)
    );
    // Each later age of the kind adds hit points, armour and build time.
    let stone = id(&w, "bld.stone_wall");
    let classical = w.rules.build_ticks(stone, &w.active_sources(P1));
    w.complete_research(P1, id(&w, "tech.age.medieval"));
    assert_eq!(w.entity(wall).unwrap().max_hp, 1800);
    assert!(w.rules.build_ticks(stone, &w.active_sources(P1)) > classical);
    w.complete_research(P1, id(&w, "tech.age.gunpowder"));
    w.complete_research(P1, id(&w, "tech.age.enlightenment"));
    w.complete_research(P1, id(&w, "tech.age.industrial"));
    let e = w.entity(wall).unwrap();
    assert_eq!(w.rules.key(e.type_id), "bld.defensive_line");
    assert_eq!(e.max_hp, 2600);
    assert_eq!(
        w.rules.key(w.entity(gate).unwrap().type_id),
        "bld.checkpoint"
    );
    // The gate stays a gate through the changes: its team still passes.
    let own = spawn(&mut w, "unit.rifleman", P1, 21, 30);
    let (face, gate_tile) = {
        let g = w.entity(gate).unwrap().pos;
        (g.face, w.surfaces[0].geo.pos_index(g).unwrap() as u32)
    };
    assert_eq!(face, 0);
    assert_eq!(w.surfaces[0].pathing.gates.get(&gate_tile), Some(&1));
    walk(&mut w, own, 21, 36);
    ticks(&mut w, 400);
    assert_eq!(tile(&w, own), (21, 36));
}

#[test]
fn siege_breaks_a_wall() {
    let mut w = world_sized(48, 48, "age.classical", &[]);
    let wall = spawn(&mut w, "bld.stone_wall", P1, 20, 20);
    let catapult = spawn(&mut w, "unit.catapult", P2, 20, 24);
    ticks(&mut w, 1);
    command(&mut w, catapult, "attack", None, Some(wall), None, false).unwrap();
    ticks(&mut w, 200);
    assert!(w.entity(wall).unwrap().hp < w.entity(wall).unwrap().max_hp);
    ticks(&mut w, 6000);
    assert!(w.entity(wall).is_none(), "the wall falls to siege");
}

#[test]
fn a_mine_is_hidden_from_enemies_until_a_detector_comes_near() {
    let mut w = world_sized(48, 48, "age.industrial", &[]);
    let mine = spawn(&mut w, "bld.minefield", P1, 20, 20);
    let rifleman = spawn(&mut w, "unit.rifleman", P2, 20, 17);
    ticks(&mut w, 2);
    assert!(sees(&w, P1, mine) && sees(&w, P3, mine), "its team sees it");
    assert!(w.is_visible(P2, 20, 20), "its tile is in sight");
    assert!(!sees(&w, P2, mine), "but the mine is not");
    assert_eq!(
        command(&mut w, rifleman, "attack", None, Some(mine), None, false),
        Err(SimError::TargetHidden)
    );
    // Mine detectors: the infantry of the player sees mines within the radius.
    w.complete_research(P2, id(&w, "tech.mine_detectors"));
    assert!(sees(&w, P2, mine));
    command(&mut w, rifleman, "attack", None, Some(mine), None, false).unwrap();
    // Out of the radius, though its tile is still in sight, it is hidden again.
    walk(&mut w, rifleman, 20, 15);
    ticks(&mut w, 300);
    assert!(w.is_visible(P2, 20, 20));
    assert!(!sees(&w, P2, mine));
    assert!(!w.remembered_buildings(P2).any(|b| b.id == mine));
}

#[test]
fn an_enemy_sets_off_a_mine_and_own_and_allied_units_do_not() {
    let mut w = world_sized(48, 48, "age.industrial", &[]);
    let mine = spawn(&mut w, "bld.minefield", P1, 20, 20);
    let own = spawn(&mut w, "unit.rifleman", P1, 20, 15);
    let ally = spawn(&mut w, "unit.rifleman", P3, 20, 13);
    walk(&mut w, own, 20, 25);
    walk(&mut w, ally, 20, 24);
    ticks(&mut w, 300);
    assert!(tile(&w, own).1 >= 24 && tile(&w, ally).1 >= 23);
    let full = w.entity(own).unwrap().max_hp;
    assert_eq!(w.entity(own).unwrap().hp, full);
    assert_eq!(w.entity(ally).unwrap().hp, full);
    assert!(w.entity(mine).is_some(), "own side passes over it");
    // Out of the enemy's way before it comes.
    walk(&mut w, own, 3, 44);
    walk(&mut w, ally, 6, 44);
    ticks(&mut w, 600);
    let enemy = spawn(&mut w, "unit.rifleman", P2, 20, 15);
    walk(&mut w, enemy, 20, 25);
    ticks(&mut w, 300);
    assert!(w.entity(mine).is_none(), "spent");
    let e = w.entity(enemy).unwrap();
    // 140 explosive against plate (100 %), less its armour of 2.
    assert_eq!(e.max_hp - e.hp, 140 - 2);
    assert!(w.blasts().iter().any(|b| b.pos == at(20, 20)));
}

#[test]
fn aircraft_pass_over_mines_and_mines_block_nobody() {
    let mut w = world_sized(48, 48, "age.industrial", &[]);
    let mine = spawn(&mut w, "bld.minefield", P1, 20, 20);
    let index = w.surfaces[0].geo.pos_index(at(20, 20)).unwrap();
    assert_eq!(w.surfaces[0].pathing.occupancy[index], 0);
    let plane = spawn(&mut w, "unit.biplane", P2, 20, 15);
    walk(&mut w, plane, 20, 25);
    ticks(&mut w, 300);
    assert!(
        w.entity(mine).is_some(),
        "an aircraft in the air sets off nothing"
    );
}

#[test]
fn walls_gates_and_mines_are_saved_as_ordinary_state() {
    let mut w = world_sized(24, 40, "age.industrial", &[]);
    for x in (0..24).filter(|x| !(11..=13).contains(x)) {
        spawn(&mut w, "bld.defensive_line", P1, x, 20);
    }
    // The checkpoint spans three tiles along the line.
    spawn(&mut w, "bld.checkpoint", P1, 12, 20);
    spawn(&mut w, "bld.minefield", P1, 12, 24);
    let own = spawn(&mut w, "unit.priest", P1, 4, 10);
    let enemy = spawn(&mut w, "unit.priest", P2, 6, 10);
    walk(&mut w, own, 4, 30);
    walk(&mut w, enemy, 6, 30);
    ticks(&mut w, 1);
    let mut copy = World::restore(w.rules().clone(), w.snapshot()).unwrap();
    assert_eq!(copy.hash(), w.hash());
    assert_eq!(copy.surfaces[0].pathing.gates, w.surfaces[0].pathing.gates);
    let sides = |w: &World| {
        w.surfaces[0]
            .pathing
            .queue
            .iter()
            .map(|r| r.side)
            .collect::<Vec<_>>()
    };
    assert_eq!(sides(&copy), sides(&w));
    ticks(&mut w, 900);
    ticks(&mut copy, 900);
    assert_eq!(copy.hash(), w.hash());
    assert!(tile(&w, own).1 > 20 && tile(&w, enemy).1 < 20);
}

/// A town walls itself in with a gate and a minefield before it; an enemy column marches on
/// it with siege, and the town's age advances under way.
fn scenario() -> u64 {
    let mut w = world_sized(64, 64, "age.industrial", &[]);
    spawn(&mut w, "bld.city", P1, 16, 16);
    let sites = w
        .line_sites(
            P1,
            id(&w, "bld.defensive_line"),
            &[at(4, 28), at(28, 28), at(28, 4)],
            &[at(16, 28)],
        )
        .unwrap();
    for site in &sites {
        w.place_turned_for_setup(site.subject, P1, site.pos, site.facing)
            .unwrap();
    }
    let mines = w
        .minefield_sites(P1, id(&w, "bld.minefield"), at(10, 30), at(22, 31))
        .unwrap();
    for site in &mines {
        w.place_for_setup(site.subject, P1, site.pos).unwrap();
    }
    let defenders: Vec<EntityId> = (0..4)
        .map(|n| spawn(&mut w, "unit.rifleman", P1, 14 + n, 20))
        .collect();
    let mut raiders: Vec<EntityId> = (0..6)
        .map(|n| spawn(&mut w, "unit.rifleman", P2, 12 + n * 2, 44))
        .collect();
    for n in 0..2 {
        raiders.push(spawn(&mut w, "unit.field_gun", P2, 14 + n * 4, 48));
    }
    for raider in &raiders {
        command(
            &mut w,
            *raider,
            "attack_move",
            None,
            None,
            Some(at(16, 16)),
            false,
        )
        .unwrap();
    }
    for defender in &defenders {
        command(
            &mut w,
            *defender,
            "attack_move",
            None,
            None,
            Some(at(16, 36)),
            false,
        )
        .unwrap();
    }
    ticks(&mut w, 600);
    w.complete_research(P1, id(&w, "tech.age.modern"));
    w.complete_research(P2, id(&w, "tech.mine_detectors"));
    ticks(&mut w, 1400);
    w.hash()
}

#[test]
fn fortification_scenario_runs_alike_twice() {
    assert_eq!(scenario(), scenario());
}

#[test]
fn a_gate_turns_its_footprint_with_its_facing() {
    let mut w = world_sized(40, 40, "age.ancient", &[]);
    let palisade = id(&w, "bld.palisade");
    let gate = id(&w, "bld.palisade_gate");
    let free = |w: &World, x: i32, y: i32| w.placement(P1, palisade, at(x, y), true).is_ok();
    // Facing 0: three tiles across.
    let across = w.place_turned_for_setup(gate, P1, at(10, 10), 0).unwrap();
    assert!(!free(&w, 9, 10) && !free(&w, 11, 10) && free(&w, 10, 9) && free(&w, 10, 11));
    // A quarter turn: three tiles down.
    let down = w.place_turned_for_setup(gate, P1, at(20, 10), 90).unwrap();
    assert_eq!(w.entity(down).unwrap().facing, 90);
    assert!(!free(&w, 20, 9) && !free(&w, 20, 11) && free(&w, 19, 10) && free(&w, 21, 10));
    // A diagonal: the square of its length, corners included.
    let diagonal = w.place_turned_for_setup(gate, P1, at(30, 30), 45).unwrap();
    for (x, y) in [(29, 29), (31, 31), (31, 29), (29, 31)] {
        assert!(!free(&w, x, y), "diagonal gate covers {:?}", (x, y));
    }
    assert!(free(&w, 32, 32));
    // Facings snap to multiples of 45 degrees, and so does what construct is given.
    assert_eq!(crate::snap_facing(100), 90);
    assert_eq!(crate::snap_facing(340), 0);
    assert_eq!(crate::turned_footprint(3, 1, 270), (1, 3));
    assert_eq!(crate::turned_footprint(3, 1, 135), (3, 3));
    // Its own team walks through every tile of a turned gate; the enemy through none.
    for g in [across, diagonal] {
        let p = w.entity(g).unwrap().pos;
        let tile = w.surfaces[0]
            .geo
            .index_of(crate::geometry::Tile::new(0, p.x / 1000 + 1, p.y / 1000))
            .unwrap() as u32;
        assert_eq!(w.surfaces[0].pathing.gates.get(&tile), Some(&1));
    }
}

/// A diagonal palisade with a gate, laid by construct orders with their facings by three
/// builders; optionally saved and restored half built.
fn diagonal_line(save_at: Option<u64>) -> (u64, World) {
    let mut w = world_sized(48, 48, "age.ancient", &["tech.military.1"]);
    spawn(&mut w, "bld.city", P1, 36, 10);
    let citizens: Vec<EntityId> = (0..3)
        .map(|n| spawn(&mut w, "unit.citizen", P1, 30 + n, 14))
        .collect();
    let sites = w
        .line_sites(
            P1,
            id(&w, "bld.palisade"),
            &[at(22, 10), at(34, 22)],
            &[at(28, 16)],
        )
        .unwrap();
    assert!(sites.iter().all(|s| s.check.is_ok()), "{sites:?}");
    let gate = id(&w, "bld.palisade_gate");
    assert_eq!(
        sites
            .iter()
            .filter(|s| s.subject == gate)
            .map(|s| s.facing)
            .collect::<Vec<_>>(),
        vec![45]
    );
    for (n, chunk) in sites.chunks(sites.len().div_ceil(3)).enumerate() {
        for (k, site) in chunk.iter().enumerate() {
            let player = P1;
            w.submit(Command {
                player,
                executor: citizens[n],
                kind: "construct".into(),
                subject: Some(site.subject),
                target_entity: None,
                target_pos: Some(site.pos),
                queue: k > 0,
                text: None,
                facing: Some(site.facing),
            })
            .unwrap();
        }
    }
    // An enemy walks round the line's end; an own citizen through its gate.
    let enemy = spawn(&mut w, "unit.priest", P2, 20, 20);
    walk(&mut w, enemy, 34, 12);
    for tick in 0..4000 {
        if save_at == Some(tick) {
            w = World::restore(w.rules().clone(), w.snapshot()).unwrap();
        }
        w.step();
    }
    (w.hash(), w)
}

#[test]
fn a_diagonal_line_with_its_gate_runs_alike_twice_and_across_a_save() {
    let (first, w) = diagonal_line(None);
    let (second, _) = diagonal_line(None);
    assert_eq!(first, second);
    let (restored, _) = diagonal_line(Some(30));
    assert_eq!(first, restored, "a save half way changes nothing");
    let gate = w
        .entities()
        .find(|e| e.type_id == id(&w, "bld.palisade_gate"))
        .unwrap();
    assert_eq!((gate.facing, gate.build_progress), (45, None));
    let segments = w
        .entities()
        .filter(|e| e.type_id == id(&w, "bld.palisade") && e.build_progress.is_none())
        .count();
    assert_eq!(segments + 3, 13, "the line stands");
}

/// Task 147b: a town walled all round is no refuge: an attack-moving column finds no way in,
/// stops at the wall and breaks through it.
#[test]
fn an_army_breaks_through_a_wall_that_rings_its_target() {
    let mut w = world_sized(48, 48, "age.classical", &[]);
    let city = spawn(&mut w, "bld.city", P1, 24, 24);
    let ring = [at(16, 16), at(32, 16), at(32, 32), at(16, 32), at(16, 16)];
    let sites = w
        .line_sites(P1, id(&w, "bld.stone_wall"), &ring, &[])
        .unwrap();
    for site in &sites {
        w.place_turned_for_setup(site.subject, P1, site.pos, site.facing)
            .unwrap();
    }
    let walls = sites.len();
    let raiders: Vec<EntityId> = (0..4)
        .map(|n| spawn(&mut w, "unit.catapult", P2, 22 + n * 2, 44))
        .collect();
    let target = w.entity(city).unwrap().pos;
    for r in &raiders {
        command(&mut w, *r, "attack_move", None, None, Some(target), false).unwrap();
    }
    ticks(&mut w, 3000);
    let standing = w
        .entities()
        .filter(|e| e.type_id == id(&w, "bld.stone_wall"))
        .count();
    assert!(standing < walls, "a breach: {standing} of {walls} stand");
}
