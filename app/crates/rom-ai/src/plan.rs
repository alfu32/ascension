//! One thought's working state: the view, the orders written so far, the money left, and
//! the wishes the managers put forward.

use crate::{
    memory::Memory,
    sites::{Ground, Hold, Site, at_spot, find_city_site, find_site, node_spot},
    view::View,
};
use rom_rules::{DefId, LimitScope, ProductionMode, ResourceKind};
use rom_sim::{Command, Entity, EntityId, Pos};
use std::collections::{BTreeMap, BTreeSet};

/// Orders a producer may hold before more training is queued on it.
const TRAIN_QUEUE: usize = 2;
/// Unaffordable wishes a thought may save up for at once.
const RESERVATIONS: usize = 1;
/// Ticks a rejected subject or a failed site search is left alone.
pub(crate) const RETRY_TICKS: u64 = 1200;

/// Something a manager wants bought.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Act {
    /// Train one unit of a type.
    Train(DefId),
    /// Research a technology.
    Research(DefId),
    /// Construct one building of a type.
    Construct(DefId),
}

impl Act {
    /// The definition the act is about.
    pub fn subject(self) -> DefId {
        match self {
            Act::Train(def) | Act::Research(def) | Act::Construct(def) => def,
        }
    }
}

/// A scored purchase; the highest score is served first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Wish {
    pub score: i64,
    pub act: Act,
    /// Save up for it: lower wishes may not spend what it still needs.
    pub reserve: bool,
    /// It is the current step of the opening book.
    pub opening: bool,
}

/// What became of a wish.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Outcome {
    /// An order was written.
    Ordered,
    /// It can be done but not paid yet.
    Unaffordable,
    /// It cannot be done now for a reason other than money.
    Impossible,
}

/// The state of one thought.
pub(crate) struct Plan<'a> {
    pub view: View<'a>,
    /// Obstacles and dangers, gathered when the first site is looked for.
    ground: Option<Ground>,
    /// A site search of this thought came back empty-handed; no more are started.
    searched: bool,
    /// Orders written during this thought.
    pub out: Vec<Command>,
    /// Orders the budget still allows.
    pub orders_left: i64,
    /// What the unaffordable wishes lacked, per resource.
    pub shortfall: BTreeMap<DefId, i64>,
    reservations: usize,
    /// Constructions started in this thought.
    pub started: i64,
    /// Amounts kept back from every purchase but the gather sites in `keep_for`: the price
    /// of the first site of a resource that pays for its own site in that resource.
    keep: BTreeMap<DefId, i64>,
    /// The gather site types the kept amounts are for.
    keep_for: BTreeSet<DefId>,
    /// Node terrains a new city should be near: those of resources with no room for another
    /// site around the cities.
    pub city_terrains: BTreeSet<DefId>,
    /// Where lost buildings stood, by the first type of their line: a building of the line
    /// goes there first when the ground is free (task 152).
    pub rebuild_at: BTreeMap<DefId, Vec<Pos>>,
}

impl<'a> Plan<'a> {
    pub fn new(view: View<'a>, orders_left: i64, memory: &Memory) -> Self {
        Self {
            view,
            ground: None,
            searched: false,
            out: Vec::new(),
            orders_left,
            shortfall: BTreeMap::new(),
            reservations: 0,
            started: 0,
            keep: BTreeMap::new(),
            keep_for: BTreeSet::new(),
            city_terrains: BTreeSet::new(),
            rebuild_at: crate::rebuild::spots(memory),
        }
        .with_first_sites(memory)
    }

