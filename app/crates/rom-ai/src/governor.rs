//! The governor of an automated colony (task 73): it keeps the sites its colony type names
//! (`et_colony_site`, up to each cap) standing in the colony's area. It places each on the
//! best free spot, rebuilds what is destroyed, finishes what is left unfinished and pays
//! from the owner's stores while they last. It runs for every owner, people included, and
//! acts only through ordinary orders in the owner's name (the colony builds its own sites);
//! it keeps no memory, so nothing of it is saved. Integers only, id order throughout.

use rom_rules::{DefId, EntityType, ProductionMode};
use rom_sim::{Command, Entity, EntityId, PlayerId, Pos, World};

/// Ticks between two rounds of the governors (two game seconds).
pub const GOVERNOR_INTERVAL: u64 = 40;
/// Tiles of free ground kept round every site the governor places, so ways stay open.
const LANE: i64 = 1000;
/// Most valid spots whose surroundings are weighed in one search, spread over all of them.
const WEIGHED: usize = 48;

/// One kind of site a colony keeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SiteCount {
    /// The site type the governor builds.
    pub site_type: DefId,
    /// Buildings of its upgrade line in the colony's area, unfinished ones included.
    pub count: i64,
    /// The most the governor keeps.
    pub cap: i64,
}

/// What a governor waits for, or does, at the moment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Waiting {
    /// Every site stands.
    Nothing,
    /// The owner paused it.
    Paused,
    /// The colony itself is unfinished, struck down or being assimilated.
    Colony,
    /// The colony is building a site.
    Building(DefId),
    /// The next site's type is not unlocked for the owner yet.
    Locked(DefId),
    /// The next site costs more than the owner has: what is missing, per resource.
    Resources(DefId, Vec<(DefId, i64)>),
    /// The next site is affordable and will be placed on the next round.
    Placing(DefId),
}

/// A colony's sites against their caps, and what its governor waits for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColonyStatus {
    /// The colony.
    pub colony: EntityId,
    /// Whether its owner paused the governor.
    pub paused: bool,
    /// Each kind of site, in the order the colony type lists them.
    pub sites: Vec<SiteCount>,
    /// What the governor waits for.
    pub waiting: Waiting,
}

/// Every own automated colony of a player, in id order.
pub fn colonies(world: &World, player: PlayerId) -> Vec<EntityId> {
    world
        .cities(player)
        .into_iter()
        .filter(|id| {
            world
                .entity(*id)
                .is_some_and(|e| world.is_colony_type(e.type_id))
        })
        .collect()
}

/// What a colony's governor is doing; None for anything but a colony.
pub fn colony_status(world: &World, colony: EntityId) -> Option<ColonyStatus> {
    let e = world.entity(colony)?;
    let rules = world.rules();
    let kept = &rules.entity_types[&e.type_id].colony.as_ref()?.sites;
    let buildings: Vec<&Entity> = world
        .colony_buildings(colony)
        .into_iter()
        .filter_map(|id| world.entity(id))
        .collect();
    let sites: Vec<SiteCount> = kept
        .iter()
        .map(|site| {
            let root = world.line_root(site.site_type_id);
            let standing = buildings
                .iter()
                .filter(|b| world.line_root(b.type_id) == root)
                .count() as i64;
            let queued = world
                .pending_orders()
                .filter(|o| {
                    o.executor == colony
                        && o.kind == "construct"
                        && o.subject.is_some_and(|s| world.line_root(s) == root)
                })
                .count() as i64;
            SiteCount {
                site_type: site.site_type_id,
                count: standing + queued,
                cap: site.cap,
            }
        })
        .collect();
    // A switch submitted this tick counts already: the last one decides.
    let paused = world
        .pending_orders()
        .filter(|o| o.executor == colony)
        .filter_map(|o| match o.kind.as_str() {
            "pause_governor" => Some(true),
            "resume_governor" => Some(false),
            _ => None,
        })
        .last()
        .unwrap_or_else(|| world.governor_paused(colony));
    let waiting = if paused {
        Waiting::Paused
    } else if e.build_progress.is_some() || e.neutralised || e.assimilation_until.is_some() {
        Waiting::Colony
    } else if let Some(site) = busy_with(world, e, &buildings) {
        Waiting::Building(site)
    } else {
        // The kinds furthest below their caps first; the first one that can be paid for is
        // placed, else the governor names what the most needed one lacks.
        let mut first: Option<Waiting> = None;
        let mut placing = None;
        for next in wanted(&sites) {
            match payable(world, e.owner, next) {
                Ok(()) => {
                    placing = Some(next);
                    break;
                }
                Err(waiting) => {
                    first.get_or_insert(waiting);
                }
            }
        }
        match (placing, first) {
            (Some(next), _) => Waiting::Placing(next),
            (None, Some(waiting)) => waiting,
            (None, None) => Waiting::Nothing,
        }
    };
    Some(ColonyStatus {
        colony,
        paused,
        sites,
        waiting,
    })
}

