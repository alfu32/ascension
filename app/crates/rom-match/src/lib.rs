//! One match core for the app and the server (task 85).
//!
//! Everything about a match that is not drawing: the rules built from the embedded data, the
//! generated map, planet or universe, the match setup, the computer players, and the tick
//! routine. A **leader** ([`MatchCore::lead`]) runs the match and returns, for every tick, the
//! [`Journal`] of what changed the world before the step; a **follower**
//! ([`MatchCore::follow`]) applies a journal and steps, and so stays identical to the leader
//! tick for tick (`app/docs/network.md`).

pub mod computer;
pub mod content;
pub mod core;
pub mod script_setup;
pub mod settings;
pub mod setup;

pub use computer::{ComputerPlayers, DEFAULT_SPEC, Spec, TickReport, load_match, save_match};
pub use content::{
    Content, ContentRequest, SCHEMA, SEED, SphereContent, UniverseContent, generate_content,
    generate_sphere_content, generate_universe_content, load_rules, model_files, seed_with_models,
};
pub use core::{Desync, Entry, Input, Journal, MatchCore, Refusal, TickOutcome, VISIT_INTERVAL};
pub use script_setup::ScriptSetup;
pub use settings::{Built, Generated, MapKind, MatchSettings};
pub use setup::{
    BUILDERS_PER_PLAYER, Match, NEUTRAL, PLAYER_ONE, PlayerStart, START_RESOURCES, SetupError,
    build_match, build_match_in_age, build_sphere_match, build_sphere_match_in_age,
    build_universe_match, builder_type, city_type, default_factions, planet_surface, start_age,
    universe_setup,
};

/// A stable hash of the embedded schema, seed and model files the rules are built from: two
/// builds play together only when it is the same (FNV-1a, 64 bits).
pub fn rules_fingerprint() -> u64 {
    static FINGERPRINT: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
    *FINGERPRINT.get_or_init(|| {
        let mut hash = Fnv::default();
        hash.write(SCHEMA.as_bytes());
        // A separator, so moving text between the files changes the hash.
        hash.write(&[0]);
        hash.write(seed_with_models().as_bytes());
        hash.0
    })
}

/// [`rules_fingerprint`] as sixteen hexadecimal digits.
pub fn rules_fingerprint_hex() -> String {
    format!("{:016x}", rules_fingerprint())
}

/// FNV-1a, 64 bits.
struct Fnv(u64);

impl Default for Fnv {
    fn default() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }
}

impl Fnv {
    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(0x0100_0000_01b3);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fingerprint_is_stable_and_covers_the_data() {
        assert_eq!(rules_fingerprint(), rules_fingerprint());
        assert_eq!(rules_fingerprint_hex().len(), 16);
        // FNV-1a's published test vector, so the hash is the standard one.
        let mut fnv = Fnv::default();
        fnv.write(b"a");
        assert_eq!(fnv.0, 0xaf63_dc4c_8601_ec8c);
        let mut other = Fnv::default();
        other.write(SCHEMA.as_bytes());
        other.write(&[0]);
        other.write(SEED.as_bytes());
        assert_ne!(other.0, rules_fingerprint(), "the model files count");
    }
}
