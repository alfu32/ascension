//! Space as the interface sees it: the plane of a system in orbit-radius units, what is
//! published for the render thread, and the event lines for ships and claims.

use std::collections::{BTreeMap, BTreeSet};

use rom_render::{
    ClaimState, ClaimView, ColonyView, ResourceBodyView, ShipView, SpaceSnapshot, TransitView,
};
use rom_rules::{BodyKind, EntityClass};
use rom_sim::{BodyClaim, BodyId, EntityId, PlayerId, Pos, Sighting, Topology, World};

use crate::sim_thread::TICKS_PER_SECOND;

use crate::selection::PLAYER;

/// The plane of one system. The simulation lays a system out with at most 240 tiles from the
/// star to the outermost orbit, so one tile is `scale` orbit-radius units; the simulation does
/// not expose that scale, so it is derived here from the same rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpaceFrame {
    /// The star body whose flat surface is the plane.
    pub body: BodyId,
    /// Orbit-radius units per tile.
    pub scale: i64,
    /// The star's coordinate in milli-tiles.
    pub centre: i64,
}

impl SpaceFrame {
    /// The frame of a system.
    pub fn of(world: &World, system: u32) -> Option<Self> {
        let body = world.space_of(system)?;
        let outer = world
            .bodies()
            .iter()
            .filter(|b| b.system == system)
            .map(|b| i64::from(b.orbit_radius))
            .max()
            .unwrap_or(0);
        let Topology::Flat { width, .. } = world.surface(body)?.topology() else {
            return None;
        };
        Some(Self {
            body,
            scale: ((outer + 239) / 240).max(1),
            centre: i64::from(width) * 500,
        })
    }

    /// The offset from the star of a position of the plane, in orbit-radius units.
    pub fn units_of(&self, pos: Pos) -> (f32, f32) {
        let to = |v: i32| ((i64::from(v) - self.centre) * self.scale) as f32 / 1000.0;
        (to(pos.x), to(pos.y))
    }

    /// The position at an offset from the star in orbit-radius units, kept on the plane.
    pub fn pos_of(&self, x: f32, y: f32) -> Pos {
        let from = |v: f32| {
            let at = self.centre + (f64::from(v) * 1000.0 / self.scale as f64).round() as i64;
            at.clamp(0, self.centre * 2 - 1) as i32
        };
        Pos::on(self.body, 0, from(x), from(y))
    }
}

fn allied(world: &World, a: PlayerId, b: PlayerId) -> bool {
    let team = |id: PlayerId| world.players().find(|p| p.id == id).map(|p| p.team);
    a == b || (team(a).is_some() && team(a) == team(b))
}

