//! Alerts as the person sees them (task 118): the event line ("Your capital is under attack",
//! "Foreign ships entered the Sol system: 3 ships of Romans (2)"), the pulsing marker on the
//! minimap and on the star map, the ships an arrival reveals, and what Backspace flies to.

use std::collections::BTreeMap;

use rom_render::{AlertMark, AlertView, HudAlert, HudSnapshot};
use rom_rules::{BodyKind, EntityClass};
use rom_sim::{Alert, AlertKind, BodyId, Entity, EntityId, PlayerId, World};

use crate::space_view::{Notice, SpaceFrame};

/// Ticks an alert's marker lasts: the rule `alert.marker_ticks`.
pub fn marker_ticks(world: &World) -> u64 {
    world
        .rules()
        .constant("alert.marker_ticks")
        .unwrap_or(0)
        .max(0) as u64
}

/// The alerts of `me` raised after tick `after`, as event lines, oldest first.
pub fn alert_notices(world: &World, me: PlayerId, after: u64) -> Vec<Notice> {
    world
        .alerts_of(me)
        .filter(|a| a.tick > after)
        .map(|alert| Notice {
            text: alert_text(world, me, alert),
            about: alert_about(world, alert),
        })
        .collect()
}

/// The tick of `me`'s newest alert, or `after` when there is none newer.
pub fn latest_alert_tick(world: &World, me: PlayerId, after: u64) -> u64 {
    world.alerts_of(me).map(|a| a.tick).fold(after, u64::max)
}

/// What an alert flies the camera to: an arriving ship still there, the victim while it lives,
/// else the city it was told by.
pub fn alert_about(world: &World, alert: &Alert) -> Option<EntityId> {
    match &alert.kind {
        AlertKind::Attack { victim, city, .. } => [Some(*victim), *city]
            .into_iter()
            .flatten()
            .find(|id| world.entity(*id).is_some()),
        AlertKind::Arrival { ships, .. } => {
            ships.iter().copied().find(|id| world.entity(*id).is_some())
        }
    }
}

/// The words of an alert for `me`.
pub fn alert_text(world: &World, me: PlayerId, alert: &Alert) -> String {
    match &alert.kind {
        AlertKind::Attack {
            city,
            field,
            victim,
            ..
        } => attack_text(world, me, alert, *city, *field, *victim),
        AlertKind::Arrival {
            system,
            owner,
            ships,
        } => {
            let count = match ships.len() {
                1 => "1 ship".to_owned(),
                n => format!("{n} ships"),
            };
            format!(
                "Foreign ships entered the {} system: {count} of {}",
                system_name(world, *system),
                crate::ascension_view::player_name(world, me, *owner)
            )
        }
    }
}

fn attack_text(
    world: &World,
    me: PlayerId,
    alert: &Alert,
    city: Option<EntityId>,
    field: bool,
    victim: EntityId,
) -> String {
    let body = alert.pos.body;
    // Off the home world the planet is named.
    let planet = world
        .body(body)
        .filter(|b| b.kind != BodyKind::Star && world.home_body(me) != Some(body))
        .map(|b| format!(" on {}", b.name))
        .unwrap_or_default();
    let buildings = world
        .entity(victim)
        .map(|e| world.rules().entity_types[&e.type_id].class == EntityClass::Building)
        .unwrap_or(false);
    let who = if buildings {
        "Your buildings are"
    } else {
        "Your army is"
    };
    match city {
        Some(city) => {
            let place = format!("{}{planet}", city_label(world, me, city));
            if field {
                format!("{who} under attack near {}", lower_first(&place))
            } else {
                format!("{place} is under attack")
            }
        }
        None => match world.body(body) {
            Some(b) if b.kind == BodyKind::Star => format!(
                "Your ships are under attack in the {} system",
                system_name(world, b.system)
            ),
            Some(b) => format!("{who} under attack on {}", b.name),
            None => format!("{who} under attack"),
        },
    }
}

/// "Your capital", or "Your city 3": the cities of `me` counted in the order they were founded.
pub fn city_label(world: &World, me: PlayerId, city: EntityId) -> String {
    if world.capital(me) == Some(city) {
        return "Your capital".to_owned();
    }
    let rank = world
        .entities()
        .filter(|e| e.owner == me && world.is_city_type(e.type_id))
        .position(|e| e.id == city);
    match rank {
        Some(i) => format!("Your city {}", i + 1),
        None => "Your city".to_owned(),
    }
}

fn lower_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(c) => c.to_lowercase().chain(chars).collect(),
        None => String::new(),
    }
}

fn system_name(world: &World, system: u32) -> String {
    world
        .systems()
        .iter()
        .find(|s| s.id == system)
        .map_or_else(|| format!("system {system}"), |s| s.name.clone())
}

