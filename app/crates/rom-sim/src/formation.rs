//! Formations (task 143): where each member of a group ends up when one player's orders of one
//! tick name a formation and share a goal. The ranks face the direction of travel, close fighters
//! in front and ranged and support members behind when the formation's `ranged_behind` says so.
//! Ground units, ships in orbit and ships in a system's space are laid out alike, each on its
//! own surface; the slot becomes the order's point, so it is saved and hashed with the order.

use crate::{BodyId, EntityId, OrderId, PlayerId, Pos, World, math::integer_sqrt, space::sine};
use rom_rules::{BodyKind, ClaimMode, DefId, FormationShape, Rules};
use std::collections::BTreeMap;

/// Least gap between the collision circles of neighbours in a formation, in milli-tiles.
const MIN_GAP: i64 = 50;
/// How close a ship keeps to its place beside a body it holds in formation, in milli-tiles.
pub(crate) const HOLD_SLACK: i64 = 250;
/// Largest group laid out as one ring by the circle formation.
const ONE_RING: usize = 12;

/// The orders of one formation: player, formation, tick of issue, what the order does and its
/// target (an entity or a point).
type GroupKey = (
    PlayerId,
    DefId,
    u64,
    Job,
    Option<EntityId>,
    Option<(BodyId, u8, i32, i32)>,
);

/// What the orders of a formation do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Job {
    /// `move`, `attack_move` or `patrol` to a point.
    Walk,
    /// `orbit`: hold a body.
    Hold,
    /// `travel` to another system.
    Travel,
}

/// `ceil(sqrt(n))`.
fn ceil_sqrt(n: i64) -> i64 {
    let n = n.max(0);
    let root = integer_sqrt(n as u128) as i64;
    if root * root < n { root + 1 } else { root }
}

/// Ranks of `count` places, `width` to a rank, the last one centred.
fn block(count: usize, width: i64, spacing: i64) -> Vec<Vec<(i64, i64)>> {
    let width = width.clamp(1, count.max(1) as i64) as usize;
    let mut ranks = Vec::new();
    let mut left = count;
    let mut rank = 0_i64;
    while left > 0 {
        let k = left.min(width);
        ranks.push(
            (0..k as i64)
                .map(|file| ((2 * file - (k as i64 - 1)) * spacing / 2, rank * spacing))
                .collect(),
        );
        left -= k;
        rank += 1;
    }
    ranks
}

/// Rings round the centre, outermost first; the first place of each ring is straight ahead.
fn rings(count: usize, spacing: i64) -> Vec<Vec<(i64, i64)>> {
    let ring = |m: usize, radius: i64| -> Vec<(i64, i64)> {
        (0..m as i64)
            .map(|j| {
                let angle = j * 360_000 / m as i64;
                let (sin, cos) = (sine(angle), sine(angle + 90_000));
                (radius * sin / 1_000_000, -radius * cos / 1_000_000)
            })
            .collect()
    };
    if count <= 1 {
        return vec![vec![(0, 0); count]];
    }
    if count <= ONE_RING {
        // One ring whose circumference gives every member its spacing.
        let radius = (count as i64 * spacing * 1000 / 6283).max(spacing);
        return vec![ring(count, radius)];
    }
    // Ring k holds 2πk members at k spacings; the outer rings fill first.
    let capacity = |k: i64| (6283 * k / 1000) as usize;
    let mut total = 0;
    let mut outer = 0_i64;
    while total < count {
        outer += 1;
        total += capacity(outer);
    }
    let mut left = count;
    let mut out = Vec::new();
    for k in (1..=outer).rev() {
        let m = left.min(capacity(k));
        if m == 0 {
            break;
        }
        out.push(ring(m, k * spacing));
        left -= m;
    }
    out
}

