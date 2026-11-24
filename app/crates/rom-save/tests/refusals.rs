//! A save is refused when it does not belong to the rules, or is damaged.

mod common;

use common::*;
use rom_save::SaveError;
use rom_sim::World;

fn small() -> (std::sync::Arc<rom_rules::Rules>, World) {
    let rules = rules();
    let mut w = World::battle_scenario(rules.clone(), 3, 4).unwrap();
    ticks(&mut w, 10);
    (rules, w)
}

#[test]
fn a_save_from_other_rules_is_refused() {
    let (rules, w) = small();
    let path = temp_path("mismatch");
    rom_save::save(&path, &w, &info(&w), &ai()).unwrap();
    let other =
        rules_with("UPDATE game_constant SET value = value + 1 WHERE key = 'combat.min_damage';");
    match rom_save::load(&path, &other) {
        Err(SaveError::RulesMismatch {
            saved_fingerprint,
            rules_fingerprint,
            ..
        }) => {
            assert_ne!(saved_fingerprint, rules_fingerprint);
        }
        other => panic!("expected a rules mismatch, got {:?}", other.map(|_| ())),
    }
    let message = rom_save::load(&path, &other).err().unwrap().to_string();
    assert!(message.contains("save was made with rules"), "{message}");
    // The same rules load it.
    rom_save::load(&path, &rules).unwrap();
    // A different rules version string is refused as well.
    let versioned = rules_with("UPDATE schema_meta SET value='9.9.9' WHERE key='rules_version';");
    assert!(matches!(
        rom_save::load(&path, &versioned),
        Err(SaveError::RulesMismatch { .. })
    ));
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn damaged_and_missing_files_are_errors_not_panics() {
    let (rules, w) = small();
    assert!(matches!(
        rom_save::load(&temp_path("absent"), &rules),
        Err(SaveError::Io(_))
    ));
    let path = temp_path("damaged");
    std::fs::write(&path, b"this is not a database at all, just text padding").unwrap();
    assert!(rom_save::load(&path, &rules).is_err());
    rom_save::save(&path, &w, &info(&w), &ai()).unwrap();
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.pragma_update(None, "foreign_keys", "OFF").unwrap();
        conn.execute("UPDATE entity SET entity_type_id = 999999 WHERE id = 1", [])
            .unwrap();
    }
    assert!(rom_save::load(&path, &rules).is_err());
    rom_save::save(&path, &w, &info(&w), &ai()).unwrap();
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.pragma_update(None, "foreign_keys", "OFF").unwrap();
        conn.execute("UPDATE surface SET terrain = x'ff'", [])
            .unwrap();
    }
    assert!(matches!(
        rom_save::load(&path, &rules),
        Err(SaveError::Corrupt(_))
    ));
    rom_save::save(&path, &w, &info(&w), &ai()).unwrap();
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.pragma_update(None, "foreign_keys", "OFF").unwrap();
        conn.execute("UPDATE match SET format = 99", []).unwrap();
    }
    assert!(matches!(
        rom_save::load(&path, &rules),
        Err(SaveError::UnsupportedFormat { found: 99 })
    ));
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn saving_replaces_an_existing_file_and_leaves_no_partial_file() {
    let (rules, mut w) = small();
    let path = temp_path("replace");
    rom_save::save(&path, &w, &info(&w), &ai()).unwrap();
    ticks(&mut w, 5);
    rom_save::save(&path, &w, &info(&w), &ai()).unwrap();
    let (loaded, _, _) = rom_save::load(&path, &rules).unwrap();
    assert_eq!(loaded.hash(), w.hash());
    let mut part = path.as_os_str().to_owned();
    part.push(".part");
    assert!(!std::path::Path::new(&part).exists());
    std::fs::remove_file(&path).unwrap();
}

/// Task 117: a format 13 save (before the match end, the counters and the history) still loads,
/// with no humans, no end, zero counters and no history.
#[test]
fn a_format_13_save_loads_without_the_report_tables() {
    let rules = rules();
    let mut w = World::battle_scenario(rules.clone(), 3, 4).unwrap();
    w.set_humans(&[P1]).unwrap();
    ticks(&mut w, 30);
    assert!(w.tally(P1).is_some());
    let path = temp_path("format13");
    rom_save::save(&path, &w, &info(&w), &ai()).unwrap();
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch(
            "DROP TABLE match_human; DROP TABLE match_end; DROP TABLE player_stat;
             DROP TABLE player_gathered; DROP TABLE player_age_reached; DROP TABLE player_history;
             UPDATE match SET format = 13;",
        )
        .unwrap();
    }
    let (old, _, _) = rom_save::load(&path, &rules).unwrap();
    assert_eq!(old.tick(), w.tick());
    assert_eq!(old.entities().count(), w.entities().count());
    assert_eq!(old.humans().count(), 0);
    assert_eq!(old.match_end(), None);
    assert!(old.history().is_empty());
    assert_eq!(
        old.tally(P1).cloned().unwrap_or_default(),
        Default::default()
    );
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute("UPDATE match SET format = 12", []).unwrap();
    }
    assert!(matches!(
        rom_save::load(&path, &rules),
        Err(SaveError::UnsupportedFormat { found: 12 })
    ));
    std::fs::remove_file(&path).unwrap();
}

/// Task 144: a save keeps the cities' names, a renamed one included; a format 19 save (before
/// city names) loads with every city named from its owner's list in id order.
#[test]
fn city_names_are_saved_and_a_format_19_save_names_its_cities() {
    let rules = rules();
    let mut w = World::wealth_scenario(rules.clone(), 1, true).unwrap();
    ticks(&mut w, 10);
    let listed: Vec<(rom_sim::EntityId, String)> = w
        .city_names()
        .map(|(id, name)| (id, name.to_owned()))
        .collect();
    assert!(listed.len() >= 2, "{listed:?}");
    let (city, _) = listed[0];
    w.submit(rom_sim::Command {
        text: Some("Caput Mundi".into()),
        ..command(&w, city, rom_sim::RENAME_CITY)
    })
    .unwrap();
    let path = temp_path("format19");
    rom_save::save(&path, &w, &info(&w), &ai()).unwrap();
    let (loaded, _, _) = rom_save::load(&path, &rules).unwrap();
    assert_eq!(loaded.city_name(city), Some("Caput Mundi"));
    assert_eq!(loaded.hash(), w.hash());
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch("ALTER TABLE city DROP COLUMN name; UPDATE match SET format = 19;")
            .unwrap();
    }
    let (old, _, _) = rom_save::load(&path, &rules).unwrap();
    let named: Vec<(rom_sim::EntityId, String)> = old
        .city_names()
        .map(|(id, name)| (id, name.to_owned()))
        .collect();
    assert_eq!(named, listed);
    std::fs::remove_file(&path).unwrap();
}
