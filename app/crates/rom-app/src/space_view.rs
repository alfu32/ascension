//! Space as the interface sees it: the plane of a system in orbit-radius units, what is
//! published for the render thread, and the event lines for ships and claims.

use std::collections::{BTreeMap, BTreeSet};

use rom_render::{
    ChargeView, ClaimState, ClaimView, ColonyView, HoldView, ResourceBodyView, ShipView,
    SpaceSnapshot, StrikeView, TransitView,
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
        Self::build(world, body, outer)
    }

    /// The frame of the system whose star is `body` and whose outermost orbit is `outer`.
    fn build(world: &World, body: BodyId, outer: i64) -> Option<Self> {
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

/// The bodies of every system, found once per match: bodies never move between systems, so the
/// per-tick snapshot looks a system up instead of scanning every body of the universe.
#[derive(Clone, Debug, Default)]
pub struct SystemIndex {
    entries: BTreeMap<u32, SystemEntry>,
}

/// One system of a [`SystemIndex`].
#[derive(Clone, Debug, Default)]
struct SystemEntry {
    /// The first star body of the system, as [`World::space_of`] finds it.
    star: Option<BodyId>,
    /// Every body of the system in body order, the star included.
    bodies: Vec<BodyId>,
    /// The plane of the system.
    frame: Option<SpaceFrame>,
}

impl SystemIndex {
    /// Indexes every body of `world` by its system, in one pass.
    pub fn new(world: &World) -> Self {
        let mut outer: BTreeMap<u32, i64> = BTreeMap::new();
        let mut entries: BTreeMap<u32, SystemEntry> = world
            .systems()
            .iter()
            .map(|s| (s.id, SystemEntry::default()))
            .collect();
        for body in world.bodies() {
            let entry = entries.entry(body.system).or_default();
            entry.bodies.push(body.id);
            if body.kind == BodyKind::Star && entry.star.is_none() {
                entry.star = Some(body.id);
            }
            let furthest = outer.entry(body.system).or_insert(0);
            *furthest = (*furthest).max(i64::from(body.orbit_radius));
        }
        for (system, entry) in &mut entries {
            entry.frame = entry
                .star
                .and_then(|star| SpaceFrame::build(world, star, outer.get(system).copied()?));
        }
        Self { entries }
    }

    /// The bodies of `system` in body order.
    pub fn bodies(&self, system: u32) -> &[BodyId] {
        self.entries.get(&system).map_or(&[], |e| &e.bodies)
    }

    /// The star whose plane holds the system's ships.
    pub fn star(&self, system: u32) -> Option<BodyId> {
        self.entries.get(&system)?.star
    }

    /// The plane of `system`.
    pub fn frame(&self, system: u32) -> Option<SpaceFrame> {
        self.entries.get(&system)?.frame
    }

    /// Every system with a plane, with its frame, in system order.
    pub fn frames(&self) -> impl Iterator<Item = (u32, SpaceFrame)> + '_ {
        self.entries
            .iter()
            .filter_map(|(&system, e)| Some((system, e.frame?)))
    }
}

fn allied(world: &World, a: PlayerId, b: PlayerId) -> bool {
    let team = |id: PlayerId| world.players().find(|p| p.id == id).map(|p| p.team);
    a == b || (team(a).is_some() && team(a) == team(b))
}

