//! The space manager, from the first age whose map scale reaches beyond the planet:
//! ships keep station at resource bodies so they pay, warships escort the claims, contest
//! the enemy's and strike enemy orbits, and transports carry builders and troops to other
//! planets, land them and found colonies (or put troops ashore beside an enemy city).
//!
//! Ship types are told apart by their rows: a ship that carries the tag a presence body
//! asks for can claim, a ship with weapons fights, an unarmed carrier is a transport, a
//! carrier whose container `launches` carries orbital craft (units whose movement climbs to
//! orbit). Craft climb to their planet's orbit and defend it; once a launching carrier is
//! in service they board it at home, travel with the escort and are launched over its
//! target. Ground batteries are ordinary defensive buildings for the construction manager.

use crate::{
    Weights,
    army::fighters,
    memory::{Job, Memory, Phase, Role, Voyage},
    plan::{Act, Plan, Wish},
    view::{self, View, operational},
};
use rom_rules::{BodyKind, ClaimMode, DefId, ProductionMode};
use rom_sim::{BodyClaim, BodyId, Entity, EntityId, Pos};
use std::collections::BTreeSet;

/// Score of the wish for a building that trains spacecraft when the player has none.
const SPACEPORT_SCORE: i64 = 620;
/// Score of the wish for a transport when a colony is waiting for one.
const TRANSPORT_SCORE: i64 = 540;
/// Ticks a transport waits at home for its passengers before it leaves with what it has.
const LOAD_TICKS: u64 = 1200;
/// Ticks a voyage stage may take before it is given up.
const STAGE_TICKS: u64 = 12_000;
/// Warships the escort keeps together before it strikes an enemy orbit.
const STRIKE_SHIPS: usize = 3;
/// Troops a transport takes along besides its builder when there is room.
const ESCORT_TROOPS: usize = 4;

/// The map-scale rank of the player's age: 0 on its planet, 1 in its star system, 2 and
/// more between systems.
pub(crate) fn rank(view: &View<'_>) -> i64 {
    view.rules
        .ages
        .get(&view.age)
        .and_then(|a| view.rules.map_scales.get(&a.map_scale))
        .map_or(0, |s| s.rank)
}

/// Spacecraft types the player can train now, by what they are for.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Ships {
    /// The cheapest type that claims presence bodies.
    pub claimer: Option<DefId>,
    /// The armed type with the most strength per resource (it may be the claimer).
    pub warship: Option<DefId>,
    /// The carrier with the most room.
    pub transport: Option<DefId>,
    /// The carrier that launches orbital craft with the most room.
    pub launcher: Option<DefId>,
    /// The orbital craft with the most strength per resource.
    pub craft: Option<DefId>,
}

/// Tags that let a unit claim a presence body.
fn claim_tags(view: &View<'_>) -> BTreeSet<u32> {
    view.rules
        .entity_types
        .values()
        .filter_map(|t| t.node.as_ref())
        .filter(|n| n.claim_mode == ClaimMode::Presence)
        .filter_map(|n| n.claim_tag_id)
        .collect()
}

/// Whether a type claims presence bodies.
fn claims(view: &View<'_>, tags: &BTreeSet<u32>, type_id: DefId) -> bool {
    tags.iter().any(|tag| view.rules.has_tag(type_id, *tag))
}

fn price_sum(view: &View<'_>, def: DefId) -> i64 {
    view.price(def).iter().map(|(_, a)| *a).sum::<i64>().max(1)
}

/// What a spacecraft type is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// A ship that fights or keeps station: armed, or able to claim, and no transport.
    Ship,
    /// An unarmed carrier.
    Transport,
    /// A carrier that launches orbital craft.
    Launcher,
    /// Anything else: unarmed ships that carry nobody.
    Other,
}

/// What a spacecraft type is for.
fn kind(view: &View<'_>, tags: &BTreeSet<u32>, type_id: DefId) -> Kind {
    let et = &view.rules.entity_types[&type_id];
    if et.container.as_ref().is_some_and(|c| c.launches) {
        Kind::Launcher
    } else if et.weapons.is_empty() && view.world.is_carrier_type(type_id) {
        Kind::Transport
    } else if !et.weapons.is_empty() || claims(view, tags, type_id) {
        Kind::Ship
    } else {
        Kind::Other
    }
}

