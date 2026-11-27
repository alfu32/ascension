//! The economy manager: how many workers each resource should have, the gather sites and
//! workers that takes, and sending idle hands to work.

use crate::{
    Weights,
    memory::Memory,
    plan::{Act, Plan, Wish},
    view::{View, operational},
};
use rom_rules::{DefId, ProductionMode, ResourceKind};
use rom_sim::{Entity, EntityId};
use std::collections::{BTreeMap, BTreeSet};

/// Weight every resource has before any shortfall is counted.
const BASE_WEIGHT: i64 = 100;
/// A store of this size halves a resource's weight.
const STOCK_EASE: i64 = 1500;
/// Share of the remembered demand that is kept at each update, in percent.
const DEMAND_KEEP_PCT: i64 = 70;
/// Score of a site for a resource with fewer than half the slots its workers need, paid in
/// that resource: above the way to the next age at its most urgent.
const STARVED_SITE_SCORE: i64 = 1100;
/// Ticks between demand updates and between workers moved from one resource to another, so
/// that a brain that thinks often does not keep its workers walking.
pub(crate) const PACE_TICKS: u64 = 100;
/// Income periods in which a resource earned only from sites without workers (energy) should
/// bring in what the plans spend of it (task 152).
const SPEND_PERIODS: i64 = 10;
/// Workers kept beyond one per city when no site has work for them: they build (task 166).
const KEPT_BUILDERS: i64 = 2;
/// Workers without any work dismissed in one thought at most.
const RELEASED_PER_THOUGHT: i64 = 4;
/// Score of such a site while a city lacks one and the income covers the plans: with the
/// technologies, below the gather sites that feed the workers.
const UNATTENDED_SCORE: i64 = 600;
/// Score of such a site while the plans spend more than comes in: as the first site of a
/// resource, and saved for.
const UNATTENDED_SHORT_SCORE: i64 = 880;

/// The state of one resource's gathering.
#[derive(Debug, Clone, Default)]
pub(crate) struct Gathering {
    /// Workers inside sites or walking to one.
    pub on: i64,
    /// Slots of finished sites.
    pub slots: i64,
    /// Sites still under construction.
    pub pending: i64,
    /// Workers the plan wants on the resource.
    pub desired: i64,
    /// Another site could still be placed.
    pub expandable: bool,
    /// The site type to build next, if any.
    pub site_type: Option<DefId>,
    /// Its sites are full, the commerce cap would pay for more hands and the plans lack it,
    /// but no further site fits around the cities: the resource waits for another city.
    pub stuck: bool,
    /// Nobody gathers it: it comes only from sites that need no workers (energy from
    /// reactors, task 152).
    pub unattended: bool,
}

/// The economy as assessed for one thought.
#[derive(Debug, Clone, Default)]
pub(crate) struct Economy {
    pub resources: BTreeMap<DefId, Gathering>,
    /// Workers the profile wants in total.
    pub worker_target: i64,
    /// Workers owned or in training, with the workers called up as militia (they come back).
    pub workers: i64,
    /// Workers called up as militia.
    pub militia: i64,
    /// Free slots per site after everything inbound and planned.
    pub free: BTreeMap<EntityId, i64>,
}

/// The resource a gather site yields.
fn site_resource(view: &View<'_>, type_id: DefId) -> Option<DefId> {
    view.rules.entity_types[&type_id]
        .gather_site
        .as_ref()
        .map(|s| s.resource_id)
}

fn is_worker(view: &View<'_>, e: &Entity) -> bool {
    !view.rules.entity_types[&e.type_id].harvester.is_empty()
}