/// What player one sees in space: the ships in the plane of every system, the claims of
/// `system` and the ships in transit.
#[cfg(test)]
pub fn space_snapshot(world: &World, system: u32, sightings: &[Sighting<'_>]) -> SpaceSnapshot {
    space_snapshot_with(PLAYER, world, &SystemIndex::new(world), system, sightings)
}

/// [`space_snapshot`] with the match's system index, built once by the caller.
pub fn space_snapshot_with(
    me: PlayerId,
    world: &World,
    index: &SystemIndex,
    system: u32,
    sightings: &[Sighting<'_>],
) -> SpaceSnapshot {
    let rules = world.rules();
    let mut snapshot = SpaceSnapshot {
        tick: world.tick(),
        system: Some(system),
        ..SpaceSnapshot::default()
    };
    // Blasts on planets the player sees flash on their globes (task 75).
    snapshot.flashes = world
        .blasts()
        .iter()
        .filter(|b| world.tick().saturating_sub(b.tick) < u64::from(rom_render::FLASH_TICKS))
        .filter(|b| {
            world.is_visible_on_body(
                me,
                b.pos.body,
                b.pos.face,
                b.pos.x.div_euclid(1000),
                b.pos.y.div_euclid(1000),
            )
        })
        .filter_map(|b| {
            let Some(Topology::Cube(cube)) = world.surface(b.pos.body).map(|s| s.topology()) else {
                return None;
            };
            let p = cube
                .sphere_point(rom_render::SurfacePos {
                    face: b.pos.face,
                    x: i64::from(b.pos.x),
                    y: i64::from(b.pos.y),
                })
                .map(|c| c as f32);
            let length = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt().max(1.0);
            Some(rom_render::FlashView {
                body: b.pos.body.0,
                dir: rom_render::to_render(p.map(|c| c / length)),
                age_ticks: world.tick().saturating_sub(b.tick) as u32,
            })
        })
        .collect();
    let Some(reach) = rules.constant("space.orbit_reach") else {
        return snapshot;
    };
    let planes: BTreeMap<BodyId, (u32, SpaceFrame)> = index
        .frames()
        .map(|(id, frame)| (frame.body, (id, frame)))
        .collect();
    snapshot.strikes = strikes(me, world, &planes);
    snapshot.alerts = crate::alert_view::space_alerts(world, me, &planes);
    // Ships a detector saw jump in are shown a while though out of sight (task 118).
    let live: Vec<&rom_sim::Entity> = sightings
        .iter()
        .filter_map(|s| match s {
            Sighting::Live(e) => Some(*e),
            Sighting::Remembered(_) => None,
        })
        .collect();
    let revealed: Vec<&rom_sim::Entity> = crate::alert_view::revealed_ships(world, me, &planes)
        .into_iter()
        .filter(|e| !live.iter().any(|l| l.id == e.id))
        .collect();
    for e in live.into_iter().chain(revealed) {
        let Some(et) = rules.entity_type(e.type_id) else {
            continue;
        };
        let Some(&(in_system, frame)) = planes.get(&e.pos.body) else {
            continue;
        };
        if et.class == EntityClass::Node {
            continue;
        }
        let held = index
            .bodies(in_system)
            .iter()
            .filter(|id| world.body(**id).is_some_and(|b| b.kind != BodyKind::Star))
            .filter_map(|id| Some((id.0, world.body_position(*id)?)))
            .map(|(id, at)| (e.pos.distance(at) as i64, id, at))
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
            model: None,
        });
    }
    snapshot.ships.sort_by_key(|s| s.id);
    if let Some(frame) = index.frame(system) {
        for (body, owner) in world.claims() {
            let Some(info) = world.body(body).filter(|b| b.system == system) else {
                continue;
            };
            let seen = allied(world, me, owner)
                || world.body_position(info.id).is_some_and(|at| {
                    world.is_visible_on_body(
                        me,
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
                me,
                frame.body,
                0,
                at.x.div_euclid(1000),
                at.y.div_euclid(1000),
            )
        };
        for info in index
            .bodies(system)
            .iter()
            .filter_map(|id| world.body(*id))
            .filter(|b| b.kind == BodyKind::Resource)
        {
            let Some(mut view) = resource_view(me, world, info.id) else {
                continue;
            };
            let seen = |owner: PlayerId| {
                allied(world, me, owner) || world.body_position(info.id).is_some_and(visible)
            };
            view.claim = match world.body_claim(info.id) {
                BodyClaim::Held(owner) if allied(world, me, owner) => ClaimState::Yours,
                BodyClaim::Held(owner) if seen(owner) => {
                    ClaimState::Other(u8::try_from(owner.0).unwrap_or(u8::MAX))
                }
                BodyClaim::Contested if seen(me) => ClaimState::Contested,
                _ => ClaimState::Unclaimed,
            };
            snapshot.resources.push(view);
        }
    }
    snapshot.transits = transits(me, world);
    snapshot.known = Some(known_systems(me, world));
    snapshot.countdowns = crate::ascension_view::countdowns(world, me);
    snapshot.residences = crate::ascension_view::residences(world, me);
    snapshot.colonies = colonies(me, world);
    snapshot.capitals = crate::capital_view::space_capitals(&world.known_capitals(me));
    snapshot.charted = Some(world.known_routes(me));
    snapshot.charges = charges(me, world, &snapshot.ships);
    snapshot.holds = holds(me, world, index);
    snapshot
}

/// Jump inhibitors' recent blows on arriving ships that player one sees (task 104): on its own
/// or an ally's ships, by its own or an ally's inhibitor, or in sight; oldest first.
fn strikes(
    me: PlayerId,
    world: &World,
    planes: &BTreeMap<BodyId, (u32, SpaceFrame)>,
) -> Vec<StrikeView> {
    world
        .arrival_strikes()
        .iter()
        .filter(|s| world.tick().saturating_sub(s.tick) < u64::from(rom_render::STRIKE_FLASH_TICKS))
        .filter(|s| {
            allied(world, me, s.ship_owner)
                || allied(world, me, s.owner)
                || world.is_visible_on_body(
                    me,
                    s.pos.body,
                    s.pos.face,
                    s.pos.x.div_euclid(1000),
                    s.pos.y.div_euclid(1000),
                )
        })
        .filter_map(|s| {
            let (system, frame) = planes.get(&s.pos.body)?;
            let (x, y) = frame.units_of(s.pos);
            Some(StrikeView {
                system: *system,
                x,
                y,
                age_ticks: world.tick().saturating_sub(s.tick) as u32,
                destroyed: s.destroyed,
            })
        })
        .collect()
}

/// The ships of player one and its allies among `ships` that are charging a jump.
pub fn charges(me: PlayerId, world: &World, ships: &[ShipView]) -> Vec<ChargeView> {
    ships
        .iter()
        .filter(|s| allied(world, me, PlayerId(u32::from(s.owner))))
        .filter_map(|s| {
            let charge = world.jump_charge(EntityId(s.id))?;
            Some(ChargeView {
                id: s.id,
                fraction: (charge.progress as f32 / charge.required.max(1) as f32).clamp(0.0, 1.0),
            })
        })
        .collect()
}

/// The systems with a jump inhibitor that player one knows of: its own and its allies', and an
/// enemy's that holds a ship of player one, or whose ground it has explored. By system.
pub fn holds(me: PlayerId, world: &World, index: &SystemIndex) -> Vec<HoldView> {
    let mut out: Vec<HoldView> = world
        .jump_inhibitors()
        .filter_map(|(e, _)| {
            let system = world.body(e.pos.body)?.system;
            let enemy = world.enemies(me, e.owner);
            let known = !enemy
                || world.is_explored_on_body(
                    me,
                    e.pos.body,
                    e.pos.face,
                    e.pos.x.div_euclid(1000),
                    e.pos.y.div_euclid(1000),
                )
                || world.entities().any(|own| {
                    own.owner == me && index.star(system).is_some_and(|star| own.pos.body == star)
                });
            known.then(|| HoldView {
                system,
                owner: u8::try_from(e.owner.0).unwrap_or(u8::MAX),
                enemy,
                arrival_damage: world.rules().entity_types[&e.type_id]
                    .inhibitor
                    .as_ref()
                    .map_or(0, |i| {
                        u32::try_from(i.arrival_damage.max(0)).unwrap_or(u32::MAX)
                    }),
            })
        })
        .collect();
    // The largest blow first among a system's inhibitors of one side: several do not stack.
    out.sort_by_key(|h| {
        (
            h.system,
            !h.enemy,
            std::cmp::Reverse(h.arrival_damage),
            h.owner,
        )
    });
    out.dedup_by_key(|h| (h.system, h.enemy));
    out
}

/// "Charging jump: 4 s" for a ship of player one that is charging, with where it is bound.
pub fn charge_line(world: &World, ship: &rom_sim::Entity) -> Option<String> {
    let charge = world.jump_charge(ship.id)?;
    let seconds = charge.remaining().div_ceil(u64::from(TICKS_PER_SECOND));
    let name = world
        .systems()
        .iter()
        .find(|s| s.id == charge.toward)
        .map_or_else(String::new, |s| format!(" to {}", s.name));
    Some(format!("Charging jump{name}: {seconds} s"))
}

/// Gives every ship of `snapshot` the composed model of its owner's faction and age from
/// `models`, the one the planet view draws it with; a ship type without one keeps none.
pub fn dress_ships(snapshot: &mut SpaceSnapshot, world: &World, models: &crate::looks::ModelBook) {
    let looks: BTreeMap<u32, (rom_rules::DefId, rom_rules::DefId)> = world
        .players()
        .map(|p| (p.id.0, (p.faction, p.age)))
        .collect();
    for ship in &mut snapshot.ships {
        ship.model = looks
            .get(&u32::from(ship.owner))
            .and_then(|&(faction, age)| {
                models.resolve(rom_rules::DefId(u32::from(ship.kind)), faction, age)
            });
    }
}

/// The planets with cities that player one knows of: its own and its allies' everywhere, other
/// players' where it has explored the city's ground. By system, body and owner.
pub fn colonies(me: PlayerId, world: &World) -> Vec<ColonyView> {
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
            allied(world, me, e.owner)
                || world.is_explored_on_body(
                    me,
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
pub fn resource_view(me: PlayerId, world: &World, body: BodyId) -> Option<ResourceBodyView> {
    let rules = world.rules();
    let info = world.body(body)?;
    let node_type = info.node_type?;
    let paid = world.body_yield(body, me)?;
    let faction = world
        .players()
        .find(|p| p.id == me)
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
    let view = resource_view(ship.owner, world, info.id)?;
    Some(format!("Holds {}: {}", view.kind_name, view.yield_text()))
}

/// The systems player one knows, ascending: those it has explored as the simulation decides
/// (`World::system_explored`, task 111), so the star map's "(unexplored)" label and where a jump
/// ends agree: its home system, the celestial home, any system in whose plane it or an ally has
/// had a ship or seen a tile, and every system once it has ascended.
pub fn known_systems(me: PlayerId, world: &World) -> Vec<u32> {
    world.explored_systems(me)
}

/// Ships between systems that belong to player one or an ally; others are not seen.
pub fn transits(me: PlayerId, world: &World) -> Vec<TransitView> {
    world
        .transits()
        .filter(|t| allied(world, me, t.entity.owner))
        .map(|t| TransitView {
            id: t.entity.id.0,
            kind: u16::try_from(t.entity.type_id.0).unwrap_or(u16::MAX),
            owner: u8::try_from(t.entity.owner.0).unwrap_or(u8::MAX),
            from: t.origin,
            to: t.destination,
            progress: (world.tick().saturating_sub(t.departed)) as f32
                / t.arrives.saturating_sub(t.departed).max(1) as f32,
            charting: !world.knows_route(t.entity.owner, t.origin, t.destination),
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
    me: PlayerId,
    before: &BTreeMap<BodyId, PlayerId>,
    now: &BTreeMap<BodyId, PlayerId>,
) -> Vec<(BodyId, ClaimChange)> {
    let bodies: BTreeSet<BodyId> = before.keys().chain(now.keys()).copied().collect();
    bodies
        .into_iter()
        .filter_map(|body| match (before.get(&body), now.get(&body)) {
            (b, Some(n)) if *n == me && b != Some(&me) => Some((body, ClaimChange::Won)),
            (Some(b), n) if *b == me && n != Some(&me) => Some((body, ClaimChange::Lost)),
            _ => None,
        })
        .collect()
}

/// The event lines for the jump inhibitors' blows struck after tick `after` (task 104): one per
/// inhibitor of player one ("Jump inhibitor at Body 2 struck 3 arriving ships, 2 destroyed"),
/// and one per enemy inhibitor that struck player one's ships.
pub fn strike_notices(me: PlayerId, world: &World, after: u64) -> Vec<Notice> {
    // (inhibitor, whether it is player one's) → (struck, destroyed, a ship that lives on).
    let mut told: BTreeMap<(EntityId, bool), (usize, usize, Option<EntityId>)> = BTreeMap::new();
    let mut systems: BTreeMap<EntityId, u32> = BTreeMap::new();
    for s in world.arrival_strikes().iter().filter(|s| s.tick > after) {
        let mine = s.owner == me;
        if !mine && s.ship_owner != me {
            continue;
        }
        systems.insert(s.inhibitor, s.system);
        let entry = told.entry((s.inhibitor, mine)).or_default();
        entry.0 += 1;
        entry.1 += usize::from(s.destroyed);
        if !s.destroyed && entry.2.is_none() && world.entity(s.ship).is_some() {
            entry.2 = Some(s.ship);
        }
    }
    told.into_iter()
        .map(|((inhibitor, mine), (struck, destroyed, alive))| {
            // Where it stands: its planet while it does, else its system.
            let place = world
                .entity(inhibitor)
                .and_then(|e| world.body(e.pos.body))
                .map(|b| b.name.clone())
                .or_else(|| {
                    world
                        .systems()
                        .iter()
                        .find(|s| Some(&s.id) == systems.get(&inhibitor))
                        .map(|s| s.name.clone())
                })
                .unwrap_or_default();
            let lost = match destroyed {
                0 => String::new(),
                n if n == struck => ": destroyed".to_owned(),
                n => format!(": {n} destroyed"),
            };
            let text = match (mine, struck) {
                (true, 1) => format!("Jump inhibitor at {place} struck an arriving ship{lost}"),
                (true, n) => format!("Jump inhibitor at {place} struck {n} arriving ships{lost}"),
                (false, 1) => {
                    format!("Enemy jump inhibitor at {place} struck your arriving ship{lost}")
                }
                (false, n) => format!(
                    "Enemy jump inhibitor at {place} struck {n} of your arriving ships{lost}"
                ),
            };
            Notice {
                text,
                about: if mine {
                    world.entity(inhibitor).map(|e| e.id)
                } else {
                    alive
                },
            }
        })
        .collect()
}

/// Remembers ships and claims of player one and words what changed.
#[derive(Debug)]
pub struct Notices {
    ships: BTreeSet<EntityId>,
    claims: BTreeMap<BodyId, PlayerId>,
    /// Ships of player one between systems, with the system each is bound for.
    travelling: BTreeMap<EntityId, u32>,
    /// The rank of player one's map scale, to explain each new reach once.
    reach: Option<i64>,
    /// The routes player one knows.
    charted: BTreeSet<(u32, u32)>,
    /// Orders of player one that an enemy jump inhibitor ended, already told.
    held: BTreeSet<rom_sim::OrderId>,
    /// The tick of the latest arrival strike already told (task 104).
    struck: u64,
    /// The tick of player one's latest alert already told (task 118).
    alerted: u64,
    /// Whether [`Notices::alerts`] has been called once.
    alerts_started: bool,
    /// The person's player.
    me: PlayerId,
    started: bool,
}

impl Default for Notices {
    /// The standalone player's notices.
    fn default() -> Self {
        Self::for_player(PLAYER)
    }
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
    /// Notices for `me`, the person's player.
    pub fn for_player(me: PlayerId) -> Self {
        Self {
            ships: BTreeSet::new(),
            claims: BTreeMap::new(),
            travelling: BTreeMap::new(),
            reach: None,
            charted: BTreeSet::new(),
            held: BTreeSet::new(),
            struck: 0,
            alerted: 0,
            alerts_started: false,
            me,
            started: false,
        }
    }

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
            .find(|p| p.id == self.me)
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
            .filter(|t| t.entity.owner == self.me)
            .map(|t| (t.entity.id, t.destination))
            .collect();
        if self.started {
            for transit in world.transits().filter(|t| t.entity.owner == self.me) {
                if !self.travelling.contains_key(&transit.entity.id) {
                    let charting = !world.knows_route(self.me, transit.origin, transit.destination);
                    notices.push(Notice {
                        text: format!(
                            "{} departed for {}{}",
                            rules.display_name(transit.entity.type_id, faction),
                            name_of(transit.destination),
                            if charting {
                                " (charting the route: a slower first voyage)"
                            } else {
                                ""
                            }
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
        let charted: BTreeSet<(u32, u32)> = world.known_routes(self.me).into_iter().collect();
        if self.started {
            let new: Vec<_> = charted.difference(&self.charted).copied().collect();
            match new.as_slice() {
                [] => {}
                [(a, b)] => notices.push(Notice {
                    text: format!("Route charted between {} and {}", name_of(*a), name_of(*b)),
                    about: None,
                }),
                more => notices.push(Notice {
                    text: format!("{} routes revealed on the star map", more.len()),
                    about: None,
                }),
            }
            for failure in world
                .order_failures(self.me)
                .filter(|f| f.reason == "inhibited" && !self.held.contains(&f.order))
            {
                notices.push(Notice {
                    text: "Held by an enemy jump inhibitor: leave the way you came, or destroy it"
                        .to_owned(),
                    about: Some(failure.executor),
                });
            }
        }
        self.held = world
            .order_failures(self.me)
            .filter(|f| f.reason == "inhibited")
            .map(|f| f.order)
            .collect();
        if self.started {
            notices.extend(strike_notices(self.me, world, self.struck));
        }
        self.struck = world
            .arrival_strikes()
            .last()
            .map_or(self.struck, |s| s.tick.max(self.struck));
        self.charted = charted;
        let reach = world.players().find(|p| p.id == self.me).and_then(|p| {
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

    /// Player one's alerts raised since the last call (task 118): a city or army under attack,
    /// foreign ships jumping in. The first call only remembers, as [`Notices::update`].
    pub fn alerts(&mut self, world: &World) -> Vec<Notice> {
        let lines = if self.alerts_started {
            crate::alert_view::alert_notices(world, self.me, self.alerted)
        } else {
            Vec::new()
        };
        self.alerted = crate::alert_view::latest_alert_tick(world, self.me, self.alerted);
        self.alerts_started = true;
        lines
    }

    fn changes(&mut self, world: &World) -> Vec<String> {
        let rules = world.rules();
        let faction = world
            .players()
            .find(|p| p.id == self.me)
            .map_or(rom_rules::DefId(0), |p| p.faction);
        let mut lines = Vec::new();
        let mut ships = BTreeSet::new();
        for e in world.entities().filter(|e| e.owner == self.me) {
            let in_space = world
                .body(e.pos.body)
                .is_some_and(|b| b.kind == BodyKind::Star);
            if in_space && rules.entity_types[&e.type_id].movement.is_some() {
                ships.insert(e.id);
                // Orbital craft come and go between the air and orbit: no notice for them.
                if self.started && !self.ships.contains(&e.id) && !world.is_craft_type(e.type_id) {
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
                .filter(|t| t.entity.owner == self.me)
                .map(|t| t.entity.id),
        );
        let claims: BTreeMap<BodyId, PlayerId> = world.claims().collect();
        if self.started {
            for (body, change) in claim_changes(self.me, &self.claims, &claims) {
                let Some(view) = resource_view(self.me, world, body) else {
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
            claim_changes(PLAYER, &before, &now),
            vec![
                (b(1), ClaimChange::Lost),
                (b(2), ClaimChange::Won),
                (b(3), ClaimChange::Won)
            ]
        );
        assert!(claim_changes(PLAYER, &now, &now).is_empty());
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
    fn the_system_index_gives_what_the_scans_gave() {
        let (_content, game) = crate::match_setup::universe_game_for_tests();
        let world = &game.world;
        let index = SystemIndex::new(world);
        assert!(!world.systems().is_empty());
        for system in world.systems() {
            let scanned: Vec<BodyId> = world
                .bodies()
                .iter()
                .filter(|b| b.system == system.id)
                .map(|b| b.id)
                .collect();
            assert_eq!(index.bodies(system.id), scanned, "bodies of {}", system.id);
            assert_eq!(index.star(system.id), world.space_of(system.id));
            assert_eq!(index.frame(system.id), SpaceFrame::of(world, system.id));
        }
        // The scan of the old known-systems rule, for the systems whose bodies the team saw.
        let seen_by_scan: BTreeSet<u32> = world
            .systems()
            .iter()
            .filter(|system| {
                world
                    .bodies()
                    .iter()
                    .filter(|b| b.system == system.id)
                    .filter_map(|b| world.body_position(b.id))
                    .any(|at| {
                        world.is_explored_on_body(
                            PLAYER,
                            world.space_of(system.id).unwrap(),
                            0,
                            at.x.div_euclid(1000),
                            at.y.div_euclid(1000),
                        )
                    })
            })
            .map(|s| s.id)
            .collect();
        let known: BTreeSet<u32> = known_systems(PLAYER, world).into_iter().collect();
        assert!(seen_by_scan.is_subset(&known));
        assert_eq!(
            space_snapshot_with(PLAYER, world, &index, 1, &[]).known,
            space_snapshot(world, 1, &[]).known
        );
    }

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
        let known = known_systems(PLAYER, &game.world);
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
        // A first voyage charts its route, and the line says so.
        assert!(
            departed.text.ends_with(&format!(
                "{} (charting the route: a slower first voyage)",
                name(other)
            )),
            "{}",
            departed.text
        );
        let arrived = arrived.expect("an arrival line");
        assert_eq!(arrived.about, Some(ship));
        // Its destination is known now: its bodies are drawn and named.
        assert!(known_systems(PLAYER, &game.world).contains(&other));
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
mod strike_tests {
    use super::*;
    use rom_sim::Command;

    /// Task 104: player two's jump inhibitor on player one's home planet strikes player one's
    /// ships as they jump home: the corvette is destroyed, the battleship hurt; the event line
    /// tells it, the system view flashes, and the hover over the held system says so.
    #[test]
    fn an_inhibitor_strike_is_told_and_shown() {
        let (content, mut game) = crate::match_setup::universe_game_for_tests();
        let world = &mut game.world;
        let rules = world.rules().clone();
        let id = |k: &str| rules.id_of(k).unwrap();
        let home = world.home_body(PLAYER).unwrap();
        let home_system = world.body(home).unwrap().system;
        let foe = world
            .players()
            .map(|p| p.id)
            .find(|p| world.enemies(PLAYER, *p) && *p != PlayerId(0))
            .unwrap();
        let city = world
            .entities()
            .find(|e| e.owner == PLAYER && e.pos.body == home)
            .unwrap()
            .pos;
        let inhibitor = [(12_000, 0), (0, 12_000), (-12_000, 0), (0, -12_000)]
            .into_iter()
            .find_map(|(dx, dy)| {
                world
                    .place_for_setup(id("bld.jump_inhibitor"), foe, world.offset(city, dx, dy))
                    .ok()
            })
            .expect("room for the inhibitor");
        let other = content
            .universe
            .routes
            .iter()
            .find_map(|r| {
                (r.system_a == home_system)
                    .then_some(r.system_b)
                    .or((r.system_b == home_system).then_some(r.system_a))
            })
            .unwrap();
        let edge = world.system_edge(other, home_system).unwrap();
        let corvette = world.spawn(id("unit.corvette"), PLAYER, edge).unwrap();
        let battleship = world.spawn(id("unit.battleship"), PLAYER, edge).unwrap();
        let mut notices = Notices::default();
        notices.update(world);
        // Into the explored home system they come out beside the held planet (task 111).
        let target = world.body_position(home).unwrap();
        for ship in [corvette, battleship] {
            world
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
        }
        let mut lines = Vec::new();
        let mut flashed = false;
        for _ in 0..12_000 {
            world.step();
            lines.extend(notices.update(world));
            let sightings = world.visible_entities(PLAYER);
            let space = space_snapshot(world, home_system, &sightings);
            flashed |= space.strikes.iter().any(|s| s.system == home_system);
            if world.entity(battleship).is_some_and(|e| {
                world
                    .body(e.pos.body)
                    .is_some_and(|b| b.system == home_system)
            }) {
                break;
            }
        }
        let damage = rules.entity_types[&id("bld.jump_inhibitor")]
            .inhibitor
            .as_ref()
            .unwrap()
            .arrival_damage;
        assert!(
            world.entity(corvette).is_none(),
            "the corvette is destroyed"
        );
        let e = world.entity(battleship).unwrap();
        assert_eq!(e.hp, e.max_hp - damage);
        assert!(flashed, "the system view flashes where they arrived");
        let place = world.body(home).unwrap().name.clone();
        let struck: Vec<_> = lines
            .iter()
            .filter(|n| n.text.starts_with("Enemy jump inhibitor"))
            .collect();
        assert!(!struck.is_empty(), "{lines:?}");
        assert!(struck.iter().all(|n| n.text.contains(&place)), "{struck:?}");
        assert!(
            struck.iter().any(|n| n.text.contains("destroyed")),
            "{struck:?}"
        );
        // The hover over the held system says that arriving ships are struck, and for how much.
        let holds = holds(PLAYER, world, &SystemIndex::new(world));
        let hold = holds.iter().find(|h| h.system == home_system).unwrap();
        assert!(hold.enemy && i64::from(hold.arrival_damage) == damage);
        let (_, what) = rom_render::space::hold_text(hold);
        assert!(
            what.contains(&format!("struck for {damage} hit points")),
            "{what}"
        );
        // The inhibitor's own owner hears of it too, once the line is from its side.
        assert!(world.entity(inhibitor).is_some());
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
        let before = resource_view(PLAYER, world, body.id).unwrap();
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
        let claimed = resource_view(PLAYER, world, body.id).unwrap();
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

#[cfg(test)]
mod model_tests {
    use super::*;
    use crate::looks::ModelBook;

    /// Ships in space wear the model the planet view gives them (their owner's faction and
    /// age), and in the galactic age the five starship classes each have one of their own.
    #[test]
    fn ships_in_space_wear_their_planet_models() {
        let (_content, mut game) = crate::match_setup::universe_game_for_tests();
        let world = &mut game.world;
        let home = world.home_body(PLAYER).unwrap();
        let system = world.body(home).unwrap().system;
        let star = world.space_of(system).unwrap();
        let classes = [
            "unit.survey_ship",
            "unit.corvette",
            "unit.space_frigate",
            "unit.space_cruiser",
            "unit.battleship",
        ];
        for (i, key) in classes.iter().enumerate() {
            let kind = world.rules().id_of(key).unwrap();
            let mut at = world.body_position(star).unwrap();
            at.x += 20_000 + 3_000 * i as i32;
            world.spawn(kind, PLAYER, at).unwrap();
        }
        let models = ModelBook::new(world.rules());
        let mut space = space_snapshot(world, system, &world.visible_entities(PLAYER));
        dress_ships(&mut space, world, &models);
        let player = world.players().find(|p| p.id == PLAYER).unwrap();
        assert!(space.ships.len() >= classes.len());
        for ship in &space.ships {
            let kind = rom_rules::DefId(u32::from(ship.kind));
            assert_eq!(ship.model, models.resolve(kind, player.faction, player.age));
        }
        let galactic = world.rules().id_of("age.galactic").unwrap();
        let mut indices: Vec<u16> = classes
            .iter()
            .map(|key| {
                let kind = world.rules().id_of(key).unwrap();
                models
                    .resolve(kind, player.faction, galactic)
                    .unwrap_or_else(|| panic!("{key} has a model"))
                    .index
            })
            .collect();
        indices.sort_unstable();
        indices.dedup();
        assert_eq!(indices.len(), classes.len());
    }
}

#[cfg(test)]
mod trade_tests {
    use super::*;
    use crate::script::ScriptSetup;
    use rom_sim::Command;

    /// A space freighter of player one between its home city and a colony (`far`: on a planet
    /// of the nearest star system one route away, with interstellar trade; otherwise on the
    /// nearest other planet of the home system). Returns the route's length in milli-tiles,
    /// its income per arrival and the wealth it earns per game minute, measured over a whole
    /// round trip once it runs (its first voyage, charting, left out).
    fn trade_rate(far: bool) -> (u64, i64, i64) {
        let options = crate::args::Options {
            universe: true,
            // Seed 2: since task 109 seed 1's Cluster has no room for a city on the nearest
            // other planet of the home system.
            seed: 2,
            ..crate::args::Options::default()
        };
        let content = crate::terrain::generate_universe_content(&options).expect("universe");
        let setup = ScriptSetup {
            age: Some("age.abundance".into()),
            techs: vec!["tech.interstellar_trade".into()],
            colonies: if far { vec![] } else { vec![1] },
            far_colonies: if far { vec![1] } else { vec![] },
            ..ScriptSetup::default()
        };
        let mut game = crate::match_setup::build_universe_match(
            &content.rules,
            &content.universe,
            &content.factions,
            1,
            None,
            &setup,
        )
        .expect("match");
        let world = &mut game.world;
        let home = world.home_body(PLAYER).unwrap();
        let kind = world.rules().id_of("unit.space_freighter").unwrap();
        let freighter = world
            .spawn(kind, PLAYER, world.body_position(home).unwrap())
            .unwrap();
        let city = world.city_type(PLAYER).unwrap();
        let colony = world
            .entities()
            .find(|e| e.owner == PLAYER && e.type_id == city && e.pos.body != home)
            .unwrap()
            .id;
        let submit = |world: &mut World, kind: &str, target: Option<EntityId>| {
            world
                .submit(Command {
                    player: PLAYER,
                    executor: freighter,
                    kind: kind.into(),
                    subject: None,
                    target_entity: target,
                    target_pos: None,
                    queue: false,
                })
                .unwrap();
        };
        let marker = world.body_marker(home);
        submit(world, "orbit", marker);
        for _ in 0..2000 {
            if world.orbit(freighter).is_some() {
                break;
            }
            world.step();
        }
        submit(world, "trade", Some(colony));
        world.step();
        let route = world.trade_routes(PLAYER).next().unwrap();
        let mut paid = Vec::new();
        let mut trips = world.stats().trade_trips;
        for _ in 0..80_000 {
            world.step();
            if world.stats().trade_trips > trips {
                trips = world.stats().trade_trips;
                paid.push(world.tick());
                if paid.len() == 3 {
                    break;
                }
            }
        }
        assert_eq!(paid.len(), 3, "three deliveries");
        let round_trip = paid[2] - paid[0];
        let per_minute = 2 * route.per_trip_income * 1200 / round_trip as i64;
        eprintln!(
            "{}: length {} milli-tiles, +{} per trip, round trip {} s, {} wealth per minute",
            if far {
                "interstellar"
            } else {
                "between planets"
            },
            route.length,
            route.per_trip_income,
            round_trip / 20,
            per_minute
        );
        (route.length, route.per_trip_income, per_minute)
    }

    /// An interstellar route pays more per arrival than a route between the planets of one
    /// system, and is worth the longer trip: more wealth per minute.
    #[test]
    fn interstellar_trade_is_worth_the_longer_trip() {
        let (near_length, near_trip, near_rate) = trade_rate(false);
        let (far_length, far_trip, far_rate) = trade_rate(true);
        assert!(far_length > near_length && far_trip > near_trip);
        assert!(
            far_rate > near_rate,
            "interstellar {far_rate} per minute against {near_rate}"
        );
    }
}
