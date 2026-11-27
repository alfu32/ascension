//! Swimming (task 168): a unit tagged `swimmer` crosses the water tagged `shallow` once its
//! stat `swim` is above zero, at that percentage of its speed, and fires from it only while
//! its stat `swim_fire` is. It walks with a path class of its own, its movement class's grid
//! plus that water (`Pathing`); a swimming unit neither gathers nor builds. Nothing here
//! names a definition: who swims, where and how fast are tags and modifier rows.

use crate::{EntityId, PlayerId, Pos, World, avoidance::Motion};
use rom_rules::{DefId, Rules};
use std::collections::BTreeSet;

/// Bits of a path class that hold the movement class; the swim level sits above them.
const SWIM_SHIFT: u32 = 20;
/// The highest swim level a path class can hold.
const SWIM_LEVEL_MAX: u32 = (1 << (32 - SWIM_SHIFT)) - 1;

/// The path class of movement class `base` whose walkers swim shallow water at `level` %.
pub(crate) fn swim_class(base: u32, level: u32) -> u32 {
    (base & ((1 << SWIM_SHIFT) - 1)) | (level.clamp(1, SWIM_LEVEL_MAX) << SWIM_SHIFT)
}

/// The movement class of a path class.
pub(crate) fn base_class(class: u32) -> u32 {
    class & ((1 << SWIM_SHIFT) - 1)
}

/// What the rules say about swimming, derived once from them.
#[derive(Debug, Clone, Default)]
pub(crate) struct SwimRules {
    /// The tag of the water a swimmer may cross.
    pub shallow: Option<u32>,
    /// The tag of the units that may learn to swim.
    pub swimmer: Option<u32>,
    /// The swim percentages the modifier rows grant, ascending: one path class each.
    pub levels: Vec<u32>,
    /// The movement classes of the types that may swim, ascending; never a flying one.
    pub classes: BTreeSet<u32>,
}

impl SwimRules {
    pub fn of(rules: &Rules) -> Self {
        let tag = |key: &str| rules.tags.values().find(|t| t.key == key).map(|t| t.id);
        let (shallow, swimmer) = (tag("shallow"), tag("swimmer"));
        let mut levels: Vec<u32> = rules
            .modifiers
            .values()
            .flatten()
            .filter(|m| m.stat_key == "swim" && m.value > 0)
            .map(|m| m.value.min(i64::from(SWIM_LEVEL_MAX)) as u32)
            .collect();
        levels.sort_unstable();
        levels.dedup();
        let classes = swimmer
            .map(|tag| {
                rules
                    .entity_types
                    .values()
                    .filter(|et| rules.has_tag(et.id, tag))
                    .filter_map(|et| et.movement.as_ref())
                    .map(|m| m.movement_class_id)
                    .filter(|c| rules.movement_classes.get(c).is_some_and(|c| !c.flies))
                    .collect()
            })
            .unwrap_or_default();
        Self {
            shallow,
            swimmer,
            levels,
            classes,
        }
    }

    /// Every (movement class, swim level) pair that gets a path class.
    pub fn variants(&self) -> impl Iterator<Item = (u32, u32)> + '_ {
        self.classes
            .iter()
            .flat_map(|c| self.levels.iter().map(move |l| (*c, *l)))
    }

    /// The level a swim stat of `pct` swims at: the highest level not above it.
    fn level(&self, pct: i64) -> Option<u32> {
        self.levels
            .iter()
            .rev()
            .find(|l| i64::from(**l) <= pct)
            .copied()
    }
}

impl World {
    /// The swim stat of a unit of `type_id` owned by `player`: its speed on shallow water as
    /// a percentage of its speed on plain land; 0 when it cannot swim (task 168).
    pub fn swim_pct(&self, player: PlayerId, type_id: DefId) -> i64 {
        let Some(tag) = self.swim.swimmer else {
            return 0;
        };
        if !self.rules.has_tag(type_id, tag) {
            return 0;
        }
        let Some(sources) = self.players.get(&player).map(|p| p.active_sources()) else {
            return 0;
        };
        self.rules
            .entity_stat("swim", type_id, 0, &sources, &rom_rules::Context::default())
            .max(0)
    }