/// The places of a formation of `count` members about its centre, rank by rank from the front
/// (ring by ring from the outside for a circle): each `(across, back)` in milli-tiles, `across`
/// to the right of the direction of travel and `back` behind the centre.
pub fn formation_ranks(
    shape: FormationShape,
    count: usize,
    rank_ratio_pct: i64,
    spacing: i64,
) -> Vec<Vec<(i64, i64)>> {
    if count == 0 {
        return Vec::new();
    }
    let n = count as i64;
    let block_width = ceil_sqrt(n * rank_ratio_pct.max(1) / 100);
    let mut ranks = match shape {
        FormationShape::Circle | FormationShape::Sphere => return rings(count, spacing),
        FormationShape::Line => block(count, n, spacing),
        FormationShape::DoubleLine => block(count, (n + 1) / 2, spacing),
        FormationShape::Wedge => {
            // One at the point, then three, five, ... a rank.
            let mut ranks = Vec::new();
            let mut left = count;
            let mut rank = 0_i64;
            while left > 0 {
                let k = left.min(2 * rank as usize + 1);
                ranks.push(
                    (0..k as i64)
                        .map(|file| ((2 * file - (k as i64 - 1)) * spacing / 2, rank * spacing))
                        .collect(),
                );
                left -= k;
                rank += 1;
            }
            ranks
        }
        _ => block(count, block_width, spacing),
    };
    // Flanks bend forward to envelop, back when refused; an echelon recedes to one side.
    for rank in &mut ranks {
        for (across, back) in rank.iter_mut() {
            match shape {
                FormationShape::Envelop => *back -= across.abs() / 2,
                FormationShape::Refused => *back += across.abs() / 2,
                FormationShape::EchelonRight => *back += *across / 2,
                FormationShape::EchelonLeft => *back -= *across / 2,
                _ => {}
            }
        }
    }
    // The centre of the formation stands on the goal.
    let (sum, all) = ranks
        .iter()
        .flatten()
        .fold((0_i64, 0_i64), |(s, c), (_, back)| (s + back, c + 1));
    let middle = if all > 0 { sum / all } else { 0 };
    for rank in &mut ranks {
        for place in rank.iter_mut() {
            place.1 -= middle;
        }
    }
    ranks
}

/// A local vector of `length` milli-tiles along `(x, y)`, or None for a zero vector.
fn scaled(x: i64, y: i64, length: i64) -> Option<(i64, i64)> {
    let norm = integer_sqrt((i128::from(x).pow(2) + i128::from(y).pow(2)) as u128) as i64;
    (norm > 0).then(|| (x * length / norm, y * length / norm))
}

/// The unit vector (thousandths) of a facing in degrees.
fn facing_vector(facing: u16) -> (i64, i64) {
    let angle = i64::from(facing) * 1000;
    (sine(angle + 90_000) / 1000, sine(angle) / 1000)
}

/// How far forward a member stands in a formation: 0 close fighters, 1 ranged (by reach),
/// 2 unarmed support.
fn rank_key(rules: &Rules, type_id: DefId) -> (u8, i64) {
    let et = &rules.entity_types[&type_id];
    let reach = et.weapons.iter().map(|w| w.range_max).max();
    match reach {
        None => (2, 0),
        Some(0) => (0, 0),
        Some(r) => (1, r),
    }
}

/// One member of a group being formed.
struct Member {
    id: EntityId,
    order: OrderId,
    type_id: DefId,
    /// Where it stands on the goal's surface, relative to the goal, if it can be measured.
    from_goal: Option<(i64, i64)>,
    /// Whether it is a ship in orbit over that surface (it needs no ground to stand on).
    in_orbit: bool,
}

impl World {
    /// Where a unit stands on `body`: its point below in orbit, else its own position.
    fn place_on(&self, id: EntityId, body: BodyId) -> Option<(Pos, bool)> {
        if let Some(orbit) = self.orbit(id).filter(|o| o.body == body) {
            return Some((orbit.pos, true));
        }
        let e = self.entity(id)?;
        (e.pos.body == body).then_some((e.pos, false))
    }

    /// A unit's facing as its formation would take it when it has no direction of travel.
    fn standing_facing(&self, id: EntityId) -> u16 {
        self.orbit(id)
            .map(|o| o.facing)
            .or_else(|| self.entity(id).map(|e| e.facing))
            .unwrap_or(0)
    }

