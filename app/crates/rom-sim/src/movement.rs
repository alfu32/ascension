//! Walking: the straight-line fast path, path requests, delivery, and waypoint following.

use crate::{
    BodyId, EntityId, Order, OrderStatus, Pos, World,
    geometry::turn_facing,
    math::{facing_of, mul_div},
    order::MoveState,
    pathing::{Found, TileRect},
};

/// Consecutive repaths without reaching a waypoint before the walk gives up.
const MAX_REPATHS: u32 = 8;

/// Outcome of one tick of walking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Walk {
    /// Moved, or is waiting for a path.
    Underway,
    /// Standing on the last waypoint.
    Arrived,
    /// The order's status has been set to a failure.
    Failed,
}

impl World {
    /// Walks one tick toward `goal`. `exempt` lets the walk enter its target's footprint;
    /// `spread` gives units sent to the same point this tick distinct end points.
    pub(crate) fn walk(
        &mut self,
        id: EntityId,
        order: &mut Order,
        goal: Pos,
        exempt: Option<TileRect>,
        spread: bool,
    ) -> Walk {
        let Some(motion) = self.avoid.motion.get(id.0 as usize).copied().flatten() else {
            order.status = OrderStatus::Failed("unsupported".into());
            return Walk::Failed;
        };
        let pos = self.entity(id).expect("walker").pos;
        let slot = self.slot(pos.body);
        let surface = &mut self.surfaces[slot];
        let (geo, pathing) = (&surface.geo, &mut surface.pathing);
        let pct = pathing.speed_pct(motion.class, pos);
        // No path leads to another body.
        if pct == 0 || goal.body != pos.body {
            order.status = OrderStatus::Failed("no_path".into());
            return Walk::Failed;
        }
        if order.movement.is_none() {
            let own = if spread {
                let index = pathing.join_spread(goal);
                let (dx, dy) = pathing.spread_offset(index);
                // Off-surface goals keep their point; the offset never leaves a flat map's
                // coordinate range and crosses edges on a sphere.
                let candidate = if geo.contains(goal) || geo.is_flat() {
                    geo.offset(goal, dx, dy).0
                } else {
                    goal
                };
                if pathing.passable(motion.class, geo.tile_of(candidate), None) {
                    candidate
                } else {
                    goal
                }
            } else {
                goal
            };
            let mut state = MoveState {
                origin: pos,
                distance: 0,
                traveled: 0,
                goal: own,
                waypoints: Vec::new(),
                index: 0,
                request: None,
                repaths: 0,
            };
            if pathing.line_clear(motion.class, pos, own, exempt) {
                state.distance = geo
                    .ceil_distance(pos, own)
                    .expect("clear lines are in reach");
            } else {
                // Group members search for the shared goal; each ends at its own point.
                state.request = Some(pathing.request((id, order.id), motion.class, pos, goal));
            }
            order.movement = Some(state);
        }
        let state = order.movement.as_mut().expect("walk state");
        if state.request.is_some() {
            return Walk::Underway;
        }
        if state.target().is_none() {
            return Walk::Arrived;
        }
        let speed = self.effective_stat(id, "speed", motion.speed).max(0);
        let denominator = i128::from(self.tick_rate) * 100;
        let entity = self.entities[id.0 as usize].as_mut().expect("walker");
        let surface = &mut self.surfaces[slot];
        let (geo, pathing) = (&surface.geo, &mut surface.pathing);
        let budget = i128::from(speed) * i128::from(pct) + entity.move_remainder;
        // Wide division is slow; take the 64-bit path whenever the budget fits.
        let (mut step, remainder) = match (i64::try_from(budget), i64::try_from(denominator)) {
            (Ok(b), Ok(d)) => (i128::from(b / d), i128::from(b % d)),
            _ => (budget / denominator, budget % denominator),
        };
        loop {
            let target = state.target().expect("walk underway");
            let traveled = (i128::from(state.traveled) + step).min(i128::from(state.distance));
            let used = traveled - i128::from(state.traveled);
            let traveled = traveled as u64;
            // The segment in its origin's frame; None if a push carried it out of reach.
            let segment = geo.vector(state.origin, target);
            // Interpolate cumulative travel from the segment start: direction rounding
            // cannot accumulate drift or strand slow diagonal movement at (0, 0).
            let next = match segment {
                _ if traveled == state.distance => Some(target),
                None => None,
                Some((dx, dy)) => {
                    let part = |d: i64| mul_div(d, traveled as i64, state.distance as i64);
                    Some(geo.offset(state.origin, part(dx), part(dy)).0)
                }
            };
            // Rounding can tilt a short step across a corner the planned segment misses;
            // such a step is fine while the segment itself is still clear.
            let clear = next.is_some_and(|next| {
                pathing.line_clear(motion.class, entity.pos, next, exempt)
                    || (pathing.passable(motion.class, geo.tile_of(next), exempt)
                        && pathing.line_clear(motion.class, state.origin, target, exempt))
            });
            if !clear {
                if state.repaths >= MAX_REPATHS {
                    order.status = OrderStatus::Failed("no_path".into());
                    return Walk::Failed;
                }
                let repaths = state.repaths + 1;
                let own = state.goal;
                let request = pathing.request((id, order.id), motion.class, entity.pos, own);
                pathing.stats.repaths += 1;
                *state = MoveState {
                    origin: entity.pos,
                    distance: 0,
                    traveled: 0,
                    goal: own,
                    waypoints: Vec::new(),
                    index: 0,
                    request: Some(request),
                    repaths,
                };
                return Walk::Underway;
            }
            let next = next.expect("clear steps exist");
            // Facing is constant along a segment in its origin's frame; set it when the
            // segment starts, and turn it with the frame whenever a face edge is crossed.
            if state.traveled == 0
                && state.origin != target
                && let Some((dx, dy)) = segment
            {
                entity.facing = facing_of(dx, dy);
            }
            if next.face != entity.pos.face {
                entity.facing = turn_facing(entity.facing, geo.turns(entity.pos.face, next.face));
            }
            state.traveled = traveled;
            entity.pos = next;
            if traveled < state.distance {
                break;
            }
            state.index += 1;
            state.repaths = 0;
            let Some(following) = state.target() else {
                entity.move_remainder = remainder;
                return Walk::Arrived;
            };
            step -= used;
            state.origin = next;
            state.traveled = 0;
            // Smoothed waypoints are always within reach; a missing one ends the segment
            // at once, and the next tick finds the line blocked and searches again.
            state.distance = geo.ceil_distance(next, following).unwrap_or(0);
            if step == 0 {
                break;
            }
        }
        entity.move_remainder = remainder;
        Walk::Underway
    }

