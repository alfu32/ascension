//! City and fort shields (task 157): the Missile Shield shoots down explosive projectiles thrown
//! at what a city covers, the Laser Shield bullets too, the Energy Shield's pool takes damage to
//! what it covers before hit points and recharges from energy while not hit, and only the city
//! and fort lines hold a shield. Every scenario runs twice and compares state hashes.

use rom_rules::{DefId, Rules, build_rules_db};
use rom_sim::{Command, EntityId, MapGrid, Player, PlayerId, Pos, World};
use std::{collections::BTreeMap, collections::BTreeSet, sync::Arc};

const ATTACKER: PlayerId = PlayerId(1);
const DEFENDER: PlayerId = PlayerId(2);
/// The defender's city stands here, on a flat 96 × 96 map.
const CITY: (i32, i32) = (40_000, 48_000);

fn rules() -> Arc<Rules> {
    let db = build_rules_db(
        include_str!("../../../db/schema.sql"),
        include_str!("../../../db/seed.sql"),
    )
    .unwrap();
    Arc::new(Rules::load(&db).unwrap())
}

fn id(rules: &Rules, key: &str) -> DefId {
    rules.id_of(key).unwrap_or_else(|| panic!("{key}"))
}

/// A flat plain with the neutral player, the attacker and the defender, who has researched
/// `techs` and holds `energy`; the defender's city stands at `CITY`.
fn world(rules: &Arc<Rules>, techs: &[&str], energy: i64) -> (World, EntityId) {
    let plain = id(rules, "ter.plain");
    let players = (0..=2)
        .map(|p| Player {
            id: PlayerId(p),
            team: p,
            faction: id(rules, "nation.romans"),
            age: id(rules, "age.fusion"),
            resources: rules
                .resources
                .keys()
                .map(|r| {
                    let amount = if *r == id(rules, "res.energy") {
                        energy
                    } else {
                        100_000
                    };
                    (*r, amount)
                })
                .collect(),
            techs: if p == DEFENDER.0 {
                techs.iter().map(|t| (id(rules, t), 1)).collect()
            } else {
                BTreeMap::new()
            },
        })
        .collect();
    let mut w = World::new(
        Arc::clone(rules),
        MapGrid::new(96, 96, vec![plain; 96 * 96]).unwrap(),
        7,
        players,
    )
    .unwrap();
    let city = w
        .spawn(id(rules, "bld.city"), DEFENDER, Pos::flat(CITY.0, CITY.1))
        .unwrap();
    (w, city)
}

fn hold_fire(w: &mut World, id: EntityId) {
    let def = w
        .rules()
        .stances
        .values()
        .find(|s| s.behavior == "hold_fire")
        .unwrap()
        .id;
    w.set_stance(w.entity(id).unwrap().owner, id, def).unwrap();
}

fn attack(w: &mut World, executor: EntityId, target: EntityId) {
    w.submit(Command {
        player: ATTACKER,
        executor,
        kind: "attack".into(),
        subject: None,
        target_entity: Some(target),
        target_pos: None,
        queue: false,
        text: None,
        facing: None,
    })
    .unwrap();
}

/// What `shooter` (of the attacker, standing 9 tiles east of its target) did to the defender's
/// target (the city itself, or a farm 9 tiles north of it inside its city area) over at most
/// `ticks`: projectiles shot down, the tick the target was down (struck to no hit points, or the
/// limit) and the state hash.
fn barrage(techs: &[&str], shooter: &str, at_farm: bool, ticks: u64) -> (usize, u64, u64) {
    let rules = rules();
    let (mut w, city) = world(&rules, techs, 0);
    hold_fire(&mut w, city);
    let target = if at_farm {
        let farm = w
            .spawn(
                id(&rules, "bld.farm"),
                DEFENDER,
                Pos::flat(CITY.0, CITY.1 - 9_000),
            )
            .unwrap();
        hold_fire(&mut w, farm);
        farm
    } else {
        city
    };
    let at = w.entity(target).unwrap().pos;
    let gun = w
        .spawn(id(&rules, shooter), ATTACKER, Pos::flat(at.x + 9_000, at.y))
        .unwrap();
    // A scout beside the target spots it for the guns (siege sees little by itself).
    let scout = w
        .spawn(
            id(&rules, "unit.scout"),
            ATTACKER,
            Pos::flat(at.x + 5_000, at.y + 4_000),
        )
        .unwrap();
    hold_fire(&mut w, scout);
    w.step();
    attack(&mut w, gun, target);
    let mut seen = BTreeSet::new();
    let mut down = ticks;
    let mut hp = w.entity(target).unwrap().hp;
    for _ in 0..ticks {
        w.step();
        for i in w.interceptions() {
            assert_eq!(i.attacker, gun);
            seen.insert((i.tick, i.weapon_id));
        }
        // Struck down: gone, at no hit points, or a city knocked out and set up again by its
        // owner at a share of its hit points within the same tick (task 67).
        let now = w.entity(target).map_or(0, |e| e.hp);
        if now <= 0 || now > hp || w.entity(target).is_some_and(|e| e.neutralised) {
            down = w.tick();
            break;
        }
        hp = now;
    }
    (seen.len(), down, w.hash())
}

