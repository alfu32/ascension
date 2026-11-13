//! Four-by-four tile buckets per face, with ordered membership.

use crate::{EntityId, Pos, geometry::Geometry};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Default)]
pub(crate) struct SpatialGrid {
    buckets: BTreeMap<(u8, i32, i32), BTreeSet<EntityId>>,
}

fn bucket(pos: Pos) -> (u8, i32, i32) {
    (pos.face, pos.x.div_euclid(4000), pos.y.div_euclid(4000))
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

    /// Entities in buckets touching the square around `center` on every face it covers.
    pub fn candidates<'a>(
        &'a self,
        geo: &Geometry,
        center: Pos,
        radius: u32,
    ) -> impl Iterator<Item = EntityId> + 'a {
        let radius = i64::from(radius);
        geo.views(center, radius)
            .into_iter()
            .flatten()
            .flat_map(move |view| {
                let min_x = (view.x - radius).div_euclid(4000) as i32;
                let max_x = (view.x + radius).div_euclid(4000) as i32;
                let min_y = (view.y - radius).div_euclid(4000) as i32;
                let max_y = (view.y + radius).div_euclid(4000) as i32;
                self.buckets
                    .range((view.face, min_x, i32::MIN)..=(view.face, max_x, i32::MAX))
                    .filter(move |((_, _, y), _)| (min_y..=max_y).contains(y))
                    .flat_map(|(_, ids)| ids.iter().copied())
            })
    }
}
