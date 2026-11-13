//! Placement rules, city spacing, the player city limit and per-city limits.

use crate::construction::footprint;
use crate::geometry;
use crate::{BodyId, EntityId, PlayerId, Pos, SimError, World};

/// How far `World::find_city_site` searches from its starting point, in milli-tiles.
pub const CITY_SITE_RADIUS: i64 = 16_000;
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
        // Limits belong to the whole building line, including automated sites.
        let root = |mut id| {
            for _ in 0..self.rules.entity_types.len() {
                let Some(previous) = self.rules.entity_types[&id].upgrades_from_id else {
                    break;
                };
                id = previous;
            }
            id
        };
        let subject_root = root(subject);
        let standing = self
            .entities()
            .filter(|e| e.owner == player && root(e.type_id) == subject_root && e.city_id == city)
            .count();
        let queued = self
            .all_orders()
            .filter(|o| {
                o.player == player
                    && o.kind == "construct"
                    && o.target_entity.is_none()
                    && o.subject.is_some_and(|id| root(id) == subject_root)
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
        ignore_territory: bool,
    ) -> Result<(), SimError> {
        let et = &self.rules.entity_types[&subject];
        let is_city = self.is_city_type(subject);
        // A player without a city may found one outside any border.
        let first_city = ignore_territory || (is_city && self.city_count(player) == 0);
        let surface = self.surf(pos.body);
        let geo = &surface.geo;
        let within = |other: Pos, distance: i64| geo.within(pos, other, distance);
        let closer = |other: Pos, distance: i64| geo.closer_than(pos, other, distance);
        let footprint_terrain = || {
            geo.footprint_tiles(pos.face, bounds)
                .map(|t| geo.index_of(t).map(|index| surface.map.terrain[index]))
        };
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
                        || geo.footprint_tiles(pos.face, bounds).all(|t| {
                            self.territory_owner_on_body(pos.body, t.face, t.x, t.y) == Some(player)
                        })
                }
                PlacementKind::InCityRadius => {
                    first_city || self.city_covering(player, pos).is_some()
                }
                PlacementKind::OnTerrain => {
                    footprint_terrain().all(|terrain| terrain == rule.ref_def_id)
                }
                PlacementKind::AdjacentTerrain => geo
                    .footprint_surroundings(pos.face, bounds, rule.distance)
                    .any(|index| rule.ref_def_id == Some(surface.map.terrain[index])),
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
                            e.pos.body,
                            e.pos.face,
                            footprint(&self.rules.entity_types[&e.type_id], e.pos),
                            pos,
                        )
                }),
                PlacementKind::OnLayer => footprint_terrain().all(|terrain| {
                    terrain.is_some_and(|t| Some(self.rules.terrains[&t].layer) == rule.layer)
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

    /// The player's city type: the first city type in id order available to its faction.
    pub fn city_type(&self, player: PlayerId) -> Option<DefId> {
        let faction = self.players.get(&player)?.faction;
        self.rules
            .entity_types
            .values()
            .find(|et| {
                et.territory.as_ref().is_some_and(|t| t.is_city)
                    && self.rules.is_available(et.id, faction)
            })
            .map(|et| et.id)
    }

    /// The nearest centre to `near` on `body`, within `CITY_SITE_RADIUS`, where the
    /// player's city type (see `city_type`) passes every placement check except the
    /// territory rules: surface bounds, terrain, overlap, city spacing and the type's
    /// other placement rules. Ties go to the lower tile index.
    pub fn find_city_site(&self, player: PlayerId, body: BodyId, near: Pos) -> Option<Pos> {
        let city = self.city_type(player)?;
        let surface = self.surface(body)?;
        let near = Pos { body, ..near };
        let geo = &surface.geo;
        let mut candidates: Vec<(u128, usize, Pos)> = geo
            .disc(near, CITY_SITE_RADIUS)
            .flatten()
            .filter_map(|index| {
                let snapped = self.snap_placement(city, geo.index_centre(index)).ok()?;
                let distance = geo.distance_squared(near, snapped)?;
                Some((distance, index, snapped))
            })
            .collect();
        candidates.sort_unstable_by_key(|(distance, index, _)| (*distance, *index));
        candidates.dedup_by_key(|(_, _, pos)| *pos);
        candidates
            .into_iter()
            .filter(|(distance, _, _)| *distance <= (CITY_SITE_RADIUS as u128).pow(2))
            .find_map(|(_, _, pos)| self.placement(player, city, pos, true).ok())
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