/// The site the colony is building now, if any: its own construction or repair order, or an
/// unfinished building of its area.
fn busy_with(world: &World, colony: &Entity, buildings: &[&Entity]) -> Option<DefId> {
    let order = colony
        .orders()
        .iter()
        .chain(world.pending_orders().filter(|o| o.executor == colony.id))
        .find(|o| matches!(o.kind.as_str(), "construct" | "repair"));
    if let Some(order) = order {
        return order
            .subject
            .or_else(|| {
                order
                    .target_entity
                    .and_then(|id| world.entity(id))
                    .map(|e| e.type_id)
            })
            .or(Some(colony.type_id));
    }
    buildings
        .iter()
        .find(|b| b.build_progress.is_some())
        .map(|b| b.type_id)
}

/// Whether the owner may build a site now, or why not (locked, or the resources it lacks).
fn payable(world: &World, owner: PlayerId, site: DefId) -> Result<(), Waiting> {
    let rules = world.rules();
    let Some(player) = world.players().find(|p| p.id == owner) else {
        return Err(Waiting::Locked(site));
    };
    if !rules.is_available(site, player.faction)
        || !rules.is_unlocked(site, &player.active_sources(), &world.owned_types(owner))
    {
        return Err(Waiting::Locked(site));
    }
    let missing: Vec<(DefId, i64)> = world
        .price(owner, site)
        .into_iter()
        .filter_map(|(resource, amount)| {
            let have = player.resources.get(&resource).copied().unwrap_or(0);
            (amount > have).then_some((resource, amount - have))
        })
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(Waiting::Resources(site, missing))
    }
}

/// The kinds below their caps, furthest below (as a share of the cap) first; ties keep the
/// colony type's order.
fn wanted(sites: &[SiteCount]) -> Vec<DefId> {
    let mut below: Vec<(i64, usize, DefId)> = sites
        .iter()
        .enumerate()
        .filter(|(_, s)| s.count < s.cap)
        .map(|(n, s)| (s.count * 1000 / s.cap.max(1), n, s.site_type))
        .collect();
    below.sort_unstable();
    below.into_iter().map(|(_, _, site)| site).collect()
}

/// This round's orders of every governor of a player: per colony at most one, in id order.
pub fn governor_orders(world: &World, player: PlayerId) -> Vec<Command> {
    let mut out = Vec::new();
    for colony in colonies(world, player) {
        let Some(status) = colony_status(world, colony) else {
            continue;
        };
        let order = |kind: &str, subject, target_entity, target_pos| Command {
            player,
            executor: colony,
            kind: kind.into(),
            subject,
            target_entity,
            target_pos,
            queue: false,
            text: None,
        };
        match status.waiting {
            Waiting::Building(_) => {
                // An unfinished site nobody builds (its order was lost): the colony finishes it.
                let e = world.entity(colony).expect("colony");
                let idle = !e
                    .orders()
                    .iter()
                    .chain(world.pending_orders().filter(|o| o.executor == colony))
                    .any(|o| matches!(o.kind.as_str(), "construct" | "repair"));
                if idle
                    && let Some(site) = world.colony_buildings(colony).into_iter().find(|id| {
                        world
                            .entity(*id)
                            .is_some_and(|b| b.build_progress.is_some())
                    })
                {
                    out.push(order("repair", None, Some(site), None));
                }
            }
            Waiting::Placing(_) => {
                // The most needed kind that can be paid for and finds room.
                let owner = world.entity(colony).expect("colony").owner;
                let placed = wanted(&status.sites)
                    .into_iter()
                    .filter(|site| payable(world, owner, *site).is_ok())
                    .find_map(|site| find_spot(world, colony, site).map(|pos| (site, pos)));
                if let Some((site, pos)) = placed {
                    out.push(order("construct", Some(site), None, Some(pos)));
                }
            }
            _ => {}
        }
    }
    out
}

