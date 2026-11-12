//! Tile-aligned placement and cooperative construction work.

use crate::{EntityId, Order, OrderStatus, Pos, SimError, World, entity::move_entity};
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
    pub(crate) fn check_placement(&self, subject: DefId, pos: Pos) -> Result<Pos, SimError> {
        let et = &self.rules.entity_types[&subject];
        // Snap footprint edges to tile boundaries, including odd-sized footprints.
        let snap = |coordinate: i32, size: i64| {
            ((i64::from(coordinate) - size * 500 + 500).div_euclid(1000) * 1000 + size * 500) as i32
        };
        let pos = Pos {
            x: snap(pos.x, et.footprint_w),
            y: snap(pos.y, et.footprint_h),
        };
        let bounds = footprint(et, pos);
        if bounds[0] < 0
            || bounds[1] < 0
            || bounds[2] > i64::from(self.map.width) * 1000
            || bounds[3] > i64::from(self.map.height) * 1000
        {
            return Err(SimError::OutsideMap);
        }
        for y in bounds[1] / 1000..bounds[3] / 1000 {
            for x in bounds[0] / 1000..bounds[2] / 1000 {
                let terrain = self
                    .map
                    .terrain_at(x as i32, y as i32)
                    .expect("checked bounds");
                if !self.rules.terrains[&terrain].buildable {
                    return Err(SimError::UnbuildableTerrain);
                }
            }
        }
        if self.entities().any(|e| {
            let other = &self.rules.entity_types[&e.type_id];
            other.class == EntityClass::Building && overlaps(bounds, footprint(other, e.pos))
        }) || self
            .all_orders()
            .filter(|o| o.kind == "construct" && o.target_entity.is_none())
            .any(|o| {
                overlaps(
                    bounds,
                    footprint(
                        &self.rules.entity_types[&o.subject.expect("construction subject")],
                        o.target_pos.expect("placement"),
                    ),
                )
            })
        {
            return Err(SimError::PlacementOverlap);
        }
        self.check_territory_and_placement_rules(subject, pos)?;
        Ok(pos)
    }

    fn check_territory_and_placement_rules(
        &self,
        _subject: DefId,
        _pos: Pos,
    ) -> Result<(), SimError> {
        Ok(())
    }

    pub(crate) fn create_site(&mut self, order: &mut Order) {
        let subject = order.subject.expect("construction subject");
        let id = self
            .spawn(subject, order.player, order.target_pos.expect("placement"))
            .expect("accepted site fits map and entity id capacity");
        let pct = self
            .rules
            .constant("build.site_start_hp_pct")
            .expect("validated site hp constant");
        let site = self.entities[id.0 as usize].as_mut().expect("new site");
        site.build_progress = Some(0);
        site.gather_slots = 0;
        site.hp = (i128::from(site.hp) * i128::from(pct) / 100) as i64;
        order.target_entity = Some(id);
    }

    pub(crate) fn advance_construction(&mut self, builder: EntityId, order: &mut Order) {
        let Some(site) = order.target_entity.and_then(|id| self.entity(id)) else {
            order.status = OrderStatus::Failed("target_gone".into());
            return;
        };
        if site.owner != order.player {
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
        let builder_pos = self.entity(builder).expect("builder").pos;
        let closest = Pos {
            x: i64::from(builder_pos.x).clamp(bounds[0], bounds[2]) as i32,
            y: i64::from(builder_pos.y).clamp(bounds[1], bounds[3]) as i32,
        };
        let reach = self
            .rules
            .constant("build.reach")
            .expect("validated build reach") as u128;
        if builder_pos.distance_squared_wide(closest) > reach * reach {
            let mut movement = order.clone();
            movement.target_pos = Some(closest);
            let entity = self.entities[builder.0 as usize].as_mut().expect("builder");
            move_entity(entity, &mut movement, &self.rules, &self.map);
            order.movement = movement.movement;
            if let OrderStatus::Failed(reason) = movement.status {
                order.status = OrderStatus::Failed(reason);
                return;
            }
            if entity.pos.distance_squared_wide(closest) > reach * reach {
                return;
            }
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
            site.hp = et.hp;
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
            let initial = i128::from(et.hp) * i128::from(pct) / 100;
            site.hp = (initial
                + (i128::from(et.hp) - initial) * i128::from(progress) / i128::from(total))
                as i64;
        }
        if progress >= total {
            let site = self.entity(site_id).expect("site");
            let slots = self.gather_slots(site.type_id, site.pos);
            self.entities[site_id.0 as usize]
                .as_mut()
                .expect("site")
                .gather_slots = slots;
        }
    }
}