/// The buildable site type for a resource: unlocked, not superseded, most slots first.
fn site_type(view: &View<'_>, memory: &Memory, resource: DefId) -> Option<DefId> {
    let buildable = |site: DefId| {
        view.unlocked(site)
            && !view.obsolete(site)
            && !view.producers(ProductionMode::Construct, site).is_empty()
            && memory
                .blocked
                .get(&site.0)
                .is_none_or(|until| *until <= view.tick)
            && memory.no_site.get(&site.0) != Some(&crate::plan::site_situation(view))
    };
    view.cat
        .gather_sites
        .get(&resource)
        .into_iter()
        .flatten()
        .copied()
        .filter(|site| buildable(*site))
        .max_by_key(|site| {
            let s = view.rules.entity_types[site].gather_site.as_ref();
            (
                s.map_or(0, |s| s.slots_max),
                s.map_or(0, |s| s.self_income),
                std::cmp::Reverse(*site),
            )
        })
}

/// Assesses gathering and decides how many workers each resource should have.
pub(crate) fn assess(view: &View<'_>, weights: &Weights, memory: &Memory) -> Economy {
    let rules = view.rules;
    let mut eco = Economy {
        worker_target: view.pop_cap * weights.worker_share / 100,
        ..Economy::default()
    };
    let age = view.age_ordinal();
    for resource in rules.resources.values() {
        let known = resource
            .first_age_id
            .and_then(|a| rules.ages.get(&a))
            .is_none_or(|a| a.ordinal <= age);
        if resource.kind != ResourceKind::Base || !known {
            continue;
        }
        let harvested = view
            .cat
            .harvesters
            .get(&resource.id)
            .is_some_and(|types| types.iter().any(|t| rules.is_available(*t, view.faction)));
        // A resource nobody gathers counts when sites that need no workers earn it.
        let unattended = !harvested
            && view
                .cat
                .gather_sites
                .get(&resource.id)
                .into_iter()
                .flatten()
                .filter(|s| rules.is_available(**s, view.faction))
                .filter_map(|s| rules.entity_types[s].gather_site.as_ref())
                .any(|g| g.slots_max == 0 && g.self_income > 0);
        if !harvested && !unattended {
            continue;
        }
        let site_type = site_type(view, memory, resource.id);
        eco.resources.insert(
            resource.id,
            Gathering {
                expandable: site_type.is_some(),
                site_type,
                unattended,
                ..Gathering::default()
            },
        );
    }
    let mut inside: BTreeMap<DefId, i64> = BTreeMap::new();
    for e in &view.own {
        if let Some(resource) = site_resource(view, e.type_id)
            && let Some(g) = eco.resources.get_mut(&resource)
        {
            if operational(e) {
                g.slots += e.gather_slots;
                let taken = view.world.occupants(e.id).len() as i64
                    + view.inbound.get(&e.id).copied().unwrap_or(0);
                // A site workers could not reach or fit into lately is not offered, nor one
                // of a city the enemy has struck down: workers there only keep the fight for
                // it going and die.
                let shunned = memory
                    .unreachable
                    .get(&e.id.0)
                    .is_some_and(|until| *until > view.tick)
                    || e.city_id
                        .and_then(|c| view.world.entity(c))
                        .is_some_and(|c| c.neutralised);
                eco.free
                    .insert(e.id, if shunned { 0 } else { e.gather_slots - taken });
            } else if e.build_progress.is_some() {
                g.pending += 1;
            }
        }
        if !is_worker(view, e) {
            continue;
        }
        let target = e.work_site.or_else(|| {
            e.orders()
                .iter()
                .find(|o| o.kind == "harvest")
                .and_then(|o| o.target_entity)
        });
        if let Some(resource) = target
            .and_then(|site| view.world.entity(site))
            .and_then(|site| site_resource(view, site.type_id))
            && let Some(g) = eco.resources.get_mut(&resource)
        {
            g.on += 1;
            if e.work_site.is_some() {
                *inside.entry(resource).or_default() += 1;
            }
        }
    }
    eco.militia = view.cat.militia.iter().map(|t| view.count(*t)).sum();
    eco.workers = rules
        .entity_types
        .values()
        .filter(|t| !t.harvester.is_empty())
        .map(|t| view.count(t.id))
        .sum::<i64>()
        + eco.militia;

    // The most workers a resource can use: its commerce cap, or its sites when no more fit.
    let cap = view.commerce_cap();
    let mut most: BTreeMap<DefId, i64> = BTreeMap::new();
    let mut stuck: BTreeMap<DefId, bool> = BTreeMap::new();
    for (resource, g) in &eco.resources {
        if g.unattended {
            // No hands to give it.
            most.insert(*resource, 0);
            continue;
        }
        let row = &rules.resources[resource];
        let income = view.world.income(view.me, *resource);
        let per_worker = view
            .cat
            .harvesters
            .get(resource)
            .into_iter()
            .flatten()
            .flat_map(|t| rules.entity_types[t].harvester.iter())
            .filter(|h| h.resource_id == *resource)
            .map(|h| h.amount)
            .max()
            .unwrap_or(1)
            .max(1);
        let working = inside.get(resource).copied().unwrap_or(0);
        let mut limit = if !row.rate_capped {
            i64::MAX / 4
        } else if income.capped {
            working
        } else {
            let pct = if income.raw > 0 {
                (income.effective * 100 / income.raw).max(1)
            } else {
                100
            };
            let room = (cap - income.effective).max(0) * 100;
            working + (room + per_worker * pct - 1) / (per_worker * pct)
        };
        // A site type the player can build, but no room left for one around the cities: the
        // resource waits for another city.
        let crowded = view
            .cat
            .gather_sites
            .get(resource)
            .into_iter()
            .flatten()
            .filter(|site| {
                view.unlocked(**site)
                    && !view.obsolete(**site)
                    && !view.producers(ProductionMode::Construct, **site).is_empty()
            })
            .any(|site| crate::plan::no_room_for(view, memory, *site));
        stuck.insert(
            *resource,
            crowded
                && limit > g.slots + g.pending
                && memory.demand.get(&resource.0).is_some_and(|d| *d > 0),
        );
        if !g.expandable {
            limit = limit.min(g.slots + g.pending);
        }
        let floor = memory.floors.get(&resource.0).copied().unwrap_or(0);
        most.insert(*resource, limit.max(floor));
    }

    // Hand out the worker target one by one to the resource that wants it most.
    // What the plans lacked raises a resource's weight; a full store lowers it.
    let weight = |resource: &DefId| {
        let wanted = BASE_WEIGHT
            + memory
                .demand
                .get(&resource.0)
                .copied()
                .unwrap_or(0)
                .clamp(0, 2000)
                / 4;
        let stock = view
            .purse
            .get(resource)
            .copied()
            .unwrap_or(0)
            .clamp(0, 1_000_000);
        wanted * STOCK_EASE / (STOCK_EASE + stock)
    };
    for (resource, g) in &mut eco.resources {
        g.stuck = stuck.get(resource).copied().unwrap_or(false);
        g.desired = memory
            .floors
            .get(&resource.0)
            .copied()
            .unwrap_or(0)
            .min(most[resource]);
    }
    let mut given: i64 = eco.resources.values().map(|g| g.desired).sum();
    while given < eco.worker_target {
        let Some(resource) = eco
            .resources
            .iter()
            .filter(|(resource, g)| g.desired < most[*resource])
            .max_by_key(|(resource, g)| {
                (
                    weight(resource) * 1000 / (g.desired + 1),
                    std::cmp::Reverse(**resource),
                )
            })
            .map(|(resource, _)| *resource)
        else {
            break;
        };
        eco.resources.get_mut(&resource).expect("listed").desired += 1;
        given += 1;
    }
    eco
}

