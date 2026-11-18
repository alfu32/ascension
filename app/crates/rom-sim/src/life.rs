//! Bringing life (task 77): every resource body was given a dormant surface before play (a cube
//! of barren ground); an ascended player may bring the body to life, which retires its resource
//! node and makes it a habitable planet, and then paint that planet's land, water, heights,
//! biomes and deposits with the brushes of the rules, stroke by stroke, while no material
//! player has a city on it (`docs/bringing-life.md`). Each stroke is an order applied at a
//! tick; the painted heights are state, hashed once a body lives and saved.

use crate::{
    BodyId, Command, EntityId, Order, OrderId, OrderStatus, PlayerId, Pos, SimError, World,
};
use rom_geo::CubeSurface;
use rom_rules::{BodyKind, ClaimMode, EntityClass, HeightOp, Layer, LifeBrush};
use std::collections::{BTreeMap, BTreeSet};

/// The dormant surface a resource body was given before play: `n` tiles a face of `terrain`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DormantSurface {
    /// Tiles along a face edge.
    pub n: u32,
    /// The barren ground that covers it.
    pub terrain: rom_rules::DefId,
}

/// A body an ascended player brought to life.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Living {
    /// The player that brought it to life; only it paints.
    pub by: PlayerId,
    /// The tick it came to life.
    pub since: u64,
    /// Strokes applied so far; views re-bake when it changes.
    pub strokes: u64,
    /// Height 0..=255 per tile in the surface's dense order; `life.sea_height` is sea level.
    pub heights: Vec<u8>,
}

/// Dormant surfaces not yet woken and the living bodies.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct LifeState {
    /// Resource bodies' dormant surfaces, by body (inputs of the universe).
    pub dormant: BTreeMap<BodyId, DormantSurface>,
    /// Bodies brought to life, by body.
    pub living: BTreeMap<BodyId, Living>,
}

/// Height of a dormant surface's ground when the rules give none.
const GROUND_HEIGHT: i64 = 112;
/// Sea level when the rules give none.
const SEA_HEIGHT: i64 = 100;
/// A raising brush lays its terrain where its dome stands above this share (per mille) of its
/// height: the rim only rises, as foothills.
const RAISE_CORE_PERMILLE: i64 = 400;
/// Water painted at the rim of a lowering brush lies this far below sea level.
const SHORE_DEPTH: i64 = 4;

/// Smoothstep of `w` per mille: 0 at the rim, 1000 at the centre, flat at both.
fn smooth(w: i64) -> i64 {
    let w = w.clamp(0, 1000);
    w * w * (3000 - 2 * w) / 1_000_000
}

impl World {
    /// The dormant surface of a resource body not yet brought to life.
    pub fn dormant(&self, body: BodyId) -> Option<&DormantSurface> {
        self.life.dormant.get(&body)
    }

