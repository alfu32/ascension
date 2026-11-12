//! Four-by-four tile buckets, with ordered membership.

use crate::{EntityId, Pos};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Default)]
pub(crate) struct SpatialGrid {
    buckets: BTreeMap<(i32, i32), BTreeSet<EntityId>>,
}

fn bucket(pos: Pos) -> (i32, i32) {
    (pos.x.div_euclid(4000), pos.y.div_euclid(4000))
}

impl SpatialGrid {
    pub fn insert(&mut self, id: EntityId, pos: Pos) {
        self.buckets.entry(bucket(pos)).or_default().insert(id);
    }

    pub fn remove(&mut self, id: EntityId, pos: Pos) {
        let key = bucket(pos);
        if let Some(ids) = self.buckets.get_mut(&key) {
            ids.remove(&id);
            if ids.is_empty() {
                self.buckets.remove(&key);
            }
        }
    }

    pub fn relocate(&mut self, id: EntityId, from: Pos, to: Pos) {
        if bucket(from) != bucket(to) {
            self.remove(id, from);
            self.insert(id, to);
        }
    }

    pub fn candidates(&self, center: Pos, radius: u32) -> impl Iterator<Item = EntityId> + '_ {
        let min_x = (i64::from(center.x) - i64::from(radius)).div_euclid(4000) as i32;
        let max_x = (i64::from(center.x) + i64::from(radius)).div_euclid(4000) as i32;
        let min_y = (i64::from(center.y) - i64::from(radius)).div_euclid(4000) as i32;
        let max_y = (i64::from(center.y) + i64::from(radius)).div_euclid(4000) as i32;
        self.buckets
            .range((min_x, i32::MIN)..=(max_x, i32::MAX))
            .filter(move |((_, y), _)| (min_y..=max_y).contains(y))
            .flat_map(|(_, ids)| ids.iter().copied())
    }
}
