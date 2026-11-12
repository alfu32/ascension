// TEMPORARY instrumentation; deleted before commit.
use rom_rules::{Rules, build_rules_db};
use rom_sim::{Command, MapGrid, Player, PlayerId, Pos, World};
use std::{collections::BTreeMap, sync::Arc};

fn rules() -> Arc<Rules> {
    let db = build_rules_db(
        include_str!("../../../db/schema.sql"),
        include_str!("../../../db/seed.sql"),
    )
    .unwrap();
    Arc::new(Rules::load(&db).unwrap())
}

/// left_owner owns the x=33000 line; `left_first` spawns that line first.
fn battle(rules: Arc<Rules>, seed: u64, per: u32, left_owner: u32, left_first: bool) -> World {
    let et = rules
        .entity_types
        .values()
        .find(|e| e.movement.is_some() && !e.weapons.is_empty())
        .unwrap();
    let type_id = et.id;
    let terrain = rules.id_of("ter.plain").unwrap();
    let faction = *rules.factions.keys().next().unwrap();
    let age = *rules.ages.keys().next().unwrap();
    let players = (1..=2)
        .map(|n| Player {
            id: PlayerId(n),
            team: n,
            faction,
            age,
            resources: BTreeMap::new(),
            techs: BTreeMap::new(),
        })
        .collect();
    let mut w = World::new(
        rules.clone(),
        MapGrid::new(96, 96, vec![terrain; 96 * 96]).unwrap(),
        seed,
        players,
    )
    .unwrap();
    let order: [bool; 2] = if left_first { [true, false] } else { [false, true] };
    for left in order {
        let owner = if left { left_owner } else { 3 - left_owner };
        let (x, target) = if left { (33000, 63000) } else { (63000, 33000) };
        for n in 0..per {
            let y = (1000 + u64::from(n) * 94000 / u64::from(per)) as i32;
            let id = w.spawn(type_id, PlayerId(owner), Pos { x, y }).unwrap();
            w.submit(Command {
                player: PlayerId(owner),
                executor: id,
                kind: "attack_move".into(),
                subject: None,
                target_entity: None,
                target_pos: Some(Pos { x: target, y }),
                queue: false,
            })
            .unwrap();
        }
    }
    w
}

fn report(label: &str, w: &mut World, left_owner: u32) {
    let mut first_hit = [None, None];
    let mut prev = [0i64; 2];
    for t in 0..2400 {
        w.step();
        let mut hp = [0i64; 2];
        for e in w.entities() {
            let left = e.owner == PlayerId(left_owner);
            hp[usize::from(!left)] += e.hp;
        }
        for s in 0..2 {
            if t > 0 && hp[s] < prev[s] && first_hit[s].is_none() {
                first_hit[s] = Some(t);
            }
        }
        if t % 100 == 0 || t == 2399 {
            let alive = |left: bool| {
                w.entities()
                    .filter(|e| (e.owner == PlayerId(left_owner)) == left)
                    .count()
            };
            let mean_x = |left: bool| {
                let v: Vec<_> = w
                    .entities()
                    .filter(|e| (e.owner == PlayerId(left_owner)) == left)
                    .map(|e| i64::from(e.pos.x))
                    .collect();
                if v.is_empty() { 0 } else { v.iter().sum::<i64>() / v.len() as i64 }
            };
            // distinct targets per side
            let targets = |left: bool| {
                let mut s = std::collections::BTreeSet::new();
                let mut engaged = 0;
                for e in w.entities().filter(|e| (e.owner == PlayerId(left_owner)) == left) {
                    if let Some(t) = e.orders().front().and_then(|o| o.target_entity) {
                        s.insert(t);
                        engaged += 1;
                    }
                }
                (engaged, s.len())
            };
            eprintln!(
                "{label} t={t} alive L={} R={} hp L={} R={} meanx L={} R={} tgt L={:?} R={:?}",
                alive(true),
                alive(false),
                hp[0],
                hp[1],
                mean_x(true),
                mean_x(false),
                targets(true),
                targets(false)
            );
        }
        prev = hp;
    }
    eprintln!("{label} first damage taken L={:?} R={:?}", first_hit[0], first_hit[1]);
}

fn mirror(w: &mut World, per: u32) {
    let mut reported = 0;
    for t in 0..1200 {
        w.step();
        for n in 0..per {
            let l = w.entity(rom_sim::EntityId(n + 1));
            let r = w.entity(rom_sim::EntityId(per + n + 1));
            let bad = match (l, r) {
                (Some(l), Some(r)) => {
                    (l.pos.x + r.pos.x != 96000 || l.pos.y != r.pos.y || l.hp != r.hp)
                        .then(|| format!("L=({},{} hp{} tgt{:?} cd{:?}) R=({},{} hp{} tgt{:?} cd{:?})", l.pos.x, l.pos.y, l.hp, l.orders().front().and_then(|o| o.target_entity), l.cooldowns, r.pos.x, r.pos.y, r.hp, r.orders().front().and_then(|o| o.target_entity), r.cooldowns))
                }
                (None, None) => None,
                _ => Some(format!("one alive l={} r={}", l.is_some(), r.is_some())),
            };
            if let Some(b) = bad {
                if reported < 12 {
                    eprintln!("mirror break t={t} row={n}: {b}");
                }
                reported += 1;
            }
        }
        if reported >= 12 {
            return;
        }
    }
}

#[test]
fn probe_mirror() {
    let r = rules();
    let per = std::env::var("PER").ok().and_then(|v| v.parse().ok()).unwrap_or(200);
    let mut w = battle(r, 3, per, 1, true);
    mirror(&mut w, per);
}

#[test]
fn probe() {
    let r = rules();
    let per = std::env::var("PER").ok().and_then(|v| v.parse().ok()).unwrap_or(200);
    let seed = std::env::var("SEED").ok().and_then(|v| v.parse().ok()).unwrap_or(3);
    for (label, lo, lf) in [
        ("base", 1, true),
        ("swap-players", 2, true),
        ("swap-spawn", 1, false),
    ] {
        let mut w = battle(r.clone(), seed, per, lo, lf);
        report(label, &mut w, lo);
    }
}
