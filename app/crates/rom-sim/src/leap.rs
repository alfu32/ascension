//! Jetpacks and orbital drop (task 169). A unit tagged `jumper` with a positive `jump_range`
//! stat jumps (order `jump`): it walks until the point lies within that range, then flies
//! there in a straight line over walls, water, cliffs and units, and lands on free land; it
//! jumps again only `jump_cooldown` ticks after it took off. A standing shield field (task 156)
//! blocks the air above its line too, so a jump across one is refused. A ship in orbit drops
//! (order `drop`) every passenger with a positive `orbital_drop` stat onto free land around a
//! visible point of the planet below, within `space.drop_range` of the point under it; they
//! fall for `space.drop_ticks`.
//!
//! A unit in the air, jumping or falling, is a [`Leap`]: it is on the `air` layer for the
//! weapons that hit it (a falling one is reached by `orbit` rows too: planetary defences), it
//! does not fire, its orders wait until it lands and nothing pushes it. Nothing here names a
//! definition: who jumps or drops, how far and how often are tags, stats and modifier rows.

use crate::{
    Command, EntityId, Order, OrderStatus, PlayerId, Pos, SimError, World,
    geometry::{Tile, turn_facing},
    math::facing_of,
    movement::Walk,
};
use rom_rules::{DefId, EntityClass, Rules};
use std::collections::BTreeSet;

/// How far from the point it is sent to a jumper looks for free land to come down on, in
/// milli-tiles.
const LANDING_SNAP: i64 = 1_500;
/// How far around the point they are dropped at falling units spread out, each on its own
/// tile, in milli-tiles.
const DROP_SPREAD: i64 = 6_000;
/// The reach of the dust and flash a drop pod raises where it lands, for the views, in
/// milli-tiles.
const POD_BLAST: i64 = 1_500;
/// The shortest jump, in ticks.
const MIN_JUMP_TICKS: u64 = 10;
/// Steps along a jump's line at which the ground below is looked at, in milli-tiles.
const LINE_STEP: i64 = 100;

/// A unit in the air: jumping from `from` to `to`, or falling from orbit onto `to` (`drop`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Leap {
    /// Where it took off (for a drop, the point it falls onto).
    pub from: Pos,
    /// Where it comes down.
    pub to: Pos,
    /// The tick it took off.
    pub start: u64,
    /// Ticks in the air.
    pub ticks: u64,
    /// Falling from a ship in orbit rather than jumping.
    pub drop: bool,
}

impl Leap {
    /// Share of the flight done at `tick`, in percent.
    pub fn progress_pct(&self, tick: u64) -> u64 {
        (tick.saturating_sub(self.start) * 100 / self.ticks.max(1)).min(100)
    }
}

/// Units in the air and when each jumper may jump again (hashed while not empty, saved).
#[derive(Debug, Clone, Default)]
pub(crate) struct LeapState {
    /// The tag of the units that may learn to jump.
    pub jumper: Option<u32>,
    /// Units in the air, by id.
    pub flights: std::collections::BTreeMap<EntityId, Leap>,
    /// The tick each unit that jumped lately may jump again; dropped once it has passed.
    pub ready: std::collections::BTreeMap<EntityId, u64>,
}

impl LeapState {
    pub fn of(rules: &Rules) -> Self {
        Self {
            jumper: rules
                .tags
                .values()
                .find(|t| t.key == "jumper")
                .map(|t| t.id),
            ..Self::default()
        }
    }
}

impl World {
    /// The farthest a unit of `type_id` owned by `player` jumps, in milli-tiles: its stat
    /// `jump_range` when tagged `jumper`; 0 when it cannot jump (task 169).
    pub fn jump_range_for(&self, player: PlayerId, type_id: DefId) -> i64 {
        let Some(tag) = self.leap.jumper else {
            return 0;
        };
        if !self.rules.has_tag(type_id, tag) || self.is_flyer_type(type_id) {
            return 0;
        }
        let Some(sources) = self.players.get(&player).map(|p| p.active_sources()) else {
            return 0;
        };
        self.rules
            .entity_stat(
                "jump_range",
                type_id,
                0,
                &sources,
                &rom_rules::Context::default(),
            )
            .max(0)
    }

    /// The farthest entity `id` jumps now, in milli-tiles; 0 when it cannot jump.
    pub fn jump_range(&self, id: EntityId) -> i64 {
        self.entity(id)
            .map_or(0, |e| self.jump_range_for(e.owner, e.type_id))
    }

