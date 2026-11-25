//! Orders a player needs besides walking and fighting: exploring, patrolling, guarding,
//! combat stances as orders, razing own buildings and keeping a formation to its slowest
//! member's pace; the report of accepted orders that failed while they ran; and terrain
//! reachability.

use crate::{
    Command, EntityId, Order, OrderFailure, OrderStatus, PlayerId, Pos, SimError, World,
    movement::Walk,
};
use rom_rules::{DefId, EntityClass, StanceFamily};
use std::collections::BTreeMap;
use std::sync::OnceLock;

/// Failures kept per player; older ones are dropped.
pub const FAILURE_LOG: usize = 64;
/// Tiles around a unit an `explore` order searches for ground its player has not seen.
const EXPLORE_TILES: i32 = 96;
/// How close a guard keeps to its ward, in milli-tiles.
const GUARD_SLACK: u64 = 3000;
/// How far around its ward a guard looks for enemies, in milli-tiles.
const GUARD_WATCH: u64 = 8000;

/// One formation's pace: player, formation and tick of issue.
type FormationKey = (PlayerId, DefId, u64);

/// Tile offsets within `EXPLORE_TILES`, nearest first (ties in a fixed order).
fn explore_offsets() -> &'static [(i32, i32)] {
    static OFFSETS: OnceLock<Vec<(i32, i32)>> = OnceLock::new();
    OFFSETS.get_or_init(|| {
        let r = EXPLORE_TILES;
        let mut out: Vec<(i32, i32)> = (-r..=r)
            .flat_map(|dy| (-r..=r).map(move |dx| (dx, dy)))
            .filter(|(dx, dy)| dx * dx + dy * dy <= r * r)
            .collect();
        out.sort_by_key(|(dx, dy)| (dx * dx + dy * dy, *dy, *dx));
        out
    })
}

impl World {
    /// Notes a failed order for its player's report; automatic orders are the simulation's
    /// own and are not reported.
    pub(crate) fn record_failure(&mut self, order: &Order, reason: &str) {
        if order.automatic || order.player == PlayerId(0) {
            return;
        }
        let log = self.failures.entry(order.player).or_default();
        if log.len() >= FAILURE_LOG {
            log.pop_front();
        }
        log.push_back(OrderFailure {
            tick: self.tick,
            order: order.id,
            executor: order.executor,
            kind: order.kind.clone(),
            subject: order.subject,
            target_entity: order.target_entity,
            target_pos: order.target_pos,
            reason: reason.into(),
        });
    }

    /// A player's latest failures of accepted orders (`no_path`, `no_slot`, `target_gone`,
    /// …), oldest first; at most `FAILURE_LOG` are kept. What the interface tells a person.
    pub fn order_failures(&self, player: PlayerId) -> impl Iterator<Item = &OrderFailure> {
        self.failures.get(&player).into_iter().flatten()
    }

    /// Whether a combat stance may be set on units.
    pub(crate) fn settable_stance(&self, stance: DefId) -> bool {
        self.rules.stances.get(&stance).is_some_and(|s| {
            s.family == StanceFamily::Combat
                && matches!(
                    s.behavior.as_str(),
                    "aggressive" | "defensive" | "stand_ground" | "hold_fire" | "raid" | "raze"
                )
        })
    }

    /// Sets an entity's combat stance and drops its automatic pursuits.
    pub(crate) fn put_stance(&mut self, id: EntityId, stance: DefId) {
        let entity = self.entities[id.0 as usize].as_mut().expect("owned entity");
        entity.stance = stance;
        let before = entity.orders.len();
        entity.orders.retain(|o| !o.automatic);
        self.stats.orders_cancelled += (before - entity.orders.len()) as u64;
        self.pending.retain(|(o, _)| {
            let remove = o.executor == id && o.automatic;
            if remove {
                self.stats.orders_cancelled += 1;
            }
            !remove
        });
        self.economy.dirty.insert(id);
    }

    /// Checks `stance`, `raze`, `guard` and the formation subject of movement orders.
    pub(crate) fn validate_tactics(&self, command: &Command) -> Result<(), SimError> {
        let executor = self.entity(command.executor).expect("checked executor");
        match command.kind.as_str() {
            "stance" if !command.subject.is_some_and(|s| self.settable_stance(s)) => {
                Err(SimError::InvalidStance)
            }
            "raze"
                if self.rules.entity_types[&executor.type_id].class != EntityClass::Building
                    || (executor.build_progress.is_none()
                        && self.is_capturable(executor.type_id)) =>
            {
                Err(SimError::CannotRaze)
            }
            "delete" => self.check_delete(executor.id),
            "guard" => {
                let ward = command
                    .target_entity
                    .and_then(|id| self.entity(id))
                    .ok_or(SimError::InvalidGuardTarget)?;
                if ward.id == executor.id || self.enemies(command.player, ward.owner) {
                    return Err(SimError::InvalidGuardTarget);
                }
                Ok(())
            }
            "move" | "attack_move" | "patrol" | "orbit" | "travel"
                if command
                    .subject
                    .is_some_and(|f| !self.rules.formations.contains_key(&f)) =>
            {
                Err(SimError::UnknownFormation)
            }
            _ => Ok(()),
        }
    }