    /// Lays out the formations among the orders applied this tick: the orders of one player,
    /// formation, tick of issue and goal make one group, and each member's point becomes its
    /// place in it. A place it cannot stand on keeps the goal.
    pub(crate) fn form_up(&mut self, applied: &[(EntityId, OrderId)]) {
        let mut groups: BTreeMap<GroupKey, Vec<(EntityId, OrderId)>> = BTreeMap::new();
        for (executor, id) in applied {
            let Some(order) = self
                .entity(*executor)
                .and_then(|e| e.orders.iter().find(|o| o.id == *id))
            else {
                continue;
            };
            let Some(formation) = order
                .subject
                .filter(|f| self.rules.formations.contains_key(f))
            else {
                continue;
            };
            let job = match order.kind.as_str() {
                "move" | "attack_move" | "patrol"
                    if order.target_entity.is_none() && order.target_pos.is_some() =>
                {
                    Job::Walk
                }
                "orbit" => Job::Hold,
                "travel" if order.target_pos.is_some() => Job::Travel,
                _ => continue,
            };
            let point = order.target_pos.map(|p| (p.body, p.face, p.x, p.y));
            groups
                .entry((
                    order.player,
                    formation,
                    order.issued_tick,
                    job,
                    order.target_entity,
                    point,
                ))
                .or_default()
                .push((*executor, *id));
        }
        for ((_, formation, _, job, target, point), members) in groups {
            if members.len() < 2 {
                continue;
            }
            let goal = point.map(|(body, face, x, y)| Pos::on(body, face, x, y));
            let places = match job {
                Job::Walk => goal.map(|goal| {
                    self.places_about(formation, &members, goal, None, (true, false), |_| true)
                }),
                Job::Hold => self.hold_places(formation, &members, target, goal),
                Job::Travel => goal.and_then(|goal| self.travel_places(formation, &members, goal)),
            };
            for (executor, id, place) in places.unwrap_or_default() {
                if let Some(order) = self.entities[executor.0 as usize]
                    .as_mut()
                    .and_then(|e| e.orders.iter_mut().find(|o| o.id == id))
                {
                    order.target_pos = Some(place);
                }
            }
            if job == Job::Walk {
                self.pace_together(formation, &members);
            }
        }
    }

    /// Paces a walking formation that keeps to its slowest member so that it arrives together:
    /// the member with the farthest to go at the slowest member's speed, the others slower by
    /// how much nearer they are (never below a quarter of it). A member that has arrived is not
    /// walked through by one still coming up behind it.
    fn pace_together(&mut self, formation: DefId, members: &[(EntityId, OrderId)]) {
        if !self
            .rules
            .formations
            .get(&formation)
            .is_some_and(|f| f.keeps_slowest)
        {
            return;
        }
        let mut walks = Vec::new();
        for (executor, id) in members {
            let Some(e) = self.entity(*executor) else {
                continue;
            };
            let Some(order) = e.orders.iter().find(|o| o.id == *id) else {
                continue;
            };
            let (Some(goal), Some(movement)) = (
                order.target_pos,
                self.rules.entity_types[&e.type_id].movement.as_ref(),
            ) else {
                continue;
            };
            let speed = self
                .effective_stat(*executor, "speed", movement.speed)
                .max(0);
            let far = self
                .place_on(*executor, goal.body)
                .and_then(|(at, _)| self.geo(goal.body).distance(at, goal));
            walks.push((*executor, *id, speed, far));
        }
        let Some(slowest) = walks.iter().map(|w| w.2).min() else {
            return;
        };
        let farthest = walks.iter().filter_map(|w| w.3).max().unwrap_or(0).max(1);
        for (executor, id, _, far) in walks {
            let far = far.unwrap_or(farthest);
            let pace = (slowest * far as i64 / farthest as i64).max(slowest / 4);
            if let Some(order) = self.entities[executor.0 as usize]
                .as_mut()
                .and_then(|e| e.orders.iter_mut().find(|o| o.id == id))
            {
                order.pace = Some(pace);
            }
        }
    }