#[test]
fn the_missile_shield_shoots_down_rockets_and_not_bullets() {
    let open = barrage(&[], "unit.rocket_artillery", false, 6000);
    let shielded = barrage(
        &["tech.missile_shield"],
        "unit.rocket_artillery",
        false,
        6000,
    );
    assert_eq!(
        shielded,
        barrage(
            &["tech.missile_shield"],
            "unit.rocket_artillery",
            false,
            6000
        )
    );
    assert_eq!(open.0, 0, "nothing shot down without the research");
    assert!(shielded.0 >= 3, "rockets shot down: {shielded:?}");
    assert!(
        open.1 < shielded.1 && shielded.1 < 6000,
        "the city still falls, later: {shielded:?} against {open:?}"
    );
    // Bullets at a farm inside the city area pass the Missile Shield.
    let rifles = barrage(&["tech.missile_shield"], "unit.rifleman", true, 1200);
    assert_eq!(rifles.0, 0, "{rifles:?}");
}

#[test]
fn the_laser_shield_shoots_down_bullets_over_the_whole_city_area() {
    let techs = ["tech.missile_shield", "tech.laser_shield"];
    let rifles = barrage(&techs, "unit.rifleman", true, 1200);
    assert_eq!(rifles, barrage(&techs, "unit.rifleman", true, 1200));
    assert!(rifles.0 >= 5, "bullets at the farm shot down: {rifles:?}");
    // A larger share of rockets than the Missile Shield alone: the city stands longer.
    let missile = barrage(&techs[..1], "unit.rocket_artillery", false, 6000);
    let laser = barrage(&techs, "unit.rocket_artillery", false, 6000);
    assert!(
        missile.1 < laser.1 && laser.1 < 6000,
        "{laser:?} against {missile:?}"
    );
}

/// The defender's pool after `charge` ticks, then a rifleman's fire at a farm in the city area
/// for `fire` ticks, then `rest` ticks without fire: (pool when charged, farm hit points lost
/// while the pool held, pool after the fire, pool after the rest, energy spent, hash).
fn pool_run(energy: i64, charge: u64, fire: u64, rest: u64) -> (i64, i64, i64, i64, i64, u64) {
    let rules = rules();
    let techs = [
        "tech.missile_shield",
        "tech.laser_shield",
        "tech.energy_shield",
    ];
    let (mut w, city) = world(&rules, &techs, energy);
    hold_fire(&mut w, city);
    let farm = w
        .spawn(
            id(&rules, "bld.farm"),
            DEFENDER,
            Pos::flat(CITY.0, CITY.1 - 9_000),
        )
        .unwrap();
    hold_fire(&mut w, farm);
    let pool = |w: &World| w.shield_view(city).map_or(0, |s| s.pool);
    for _ in 0..charge {
        w.step();
    }
    let charged = pool(&w);
    let view = w.shield_view(city).unwrap();
    assert!(view.max > 0 && view.reach >= 16_000, "{view:?}");
    let farm_hp = w.entity(farm).unwrap().hp;
    let at = w.entity(farm).unwrap().pos;
    let gun = w
        .spawn(
            id(&rules, "unit.machine_gunner"),
            ATTACKER,
            Pos::flat(at.x + 6_000, at.y),
        )
        .unwrap();
    w.step();
    attack(&mut w, gun, farm);
    let mut lost_while_held = 0;
    for _ in 0..fire {
        let hp = w.entity(farm).map_or(0, |e| e.hp);
        w.step();
        if pool(&w) > 0 {
            lost_while_held += hp - w.entity(farm).map_or(0, |e| e.hp);
        }
    }
    w.despawn(gun).unwrap();
    let after_fire = pool(&w);
    let energy_before =
        w.players().find(|p| p.id == DEFENDER).unwrap().resources[&id(&rules, "res.energy")];
    for _ in 0..rest {
        w.step();
    }
    let spent = energy_before
        - w.players().find(|p| p.id == DEFENDER).unwrap().resources[&id(&rules, "res.energy")];
    let _ = farm_hp;
    (
        charged,
        lost_while_held,
        after_fire,
        pool(&w),
        spent,
        w.hash(),
    )
}

#[test]
fn the_energy_shield_pool_takes_the_damage_first_and_recharges_from_energy() {
    let run = pool_run(10_000, 900, 900, 900);
    assert_eq!(run, pool_run(10_000, 900, 900, 900));
    let (charged, lost, after_fire, rested, spent, _) = run;
    let rules = rules();
    let (w, city) = world(
        &rules,
        &[
            "tech.missile_shield",
            "tech.laser_shield",
            "tech.energy_shield",
        ],
        0,
    );
    let max = w.entity(city).unwrap().max_hp / 2;
    assert_eq!(
        charged, max,
        "the pool charged to half the city's hit points"
    );
    assert_eq!(lost, 0, "nothing reached the farm while the pool held");
    assert!(after_fire < max, "the pool took the fire: {run:?}");
    assert_eq!(rested, max, "and recharged: {run:?}");
    // 10 energy per 100 points restored, rounded up per recharge.
    assert!(spent >= (max - after_fire) / 10 && spent > 0, "{run:?}");
}