impl Economy {
    /// Node terrains (forest, rock) of resources that wait for room for another site: a new
    /// city goes where they are.
    pub fn stuck_terrains(&self, view: &View<'_>) -> BTreeSet<DefId> {
        self.resources
            .iter()
            .filter(|(_, g)| g.stuck)
            .flat_map(|(resource, _)| view.cat.gather_sites.get(resource).into_iter().flatten())
            .filter_map(|site| view.rules.entity_types[site].gather_site.as_ref())
            .filter(|g| g.slots_per_node > 0)
            .filter_map(|g| g.needs_node_terrain_id)
            .collect()
    }

    /// Whether any resource waits for room for another site.
    pub fn stuck(&self) -> bool {
        self.resources.values().any(|g| g.stuck)
    }
}

/// The ticks between demand updates and worker moves for a difficulty: `PACE_TICKS`, or
/// twice its reaction time when that is longer (a slow mind rebalances late).
pub(crate) fn pace(difficulty: &rom_rules::AiDifficulty) -> u64 {
    PACE_TICKS.max(2 * difficulty.reaction_ticks.max(0) as u64)
}

/// Every `pace` ticks, keeps part of the old demand and adds what this thought could not
/// pay; a worker may then change resource too.
pub(crate) fn update_demand(
    memory: &mut Memory,
    shortfall: &BTreeMap<DefId, i64>,
    tick: u64,
    pace: u64,
) {
    if tick < memory.economy_tick + pace && tick >= memory.economy_tick {
        return;
    }
    memory.economy_tick = tick;
    for value in memory.demand.values_mut() {
        *value = *value * DEMAND_KEEP_PCT / 100;
    }
    for (resource, missing) in shortfall {
        *memory.demand.entry(resource.0).or_default() += missing;
    }
    memory.demand.retain(|_, value| *value > 0);
}

