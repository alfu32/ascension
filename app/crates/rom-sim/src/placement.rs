//! Placement rules, city spacing, the player city limit and per-city limits.

use crate::construction::footprint;
use crate::geometry::{self, footprint_tiles};
use crate::{EntityId, PlayerId, Pos, SimError, World};
use rom_rules::{DefId, LimitScope, PlacementKind};

/// Something standing or about to stand on the map: owner, type and position.
type Placed = (PlayerId, DefId, Pos);

impl World {
    /// Entities, plus construction orders that have not created their site yet.
    fn placed_and_queued(&self) -> impl Iterator<Item = Placed> + '_ {
        self.entities().map(|e| (e.owner, e.type_id, e.pos)).chain(
            self.all_orders()
                .filter(|o| o.kind == "construct" && o.target_entity.is_none())
                .map(|o| {
                    (
                        o.player,
                        o.subject.expect("construction subject"),
                        o.target_pos.expect("placement"),
                    )
                }),
        )
    }

    /// The player's cities: complete, sites and queued constructions.
    pub(crate) fn city_count(&self, player: PlayerId) -> i64 {
        let standing = self
            .territory
            .cities
            .iter()
            .filter(|id| self.entity(**id).is_some_and(|e| e.owner == player))
            .count();
        let queued = self
            .all_orders()
            .filter(|o| {
                o.player == player
                    && o.kind == "construct"
                    && o.target_entity.is_none()
                    && o.subject.is_some_and(|s| self.is_city_type(s))
            })
            .count();
        (standing + queued) as i64
    }

    /// How many cities the player may have, after modifiers.
    pub(crate) fn city_limit(&self, player: PlayerId) -> i64 {
        self.rules.player_stat(
            "city_limit",
            self.rules
                .constant("player.city_limit.base")
                .expect("validated city limit"),
            &self.players[&player].active_sources(),
        )
    }

    /// Rejects a building whose city already holds its per-city limit, counting
    /// complete buildings, sites and queued constructions.
    pub(crate) fn check_city_limit(
        &self,
        player: PlayerId,
        subject: DefId,
        pos: Pos,
    ) -> Result<(), SimError> {
        let et = &self.rules.entity_types[&subject];
        if et.limit_scope != LimitScope::PerCity {
            return Ok(());
        }
        let city = self.city_covering(player, pos);
        let standing = self
            .entities()
            .filter(|e| e.owner == player && e.type_id == subject && e.city_id == city)
            .count();
        let queued = self
            .all_orders()
            .filter(|o| {
                o.player == player
                    && o.kind == "construct"
                    && o.target_entity.is_none()
                    && o.subject == Some(subject)
                    && self.city_covering(player, o.target_pos.expect("placement")) == city
            })
            .count();
        if (standing + queued) as i64 >= et.limit_count {
            return Err(SimError::LimitReached);
        }
        Ok(())
    }

    /// Checks city spacing for cities, then every placement rule of the type.
    pub(crate) fn check_placement_rules(
        &self,
        player: PlayerId,
        subject: DefId,
        pos: Pos,
        bounds: [i64; 4],
    ) -> Result<(), SimError> {
        let et = &self.rules.entity_types[&subject];
        let is_city = self.is_city_type(subject);
        // A player without a city may found one outside any border.
        let first_city = is_city && self.city_count(player) == 0;
        let within = |other: Pos, distance: i64| geometry::within(pos, other, distance);
        let closer = |other: Pos, distance: i64| geometry::closer_than(pos, other, distance);
        if is_city {
            let spacing = self
                .rules
                .constant("city.spacing")
                .expect("validated city spacing");
            if self
                .placed_and_queued()
                .any(|(_, t, p)| self.is_city_type(t) && closer(p, spacing))
            {
                return Err(SimError::CitiesTooClose);
            }
        }
        for rule in &et.placement_rules {
            let pass = match rule.rule {
                PlacementKind::InOwnTerritory => {
                    first_city
                        || footprint_tiles(bounds)
                            .all(|(x, y)| self.territory_owner(x, y) == Some(player))
                }
                PlacementKind::InCityRadius => {
                    first_city || self.city_covering(player, pos).is_some()
                }
                PlacementKind::OnTerrain => footprint_tiles(bounds)
                    .all(|(x, y)| self.map.terrain_at(x, y) == rule.ref_def_id),
                PlacementKind::AdjacentTerrain => {
                    geometry::footprint_surroundings(bounds, rule.distance).any(|(x, y)| {
                        rule.ref_def_id
                            .is_some_and(|t| self.map.terrain_at(x, y) == Some(t))
                    })
                }
                PlacementKind::NearType => self.entities().any(|e| {
                    e.owner == player
                        && Some(e.type_id) == rule.ref_def_id
                        && within(e.pos, rule.distance)
                }),
                PlacementKind::MinDistanceType => !self
                    .placed_and_queued()
                    .any(|(_, t, p)| Some(t) == rule.ref_def_id && closer(p, rule.distance)),
                PlacementKind::MinDistanceEnemyCity => {
                    !self.placed_and_queued().any(|(owner, t, p)| {
                        self.is_city_type(t)
                            && self.enemies(player, owner)
                            && closer(p, rule.distance)
                    })
                }
                PlacementKind::OnNode => self.entities().any(|e| {
                    Some(e.type_id) == rule.ref_def_id
                        && geometry::footprint_contains(
                            footprint(&self.rules.entity_types[&e.type_id], e.pos),
                            pos,
                        )
                }),
                PlacementKind::OnLayer => footprint_tiles(bounds).all(|(x, y)| {
                    self.map
                        .terrain_at(x, y)
                        .is_some_and(|t| Some(self.rules.terrains[&t].layer) == rule.layer)
                }),
            };
            if !pass {
                return Err(match rule.rule {
                    PlacementKind::InOwnTerritory => SimError::NotInOwnTerritory,
                    PlacementKind::InCityRadius => SimError::NotInCityRadius,
                    PlacementKind::OnTerrain => SimError::NotOnTerrain,
                    PlacementKind::AdjacentTerrain => SimError::NotAdjacentTerrain,
                    PlacementKind::NearType => SimError::NotNearType,
                    PlacementKind::MinDistanceType => SimError::TooCloseToType,
                    PlacementKind::MinDistanceEnemyCity => SimError::TooCloseToEnemyCity,
                    PlacementKind::OnNode => SimError::NotOnNode,
                    PlacementKind::OnLayer => SimError::NotOnLayer,
                });
            }
        }
        Ok(())
    }

    /// Ids of the cities a player has, complete or not, in id order.
    pub fn cities(&self, player: PlayerId) -> Vec<EntityId> {
        self.territory
            .cities
            .iter()
            .copied()
            .filter(|id| self.entity(*id).is_some_and(|e| e.owner == player))
            .collect()
    }
}