    /// The path class a unit of `type_id` owned by `player` walks with: its movement class,
    /// widened by shallow water once it can swim.
    pub(crate) fn path_class(&self, player: PlayerId, type_id: DefId) -> Option<u32> {
        let base = self
            .rules
            .entity_type(type_id)?
            .movement
            .as_ref()?
            .movement_class_id;
        if !self.swim.classes.contains(&base) {
            return Some(base);
        }
        Some(
            self.swim
                .level(self.swim_pct(player, type_id))
                .map_or(base, |level| swim_class(base, level)),
        )
    }

    /// The motion of entity `id`: its type's, with the path class its owner's research gives.
    pub(crate) fn motion_of(&self, id: EntityId) -> Option<Motion> {
        let e = self.entity(id)?;
        let mut motion = Motion::of(&self.rules, e.type_id)?;
        if let Some(class) = self.path_class(e.owner, e.type_id) {
            motion.class = class;
        }
        Some(motion)
    }

    /// Brings a walking entity's path class up to date after its owner's research changed.
    pub(crate) fn refresh_swim(&mut self, id: EntityId) {
        let Some(Some(motion)) = self.avoid.motion.get(id.0 as usize).copied() else {
            return;
        };
        let Some(e) = self.entity(id) else {
            return;
        };
        if let Some(class) = self.path_class(e.owner, e.type_id)
            && class != motion.class
        {
            self.avoid.motion[id.0 as usize] = Some(Motion { class, ..motion });
        }
    }

    /// Whether a unit can cross shallow water now.
    pub fn can_swim(&self, player: PlayerId, type_id: DefId) -> bool {
        self.path_class(player, type_id)
            .is_some_and(|class| class != base_class(class))
    }

    /// Whether a unit stands in water it can only swim (task 168).
    pub fn swimming(&self, id: EntityId) -> bool {
        let Some(Some(motion)) = self.avoid.motion.get(id.0 as usize).copied() else {
            return false;
        };
        let base = base_class(motion.class);
        if motion.class == base || motion.flies {
            return false;
        }
        let Some(e) = self.entity(id) else {
            return false;
        };
        let Some(surface) = self.surface(e.pos.body) else {
            return false;
        };
        surface.pathing.speed_pct(base, e.pos) == 0
            && surface.pathing.speed_pct(motion.class, e.pos) > 0
    }

    /// Whether a swimming unit holds its fire: it fires from the water only with a positive
    /// `swim_fire` stat.
    pub(crate) fn holds_fire_in_water(&self, id: EntityId) -> bool {
        self.swimming(id) && self.effective_stat(id, "swim_fire", 0) <= 0
    }

    /// [`World::reachable`] for a unit of `player`: over shallow water too once it can swim.
    pub fn reachable_for(
        &self,
        player: PlayerId,
        type_id: DefId,
        from: Pos,
        to: Pos,
    ) -> Option<bool> {
        let class = self.path_class(player, type_id)?;
        self.reachable_class(class, from, to)
    }

    /// Whether a unit of `type_id` could cross from `from` to `to` over the terrain if it swam
    /// shallow water at `pct` % (computer players weigh a swimming research by it).
    pub fn reachable_swimming(&self, type_id: DefId, pct: i64, from: Pos, to: Pos) -> Option<bool> {
        let base = self
            .rules
            .entity_type(type_id)?
            .movement
            .as_ref()?
            .movement_class_id;
        let swimmer = self
            .swim
            .swimmer
            .is_some_and(|tag| self.rules.has_tag(type_id, tag));
        let class = match self.swim.level(pct) {
            Some(level) if swimmer && self.swim.classes.contains(&base) => swim_class(base, level),
            _ => base,
        };
        self.reachable_class(class, from, to)
    }

    /// The swim percentages the rules grant, ascending.
    pub fn swim_levels(&self) -> &[u32] {
        &self.swim.levels
    }
}