/// The resources each worker type harvests, for types the roster lists.
fn worker_types(view: &View<'_>) -> Vec<(DefId, BTreeSet<DefId>)> {
    view.rules
        .entity_types
        .values()
        .filter(|t| !t.harvester.is_empty() && view.rules.is_available(t.id, view.faction))
        .map(|t| (t.id, t.harvester.iter().map(|h| h.resource_id).collect()))
        .collect()
}

/// Wishes for gather sites and workers. A `late` thought (a slip of a mind that makes
/// mistakes) notices only resources without any site: further sites wait for a later one.
pub(crate) fn wishes(view: &View<'_>, eco: &Economy, late: bool, wishes: &mut Vec<Wish>) {
    let rules = view.rules;
    for g in eco.resources.values() {
        // Sites that need no workers are wished for by what the plans spend (`unattended`).
        let Some(site) = g.site_type.filter(|_| !g.unattended) else {
            continue;
        };
        let spec = rules.entity_types[&site]
            .gather_site
            .as_ref()
            .expect("gather site");
        if spec.slots_max > 0 {
            // A site under construction is expected to bring what is missing, except for
            // sites of a single slot, which are built two at a time.
            let coming = if spec.slots_max <= 2 {
                g.pending * spec.slots_max
            } else {
                g.pending * g.desired
            };
            let pending_limit = if spec.slots_max <= 2 { 2 } else { 1 };
            // The first site of a resource is always wished for: without it the resource
            // has no income at all, whatever the worker plan says.
            let nothing = g.slots == 0 && g.pending == 0;
            if (nothing || (!late && g.slots + coming < g.desired)) && g.pending < pending_limit {
                // A starved resource whose site is paid in that very resource (a camp paid in
                // timber, with timber trickling in from one tree) comes before everything that
                // spends it, the way to the next age included: else each of them in turn holds
                // the savings and the site is never bought.
                let starved = g.slots * 2 < g.desired
                    && view
                        .price(site)
                        .iter()
                        .any(|(paid, _)| *paid == spec.resource_id);
                wishes.push(Wish {
                    score: if starved {
                        STARVED_SITE_SCORE
                    } else if nothing {
                        880
                    } else {
                        760 + (g.desired - g.slots).clamp(0, 5) * 10
                    },
                    act: Act::Construct(site),
                    // A resource with less than half the slots its workers need saves for
                    // its next site: else cheaper wishes spend what it earns before it ever
                    // pays for one, and idle hands wait for good.
                    reserve: nothing || g.slots * 2 < g.desired,
                    opening: false,
                });
            }
        } else if spec.self_income > 0 && g.pending == 0 {
            // Unattended sites: one more whenever the cap still has room.
            let resource = spec.resource_id;
            let income = view.world.income(view.me, resource);
            if !income.capped {
                wishes.push(Wish {
                    score: 760,
                    act: Act::Construct(site),
                    reserve: false,
                    opening: false,
                });
            }
        }
    }
    let unfinished = view
        .own
        .iter()
        .filter(|e| e.build_progress.is_some() && view.is_building(e.type_id))
        .count() as i64;
    let mut covered: BTreeSet<DefId> = BTreeSet::new();
    for (worker, harvests) in worker_types(view) {
        if view.obsolete(worker)
            || !view.unlocked(worker)
            || view.producers(ProductionMode::Train, worker).is_empty()
            || harvests.iter().all(|r| covered.contains(r))
        {
            continue;
        }
        let jobs: i64 = harvests
            .iter()
            .filter(|r| !covered.contains(*r))
            .filter_map(|r| eco.resources.get(r))
            .map(|g| g.desired.min(g.slots))
            .sum();
        // Workers called up as militia come back: none is trained to replace them.
        let called_up = if rules.entity_types[&worker].call_up.is_some() {
            eco.militia
        } else {
            0
        };
        let hands: i64 = worker_types(view)
            .iter()
            .filter(|(_, other)| other.iter().any(|r| harvests.contains(r)))
            .map(|(t, _)| view.count(*t))
            .sum::<i64>()
            + called_up;
        let spare = if rules.entity_types[&worker].builder.is_some() {
            1 + unfinished.min(2)
        } else {
            0
        };
        // Room is kept for the workers of resources this type cannot gather and nobody
        // gathers yet (scholars before the first university), so that the population cap
        // is not full of the wrong hands when their time comes.
        let held_back: i64 = view
            .cat
            .harvesters
            .iter()
            .filter(|(resource, types)| {
                !harvests.contains(*resource)
                    && types.iter().any(|t| rules.is_available(*t, view.faction))
                    && eco.resources.get(*resource).is_none_or(|g| g.on == 0)
            })
            .map(|(resource, _)| {
                view.cat
                    .gather_sites
                    .get(resource)
                    .into_iter()
                    .flatten()
                    .filter(|site| rules.is_available(**site, view.faction))
                    .filter_map(|site| rules.entity_types[site].gather_site.as_ref())
                    .map(|site| site.slots_max)
                    .max()
                    .unwrap_or(0)
                    .min(eco.worker_target / 4)
            })
            .sum();
        covered.extend(harvests);
        if hands < (jobs + spare).min(eco.worker_target - held_back) {
            wishes.push(Wish {
                // Hands for free slots pay back faster than anything else.
                score: if eco.workers < 8 { 900 } else { 860 },
                act: Act::Train(worker),
                reserve: false,
                opening: false,
            });
        }
    }
}