/// What player one sees in space: the ships in the plane of every system, the claims of
/// `system` and the ships in transit.
pub fn space_snapshot(world: &World, system: u32, sightings: &[Sighting<'_>]) -> SpaceSnapshot {
    let rules = world.rules();
    let mut snapshot = SpaceSnapshot {
        tick: world.tick(),
        system: Some(system),
        ..SpaceSnapshot::default()
    };
    let Some(reach) = rules.constant("space.orbit_reach") else {
        return snapshot;
    };
    let planes: BTreeMap<BodyId, (u32, SpaceFrame)> = world
        .systems()
        .iter()
        .filter_map(|s| {
            let frame = SpaceFrame::of(world, s.id)?;
            Some((frame.body, (s.id, frame)))
        })
        .collect();
    let bodies: Vec<(u32, u32, Pos)> = world
        .bodies()
        .iter()
        .filter(|b| b.kind != BodyKind::Star)
        .filter_map(|b| Some((b.system, b.id.0, world.body_position(b.id)?)))
        .collect();
    for sighting in sightings {
        let Sighting::Live(e) = sighting else {
            continue;
        };
        let Some(et) = rules.entity_type(e.type_id) else {
            continue;
        };
        let Some(&(in_system, frame)) = planes.get(&e.pos.body) else {
            continue;
        };
        if et.class == EntityClass::Node {
            continue;
        }
        let held = bodies
            .iter()
            .filter(|(s, _, _)| *s == in_system)
            .map(|(_, id, at)| (e.pos.distance(*at) as i64, *id, *at))
            .filter(|(d, _, _)| *d <= reach * 3 / 2)
            .min_by_key(|(d, id, _)| (*d, *id));
        let (body, x, y) = if let Some(orbit) = world.orbit(e.id) {
            let point = match world.surface(orbit.body).map(|surface| surface.topology()) {
                Some(Topology::Cube(cube)) => cube.sphere_point(rom_render::SurfacePos {
                    face: orbit.pos.face,
                    x: i64::from(orbit.pos.x),
                    y: i64::from(orbit.pos.y),
                }),
                _ => [1_000_000, 0, 0],
            };
            let length = (point[0] as f32).hypot(point[2] as f32).max(1.0);
            (
                Some(orbit.body.0),
                point[0] as f32 / length,
                point[2] as f32 / length,
            )
        } else {
            match held {
                Some((_, id, at)) => (
                    Some(id),
                    (i64::from(e.pos.x) - i64::from(at.x)) as f32 / reach as f32,
                    (i64::from(e.pos.y) - i64::from(at.y)) as f32 / reach as f32,
                ),
                None => {
                    let (x, y) = frame.units_of(e.pos);
                    (None, x, y)
                }
            }
        };
        snapshot.ships.push(ShipView {
            id: e.id.0,
            kind: u16::try_from(e.type_id.0).unwrap_or(u16::MAX),
            owner: u8::try_from(e.owner.0).unwrap_or(u8::MAX),
            system: in_system,
            body,
            x,
            y,
            facing_deg: f32::from(e.facing),
            hp_frac: (e.hp as f32 / et.hp.max(1) as f32).clamp(0.0, 1.0),
        });
    }
    snapshot.ships.sort_by_key(|s| s.id);
    if let Some(frame) = SpaceFrame::of(world, system) {
        for (body, owner) in world.claims() {
            let Some(info) = world.body(body).filter(|b| b.system == system) else {
                continue;
            };
            let seen = allied(world, PLAYER, owner)
                || world.body_position(info.id).is_some_and(|at| {
                    world.is_visible_on_body(
                        PLAYER,
                        frame.body,
                        0,
                        at.x.div_euclid(1000),
                        at.y.div_euclid(1000),
                    )
                });
            if seen {
                snapshot.claims.push(ClaimView {
                    body: body.0,
                    owner: u8::try_from(owner.0).unwrap_or(u8::MAX),
                });
            }
        }
        let visible = |at: Pos| {
            world.is_visible_on_body(
                PLAYER,
                frame.body,
                0,
                at.x.div_euclid(1000),
                at.y.div_euclid(1000),
            )
        };
        for info in world
            .bodies()
            .iter()
            .filter(|b| b.system == system && b.kind == BodyKind::Resource)
        {
            let Some(mut view) = resource_view(world, info.id) else {
                continue;
            };
            let seen = |owner: PlayerId| {
                allied(world, PLAYER, owner) || world.body_position(info.id).is_some_and(visible)
            };
            view.claim = match world.body_claim(info.id) {
                BodyClaim::Held(owner) if allied(world, PLAYER, owner) => ClaimState::Yours,
                BodyClaim::Held(owner) if seen(owner) => {
                    ClaimState::Other(u8::try_from(owner.0).unwrap_or(u8::MAX))
                }
                BodyClaim::Contested if seen(PLAYER) => ClaimState::Contested,
                _ => ClaimState::Unclaimed,
            };
            snapshot.resources.push(view);
        }
    }
    snapshot.transits = transits(world);
    snapshot.known = Some(known_systems(world));
    snapshot.colonies = colonies(world);
    snapshot
}

/// The planets with cities that player one knows of: its own and its allies' everywhere, other
/// players' where it has explored the city's ground. By system, body and owner.
pub fn colonies(world: &World) -> Vec<ColonyView> {
    let rules = world.rules();
    let mut out: Vec<ColonyView> = world
        .entities()
        .filter(|e| {
            e.owner != PlayerId(0)
                && e.build_progress.is_none()
                && rules.entity_types[&e.type_id]
                    .territory
                    .as_ref()
                    .is_some_and(|t| t.is_city)
        })
        .filter(|e| {
            allied(world, PLAYER, e.owner)
                || world.is_explored_on_body(
                    PLAYER,
                    e.pos.body,
                    e.pos.face,
                    e.pos.x.div_euclid(1000),
                    e.pos.y.div_euclid(1000),
                )
        })
        .filter_map(|e| {
            let body = world.body(e.pos.body)?;
            Some(ColonyView {
                system: body.system,
                body: body.id.0,
                owner: u8::try_from(e.owner.0).unwrap_or(u8::MAX),
            })
        })
        .collect();
    out.sort_unstable();
    out.dedup();
    out
}

/// What the resource body `body` holds and pays player one, with no claim state filled in.
/// `None` for bodies that are not presence resource bodies.
pub fn resource_view(world: &World, body: BodyId) -> Option<ResourceBodyView> {
    let rules = world.rules();
    let info = world.body(body)?;
    let node_type = info.node_type?;
    let paid = world.body_yield(body, PLAYER)?;
    let faction = world
        .players()
        .find(|p| p.id == PLAYER)
        .map_or(rom_rules::DefId(0), |p| p.faction);
    Some(ResourceBodyView {
        body: body.0,
        kind_name: rules.display_name(node_type, faction).to_owned(),
        resource_name: rules.def(paid.resource).name.clone(),
        icon: crate::hud::resource_icon(rules, paid.resource),
        colour: crate::hud::resource_colour(rules, paid.resource),
        per_period: paid.per_period,
        period_secs: paid.period_ticks / i64::from(TICKS_PER_SECOND),
        claim: ClaimState::Unclaimed,
    })
}

/// "Tar Moon: Oil +42 / 30 s" for a ship of player `owner` that keeps station at a resource body
/// the owner holds, for the selected ship's card.
pub fn holding_line(world: &World, ship: &rom_sim::Entity) -> Option<String> {
    let rules = world.rules();
    let (info, _) = world
        .claims()
        .filter(|(_, owner)| *owner == ship.owner)
        .filter_map(|(body, owner)| Some((world.body(body)?, owner)))
        .find(|(info, _)| {
            let radius = info
                .node_type
                .and_then(|t| rules.entity_types.get(&t)?.node.as_ref())
                .map_or(0, |n| n.claim_radius);
            world.body_position(info.id).is_some_and(|at| {
                at.body == ship.pos.body && ship.pos.distance(at) as i64 <= radius
            })
        })?;
    let view = resource_view(world, info.id)?;
    Some(format!("Holds {}: {}", view.kind_name, view.yield_text()))
}

/// The systems player one knows, ascending: its home system, any system it has ships in or
/// bound for, and any whose star or bodies its team has seen.
pub fn known_systems(world: &World) -> Vec<u32> {
    let home = world
        .home_body(PLAYER)
        .and_then(|b| world.body(b))
        .map(|b| b.system);
    let mut known: BTreeSet<u32> = home.into_iter().collect();
    for t in world
        .transits()
        .filter(|t| allied(world, PLAYER, t.entity.owner))
    {
        known.insert(t.origin);
        known.insert(t.destination);
    }
    let crowded: BTreeSet<BodyId> = world
        .entities()
        .filter(|e| allied(world, PLAYER, e.owner))
        .map(|e| e.pos.body)
        .collect();
    for system in world.systems() {
        let Some(star) = world.space_of(system.id) else {
            continue;
        };
        let seen = |at: Pos| {
            world.is_explored_on_body(
                PLAYER,
                star,
                0,
                at.x.div_euclid(1000),
                at.y.div_euclid(1000),
            )
        };
        let bodies = world
            .bodies()
            .iter()
            .filter(|b| b.system == system.id)
            .filter_map(|b| world.body_position(b.id));
        if crowded.contains(&star)
            || world.body_position(star).is_some_and(seen)
            || bodies.into_iter().any(seen)
        {
            known.insert(system.id);
        }
    }
    known.into_iter().collect()
}

/// Ships between systems that belong to player one or an ally; others are not seen.
pub fn transits(world: &World) -> Vec<TransitView> {
    world
        .transits()
        .filter(|t| allied(world, PLAYER, t.entity.owner))
        .map(|t| TransitView {
            id: t.entity.id.0,
            kind: u16::try_from(t.entity.type_id.0).unwrap_or(u16::MAX),
            owner: u8::try_from(t.entity.owner.0).unwrap_or(u8::MAX),
            from: t.origin,
            to: t.destination,
            progress: (world.tick().saturating_sub(t.departed)) as f32
                / t.arrives.saturating_sub(t.departed).max(1) as f32,
        })
        .collect()
}

/// A change of a claim that player one hears about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClaimChange {
    /// Player one holds the body now.
    Won,
    /// Player one held the body and does not any more.
    Lost,
}

