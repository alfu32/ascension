//! One thought's working state: the view, the orders written so far, the money left, and
//! the wishes the managers put forward.

use crate::{
    memory::Memory,
    sites::{Ground, Site, find_city_site, find_site},
    view::View,
};
use rom_rules::{DefId, LimitScope, ProductionMode};
use rom_sim::{Command, Entity, EntityId, Pos};
use std::collections::BTreeMap;

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
}

impl<'a> Plan<'a> {
    pub fn new(view: View<'a>, orders_left: i64) -> Self {
        Self {
            view,
            ground: None,
            searched: false,
            out: Vec::new(),
            orders_left,
            shortfall: BTreeMap::new(),
            reservations: 0,
            started: 0,
        }
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
        });
        true
    }

    /// The price of a definition now; what this thought already bought is in the view's
    /// counts, so ramping prices rise with it.
    pub fn price(&self, def: DefId) -> Vec<(DefId, i64)> {
        self.view.price(def)
    }

    fn affordable(&self, price: &[(DefId, i64)]) -> bool {
        price.iter().all(|(resource, amount)| {
            self.view.purse.get(resource).copied().unwrap_or(0) >= *amount
        })
    }

    fn pay(&mut self, price: &[(DefId, i64)]) {
        for (resource, amount) in price {
            *self.view.purse.entry(*resource).or_default() -= amount;
        }
    }

    /// Notes what a wish lacks and, when it is worth saving for, keeps its price aside.
    fn lack(&mut self, price: &[(DefId, i64)], reserve: bool) {
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
        let Some(producer) = self
            .view
            .producers(ProductionMode::Research, tech)
            .into_iter()
            .filter(|p| self.queue_of(p.id) == 0)
            .map(|p| p.id)
            .next()
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
        *self.view.queue_len.entry(producer).or_default() += 1;
        Outcome::Ordered
    }

    /// Own units that could start building `building` now: not already building.
    fn builders(&self, building: DefId) -> Vec<&'a Entity> {
        self.view
            .producers(ProductionMode::Construct, building)
            .into_iter()
            .filter(|e| {
                !self.view.busy.contains(&e.id)
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
        if is_city && self.view.line_count(building) >= self.view.city_limit() {
            return Outcome::Impossible;
        }
        let builders = self.builders(building);
        if builders.is_empty() {
            return Outcome::Impossible;
        }
        let price = self.price(building);
        if !self.affordable(&price) {
            self.lack(&price, wish.reserve);
            return Outcome::Unaffordable;
        }
        if self.searched {
            return Outcome::Impossible;
        }
        let ground = self
            .ground
            .get_or_insert_with(|| Ground::new(&self.view, &memory.enemies));
        let site = if is_city {
            find_city_site(&self.view, ground, building)
        } else {
            find_site(&self.view, ground, building)
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
