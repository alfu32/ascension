//! Task 141: workers called up as militia (`call_up`) and stood down again (`stand_down`), at
//! once and for nothing, while they stand on their owner's territory. Which worker becomes which
//! militia is data (`et_call_up` and the two upgrade lines); the unit keeps its id and place,
//! its orders are cleared and its health keeps its share of the maximum. One that cannot stand
//! where it is as its new type moves to the nearest tile it can, or waits (task 172).

use crate::{EntityId, Order, PlayerId, SimError, World};
use rom_rules::DefId;

/// The order that calls a worker up as militia.
pub const CALL_UP: &str = "call_up";
/// The order that stands militia down into their worker again.
pub const STAND_DOWN: &str = "stand_down";

impl World {
    /// The newest step of the upgrade line beginning at `root` that `player` has unlocked and
    /// its faction may use: from `root`, the lowest-id unlocked successor, as long as there is
    /// one.
    fn newest_step(&self, player: PlayerId, root: DefId) -> DefId {
        let Some(p) = self.players.get(&player) else {
            return root;
        };
        let (sources, owned) = (p.active_sources(), self.owned_types(player));
        let mut current = root;
        let mut seen = std::collections::BTreeSet::new();
        while seen.insert(current) {
            let next = self.rules.entity_types.values().find(|next| {
                next.upgrades_from_id == Some(current)
                    && self.rules.is_available(next.id, p.faction)
                    && self.rules.is_unlocked(next.id, &sources, &owned)
            });
            match next {
                Some(next) => current = next.id,
                None => break,
            }
        }
        current
    }

    /// Whether units of `type_id` can be called up as militia (an `et_call_up` row).
    pub fn calls_up(&self, type_id: DefId) -> bool {
        self.rules
            .entity_type(type_id)
            .is_some_and(|et| et.call_up.is_some())
    }

    /// Whether units of `type_id` are militia that stand down into a worker.
    pub fn is_militia_type(&self, type_id: DefId) -> bool {
        self.rules.militia_worker(type_id).is_some()
    }

    /// What `id` becomes on a `call_up` or `stand_down` order now, or why it cannot: only a
    /// worker with a call-up is called up, only militia stand down, and either only while the
    /// unit stands on a tile of its owner's territory.
    pub fn militia_change(&self, id: EntityId, kind: &str) -> Result<DefId, SimError> {
        let e = self.entity(id).ok_or(SimError::UnknownExecutor(id))?;
        let root = match kind {
            CALL_UP => {
                let call_up = self.rules.entity_types[&e.type_id]
                    .call_up
                    .as_ref()
                    .ok_or(SimError::CannotCallUp)?;
                call_up.militia_type_id
            }
            STAND_DOWN => self
                .rules
                .militia_worker(e.type_id)
                .ok_or(SimError::CannotStandDown)?,
            _ => return Err(SimError::UnknownKind(kind.into())),
        };
        let inside = self.territory_owner_on_body(
            e.pos.body,
            e.pos.face,
            e.pos.x.div_euclid(1000),
            e.pos.y.div_euclid(1000),
        ) == Some(e.owner);
        if !inside || self.orbit(id).is_some() {
            return Err(SimError::MilitiaOutsideTerritory);
        }
        let faction = self.players.get(&e.owner).map(|p| p.faction);
        if faction.is_none_or(|f| !self.rules.is_available(root, f)) {
            return Err(SimError::Unavailable);
        }
        Ok(self.newest_step(e.owner, root))
    }

    /// Applies a `call_up` or `stand_down` order as it leaves `pending`: the unit's orders are
    /// cleared (it leaves the site it works at) and it becomes its new type where it stands,
    /// keeping its share of health. Refused now, the order is cancelled.
    pub(crate) fn apply_militia(&mut self, order: &Order) {
        let id = order.executor;
        let Ok(target) = self.militia_change(id, &order.kind) else {
            self.stats.orders_cancelled += 1;
            return;
        };
        self.cancel_entity_orders(id);
        // A worker swimming where its militia cannot (task 172) comes ashore first, or the
        // change waits until it stands on ground the new type can.
        if !self.make_room(id, target) {
            self.retype_waiting.insert(id, target);
            self.stats.orders_done += 1;
            return;
        }
        self.retype_waiting.remove(&id);
        let e = self.entities[id.0 as usize]
            .as_mut()
            .expect("checked executor");
        let old = e.type_id;
        e.type_id = target;
        e.idle_ticks = 0;
        // Reloads belong to the weapons of the old type.
        let weapons: Vec<_> = self.rules.entity_types[&target]
            .weapons
            .iter()
            .map(|w| w.id)
            .collect();
        e.cooldowns.retain(|weapon, _| weapons.contains(weapon));
        self.retyped(id, old);
        self.refresh_entity_stats(id);
        self.stats.orders_done += 1;
    }
}