/// The changes between two claim tables, in body order.
pub fn claim_changes(
    before: &BTreeMap<BodyId, PlayerId>,
    now: &BTreeMap<BodyId, PlayerId>,
) -> Vec<(BodyId, ClaimChange)> {
    let bodies: BTreeSet<BodyId> = before.keys().chain(now.keys()).copied().collect();
    bodies
        .into_iter()
        .filter_map(|body| match (before.get(&body), now.get(&body)) {
            (b, Some(n)) if *n == PLAYER && b != Some(&PLAYER) => Some((body, ClaimChange::Won)),
            (Some(b), n) if *b == PLAYER && n != Some(&PLAYER) => Some((body, ClaimChange::Lost)),
            _ => None,
        })
        .collect()
}

/// Remembers ships and claims of player one and words what changed.
#[derive(Debug, Default)]
pub struct Notices {
    ships: BTreeSet<EntityId>,
    claims: BTreeMap<BodyId, PlayerId>,
    /// Ships of player one between systems, with the system each is bound for.
    travelling: BTreeMap<EntityId, u32>,
    /// The rank of player one's map scale, to explain each new reach once.
    reach: Option<i64>,
    started: bool,
}

/// How to see a newly reached tier (by its rank: 1 a star system, 2 the star map).
pub fn reach_hint(rank: i64) -> Option<&'static str> {
    match rank {
        1 => Some(
            "New reach: your star system. Scroll out past the planet to see it; \
             double-click a planet to fly there",
        ),
        2.. => Some(
            "New reach: the star map. Scroll out past the system to see it; \
             double-click or scroll in on a system to enter it",
        ),
        _ => None,
    }
}