    /// Keeps back the price of the first gather site of every base resource the player has
    /// no site for yet (the cheapest site type it can build, unless a search for it just
    /// failed): spending everything on other wishes would leave the resource unearned, and
    /// a site paid in its own resource could never be bought again.
    fn with_first_sites(mut self, memory: &Memory) -> Self {
        let view = &self.view;
        let situation = site_situation(view);
        for resource in view.rules.resources.values() {
            if resource.kind != ResourceKind::Base {
                continue;
            }
            let sites = view
                .cat
                .gather_sites
                .get(&resource.id)
                .map(Vec::as_slice)
                .unwrap_or_default();
            if sites.iter().any(|s| view.line_count(*s) > 0) {
                continue;
            }
            let cheapest = sites
                .iter()
                .copied()
                .filter(|s| {
                    view.unlocked(*s)
                        && !view.obsolete(*s)
                        && memory.no_site.get(&s.0) != Some(&situation)
                })
                .map(|s| {
                    let price = view.price(s);
                    (price.iter().map(|(_, a)| *a).sum::<i64>(), s, price)
                })
                .min_by_key(|(total, s, _)| (*total, *s));
            if let Some((_, _, price)) = cheapest {
                for (paid_in, amount) in price {
                    *self.keep.entry(paid_in).or_default() += amount;
                }
                self.keep_for.extend(sites.iter().copied());
            }
        }
        self
    }

    /// Writes an order if the budget allows; the executor is busy for the rest of the thought.
    pub fn order(
        &mut self,
        executor: EntityId,
        kind: &str,
        subject: Option<DefId>,
        target_entity: Option<EntityId>,
        target_pos: Option<Pos>,
    ) -> bool {
        if self.orders_left <= 0 {
            return false;
        }
        self.orders_left -= 1;
        self.view.busy.insert(executor);
        self.out.push(Command {
            player: self.view.me,
            executor,
            kind: kind.into(),
            subject,
            target_entity,
            target_pos,
            queue: false,
            text: None,
            facing: None,
        });
        true
    }

    /// The price of a definition now; what this thought already bought is in the view's
    /// counts, so ramping prices rise with it.
    pub fn price(&self, def: DefId) -> Vec<(DefId, i64)> {
        self.view.price(def)
    }

    pub fn affordable(&self, price: &[(DefId, i64)]) -> bool {
        price.iter().all(|(resource, amount)| {
            self.view.purse.get(resource).copied().unwrap_or(0)
                - self.keep.get(resource).copied().unwrap_or(0)
                >= *amount
        })
    }

    /// Whether a building can be paid; the first gather site of a resource may spend what
    /// is kept back for it.
    fn affordable_site(&self, building: DefId, price: &[(DefId, i64)]) -> bool {
        if !self.keep_for.contains(&building) {
            return self.affordable(price);
        }
        price.iter().all(|(resource, amount)| {
            self.view.purse.get(resource).copied().unwrap_or(0) >= *amount
        })
    }

    pub fn pay(&mut self, price: &[(DefId, i64)]) {
        for (resource, amount) in price {
            *self.view.purse.entry(*resource).or_default() -= amount;
        }
    }

    /// Notes what a wish lacks and, when it is worth saving for, keeps its price aside.
    pub fn lack(&mut self, price: &[(DefId, i64)], reserve: bool) {
        let reserve = reserve && self.reservations < RESERVATIONS;
        if reserve {
            self.reservations += 1;
        }
        for (resource, amount) in price {
            let purse = self.view.purse.entry(*resource).or_default();
            let missing = (*amount - (*purse).max(0)).max(0);
            if missing > 0 {
                *self.shortfall.entry(*resource).or_default() += missing;
            }
            if reserve {
                *purse -= amount;
            }
        }
    }

    /// Tries to turn a wish into an order.
    pub fn serve(&mut self, wish: &Wish, memory: &mut Memory) -> Outcome {
        let subject = wish.act.subject();
        if self.orders_left <= 0
            || memory
                .blocked
                .get(&subject.0)
                .is_some_and(|until| *until > self.view.tick)
            || !self.view.unlocked(subject)
        {
            return Outcome::Impossible;
        }
        match wish.act {
            Act::Train(unit) => self.train(unit, wish.reserve),
            Act::Research(tech) => self.research(tech, wish.reserve),
            Act::Construct(building) => self.construct(building, wish, memory),
        }
    }