    /// The places of a group about `goal` on its surface. `forward` is the direction of
    /// travel when the caller knows it; otherwise it runs from the group's centre to the goal,
    /// or along the first member's facing when the group stands on the goal. With `ground` a
    /// member not in orbit gets only a place its movement class can stand on; `keep` limits
    /// the places further. A member without a place keeps the goal. Members in orbit, or all
    /// with `aloft` (ships entering a planet's orbit), keep `formation.orbit_spacing_pct` of
    /// their spacing: above a planet ships are drawn larger than they are.
    fn places_about(
        &self,
        formation: DefId,
        members: &[(EntityId, OrderId)],
        goal: Pos,
        forward: Option<(i64, i64)>,
        (ground, aloft): (bool, bool),
        keep: impl Fn(Pos) -> bool,
    ) -> Vec<(EntityId, OrderId, Pos)> {
        let Some(row) = self.rules.formations.get(&formation) else {
            return Vec::new();
        };
        let Some(surface) = self.surface(goal.body) else {
            return Vec::new();
        };
        let geo = &surface.geo;
        if !geo.contains(goal) {
            return Vec::new();
        }
        let mut group: Vec<Member> = members
            .iter()
            .filter_map(|(id, order)| {
                let e = self.entity(*id)?;
                self.rules.entity_types[&e.type_id].movement.as_ref()?;
                let (from_goal, in_orbit) = match self.place_on(*id, goal.body) {
                    Some((pos, orbit)) => (geo.vector(goal, pos), orbit),
                    None => (None, false),
                };
                Some(Member {
                    id: *id,
                    order: *order,
                    type_id: e.type_id,
                    from_goal,
                    in_orbit,
                })
            })
            .collect();
        if group.len() < 2 {
            return Vec::new();
        }
        let orbit_pct = self
            .rules
            .constant("formation.orbit_spacing_pct")
            .unwrap_or(100)
            .max(10);
        let spacing = group
            .iter()
            .map(|m| {
                let et = &self.rules.entity_types[&m.type_id];
                let above = if aloft || m.in_orbit { orbit_pct } else { 100 };
                let spaced = et.movement.as_ref().map_or(0, |mv| mv.formation_spacing)
                    * row.spacing_pct.max(10)
                    / 100
                    * above
                    / 100;
                spaced.max(2 * et.radius.max(0) + MIN_GAP)
            })
            .max()
            .unwrap_or(0);
        // The group's centre, relative to the goal.
        let measured: Vec<(i64, i64)> = group.iter().filter_map(|m| m.from_goal).collect();
        let centre = if measured.is_empty() {
            (0, 0)
        } else {
            let n = measured.len() as i64;
            let (sx, sy) = measured
                .iter()
                .fold((0, 0), |(x, y), (dx, dy)| (x + dx, y + dy));
            (sx / n, sy / n)
        };
        let far = integer_sqrt((i128::from(centre.0).pow(2) + i128::from(centre.1).pow(2)) as u128)
            as i64
            >= spacing;
        let (fx, fy) = forward
            .and_then(|(x, y)| scaled(x, y, 1000))
            .or_else(|| far.then(|| scaled(-centre.0, -centre.1, 1000)).flatten())
            .unwrap_or_else(|| {
                let first = group.iter().map(|m| m.id).min().expect("a group");
                facing_vector(self.standing_facing(first))
            });
        // Close fighters first, then ranged by reach, then support; within each the one
        // farthest ahead first, then by id.
        let ahead = |m: &Member| {
            m.from_goal.map_or(0, |v| {
                ((v.0 - centre.0) * fx + (v.1 - centre.1) * fy) / 1000
            })
        };
        if row.ranged_behind {
            group.sort_by_key(|m| (rank_key(&self.rules, m.type_id), -ahead(m), m.id));
        } else {
            group.sort_by_key(|m| (-ahead(m), m.id));
        }
        let ranks = formation_ranks(row.shape, group.len(), row.rank_ratio_pct, spacing);
        let across = |v: (i64, i64)| ((v.0 - centre.0) * -fy + (v.1 - centre.1) * fx) / 1000;
        let at = |(side, back): (i64, i64)| {
            let dx = (-fy * side - fx * back) / 1000;
            let dy = (fx * side - fy * back) / 1000;
            geo.offset(goal, dx, dy).0
        };
        let fits = |member: &Member, place: Pos| {
            geo.contains(place)
                && keep(place)
                && (!ground
                    || member.in_orbit
                    || self
                        .avoid
                        .motion
                        .get(member.id.0 as usize)
                        .copied()
                        .flatten()
                        .is_some_and(|m| {
                            surface.pathing.passable(m.class, geo.tile_of(place), None)
                        }))
        };
        let mut out = Vec::with_capacity(group.len());
        let mut taken: Vec<(i64, i64)> = Vec::with_capacity(group.len());
        let mut blocked = Vec::new();
        let mut rest = group.as_slice();
        for rank in ranks {
            let (here, after) = rest.split_at(rank.len().min(rest.len()));
            rest = after;
            // Left to right in the rank as they stand now, so paths cross as little as may be.
            let mut here: Vec<&Member> = here.iter().collect();
            here.sort_by_key(|m| (m.from_goal.map_or(0, across), m.id));
            let mut rank = rank;
            rank.sort();
            for (member, slot) in here.into_iter().zip(rank) {
                taken.push(slot);
                let place = at(slot);
                if fits(member, place) {
                    out.push((member.id, member.order, place));
                } else {
                    blocked.push((member, slot));
                }
            }
        }
        // A member whose place it cannot stand on (a building, water) takes the nearest free
        // spot of the formation's lattice about it, behind it first; failing that, the goal.
        let mut near: Vec<(i64, i64)> = (-3..=3_i64)
            .flat_map(|a| (-3..=3_i64).map(move |b| (a, b)))
            .filter(|p| *p != (0, 0))
            .collect();
        near.sort_by_key(|(a, b)| (a.abs().max(b.abs()), a * a + b * b, -b, *a));
        for (member, slot) in blocked {
            let spot = near.iter().find_map(|(a, b)| {
                let candidate = (slot.0 + a * spacing, slot.1 + b * spacing);
                let free = taken
                    .iter()
                    .all(|t| (t.0 - candidate.0).abs() + (t.1 - candidate.1).abs() >= spacing / 2);
                let place = at(candidate);
                (free && fits(member, place)).then_some((candidate, place))
            });
            if let Some((candidate, place)) = spot {
                taken.push(candidate);
                out.push((member.id, member.order, place));
            }
        }
        out
    }