/// One line for the event line, and the entity it is about (clicking the line flies there).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notice {
    /// The words.
    pub text: String,
    /// The ship that departed or arrived, if any.
    pub about: Option<EntityId>,
}

impl Notices {
    /// Event lines for what changed since the last call; the first call only remembers.
    pub fn update(&mut self, world: &World) -> Vec<Notice> {
        let mut notices: Vec<Notice> = self
            .changes(world)
            .into_iter()
            .map(|text| Notice { text, about: None })
            .collect();
        let rules = world.rules();
        let faction = world
            .players()
            .find(|p| p.id == PLAYER)
            .map_or(rom_rules::DefId(0), |p| p.faction);
        let name_of = |system: u32| {
            world
                .systems()
                .iter()
                .find(|s| s.id == system)
                .map_or_else(|| format!("system {system}"), |s| s.name.clone())
        };
        let travelling: BTreeMap<EntityId, u32> = world
            .transits()
            .filter(|t| t.entity.owner == PLAYER)
            .map(|t| (t.entity.id, t.destination))
            .collect();
        if self.started {
            for transit in world.transits().filter(|t| t.entity.owner == PLAYER) {
                if !self.travelling.contains_key(&transit.entity.id) {
                    notices.push(Notice {
                        text: format!(
                            "{} departed for {}",
                            rules.display_name(transit.entity.type_id, faction),
                            name_of(transit.destination)
                        ),
                        about: Some(transit.entity.id),
                    });
                }
            }
            for (&id, &destination) in &self.travelling {
                if travelling.contains_key(&id) {
                    continue;
                }
                if let Some(ship) = world.entity(id) {
                    notices.push(Notice {
                        text: format!(
                            "{} arrived at {}",
                            rules.display_name(ship.type_id, faction),
                            name_of(destination)
                        ),
                        about: Some(id),
                    });
                }
            }
        }
        self.travelling = travelling;
        let reach = world.players().find(|p| p.id == PLAYER).and_then(|p| {
            let age = rules.ages.get(&p.age)?;
            Some(rules.map_scales.get(&age.map_scale)?.rank)
        });
        if self.started
            && let (Some(before), Some(now)) = (self.reach, reach)
            && now > before
            && let Some(text) = reach_hint(now)
        {
            notices.push(Notice {
                text: text.to_owned(),
                about: None,
            });
        }
        self.reach = reach;
        self.started = true;
        notices
    }

