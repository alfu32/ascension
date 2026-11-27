//! Tile-aligned placement and cooperative construction work.

use crate::{EntityId, Order, OrderStatus, PlayerId, Pos, SimError, World};
use rom_rules::{DefId, EntityClass, EntityType};

/// A facing snapped to the nearest multiple of 45 degrees, in `0..360` (task 147).
pub fn snap_facing(facing: u16) -> u16 {
    ((u32::from(facing % 360) + 22) / 45 * 45 % 360) as u16
}

/// A footprint of `w` by `h` tiles turned to `facing` (task 147), as tiles across and down: a
/// quarter turn swaps the sides, a diagonal covers the square of the longer side.
pub fn turned_footprint(w: i64, h: i64, facing: u16) -> (i64, i64) {
    match snap_facing(facing) / 45 % 4 {
        0 => (w, h),
        2 => (h, w),
        _ => (w.max(h), w.max(h)),
    }
}

/// The bounds of a building of type `et` centred at `pos` and facing `facing`.
pub(crate) fn footprint(et: &EntityType, pos: Pos, facing: u16) -> [i64; 4] {
    let (w, h) = turned_footprint(et.footprint_w, et.footprint_h, facing);
    let x = i64::from(pos.x);
    let y = i64::from(pos.y);
    [x - w * 500, y - h * 500, x + w * 500, y + h * 500]
}

pub(crate) fn overlaps(a: [i64; 4], b: [i64; 4]) -> bool {
    a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3]
}

impl World {
    /// Puts a footprint on whole tiles nearest to `pos`; other entities keep their point.
    pub(crate) fn snap_placement(&self, subject: DefId, pos: Pos) -> Result<Pos, SimError> {
        self.snap_turned(subject, pos, 0)
    }

    /// [`World::snap_placement`] for a building facing `facing` (task 147).
    pub(crate) fn snap_turned(
        &self,
        subject: DefId,
        pos: Pos,
        facing: u16,
    ) -> Result<Pos, SimError> {
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
        let (w, h) = turned_footprint(et.footprint_w, et.footprint_h, facing);
        let pos = Pos {
            x: snap(pos.x, w),
            y: snap(pos.y, h),
            ..pos
        };
        let fits = geo.footprint_fits(footprint(et, pos, facing));
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
        self.place_turned_for_setup(type_id, owner, pos, 0)
    }