    fn within_limit(&self, type_id: DefId) -> bool {
        let et = &self.view.rules.entity_types[&type_id];
        !matches!(et.limit_scope, LimitScope::PerPlayer | LimitScope::PerMatch)
            || self.view.count(type_id) < et.limit_count
    }

    /// Whether a producer's queue holds a research already.
    fn researches(&self, producer: EntityId) -> bool {
        self.view.researching_at.contains(&producer)
    }

    fn queue_of(&self, producer: EntityId) -> usize {
        self.view.queue_len.get(&producer).copied().unwrap_or(0)
    }

    fn train(&mut self, unit: DefId, reserve: bool) -> Outcome {
        let Some(et) = self.view.rules.entity_types.get(&unit) else {
            return Outcome::Impossible;
        };
        if self.view.obsolete(unit)
            || !self.within_limit(unit)
            || self.view.pop_used + self.view.queued_pop + et.pop_cost > self.view.pop_cap
        {
            return Outcome::Impossible;
        }
        let Some(producer) = self
            .view
            .producers(ProductionMode::Train, unit)
            .into_iter()
            .filter(|p| self.queue_of(p.id) < TRAIN_QUEUE)
            .min_by_key(|p| (self.queue_of(p.id), p.id))
            .map(|p| p.id)
        else {
            return Outcome::Impossible;
        };
        let price = self.price(unit);
        if !self.affordable(&price) {
            self.lack(&price, reserve);
            return Outcome::Unaffordable;
        }
        let busy = self.view.busy.contains(&producer);
        if !self.order(producer, "train", Some(unit), None, None) {
            return Outcome::Impossible;
        }
        if !busy {
            // A producer takes several orders in one thought.
            self.view.busy.remove(&producer);
        }
        self.out.last_mut().expect("order written").queue = true;
        self.pay(&price);
        self.view.note_planned(unit);
        self.view.queued_pop += et.pop_cost;
        *self.view.queue_len.entry(producer).or_default() += 1;
        Outcome::Ordered
    }

    fn research(&mut self, tech: DefId, reserve: bool) -> Outcome {
        let Some(row) = self.view.rules.techs.get(&tech) else {
            return Outcome::Impossible;
        };
        if (self.view.researched(tech) && !row.repeatable) || self.view.researching.contains(&tech)
        {
            return Outcome::Impossible;
        }
        if !row.exclusive_group.is_empty()
            && self
                .view
                .active
                .iter()
                .chain(self.view.researching.iter())
                .any(|other| {
                    *other != tech
                        && self
                            .view
                            .rules
                            .techs
                            .get(other)
                            .is_some_and(|t| t.exclusive_group == row.exclusive_group)
                })
        {
            return Outcome::Impossible;
        }
        // A producer busy training takes the research behind what it trains, so upgrades
        // are not held back for good by a building that never stops training.
        let Some(producer) = self
            .view
            .producers(ProductionMode::Research, tech)
            .into_iter()
            .filter(|p| self.queue_of(p.id) < TRAIN_QUEUE && !self.researches(p.id))
            .min_by_key(|p| (self.queue_of(p.id), p.id))
            .map(|p| p.id)
        else {
            return Outcome::Impossible;
        };
        let price = self.price(tech);
        if !self.affordable(&price) {
            self.lack(&price, reserve);
            return Outcome::Unaffordable;
        }
        if !self.order(producer, "research", Some(tech), None, None) {
            return Outcome::Impossible;
        }
        self.out.last_mut().expect("order written").queue = true;
        self.pay(&price);
        self.view.researching.insert(tech);
        self.view.researching_at.insert(producer);
        *self.view.queue_len.entry(producer).or_default() += 1;
        Outcome::Ordered
    }