    /// Applies a `stance` order as it leaves `pending`; the queue stays.
    pub(crate) fn apply_stance(&mut self, order: &Order) {
        if let Some(stance) = order.subject.filter(|s| self.settable_stance(*s)) {
            self.put_stance(order.executor, stance);
            self.stats.orders_done += 1;
        } else {
            self.stats.orders_cancelled += 1;
        }
    }

    /// Applies a `raze` order as it leaves `pending`. An unfinished building is given up and
    /// its price for the work not yet done comes back, as `delete` refunds it (task 142); a
    /// finished one that cannot be captured is pulled down. Either way it falls with the dead
    /// of this tick.
    pub(crate) fn apply_raze(&mut self, order: &Order) {
        let Some(e) = self.entity(order.executor) else {
            self.stats.orders_cancelled += 1;
            return;
        };
        let (type_id, owner, progress) = (e.type_id, e.owner, e.build_progress);
        if progress.is_none() && self.is_capturable(type_id) {
            self.stats.orders_cancelled += 1;
            return;
        }
        if progress.is_some() {
            let refund = self.delete_refund(order.executor);
            self.credit(owner, &refund);
        }
        let e = self.entities[order.executor.0 as usize]
            .as_mut()
            .expect("razed building");
        e.hp = 0;
        e.killer = None;
        // Pulled down, not destroyed: it leaves no stain.
        self.razed.push(order.executor);
        self.stats.orders_done += 1;
    }

    /// Gives the movement orders applied this tick that name a formation keeping to its
    /// slowest member the pace of that member: orders of one player, formation and tick of
    /// issue make one formation, whatever their kind (the unarmed members of an attack move
    /// walk with it; task 143).
    pub(crate) fn pace_formations(&mut self, applied: &[(EntityId, crate::OrderId)]) {
        let mut groups: BTreeMap<FormationKey, Vec<(EntityId, crate::OrderId)>> = BTreeMap::new();
        for (executor, id) in applied {
            let Some(order) = self
                .entity(*executor)
                .and_then(|e| e.orders.iter().find(|o| o.id == *id))
            else {
                continue;
            };
            // A formation laid out about a shared goal has its pace already (task 143).
            let Some(formation) = order.subject.filter(|f| {
                order.pace.is_none()
                    && matches!(order.kind.as_str(), "move" | "attack_move" | "patrol")
                    && self
                        .rules
                        .formations
                        .get(f)
                        .is_some_and(|f| f.keeps_slowest)
            }) else {
                continue;
            };
            groups
                .entry((order.player, formation, order.issued_tick))
                .or_default()
                .push((*executor, *id));
        }
        for members in groups.into_values() {
            let slowest = members
                .iter()
                .filter_map(|(executor, _)| {
                    let e = self.entity(*executor)?;
                    let speed = self.rules.entity_types[&e.type_id].movement.as_ref()?.speed;
                    Some(self.effective_stat(*executor, "speed", speed).max(0))
                })
                .min();
            let Some(slowest) = slowest else { continue };
            for (executor, id) in members {
                if let Some(order) = self.entities[executor.0 as usize]
                    .as_mut()
                    .and_then(|e| e.orders.iter_mut().find(|o| o.id == id))
                {
                    order.pace = Some(slowest);
                }
            }
        }
    }

    /// Whether a unit of `type_id` could walk from `from` to `to` over the terrain: both on
    /// one body and in one connected stretch of ground its movement class can cross
    /// (buildings are not counted; they come and go). `None` for a type that does not move
    /// or a point off the surfaces.
    pub fn reachable(&self, type_id: DefId, from: Pos, to: Pos) -> Option<bool> {
        let class = self
            .rules
            .entity_types
            .get(&type_id)?
            .movement
            .as_ref()?
            .movement_class_id;
        let surface = self.surface(from.body)?;
        let a = surface.geo.pos_index(from)?;
        if from.body != to.body {
            return Some(false);
        }
        let b = surface.geo.pos_index(to)?;
        if surface.pathing.flies(class) {
            return Some(true);
        }
        let labels = surface.pathing.components(class)?;
        Some(labels[a] != 0 && labels[a] == labels[b])
    }

    /// The nearest tile centre within `EXPLORE_TILES` that the unit's player has not
    /// explored, its class can stand on and it can reach over the terrain.
    fn unexplored_near(&self, id: EntityId) -> Option<Pos> {
        let e = self.entity(id)?;
        let class = self.rules.entity_types[&e.type_id]
            .movement
            .as_ref()?
            .movement_class_id;
        let surface = self.surface(e.pos.body)?;
        let geo = &surface.geo;
        let labels = surface.pathing.components(class)?;
        let here = labels[geo.pos_index(e.pos)?];
        let flies = surface.pathing.flies(class);
        let centre = geo.tile_centre(geo.tile_of(e.pos));
        let mut seen = std::collections::BTreeSet::new();
        for (dx, dy) in explore_offsets() {
            let (at, _) = geo.offset(centre, i64::from(*dx) * 1000, i64::from(*dy) * 1000);
            let Some(index) = geo.pos_index(at) else {
                continue;
            };
            if !seen.insert(index) {
                continue;
            }
            let tile = geo.tile_at(index);
            if (flies || (here != 0 && labels[index] == here))
                && !self.is_explored_on_body(e.owner, e.pos.body, tile.face, tile.x, tile.y)
            {
                return Some(geo.index_centre(index));
            }
        }
        None
    }

