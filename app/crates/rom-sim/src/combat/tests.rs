use crate::{Command, EntityId, MapGrid, Player, PlayerId, Pos, SimError, World};
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

fn world(sql: &str) -> World {
    let rules = rules(sql);
    let players = (0..=3)
        .map(|id| Player {
            id: PlayerId(id),
            team: if id == 3 { 1 } else { id },
            faction: rules.id_of("nation.romans").unwrap(),
            age: *rules.ages.keys().next().unwrap(),
            resources: BTreeMap::new(),
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

fn at(x: i32, y: i32) -> Pos {
    Pos::flat(x, y)
}
fn ticks(w: &mut World, n: u64) {
    for _ in 0..n {
        w.step();
    }
}
fn stance(w: &mut World, id: EntityId, behavior: &str) {
    let def = w
        .rules
        .stances
        .values()
        .find(|s| s.behavior == behavior)
        .unwrap()
        .id;
    w.set_stance(w.entity(id).unwrap().owner, id, def).unwrap();
}
fn spawn(w: &mut World, key: &str, owner: u32, pos: Pos) -> EntityId {
    let id = w
        .spawn(w.rules.id_of(key).unwrap(), PlayerId(owner), pos)
        .unwrap();
    stance(w, id, "hold_fire");
    id
}
fn command(
    w: &World,
    id: EntityId,
    kind: &str,
    target: Option<EntityId>,
    point: Option<Pos>,
) -> Command {
    Command {
        player: w.entity(id).unwrap().owner,
        executor: id,
        kind: kind.into(),
        subject: None,
        target_entity: target,
        target_pos: point,
        queue: false,
    }
}
fn attack(w: &mut World, id: EntityId, target: EntityId) {
    w.submit(command(w, id, "attack", Some(target), None))
        .unwrap();
}
fn injure(w: &mut World, id: EntityId, hp: i64) {
    w.entities[id.0 as usize].as_mut().unwrap().hp = hp;
}
fn hp(w: &World, id: EntityId) -> i64 {
    w.entity(id).unwrap().hp
}

#[test]
fn melee_unit_walks_into_range_then_hits_on_cooldown() {
    let mut w = world("");
    let a = spawn(&mut w, "unit.spearman", 1, at(10000, 10000));
    let b = spawn(&mut w, "unit.citizen", 2, at(13000, 10000));
    attack(&mut w, a, b);
    w.step();
    assert!(w.entity(a).unwrap().pos.x > 10000);
    assert_eq!(hp(&w, b), 40);
    while hp(&w, b) == 40 && w.tick() < 120 {
        w.step();
    }
    assert_eq!(hp(&w, b), 31);
    ticks(&mut w, 29);
    assert_eq!(hp(&w, b), 31);
    w.step();
    assert_eq!(hp(&w, b), 22);
}

#[test]
fn damage_matrix_and_armor_apply() {
    let mut w = world("");
    let city = spawn(&mut w, "bld.city", 2, at(20000, 20000));
    let a = spawn(&mut w, "unit.spearman", 1, at(17000, 20000));
    let initial = hp(&w, city);
    attack(&mut w, a, city);
    w.step();
    assert_eq!(initial - hp(&w, city), 1); // floor(9 × 50 / 100) − 3.
}

#[test]
fn minimum_damage_is_one() {
    let mut w = world(
        "UPDATE entity_type SET armor=1000 WHERE id=(SELECT id FROM def WHERE key='bld.city');",
    );
    let city = spawn(&mut w, "bld.city", 2, at(20000, 20000));
    let a = spawn(&mut w, "unit.spearman", 1, at(17000, 20000));
    let initial = hp(&w, city);
    attack(&mut w, a, city);
    w.step();
    assert_eq!(initial - hp(&w, city), 1);
}

#[test]
fn ranged_building_attacks_enemy_in_range_only() {
    let mut w = world("UPDATE game_constant SET value=0 WHERE key='combat.attrition_per_period';");
    let city = spawn(&mut w, "bld.city", 1, at(20000, 20000));
    let near = spawn(&mut w, "unit.citizen", 2, at(30000, 20000));
    // Out of range but still inside the city's line of sight.
    let far = spawn(&mut w, "unit.citizen", 2, at(32000, 20000));
    attack(&mut w, city, far);
    ticks(&mut w, 50);
    assert_eq!(hp(&w, far), 40);
    attack(&mut w, city, near);
    w.step();
    assert_eq!(hp(&w, near), 32);
    assert_eq!(w.entity(city).unwrap().pos, at(20000, 20000));
}

#[test]
fn accuracy_roll_is_deterministic_and_can_miss() {
    let run = || {
        let mut w = world(
            "UPDATE et_weapon SET accuracy_pct=50, range_max=5000, cooldown_ticks=1; UPDATE entity_type SET hp=10000;",
        );
        let a = spawn(&mut w, "unit.spearman", 1, at(10000, 10000));
        let b = spawn(&mut w, "unit.citizen", 2, at(12000, 10000));
        attack(&mut w, a, b);
        ticks(&mut w, 100);
        let damage = 10000 - hp(&w, b);
        assert!(damage > 0 && damage < 900);
        assert_eq!(damage % 9, 0);
        w.hash()
    };
    assert_eq!(run(), run());
}

const PROJECTILE: &str = "UPDATE et_weapon SET range_max=10000, projectile_speed=1000, cooldown_ticks=1000 WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.spearman');";

#[test]
fn projectile_lands_after_travel_time() {
    let mut w = world(PROJECTILE);
    let a = spawn(&mut w, "unit.spearman", 1, at(10000, 10000));
    let b = spawn(&mut w, "unit.citizen", 2, at(12250, 10000)); // radius 250: two tiles to edge.
    attack(&mut w, a, b);
    w.step();
    assert_eq!(w.hits[0].land_tick, 40);
    assert_eq!(hp(&w, b), 40);
    w.despawn(a).unwrap();
    ticks(&mut w, 39);
    assert_eq!(hp(&w, b), 40);
    w.step();
    assert_eq!(hp(&w, b), 31);
}

#[test]
fn projectile_is_lost_when_target_gone() {
    let mut w = world(PROJECTILE);
    let a = spawn(&mut w, "unit.spearman", 1, at(10000, 10000));
    let b = spawn(&mut w, "unit.citizen", 2, at(12250, 10000));
    attack(&mut w, a, b);
    w.step();
    w.despawn(b).unwrap();
    let replacement = spawn(&mut w, "unit.citizen", 2, at(12250, 10000));
    ticks(&mut w, 50);
    assert_eq!(hp(&w, replacement), 40);
    assert!(w.hits.is_empty());
}

#[test]
fn splash_hits_neighbours_but_not_friends() {
    let mut w = world("UPDATE et_weapon SET range_max=10000, splash_radius=2000,splash_pct=50;");
    let a = spawn(&mut w, "unit.spearman", 1, at(10000, 10000));
    let b = spawn(&mut w, "unit.citizen", 2, at(15000, 10000));
    let neighbor = spawn(&mut w, "unit.spearman", 2, at(16000, 10000));
    let friend = spawn(&mut w, "unit.citizen", 3, at(15000, 11000));
    let far = spawn(&mut w, "unit.citizen", 2, at(18000, 10000));
    attack(&mut w, a, b);
    w.step();
    assert_eq!(hp(&w, b), 31);
    assert_eq!(hp(&w, neighbor), 97); // (9 − 2) × 50 / 100.
    assert_eq!(hp(&w, friend), 40);
    assert_eq!(hp(&w, far), 40);
}

#[test]
fn consumed_weapon_destroys_attacker() {
    let mut w = world("UPDATE et_weapon SET consumed=1,range_max=5000;");
    let a = spawn(&mut w, "unit.spearman", 1, at(10000, 10000));
    let b = spawn(&mut w, "unit.citizen", 2, at(12000, 10000));
    attack(&mut w, a, b);
    w.step();
    assert!(w.entity(a).is_none());
    assert_eq!(hp(&w, b), 31);
    assert_eq!(w.stats().losses[&PlayerId(1)], 1);
}

#[test]
fn weapon_cannot_hit_layer_without_target_row() {
    let mut w = world("UPDATE et_weapon SET range_max=10000;");
    let a = spawn(&mut w, "unit.spearman", 1, at(10000, 10000));
    let b = spawn(&mut w, "unit.repair_drone", 2, at(12000, 10000));
    attack(&mut w, a, b);
    ticks(&mut w, 100);
    assert_eq!(hp(&w, b), 45);
    assert!(w.entity(a).unwrap().cooldowns.is_empty());
}

#[test]
fn death_despawns_at_end_of_tick_and_mutual_kills_happen() {
    let mut w = world("UPDATE et_weapon SET range_max=5000;");
    let a = spawn(&mut w, "unit.spearman", 1, at(10000, 10000));
    let b = spawn(&mut w, "unit.spearman", 2, at(12000, 10000));
    injure(&mut w, a, 7);
    injure(&mut w, b, 7);
    attack(&mut w, a, b);
    attack(&mut w, b, a);
    w.step();
    assert!(w.entity(a).is_none() && w.entity(b).is_none());
    assert_eq!(w.stats().kills[&PlayerId(1)], 1);
    assert_eq!(w.stats().kills[&PlayerId(2)], 1);
}

#[test]
fn kill_frees_gather_slot_and_ends_orders_targeting_it() {
    let mut w = world("UPDATE et_weapon SET range_max=10000; UPDATE entity_type SET los=10000;");
    let farm = spawn(&mut w, "bld.farm", 2, at(20000, 20000));
    let worker = spawn(&mut w, "unit.citizen", 2, at(17000, 20000));
    let waiting = spawn(&mut w, "unit.citizen", 2, at(11000, 20000));
    w.submit(command(&w, worker, "harvest", Some(farm), None))
        .unwrap();
    ticks(&mut w, 50);
    assert_eq!(w.entity(worker).unwrap().work_site, Some(farm));
    w.submit(command(&w, waiting, "harvest", Some(farm), None))
        .unwrap();
    let a = spawn(&mut w, "unit.spearman", 1, at(14000, 20000));
    injure(&mut w, farm, 1);
    attack(&mut w, a, farm);
    w.step();
    assert!(w.entity(farm).is_none());
    assert_eq!(w.entity(worker).unwrap().work_site, None);
    assert!(w.entity(worker).unwrap().orders().is_empty());
    assert!(w.entity(waiting).unwrap().orders().is_empty());
    assert_eq!(w.stats().orders_target_gone, 2);
}

#[test]
fn plunder_is_paid_to_the_killer() {
    let mut w = world(
        "UPDATE et_weapon SET range_max=5000; UPDATE entity_type SET plunder_pct=25 WHERE id=(SELECT id FROM def WHERE key='unit.spearman');",
    );
    let a = spawn(&mut w, "unit.spearman", 1, at(10000, 10000));
    let b = spawn(&mut w, "unit.spearman", 2, at(12000, 10000));
    injure(&mut w, b, 1);
    attack(&mut w, a, b);
    w.step();
    let p = &w.players[&PlayerId(1)];
    assert_eq!(p.resources[&w.rules.id_of("res.food").unwrap()], 10);
    assert_eq!(p.resources[&w.rules.id_of("res.timber").unwrap()], 5);
}

#[test]
fn attack_rejected_on_friend_or_same_team() {
    let mut w = world("");
    let a = spawn(&mut w, "unit.spearman", 1, at(10000, 10000));
    for owner in [1, 3] {
        let b = spawn(&mut w, "unit.citizen", owner, at(13000, 10000));
        let before = w.hash();
        assert_eq!(
            w.submit(command(&w, a, "attack", Some(b), None)),
            Err(SimError::InvalidAttackTarget)
        );
        assert_eq!(w.hash(), before);
    }
}

#[test]
fn aggressive_acquires_and_chases() {
    let mut w = world("");
    let a = spawn(&mut w, "unit.spearman", 1, at(10000, 10000));
    let b = spawn(&mut w, "unit.citizen", 2, at(14000, 10000));
    stance(&mut w, a, "aggressive");
    ticks(&mut w, 10);
    assert_eq!(w.entity(a).unwrap().orders()[0].target_entity, Some(b));
    w.submit(command(&w, b, "move", None, Some(at(50000, 10000))))
        .unwrap();
    ticks(&mut w, 200);
    assert!(w.entity(a).unwrap().pos.x > 16000);
    assert_eq!(w.entity(a).unwrap().orders()[0].target_entity, Some(b));
}

#[test]
fn defensive_returns_home() {
    let mut w = world("");
    let home = at(10000, 10000);
    let a = spawn(&mut w, "unit.spearman", 1, home);
    let b = spawn(&mut w, "unit.citizen", 2, at(12500, 10000));
    stance(&mut w, a, "defensive");
    ticks(&mut w, 10);
    assert_eq!(w.entity(a).unwrap().orders()[0].home, Some(home));
    w.submit(command(&w, b, "move", None, Some(at(40000, 10000))))
        .unwrap();
    ticks(&mut w, 250);
    assert_eq!(w.entity(a).unwrap().pos, home);
    assert!(w.entity(a).unwrap().orders().is_empty());
}

#[test]
fn stand_ground_does_not_move() {
    let mut w = world("");
    let a = spawn(&mut w, "unit.spearman", 1, at(10000, 10000));
    let b = spawn(&mut w, "unit.citizen", 2, at(12000, 10000));
    stance(&mut w, a, "stand_ground");
    ticks(&mut w, 100);
    assert_eq!(w.entity(a).unwrap().pos, at(10000, 10000));
    assert_eq!(hp(&w, b), 40);
}

#[test]
fn hold_fire_never_attacks() {
    let mut w = world("UPDATE et_weapon SET range_max=10000;");
    let a = spawn(&mut w, "unit.spearman", 1, at(10000, 10000));
    let b = spawn(&mut w, "unit.citizen", 2, at(12000, 10000));
    ticks(&mut w, 100);
    assert_eq!(hp(&w, b), 40);
    assert!(w.entity(a).unwrap().orders().is_empty());
}

#[test]
fn raid_prefers_civilians() {
    let mut w = world("");
    let a = spawn(&mut w, "unit.spearman", 1, at(10000, 10000));
    spawn(&mut w, "unit.spearman", 2, at(12000, 10000));
    let civilian = spawn(&mut w, "unit.citizen", 2, at(15000, 10000));
    stance(&mut w, a, "raid");
    ticks(&mut w, 3);
    assert_eq!(
        w.entity(a).unwrap().orders()[0].target_entity,
        Some(civilian)
    );
}

#[test]
fn attack_move_fights_then_continues() {
    let mut w = world("");
    let a = spawn(&mut w, "unit.spearman", 1, at(10000, 10000));
    let b = spawn(&mut w, "unit.citizen", 2, at(13000, 10000));
    injure(&mut w, b, 9);
    w.submit(command(&w, a, "attack_move", None, Some(at(22000, 10000))))
        .unwrap();
    ticks(&mut w, 400);
    assert!(w.entity(b).is_none());
    assert!(w.entity(a).unwrap().pos.x >= 22000);
    assert!(w.entity(a).unwrap().orders().is_empty());
}

#[test]
fn idle_unit_retaliates_when_hit() {
    let mut w = world("UPDATE et_weapon SET range_max=20000; UPDATE entity_type SET los=20000;");
    let a = spawn(&mut w, "unit.spearman", 1, at(10000, 10000));
    let b = spawn(&mut w, "unit.spearman", 2, at(25000, 10000));
    stance(&mut w, b, "aggressive");
    attack(&mut w, a, b);
    w.step();
    w.step();
    assert_eq!(w.entity(b).unwrap().orders()[0].target_entity, Some(a));
    assert!(hp(&w, a) < 110);
}

#[test]
fn priest_heals_most_damaged_biological_in_range_only() {
    let mut w = world("");
    let h = spawn(&mut w, "unit.priest", 1, at(10000, 10000));
    let a = spawn(&mut w, "unit.citizen", 1, at(12000, 10000));
    let b = spawn(&mut w, "unit.spearman", 3, at(14000, 10000));
    let far = spawn(&mut w, "unit.spearman", 1, at(18000, 10000));
    let enemy = spawn(&mut w, "unit.spearman", 2, at(13000, 11000));
    injure(&mut w, h, 1);
    injure(&mut w, a, 35);
    injure(&mut w, b, 50);
    injure(&mut w, far, 1);
    injure(&mut w, enemy, 1);
    ticks(&mut w, 20);
    assert_eq!(hp(&w, b), 54);
    assert_eq!(hp(&w, a), 35);
    assert_eq!(hp(&w, far), 1);
    assert_eq!(hp(&w, h), 1);
    assert_eq!(hp(&w, enemy), 1);
}

#[test]
fn repair_drone_heals_mechanical_not_biological() {
    let mut w = world("");
    spawn(&mut w, "unit.repair_drone", 1, at(10000, 10000));
    let a = spawn(&mut w, "unit.citizen", 1, at(11000, 10000));
    let b = spawn(&mut w, "unit.repair_drone", 1, at(11500, 11000));
    injure(&mut w, a, 1);
    injure(&mut w, b, 10);
    ticks(&mut w, 20);
    assert_eq!(hp(&w, a), 1);
    assert_eq!(hp(&w, b), 18);
}

#[test]
fn medic_spends_energy_and_stops_at_zero() {
    let mut w =
        world("UPDATE et_energy SET max=2,regen_per_second=0; UPDATE et_healer SET auto=0;");
    let h = spawn(&mut w, "unit.medic", 1, at(10000, 10000));
    let a = spawn(&mut w, "unit.spearman", 1, at(11000, 10000));
    injure(&mut w, a, 1);
    w.submit(command(&w, h, "heal", Some(a), None)).unwrap();
    ticks(&mut w, 100);
    assert_eq!(hp(&w, a), 3);
    assert_eq!(w.entity(h).unwrap().energy, 0);
}

#[test]
fn energy_regenerates_to_max() {
    let mut w = world("");
    let h = spawn(&mut w, "unit.medic", 1, at(10000, 10000));
    assert_eq!(w.entity(h).unwrap().energy, 100);
    w.entities[h.0 as usize].as_mut().unwrap().energy = 0;
    ticks(&mut w, 9);
    assert_eq!(w.entity(h).unwrap().energy, 0);
    w.step();
    assert_eq!(w.entity(h).unwrap().energy, 1);
    ticks(&mut w, 1100);
    assert_eq!(w.entity(h).unwrap().energy, 100);
    assert_eq!(w.entity(h).unwrap().energy_remainder, 0);
}

#[test]
fn race_modifier_changes_max_hp_keeping_ratio() {
    let mut w = world(
        "INSERT INTO tech_effect(tech_id,effect,target_def_id) SELECT t.id,'change_faction',f.id FROM def t,def f WHERE t.key='tech.religion' AND f.key='race.reptilian';",
    );
    let a = spawn(&mut w, "unit.citizen", 1, at(10000, 10000));
    injure(&mut w, a, 20);
    w.complete_research(PlayerId(1), w.rules.id_of("tech.religion").unwrap());
    assert_eq!(w.entity(a).unwrap().max_hp, 46);
    assert_eq!(hp(&w, a), 23);
    let b = spawn(&mut w, "unit.spearman", 1, at(12000, 10000));
    assert_eq!(hp(&w, b), 115);
}

#[test]
fn nature_is_not_automatically_acquired_and_ties_use_geometry() {
    // Ties used to go to the lower id, which always favoured whoever spawned first.
    let mut w = world("");
    let a = spawn(&mut w, "unit.spearman", 1, at(20000, 20000));
    spawn(&mut w, "unit.citizen", 0, at(21000, 20000));
    let first = spawn(&mut w, "unit.citizen", 2, at(23000, 20000));
    let second = spawn(&mut w, "unit.citizen", 2, at(17000, 20000));
    stance(&mut w, a, "aggressive");
    // Both have the same relative rank; the final geometric tie uses position.
    assert!(first < second);
    w.step();
    assert_eq!(w.entity(a).unwrap().orders()[0].target_entity, Some(second));
}

#[test]
fn simultaneous_strikes_both_land() {
    // Without a range override both are melee: whoever updates first must not win.
    let mut w = world("");
    let a = spawn(&mut w, "unit.spearman", 1, at(10000, 10000));
    let b = spawn(&mut w, "unit.spearman", 2, at(10200, 10000));
    injure(&mut w, a, 7);
    injure(&mut w, b, 7);
    attack(&mut w, a, b);
    attack(&mut w, b, a);
    w.step();
    assert!(w.entity(a).is_none() && w.entity(b).is_none());
    assert_eq!(w.stats().kills[&PlayerId(1)], 1);
    assert_eq!(w.stats().kills[&PlayerId(2)], 1);
}

#[test]
fn simultaneous_attackers_do_not_cancel_after_an_earlier_lethal_hit() {
    let mut w = world("");
    let a = spawn(&mut w, "unit.spearman", 1, at(10000, 10000));
    let b = spawn(&mut w, "unit.spearman", 1, at(10400, 10000));
    let target = spawn(&mut w, "unit.spearman", 2, at(10200, 10000));
    injure(&mut w, target, 7);
    attack(&mut w, a, target);
    attack(&mut w, b, target);
    w.step();
    assert!(w.entity(target).is_none());
    assert!(!w.entity(a).unwrap().cooldowns.is_empty());
    assert_eq!(
        w.entity(a).unwrap().cooldowns,
        w.entity(b).unwrap().cooldowns
    );
}

#[test]
fn melee_pair_closing_in_strikes_on_the_same_tick() {
    // Approaching from equal distances, neither may get the first blow by update order.
    let mut w = world("");
    let a = spawn(&mut w, "unit.spearman", 1, at(10000, 10000));
    let b = spawn(&mut w, "unit.spearman", 2, at(14000, 10000));
    attack(&mut w, a, b);
    attack(&mut w, b, a);
    while hp(&w, a) == 110 && hp(&w, b) == 110 && w.tick() < 200 {
        w.step();
    }
    assert!(hp(&w, a) < 110);
    assert_eq!(hp(&w, a), hp(&w, b));
    ticks(&mut w, 600);
    assert!(w.entity(a).is_none() && w.entity(b).is_none());
}

#[test]
fn weapons_use_slot_order_and_skip_minimum_range() {
    let mut w = world("UPDATE et_weapon SET slot=2,range_max=10000,damage=20 WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.spearman');
        INSERT INTO et_weapon(entity_type_id,slot,damage,damage_class_id,range_min,range_max,cooldown_ticks)
        SELECT id,0,5,1,3000,10000,10 FROM def WHERE key='unit.spearman';
        INSERT INTO et_weapon_target SELECT id,'ground',100 FROM et_weapon WHERE slot=0 AND damage=5;");
    let a = spawn(&mut w, "unit.spearman", 1, at(10000, 10000));
    let b = spawn(&mut w, "unit.citizen", 2, at(15000, 10000));
    attack(&mut w, a, b);
    w.step();
    assert_eq!(hp(&w, b), 35);
    let c = spawn(&mut w, "unit.citizen", 2, at(12000, 10000));
    attack(&mut w, a, c);
    w.step();
    assert_eq!(hp(&w, c), 20);
}

#[test]
fn modifiers_apply_to_attack_armor_range_speed_los_and_heal_rate() {
    let mut w = world("INSERT INTO modifier(source_def_id,stat_key,op,value) SELECT id,'attack','add',3 FROM def WHERE key='nation.romans';
        INSERT INTO modifier(source_def_id,stat_key,op,value) SELECT id,'armor','add',1 FROM def WHERE key='nation.romans';
        INSERT INTO modifier(source_def_id,stat_key,op,value) SELECT id,'range','add',2000 FROM def WHERE key='nation.romans';
        INSERT INTO modifier(source_def_id,stat_key,op,value) SELECT id,'speed','pct',100 FROM def WHERE key='nation.romans';
        INSERT INTO modifier(source_def_id,stat_key,op,value) SELECT id,'los','pct',100 FROM def WHERE key='nation.romans';
        INSERT INTO modifier(source_def_id,stat_key,op,value) SELECT id,'heal_rate','pct',100 FROM def WHERE key='nation.romans';");
    let a = spawn(&mut w, "unit.spearman", 1, at(10000, 10000));
    let b = spawn(&mut w, "unit.citizen", 2, at(12000, 10000));
    attack(&mut w, a, b);
    w.step();
    assert_eq!(hp(&w, b), 29); // 9 + 3 − 1, at extended range.
    assert_eq!(w.entity(a).unwrap().los, 10000);
    w.submit(command(&w, a, "move", None, Some(at(20000, 10000))))
        .unwrap();
    w.step();
    assert_eq!(w.entity(a).unwrap().pos.x, 10125);
    let h = spawn(&mut w, "unit.priest", 1, at(20000, 20000));
    let t = spawn(&mut w, "unit.spearman", 1, at(21000, 20000));
    injure(&mut w, t, 50);
    ticks(&mut w, 20);
    assert_eq!(hp(&w, t), 58);
    assert_eq!(hp(&w, h), 35);
}

#[test]
fn raze_prefers_buildings_and_stance_change_cancels_automatic_orders() {
    let mut w = world("");
    let a = spawn(&mut w, "unit.spearman", 1, at(10000, 10000));
    spawn(&mut w, "unit.citizen", 2, at(12000, 10000));
    let farm = spawn(&mut w, "bld.farm", 2, at(14500, 10000));
    stance(&mut w, a, "raze");
    ticks(&mut w, 2);
    assert_eq!(w.entity(a).unwrap().orders()[0].target_entity, Some(farm));
    stance(&mut w, a, "hold_fire");
    assert!(w.entity(a).unwrap().orders().is_empty());
}

#[test]
fn healer_orders_hold_position_and_reject_invalid_patients() {
    let mut w = world("UPDATE et_healer SET auto=0;");
    let h = spawn(&mut w, "unit.priest", 1, at(10000, 10000));
    let t = spawn(&mut w, "unit.citizen", 1, at(18000, 10000));
    injure(&mut w, t, 10);
    for target in [
        h,
        spawn(&mut w, "unit.citizen", 2, at(12000, 10000)),
        spawn(&mut w, "unit.repair_drone", 1, at(12000, 12000)),
    ] {
        assert_eq!(
            w.submit(command(&w, h, "heal", Some(target), None)),
            Err(SimError::InvalidHealTarget)
        );
    }
    w.submit(command(&w, h, "heal", Some(t), None)).unwrap();
    ticks(&mut w, 40);
    assert_eq!(hp(&w, t), 10);
    assert_eq!(w.entity(h).unwrap().pos, at(10000, 10000));
    w.despawn(t).unwrap();
    w.step();
    assert!(w.entity(h).unwrap().orders().is_empty());
}

#[test]
fn hash_covers_combat_healing_projectiles_and_statistics() {
    fn changes(w: &mut World, change: impl FnOnce(&mut World)) {
        let old = w.hash();
        change(w);
        assert_ne!(old, w.hash());
    }
    let mut w = world(PROJECTILE);
    let a = spawn(&mut w, "unit.spearman", 1, at(10000, 10000));
    let b = spawn(&mut w, "unit.citizen", 2, at(15000, 10000));
    attack(&mut w, a, b);
    w.step();
    changes(&mut w, |w| {
        w.entities[a.0 as usize].as_mut().unwrap().max_hp += 1
    });
    changes(&mut w, |w| {
        w.entities[a.0 as usize].as_mut().unwrap().los += 1
    });
    changes(&mut w, |w| {
        w.entities[a.0 as usize].as_mut().unwrap().stance = w.default_stance()
    });
    changes(&mut w, |w| {
        *w.entities[a.0 as usize]
            .as_mut()
            .unwrap()
            .cooldowns
            .values_mut()
            .next()
            .unwrap() += 1
    });
    changes(&mut w, |w| {
        w.entities[a.0 as usize].as_mut().unwrap().energy += 1
    });
    changes(&mut w, |w| {
        w.entities[a.0 as usize].as_mut().unwrap().energy_remainder += 1
    });
    changes(&mut w, |w| {
        w.entities[a.0 as usize].as_mut().unwrap().heal_remainder += 1
    });
    changes(&mut w, |w| {
        w.entities[a.0 as usize].as_mut().unwrap().killer = Some(PlayerId(2))
    });
    changes(&mut w, |w| {
        w.entities[a.0 as usize].as_mut().unwrap().orders[0].automatic = true
    });
    changes(&mut w, |w| {
        w.entities[a.0 as usize].as_mut().unwrap().orders[0].home = Some(at(1, 2))
    });
    changes(&mut w, |w| w.hits[0].land_tick += 1);
    changes(&mut w, |w| w.hits[0].attacker = b);
    changes(&mut w, |w| w.hits[0].owner = PlayerId(2));
    changes(&mut w, |w| w.hits[0].source_type.0 += 1);
    changes(&mut w, |w| w.hits[0].weapon_id += 1);
    changes(&mut w, |w| w.hits[0].target = a);
    changes(&mut w, |w| w.hits[0].damage += 1);
    changes(&mut w, |w| w.seen.push(at(1, 2)));
    changes(&mut w, |w| w.seen[0].x += 1);
    changes(&mut w, |w| w.spent.push(a));
    changes(&mut w, |w| w.spent[0] = b);
    changes(&mut w, |w| {
        w.shots.push(super::PendingShot {
            origin: at(1, 2),
            target_pos: at(3, 4),
            accuracy: 50,
            hit: w.hits[0].clone(),
        })
    });
    changes(&mut w, |w| w.shots[0].origin.x += 1);
    changes(&mut w, |w| w.shots[0].target_pos.y += 1);
    changes(&mut w, |w| w.shots[0].accuracy += 1);
    changes(&mut w, |w| w.shots[0].hit.land_tick += 1);
    changes(&mut w, |w| w.shots[0].hit.attacker = a);
    changes(&mut w, |w| w.shots[0].hit.owner = PlayerId(1));
    changes(&mut w, |w| w.shots[0].hit.source_type.0 += 1);
    changes(&mut w, |w| w.shots[0].hit.weapon_id += 1);
    changes(&mut w, |w| w.shots[0].hit.target = b);
    changes(&mut w, |w| w.shots[0].hit.damage += 1);
    changes(&mut w, |w| {
        w.stats.kills.insert(PlayerId(1), 2);
    });
    changes(&mut w, |w| {
        w.stats.losses.insert(PlayerId(2), 2);
    });
}

#[test]
fn armed_buildings_acquire_automatically_and_release_out_of_range_targets() {
    let mut w = world("");
    let city = spawn(&mut w, "bld.city", 1, at(20000, 20000));
    let a = spawn(&mut w, "unit.citizen", 2, at(30000, 20000));
    ticks(&mut w, 2);
    assert_eq!(hp(&w, a), 32);
    w.submit(command(&w, a, "move", None, Some(at(45000, 20000))))
        .unwrap();
    ticks(&mut w, 30);
    assert!(w.entity(city).unwrap().orders().is_empty());
    let b = spawn(&mut w, "unit.citizen", 2, at(20000, 30000));
    ticks(&mut w, 20);
    assert!(hp(&w, b) < 40);
    assert_eq!(w.entity(city).unwrap().pos, at(20000, 20000));
}

#[test]
fn projectile_splash_uses_the_targets_position_when_it_lands() {
    let mut w = world(
        "UPDATE et_weapon SET range_max=10000, projectile_speed=1000, cooldown_ticks=1000, splash_radius=1500, splash_pct=100 WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.spearman');",
    );
    let a = spawn(&mut w, "unit.spearman", 1, at(10000, 10000));
    let b = spawn(&mut w, "unit.citizen", 2, at(12250, 10000));
    let old_neighbor = spawn(&mut w, "unit.citizen", 2, at(12250, 11000));
    let new_neighbor = spawn(&mut w, "unit.citizen", 2, at(15000, 10000));
    attack(&mut w, a, b);
    w.step();
    w.submit(command(&w, b, "move", None, Some(at(14850, 10000))))
        .unwrap();
    ticks(&mut w, 40);
    assert_eq!(hp(&w, b), 31);
    assert_eq!(hp(&w, old_neighbor), 40);
    assert_eq!(hp(&w, new_neighbor), 31);
}