    /// Ticks before entity `id` may jump again; 0 when it may now.
    pub fn jump_cooldown_left(&self, id: EntityId) -> u64 {
        self.leap
            .ready
            .get(&id)
            .map_or(0, |ready| ready.saturating_sub(self.tick))
    }

    /// Whether a unit of `type_id` owned by `player` can be dropped from a ship in orbit: its
    /// stat `orbital_drop` is above 0 (task 169).
    pub fn droppable_for(&self, player: PlayerId, type_id: DefId) -> bool {
        let Some(et) = self.rules.entity_type(type_id) else {
            return false;
        };
        if et.class != EntityClass::Unit || et.movement.is_none() || self.is_flyer_type(type_id) {
            return false;
        }
        let Some(sources) = self.players.get(&player).map(|p| p.active_sources()) else {
            return false;
        };
        self.rules.entity_stat(
            "orbital_drop",
            type_id,
            0,
            &sources,
            &rom_rules::Context::default(),
        ) > 0
    }

    /// The passengers of `ship` that can be dropped from orbit, in id order.
    pub fn droppable_passengers(&self, ship: EntityId) -> Vec<EntityId> {
        self.passengers(ship)
            .into_iter()
            .filter(|id| {
                self.entity(*id)
                    .is_some_and(|e| self.droppable_for(e.owner, e.type_id))
            })
            .collect()
    }

    /// Whether this type moves in the air.
    fn is_flyer_type(&self, type_id: DefId) -> bool {
        self.rules
            .entity_type(type_id)
            .and_then(|et| et.movement.as_ref())
            .is_some_and(|m| {
                self.rules
                    .movement_classes
                    .get(&m.movement_class_id)
                    .is_some_and(|c| c.flies)
            })
    }

    /// The unit's flight, while it is in the air after a jump or a drop.
    pub fn leap(&self, id: EntityId) -> Option<&Leap> {
        self.leap.flights.get(&id)
    }

    /// Every unit in the air, in id order.
    pub fn leaps(&self) -> impl Iterator<Item = (EntityId, &Leap)> {
        self.leap.flights.iter().map(|(id, l)| (*id, l))
    }

    /// Whether a unit is in the air after a jump or a drop.
    pub fn leaping(&self, id: EntityId) -> bool {
        self.leap.flights.contains_key(&id)
    }

    /// Whether a unit falls from orbit after a drop.
    pub(crate) fn dropping(&self, id: EntityId) -> bool {
        self.leap.flights.get(&id).is_some_and(|l| l.drop)
    }

    /// The movement class a unit of `type_id` stands on land with.
    fn land_class(&self, type_id: DefId) -> Option<u32> {
        self.rules
            .entity_type(type_id)?
            .movement
            .as_ref()
            .map(|m| m.movement_class_id)
    }

    /// Whether a unit of `type_id` can stand on the tile at `index` of `body`: land its
    /// movement class walks, with no building or field on it.
    fn free_land(&self, type_id: DefId, body: crate::BodyId, index: usize) -> bool {
        let (Some(class), Some(surface)) = (self.land_class(type_id), self.surface(body)) else {
            return false;
        };
        surface
            .pathing
            .passable(class, surface.geo.tile_at(index), None)
    }

    /// Where a unit of `type_id` comes down when sent to `at`: the point itself on free land,
    /// else the centre of the nearest free land tile within [`LANDING_SNAP`], ties to the lower
    /// tile index. `None` when there is none.
    pub fn jump_landing(&self, type_id: DefId, at: Pos) -> Option<Pos> {
        let surface = self.surface(at.body)?;
        let geo = &surface.geo;
        if !geo.contains(at) {
            return None;
        }
        if geo
            .pos_index(at)
            .is_some_and(|i| self.free_land(type_id, at.body, i))
        {
            return Some(at);
        }
        geo.disc(at, LANDING_SNAP)
            .flatten()
            .filter(|&i| self.free_land(type_id, at.body, i))
            .filter_map(|i| Some((geo.distance_squared(at, geo.index_centre(i))?, i)))
            .min()
            .map(|(_, i)| geo.index_centre(i))
    }

