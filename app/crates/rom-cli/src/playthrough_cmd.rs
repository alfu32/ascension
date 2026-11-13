//! The order-driven full-content audit.

use rom_rules::{EntityClass, Rules};
use rom_sim::{PlayerId, Playthrough};
use std::{path::PathBuf, sync::Arc, time::Instant};

/// Runs the audit with the arguments following `playthrough`.
pub fn run(mut args: impl Iterator<Item = String>) -> Result<(), Box<dyn std::error::Error>> {
    let (mut db, mut seed) = (None, None);
    while let Some(flag) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_str() {
            "--db" => db = Some(PathBuf::from(value)),
            "--seed" => seed = Some(value.parse::<u64>()?),
            _ => return Err(format!("unknown argument: {flag}").into()),
        }
    }
    let start = Instant::now();
    let conn = rusqlite::Connection::open_with_flags(
        db.ok_or("missing --db")?,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;
    let rules = Arc::new(Rules::load(&conn)?);
    let age = rules
        .ages
        .values()
        .min_by_key(|a| (a.ordinal, a.id))
        .ok_or("missing ages")?
        .id;
    let faction = rules
        .playable_start_factions(age)
        .into_iter()
        .next()
        .ok_or("missing starting faction")?;
    let style = rules
        .map_styles
        .values()
        .find(|s| s.planets == 1)
        .ok_or("missing one-world style")?
        .id;
    let seed = seed.ok_or("missing --seed")?;
    let map = rom_mapgen::generate(
        &rules,
        &rom_mapgen::MapRequest::uniform(style, seed, 128, 128, 1, faction),
    )?;
    let nodes: Vec<_> = map.nodes.iter().map(|n| (n.entity_type, n.pos)).collect();
    let mut audit = Playthrough::new(rules.clone(), map.grid, map.starts[0], &nodes, seed)?;
    let final_age = rules
        .ages
        .values()
        .max_by_key(|a| (a.ordinal, a.id))
        .ok_or("missing final age")?
        .id;
    let mut printed = 0;
    loop {
        for (age, tick) in &audit.ages()[printed..] {
            println!("age {} tick={tick}", rules.key(*age));
        }
        printed = audit.ages().len();
        let player = audit
            .world()
            .players()
            .find(|p| p.id == PlayerId(1))
            .expect("audit player");
        if (player.age == final_age && audit.is_complete()) || audit.world().tick() >= 400_000 {
            break;
        }
        audit.step();
    }
    let world = audit.world();
    let player = world
        .players()
        .find(|p| p.id == PlayerId(1))
        .expect("audit player");
    let count = |class| {
        world
            .entities()
            .filter(|e| e.owner == player.id && rules.entity_types[&e.type_id].class == class)
            .count()
    };
    println!(
        "playthrough final_age={} buildings={} units={} techs={} hash={:016x}",
        rules.key(player.age),
        count(EntityClass::Building),
        count(EntityClass::Unit),
        player.techs.len(),
        audit.hash()
    );
    eprintln!(
        "playthrough ticks={} elapsed={:.3}s",
        world.tick(),
        start.elapsed().as_secs_f64()
    );
    if player.age != final_age || !audit.is_complete() {
        return Err("playthrough did not complete all content within 400000 ticks".into());
    }
    Ok(())
}