impl Ships {
    /// Reads the trainable spacecraft from the producers the player owns.
    pub fn new(view: &View<'_>) -> Self {
        let rules = view.rules;
        let tags = claim_tags(view);
        let mut types: Vec<DefId> = view
            .owned
            .iter()
            .flat_map(|p| rules.production.get(p).into_iter().flatten())
            .filter(|row| row.mode == ProductionMode::Train)
            .map(|row| row.product_def_id)
            .filter(|unit| {
                view.world.is_spacecraft_type(*unit)
                    && view.unlocked(*unit)
                    && !view.obsolete(*unit)
            })
            .collect();
        types.sort_unstable();
        types.dedup();
        let of_kind = |k: Kind| -> Vec<DefId> {
            types
                .iter()
                .copied()
                .filter(|t| kind(view, &tags, *t) == k)
                .collect()
        };
        Ships {
            claimer: of_kind(Kind::Ship)
                .into_iter()
                .filter(|t| claims(view, &tags, *t))
                .min_by_key(|t| (price_sum(view, *t), *t)),
            warship: of_kind(Kind::Ship)
                .into_iter()
                .filter(|t| !rules.entity_types[t].weapons.is_empty())
                .max_by_key(|t| {
                    (
                        view::strength(rules, *t) * 100 / price_sum(view, *t),
                        std::cmp::Reverse(*t),
                    )
                }),
            transport: of_kind(Kind::Transport).into_iter().max_by_key(|t| {
                (
                    rules.entity_types[t]
                        .container
                        .as_ref()
                        .map_or(0, |c| c.capacity),
                    std::cmp::Reverse(price_sum(view, *t)),
                    std::cmp::Reverse(*t),
                )
            }),
            launcher: of_kind(Kind::Launcher).into_iter().max_by_key(|t| {
                (
                    rules.entity_types[t]
                        .container
                        .as_ref()
                        .map_or(0, |c| c.capacity),
                    std::cmp::Reverse(*t),
                )
            }),
            craft: view
                .owned
                .iter()
                .flat_map(|p| rules.production.get(p).into_iter().flatten())
                .filter(|row| row.mode == ProductionMode::Train)
                .map(|row| row.product_def_id)
                .filter(|unit| {
                    view.world.is_craft_type(*unit) && view.unlocked(*unit) && !view.obsolete(*unit)
                })
                .max_by_key(|t| {
                    (
                        view::strength(rules, *t) * 100 / price_sum(view, *t),
                        std::cmp::Reverse(*t),
                    )
                }),
        }
    }
}

/// The body the player's capital stands on.
fn home(view: &View<'_>) -> Option<BodyId> {
    view.cities.first().map(|c| c.pos.body)
}

/// The system a body belongs to.
fn system_of(view: &View<'_>, body: BodyId) -> Option<u32> {
    view.world.body(body).map(|b| b.system)
}

/// Systems a ship of the player can reach: its home system, and those joined to it by
/// routes once the map scale allows travel between systems.
fn reachable(view: &View<'_>) -> BTreeSet<u32> {
    let Some(start) = home(view).and_then(|b| system_of(view, b)) else {
        return BTreeSet::new();
    };
    let mut seen = BTreeSet::from([start]);
    if rank(view) < 2 {
        return seen;
    }
    let mut open = vec![start];
    while let Some(system) = open.pop() {
        for route in view.world.routes() {
            let next = if route.system_a == system {
                route.system_b
            } else if route.system_b == system {
                route.system_a
            } else {
                continue;
            };
            if seen.insert(next) {
                open.push(next);
            }
        }
    }
    seen
}

/// Own ships on a surface or in space now, in id order.
fn own_ships<'a>(view: &View<'a>) -> Vec<&'a Entity> {
    view.own
        .iter()
        .copied()
        .filter(|e| view.world.is_spacecraft_type(e.type_id) && operational(e) && e.hp > 0)
        .collect()
}

/// Whether a ship is still the player's, here or between systems.
fn exists(view: &View<'_>, id: u32) -> bool {
    view.world
        .entity(EntityId(id))
        .is_some_and(|e| e.owner == view.me && e.hp > 0)
        || view
            .world
            .transits()
            .any(|t| t.entity.id.0 == id && t.entity.owner == view.me)
}

