//! Tile-aligned placement and cooperative construction work.

use crate::{EntityId, Order, OrderStatus, PlayerId, Pos, SimError, World};
use rom_rules::{DefId, EntityClass, EntityType};

pub(crate) fn footprint(et: &EntityType, pos: Pos) -> [i64; 4] {
    let x = i64::from(pos.x);
    let y = i64::from(pos.y);
    [
        x - et.footprint_w * 500,
        y - et.footprint_h * 500,
        x + et.footprint_w * 500,
        y + et.footprint_h * 500,
    ]
}

fn overlaps(a: [i64; 4], b: [i64; 4]) -> bool {
    a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3]
}

impl World {
    /// Puts a footprint on whole tiles nearest to `pos`; other entities keep their point.
    pub(crate) fn snap_placement(&self, subject: DefId, pos: Pos) -> Result<Pos, SimError> {
        let et = &self.rules.entity_types[&subject];
        let geo = &self
            .surface(pos.body)
            .ok_or(SimError::UnknownBody(pos.body))?
            .geo;
        if et.class != EntityClass::Building {
            return Ok(pos);
        }
        // Snap footprint edges to tile boundaries, including odd-sized footprints.
        let snap = |coordinate: i32, size: i64| {
            ((i64::from(coordinate) - size * 500 + 500).div_euclid(1000) * 1000 + size * 500) as i32
        };
        let requested = pos;
        let pos = Pos {
            x: snap(pos.x, et.footprint_w),
            y: snap(pos.y, et.footprint_h),
            ..pos
        };
        let fits = geo.footprint_fits(footprint(et, pos));
        if geo.is_flat() {
            return if fits {
                Ok(pos)
            } else {
                Err(SimError::OutsideMap)
            };
        }
        // On a sphere a requested point must be on the surface, and the footprint
        // snapped around it must lie within that point's face.
        if !geo.contains(requested) {
            return Err(SimError::OutsideMap);
        }
        if !fits {
            return Err(SimError::CrossesFaceEdge);
        }
        if !geo.contains(pos) {
            return Err(SimError::OutsideMap);
        }
        Ok(pos)
    }

    /// Places a complete entity for match setup, exempt from every placement rule;
    /// buildings are snapped to whole tiles and must fit on the surface of the position's
    /// body.
    pub fn place_for_setup(
        &mut self,
        type_id: DefId,
        owner: PlayerId,
        pos: Pos,
    ) -> Result<EntityId, SimError> {
        if self.rules.entity_type(type_id).is_none() {
            return Err(SimError::UnknownType(type_id));
        }
        let pos = self.snap_placement(type_id, pos)?;
        self.spawn(type_id, owner, pos)
    }

    /// Whether `player` may place a building of type `subject` centred near `pos`: map bounds,
    /// terrain, overlap with buildings and queued sites, and every placement rule. Returns the
    /// snapped centre. This is the same check `construct` runs at submit, without side effects,
    /// so interfaces can preview a placement.
    pub fn check_placement(
        &self,
        player: PlayerId,
        subject: DefId,
        pos: Pos,
    ) -> Result<Pos, SimError> {
        self.placement(player, subject, pos, false)
    }

    /// `check_placement`, optionally leaving out the territory rules (`in_own_territory`,
    /// `in_city_radius`).
    pub(crate) fn placement(
        &self,
        player: PlayerId,
        subject: DefId,
        pos: Pos,
        ignore_territory: bool,
    ) -> Result<Pos, SimError> {
        let et = self
            .rules
            .entity_types
            .get(&subject)
            .ok_or(SimError::UnknownType(subject))?;
        let pos = self.snap_placement(subject, pos)?;
        let bounds = footprint(et, pos);
        let surface = self.surf(pos.body);
        for tile in surface.geo.footprint_tiles(pos.face, bounds) {
            let index = surface.geo.index_of(tile).expect("checked bounds");
            if !self.rules.terrains[&surface.map.terrain[index]].buildable {
                return Err(SimError::UnbuildableTerrain);
            }
        }
        // Footprints lie within one face, so only footprints on the same face can overlap.
        if self.entities().any(|e| {
            let other = &self.rules.entity_types[&e.type_id];
            other.class == EntityClass::Building
                && e.pos.face == pos.face
                && e.pos.body == pos.body
                && overlaps(bounds, footprint(other, e.pos))
        }) || self
            .all_orders()
            .filter(|o| o.kind == "construct" && o.target_entity.is_none())
            .any(|o| {
                let at = o.target_pos.expect("placement");
                at.face == pos.face
                    && at.body == pos.body
                    && overlaps(
                        bounds,
                        footprint(
                            &self.rules.entity_types[&o.subject.expect("construction subject")],
                            at,
                        ),
                    )
            })
        {
            return Err(SimError::PlacementOverlap);
        }
        self.check_placement_rules(player, subject, pos, bounds, ignore_territory)?;
        Ok(pos)
    }