/// Runs every player's governors on their round (every `GOVERNOR_INTERVAL` ticks) and submits
/// what they order, before `World::step`. Refused orders are simply tried again on a later
/// round. Returns how many orders were accepted.
pub fn run_governors(world: &mut World) -> usize {
    if !world.tick().is_multiple_of(GOVERNOR_INTERVAL) {
        return 0;
    }
    let players: Vec<PlayerId> = world.players().map(|p| p.id).collect();
    let mut accepted = 0;
    for player in players {
        for command in governor_orders(world, player) {
            if world.submit(command).is_ok() {
                accepted += 1;
            }
        }
    }
    accepted
}

/// A footprint as east, south, west and north edges in milli-tiles on its face.
fn rect(et: &EntityType, pos: Pos, margin: i64) -> [i64; 4] {
    let (w, h) = (et.footprint_w * 500 + margin, et.footprint_h * 500 + margin);
    [
        i64::from(pos.x) - w,
        i64::from(pos.y) - h,
        i64::from(pos.x) + w,
        i64::from(pos.y) + h,
    ]
}

fn overlaps(a: [i64; 4], b: [i64; 4]) -> bool {
    a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3]
}

/// The best free spot in a colony's area for a site: one the simulation accepts, that the
/// colony itself serves, with a free lane round it. A site drawing from a node terrain
/// (forest) goes where most of it lies in reach, any other as near the colony as it fits.
fn find_spot(world: &World, colony: EntityId, site: DefId) -> Option<Pos> {
    let rules = world.rules();
    let c = world.entity(colony)?;
    let et = &rules.entity_types[&site];
    if !rules
        .production
        .get(&c.type_id)
        .into_iter()
        .flatten()
        .any(|p| p.mode == ProductionMode::Construct && p.product_def_id == site)
    {
        return None;
    }
    let radius = world.city_economic_radius(colony);
    // Footprints already standing near the colony, with their lane.
    let near = world.entities_in_radius(
        c.pos,
        (radius + 12_000).clamp(0, i64::from(u32::MAX)) as u32,
    );
    let taken: Vec<(u8, [i64; 4])> = near
        .iter()
        .filter_map(|id| world.entity(*id))
        .filter(|e| rules.entity_types[&e.type_id].class == rom_rules::EntityClass::Building)
        .map(|e| (e.pos.face, rect(&rules.entity_types[&e.type_id], e.pos, 0)))
        .collect();
    let node = et
        .gather_site
        .as_ref()
        .and_then(|g| g.needs_node_terrain_id.map(|t| (t, g.node_radius.max(0))));
    let tiles = (radius / 1000) as i32;
    let mut spots: Vec<(u64, i32, i32, Pos)> = Vec::new();
    for dy in -tiles..=tiles {
        for dx in -tiles..=tiles {
            if i64::from(dx * dx + dy * dy) * 1_000_000 > radius * radius {
                continue;
            }
            let at = world.offset(c.pos, i64::from(dx) * 1000, i64::from(dy) * 1000);
            let Some(distance) = world.distance(c.pos, at) else {
                continue;
            };
            spots.push((distance, dy, dx, at));
        }
    }
    spots.sort_unstable_by_key(|(distance, dy, dx, _)| (*distance, *dy, *dx));
    let mut found: Vec<Pos> = Vec::new();
    for (_, _, _, at) in spots {
        let Ok(pos) = world.check_placement(c.owner, site, at) else {
            continue;
        };
        if found.contains(&pos)
            || world.city_covering(c.owner, pos) != Some(colony)
            || taken
                .iter()
                .any(|(face, other)| *face == pos.face && overlaps(rect(et, pos, LANE), *other))
        {
            continue;
        }
        found.push(pos);
        // A plain site takes the nearest spot; a node site weighs the whole area.
        if node.is_none() {
            break;
        }
    }
    let Some((terrain, reach)) = node else {
        return found.first().copied();
    };
    let tiles = (reach / 1000) as i32;
    let step = found.len().div_ceil(WEIGHED).max(1);
    found
        .into_iter()
        .enumerate()
        .step_by(step)
        .max_by_key(|(order, pos)| {
            let mut count = 0_i64;
            for dy in -tiles..=tiles {
                for dx in -tiles..=tiles {
                    if i64::from(dx * dx + dy * dy) * 1_000_000 > reach * reach {
                        continue;
                    }
                    let at = world.offset(*pos, i64::from(dx) * 1000, i64::from(dy) * 1000);
                    if world.terrain_at(at) == Some(terrain) {
                        count += 1;
                    }
                }
            }
            (count, std::cmp::Reverse(*order))
        })
        .map(|(_, pos)| pos)
}