    /// One tick of `explore`: walks to the nearest ground its player has not seen, picks
    /// the next once that ground is seen, and is done when none is left within reach.
    pub(crate) fn advance_explore(&mut self, id: EntityId, order: &mut Order) {
        order.progress_ticks += 1;
        let owner = self.entity(id).expect("explorer").owner;
        if let Some(goal) = order.target_pos {
            let tile = self.geo(goal.body).tile_of(goal);
            if self.is_explored_on_body(owner, goal.body, tile.face, tile.x, tile.y) {
                order.target_pos = None;
                order.movement = None;
            }
        }
        if order.target_pos.is_none() {
            match self.unexplored_near(id) {
                Some(goal) => order.target_pos = Some(goal),
                None => {
                    order.status = OrderStatus::Done;
                    return;
                }
            }
        }
        let goal = order.target_pos.expect("explore goal");
        if self.walk(id, order, goal, None, false) == Walk::Arrived {
            order.target_pos = None;
            order.movement = None;
        }
    }

    /// The enemy nearest a ward within `GUARD_WATCH` of it that the guard can see and aim
    /// at, if any.
    fn threat_to(&self, guard: EntityId, ward: Pos) -> Option<EntityId> {
        let owner = self.entity(guard)?.owner;
        let geo = self.geo(ward.body);
        self.entities_in_radius(ward, GUARD_WATCH as u32)
            .into_iter()
            .filter_map(|target| {
                let t = self.entity(target)?;
                (t.hp > 0
                    && t.container_id.is_none()
                    && !t.neutralised
                    && self.enemies(owner, t.owner)
                    && !(t.owner == PlayerId(0) && self.players[&t.owner].team == 0)
                    && self.sees_entity(owner, t)
                    && self.can_target(guard, target))
                .then(|| {
                    (
                        geo.rank_distance(ward, t.pos),
                        t.pos.face,
                        t.pos.y,
                        t.pos.x,
                        target,
                    )
                })
            })
            .min()
            .map(|(.., target)| target)
    }

    /// One tick of `guard`: stays beside its ward and fights off enemies that come near it,
    /// until the ward is gone.
    pub(crate) fn advance_guard(&mut self, id: EntityId, order: &mut Order) {
        order.progress_ticks += 1;
        let Some(ward) = order
            .target_entity
            .and_then(|w| self.entity(w))
            .filter(|w| w.hp > 0 && !self.enemies(order.player, w.owner))
            .map(|w| w.pos)
        else {
            order.status = OrderStatus::Failed("target_gone".into());
            return;
        };
        // An enemy fought off is let go once it is dead or has drawn the guard too far off.
        if let Some(enemy) = order.engaged
            && self.entity(enemy).is_none_or(|t| {
                t.hp <= 0
                    || self
                        .geo(ward.body)
                        .distance(t.pos, ward)
                        .is_none_or(|d| d > 2 * GUARD_WATCH)
            })
        {
            order.engaged = None;
            order.movement = None;
        }
        if order.engaged.is_none()
            && self.tick.is_multiple_of(10)
            && let Some(enemy) = self.threat_to(id, ward)
        {
            order.engaged = Some(enemy);
            order.movement = None;
        }
        if let Some(enemy) = order.engaged {
            let mut attack = order.clone();
            attack.kind = "attack".into();
            attack.target_entity = Some(enemy);
            attack.home = None;
            attack.status = OrderStatus::Active;
            self.advance_combat(id, &mut attack);
            order.movement = attack.movement;
            if attack.status != OrderStatus::Active {
                order.engaged = None;
                order.movement = None;
            }
            return;
        }
        // Beside a building means at the edge of its footprint, which nobody can stand in.
        let ward_entity = self
            .entity(order.target_entity.expect("ward"))
            .expect("ward");
        let ward_type = &self.rules.entity_types[&ward_entity.type_id];
        if ward_type.class == EntityClass::Building {
            let bounds = crate::construction::footprint(ward_type, ward, ward_entity.facing);
            if self.approach(id, order, ward, bounds) {
                order.movement = None;
            }
            return;
        }
        let at = self.entity(id).expect("guard").pos;
        let far = self
            .geo(ward.body)
            .distance(at, ward)
            .is_none_or(|d| d > GUARD_SLACK);
        // A ward that moved on gets a fresh walk.
        if order.movement.as_ref().is_some_and(|m| {
            self.geo(ward.body)
                .distance(m.goal, ward)
                .is_none_or(|d| d > GUARD_SLACK)
        }) {
            order.movement = None;
        }
        if !far || self.walk(id, order, ward, None, false) == Walk::Arrived {
            order.movement = None;
        }
    }
}
