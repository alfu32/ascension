//! The capitals the person knows, as the views draw them (task 131): from
//! [`World::known_capitals`], each at the level it is known and nothing finer. A capital known
//! by its system marks the system on the star map; by its planet, the planet in the system view
//! as well; by its city, the city in the world view and on the minimap as well.

use rom_render::{CapitalView, HudCapital};
use rom_sim::{BodyId, CapitalLevel, Elimination, Entity, EntityId, KnownCapital, PlayerId, World};

/// How far outside the city's economic radius the ring round a capital lies, in milli-tiles.
const RING_MARGIN: i64 = 1_500;

/// The capitals known at city level that stand on `body`, for the world view and the minimap;
/// `radius` gives a city's economic radius in milli-tiles (0 when unknown), which the ring
/// round it clears. A capital out of sight is remembered at its last known place.
pub fn surface_capitals(
    known: &[KnownCapital],
    body: BodyId,
    radius: impl Fn(EntityId) -> i64,
    fallback_radius: i64,
) -> Vec<HudCapital> {
    known
        .iter()
        .filter(|k| k.level == CapitalLevel::City)
        .filter_map(|k| {
            let (city, pos) = (k.city?, k.pos?);
            (pos.body == body).then(|| {
                let economic = radius(city);
                let economic = if economic > 0 {
                    economic
                } else {
                    fallback_radius
                };
                HudCapital {
                    city: city.0,
                    owner: owner_byte(k.owner),
                    face: pos.face,
                    x: pos.x,
                    y: pos.y,
                    ring: economic + RING_MARGIN,
                    remembered: !k.visible,
                }
            })
        })
        .collect()
}

/// Every known capital's system, and its planet from the planet level on, for the system view
/// and the star map.
pub fn space_capitals(known: &[KnownCapital]) -> Vec<CapitalView> {
    known
        .iter()
        .filter_map(|k| {
            Some(CapitalView {
                owner: owner_byte(k.owner),
                system: k.system?,
                body: k.body.map(|b| b.0),
            })
        })
        .collect()
}

/// [`surface_capitals`] of what `me` knows on `body`, ringed outside each city's radius.
pub fn shown_capitals(world: &World, me: PlayerId, body: BodyId) -> Vec<HudCapital> {
    let rules = world.rules();
    // A city no longer in the world keeps the widest base radius of a city type.
    let fallback = rules
        .entity_types
        .values()
        .filter_map(|et| et.territory.as_ref())
        .filter(|t| t.is_city)
        .map(|t| t.economic_radius)
        .max()
        .unwrap_or(0);
    surface_capitals(
        &world.known_capitals(me),
        body,
        |id| world.city_economic_radius(id),
        fallback,
    )
}

/// Whether `me` knows the city `id` as a capital: its own, an ally's, or an enemy's known at
/// city level.
pub fn is_known_capital(known: &[KnownCapital], id: EntityId) -> bool {
    known.iter().any(|k| k.city == Some(id))
}

/// What the help line says of a selected capital (task 131): that losing it can end the game,
/// when the match's elimination rule makes it so. `None` for any other entity, and under the
/// conquest rule, where a capital is a city like any other.
pub fn capital_help(world: &World, me: PlayerId, entity: &Entity) -> Option<String> {
    if !is_known_capital(&world.known_capitals(me), entity.id) {
        return None;
    }
    let sudden = match world.match_options().elimination {
        Elimination::Conquest => return None,
        Elimination::Capital => false,
        Elimination::SuddenDeath => true,
    };
    let lose = if sudden {
        "if an enemy takes it"
    } else {
        "if an enemy holds it long enough"
    };
    Some(if entity.owner == me {
        format!("Your capital: {lose}, you lose the game")
    } else if world.enemies(me, entity.owner) {
        let how = if sudden {
            "taking it"
        } else {
            "holding it long enough"
        };
        format!("Enemy capital: {how} defeats its owner")
    } else {
        format!("Allied capital: {lose}, its owner loses the game")
    })
}

fn owner_byte(owner: PlayerId) -> u8 {
    u8::try_from(owner.0).unwrap_or(u8::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rom_sim::Pos;

    const HOME: BodyId = BodyId(3);

    /// A capital of `owner` known at `level`: fields finer than the level stay empty, as
    /// [`World::known_capitals`] gives them.
    fn known(owner: u32, level: CapitalLevel, visible: bool) -> KnownCapital {
        let planet = level >= CapitalLevel::Planet;
        let city = level == CapitalLevel::City;
        KnownCapital {
            owner: PlayerId(owner),
            level,
            system: Some(7),
            body: planet.then_some(HOME),
            city: city.then_some(EntityId(40 + owner)),
            pos: city.then_some(Pos {
                x: 12_500,
                y: 30_000,
                face: 2,
                body: HOME,
            }),
            visible,
        }
    }

    fn marks(k: &[KnownCapital]) -> (Vec<HudCapital>, Vec<CapitalView>) {
        (
            surface_capitals(k, HOME, |_| 16_000, 16_000),
            space_capitals(k),
        )
    }

    #[test]
    fn a_capital_known_by_its_system_marks_only_the_system() {
        let (surface, space) = marks(&[known(2, CapitalLevel::System, false)]);
        assert!(surface.is_empty(), "no city, no minimap");
        assert_eq!(
            space,
            vec![CapitalView {
                owner: 2,
                system: 7,
                body: None
            }],
            "no planet"
        );
    }

    #[test]
    fn a_capital_known_by_its_planet_marks_the_system_and_the_planet() {
        let (surface, space) = marks(&[known(2, CapitalLevel::Planet, false)]);
        assert!(surface.is_empty(), "no city, no minimap");
        assert_eq!(
            space,
            vec![CapitalView {
                owner: 2,
                system: 7,
                body: Some(HOME.0)
            }]
        );
    }

    #[test]
    fn a_capital_known_by_its_city_marks_all_three_and_a_remembered_one_is_faded() {
        let (surface, space) = marks(&[
            known(1, CapitalLevel::City, true),
            known(2, CapitalLevel::City, false),
        ]);
        assert_eq!(space.len(), 2);
        assert!(space.iter().all(|c| c.body == Some(HOME.0)));
        assert_eq!(surface.len(), 2);
        let own = surface[0];
        assert_eq!((own.city, own.owner, own.face), (41, 1, 2));
        assert_eq!((own.x, own.y), (12_500, 30_000));
        assert!(!own.remembered, "in sight");
        assert!(surface[1].remembered, "out of sight: remembered");
        // The ring clears the city's economic radius.
        assert!(own.ring > 16_000);
    }

    #[test]
    fn a_capital_on_another_planet_is_not_drawn_on_this_one() {
        let k = [known(2, CapitalLevel::City, true)];
        assert!(surface_capitals(&k, BodyId(9), |_| 16_000, 16_000).is_empty());
        assert_eq!(
            space_capitals(&k).len(),
            1,
            "its system and planet still show"
        );
    }

    #[test]
    fn a_vanished_city_keeps_a_ring_of_the_fallback_radius() {
        let k = [known(2, CapitalLevel::City, false)];
        let surface = surface_capitals(&k, HOME, |_| 0, 16_000);
        assert_eq!(surface[0].ring, 16_000 + RING_MARGIN);
    }
}