/// Resource bodies in reach that pay through presence, with their claim.
fn resource_bodies(view: &View<'_>, systems: &BTreeSet<u32>) -> Vec<(BodyId, BodyClaim)> {
    view.world
        .bodies()
        .iter()
        .filter(|b| b.kind == BodyKind::Resource && systems.contains(&b.system))
        .filter(|b| view.world.body_yield(b.id, view.me).is_some())
        .map(|b| (b.id, view.world.body_claim(b.id)))
        .collect()
}

/// Habitable bodies in reach where the player has no city, the home planet left out:
/// those without known enemies first, then by system and id.
fn colony_targets(view: &View<'_>, memory: &Memory, systems: &BTreeSet<u32>) -> Vec<BodyId> {
    let home = home(view);
    let mut out: Vec<(bool, u32, BodyId)> = view
        .world
        .bodies()
        .iter()
        .filter(|b| {
            b.kind == BodyKind::Habitable
                && systems.contains(&b.system)
                && Some(b.id) != home
                && !view.cities.iter().any(|c| c.pos.body == b.id)
        })
        .map(|b| {
            let hostile = memory
                .enemies
                .values()
                .any(|e| e.building && e.pos.body == b.id);
            (hostile, b.system, b.id)
        })
        .collect();
    out.sort_unstable();
    out.into_iter().map(|(_, _, b)| b).collect()
}

/// The building that trains spacecraft, when the player can build one and has none.
fn spaceport(view: &View<'_>) -> Option<DefId> {
    let rules = view.rules;
    view.owned
        .iter()
        .filter(|p| rules.entity_types[*p].builder.is_some())
        .flat_map(|p| rules.production.get(p).into_iter().flatten())
        .filter(|row| row.mode == ProductionMode::Construct)
        .map(|row| row.product_def_id)
        .filter(|b| {
            view.unlocked(*b)
                && !view.obsolete(*b)
                && rules.production.get(b).into_iter().flatten().any(|row| {
                    row.mode == ProductionMode::Train
                        && view.world.is_spacecraft_type(row.product_def_id)
                })
        })
        .min()
        .filter(|port| view.line_count(*port) == 0)
}

/// Wishes of the space manager: a spaceport, ships for the free bodies, warships for the
/// claims and a strike force, and a transport when a colony waits.
pub(crate) fn wishes(view: &View<'_>, weights: &Weights, memory: &Memory, wishes: &mut Vec<Wish>) {
    if rank(view) < 1 || view.world.systems().is_empty() {
        return;
    }
    let ships = Ships::new(view);
    if ships == Ships::default() {
        // Nothing to fly yet: the building that trains spacecraft.
        if let Some(port) = spaceport(view) {
            wishes.push(Wish {
                score: SPACEPORT_SCORE,
                act: Act::Construct(port),
                reserve: false,
                opening: false,
            });
        }
        return;
    }
    let systems = reachable(view);
    let bodies = resource_bodies(view, &systems);
    let tags = claim_tags(view);
    // Ships that fight or keep station, with those in training.
    let fleet: i64 = view
        .have
        .iter()
        .filter(|(t, _)| view.world.is_spacecraft_type(**t) && kind(view, &tags, **t) == Kind::Ship)
        .map(|(_, n)| *n)
        .sum();
    let count =
        |test: &dyn Fn(&BodyClaim) -> bool| bodies.iter().filter(|(_, c)| test(c)).count() as i64;
    let free = count(&|c| *c == BodyClaim::Unclaimed);
    let held = count(&|c| *c == BodyClaim::Held(view.me));
    let contested =
        count(&|c| matches!(c, BodyClaim::Held(p) if *p != view.me) || *c == BodyClaim::Contested);
    // Eagerness decides how many of the free bodies are worth a ship.
    let claim_want = (held + (free * weights.claim + 99) / 100).min(held + free);
    let escort_want = STRIKE_SHIPS as i64 + contested;
    let wish = if fleet < claim_want {
        ships
            .claimer
            .or(ships.warship)
            .map(|t| (t, 480 + 3 * weights.claim))
    } else if fleet < claim_want + escort_want {
        ships
            .warship
            .map(|t| (t, 440 + weights.claim + weights.army_share))
    } else {
        None
    };
    if let Some((unit, score)) = wish {
        wishes.push(Wish {
            score,
            act: Act::Train(unit),
            reserve: false,
            opening: false,
        });
    }
    // A launching carrier once the strike force stands, and craft to fill it.
    if let Some(launcher) = ships.launcher
        && view.count(launcher) == 0
        && fleet >= STRIKE_SHIPS as i64
    {
        wishes.push(Wish {
            score: 430 + weights.army_share,
            act: Act::Train(launcher),
            reserve: false,
            opening: false,
        });
    }
    if let Some(craft) = ships.craft {
        let room: i64 = view
            .own
            .iter()
            .filter(|e| kind(view, &tags, e.type_id) == Kind::Launcher)
            .map(|e| view.world.carry_capacity(e.id))
            .sum();
        let have: i64 = view
            .have
            .iter()
            .filter(|(t, _)| view.world.is_craft_type(**t))
            .map(|(_, n)| *n)
            .sum();
        if have < room {
            wishes.push(Wish {
                score: 420 + weights.army_share,
                act: Act::Train(craft),
                reserve: false,
                opening: false,
            });
        }
    }
    if let Some(transport) = ships.transport
        && view.count(transport) == 0
        && !colony_targets(view, memory, &systems).is_empty()
        && city_room(view)
    {
        wishes.push(Wish {
            score: TRANSPORT_SCORE,
            act: Act::Train(transport),
            reserve: false,
            opening: false,
        });
    }
}