/// Fills the alert markers of the surface `body` and what Backspace flies to.
pub fn fill_hud(world: &World, me: PlayerId, body: BodyId, hud: &mut HudSnapshot) {
    let life = marker_ticks(world);
    let fresh = |a: &&Alert| world.tick().saturating_sub(a.tick) <= life;
    hud.alerts = world
        .alerts_of(me)
        .filter(fresh)
        .filter(|a| a.pos.body == body)
        .map(|a| HudAlert {
            face: a.pos.face,
            x: a.pos.x,
            y: a.pos.y,
            age_ticks: world.tick().saturating_sub(a.tick) as u32,
            life_ticks: life as u32,
            kind: mark_of(a),
        })
        .collect();
    hud.alert_about = world
        .alerts_of(me)
        .filter(fresh)
        .filter_map(|a| alert_about(world, a))
        .last()
        .map(|id| id.0);
}

fn mark_of(alert: &Alert) -> AlertMark {
    match alert.kind {
        AlertKind::Attack { .. } => AlertMark::Attack,
        AlertKind::Arrival { .. } => AlertMark::Arrival,
    }
}

/// `me`'s fresh alerts as the space views draw them: at the planet for an attack on a planet's
/// surface, at the point of the system's plane otherwise.
pub fn space_alerts(
    world: &World,
    me: PlayerId,
    planes: &BTreeMap<BodyId, (u32, SpaceFrame)>,
) -> Vec<AlertView> {
    let life = marker_ticks(world);
    world
        .alerts_of(me)
        .filter(|a| world.tick().saturating_sub(a.tick) <= life)
        .filter_map(|a| {
            let (system, body, (x, y)) = match planes.get(&a.pos.body) {
                Some((system, frame)) => (*system, None, frame.units_of(a.pos)),
                None => {
                    let info = world.body(a.pos.body)?;
                    (info.system, Some(info.id.0), (0.0, 0.0))
                }
            };
            Some(AlertView {
                system,
                body,
                x,
                y,
                age_ticks: world.tick().saturating_sub(a.tick) as u32,
                life_ticks: life as u32,
                kind: mark_of(a),
            })
        })
        .collect()
}