    /// The places of ships told to hold a body. Over a habitable planet they enter its orbit in
    /// formation about the point the order names (the middle of its first face by default). Beside
    /// any other body the formation stands half the holding reach out on the group's side,
    /// facing the body, and only places within reach of it are given.
    fn hold_places(
        &self,
        formation: DefId,
        members: &[(EntityId, OrderId)],
        target: Option<EntityId>,
        point: Option<Pos>,
    ) -> Option<Vec<(EntityId, OrderId, Pos)>> {
        let habitable = |body: BodyId| {
            self.body(body)
                .is_some_and(|b| b.kind == BodyKind::Habitable)
        };
        let planet = point.map(|p| p.body).filter(|b| habitable(*b)).or_else(|| {
            target.and_then(|t| {
                self.space
                    .markers
                    .iter()
                    .find_map(|(body, id)| (*id == t && habitable(*body)).then_some(*body))
            })
        });
        if let Some(body) = planet {
            let (width, height) = self.surface(body)?.geo.face_size();
            let entry = point
                .filter(|p| p.body == body)
                .unwrap_or_else(|| Pos::on(body, 0, width * 500, height * 500));
            return Some(
                self.places_about(formation, members, entry, None, (false, true), |_| true),
            );
        }
        let held = self.entity(target?)?;
        let at = held.pos;
        let mut reach = self.rules.constant("space.orbit_reach")?;
        if let Some(node) = self.rules.entity_types[&held.type_id].node.as_ref()
            && node.claim_mode == ClaimMode::Presence
            && node.claim_radius > 0
        {
            reach = reach.min(node.claim_radius);
        }
        let geo = self.geo(at.body);
        // From the body toward the group, half the reach out.
        let (sx, sy, n) = members
            .iter()
            .filter_map(|(id, _)| geo.vector(at, self.entity(*id)?.pos))
            .fold((0_i64, 0_i64, 0_i64), |(x, y, n), (dx, dy)| {
                (x + dx, y + dy, n + 1)
            });
        let out = if n > 0 {
            scaled(sx / n, sy / n, 1000)
        } else {
            None
        }
        .unwrap_or_else(|| {
            let first = members.iter().map(|(id, _)| *id).min().expect("a group");
            let (x, y) = facing_vector(self.standing_facing(first));
            (-x, -y)
        });
        let anchor = geo.offset(at, out.0 * reach / 2000, out.1 * reach / 2000).0;
        Some(self.places_about(
            formation,
            members,
            anchor,
            Some((-out.0, -out.1)),
            (false, false),
            |place| geo.within(place, at, reach - HOLD_SLACK),
        ))
    }