/// Whether the city limit leaves room for another city.
fn city_room(view: &View<'_>) -> bool {
    view.world
        .city_type(view.me)
        .is_some_and(|city| view.line_count(city) < view.city_limit())
}

/// Sends a ship toward a body: an orbit order in its system, a route journey otherwise.
/// Returns whether it is there (in orbit of a habitable body, or beside a resource body).
fn fly(plan: &mut Plan<'_>, ship: &Entity, body: BodyId, reach: u64) -> bool {
    let view = &plan.view;
    if view.world.orbit(ship.id).is_some_and(|o| o.body == body) {
        return true;
    }
    let (Some(here), Some(there)) = (system_of(view, ship.pos.body), system_of(view, body)) else {
        return false;
    };
    let Some(marker) = view.world.body_marker(body) else {
        return false;
    };
    let habitable = view
        .world
        .body(body)
        .is_some_and(|b| b.kind == BodyKind::Habitable);
    if here == there
        && !habitable
        && view
            .world
            .body_position(body)
            .is_some_and(|at| view.distance(ship.pos, at) <= reach)
    {
        return true;
    }
    let busy = ship.orders().front().is_some_and(|o| {
        (o.kind == "orbit" && o.target_entity == Some(marker))
            || o.kind == "travel"
            || o.kind == "launch"
    });
    if busy || view.busy.contains(&ship.id) {
        return false;
    }
    if here == there {
        plan.order(ship.id, "orbit", None, Some(marker), None);
    } else if let Some(star) = view
        .world
        .space_of(there)
        .and_then(|s| view.world.body_position(s))
    {
        plan.order(ship.id, "travel", None, None, Some(star));
    }
    false
}

/// Leads the ships for one thought.
pub(crate) fn command(plan: &mut Plan<'_>, memory: &mut Memory, weights: &Weights) {
    if rank(&plan.view) < 1 || plan.view.world.systems().is_empty() {
        return;
    }
    let tick = plan.view.tick;
    memory.voyages.retain(|id, _| exists(&plan.view, *id));
    let ships = own_ships(&plan.view);
    let tags = claim_tags(&plan.view);
    let systems = reachable(&plan.view);
    for ship in &ships {
        if let Some(orbit) = plan.view.world.orbit(ship.id) {
            memory.visited.insert(orbit.body.0);
        }
    }
    let of_kind = |k: Kind| -> Vec<&Entity> {
        ships
            .iter()
            .copied()
            .filter(|s| kind(&plan.view, &tags, s.type_id) == k)
            .collect()
    };
    let (fighting, transports, launchers) = (
        of_kind(Kind::Ship),
        of_kind(Kind::Transport),
        of_kind(Kind::Launcher),
    );
    // Ships that can claim keep station at the free bodies first; the rest escort.
    let claimers: Vec<&Entity> = fighting
        .iter()
        .copied()
        .filter(|s| claims(&plan.view, &tags, s.type_id))
        .collect();
    claim(plan, memory, &claimers, &systems);
    // A carrier waits at home while craft there still have to board it.
    let waiting = crafts(plan, &launchers);
    let warships: Vec<&Entity> = fighting
        .into_iter()
        .filter(|s| !memory.voyages.contains_key(&s.id.0))
        .chain(launchers.into_iter().filter(|l| !waiting.contains(&l.id)))
        .collect();
    escort(plan, memory, &warships, &systems);
    colonise(plan, memory, &transports, &systems, weights, tick);
}