    /// Own units that could start building `building` now: not already building. A builder
    /// that does not move (an automated colony) is left to its governor.
    fn builders(&self, building: DefId) -> Vec<&'a Entity> {
        self.view
            .producers(ProductionMode::Construct, building)
            .into_iter()
            .filter(|e| {
                self.view.rules.entity_types[&e.type_id].movement.is_some()
                    && !self.view.busy.contains(&e.id)
                    && !e
                        .orders()
                        .iter()
                        .any(|o| matches!(o.kind.as_str(), "construct" | "repair"))
            })
            .collect()
    }

    fn construct(&mut self, building: DefId, wish: &Wish, memory: &mut Memory) -> Outcome {
        let situation = site_situation(&self.view);
        if memory.no_site.get(&building.0) == Some(&situation)
            || self.view.obsolete(building)
            || !self.within_limit(building)
        {
            return Outcome::Impossible;
        }
        let is_city = self.view.is_city(building);
        if is_city
            && !self.view.is_colony(building)
            && self.view.line_count(building) >= self.view.city_limit()
        {
            return Outcome::Impossible;
        }
        let builders = self.builders(building);
        if builders.is_empty() {
            return Outcome::Impossible;
        }
        let price = self.price(building);
        if !self.affordable_site(building, &price) {
            self.lack(&price, wish.reserve);
            return Outcome::Unaffordable;
        }
        if self.searched {
            return Outcome::Impossible;
        }
        let ground = self.ground.get_or_insert_with(|| {
            let mut ground = Ground::new(&self.view, &memory.enemies);
            if let Some(rally) = memory.rally {
                ground.keep_clear(rally, crate::production::RALLY_CLEAR);
            }
            ground
        });
        if !ground.reserved_done {
            ground.reserved_done = true;
            reserve_first_sites(&self.view, ground, memory, situation);
            // The places of lost buildings are kept for their lines as loosely as the next
            // site of a resource: other buildings take them only when there is no other room.
            for (root, spots) in &self.rebuild_at {
                if let Some(et) = self.view.rules.entity_types.get(root) {
                    for spot in spots {
                        ground.reserve(et, *spot, Hold::Next);
                    }
                }
            }
        }
        // Where a lost building of the line stood, if the ground is still free.
        let mut again = Site::Nowhere;
        if !is_city {
            for spot in self
                .rebuild_at
                .get(&self.view.cat.root_of(building))
                .into_iter()
                .flatten()
            {
                again = at_spot(&self.view, ground, building, *spot);
                if again != Site::Nowhere {
                    break;
                }
            }
        }
        let site = match again {
            Site::At(_) => again,
            _ if is_city => find_city_site(&self.view, ground, building, &self.city_terrains),
            _ => find_site(&self.view, ground, building),
        };
        let site = match site {
            Site::At(site) => site,
            // Either way the type is left alone until the situation changes, so one
            // hopeless search cannot hold up the wishes behind it thought after thought.
            Site::Nowhere | Site::Later => {
                self.searched = true;
                memory.no_site.insert(building.0, situation);
                return Outcome::Impossible;
            }
        };
        let builder = builders
            .into_iter()
            .filter(|e| e.pos.body == site.body)
            .min_by_key(|e| (!self.view.is_idle(e), self.view.distance(e.pos, site), e.id))
            .map(|e| e.id);
        let Some(builder) = builder else {
            return Outcome::Impossible;
        };
        if !self.order(builder, "construct", Some(building), None, Some(site)) {
            return Outcome::Impossible;
        }
        self.pay(&price);
        if let Some(ground) = &mut self.ground {
            ground.plan(&self.view, building, site);
        }
        self.view.note_planned(building);
        self.started += 1;
        Outcome::Ordered
    }
}

