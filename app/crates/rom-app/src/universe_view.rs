//! The generated universe as the render thread draws it, and the zoom limit by age.

use rom_mapgen::Universe;
use rom_render::universe::{BodyView, BodyViewKind, SystemView, UniverseScene};
use rom_render::{Level, owner_color};
use rom_rules::{BodyKind, DefId, Rules};

/// Star colours, chosen by system id.
const STAR_COLOURS: [[u8; 3]; 5] = [
    [255, 244, 214],
    [255, 226, 170],
    [215, 228, 255],
    [255, 200, 150],
    [240, 240, 255],
];

/// The first age, by ordinal, whose map scale is not the smallest one (the planetary scale is
/// the one of lowest rank): the age in which the camera may leave the home planet.
pub fn space_age_ordinal(rules: &Rules) -> Option<i64> {
    let planetary = rules.map_scales.values().min_by_key(|s| s.rank)?;
    rules
        .ages
        .values()
        .filter(|age| age.map_scale != planetary.key)
        .map(|age| age.ordinal)
        .min()
}

/// The farthest level the camera may reach for a player in `age`: the globe before the space
/// age, the star map from it.
pub fn max_level(rules: &Rules, age: DefId) -> Level {
    match (space_age_ordinal(rules), rules.ages.get(&age)) {
        (Some(space), Some(age)) if age.ordinal >= space => Level::StarMap,
        _ => Level::Globe,
    }
}

/// The largest ground half-width, in milli-tiles, the camera of a player in `age` may show on the
/// home planet; 0 for the whole globe (and for an unknown age).
pub fn view_range(rules: &Rules, age: DefId) -> u32 {
    rules
        .ages
        .get(&age)
        .map_or(0, |age| u32::try_from(age.view_range).unwrap_or(0))
}

/// Converts a generated universe. `start_bodies` holds the home planet of each player in player
/// order; a home planet takes the colour of the first player living there.
pub fn to_universe_scene(universe: &Universe, start_bodies: &[u32]) -> UniverseScene {
    let colour_of = |body: u32| {
        start_bodies
            .iter()
            .position(|&b| b == body)
            .map(|player| owner_color(player as u8 + 1).map(|c| (c * 255.0).round() as u8))
    };
    UniverseScene {
        systems: universe
            .systems
            .iter()
            .map(|s| SystemView {
                id: s.id,
                name: s.name.clone(),
                x: s.x,
                y: s.y,
                star_colour: STAR_COLOURS[s.id as usize % STAR_COLOURS.len()],
            })
            .collect(),
        bodies: universe
            .bodies
            .iter()
            .map(|b| BodyView {
                id: b.id,
                system: b.system_id,
                kind: match b.kind {
                    BodyKind::Star => BodyViewKind::Star,
                    BodyKind::Habitable => BodyViewKind::Habitable,
                    BodyKind::Resource => BodyViewKind::Resource,
                },
                orbit_radius: b.orbit_radius,
                orbit_angle: b.orbit_angle_mdeg,
                n_or_size: b
                    .n
                    .unwrap_or_else(|| 1 + b.node_entity_type.map_or(0, |t| t.0 % 3)),
                home_race_colour: b.home_race.and_then(|_| colour_of(b.id)),
            })
            .collect(),
        routes: universe
            .routes
            .iter()
            .map(|r| (r.system_a, r.system_b))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::Options;
    use crate::terrain::{generate_universe_content, load_rules};

    #[test]
    fn space_age_is_found_by_data() {
        let rules = load_rules().expect("rules");
        let space = space_age_ordinal(&rules).expect("a space age");
        let planetary = rules.map_scales.values().min_by_key(|s| s.rank).unwrap();
        for age in rules.ages.values() {
            assert!(
                age.map_scale != planetary.key || age.ordinal < space,
                "{age:?}"
            );
            assert_eq!(
                max_level(&rules, age.id) == Level::StarMap,
                age.ordinal >= space
            );
        }
        let first = rules.ages.values().min_by_key(|a| a.ordinal).unwrap();
        assert_eq!(max_level(&rules, first.id), Level::Globe);
        assert_eq!(max_level(&rules, DefId(u32::MAX)), Level::Globe);
    }

    #[test]
    fn zoom_cap_follows_the_age() {
        let rules = load_rules().expect("rules");
        let by_ordinal = |n: i64| rules.ages.values().find(|a| a.ordinal == n).unwrap().id;
        let mut last = 0;
        for n in 0..=8 {
            let range = view_range(&rules, by_ordinal(n));
            assert!(range > last, "age {n} must see more than the one before");
            assert_eq!(max_level(&rules, by_ordinal(n)), Level::Globe);
            last = range;
        }
        // The whole globe is allowed from the age before space, and nothing is known of a stray id.
        assert_eq!(view_range(&rules, by_ordinal(9)), 0);
        assert_eq!(view_range(&rules, DefId(u32::MAX)), 0);
    }

    #[test]
    fn universe_scene_matches_the_generated_universe() {
        let options = Options {
            universe: true,
            seed: 1,
            ..Options::default()
        };
        let content = generate_universe_content(&options).expect("universe");
        let u = &content.universe;
        let bodies: Vec<u32> = u.starts.iter().map(|s| s.0).collect();
        let scene = to_universe_scene(u, &bodies);
        assert!(scene.is_valid());
        assert_eq!(scene.systems.len(), u.systems.len());
        assert_eq!(scene.bodies.len(), u.bodies.len());
        assert_eq!(scene.routes.len(), u.routes.len());
        for (view, system) in scene.systems.iter().zip(&u.systems) {
            assert_eq!(
                (view.id, &view.name, view.x, view.y),
                (system.id, &system.name, system.x, system.y)
            );
        }
        for (view, body) in scene.bodies.iter().zip(&u.bodies) {
            assert_eq!(
                (view.id, view.system, view.orbit_radius, view.orbit_angle),
                (
                    body.id,
                    body.system_id,
                    body.orbit_radius,
                    body.orbit_angle_mdeg
                )
            );
            assert_eq!(view.home_race_colour.is_some(), body.home_race.is_some());
        }
        // The first player's home planet is a habitable body with a generated surface.
        let home = scene.bodies.iter().find(|b| b.id == bodies[0]).unwrap();
        assert_eq!(home.kind, BodyViewKind::Habitable);
        assert!(u.planets.contains_key(&home.id));
        // The same options give the same scene.
        let again = generate_universe_content(&options).unwrap();
        assert_eq!(scene, to_universe_scene(&again.universe, &bodies));
    }
}