/// Orbital craft: idle on the ground they climb to their planet's orbit; in orbit beside a
/// launching carrier of their planet that has room, they board it. Returns the carriers
/// still waiting for craft at home.
fn crafts(plan: &mut Plan<'_>, launchers: &[&Entity]) -> BTreeSet<EntityId> {
    let view = &plan.view;
    let craft: Vec<&Entity> = view
        .own
        .iter()
        .copied()
        .filter(|e| {
            view.world.is_craft_type(e.type_id)
                && operational(e)
                && e.container_id.is_none()
                && e.orders().is_empty()
                && !view.busy.contains(&e.id)
        })
        .collect();
    let mut waiting = BTreeSet::new();
    let mut orders: Vec<(EntityId, &str, Option<EntityId>)> = Vec::new();
    let mut room: Vec<(EntityId, BodyId, i64)> = launchers
        .iter()
        .filter_map(|l| {
            let orbit = view.world.orbit(l.id)?;
            let free = view.world.carry_capacity(l.id) - view.world.carried_load(l.id);
            Some((l.id, orbit.body, free))
        })
        .collect();
    for c in craft {
        let planet = view.world.orbit(c.id).map_or(c.pos.body, |o| o.body);
        let pop = view.rules.entity_types[&c.type_id].pop_cost.max(0);
        if let Some(slot) = room
            .iter_mut()
            .find(|(_, body, free)| *body == planet && *free >= pop)
        {
            slot.2 -= pop;
            waiting.insert(slot.0);
            orders.push((c.id, "enter", Some(slot.0)));
        } else if view.world.orbit(c.id).is_none() {
            orders.push((c.id, "lift", None));
        }
    }
    for (id, kind, target) in orders {
        if !plan.order(id, kind, None, target, None) {
            break;
        }
    }
    waiting
}

/// Ships keeping station stay; others go to the most valuable free body in reach that no
/// other ship is bound for. Ships with nowhere to go are left to the escort.
fn claim(plan: &mut Plan<'_>, memory: &mut Memory, claimers: &[&Entity], systems: &BTreeSet<u32>) {
    let mut taken: BTreeSet<u32> = memory
        .voyages
        .values()
        .filter(|v| v.job == Job::Claim)
        .map(|v| v.body)
        .collect();
    let free: Vec<BodyId> = resource_bodies(&plan.view, systems)
        .into_iter()
        .filter(|(_, c)| *c == BodyClaim::Unclaimed)
        .map(|(b, _)| b)
        .collect();
    for ship in claimers.iter().copied() {
        let body = match memory.voyages.get(&ship.id.0) {
            Some(v) if v.job == Job::Claim => BodyId(v.body),
            Some(_) => continue,
            None => {
                let view = &plan.view;
                let best = free
                    .iter()
                    .filter(|b| !taken.contains(&b.0))
                    .filter_map(|b| {
                        let paid = view.world.body_yield(*b, view.me)?.per_period;
                        let at = view.world.body_position(*b)?;
                        let tiles = (view.distance(ship.pos, at) / 1000).min(1 << 20) as i64;
                        let far = i64::from(system_of(view, *b) != system_of(view, ship.pos.body));
                        Some((paid * 1000 / (tiles + 50) / (1 + 4 * far), *b))
                    })
                    .max_by_key(|(value, b)| (*value, std::cmp::Reverse(*b)));
                let Some((_, body)) = best else {
                    continue;
                };
                taken.insert(body.0);
                memory.voyages.insert(
                    ship.id.0,
                    Voyage {
                        job: Job::Claim,
                        body: body.0,
                        stage: 1,
                        since: view.tick,
                    },
                );
                body
            }
        };
        let reach = presence_reach(&plan.view, body);
        fly(plan, ship, body, reach);
    }
}