    /// Whether the straight line from `from` to `to` crosses a standing shield field (task
    /// 156): a field tile, or an emitter holding a standing field. Where the line passes a
    /// corner between two tiles, both count.
    pub fn crosses_field(&self, from: Pos, to: Pos) -> bool {
        if self.fields.tiles.is_empty() || from.body != to.body {
            return false;
        }
        let Some(surface) = self.surface(from.body) else {
            return false;
        };
        let geo = &surface.geo;
        let Some((dx, dy)) = geo.vector(from, to) else {
            return false;
        };
        let body = from.body;
        // The tiles of the emitters at both ends of every standing field on this body.
        let mut emitters: BTreeSet<usize> = BTreeSet::new();
        for link in self.fields.links.iter().filter(|l| l.up) {
            for id in [link.from, link.to] {
                let Some(e) = self.entity(id).filter(|e| e.pos.body == body) else {
                    continue;
                };
                let et = &self.rules.entity_types[&e.type_id];
                let bounds = crate::construction::footprint(et, e.pos, e.facing);
                emitters.extend(
                    geo.footprint_tiles(e.pos.face, bounds)
                        .filter_map(|t| geo.index_of(t)),
                );
            }
        }
        let blocked = |index: usize| {
            self.fields.tiles.contains(&(body, index as u32)) || emitters.contains(&index)
        };
        let length = crate::integer_sqrt((dx * dx + dy * dy) as u128) as i64;
        let steps = (length / LINE_STEP).max(1);
        let mut last: Option<Tile> = None;
        for k in 0..=steps {
            let p = geo.offset(from, dx * k / steps, dy * k / steps).0;
            let tile = geo.tile_of(p);
            if geo.index_of(tile).is_some_and(blocked) {
                return true;
            }
            if let Some(prev) = last
                && prev.face == tile.face
                && (prev.x - tile.x).abs() == 1
                && (prev.y - tile.y).abs() == 1
            {
                let sides = [
                    Tile::new(tile.face, prev.x, tile.y),
                    Tile::new(tile.face, tile.x, prev.y),
                ];
                if sides
                    .into_iter()
                    .any(|t| geo.index_of(t).is_some_and(blocked))
                {
                    return true;
                }
            }
            last = Some(tile);
        }
        false
    }

    /// Whether walking unit `id` could walk the straight line to `to` now: no building (its
    /// team's gates open), field or ground it cannot cross lies on it. False for anything that
    /// does not walk.
    pub fn walkable_line(&self, id: EntityId, to: Pos) -> bool {
        let (Some(e), Some(Some(motion))) = (self.entity(id), self.avoid.motion.get(id.0 as usize))
        else {
            return false;
        };
        let Some(surface) = self.surface(e.pos.body) else {
            return false;
        };
        surface
            .pathing
            .line_open(motion.class, Some(self.side_of(e.owner)), e.pos, to, None)
    }

    /// Why entity `id` cannot jump to `at` now, if it cannot: it cannot jump at all, it is
    /// cooling down, there is no free land there, or (within its range) a standing shield
    /// field lies in the way. A point out of range is fine: the unit walks closer first.
    pub fn check_jump(&self, id: EntityId, at: Pos) -> Result<Pos, SimError> {
        let e = self.entity(id).ok_or(SimError::UnknownExecutor(id))?;
        let range = self.jump_range(id);
        if range <= 0 {
            return Err(SimError::CannotJump);
        }
        if self.jump_cooldown_left(id) > 0 || self.leaping(id) {
            return Err(SimError::JumpCooling);
        }
        let landing = self
            .jump_landing(e.type_id, at)
            .ok_or(SimError::NoLandingGround)?;
        if self.geo(e.pos.body).within(e.pos, landing, range) && self.crosses_field(e.pos, landing)
        {
            return Err(SimError::FieldInTheWay);
        }
        Ok(landing)
    }

    /// Where `ship` would drop its passengers sent to `at`, or why it cannot: it is not in
    /// orbit, none of its passengers can be dropped, or the point is not visible free land of
    /// the planet below.
    pub fn check_drop(&self, ship: EntityId, at: Pos) -> Result<(), SimError> {
        let e = self.entity(ship).ok_or(SimError::UnknownExecutor(ship))?;
        let orbit = self.orbit(ship).ok_or(SimError::NotInOrbit)?;
        let droppable = self.droppable_passengers(ship);
        if droppable.is_empty() {
            return Err(SimError::CannotDrop);
        }
        let slot = self.slot_of(orbit.body).ok_or(SimError::DropZone)?;
        let geo = &self.surfaces[slot].geo;
        if at.body != orbit.body || !geo.contains(at) {
            return Err(SimError::DropZone);
        }
        let index = geo.pos_index(at).ok_or(SimError::DropZone)?;
        if !self.visible_index(e.owner, slot, index) {
            return Err(SimError::DropZone);
        }
        let types: BTreeSet<DefId> = droppable
            .iter()
            .filter_map(|id| self.entity(*id).map(|p| p.type_id))
            .collect();
        let land = geo
            .disc(at, DROP_SPREAD)
            .flatten()
            .any(|i| types.iter().any(|t| self.free_land(*t, at.body, i)));
        if !land {
            return Err(SimError::DropZone);
        }
        Ok(())
    }

