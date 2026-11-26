//! Task 156: shield emitters. An emitter is a fortification (a segment or a gate) whose type has
//! an `et_field_emitter` row. From the tile at its forward end (its footprint's end along its
//! facing) its field runs along the facing over at most `gap` tiles to the nearest standing
//! emitter of its owner; those tiles then block every walker, as a segment does, while both
//! emitters are powered. An emitter is powered for `upkeep_period_ticks` each time it pays its
//! `upkeep` from its owner's store; the tick it is paid until is kept in its
//! [`crate::Entity::energy`] (hashed and saved with every entity). Unpaid, its field drops and
//! only the emitters block.
//!
//! A gate's site (task 165) is an end of a field too, so a gate put into a standing line keeps
//! the field joined on both sides of it while it is built.
//!
//! The fields are derived from the entities: [`World::refresh_fields`] runs at the end of every
//! tick and after a restore, so a restored world blocks the same tiles.

use crate::construction::snap_facing;
use crate::{BodyId, EntityId, PlayerId, Pos, World};
use rom_rules::{DefId, Rules};
use std::collections::{BTreeMap, BTreeSet};

/// One stretch of field between two emitters of one owner (derived from the entities; not
/// hashed, not saved).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldLink {
    /// The emitters' owner.
    pub owner: PlayerId,
    /// The emitter it runs from.
    pub from: EntityId,
    /// The emitter it reaches.
    pub to: EntityId,
    /// The centre of the tile at `from`'s forward end.
    pub start: Pos,
    /// The centre of the first tile of `to` it reaches.
    pub end: Pos,
    /// Tiles of field between them (0 for neighbours).
    pub tiles: u8,
    /// Whether the field stands: both emitters powered.
    pub up: bool,
}

/// Field emitter rules and the fields standing (all derived).
#[derive(Debug, Clone, Default)]
pub(crate) struct FieldState {
    /// Types with an `et_field_emitter` row, in id order.
    pub types: Vec<DefId>,
    /// Every stretch between two emitters, up or down, in the order of their emitters' ids.
    pub links: Vec<FieldLink>,
    /// The tiles a standing field blocks, each counted once in the path occupancy.
    pub tiles: BTreeSet<(BodyId, u32)>,
}

impl FieldState {
    /// The rules' emitter types.
    pub fn of(rules: &Rules) -> Self {
        Self {
            types: rules
                .entity_types
                .values()
                .filter(|et| et.field_emitter.is_some())
                .map(|et| et.id)
                .collect(),
            ..Self::default()
        }
    }
}

/// The step in tile coordinates of a facing (a multiple of 45 degrees, as `facing_of` gives).
pub(crate) fn facing_step(facing: u16) -> (i32, i32) {
    const STEPS: [(i32, i32); 8] = [
        (1, 0),
        (1, 1),
        (0, 1),
        (-1, 1),
        (-1, 0),
        (-1, -1),
        (0, -1),
        (1, -1),
    ];
    STEPS[usize::from(snap_facing(facing) / 45 % 8)]
}

impl World {
    /// Field emitters in id order: finished, standing buildings of an emitter type on a surface.
    fn emitters(&self) -> Vec<EntityId> {
        let mut ids: Vec<EntityId> = self
            .entities_of_types(|t| self.fields.types.contains(&t))
            .filter(|e| e.build_progress.is_none() && e.hp > 0 && e.container_id.is_none())
            .map(|e| e.id)
            .collect();
        ids.sort_unstable();
        ids
    }

    /// What a field runs between, in id order: the finished emitters and the sites of gates
    /// that are emitters (task 165). A gate put into a standing line holds the field on both
    /// sides of it while it is built.
    fn field_ends(&self) -> Vec<EntityId> {
        let mut ids: Vec<EntityId> = self
            .entities_of_types(|t| self.fields.types.contains(&t))
            .filter(|e| e.hp > 0 && e.container_id.is_none())
            .filter(|e| e.build_progress.is_none() || self.is_gate_type(e.type_id))
            .map(|e| e.id)
            .collect();
        ids.sort_unstable();
        ids
    }

    /// Whether a field end holds its side of a field: a finished emitter while powered, a gate's
    /// site always (task 165).
    fn holds_field(&self, id: EntityId) -> bool {
        self.entity(id)
            .is_some_and(|e| e.build_progress.is_some() || self.emitter_powered(id))
    }

    /// Whether the emitter `id` is powered now: paid for beyond this tick and not neutralised.
    pub(crate) fn emitter_powered(&self, id: EntityId) -> bool {
        self.entity(id).is_some_and(|e| {
            !e.neutralised
                && e.build_progress.is_none()
                && self.rules.entity_types[&e.type_id].field_emitter.is_some()
                && e.energy > i64::try_from(self.tick).unwrap_or(i64::MAX)
        })
    }