/// The radius within which a ship holds a presence body, with some room to spare.
fn presence_reach(view: &View<'_>, body: BodyId) -> u64 {
    view.world
        .body_marker(body)
        .and_then(|m| view.world.entity(m))
        .and_then(|m| view.rules.entity_types[&m.type_id].node.as_ref())
        .map(|n| n.claim_radius)
        .filter(|r| *r > 0)
        .or_else(|| view.rules.constant("space.orbit_reach"))
        .unwrap_or(3000)
        .max(0) as u64
        * 3
        / 4
}

/// Warships: contest enemy-held bodies, escort own claims, and with enough of them strike
/// the orbit of a planet where enemy buildings are known.
fn escort(plan: &mut Plan<'_>, memory: &mut Memory, ships: &[&Entity], systems: &BTreeSet<u32>) {
    let mut army = memory.armies.remove(&Role::Escort).unwrap_or_default();
    army.members.retain(|id| exists(&plan.view, *id));
    army.members.extend(ships.iter().map(|s| s.id.0));
    if ships.is_empty() {
        memory.armies.insert(Role::Escort, army);
        return;
    }
    let view = &plan.view;
    let bodies = resource_bodies(view, systems);
    // Where enemy buildings are known on a habitable body in reach.
    let enemy_world = memory
        .enemies
        .values()
        .filter(|e| e.building)
        .map(|e| e.pos.body)
        .filter(|b| {
            view.world.body(*b).is_some_and(|info| {
                info.kind == BodyKind::Habitable && systems.contains(&info.system)
            })
        })
        .min();
    let contested = bodies
        .iter()
        .find(|(_, c)| {
            matches!(c, BodyClaim::Held(p) if *p != view.me) || *c == BodyClaim::Contested
        })
        .map(|(b, _)| *b);
    let guarded = bodies
        .iter()
        .find(|(_, c)| *c == BodyClaim::Held(view.me))
        .map(|(b, _)| *b);
    let target = match enemy_world {
        Some(world) if ships.len() >= STRIKE_SHIPS => Some(world),
        _ => contested.or(guarded),
    };
    army.target = target.map(|b| b.0);
    army.phase = if target.is_some() {
        Phase::Advance
    } else {
        Phase::Gather
    };
    let Some(target) = target else {
        memory.armies.insert(Role::Escort, army);
        return;
    };
    let reach = presence_reach(view, target);
    for ship in ships.iter().copied() {
        if !fly(plan, ship, target, reach) {
            continue;
        }
        // A carrier over its target launches its craft.
        if plan.view.world.launches_craft(ship.id)
            && plan.view.world.orbit(ship.id).is_some()
            && !plan.view.world.passengers(ship.id).is_empty()
            && ship.orders().is_empty()
            && !plan.view.busy.contains(&ship.id)
        {
            plan.order(ship.id, "unload", None, None, None);
            continue;
        }
        // Over an enemy world: fire on the nearest building or unit seen there now.
        let Some(below) = plan
            .view
            .world
            .orbit(ship.id)
            .filter(|o| o.body == target)
            .map(|o| o.pos)
        else {
            continue;
        };
        if !ship.orders().is_empty() || plan.view.busy.contains(&ship.id) {
            continue;
        }
        let mark = memory
            .enemies
            .iter()
            .filter(|(_, e)| e.pos.body == target && e.tick == plan.view.tick)
            .min_by_key(|(id, e)| (plan.view.distance(below, e.pos), **id))
            .map(|(id, _)| EntityId(*id));
        if let Some(mark) = mark
            && !plan.order(ship.id, "attack", None, Some(mark), None)
        {
            break;
        }
    }
    memory.armies.insert(Role::Escort, army);
}