    /// Checks `jump` and `drop` orders at submission.
    pub(crate) fn validate_leap(&self, command: &Command) -> Result<(), SimError> {
        match command.kind.as_str() {
            "jump" => {
                let at = command.target_pos.ok_or(SimError::WrongTargetShape)?;
                if command.queue {
                    // Queued behind other orders it may wait for its cooldown and its place.
                    let e = self.entity(command.executor).expect("checked executor");
                    if self.jump_range(e.id) <= 0 {
                        return Err(SimError::CannotJump);
                    }
                    self.jump_landing(e.type_id, at)
                        .ok_or(SimError::NoLandingGround)?;
                } else {
                    self.check_jump(command.executor, at)?;
                }
            }
            "drop" => {
                let at = command.target_pos.ok_or(SimError::WrongTargetShape)?;
                self.check_drop(command.executor, at)?;
            }
            _ => {}
        }
        Ok(())
    }

    /// Advances one `jump` or `drop` order.
    pub(crate) fn advance_leap_order(&mut self, id: EntityId, order: &mut Order) {
        order.progress_ticks += 1;
        if order.kind == "drop" {
            self.advance_drop(id, order);
        } else {
            self.advance_jump(id, order);
        }
    }

    fn advance_jump(&mut self, id: EntityId, order: &mut Order) {
        let Some(target) = order.target_pos else {
            order.status = OrderStatus::Failed("unsupported".into());
            return;
        };
        let range = self.jump_range(id);
        if range <= 0 {
            order.status = OrderStatus::Failed("cannot_jump".into());
            return;
        }
        let type_id = self.entity(id).expect("jumper").type_id;
        let Some(landing) = self.jump_landing(type_id, target) else {
            order.status = OrderStatus::Failed("no_landing".into());
            return;
        };
        let pos = self.entity(id).expect("jumper").pos;
        let geo = self.geo(pos.body);
        if !geo.within(pos, landing, range) {
            // Out of range it walks closer first.
            match self.walk(id, order, landing, None, false) {
                Walk::Failed => return,
                Walk::Arrived => {
                    order.movement = None;
                    let pos = self.entity(id).expect("jumper").pos;
                    if !self.geo(pos.body).within(pos, landing, range) {
                        order.status = OrderStatus::Failed("out_of_range".into());
                    }
                }
                Walk::Underway => {}
            }
            return;
        }
        order.movement = None;
        if self.jump_cooldown_left(id) > 0 {
            return;
        }
        // Units sent to one point this tick come down on distinct tiles near it.
        let slot = self.slot(pos.body);
        let pathing = &mut self.surfaces[slot].pathing;
        let index = pathing.join_spread(landing);
        let (dx, dy) = pathing.spread_offset(index);
        let spread = self.geo(pos.body).offset(landing, dx, dy).0;
        let landing = [spread, landing].into_iter().find(|p| {
            self.geo(pos.body).within(pos, *p, range)
                && self
                    .geo(p.body)
                    .pos_index(*p)
                    .is_some_and(|i| self.free_land(type_id, p.body, i))
                && !self.crosses_field(pos, *p)
        });
        let Some(landing) = landing else {
            order.status = OrderStatus::Failed("field".into());
            return;
        };
        let distance = self.geo(pos.body).distance(pos, landing).unwrap_or(0);
        let speed = self.rules.constant("air.jump_speed").unwrap_or(4000).max(1) as u64;
        let ticks = (distance * self.tick_rate as u64 / speed).max(MIN_JUMP_TICKS);
        let cooldown = self.effective_stat(id, "jump_cooldown", 0).max(0) as u64;
        self.leap
            .ready
            .insert(id, self.tick.saturating_add(cooldown.max(ticks)));
        self.take_off(id, pos, landing, ticks, false);
        order.status = OrderStatus::Done;
    }