    /// Walks toward the closest point of a footprint; true once within build reach.
    /// `bounds` lie on the face and body of `at`.
    pub(crate) fn approach(
        &mut self,
        id: EntityId,
        order: &mut Order,
        at: Pos,
        bounds: [i64; 4],
    ) -> bool {
        let (face, body): (u8, BodyId) = (at.face, at.body);
        let reach = self.rules.constant("build.reach").expect("validated reach") as u128;
        // The footprint's nearest point, or its centre while it is out of reach.
        let closest = |world: &World, pos: Pos| {
            world
                .geo(body)
                .closest_in_bounds(face, bounds, pos)
                .unwrap_or_else(|| {
                    let centre = Pos {
                        x: ((bounds[0] + bounds[2]) / 2) as i32,
                        y: ((bounds[1] + bounds[3]) / 2) as i32,
                        face,
                        body,
                    };
                    (centre, u128::MAX)
                })
        };
        let pos = self.entity(id).expect("walker").pos;
        let (goal, squared) = closest(self, pos);
        if squared <= reach * reach {
            return true;
        }
        let walk = self.walk(
            id,
            order,
            goal,
            Some(TileRect::covering(face, bounds)),
            false,
        );
        if walk == Walk::Failed {
            return false;
        }
        let pos = self.entity(id).expect("walker").pos;
        let near = closest(self, pos).1 <= reach * reach;
        if !near && walk == Walk::Arrived {
            // Start a fresh walk next tick; it fails if no step gets closer.
            order.movement = None;
        }
        near
    }