/// Transports: load a builder and troops at home, fly to a planet without an own city,
/// land, found a city (troops ashore beside an enemy attack the nearest building), lift
/// off and come home for the next one.
fn colonise(
    plan: &mut Plan<'_>,
    memory: &mut Memory,
    ships: &[&Entity],
    systems: &BTreeSet<u32>,
    weights: &Weights,
    tick: u64,
) {
    let Some(home_body) = home(&plan.view) else {
        return;
    };
    for ship in ships.iter().copied() {
        let voyage = match memory.voyages.get(&ship.id.0).copied() {
            Some(v) if tick < v.since + STAGE_TICKS => v,
            stale => {
                if stale.is_some() {
                    memory.voyages.remove(&ship.id.0);
                }
                if !city_room(&plan.view) {
                    continue;
                }
                let taken: BTreeSet<u32> = memory.voyages.values().map(|v| v.body).collect();
                let Some(body) = colony_targets(&plan.view, memory, systems)
                    .into_iter()
                    .find(|b| !taken.contains(&b.0))
                else {
                    continue;
                };
                let voyage = Voyage {
                    job: Job::Colonise,
                    body: body.0,
                    stage: 0,
                    since: tick,
                };
                memory.voyages.insert(ship.id.0, voyage);
                voyage
            }
        };
        let body = BodyId(voyage.body);
        let next = |memory: &mut Memory, stage: u8| {
            memory.voyages.insert(
                ship.id.0,
                Voyage {
                    stage,
                    since: tick,
                    ..voyage
                },
            );
        };
        match voyage.stage {
            0 => {
                // Troops board only a landed ship (task 68): it comes down near a home city.
                if !(plan.view.world.is_landed(ship.id) && ship.pos.body == home_body) {
                    if !fly(plan, ship, home_body, 0) {
                        continue;
                    }
                    if !ship.orders().front().is_some_and(|o| o.kind == "land")
                        && !plan.view.busy.contains(&ship.id)
                        && let Some(site) = plan
                            .view
                            .cities
                            .iter()
                            .find(|c| c.pos.body == home_body)
                            .and_then(|c| landing(&plan.view, ship.id, c.pos))
                    {
                        plan.order(ship.id, "land", None, None, Some(site));
                    }
                    continue;
                }
                let view = &plan.view;
                let has_builder = view.world.passengers(ship.id).iter().any(|id| {
                    view.world
                        .entity(*id)
                        .is_some_and(|e| view.rules.entity_types[&e.type_id].builder.is_some())
                });
                let room = view.world.carry_capacity(ship.id) - view.world.carried_load(ship.id);
                if has_builder && (room <= 0 || tick >= voyage.since + LOAD_TICKS) {
                    next(memory, 1);
                    continue;
                }
                board(plan, memory, ship, home_body, has_builder, room, weights);
            }
            1 => {
                // Loaded on the ground: up again first (task 68).
                if plan.view.world.is_landed(ship.id) {
                    if ship.orders().is_empty() && !plan.view.busy.contains(&ship.id) {
                        plan.order(ship.id, "lift", None, None, None);
                    }
                    continue;
                }
                if fly(plan, ship, body, 0) {
                    next(memory, 2);
                }
            }
            2 => {
                if plan.view.world.is_landed(ship.id) {
                    next(memory, 3);
                    continue;
                }
                if ship.orders().front().is_some_and(|o| o.kind == "land")
                    || plan.view.busy.contains(&ship.id)
                {
                    continue;
                }
                let Some(below) = plan.view.world.orbit(ship.id).map(|o| o.pos) else {
                    // Knocked out of orbit: fly there again.
                    next(memory, 1);
                    continue;
                };
                if let Some(site) = landing(&plan.view, ship.id, below) {
                    plan.order(ship.id, "land", None, None, Some(site));
                }
            }
            _ => settle(plan, memory, ship, body),
        }
    }
}

/// A point to land on: open ground near the orbit's subpoint, else near a face centre.
fn landing(view: &View<'_>, ship: EntityId, below: Pos) -> Option<Pos> {
    let world = view.world;
    world.landing_site(ship, below).or_else(|| {
        let n = world.body(below.body)?.n? as i32;
        (0..6)
            .find_map(|face| world.landing_site(ship, Pos::on(below.body, face, n * 500, n * 500)))
    })
}