#[test]
fn without_energy_the_pool_does_not_recharge() {
    let run = pool_run(0, 900, 300, 900);
    assert_eq!(run, pool_run(0, 900, 300, 900));
    assert_eq!((run.0, run.2, run.3, run.4), (0, 0, 0, 0), "{run:?}");
}

#[test]
fn every_city_and_fort_of_their_lines_holds_a_shield_and_nothing_else() {
    let rules = rules();
    let tag = |key: &str| rules.tags.values().find(|t| t.key == key).unwrap().id;
    let (city, fort) = (tag("city"), tag("fort"));
    // The fort line by its upgrades, from the fort.
    let mut line = vec![id(&rules, "bld.fort")];
    while let Some(next) = rules
        .entity_types
        .values()
        .find(|et| et.upgrades_from_id == line.last().copied())
    {
        line.push(next.id);
    }
    assert!(line.len() >= 4, "{line:?}");
    let shielded: Vec<DefId> = rules
        .entity_types
        .values()
        .filter(|et| et.shield.is_some())
        .map(|et| et.id)
        .collect();
    for t in &line {
        assert!(shielded.contains(t), "{}", rules.key(*t));
    }
    assert!(shielded.contains(&id(&rules, "bld.city")));
    for et in rules.entity_types.values() {
        assert_eq!(
            et.shield.is_some(),
            rules.has_tag(et.id, city) || rules.has_tag(et.id, fort),
            "{}",
            rules.key(et.id)
        );
    }
    // In a match: with every shield research, each of them shows a shield, a tower does not.
    let techs = [
        "tech.missile_shield",
        "tech.laser_shield",
        "tech.energy_shield",
    ];
    let (mut w, _) = world(&rules, &techs, 0);
    let mut x = 10_000;
    let mut ids = Vec::new();
    for t in line.iter().chain([&id(&rules, "bld.tower")]) {
        ids.push((*t, w.spawn(*t, DEFENDER, Pos::flat(x, 85_000)).unwrap()));
        x += 9_000;
    }
    w.step();
    for (t, e) in ids {
        assert_eq!(
            w.shield_view(e).is_some(),
            rules.entity_types[&t].shield.is_some(),
            "{}",
            rules.key(t)
        );
    }
}

/// A citadel with every shield charged, standing in the city area (so the city's dome covers it
/// too), then three fusion tanks attacking it from the east for `ticks`: (projectiles shot down,
/// blows on its own pool, blows that came from the east, its hit points lost, hash).
fn citadel_under_fire(ticks: u64) -> (usize, usize, usize, i64, u64) {
    let rules = rules();
    let techs = [
        "tech.missile_shield",
        "tech.laser_shield",
        "tech.energy_shield",
    ];
    let (mut w, city) = world(&rules, &techs, 10_000);
    hold_fire(&mut w, city);
    let at = Pos::flat(CITY.0, CITY.1 + 12_000);
    let citadel = w.spawn(id(&rules, "bld.citadel"), DEFENDER, at).unwrap();
    hold_fire(&mut w, citadel);
    for _ in 0..900 {
        w.step();
    }
    let full = w.shield_view(citadel).unwrap();
    assert!(full.max > 0 && full.pool == full.max, "{full:?}");
    let hp = w.entity(citadel).unwrap().hp;
    let tanks: Vec<EntityId> = (0..3)
        .map(|n| {
            w.spawn(
                id(&rules, "unit.fusion_tank"),
                ATTACKER,
                Pos::flat(at.x + 6_000, at.y - 3_000 + n * 3_000),
            )
            .unwrap()
        })
        .collect();
    w.step();
    for tank in &tanks {
        attack(&mut w, *tank, citadel);
    }
    let (mut downed, mut blows, mut east) = (BTreeSet::new(), BTreeSet::new(), 0);
    for _ in 0..ticks {
        w.step();
        for i in w.interceptions() {
            assert_eq!(i.shield, citadel);
            downed.insert((i.tick, i.attacker));
        }
        for h in w.shield_hits() {
            // The nearest pool takes the blow: the citadel's own, not the city's.
            assert_eq!(h.shield, citadel);
            if blows.insert((h.tick, h.absorbed, h.at.x, h.at.y))
                && h.from.is_some_and(|f| f.x > at.x)
            {
                east += 1;
            }
        }
    }
    let lost = hp - w.entity(citadel).map_or(0, |e| e.hp);
    (downed.len(), blows.len(), east, lost, w.hash())
}

#[test]
fn a_fort_shoots_down_shells_and_its_pool_takes_the_rest() {
    let run = citadel_under_fire(400);
    assert_eq!(run, citadel_under_fire(400));
    let (downed, blows, east, lost, _) = run;
    assert!(downed > 0, "{run:?}");
    assert!(blows > 0 && east == blows, "{run:?}");
    assert_eq!(lost, 0, "the pool held: {run:?}");
}