/// Foreign ships `me`'s detectors saw jump in lately: shown on the star map and in the system
/// for `alert.marker_ticks` though they are out of sight, while they stay in a system's plane.
pub fn revealed_ships<'w>(
    world: &'w World,
    me: PlayerId,
    planes: &BTreeMap<BodyId, (u32, SpaceFrame)>,
) -> Vec<&'w Entity> {
    let life = marker_ticks(world);
    let mut out: Vec<&Entity> = world
        .alerts_of(me)
        .filter(|a| world.tick().saturating_sub(a.tick) <= life)
        .filter_map(|a| match &a.kind {
            AlertKind::Arrival { ships, .. } => Some(ships),
            AlertKind::Attack { .. } => None,
        })
        .flatten()
        .filter_map(|id| world.entity(*id))
        .filter(|e| planes.contains_key(&e.pos.body))
        .collect();
    out.sort_by_key(|e| e.id);
    out.dedup_by_key(|e| e.id);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::selection::PLAYER;
    use crate::space_view::{Notices, SystemIndex, space_snapshot_with};
    use rom_sim::Command;

    fn foe(world: &World) -> PlayerId {
        world
            .players()
            .map(|p| p.id)
            .find(|p| world.enemies(PLAYER, *p) && *p != PlayerId(0))
            .unwrap()
    }

    /// Enemy marines beside player one's capital: the event line says the capital is under
    /// attack, once; the minimap marks the place; Backspace flies there; the star map marks the
    /// system.
    #[test]
    fn an_attack_on_the_capital_is_told_once_and_marked() {
        let (_content, mut game) = crate::match_setup::universe_game_for_tests();
        let world = &mut game.world;
        let rules = world.rules().clone();
        let foe = foe(world);
        let home = world.home_body(PLAYER).unwrap();
        let capital = world.capital(PLAYER).unwrap();
        let at = world.entity(capital).unwrap().pos;
        let marine = rules.id_of("unit.marine").unwrap();
        let mut notices = Notices::default();
        notices.update(world);
        for (dx, dy) in [(5_000, 0), (-5_000, 0), (0, 5_000)] {
            let unit = world.spawn(marine, foe, world.offset(at, dx, dy)).unwrap();
            world
                .submit(Command {
                    player: foe,
                    executor: unit,
                    kind: "attack_move".into(),
                    subject: None,
                    target_entity: None,
                    target_pos: Some(at),
                    queue: false,
                })
                .unwrap();
        }
        let mut lines = Vec::new();
        for _ in 0..200 {
            world.step();
            lines.extend(notices.update(world));
        }
        let told: Vec<_> = lines
            .iter()
            .filter(|n| n.text.contains("under attack"))
            .collect();
        assert_eq!(told.len(), 1, "{lines:?}");
        assert_eq!(told[0].text, "Your capital is under attack");
        assert!(told[0].about.is_some());
        let mut hud = HudSnapshot::default();
        fill_hud(world, PLAYER, home, &mut hud);
        assert_eq!(hud.alerts.len(), 1);
        assert_eq!(hud.alerts[0].kind, AlertMark::Attack);
        assert!(hud.alert_about.is_some());
        let system = world.body(home).unwrap().system;
        let sightings = world.visible_entities(PLAYER);
        let space =
            space_snapshot_with(PLAYER, world, &SystemIndex::new(world), system, &sightings);
        assert!(
            space
                .alerts
                .iter()
                .any(|a| a.system == system && a.body == Some(home.0)),
            "{:?}",
            space.alerts
        );
        // Past the marker's life the markers are gone.
        for _ in 0..=marker_ticks(world) {
            world.step();
        }
        let mut hud = HudSnapshot::default();
        fill_hud(world, PLAYER, home, &mut hud);
        assert!(hud.alerts.is_empty(), "{:?}", hud.alerts);
    }

    /// A detector of player one on its home planet sees two enemy corvettes jump into the home
    /// system: one line names the system, the count and the owner; the ships are shown in the
    /// system though nothing of player one's sees them; Backspace flies to one of them.
    #[test]
    fn a_detector_tells_of_ships_jumping_in_and_shows_them() {
        let (content, mut game) = crate::match_setup::universe_game_for_tests();
        let world = &mut game.world;
        let rules = world.rules().clone();
        let id = |k: &str| rules.id_of(k).unwrap();
        let foe = foe(world);
        let home = world.home_body(PLAYER).unwrap();
        let system = world.body(home).unwrap().system;
        let city = world.entity(world.capital(PLAYER).unwrap()).unwrap().pos;
        [(12_000, 0), (0, 12_000), (-12_000, 0), (0, -12_000)]
            .into_iter()
            .find_map(|(dx, dy)| {
                world
                    .place_for_setup(id("bld.jump_detector"), PLAYER, world.offset(city, dx, dy))
                    .ok()
            })
            .expect("room for the detector");
        let other = content
            .universe
            .routes
            .iter()
            .find_map(|r| {
                (r.system_a == system)
                    .then_some(r.system_b)
                    .or((r.system_b == system).then_some(r.system_a))
            })
            .unwrap();
        let edge = world.system_edge(other, system).unwrap();
        let ships: Vec<_> = (0..2)
            .map(|_| world.spawn(id("unit.corvette"), foe, edge).unwrap())
            .collect();
        let mut notices = Notices::default();
        notices.update(world);
        // The rim of the home system, far from anything of player one's.
        let target = world.system_edge(system, other).unwrap();
        for ship in &ships {
            world
                .submit(Command {
                    player: foe,
                    executor: *ship,
                    kind: "travel".into(),
                    subject: None,
                    target_entity: None,
                    target_pos: Some(target),
                    queue: false,
                })
                .unwrap();
        }
        let mut lines = Vec::new();
        let star = world.space_of(system).unwrap();
        for _ in 0..12_000 {
            world.step();
            lines.extend(notices.update(world));
            if ships
                .iter()
                .all(|s| world.entity(*s).is_some_and(|e| e.pos.body == star))
            {
                break;
            }
        }
        world.step();
        lines.extend(notices.update(world));
        let told: Vec<_> = lines
            .iter()
            .filter(|n| n.text.starts_with("Foreign ships entered"))
            .collect();
        assert_eq!(told.len(), 1, "{lines:?}");
        let name = &world
            .systems()
            .iter()
            .find(|s| s.id == system)
            .unwrap()
            .name;
        let owner = crate::ascension_view::player_name(world, PLAYER, foe);
        assert_eq!(
            told[0].text,
            format!("Foreign ships entered the {name} system: 2 ships of {owner}")
        );
        assert!(ships.iter().any(|s| told[0].about == Some(*s)));
        // Shown in the system's plane though out of sight.
        let sightings = world.visible_entities(PLAYER);
        let space =
            space_snapshot_with(PLAYER, world, &SystemIndex::new(world), system, &sightings);
        for ship in &ships {
            assert!(
                space.ships.iter().any(|s| s.id == ship.0),
                "{ship:?} not shown"
            );
        }
        assert!(
            space
                .alerts
                .iter()
                .any(|a| a.system == system && a.kind == AlertMark::Arrival && a.body.is_none())
        );
        let mut hud = HudSnapshot::default();
        fill_hud(world, PLAYER, home, &mut hud);
        assert!(hud.alerts.is_empty(), "no marker on the planet's minimap");
        assert!(ships.iter().any(|s| hud.alert_about == Some(s.0)));
    }
}