    fn advance_drop(&mut self, ship: EntityId, order: &mut Order) {
        let Some(target) = order.target_pos else {
            order.status = OrderStatus::Failed("unsupported".into());
            return;
        };
        let Some(below) = self.orbit(ship).map(|o| o.pos) else {
            order.status = OrderStatus::Failed("not_in_orbit".into());
            return;
        };
        let droppable = self.droppable_passengers(ship);
        if droppable.is_empty() || target.body != below.body {
            order.status = OrderStatus::Failed("cannot_drop".into());
            return;
        }
        let range = self.rules.constant("space.drop_range").unwrap_or(0).max(0);
        // It moves within its orbit until the point lies within the drop range of the point
        // below it.
        if !self.geo(below.body).within(below, target, range) {
            self.orbit_move(ship, target);
            return;
        }
        let ticks = self.rules.constant("space.drop_ticks").unwrap_or(1).max(1) as u64;
        let mut used: BTreeSet<usize> = BTreeSet::new();
        for unit in droppable {
            let type_id = self.entity(unit).expect("passenger").type_id;
            let surface = self.surf(target.body);
            let geo = &surface.geo;
            let tile = geo
                .disc(target, DROP_SPREAD)
                .flatten()
                .filter(|i| !used.contains(i))
                .filter(|&i| geo.within(below, geo.index_centre(i), range))
                .filter(|&i| self.free_land(type_id, target.body, i))
                .filter_map(|i| Some((geo.distance_squared(target, geo.index_centre(i))?, i)))
                .min()
                .map(|(_, i)| i);
            let Some(tile) = tile else {
                continue;
            };
            used.insert(tile);
            let at = geo.index_centre(tile);
            self.disembark(unit, at);
            self.take_off(unit, at, at, ticks, true);
        }
        if used.is_empty() {
            order.status = OrderStatus::Failed("no_landing".into());
        } else {
            order.status = OrderStatus::Done;
        }
    }

    /// Puts unit `id` in the air from `from` to `to` for `ticks`.
    fn take_off(&mut self, id: EntityId, from: Pos, to: Pos, ticks: u64, drop: bool) {
        self.set_work_site(id, None);
        self.avoid.set(id, None, from);
        self.leap.flights.insert(
            id,
            Leap {
                from,
                to,
                start: self.tick,
                ticks: ticks.max(1),
                drop,
            },
        );
        let heading = (!drop)
            .then(|| self.geo(from.body).vector(from, to))
            .flatten();
        if let Some((dx, dy)) = heading
            && let Some(e) = self.entities[id.0 as usize].as_mut()
        {
            e.facing = facing_of(dx, dy);
        }
    }

    /// Moves a unit in the air one tick along its flight; it lands at the end. True when it
    /// landed this tick.
    pub(crate) fn fly_leap(&mut self, id: EntityId) -> bool {
        let Some(leap) = self.leap.flights.get(&id).copied() else {
            return false;
        };
        let done = (self.tick + 1).saturating_sub(leap.start);
        if done >= leap.ticks {
            self.leap.flights.remove(&id);
            // Something built under it meanwhile: it comes down on the nearest free land.
            let type_id = self.entity(id).expect("leaper").type_id;
            let at = self.jump_landing(type_id, leap.to).unwrap_or(leap.to);
            let e = self.entities[id.0 as usize].as_mut().expect("leaper");
            e.pos = at;
            for order in &mut e.orders {
                order.movement = None;
            }
            let motion = self.motion_of(id);
            self.avoid.set(id, motion, at);
            self.economy.dirty.insert(id);
            // A drop pod's impact, for the views (a report, not hashed, not saved).
            if leap.drop {
                let (owner, source_type) = self
                    .entity(id)
                    .map(|e| (e.owner, e.type_id))
                    .expect("leaper");
                self.blasts.push(crate::Blast {
                    tick: self.tick,
                    pos: at,
                    radius: POD_BLAST,
                    owner,
                    source_type,
                    drop: true,
                });
            }
            return true;
        }
        // Nothing pushes it in the air, even after a change of type (militia called up).
        if self
            .avoid
            .motion
            .get(id.0 as usize)
            .is_some_and(Option::is_some)
        {
            let pos = self.entity(id).expect("leaper").pos;
            self.avoid.set(id, None, pos);
        }
        if leap.drop {
            return false;
        }
        let geo = &self.surfaces[self.slot(leap.from.body)].geo;
        let Some((dx, dy)) = geo.vector(leap.from, leap.to) else {
            return false;
        };
        let n = leap.ticks as i64;
        let k = done as i64;
        let (at, turns) = geo.offset(leap.from, dx * k / n, dy * k / n);
        let facing = turn_facing(facing_of(dx, dy), turns);
        let e = self.entities[id.0 as usize].as_mut().expect("leaper");
        e.pos = at;
        e.facing = facing;
        false
    }

    /// Forgets the cooldowns that have run out.
    pub(crate) fn forget_jumps(&mut self) {
        if !self.leap.ready.is_empty() {
            let tick = self.tick;
            self.leap.ready.retain(|_, ready| *ready > tick);
        }
    }
}