    fn changes(&mut self, world: &World) -> Vec<String> {
        let rules = world.rules();
        let faction = world
            .players()
            .find(|p| p.id == PLAYER)
            .map_or(rom_rules::DefId(0), |p| p.faction);
        let mut lines = Vec::new();
        let mut ships = BTreeSet::new();
        for e in world.entities().filter(|e| e.owner == PLAYER) {
            let in_space = world
                .body(e.pos.body)
                .is_some_and(|b| b.kind == BodyKind::Star);
            if in_space && rules.entity_types[&e.type_id].movement.is_some() {
                ships.insert(e.id);
                if self.started && !self.ships.contains(&e.id) {
                    lines.push(format!(
                        "New ship: {}",
                        rules.display_name(e.type_id, faction)
                    ));
                }
            }
        }
        // Ships in transit are not in the world's entities; keep them known.
        ships.extend(
            world
                .transits()
                .filter(|t| t.entity.owner == PLAYER)
                .map(|t| t.entity.id),
        );
        let claims: BTreeMap<BodyId, PlayerId> = world.claims().collect();
        if self.started {
            for (body, change) in claim_changes(&self.claims, &claims) {
                let Some(view) = resource_view(world, body) else {
                    continue;
                };
                lines.push(match change {
                    ClaimChange::Won => {
                        format!("{} claimed: {}", view.kind_name, view.yield_text())
                    }
                    ClaimChange::Lost if world.body_claim(body) == BodyClaim::Contested => {
                        format!("{} contested", view.kind_name)
                    }
                    ClaimChange::Lost => format!("{} lost", view.kind_name),
                });
            }
        }
        self.ships = ships;
        self.claims = claims;
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claim_changes_name_what_player_one_won_and_lost() {
        let p = |n| PlayerId(n);
        let b = |n| BodyId(n);
        let before = BTreeMap::from([(b(1), p(1)), (b(2), p(2))]);
        let now = BTreeMap::from([(b(2), p(1)), (b(3), p(1)), (b(4), p(2))]);
        assert_eq!(
            claim_changes(&before, &now),
            vec![
                (b(1), ClaimChange::Lost),
                (b(2), ClaimChange::Won),
                (b(3), ClaimChange::Won)
            ]
        );
        assert!(claim_changes(&now, &now).is_empty());
    }
}

#[cfg(test)]
mod transit_tests {
    use super::*;
    use crate::selection::Controller;
    use rom_render::{HudSnapshot, Modifiers, PickEvent};
    use rom_sim::Command;

    /// A ship of player one sent from its home system to a neighbouring one stays selected and
    /// visible for its whole journey, and its departure and arrival are announced about it.
    #[test]
    fn a_ship_in_transit_stays_selected_seen_and_announced() {
        let (content, mut game) = crate::match_setup::universe_game_for_tests();
        let home = game.world.home_body(PLAYER).unwrap();
        let home_system = game.world.body(home).unwrap().system;
        let ship_type = game.world.rules().id_of("unit.survey_ship").unwrap();
        let star = game.world.space_of(home_system).unwrap();
        let mut at = game.world.body_position(star).unwrap();
        at.x += 20_000;
        let ship = game.world.spawn(ship_type, PLAYER, at).unwrap();
        let other = content
            .universe
            .routes
            .iter()
            .find_map(|r| {
                if r.system_a == home_system {
                    Some(r.system_b)
                } else if r.system_b == home_system {
                    Some(r.system_a)
                } else {
                    None
                }
            })
            .unwrap();
        // At first only the home system is known; the other end of the lane is not.
        let known = known_systems(&game.world);
        assert!(known.contains(&home_system) && !known.contains(&other));
        let mut controller = Controller::new();
        let mut notices = Notices::default();
        notices.update(&game.world);
        controller.handle(
            &mut game.world,
            PickEvent::BoxSelect {
                entities: vec![ship.0],
                modifiers: Modifiers::default(),
            },
        );
        let target = game
            .world
            .body_position(game.world.space_of(other).unwrap())
            .unwrap();
        game.world
            .submit(Command {
                player: PLAYER,
                executor: ship,
                kind: "travel".into(),
                subject: None,
                target_entity: None,
                target_pos: Some(target),
                queue: false,
            })
            .unwrap();
        let (mut departed, mut arrived, mut seen_in_transit) = (None, None, false);
        for _ in 0..12_000 {
            game.world.step();
            controller.after_step(&mut game.world);
            for notice in notices.update(&game.world) {
                if notice.text.contains("departed") {
                    departed = Some(notice.clone());
                }
                if notice.text.contains("arrived") {
                    arrived = Some(notice.clone());
                }
            }
            if game.world.transits().any(|t| t.entity.id == ship) {
                let mut hud = HudSnapshot::default();
                controller.fill(&game.world, &mut hud);
                assert_eq!(hud.selected, vec![ship.0], "still selected in transit");
                let sightings = game.world.visible_entities(PLAYER);
                let space = space_snapshot(&game.world, home_system, &sightings);
                let transit = space.transits.iter().find(|t| t.id == ship.0).unwrap();
                assert_eq!((transit.from, transit.to), (home_system, other));
                assert!((0.0..=1.0).contains(&transit.progress));
                seen_in_transit = true;
            }
            if arrived.is_some() {
                break;
            }
        }
        assert!(seen_in_transit);
        let departed = departed.expect("a departure line");
        assert_eq!(departed.about, Some(ship));
        let name = |id: u32| {
            game.world
                .systems()
                .iter()
                .find(|s| s.id == id)
                .unwrap()
                .name
                .clone()
        };
        assert!(departed.text.ends_with(&name(other)), "{}", departed.text);
        let arrived = arrived.expect("an arrival line");
        assert_eq!(arrived.about, Some(ship));
        // Its destination is known now: its bodies are drawn and named.
        assert!(known_systems(&game.world).contains(&other));
        // Arrived, it is in the space of the other system, still selected.
        let sightings = game.world.visible_entities(PLAYER);
        let space = space_snapshot(&game.world, home_system, &sightings);
        assert_eq!(
            space
                .ships
                .iter()
                .find(|s| s.id == ship.0)
                .map(|s| s.system),
            Some(other)
        );
        let mut hud = HudSnapshot::default();
        controller.fill(&game.world, &mut hud);
        assert_eq!(hud.selected, vec![ship.0]);
    }
}

#[cfg(test)]
mod resource_body_tests {
    use super::*;
    use rom_sim::Command;