    fn waiter_valid(&self, waiter: (EntityId, crate::OrderId), request: u64) -> bool {
        self.entity(waiter.0)
            .and_then(|e| e.orders.front())
            .is_some_and(|o| {
                o.id == waiter.1
                    && o.movement
                        .as_ref()
                        .is_some_and(|m| m.request == Some(request))
            })
    }

    /// Runs each surface's queued searches in order within that surface's node budget per
    /// tick, delivering finished paths; a surface with nothing queued costs nothing.
    pub(crate) fn run_path_searches(&mut self) {
        for slot in 0..self.surfaces.len() {
            if self.surfaces[slot].pathing.search.is_none()
                && self.surfaces[slot].pathing.queue.is_empty()
            {
                continue;
            }
            self.run_surface_searches(slot);
        }
    }

    fn run_surface_searches(&mut self, slot: usize) {
        let mut budget = self.surfaces[slot].pathing.budget;
        while budget > 0 {
            if self.surfaces[slot].pathing.search.is_none() {
                let Some(front) = self.surfaces[slot].pathing.queue.front() else {
                    break;
                };
                let id = front.id;
                if !front.waiters.iter().any(|w| self.waiter_valid(*w, id)) {
                    self.surfaces[slot].pathing.queue.pop_front();
                    continue;
                }
                self.surfaces[slot].pathing.begin();
            }
            let (used, found) = self.surfaces[slot].pathing.advance(budget);
            budget -= used;
            let Some(found) = found else {
                break;
            };
            let request = self.surfaces[slot]
                .pathing
                .queue
                .pop_front()
                .expect("finished request");
            self.deliver(slot, &request, &found);
        }
    }

    fn deliver(&mut self, slot: usize, request: &crate::pathing::PathRequest, found: &Found) {
        let start = request.start;
        let end = *found.tiles.last().expect("path has a start");
        for waiter in &request.waiters {
            if !self.waiter_valid(*waiter, request.id) {
                continue;
            }
            let entity = self.entity(waiter.0).expect("valid waiter");
            let pos = entity.pos;
            let class = self.avoid.motion[waiter.0.0 as usize]
                .expect("walker motion")
                .class;
            let own = entity.orders[0]
                .movement
                .as_ref()
                .expect("waiting walk")
                .goal;
            let surface = &self.surfaces[slot];
            let (geo, pathing) = (&surface.geo, &surface.pathing);
            let waypoints = if end == start && !found.reached {
                None
            } else {
                let mut points = Vec::new();
                if geo.tile_of(pos) != start {
                    points.push(geo.tile_centre(start));
                }
                points.extend(found.tiles[1..].iter().map(|t| geo.tile_centre(*t)));
                let last = points.last().copied().unwrap_or(pos);
                if pathing.line_clear(class, last, own, None) {
                    points.push(own);
                } else if points.is_empty() {
                    points.push(geo.tile_centre(end));
                }
                Some(pathing.smooth(class, pos, &points))
            };
            let entity = self.entities[waiter.0.0 as usize]
                .as_mut()
                .expect("valid waiter");
            let Some(waypoints) = waypoints else {
                // No step gets closer: the order fails now, as a blocked straight line did.
                let mut order = entity.orders.pop_front().expect("waiting order");
                order.status = OrderStatus::Failed("no_path".into());
                self.finish_order(&order);
                self.economy.dirty.insert(waiter.0);
                continue;
            };
            let state = entity.orders[0].movement.as_mut().expect("waiting walk");
            let mut waypoints = waypoints;
            state.goal = waypoints.pop().expect("a path has an end point");
            state.request = None;
            state.origin = pos;
            state.traveled = 0;
            state.index = 0;
            state.distance = self.surfaces[slot]
                .geo
                .ceil_distance(pos, waypoints.first().copied().unwrap_or(state.goal))
                .unwrap_or(0);
            state.waypoints = waypoints;
        }
    }
}