/// Wishes for sites of resources nobody gathers, which sites that need no workers earn (energy:
/// the nuclear reactor, later the fusion plant), made after every other manager has wished
/// (task 152). The plans want income proportional to what they spend of the resource: the
/// price of this thought's wishes and what earlier ones lacked, brought in over
/// `SPEND_PERIODS` income periods; and at least one such site per city once one can be built,
/// as far as the type's limit allows. While the plans spend more than comes in, the site is
/// wished for as the first site of a resource and saved for.
pub(crate) fn unattended_wishes(
    view: &View<'_>,
    eco: &Economy,
    memory: &Memory,
    wishes: &mut Vec<Wish>,
) {
    let rules = view.rules;
    let cities = view.cities.iter().filter(|c| c.complete).count() as i64;
    for (resource, g) in eco.resources.iter().filter(|(_, g)| g.unattended) {
        let Some(site) = g.site_type else {
            continue;
        };
        let et = &rules.entity_types[&site];
        let per_site = et.gather_site.as_ref().map_or(0, |s| s.self_income);
        let income = view.world.income(view.me, *resource);
        if per_site <= 0 || income.capped {
            continue;
        }
        let spend: i64 = wishes
            .iter()
            .flat_map(|w| view.price(w.act.subject()))
            .filter(|(paid, _)| paid == resource)
            .map(|(_, amount)| amount)
            .sum::<i64>()
            + memory.demand.get(&resource.0).copied().unwrap_or(0).max(0);
        let need = (spend + SPEND_PERIODS - 1) / SPEND_PERIODS;
        let root = view.cat.root_of(site);
        let have = view.line_count(site);
        let working = view
            .own
            .iter()
            .filter(|e| view.cat.root_of(e.type_id) == root && operational(e))
            .count() as i64;
        let short = need > income.effective;
        let for_spend = if short {
            working + (need - income.effective + per_site - 1) / per_site
        } else {
            0
        };
        let allowed = match et.limit_scope {
            rom_rules::LimitScope::PerCity => et.limit_count * cities,
            rom_rules::LimitScope::PerPlayer | rom_rules::LimitScope::PerMatch => et.limit_count,
            _ => i64::MAX,
        };
        if have >= cities.max(for_spend).min(allowed) {
            continue;
        }
        wishes.push(Wish {
            score: if short {
                UNATTENDED_SHORT_SCORE
            } else {
                UNATTENDED_SCORE
            },
            act: Act::Construct(site),
            reserve: short,
            opening: false,
        });
    }
}