    /// Resource bodies with a dormant surface, in body order.
    pub fn dormant_bodies(&self) -> impl Iterator<Item = (BodyId, &DormantSurface)> + '_ {
        self.life.dormant.iter().map(|(b, d)| (*b, d))
    }

    /// A body brought to life, with its painted heights.
    pub fn living(&self, body: BodyId) -> Option<&Living> {
        self.life.living.get(&body)
    }

    /// Bodies brought to life, in body order.
    pub fn living_bodies(&self) -> impl Iterator<Item = (BodyId, &Living)> + '_ {
        self.life.living.iter().map(|(b, l)| (*b, l))
    }

    /// Sea level of a planet brought to life (`life.sea_height`).
    pub fn sea_height(&self) -> u8 {
        self.rules
            .constant("life.sea_height")
            .unwrap_or(SEA_HEIGHT)
            .clamp(1, 254) as u8
    }

    fn ground_height(&self) -> i64 {
        self.rules
            .constant("life.ground_height")
            .unwrap_or(GROUND_HEIGHT)
            .clamp(i64::from(self.sea_height()), 255)
    }

    /// The resource body whose marker is `marker`.
    pub fn body_of_marker(&self, marker: EntityId) -> Option<BodyId> {
        self.space
            .markers
            .iter()
            .find(|(_, m)| **m == marker)
            .map(|(b, _)| *b)
    }

    /// Whether `player` may bring `body` to life now: it is ascended and the body is a
    /// resource body with a dormant surface.
    pub fn life_check(&self, player: PlayerId, body: BodyId) -> Result<(), SimError> {
        if !self.is_ascended(player) {
            return Err(SimError::OrdersClosed);
        }
        let resource = self
            .body(body)
            .is_some_and(|b| b.kind == BodyKind::Resource);
        if !resource || !self.life.dormant.contains_key(&body) || self.slot_of(body).is_some() {
            return Err(SimError::NoDormantSurface);
        }
        Ok(())
    }

    /// Whether `player` may paint `body` now: it is ascended, it brought the body to life, and
    /// no city stands there.
    pub fn paint_check(&self, player: PlayerId, body: BodyId) -> Result<(), SimError> {
        if !self.is_ascended(player) {
            return Err(SimError::OrdersClosed);
        }
        if self.life.living.get(&body).is_none_or(|l| l.by != player) {
            return Err(SimError::NotYourLiving);
        }
        if self.entities().any(|e| {
            e.pos.body == body
                && self.rules.entity_types[&e.type_id]
                    .territory
                    .as_ref()
                    .is_some_and(|t| t.is_city)
        }) {
            return Err(SimError::Settled);
        }
        Ok(())
    }

    /// Checks a `bring_life` or `paint` command (no executor) and queues it for the next tick.
    pub(crate) fn submit_life(&mut self, command: Command) -> Result<OrderId, SimError> {
        if !self.rules.order_kinds.contains_key(&command.kind) {
            return Err(SimError::UnknownKind(command.kind));
        }
        let (target_entity, target_pos, subject) = if command.kind == "paint" {
            let (Some(at), None, Some(brush)) =
                (command.target_pos, command.target_entity, command.subject)
            else {
                return Err(SimError::WrongTargetShape);
            };
            if !self.rules.brushes.contains_key(&brush) {
                return Err(SimError::UnknownBrush);
            }
            self.paint_check(command.player, at.body)?;
            if !self.geo(at.body).contains(at) {
                return Err(SimError::OutsideMap);
            }
            (None, Some(at), Some(brush))
        } else {
            let (None, Some(marker), None) =
                (command.target_pos, command.target_entity, command.subject)
            else {
                return Err(SimError::WrongTargetShape);
            };
            let body = self
                .body_of_marker(marker)
                .ok_or(SimError::NoDormantSurface)?;
            self.life_check(command.player, body)?;
            if self
                .pending
                .iter()
                .any(|(o, _)| o.kind == command.kind && o.target_entity == Some(marker))
            {
                return Err(SimError::NoDormantSurface);
            }
            (Some(marker), None, None)
        };
        let next = self
            .next_order_id
            .checked_add(1)
            .ok_or(SimError::IdExhausted)?;
        let mut order = Order::internal(
            command.player,
            EntityId(0),
            &command.kind,
            target_entity,
            target_pos,
        );
        order.id = OrderId(self.next_order_id);
        order.subject = subject;
        order.status = OrderStatus::Queued;
        order.issued_tick = self.tick;
        order.automatic = false;
        self.pending.push((order, false));
        self.next_order_id = next;
        Ok(OrderId(self.next_order_id - 1))
    }

    /// Applies a queued `bring_life`: the body's resource node is retired (claims on it end),
    /// a planet marker stands in its place, and the body becomes a habitable planet with its
    /// dormant surface as its ground. Fails (and is counted) if that is no longer possible.
    pub(crate) fn apply_bring_life(&mut self, order: &Order) {
        let Some(body) = order.target_entity.and_then(|m| self.body_of_marker(m)) else {
            self.stats.orders_failed += 1;
            return;
        };
        if self.life_check(order.player, body).is_err() {
            self.stats.orders_failed += 1;
            return;
        }
        let Some(marker_type) = self
            .rules
            .entity_types
            .values()
            .find(|e| e.class == EntityClass::Node && e.node.is_none() && e.movement.is_none())
            .map(|e| e.id)
        else {
            self.stats.orders_failed += 1;
            return;
        };
        let dormant = self.life.dormant.remove(&body).expect("checked dormant");
        let surface = CubeSurface { n: dormant.n };
        let tiles = surface.len();
        if self
            .insert_surface(body, surface, vec![dormant.terrain; tiles])
            .is_err()
        {
            self.life.dormant.insert(body, dormant);
            self.stats.orders_failed += 1;
            return;
        }
        // The resource node leaves: whoever held it is paid no more.
        self.space.claims.remove(&body);
        if let Some(old) = self.space.markers.remove(&body) {
            let at = self.entity(old).map(|e| e.pos);
            self.despawn(old).expect("live marker");
            if let Some(at) = at
                && let Ok(id) = self.spawn(marker_type, PlayerId(0), at)
            {
                self.space.markers.insert(body, id);
            }
        }
        let index = self
            .universe
            .bodies
            .binary_search_by_key(&body, |b| b.id)
            .expect("checked body");
        let info = &mut self.universe.bodies[index];
        info.kind = BodyKind::Habitable;
        info.n = Some(dormant.n);
        info.node_type = None;
        let ground = self.ground_height() as u8;
        self.life.living.insert(
            body,
            Living {
                by: order.player,
                since: self.tick,
                strokes: 0,
                heights: vec![ground; tiles],
            },
        );
        self.stats.orders_done += 1;
    }

    /// Applies a queued `paint` stroke. Tiles under a building or a unit keep their ground.
    pub(crate) fn apply_paint(&mut self, order: &Order) {
        let (Some(at), Some(brush)) = (
            order.target_pos,
            order.subject.and_then(|s| self.rules.brushes.get(&s)),
        ) else {
            self.stats.orders_failed += 1;
            return;
        };
        if self.paint_check(order.player, at.body).is_err() {
            self.stats.orders_failed += 1;
            return;
        }
        let brush = brush.clone();
        self.paint_stroke(at, &brush);
        self.life
            .living
            .get_mut(&at.body)
            .expect("checked living")
            .strokes += 1;
        self.stats.orders_done += 1;
    }

    /// Lays `brush` around `centre`: terrain, heights, and the brush's node at the centre.
    fn paint_stroke(&mut self, centre: Pos, brush: &LifeBrush) {
        let body = centre.body;
        let sea = i64::from(self.sea_height());
        let ground = self.ground_height();
        let radius = brush.radius.max(1);
        // Tiles that hold something keep their ground.
        let mut held: BTreeSet<usize> = BTreeSet::new();
        let geo = self.geo(body).clone();
        for id in self.entities_in_radius(centre, (radius + 4000).clamp(0, 1 << 30) as u32) {
            if let Some(e) = self.entity(id)
                && let Some(i) = geo.pos_index(e.pos)
            {
                held.insert(i);
            }
        }
        let slot = self.slot(body);
        let mut tiles: Vec<(usize, i64)> = geo
            .disc(centre, radius)
            .flatten()
            .filter_map(|i| Some((i, geo.distance(centre, geo.index_centre(i))? as i64)))
            .filter(|(i, d)| *d <= radius && !held.contains(i))
            .collect();
        tiles.sort_unstable();
        tiles.dedup_by_key(|(i, _)| *i);
        let rules = std::sync::Arc::clone(&self.rules);
        let is_ground = |t: rom_rules::DefId| rules.terrains[&t].layer == Layer::Ground;
        let land_only = match brush.terrain_id {
            None => true,
            Some(t) => brush.height_op == HeightOp::Keep && is_ground(t),
        };
        let surface = &mut self.surfaces[slot];
        let living = self.life.living.get_mut(&body).expect("checked living");
        let mut ground_delta: i64 = 0;
        for (i, d) in tiles {
            if surface.pathing.occupancy[i] > 0 {
                continue;
            }
            let old = surface.map.terrain[i];
            if land_only && !is_ground(old) {
                continue;
            }
            let w = (radius - d) * 1000 / radius;
            let current = i64::from(living.heights[i]);
            let target = brush.height.clamp(0, 255);
            let mut terrain = brush.terrain_id.unwrap_or(old);
            let height = match brush.height_op {
                HeightOp::Keep => current,
                HeightOp::Set => target,
                HeightOp::Raise => {
                    let dome = ground + (target - ground) * smooth(w) / 1000;
                    if brush.terrain_id.is_some() && w < RAISE_CORE_PERMILLE {
                        terrain = old;
                    }
                    current.max(dome)
                }
                HeightOp::Lower => {
                    let rim = sea - SHORE_DEPTH;
                    current.min(rim + (target - rim) * smooth(w) / 1000)
                }
            };
            // Water lies below the sea and ground above it.
            let height = if is_ground(terrain) {
                if height < sea { ground } else { height }
            } else {
                height.min(sea - 1)
            };
            if is_ground(old) != is_ground(terrain) {
                ground_delta += if is_ground(terrain) { 1 } else { -1 };
            }
            surface.map.terrain[i] = terrain;
            living.heights[i] = height.clamp(0, 255) as u8;
        }
        surface.pathing.refresh_terrain(&rules, &surface.map);
        self.victory.ownable_tiles = self
            .victory
            .ownable_tiles
            .saturating_add_signed(ground_delta);
        if let Some(node) = brush.node_type_id {
            self.paint_node(centre, node, &held);
        }
    }

    /// Sets a deposit node at the tile under `centre`, if that tile is free ground with no
    /// other node within the node's own size.
    fn paint_node(&mut self, centre: Pos, node: rom_rules::DefId, held: &BTreeSet<usize>) {
        let geo = self.geo(centre.body);
        let Some(index) = geo.pos_index(centre) else {
            return;
        };
        let at = geo.index_centre(index);
        let et = &self.rules.entity_types[&node];
        if held.contains(&index)
            || !self
                .terrain_at(at)
                .is_some_and(|t| self.rules.terrains[&t].layer == Layer::Ground)
            || !et
                .node
                .as_ref()
                .is_some_and(|n| n.claim_mode != ClaimMode::Presence)
        {
            return;
        }
        let reach = (et.footprint_w.max(et.footprint_h) * 1000).clamp(1000, 16_000) as u32;
        let crowded = self.entities_in_radius(at, reach).into_iter().any(|id| {
            self.entity(id)
                .is_some_and(|e| self.rules.entity_types[&e.type_id].node.is_some())
        });
        if !crowded {
            let _ = self.spawn(node, PlayerId(0), at);
        }
    }
}
