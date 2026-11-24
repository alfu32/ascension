//! Task 102: a projectile takes its flight time. The simulation computes the landing tick when
//! it fires (`distance × tick rate / projectile_speed`, data) and applies the damage then; a beam
//! still hits the tick it fires.

use rom_rules::{Projectile, Rules, build_rules_db};
use rom_sim::{Command, EntityId, PlayerId, Pos, World};
use std::sync::Arc;

fn rules() -> Arc<Rules> {
    let db = build_rules_db(
        include_str!("../../../db/schema.sql"),
        include_str!("../../../db/seed.sql"),
    )
    .unwrap();
    Arc::new(Rules::load(&db).unwrap())
}

fn stance(w: &mut World, id: EntityId, behavior: &str) {
    let def = w
        .rules()
        .stances
        .values()
        .find(|s| s.behavior == behavior)
        .unwrap()
        .id;
    w.set_stance(w.entity(id).unwrap().owner, id, def).unwrap();
}

fn hp(w: &World, id: EntityId) -> i64 {
    w.entity(id).map_or(0, |e| e.hp)
}

#[test]
fn every_travelling_projectile_has_a_speed_from_the_data() {
    let rules = rules();
    let mut flying = 0;
    for et in rules.entity_types.values() {
        for w in &et.weapons {
            let instant = matches!(
                w.projectile,
                Projectile::None | Projectile::Beam | Projectile::Railgun
            );
            if instant {
                assert_eq!(w.projectile_speed, 0, "weapon {} hits at once", w.id);
            } else {
                assert!(w.projectile_speed > 0, "weapon {} has no speed", w.id);
                flying += 1;
            }
        }
    }
    assert!(flying > 50, "{flying} weapons fly");
}

/// What one shot of `shooter` at `target`, `gap` milli-tiles apart centre to centre, did: the
/// tick it fired in, the tick it landed in, and the state hash some ticks later.
fn one_shot(rules: &Arc<Rules>, shooter: &str, target: &str, gap: i32) -> (u64, u64, u64) {
    let mut w = World::battle_scenario(Arc::clone(rules), 5, 0).unwrap();
    let a = w
        .spawn(
            rules.id_of(shooter).unwrap(),
            PlayerId(1),
            Pos::flat(30_000, 30_000),
        )
        .unwrap();
    let b = w
        .spawn(
            rules.id_of(target).unwrap(),
            PlayerId(2),
            Pos::flat(30_000 + gap, 30_000),
        )
        .unwrap();
    // Neither fires on its own: the shooter only at the order below.
    stance(&mut w, a, "hold_fire");
    stance(&mut w, b, "hold_fire");
    w.step();
    w.submit(Command {
        player: PlayerId(1),
        executor: a,
        kind: "attack".into(),
        subject: None,
        target_entity: Some(b),
        target_pos: None,
        queue: false,
        text: None,
    })
    .unwrap();
    let weapon = rules.entity_types[&rules.id_of(shooter).unwrap()].weapons[0].clone();
    let full = hp(&w, b);
    // The tick it fires in: the step after which the weapon is reloading.
    let fired = loop {
        assert!(w.tick() < 200, "{shooter} never fired");
        w.step();
        if w.entity(a).unwrap().cooldowns.contains_key(&weapon.id) {
            break w.tick() - 1;
        }
    };
    let land = if weapon.projectile_speed == 0 {
        // A hit at once lands the tick it fires: nothing is on its way.
        assert!(hp(&w, b) < full, "{shooter}: the beam struck at once");
        assert!(w.hits_in_flight().all(|h| h.attacker != a));
        fired
    } else {
        // Landing tick by the numbers, from the data: distance to the target's edge over the
        // weapon's speed, at least a tick.
        let radius = rules.entity_types[&rules.id_of(target).unwrap()].radius;
        let rate = rules.constant("sim.ticks_per_second").unwrap();
        let travel = ((i64::from(gap) - radius) * rate / weapon.projectile_speed).max(1) as u64;
        let hit = w
            .hits_in_flight()
            .find(|h| h.attacker == a)
            .expect("the hit is on its way");
        assert_eq!(hit.land_tick, fired + travel, "{shooter}");
        // Nothing lands before then: the target is untouched until the landing tick.
        while w.tick() < hit.land_tick {
            assert_eq!(hp(&w, b), full, "{shooter} at tick {}", w.tick());
            w.step();
        }
        assert_eq!(hp(&w, b), full);
        w.step();
        assert!(hp(&w, b) < full, "{shooter}: the shot landed");
        hit.land_tick
    };
    for _ in 0..20 {
        w.step();
    }
    (fired, land, w.hash())
}

#[test]
fn damage_lands_when_the_projectile_arrives_and_not_before() {
    let rules = rules();
    for (shooter, target, gap) in [
        // An arrow over four tiles, a lobbed howitzer shell over almost five, and a beam.
        ("unit.archer", "unit.spearman", 4_000),
        ("unit.howitzer", "unit.spearman", 4_800),
        ("unit.beam_trooper", "unit.spearman", 4_000),
    ] {
        let first = one_shot(&rules, shooter, target, gap);
        let (fired, land, _) = first;
        let instant =
            rules.entity_types[&rules.id_of(shooter).unwrap()].weapons[0].projectile_speed == 0;
        assert_eq!(land == fired, instant, "{shooter}: {fired} → {land}");
        // The same scenario again: the same ticks and the same state.
        assert_eq!(one_shot(&rules, shooter, target, gap), first, "{shooter}");
    }
}