/// The site with a free slot for `resource` nearest to a worker.
fn free_site(plan: &Plan<'_>, eco: &Economy, worker: &Entity, resource: DefId) -> Option<EntityId> {
    let view = &plan.view;
    eco.free
        .iter()
        .filter(|(_, free)| **free > 0)
        .filter_map(|(site, _)| view.world.entity(*site))
        .filter(|site| {
            site.pos.body == worker.pos.body && site_resource(view, site.type_id) == Some(resource)
        })
        .min_by_key(|site| (view.distance(worker.pos, site.pos), site.id))
        .map(|site| site.id)
}

/// Whether no gather site of the resources a worker harvests has, or will ever have, a slot
/// for it: none stands with slots, and every site type of those resources the player's
/// nation has and that is not superseded needs no workers. Before the automated ages there
/// is always such a type with slots (the farm, or the university not yet built).
fn workless(view: &View<'_>, eco: &Economy, worker: &Entity) -> bool {
    let rules = view.rules;
    rules.entity_types[&worker.type_id]
        .harvester
        .iter()
        .all(|h| {
            let standing = eco.resources.get(&h.resource_id).map_or(0, |g| g.slots);
            let types: Vec<DefId> = view
                .cat
                .gather_sites
                .get(&h.resource_id)
                .into_iter()
                .flatten()
                .copied()
                .filter(|site| rules.is_available(*site, view.faction) && !view.obsolete(*site))
                .collect();
            standing == 0
                && !types.is_empty()
                && types.iter().all(|site| {
                    rules.entity_types[site]
                        .gather_site
                        .as_ref()
                        .is_none_or(|s| s.slots_max == 0)
                })
        })
}