    /// [`World::place_for_setup`] facing `facing`, snapped to a multiple of 45 degrees: a
    /// building's footprint turns with it (task 147).
    pub fn place_turned_for_setup(
        &mut self,
        type_id: DefId,
        owner: PlayerId,
        pos: Pos,
        facing: u16,
    ) -> Result<EntityId, SimError> {
        if self.rules.entity_type(type_id).is_none() {
            return Err(SimError::UnknownType(type_id));
        }
        let facing = snap_facing(facing);
        let pos = self.snap_turned(type_id, pos, facing)?;
        self.spawn_turned(type_id, owner, pos, false, facing)
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

    /// [`World::check_placement`] for a building facing `facing` (task 147), snapped to a
    /// multiple of 45 degrees as `construct` snaps it.
    pub fn check_placement_facing(
        &self,
        player: PlayerId,
        subject: DefId,
        pos: Pos,
        facing: u16,
    ) -> Result<Pos, SimError> {
        self.placement_turned(player, subject, pos, snap_facing(facing), false)
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
        self.placement_turned(player, subject, pos, 0, ignore_territory)
    }

    /// [`World::placement`] for a building facing `facing`.
    pub(crate) fn placement_turned(
        &self,
        player: PlayerId,
        subject: DefId,
        pos: Pos,
        facing: u16,
        ignore_territory: bool,
    ) -> Result<Pos, SimError> {
        let et = self
            .rules
            .entity_types
            .get(&subject)
            .ok_or(SimError::UnknownType(subject))?;
        if let Some(age) = self.placement_discovery_age(player, subject) {
            return Err(SimError::ResourceAgeRequired(age));
        }
        let pos = self.snap_turned(subject, pos, facing)?;
        let bounds = footprint(et, pos, facing);
        let surface = self.surf(pos.body);
        for tile in surface.geo.footprint_tiles(pos.face, bounds) {
            let index = surface.geo.index_of(tile).expect("checked bounds");
            let terrain = surface.map.terrain[index];
            // Ground cover takes the buildings whose units can all move through it (task 172).
            if !self.rules.terrains[&terrain].buildable
                && !self.rules.stands_in_cover(subject, terrain)
            {
                return Err(
                    if self.rules.judged_for_cover(subject) && self.rules.is_cover(terrain) {
                        SimError::UnfitForCover(terrain)
                    } else {
                        SimError::UnbuildableTerrain
                    },
                );
            }
        }
        // A gate may go into the player's own line, over its segments (task 165).
        let replaced = if self.is_gate_type(subject) {
            self.gate_replaces(player, subject, pos, facing)?
        } else {
            Vec::new()
        };
        // Footprints lie within one face, so only footprints on the same face can overlap.
        if (replaced.is_empty() && self.building_overlaps(pos, bounds))
            || self.queued_sites().any(|(other, at, turned)| {
                at.face == pos.face
                    && at.body == pos.body
                    && overlaps(
                        bounds,
                        footprint(&self.rules.entity_types[&other], at, turned),
                    )
            })
        {
            return Err(SimError::PlacementOverlap);
        }
        self.check_placement_rules(player, subject, pos, bounds, ignore_territory)?;
        Ok(pos)
    }

    /// Whether a standing building's footprint overlaps `bounds` on the face of `pos`.
    fn building_overlaps(&self, pos: Pos, bounds: [i64; 4]) -> bool {
        !self.buildings_overlapping(pos, bounds, true).is_empty()
    }

    /// The standing buildings whose footprints overlap `bounds` on the face of `pos`, in id
    /// order; only the first found when `first` is set. Looks only at the building index tiles
    /// a centre of an overlapping building could be on.
    pub(crate) fn buildings_overlapping(
        &self,
        pos: Pos,
        bounds: [i64; 4],
        first: bool,
    ) -> Vec<EntityId> {
        let mut found = Vec::new();
        let Some(surface) = self.surface(pos.body) else {
            return found;
        };
        let (reach_x, reach_y) = (self.max_footprint.0 * 500, self.max_footprint.1 * 500);
        let tiles = |low: i64, high: i64| {
            ((low.div_euclid(1000)) as i32)..=((high.div_euclid(1000)) as i32)
        };
        let geo = &surface.geo;
        let buildings = &surface.vision.buildings;
        for y in tiles(bounds[1] - reach_y, bounds[3] + reach_y) {
            for x in tiles(bounds[0] - reach_x, bounds[2] + reach_x) {
                let Some(index) = geo.index_of(crate::geometry::Tile::new(pos.face, x, y)) else {
                    continue;
                };
                let Some(ids) = buildings.get(&(index as u32)) else {
                    continue;
                };
                for e in ids.iter().filter_map(|id| self.entity(*id)) {
                    if e.pos.face == pos.face
                        && e.pos.body == pos.body
                        && overlaps(
                            bounds,
                            footprint(&self.rules.entity_types[&e.type_id], e.pos, e.facing),
                        )
                    {
                        found.push(e.id);
                        if first {
                            return found;
                        }
                    }
                }
            }
        }
        found.sort_unstable();
        found.dedup();
        found
    }

    /// The free tile of a movement class nearest `anchor` among the tiles within `reach`
    /// milli-tiles around a footprint, as a tile centre. Tiles of buildings, tiles the class
    /// cannot enter (land for ships) and the footprint itself are never chosen.
    pub(crate) fn free_tile_beside(
        &self,
        class: u32,
        at: Pos,
        bounds: [i64; 4],
        anchor: Pos,
        reach: i64,
    ) -> Option<Pos> {
        let surface = self.surf(at.body);
        let geo = &surface.geo;
        geo.footprint_surroundings(at.face, bounds, reach)
            .filter(|&index| surface.pathing.passable(class, geo.tile_at(index), None))
            .map(|index| (geo.rank_distance(anchor, geo.index_centre(index)), index))
            .min()
            .map(|(_, index)| geo.index_centre(index))
    }

    /// Moves every walking unit standing on a building's footprint to the nearest free tile its
    /// movement class can enter, so nothing is ever shut inside a site or a finished building.
    pub(crate) fn clear_footprint(&mut self, building: EntityId) {
        let Some(site) = self.entity(building) else {
            return;
        };
        // A mine blocks nothing: units stay where they stand (task 147).
        if self.mine_types.contains(&site.type_id) {
            return;
        }
        let (at, bounds) = (
            site.pos,
            footprint(
                &self.rules.entity_types[&site.type_id],
                site.pos,
                site.facing,
            ),
        );
        let inside: Vec<(EntityId, Pos, u32)> = self
            .entities()
            .filter(|e| {
                e.id != building
                    && e.container_id.is_none()
                    && e.pos.body == at.body
                    && e.pos.face == at.face
                    && (bounds[0]..bounds[2]).contains(&i64::from(e.pos.x))
                    && (bounds[1]..bounds[3]).contains(&i64::from(e.pos.y))
            })
            .filter_map(|e| {
                let et = &self.rules.entity_types[&e.type_id];
                let class = et.movement.as_ref()?.movement_class_id;
                (et.class != EntityClass::Building && !self.rules.movement_classes[&class].flies)
                    .then_some((e.id, e.pos, class))
            })
            .collect();
        for (id, old, class) in inside {
            let Some(to) = self.free_tile_beside(class, at, bounds, old, 12_000) else {
                continue;
            };
            let unit = self.entities[id.0 as usize].as_mut().expect("unit");
            unit.pos = to;
            // The unit repaths from where it now stands.
            for order in &mut unit.orders {
                order.movement = None;
            }
            self.relocate(id, old, to);
        }
    }

    pub(crate) fn create_site(&mut self, order: &mut Order) {
        let subject = order.subject.expect("construction subject");
        // A gate put into a line takes the place of the segments under it (task 165).
        if self.is_gate_type(subject) {
            let at = order.target_pos.expect("placement");
            self.clear_under_gate(order.player, subject, at, order.facing);
        }
        let id = self
            .spawn_turned(
                subject,
                order.player,
                order.target_pos.expect("placement"),
                true,
                order.facing,
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
        self.clear_footprint(id);
        self.refresh_site(id);
        self.economy.dirty.insert(id);
    }

    /// One tick of a builder mending a damaged own building (task 140), free of cost: each
    /// builder restores `repair_pct` % of the building's hit points over its build time.
    fn advance_mending(&mut self, builder: EntityId, order: &mut Order) {
        let site = self
            .entity(order.target_entity.expect("repair target"))
            .expect("checked target");
        let et = &self.rules.entity_types[&site.type_id];
        if et.class != EntityClass::Building
            || site.neutralised
            || site.assimilation_until.is_some()
        {
            order.status = OrderStatus::Failed("unsupported".into());
            return;
        }
        if site.hp >= site.max_hp {
            order.status = OrderStatus::Done;
            return;
        }
        let (site_id, at, bounds) = (site.id, site.pos, footprint(et, site.pos, site.facing));
        let total = self
            .rules
            .build_ticks(site.type_id, &self.players[&site.owner].active_sources())
            .max(1);
        if !self.approach(builder, order, at, bounds) {
            return;
        }
        let et = &self.rules.entity_types[&self.entity(builder).expect("builder").type_id];
        let Some(component) = et.builder.as_ref() else {
            order.status = OrderStatus::Failed("unsupported".into());
            return;
        };
        let site = self.entity(site_id).expect("site");
        let per_hp = 100 * i128::from(total);
        let work = i128::from(site.max_hp) * i128::from(component.repair_pct.max(0))
            + i128::from(order.build_remainder);
        order.build_remainder = (work % per_hp) as i64;
        order.progress_ticks += 1;
        let site = self.entities[site_id.0 as usize].as_mut().expect("site");
        site.hp = (i128::from(site.hp) + work / per_hp).min(i128::from(site.max_hp)) as i64;
        if site.hp >= site.max_hp {
            order.status = OrderStatus::Done;
        }
    }

    pub(crate) fn advance_construction(&mut self, builder: EntityId, order: &mut Order) {
        let Some(site) = order.target_entity.and_then(|id| self.entity(id)) else {
            order.status = OrderStatus::Failed("target_gone".into());
            return;
        };
        if site.neutralised && site.owner == order.player && order.kind == "repair" {
            let site_id = site.id;
            // Attackers holding the city down keep it down: the builder waits.
            if self.capture.sieges.contains_key(&site_id) {
                return;
            }
            let at = site.pos;
            let bounds = footprint(&self.rules.entity_types[&site.type_id], at, site.facing);
            if self.approach(builder, order, at, bounds) {
                self.restore_city(site_id, order.player);
                order.status = OrderStatus::Done;
            }
            return;
        }
        if site.hp <= 0 || site.owner != order.player {
            order.status = OrderStatus::Failed("target_gone".into());
            return;
        }
        if site.build_progress.is_none() {
            // A repair given for a standing building (no site work recorded) mends it.
            if order.kind == "repair" && order.total_ticks.is_none() {
                self.advance_mending(builder, order);
                return;
            }
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
        let bounds = footprint(
            &self.rules.entity_types[&site.type_id],
            site.pos,
            site.facing,
        );
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
            let (owner, type_id) = (site.owner, site.type_id);
            self.types.site_done(owner, type_id);
            if owner != PlayerId(0) {
                let t = self.tally.players.entry(owner).or_default();
                t.buildings_completed += 1;
                t.cities_founded += u64::from(et.territory.as_ref().is_some_and(|t| t.is_city));
            }
            // A first yield, drawn from the world: at the setup's gathering rate (task 159).
            let bonuses: Vec<_> = et
                .completion_bonus
                .iter()
                .map(|bonus| (bonus.resource_id, bonus.amount))
                .collect();
            for (resource, amount) in bonuses {
                self.credit_gathered(order.player, resource, amount);
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
            self.note_capital(site_id);
            self.clear_footprint(site_id);
        }
    }
}