/// Orders a builder and some idle troops at home aboard a transport in orbit.
fn board(
    plan: &mut Plan<'_>,
    memory: &Memory,
    ship: &Entity,
    home: BodyId,
    has_builder: bool,
    room: i64,
    weights: &Weights,
) {
    let view = &plan.view;
    let walking_in = |e: &Entity| {
        e.orders()
            .front()
            .is_some_and(|o| o.kind == "enter" && o.target_entity == Some(ship.id))
    };
    let mut room = room;
    let mut riders: Vec<EntityId> = Vec::new();
    let builder_on_way = view
        .own
        .iter()
        .any(|e| walking_in(e) && view.rules.entity_types[&e.type_id].builder.is_some());
    if !has_builder
        && !builder_on_way
        && let Some(builder) = view
            .own
            .iter()
            .filter(|e| {
                let et = &view.rules.entity_types[&e.type_id];
                et.builder.is_some()
                    && et.movement.is_some()
                    && e.pos.body == home
                    && operational(e)
                    && e.container_id.is_none()
                    && !view.busy.contains(&e.id)
            })
            // An idle builder first; a busy one leaves its site once the ship waits landed
            // (task 68: troops board only on the ground, so the ship cannot wait for long).
            .min_by_key(|e| {
                let building = e
                    .orders()
                    .iter()
                    .any(|o| matches!(o.kind.as_str(), "construct" | "repair"));
                (!view.is_idle(e), building, e.id)
            })
    {
        riders.push(builder.id);
        room -= view.rules.entity_types[&builder.type_id].pop_cost.max(0);
    }
    // Troops from the main army while it waits at home, as many as the profile spares.
    let spare = (ESCORT_TROOPS as i64 * (100 - weights.defense_share) / 100).max(1) as usize;
    let walking = view.own.iter().filter(|e| walking_in(e)).count();
    if let Some(army) = memory.armies.get(&Role::Attack)
        && army.phase == Phase::Gather
    {
        for e in fighters(view)
            .into_iter()
            .filter(|e| army.members.contains(&e.id.0) && e.pos.body == home)
            .filter(|e| !walking_in(e))
            .take(spare.saturating_sub(walking))
        {
            let pop = view.rules.entity_types[&e.type_id].pop_cost.max(0);
            if room - pop < 0 {
                break;
            }
            room -= pop;
            riders.push(e.id);
        }
    }
    for rider in riders {
        if !plan.order(rider, "enter", None, Some(ship.id), None) {
            return;
        }
    }
}

/// After landing: found the city with a landed builder, send landed troops at the nearest
/// known enemy building on that planet, and lift off once the city stands.
fn settle(plan: &mut Plan<'_>, memory: &mut Memory, ship: &Entity, body: BodyId) {
    let view = &plan.view;
    if view.cities.iter().any(|c| c.pos.body == body) {
        if view.world.is_landed(ship.id)
            && ship.orders().is_empty()
            && !view.busy.contains(&ship.id)
        {
            plan.order(ship.id, "lift", None, None, None);
        }
        memory.voyages.remove(&ship.id.0);
        return;
    }
    // Troops ashore go for the nearest enemy building known on this planet.
    let attacks: Vec<(EntityId, Pos)> = fighters(view)
        .into_iter()
        .filter(|e| e.pos.body == body && e.orders().is_empty())
        .filter_map(|e| {
            memory
                .enemies
                .values()
                .filter(|x| x.building && x.pos.body == body)
                .min_by_key(|x| (view.distance(e.pos, x.pos), x.pos.face, x.pos.x, x.pos.y))
                .map(|x| (e.id, x.pos))
        })
        .collect();
    for (id, at) in attacks {
        if !plan.order(id, "attack_move", None, None, Some(at)) {
            return;
        }
    }
    let view = &plan.view;
    let Some(city) = view.world.city_type(view.me) else {
        return;
    };
    if !city_room(view)
        || view
            .own
            .iter()
            .any(|e| e.pos.body == body && e.type_id == city)
    {
        return;
    }
    let builder = view
        .own
        .iter()
        .filter(|e| {
            view.rules.entity_types[&e.type_id].builder.is_some()
                && e.pos.body == body
                && e.container_id.is_none()
                && !view.busy.contains(&e.id)
                && !e
                    .orders()
                    .iter()
                    .any(|o| matches!(o.kind.as_str(), "construct" | "repair"))
        })
        .min_by_key(|e| e.id)
        .map(|e| (e.id, e.pos));
    let Some((builder, near)) = builder else {
        return;
    };
    let Some(site) = view.world.find_city_site(view.me, body, near) else {
        return;
    };
    let price = plan.price(city);
    if !plan.affordable(&price) {
        plan.lack(&price, true);
        return;
    }
    if plan.order(builder, "construct", Some(city), None, Some(site)) {
        plan.pay(&price);
        plan.view.note_planned(city);
    }
}