/// Sends builders to untended sites, idle workers to the resource that lacks them most,
/// moves one worker off an overstaffed resource, and puts idle traders on a route.
pub(crate) fn assign(
    plan: &mut Plan<'_>,
    eco: &mut Economy,
    rebalance: bool,
    unreachable: &BTreeSet<u32>,
) {
    let workers: Vec<&Entity> = plan
        .view
        .own
        .iter()
        .copied()
        .filter(|e| operational(e) && e.container_id.is_none() && is_worker(&plan.view, e))
        .collect();

    // Unfinished buildings nobody is working on.
    let untended: Vec<&Entity> = plan
        .view
        .own
        .iter()
        .copied()
        .filter(|e| {
            e.build_progress.is_some()
                && plan.view.is_building(e.type_id)
                && !plan.view.tended.contains(&e.id)
                // A site no builder could reach lately is left alone for a while.
                && !unreachable.contains(&e.id.0)
        })
        .collect();
    for site in untended {
        let builder = workers
            .iter()
            .filter(|w| {
                plan.view.rules.entity_types[&w.type_id].builder.is_some()
                    && plan.view.rules.entity_types[&w.type_id].movement.is_some()
                    && w.pos.body == site.pos.body
                    && !plan.view.busy.contains(&w.id)
                    && !w
                        .orders()
                        .iter()
                        .any(|o| matches!(o.kind.as_str(), "construct" | "repair"))
            })
            .min_by_key(|w| {
                (
                    !plan.view.is_idle(w),
                    plan.view.distance(w.pos, site.pos),
                    w.id,
                )
            })
            .map(|w| w.id);
        if let Some(builder) = builder
            && !plan.order(builder, "repair", None, Some(site.id), None)
        {
            return;
        }
    }

    // Workers in a gather site of a city the enemy has struck down are called out of it,
    // one site a thought: they go to work elsewhere instead of holding the city for him.
    let struck = plan
        .view
        .own
        .iter()
        .find(|e| {
            !plan.view.world.occupants(e.id).is_empty()
                && e.city_id
                    .and_then(|c| plan.view.world.entity(c))
                    .is_some_and(|c| c.neutralised)
        })
        .map(|e| e.id);
    if let Some(site) = struck
        && !plan.order(site, "release", None, None, None)
    {
        return;
    }

    // Idle workers go where the plan is shortest of hands.
    let crowded = plan.view.pop_used + plan.view.queued_pop >= plan.view.pop_cap;
    let mut disbanded = false;
    // Workers kept for building when no gather site has work for anyone (task 166).
    let keep = plan.view.cities.len() as i64 + KEPT_BUILDERS;
    let mut released = 0;
    let idle: Vec<&Entity> = workers
        .iter()
        .copied()
        .filter(|w| plan.view.is_idle(w))
        .collect();
    for worker in idle {
        let harvests = &plan.view.rules.entity_types[&worker.type_id].harvester;
        let choice = harvests
            .iter()
            .filter_map(|h| {
                let g = eco.resources.get(&h.resource_id)?;
                let site = free_site(plan, eco, worker, h.resource_id)?;
                Some((g.desired - g.on, std::cmp::Reverse(h.resource_id), site))
            })
            .max();
        if let Some((_, std::cmp::Reverse(resource), site)) = choice {
            if !plan.order(worker.id, "harvest", None, Some(site), None) {
                return;
            }
            *eco.free.entry(site).or_default() -= 1;
            eco.resources.get_mut(&resource).expect("listed").on += 1;
        } else if crowded && !disbanded && eco.workers > eco.worker_target {
            // Nothing for it to do and no room for anyone else: it leaves the match and
            // frees its place in the population for someone useful.
            if !plan.order(worker.id, "disband", None, None, None) {
                return;
            }
            disbanded = true;
        } else if released < RELEASED_PER_THOUGHT
            && eco.workers - released > keep
            && workless(&plan.view, eco, worker)
        {
            // No site anywhere has work for its kind (the automated ages, where every site
            // runs unattended) and none will: beyond a few builders it leaves the match, so
            // idle hands do not fill the population for good (task 166).
            if !plan.order(worker.id, "disband", None, None, None) {
                return;
            }
            released += 1;
        }
    }

    // One worker per thought moves from the most overstaffed resource to one short of hands.
    let over = eco
        .resources
        .iter()
        .filter(|(_, g)| g.on > g.desired)
        .max_by_key(|(resource, g)| (g.on - g.desired, std::cmp::Reverse(**resource)))
        .map(|(resource, _)| *resource);
    if let Some(over) = over.filter(|_| rebalance) {
        let mover = workers.iter().find(|w| {
            !plan.view.busy.contains(&w.id)
                && w.work_site
                    .and_then(|site| plan.view.world.entity(site))
                    .and_then(|site| site_resource(&plan.view, site.type_id))
                    == Some(over)
        });
        if let Some(worker) = mover {
            let harvests = &plan.view.rules.entity_types[&worker.type_id].harvester;
            let choice = harvests
                .iter()
                .filter(|h| h.resource_id != over)
                .filter_map(|h| {
                    let g = eco.resources.get(&h.resource_id)?;
                    let site = free_site(plan, eco, worker, h.resource_id)?;
                    (g.on < g.desired).then_some((
                        g.desired - g.on,
                        std::cmp::Reverse(h.resource_id),
                        site,
                    ))
                })
                .max();
            if let Some((_, std::cmp::Reverse(resource), site)) = choice {
                if !plan.order(worker.id, "harvest", None, Some(site), None) {
                    return;
                }
                *eco.free.entry(site).or_default() -= 1;
                eco.resources.get_mut(&resource).expect("listed").on += 1;
                eco.resources.get_mut(&over).expect("listed").on -= 1;
            }
        }
    }

    trade(plan, unreachable);
}

