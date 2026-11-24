//! Rally points (`set_rally`), disbanding (`disband`), granted handicaps and the read-only
//! queries computer players and interfaces share.

use crate::{Command, EntityId, Order, PlayerId, Pos, SimError, World};
use rom_rules::{ActiveSources, DefId, TechKind};

/// Where a producer sends the units it trains.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rally {
    /// Walk to this point.
    Point(Pos),
    /// Walk to this entity; a harvester whose resource the entity yields harvests there.
    Entity(EntityId),
}

impl World {
    /// The rally point of a producer, if one is set.
    pub fn rally(&self, producer: EntityId) -> Option<Rally> {
        self.rallies.get(&producer).copied()
    }

    /// Every rally point, by producer id.
    pub fn rallies(&self) -> impl Iterator<Item = (EntityId, Rally)> + '_ {
        self.rallies.iter().map(|(id, rally)| (*id, *rally))
    }

    /// Orders submitted and not yet applied; they join their executors' queues at the next
    /// step, in this order.
    pub fn pending_orders(&self) -> impl Iterator<Item = &Order> {
        self.pending.iter().map(|(order, _)| order)
    }

    /// The modifier sources of a player: researched and granted technologies, age and
    /// faction. Empty for an unknown player.
    pub fn active_sources(&self, player: PlayerId) -> ActiveSources {
        self.players
            .get(&player)
            .map(|p| p.active_sources())
            .unwrap_or_default()
    }

    /// Grants a player a handicap technology at match setup: its modifiers apply like a
    /// researched technology's (a difficulty's income bonus). Only technologies of kind
    /// `handicap` can be granted, and only before the first tick.
    pub fn grant_handicap(&mut self, player: PlayerId, tech: DefId) -> Result<(), SimError> {
        if self
            .rules
            .techs
            .get(&tech)
            .is_none_or(|t| t.kind != TechKind::Handicap)
            || self.tick != 0
        {
            return Err(SimError::NotAHandicap);
        }
        self.players
            .get_mut(&player)
            .ok_or(SimError::UnknownPlayer(player))?
            .techs
            .insert(tech, 1);
        let ids: Vec<_> = self
            .entities()
            .filter(|e| e.owner == player)
            .map(|e| e.id)
            .collect();
        for id in ids {
            self.refresh_entity_stats(id);
        }
        Ok(())
    }

    /// Checks `set_rally` and `disband` at submission.
    pub(crate) fn validate_rally_and_disband(&self, command: &Command) -> Result<(), SimError> {
        match command.kind.as_str() {
            "set_rally" => {
                let producer = self.entity(command.executor).expect("checked executor");
                let has_rally = self.rules.entity_types[&producer.type_id]
                    .producer
                    .as_ref()
                    .is_some_and(|p| p.has_rally);
                if !has_rally {
                    return Err(SimError::NoRallyPoint);
                }
                if command
                    .target_entity
                    .is_some_and(|target| self.entity(target).is_none())
                {
                    return Err(SimError::InvalidRallyTarget);
                }
                Ok(())
            }
            "disband" if !self.passengers(command.executor).is_empty() => {
                Err(SimError::PassengersAboard)
            }
            _ => Ok(()),
        }
    }

    /// Sets or clears (the producer itself as target) a rally point; `set_rally` takes effect
    /// when it leaves `pending` and leaves the producer's queue alone.
    pub(crate) fn apply_rally(&mut self, order: &Order) {
        let rally = match (order.target_entity, order.target_pos) {
            (Some(target), _) if target == order.executor => None,
            (Some(target), _) => Some(Rally::Entity(target)),
            (None, Some(pos)) => Some(Rally::Point(pos)),
            (None, None) => None,
        };
        match rally {
            Some(rally) => self.rallies.insert(order.executor, rally),
            None => self.rallies.remove(&order.executor),
        };
        self.stats.orders_done += 1;
    }

    /// Sends a unit a producer just trained to the producer's rally point: a harvester to a
    /// gather site it can work harvests there, anything else walks; a ship a spaceport launched
    /// moves within its orbit to above the point. Nothing happens without a rally point, for a
    /// unit that cannot move, or when the point is on another body.
    pub(crate) fn send_to_rally(&mut self, producer: EntityId, unit: EntityId) {
        let Some(rally) = self.rally(producer) else {
            return;
        };
        let Some(e) = self.entity(unit) else {
            return;
        };
        if self.rules.entity_types[&e.type_id].movement.is_none() {
            return;
        }
        let (player, body) = (e.owner, e.pos.body);
        let order = |kind: &str, target_entity, target_pos| Command {
            player,
            executor: unit,
            kind: kind.into(),
            subject: None,
            target_entity,
            target_pos,
            queue: false,
            text: None,
        };
        let goal = match rally {
            Rally::Point(pos) => pos,
            Rally::Entity(target) => {
                let Some(pos) = self.entity(target).map(|t| t.pos) else {
                    return;
                };
                if pos.body == body && self.submit(order("harvest", Some(target), None)).is_ok() {
                    return;
                }
                pos
            }
        };
        // A ship in orbit moves within its orbit to above the point (task 142): it stays in
        // orbit and never lands there.
        let orbiting = self.orbit(unit).is_some_and(|o| o.body == goal.body);
        if goal.body == body || orbiting {
            let _ = self.submit(order("move", None, Some(goal)));
        }
    }

    /// Removes the units whose queued `disband` came up this tick.
    pub(crate) fn remove_disbanded(&mut self) {
        for id in std::mem::take(&mut self.disbanded) {
            if self.entity(id).is_some() {
                self.despawn(id).expect("disbanded unit");
            }
        }
    }
}
