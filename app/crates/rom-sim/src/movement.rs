//! Walking: the straight-line fast path, path requests, delivery, and waypoint following.

use crate::{
    EntityId, Order, OrderStatus, Pos, World,
    math::{facing, mul_div},
    order::MoveState,
    pathing::{Found, TileRect, ceil_distance, centre, tile_of},
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
        let pct = self.pathing.speed_pct(motion.class, pos);
        if pct == 0 {
            order.status = OrderStatus::Failed("no_path".into());
            return Walk::Failed;
        }
        if order.movement.is_none() {
            let own = if spread {
                let index = self.pathing.join_spread(goal);
                let (dx, dy) = self.pathing.spread_offset(index);
                let candidate = Pos {
                    x: (i64::from(goal.x) + dx).clamp(i64::from(i32::MIN), i64::from(i32::MAX))
                        as i32,
                    y: (i64::from(goal.y) + dy).clamp(i64::from(i32::MIN), i64::from(i32::MAX))
                        as i32,
                };
                let (tx, ty) = tile_of(candidate);
                if self.pathing.passable(motion.class, tx, ty, None) {
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
            if self.pathing.line_clear(motion.class, pos, own, exempt) {
                state.distance = ceil_distance(pos, own);
            } else {
                // Group members search for the shared goal; each ends at its own point.
                state.request = Some(
                    self.pathing
                        .request((id, order.id), motion.class, pos, goal),
                );
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
        let denominator = i128::from(self.pathing.tick_rate) * 100;
        let entity = self.entities[id.0 as usize].as_mut().expect("walker");
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
            // Interpolate cumulative travel from the segment start: direction rounding
            // cannot accumulate drift or strand slow diagonal movement at (0, 0).
            let next = if traveled == state.distance {
                target
            } else {
                let coordinate = |from: i32, to: i32| {
                    (i64::from(from)
                        + mul_div(
                            i64::from(to) - i64::from(from),
                            traveled as i64,
                            state.distance as i64,
                        )) as i32
                };
                Pos {
                    x: coordinate(state.origin.x, target.x),
                    y: coordinate(state.origin.y, target.y),
                }
            };
            // Rounding can tilt a short step across a corner the planned segment misses;
            // such a step is fine while the segment itself is still clear.
            let (nx, ny) = tile_of(next);
            let clear = self
                .pathing
                .line_clear(motion.class, entity.pos, next, exempt)
                || (self.pathing.passable(motion.class, nx, ny, exempt)
                    && self
                        .pathing
                        .line_clear(motion.class, state.origin, target, exempt));
            if !clear {
                if state.repaths >= MAX_REPATHS {
                    order.status = OrderStatus::Failed("no_path".into());
                    return Walk::Failed;
                }
                let repaths = state.repaths + 1;
                let own = state.goal;
                let request = self
                    .pathing
                    .request((id, order.id), motion.class, entity.pos, own);
                self.pathing.stats.repaths += 1;
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
            // Facing is constant along a segment; set it when the segment starts.
            if state.traveled == 0 && state.origin != target {
                entity.facing = facing(state.origin, target);
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
            state.distance = ceil_distance(next, following);
            if step == 0 {
                break;
            }
        }
        entity.move_remainder = remainder;
        Walk::Underway
    }

    /// Walks toward the closest point of a footprint; true once within build reach.
    pub(crate) fn approach(&mut self, id: EntityId, order: &mut Order, bounds: [i64; 4]) -> bool {
        let reach = self.rules.constant("build.reach").expect("validated reach") as u128;
        let closest = |pos: Pos| Pos {
            x: i64::from(pos.x).clamp(bounds[0], bounds[2]) as i32,
            y: i64::from(pos.y).clamp(bounds[1], bounds[3]) as i32,
        };
        let pos = self.entity(id).expect("walker").pos;
        if pos.distance_squared_wide(closest(pos)) <= reach * reach {
            return true;
        }
        let walk = self.walk(
            id,
            order,
            closest(pos),
            Some(TileRect::covering(bounds)),
            false,
        );
        if walk == Walk::Failed {
            return false;
        }
        let pos = self.entity(id).expect("walker").pos;
        let near = pos.distance_squared_wide(closest(pos)) <= reach * reach;
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

    /// Runs queued searches in order within the tick's node budget, delivering finished paths.
    pub(crate) fn run_path_searches(&mut self) {
        let mut budget = self.pathing.budget;
        while budget > 0 {
            if self.pathing.search.is_none() {
                let Some(front) = self.pathing.queue.front() else {
                    break;
                };
                let id = front.id;
                if !front.waiters.iter().any(|w| self.waiter_valid(*w, id)) {
                    self.pathing.queue.pop_front();
                    continue;
                }
                self.pathing.begin();
            }
            let (used, found) = self.pathing.advance(budget);
            budget -= used;
            let Some(found) = found else {
                break;
            };
            let request = self.pathing.queue.pop_front().expect("finished request");
            self.deliver(&request, &found);
        }
    }

    fn deliver(&mut self, request: &crate::pathing::PathRequest, found: &Found) {
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
            let waypoints = if end == start && !found.reached {
                None
            } else {
                let mut points = Vec::new();
                if tile_of(pos) != start {
                    points.push(centre(start));
                }
                points.extend(found.tiles[1..].iter().map(|t| centre(*t)));
                let last = points.last().copied().unwrap_or(pos);
                if self.pathing.line_clear(class, last, own, None) {
                    points.push(own);
                } else if points.is_empty() {
                    points.push(centre(end));
                }
                Some(self.pathing.smooth(class, pos, &points))
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
            state.distance = ceil_distance(pos, waypoints.first().copied().unwrap_or(state.goal));
            state.waypoints = waypoints;
        }
    }
}