/// The other end of a route for a trader: for one that flies through space (task 68's
/// freighter), an own city on another planet of its star system; for one that walks, the
/// own city farthest from its home city on its planet, far enough for a route. A city traders
/// found no way to lately (`unreachable`) is passed over (task 153).
fn route_for(view: &View<'_>, trader: &Entity, unreachable: &BTreeSet<u32>) -> Option<EntityId> {
    if view.world.is_spacecraft_type(trader.type_id) {
        let here = view
            .world
            .orbit(trader.id)
            .map_or(trader.pos.body, |o| o.body);
        let system = view.world.body(here)?.system;
        return view
            .cities
            .iter()
            .filter(|c| {
                c.complete
                    && c.pos.body != here
                    && view
                        .world
                        .body(c.pos.body)
                        .is_some_and(|b| b.system == system)
            })
            .min_by_key(|c| (c.pos.body, c.id))
            .map(|c| c.id);
    }
    let minimum = view.rules.constant("trade.min_route").unwrap_or(0).max(0) as u64;
    let cities: Vec<_> = view
        .cities
        .iter()
        .filter(|c| c.complete && c.pos.body == trader.pos.body)
        .collect();
    let home = cities
        .iter()
        .min_by_key(|c| (view.distance(trader.pos, c.pos), c.id))?;
    cities
        .iter()
        .filter(|c| c.id != home.id && !unreachable.contains(&c.id.0))
        .map(|c| (view.distance(home.pos, c.pos), c.id))
        .filter(|(distance, _)| *distance >= minimum && *distance < u64::MAX / 4)
        .max()
        .map(|(_, id)| id)
}

/// Whether two own cities stand far enough apart for a trade route.
pub(crate) fn can_trade(view: &View<'_>) -> bool {
    let minimum = view.rules.constant("trade.min_route").unwrap_or(0).max(0) as u64;
    view.cities.iter().filter(|c| c.complete).any(|a| {
        view.cities.iter().filter(|c| c.complete).any(|b| {
            a.id < b.id && a.pos.body == b.pos.body && {
                let d = view.distance(a.pos, b.pos);
                d >= minimum && d < u64::MAX / 4
            }
        })
    })
}

/// Whether own cities stand on two planets of one star system: a route for a trader that
/// flies.
pub(crate) fn can_trade_in_space(view: &View<'_>) -> bool {
    let systems: Vec<(u32, rom_sim::BodyId)> = view
        .cities
        .iter()
        .filter(|c| c.complete)
        .filter_map(|c| Some((view.world.body(c.pos.body)?.system, c.pos.body)))
        .collect();
    systems
        .iter()
        .any(|(system, body)| systems.iter().any(|(s, b)| s == system && b != body))
}

/// Own units that run trade routes.
pub(crate) fn traders<'a>(view: &View<'a>) -> Vec<&'a Entity> {
    view.own
        .iter()
        .copied()
        .filter(|e| {
            view.rules.entity_types[&e.type_id]
                .trader
                .as_ref()
                .is_some_and(|t| t.runs_routes && t.route_resource_id.is_some())
        })
        .collect()
}

fn trade(plan: &mut Plan<'_>, unreachable: &BTreeSet<u32>) {
    let running = plan.view.world.trade_routes(plan.view.me).count() as i64;
    let mut room = plan.view.caravan_limit() - running;
    for trader in traders(&plan.view) {
        if room <= 0 {
            return;
        }
        if !operational(trader) || !plan.view.is_idle(trader) {
            continue;
        }
        if let Some(target) = route_for(&plan.view, trader, unreachable) {
            if !plan.order(trader.id, "trade", None, Some(target), None) {
                return;
            }
            room -= 1;
        }
    }
}