/// Keeps ground free for gather sites drawn from node terrain (forest, rock): for every base
/// resource the player has no such site for — also one whose age has not come yet, so the
/// only spot beside the rock is not given to a farm before mines are known — and for every
/// resource whose sites hold fewer workers than twice its commerce cap pays for, the spot
/// its next site would take is kept free of other buildings, also while that very site is
/// being placed (else another node site takes its spot); nothing after a search for it
/// found nothing in this situation.
fn reserve_first_sites(view: &View<'_>, ground: &mut Ground, memory: &Memory, situation: u64) {
    let rules = view.rules;
    let cap = view.commerce_cap();
    let mut wanted: Vec<(Hold, DefId)> = Vec::new();
    for (resource, sites) in &view.cat.gather_sites {
        let Some(row) = rules.resources.get(resource) else {
            continue;
        };
        if row.kind != ResourceKind::Base {
            continue;
        }
        let node_site = |s: &DefId| {
            rules.is_available(*s, view.faction)
                && !view.obsolete(*s)
                && rules.entity_types[s]
                    .gather_site
                    .as_ref()
                    .is_some_and(|g| g.slots_per_node > 0 && g.needs_node_terrain_id.is_some())
        };
        // The site the player can build now, else the first one still to come.
        let Some(site) = sites
            .iter()
            .copied()
            .filter(node_site)
            .find(|s| view.unlocked(*s))
            .or_else(|| sites.iter().copied().find(node_site))
        else {
            continue;
        };
        let first = !sites.iter().any(|s| view.line_count(*s) > 0);
        if !first {
            // Sites stand already: room is kept for another while they hold fewer workers
            // than twice the commerce cap pays for.
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
                .unwrap_or(0);
            let slots: i64 = view
                .own
                .iter()
                .filter(|e| sites.contains(&e.type_id))
                .map(|e| {
                    if e.build_progress.is_some() {
                        crate::sites::site_slots(view, e.type_id, e.pos)
                    } else {
                        e.gather_slots
                    }
                })
                .sum();
            // Room is kept ahead of the cap: it rises with every commerce technology, and by
            // then farms and barracks have taken the ground beside the wood.
            if !row.rate_capped || per_worker <= 0 || slots * per_worker >= NEXT_SITE_CAP * cap {
                continue;
            }
        }
        // A search that already failed in this situation is not repeated for a reservation.
        if memory.no_site.get(&site.0) == Some(&situation) {
            continue;
        }
        // The first site of a resource the player can build now keeps its spot whatever
        // else wants it; one whose age has not come only while other buildings find room
        // elsewhere (a town on a small island had no room for a farm), and the next one
        // gives way before it.
        let hold = match (first, view.unlocked(site)) {
            (true, true) => Hold::First,
            (true, false) => Hold::Coming,
            (false, _) => Hold::Next,
        };
        wanted.push((hold, site));
    }
    // The firmest first: a next woodcutter's camp must not take the spot of the first mine.
    wanted.sort_by_key(|(hold, site)| (std::cmp::Reverse(*hold), *site));
    for (hold, site) in wanted {
        if let Some((pos, _)) = node_spot(view, ground, site) {
            ground.reserve(&rules.entity_types[&site], pos, hold);
        }
    }
}

/// Node sites keep room for another until their slots pay this many times the commerce cap.
const NEXT_SITE_CAP: i64 = 2;

/// Whether the last search for a site of `building` came back empty with as many cities as
/// now and in the same age: there is no room for it around them.
pub(crate) fn no_room_for(view: &View<'_>, memory: &Memory, building: DefId) -> bool {
    let now = site_situation(view);
    // Cities and age, the bits of the situation that free ground.
    let place = |s: u64| (s >> 24) & 0xffff;
    memory
        .no_site
        .get(&building.0)
        .is_some_and(|then| place(*then) == place(now))
}

/// A number that changes when a failed site search is worth repeating: a city was
/// finished, a few more buildings stand (cities grow with them), the age advanced, or a
/// while has passed.
pub(crate) fn site_situation(view: &View<'_>) -> u64 {
    let cities = view.cities.iter().filter(|c| c.complete).count() as u64;
    let buildings = view
        .own
        .iter()
        .filter(|e| e.build_progress.is_none() && view.is_building(e.type_id))
        .count() as u64;
    ((buildings / 3) << 40)
        | ((cities & 0xff) << 32)
        | ((view.age_ordinal() as u64 & 0xff) << 24)
        | ((view.tick / RETRY_TICKS) & 0xff_ffff)
}
