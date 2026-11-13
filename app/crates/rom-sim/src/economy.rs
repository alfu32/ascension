//! Resource accounting, ramp counters, and submission checks.

use crate::{Command, EntityId, Order, PlayerId, SimError, World};
use rom_rules::{DefId, EntityClass, LimitScope, Owned, ProductionMode, RampMode};

impl World {
    /// Grants a nonnegative resource amount for scenario setup.
    pub fn grant(
        &mut self,
        player: PlayerId,
        resource: DefId,
        amount: i64,
    ) -> Result<(), SimError> {
        if !self.rules.resources.contains_key(&resource) {
            return Err(SimError::UnknownResource(resource));
        }
        if amount < 0 {
            return Err(SimError::InvalidRules("negative resource grant"));
        }
        let player = self
            .players
            .get_mut(&player)
            .ok_or(SimError::UnknownPlayer(player))?;
        let balance = player.resources.entry(resource).or_default();
        *balance = balance.saturating_add(amount);
        Ok(())
    }

    /// Returns a player's completed entity types, or an empty set for an unknown player.
    pub fn owned_types(&self, player: PlayerId) -> Owned {
        self.players
            .get(&player)
            .map_or_else(Owned::new, |p| p.owned_types(self))
    }

    /// Returns population occupied by units, including unfinished units.
    pub fn population_used(&self, player: PlayerId) -> i64 {
        self.entities()
            .filter(|e| e.owner == player)
            .map(|e| &self.rules.entity_types[&e.type_id])
            .filter(|et| et.class == EntityClass::Unit)
            .map(|et| et.pop_cost)
            .sum()
    }

    /// Evaluates the player's current population cap.
    pub fn population_cap(&self, player: PlayerId) -> i64 {
        self.rules.player_stat(
            "pop_cap",
            self.rules
                .constant("player.pop_cap.base")
                .expect("validated population constant"),
            &self.players[&player].active_sources(),
        )
    }

    /// Prices a subject with owned and queued ramp counts before applying modifiers.
    pub fn price(&self, player: PlayerId, def: DefId) -> Vec<(DefId, i64)> {
        let sources = self.players[&player].active_sources();
        let n = self.rules.entity_type(def).map_or(0, |et| {
            self.count_subjects(Some(player), |id| match et.ramp_mode {
                RampMode::None => false,
                RampMode::PerType => id == def,
                RampMode::PerGroup => self
                    .rules
                    .entity_type(id)
                    .is_some_and(|other| other.ramp_group == et.ramp_group),
            })
        });
        self.rules
            .cost_of_ramped(def, &sources, u64::try_from(n).unwrap_or(0))
    }

    pub(crate) fn all_orders(&self) -> impl Iterator<Item = &Order> {
        self.entities()
            .flat_map(|e| e.orders.iter())
            .chain(self.pending.iter().map(|(o, _)| o))
    }

    fn count_subjects(&self, player: Option<PlayerId>, matches: impl Fn(DefId) -> bool) -> i64 {
        let owned = self
            .entities()
            .filter(|e| player.is_none_or(|p| p == e.owner) && matches(e.type_id))
            .count();
        let queued = self
            .all_orders()
            .filter(|o| {
                player.is_none_or(|p| p == o.player)
                    && (o.kind == "train" || (o.kind == "construct" && o.target_entity.is_none()))
                    && o.subject.is_some_and(&matches)
            })
            .count();
        (owned + queued) as i64
    }

    pub(crate) fn validate_economy_order(
        &self,
        command: &mut Command,
    ) -> Result<Vec<(DefId, i64)>, SimError> {
        let mode = match command.kind.as_str() {
            "train" => ProductionMode::Train,
            "research" => ProductionMode::Research,
            "construct" => ProductionMode::Construct,
            _ => return Ok(Vec::new()),
        };
        let executor = self.entity(command.executor).expect("checked executor");
        let subject = command.subject.ok_or(SimError::WrongTargetShape)?;
        if !self
            .rules
            .production
            .get(&executor.type_id)
            .into_iter()
            .flatten()
            .any(|p| p.product_def_id == subject && p.mode == mode)
            || match mode {
                ProductionMode::Research => !self.rules.techs.contains_key(&subject),
                ProductionMode::Construct => self
                    .rules
                    .entity_type(subject)
                    .is_none_or(|e| e.class != EntityClass::Building),
                ProductionMode::Train => !self.rules.entity_types.contains_key(&subject),
                ProductionMode::Morph => true,
            }
        {
            return Err(SimError::ProductionUnavailable);
        }
        if executor.build_progress.is_some() {
            return Err(SimError::IncompleteExecutor);
        }
        let player = &self.players[&command.player];
        if !self.rules.is_available(subject, player.faction) {
            return Err(SimError::Unavailable);
        }
        if !self
            .rules
            .is_unlocked(subject, &player.active_sources(), &player.owned_types(self))
        {
            return Err(SimError::Locked);
        }
        if let Some(et) = self.rules.entity_type(subject) {
            let owner = match et.limit_scope {
                LimitScope::PerPlayer => Some(Some(command.player)),
                LimitScope::PerMatch => Some(None),
                _ => None,
            };
            if owner.is_some_and(|owner| {
                self.count_subjects(owner, |id| id == subject) >= et.limit_count
            }) {
                return Err(SimError::LimitReached);
            }
            if mode == ProductionMode::Construct
                && self.is_city_type(subject)
                && self.city_count(command.player) >= self.city_limit(command.player)
            {
                return Err(SimError::CityLimit);
            }
        }
        if mode != ProductionMode::Construct {
            let producer = self.rules.entity_types[&executor.type_id]
                .producer
                .as_ref()
                .ok_or_else(|| SimError::MissingComponent("producer".into()))?;
            let queued = self
                .all_orders()
                .filter(|o| o.executor == command.executor)
                .count() as i64;
            if queued >= producer.queue_limit {
                return Err(SimError::QueueFull);
            }
        }
        if mode == ProductionMode::Research {
            self.check_research(command.player, subject)?;
        }
        if mode == ProductionMode::Construct {
            let pos = command.target_pos.ok_or(SimError::WrongTargetShape)?;
            let pos = self.check_placement(command.player, subject, pos)?;
            self.check_city_limit(command.player, subject, pos)?;
            command.target_pos = Some(pos);
        }
        let price = self.price(command.player, subject);
        if price.iter().any(|(resource, amount)| {
            player.resources.get(resource).copied().unwrap_or(0) < *amount
        }) {
            return Err(SimError::InsufficientResources);
        }
        Ok(price)
    }

    pub(crate) fn cancel_orders(&mut self, executor: EntityId) {
        self.set_work_site(executor, None);
        self.economy.dirty.insert(executor);
        let orders = std::mem::take(
            &mut self.entities[executor.0 as usize]
                .as_mut()
                .expect("live executor")
                .orders,
        );
        self.stats.orders_cancelled += orders.len() as u64;
        let pct = self
            .rules
            .constant("economy.cancel_refund_pct")
            .expect("validated refund constant");
        for order in orders {
            if matches!(order.kind.as_str(), "train" | "research") {
                for (resource, amount) in order.payment {
                    let refund = (i128::from(amount) * i128::from(pct) / 100) as i64;
                    let balance = self
                        .players
                        .get_mut(&order.player)
                        .expect("checked owner")
                        .resources
                        .entry(resource)
                        .or_default();
                    *balance = balance.saturating_add(refund);
                }
            }
        }
    }
}
