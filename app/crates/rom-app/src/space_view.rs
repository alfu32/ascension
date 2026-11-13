//! Space as the interface sees it: the plane of a system in orbit-radius units, what is
//! published for the render thread, and the event lines for ships and claims.

use std::collections::{BTreeMap, BTreeSet};

use rom_render::{ClaimView, ShipView, SpaceSnapshot, TransitView};
use rom_rules::{BodyKind, EntityClass};
use rom_sim::{BodyId, EntityId, PlayerId, Pos, Sighting, Topology, World};

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

/// What player one sees in the space of `system`, on the star map and in transit.
pub fn space_snapshot(world: &World, system: u32, sightings: &[Sighting<'_>]) -> SpaceSnapshot {
    let rules = world.rules();
    let mut snapshot = SpaceSnapshot {
        tick: world.tick(),
        system: Some(system),
        ..SpaceSnapshot::default()
    };
    let (Some(frame), Some(reach)) = (
        SpaceFrame::of(world, system),
        rules.constant("space.orbit_reach"),
    ) else {
        return snapshot;
    };
    let bodies: Vec<(u32, Pos)> = world
        .bodies()
        .iter()
        .filter(|b| b.system == system && b.kind != BodyKind::Star)
        .filter_map(|b| Some((b.id.0, world.body_position(b.id)?)))
        .collect();
    for sighting in sightings {
        let Sighting::Live(e) = sighting else {
            continue;
        };
        let Some(et) = rules.entity_type(e.type_id) else {
            continue;
        };
        if e.pos.body != frame.body || et.class == EntityClass::Node {
            continue;
        }
        let held = bodies
            .iter()
            .map(|(id, at)| (e.pos.distance(*at) as i64, *id, *at))
            .filter(|(d, _, _)| *d <= reach * 3 / 2)
            .min_by_key(|(d, id, _)| (*d, *id));
        let (body, x, y) = match held {
            Some((_, id, at)) => (
                Some(id),
                (i64::from(e.pos.x) - i64::from(at.x)) as f32 / reach as f32,
                (i64::from(e.pos.y) - i64::from(at.y)) as f32 / reach as f32,
            ),
            None => {
                let (x, y) = frame.units_of(e.pos);
                (None, x, y)
            }
        };
        snapshot.ships.push(ShipView {
            id: e.id.0,
            kind: u16::try_from(e.type_id.0).unwrap_or(u16::MAX),
            owner: u8::try_from(e.owner.0).unwrap_or(u8::MAX),
            body,
            x,
            y,
            facing_deg: f32::from(e.facing),
            hp_frac: (e.hp as f32 / et.hp.max(1) as f32).clamp(0.0, 1.0),
        });
    }
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
    snapshot.transits = transits(world);
    snapshot
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
    started: bool,
}

impl Notices {
    /// Event lines for what changed since the last call; the first call only remembers.
    pub fn update(&mut self, world: &World) -> Vec<String> {
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
                let name = world.body(body).map_or("a body", |b| b.name.as_str());
                lines.push(match change {
                    ClaimChange::Won => format!("Claim won: {name}"),
                    ClaimChange::Lost => format!("Claim lost: {name}"),
                });
            }
        }
        self.ships = ships;
        self.claims = claims;
        self.started = true;
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