    pub(crate) fn create_site(&mut self, order: &mut Order) {
        let subject = order.subject.expect("construction subject");
        let id = self
            .spawn_entity(
                subject,
                order.player,
                order.target_pos.expect("placement"),
                true,
            )
            .expect("accepted site fits map and entity id capacity");
        let pct = self
            .rules
            .constant("build.site_start_hp_pct")
            .expect("validated site hp constant");
        let site = self.entities[id.0 as usize].as_mut().expect("new site");
        site.gather_slots = 0;
        site.hp = (i128::from(site.hp) * i128::from(pct) / 100) as i64;
        order.target_entity = Some(id);
        self.refresh_site(id);
        self.economy.dirty.insert(id);
    }

    pub(crate) fn advance_construction(&mut self, builder: EntityId, order: &mut Order) {
        let Some(site) = order.target_entity.and_then(|id| self.entity(id)) else {
            order.status = OrderStatus::Failed("target_gone".into());
            return;
        };
        if site.hp <= 0 || site.owner != order.player {
            order.status = OrderStatus::Failed("target_gone".into());
            return;
        }
        if site.build_progress.is_none() {
            order.status = if order.kind == "construct" || order.total_ticks.is_some() {
                OrderStatus::Done
            } else {
                OrderStatus::Failed("unsupported".into())
            };
            return;
        }
        let site_id = site.id;
        let total = self
            .rules
            .build_ticks(site.type_id, &self.players[&site.owner].active_sources())
            .clamp(0, i64::from(u32::MAX)) as u32;
        order.total_ticks = Some(u64::from(total));
        let bounds = footprint(&self.rules.entity_types[&site.type_id], site.pos);
        if !self.approach(builder, order, site.pos, bounds) {
            return;
        }
        let et = &self.rules.entity_types[&self.entity(builder).expect("builder").type_id];
        let Some(component) = et.builder.as_ref() else {
            order.status = OrderStatus::Failed("unsupported".into());
            return;
        };
        let work = i128::from(component.build_pct) + i128::from(order.build_remainder);
        order.build_remainder = (work % 100) as i64;
        order.progress_ticks += 1;
        let site = self.entities[site_id.0 as usize].as_mut().expect("site");
        let progress = (u64::from(site.build_progress.expect("incomplete"))
            + (work / 100).max(0) as u64)
            .min(u64::from(total)) as u32;
        let et = &self.rules.entity_types[&site.type_id];
        if progress >= total {
            site.build_progress = None;
            site.hp = site.max_hp;
            for bonus in &et.completion_bonus {
                let balance = self
                    .players
                    .get_mut(&order.player)
                    .expect("owner")
                    .resources
                    .entry(bonus.resource_id)
                    .or_default();
                *balance = balance.saturating_add(bonus.amount);
            }
            order.status = OrderStatus::Done;
        } else {
            site.build_progress = Some(progress);
            let pct = self
                .rules
                .constant("build.site_start_hp_pct")
                .expect("site hp");
            let initial = i128::from(site.max_hp) * i128::from(pct) / 100;
            site.hp = (initial
                + (i128::from(site.max_hp) - initial) * i128::from(progress) / i128::from(total))
                as i64;
        }
        if progress >= total {
            let site = self.entity(site_id).expect("site");
            let slots = self.gather_slots(site.type_id, site.pos);
            self.entities[site_id.0 as usize]
                .as_mut()
                .expect("site")
                .gather_slots = slots;
            self.refresh_site(site_id);
            self.economy.dirty.insert(site_id);
            self.refresh_territory(site_id);
        }
    }
}