    /// A survey ship claims a resource body of the home system: the body lists its kind, resource
    /// and the yield the economy then pays, the event line says what was claimed, and the ship's
    /// card says what it holds. A rival ship beside it makes the body contested.
    #[test]
    fn a_claim_says_what_the_body_holds_and_pays() {
        let (_content, mut game) = crate::match_setup::universe_game_for_tests();
        let world = &mut game.world;
        let home = world.home_body(PLAYER).unwrap();
        let system = world.body(home).unwrap().system;
        let body = world
            .bodies()
            .iter()
            .find(|b| b.system == system && b.kind == BodyKind::Resource)
            .unwrap()
            .clone();
        let before = resource_view(world, body.id).unwrap();
        assert_eq!(before.claim, ClaimState::Unclaimed);
        assert!(before.per_period > 0 && before.period_secs > 0);
        assert!(!before.kind_name.is_empty() && before.icon.starts_with("icons/res_"));
        let resource = world.body_yield(body.id, PLAYER).unwrap().resource;
        let income_before = world.income(PLAYER, resource).effective;
        let ship_type = world.rules().id_of("unit.survey_ship").unwrap();
        let ship = world
            .spawn(ship_type, PLAYER, world.body_position(body.id).unwrap())
            .unwrap();
        let mut notices = Notices::default();
        notices.update(world);
        world
            .submit(Command {
                player: PLAYER,
                executor: ship,
                kind: "orbit".into(),
                subject: None,
                target_entity: world.body_marker(body.id),
                target_pos: None,
                queue: false,
            })
            .unwrap();
        let mut lines = Vec::new();
        for _ in 0..400 {
            world.step();
            lines.extend(notices.update(world).into_iter().map(|n| n.text));
            if world.claims().any(|(b, _)| b == body.id) {
                break;
            }
        }
        let claimed = resource_view(world, body.id).unwrap();
        let sightings = world.visible_entities(PLAYER);
        let space = space_snapshot(world, system, &sightings);
        let listed = space.resource_body(body.id.0).expect("listed");
        assert_eq!(listed.claim, ClaimState::Yours);
        assert_eq!(listed.kind_name, claimed.kind_name);
        // The event line names the body kind and the yield.
        let expected = format!("{} claimed: {}", claimed.kind_name, claimed.yield_text());
        assert!(lines.contains(&expected), "{lines:?}");
        // The listed yield is what the economy pays more once the body is held.
        let income_after = world.income(PLAYER, resource).effective;
        assert_eq!(income_after - income_before, listed.per_period);
        // The ship's card says what it holds.
        let card = holding_line(world, world.entity(ship).unwrap()).expect("holding line");
        assert_eq!(
            card,
            format!("Holds {}: {}", claimed.kind_name, claimed.yield_text())
        );
    }
}