    /// The places of ships sent to another system: about the point their order names in it,
    /// facing away from the edge they come in by.
    fn travel_places(
        &self,
        formation: DefId,
        members: &[(EntityId, OrderId)],
        goal: Pos,
    ) -> Option<Vec<(EntityId, OrderId, Pos)>> {
        let first = members.iter().map(|(id, _)| *id).min()?;
        let origin = self.ship_system(self.entity(first)?)?;
        let destination = self.system_space(goal.body)?;
        if origin == destination {
            return None;
        }
        let edge = self.system_edge(destination, origin)?;
        let forward = self.geo(goal.body).vector(edge, goal);
        Some(self.places_about(formation, members, goal, forward, (false, false), |_| true))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat(ranks: &[Vec<(i64, i64)>]) -> Vec<(i64, i64)> {
        ranks.iter().flatten().copied().collect()
    }

    #[test]
    fn shapes_have_their_ranks() {
        let s = 1000;
        // Square: as square as the group allows.
        let square = formation_ranks(FormationShape::Square, 9, 100, s);
        assert_eq!(square.iter().map(Vec::len).collect::<Vec<_>>(), [3, 3, 3]);
        let square = formation_ranks(FormationShape::Square, 10, 100, s);
        assert_eq!(square.iter().map(Vec::len).collect::<Vec<_>>(), [4, 4, 2]);
        // Line: one rank; double line: two.
        assert_eq!(formation_ranks(FormationShape::Line, 7, 400, s).len(), 1);
        let double = formation_ranks(FormationShape::DoubleLine, 7, 100, s);
        assert_eq!(double.iter().map(Vec::len).collect::<Vec<_>>(), [4, 3]);
        // Column: deep and narrow.
        let column = formation_ranks(FormationShape::Column, 16, 25, s);
        assert_eq!(column.len(), 8);
        // Wedge: one at the point, then three.
        let wedge = formation_ranks(FormationShape::Wedge, 4, 400, s);
        assert_eq!(wedge.iter().map(Vec::len).collect::<Vec<_>>(), [1, 3]);
        for shape in [
            FormationShape::Square,
            FormationShape::Line,
            FormationShape::DoubleLine,
            FormationShape::Column,
            FormationShape::Wedge,
            FormationShape::Envelop,
            FormationShape::Refused,
            FormationShape::EchelonLeft,
            FormationShape::EchelonRight,
            FormationShape::Circle,
            FormationShape::Compact,
        ] {
            for n in 1..60 {
                let places = flat(&formation_ranks(shape, n, 100, s));
                assert_eq!(places.len(), n, "{shape:?} {n}");
                let mut unique = places.clone();
                unique.sort();
                unique.dedup();
                assert_eq!(unique.len(), n, "{shape:?} {n}: places overlap");
            }
        }
    }

    #[test]
    fn ranks_run_front_to_back_about_the_centre() {
        let ranks = formation_ranks(FormationShape::DoubleLine, 6, 100, 1000);
        assert!(ranks[0].iter().all(|(_, back)| *back < 0));
        assert!(ranks[1].iter().all(|(_, back)| *back > 0));
        let line = &formation_ranks(FormationShape::Line, 3, 100, 1000)[0];
        assert_eq!(line, &[(-1000, 0), (0, 0), (1000, 0)]);
    }

    #[test]
    fn a_circle_is_a_ring_and_big_ones_are_rings() {
        let ring = formation_ranks(FormationShape::Circle, 8, 100, 1000);
        assert_eq!(ring.len(), 1);
        for (x, y) in &ring[0] {
            let r = integer_sqrt((x * x + y * y) as u128) as i64;
            assert!((1250..=1290).contains(&r), "radius {r}");
        }
        // The first place is straight ahead.
        assert_eq!(ring[0][0].0, 0);
        assert!(ring[0][0].1 < 0);
        let big = formation_ranks(FormationShape::Circle, 40, 100, 1000);
        assert!(big.len() > 1);
        assert_eq!(flat(&big).len(), 40);
    }
}