    /// Pays the upkeep of every finished emitter whose paid time has run out, in id order: one
    /// period more for each that its owner's store can pay; the others stay unpowered and try
    /// again next tick.
    pub(crate) fn power_fields(&mut self) {
        if self.fields.types.is_empty() {
            return;
        }
        let now = i64::try_from(self.tick).unwrap_or(i64::MAX);
        for id in self.emitters() {
            let e = self.entity(id).expect("emitter");
            if e.energy > now {
                continue;
            }
            let owner = e.owner;
            let spec = self.rules.entity_types[&e.type_id]
                .field_emitter
                .clone()
                .expect("emitter type");
            if let Some(resource) = spec.upkeep_resource_id.filter(|_| spec.upkeep > 0) {
                let Some(player) = self.players.get_mut(&owner) else {
                    continue;
                };
                let store = player.resources.entry(resource).or_default();
                if *store < spec.upkeep {
                    continue;
                }
                *store -= spec.upkeep;
            }
            let e = self.entities[id.0 as usize].as_mut().expect("emitter");
            e.energy = now.saturating_add(spec.upkeep_period_ticks.max(2));
        }
    }

    /// Rebuilds the stretches of field from the emitters standing and puts the tiles of the
    /// standing ones into the path occupancy (taking out those that dropped).
    pub(crate) fn refresh_fields(&mut self) {
        if self.fields.types.is_empty() {
            return;
        }
        let emitters = self.field_ends();
        if emitters.is_empty() && self.fields.tiles.is_empty() && self.fields.links.is_empty() {
            return;
        }
        // Every tile of every emitter, to the emitter standing on it.
        let mut on_tile: BTreeMap<(BodyId, usize), EntityId> = BTreeMap::new();
        for &id in &emitters {
            let e = self.entity(id).expect("emitter");
            let et = &self.rules.entity_types[&e.type_id];
            let geo = self.geo(e.pos.body);
            let bounds = crate::construction::footprint(et, e.pos, e.facing);
            for tile in geo.footprint_tiles(e.pos.face, bounds) {
                if let Some(index) = geo.index_of(tile) {
                    on_tile.insert((e.pos.body, index), id);
                }
            }
        }
        let mut found: BTreeMap<(EntityId, EntityId), FieldLink> = BTreeMap::new();
        let mut tiles: BTreeSet<(BodyId, u32)> = BTreeSet::new();
        for &id in &emitters {
            let e = self.entity(id).expect("emitter");
            let et = &self.rules.entity_types[&e.type_id];
            let gap = et.field_emitter.as_ref().map_or(0, |f| f.gap.clamp(0, 16)) as i32;
            let (owner, body) = (e.owner, e.pos.body);
            let geo = self.geo(body);
            let (dx, dy) = facing_step(e.facing);
            let centre = geo.tile_of(e.pos);
            let half = ((et.footprint_w.max(1) - 1) / 2) as i32;
            let (ax, ay) = (centre.x + dx * half, centre.y + dy * half);
            let Some(anchor) = geo.plane_index(centre.face, ax, ay) else {
                continue;
            };
            let mut between: Vec<usize> = Vec::new();
            let mut reached = None;
            for k in 1..=gap + 1 {
                let Some(index) = geo.plane_index(centre.face, ax + dx * k, ay + dy * k) else {
                    break;
                };
                match on_tile.get(&(body, index)) {
                    Some(&other) if other == id => break,
                    Some(&other) => {
                        if self.entity(other).is_some_and(|o| o.owner == owner) {
                            reached = Some((other, index));
                        }
                        break;
                    }
                    None => between.push(index),
                }
            }
            let Some((other, end)) = reached else {
                continue;
            };
            let key = (id.min(other), id.max(other));
            if found.contains_key(&key) {
                continue;
            }
            // A field stands between two powered emitters, or a powered emitter and a gate's
            // site; never between two sites.
            let finished = |e: EntityId| self.entity(e).is_some_and(|e| e.build_progress.is_none());
            let up = self.holds_field(id)
                && self.holds_field(other)
                && (finished(id) || finished(other));
            if up {
                tiles.extend(between.iter().map(|&i| (body, i as u32)));
            }
            found.insert(
                key,
                FieldLink {
                    owner,
                    from: id,
                    to: other,
                    start: geo.index_centre(anchor),
                    end: geo.index_centre(end),
                    tiles: between.len() as u8,
                    up,
                },
            );
        }
        // The path occupancy follows: tiles that dropped come out, new ones go in.
        let dropped: Vec<(BodyId, u32)> = self.fields.tiles.difference(&tiles).copied().collect();
        let raised: Vec<(BodyId, u32)> = tiles.difference(&self.fields.tiles).copied().collect();
        for (list, add) in [(dropped, false), (raised, true)] {
            for (body, index) in list {
                if let Some(slot) = self.slot_of(body) {
                    self.surfaces[slot].pathing.block_tile(index as usize, add);
                }
            }
        }
        self.fields.tiles = tiles;
        self.fields.links = found.into_values().collect();
    }

    /// The stretches of field between emitters, up or down, in the order of their emitters'
    /// ids (task 156).
    pub fn field_links(&self) -> &[FieldLink] {
        &self.fields.links
    }

    /// Whether a field stands on the tile under `pos` (task 156).
    pub fn field_at(&self, pos: Pos) -> bool {
        self.surface(pos.body)
            .and_then(|s| s.geo.pos_index(pos))
            .is_some_and(|index| self.fields.tiles.contains(&(pos.body, index as u32)))
    }

    /// Whether the emitter `id` is powered now (task 156); false for anything else.
    pub fn field_powered(&self, id: EntityId) -> bool {
        self.emitter_powered(id)
    }
}
