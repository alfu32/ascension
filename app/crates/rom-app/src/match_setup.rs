//! Match setup: `rom_match::setup`, shared with the server (task 85).

pub use rom_match::setup::*;

#[cfg(test)]
use crate::script::ScriptSetup;

/// A one-seed interstellar universe world with stock for player one, for tests.
#[cfg(test)]
pub(crate) fn universe_game_for_tests() -> (crate::terrain::UniverseContent, Match) {
    let options = crate::args::Options {
        universe: true,
        seed: 1,
        ..crate::args::Options::default()
    };
    let content = crate::terrain::generate_universe_content(&options).expect("universe");
    let setup = ScriptSetup {
        age: Some("age.expansion".into()),
        grants: vec![("res.metal".into(), 5000), ("res.energy".into(), 5000)],
        ..ScriptSetup::default()
    };
    let game = build_universe_match(
        &content.rules,
        &content.universe,
        &content.factions,
        1,
        None,
        &setup,
    )
    .expect("match");
    (content, game)
}
