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
    /// The earliest resource discovery age of a terrain, derived from gather-site components.
    /// Terrain with any ungated yield (or no yield) is ordinary, always-known ground.
    pub fn terrain_discovery_age(&self, terrain: DefId) -> Option<DefId> {
        let mut first = None;
        for et in self.rules.entity_types.values() {
            let Some(site) = &et.gather_site else {
                continue;
            };
            if site.needs_node_terrain_id != Some(terrain)
                && !et.placement_rules.iter().any(|rule| {
                    rule.rule == PlacementKind::OnTerrain && rule.ref_def_id == Some(terrain)
                })
            {
                continue;
            }
            let age = self.rules.resources[&site.resource_id].first_age_id?;
            let ordinal = self.rules.ages[&age].ordinal;
            if first.is_none_or(|(_, old)| ordinal < old) {
                first = Some((age, ordinal));
            }
        }
        first.map(|(age, _)| age)
    }

    /// Whether the player's current age reveals a terrain's resource deposit.
    pub fn knows_terrain(&self, player: PlayerId, terrain: DefId) -> bool {
        self.players.get(&player).is_some_and(|p| {
            self.terrain_discovery_age(terrain)
                .is_none_or(|age| self.rules.ages[&p.age].ordinal >= self.rules.ages[&age].ordinal)
        })
    }

    /// The discovery age still needed by the terrain requirements of a building.
    pub fn placement_discovery_age(&self, player: PlayerId, subject: DefId) -> Option<DefId> {
        let et = self.rules.entity_types.get(&subject)?;
        et.placement_rules
            .iter()
            .filter(|r| {
                matches!(
                    r.rule,
                    PlacementKind::OnTerrain | PlacementKind::AdjacentTerrain
                )
            })
            .filter_map(|r| r.ref_def_id)
            .chain(
                et.gather_site
                    .as_ref()
                    .and_then(|s| s.needs_node_terrain_id),
            )
            .filter(|terrain| !self.knows_terrain(player, *terrain))
            .filter_map(|terrain| self.terrain_discovery_age(terrain))
            .max_by_key(|age| (self.rules.ages[age].ordinal, *age))
    }

    /// Entities of the matching types, plus constructions of them that have not created
    /// their site yet.
    fn placed_and_queued<'a>(
        &'a self,
        types: impl Fn(DefId) -> bool + Copy + 'a,
    ) -> impl Iterator<Item = Placed> + 'a {
        self.entities_of_types(types)
            .map(|e| (e.owner, e.type_id, e.pos))
            .chain(
                self.queued_constructions()
                    .filter(move |(_, t, _)| types(*t)),
            )
    }

    /// The player's cities that count towards the city limit (automated colonies do not):
    /// complete, sites and queued constructions.
    pub fn city_count(&self, player: PlayerId) -> i64 {
        let standing = self
            .territory
            .cities
            .iter()
            .filter(|id| {
                self.entity(**id)
                    .is_some_and(|e| e.owner == player && self.counts_as_city(e.type_id))
            })
            .count();
        let queued = self
            .queued_constructions()
            .filter(|(owner, subject, _)| *owner == player && self.counts_as_city(*subject))
            .count();
        (standing + queued) as i64
    }

    /// Whether the player has any city at all, colonies, sites and queued ones included.
    fn has_any_city(&self, player: PlayerId) -> bool {
        self.territory
            .cities
            .iter()
            .any(|id| self.entity(*id).is_some_and(|e| e.owner == player))
            || self
                .queued_constructions()
                .any(|(owner, subject, _)| owner == player && self.is_city_type(subject))
    }

    /// Whether the player has a city, a city site or a queued city on `body`.
    pub(crate) fn has_city_on(&self, player: PlayerId, body: BodyId) -> bool {
        self.territory.cities.iter().any(|id| {
            self.entity(*id)
                .is_some_and(|e| e.owner == player && e.pos.body == body)
        }) || self.queued_constructions().any(|(owner, subject, at)| {
            owner == player && self.is_city_type(subject) && at.body == body
        })
    }

    /// How many cities the player may have, after modifiers.
    pub fn city_limit(&self, player: PlayerId) -> i64 {
        self.rules.player_stat(
            "city_limit",
            self.rules
                .constant("player.city_limit.base")
                .expect("validated city limit"),
            &self.active_sources(player),
        )
    }

    /// Rejects a building whose city already holds its per-city limit, counting
    /// complete buildings, sites and queued constructions. In an automated colony's area the
    /// colony's cap for the line is the limit (`et_colony_site.cap`).
    pub(crate) fn check_city_limit(
        &self,
        player: PlayerId,
        subject: DefId,
        pos: Pos,
    ) -> Result<(), SimError> {
        let et = &self.rules.entity_types[&subject];
        let city = self.city_covering(player, pos);
        let cap = city
            .and_then(|id| self.entity(id))
            .and_then(|c| self.colony_cap(c.type_id, subject));
        if et.limit_scope != LimitScope::PerCity && cap.is_none() {
            return Ok(());
        }
        let limit = cap.unwrap_or(et.limit_count);
        // Limits belong to the whole building line, including automated sites.
        let root = |id| self.line_root(id);
        let subject_root = root(subject);
        let standing: usize = self
            .types
            .of(player)
            .filter(|(type_id, _)| root(*type_id) == subject_root)
            .map(|(type_id, _)| {
                self.own_of_type(player, type_id)
                    .filter(|e| e.city_id == city)
                    .count()
            })
            .sum();
        let queued = self
            .queued_constructions()
            .filter(|(owner, id, at)| {
                *owner == player
                    && root(*id) == subject_root
                    && self.city_covering(player, *at) == city
            })
            .count();
        if (standing + queued) as i64 >= limit {
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
        let first_city = ignore_territory || (is_city && !self.has_any_city(player));
        // The first city on another planet may also stand outside any border, though never
        // inside another player's (as free colonisation allows everywhere).
        let free_colonisation = self.rules.ages[&self.players[&player].age].free_colonisation
            || (is_city && !first_city && !self.has_city_on(player, pos.body));
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
                .placed_and_queued(|t| self.is_city_type(t))
                .any(|(_, _, p)| closer(p, spacing))
            {
                return Err(SimError::CitiesTooClose);
            }
        }
        for rule in &et.placement_rules {
            let pass = match rule.rule {
                PlacementKind::InOwnTerritory
                    if is_city && free_colonisation && !ignore_territory =>
                {
                    // Free colonisation: only another player's land is closed to a new city.
                    geo.footprint_tiles(pos.face, bounds).all(|t| {
                        self.territory_owner_on_body(pos.body, t.face, t.x, t.y)
                            .is_none_or(|owner| owner == player)
                    })
                }
                PlacementKind::InOwnTerritory => {
                    first_city
                        || geo.footprint_tiles(pos.face, bounds).all(|t| {
                            self.territory_owner_on_body(pos.body, t.face, t.x, t.y) == Some(player)
                        })
                }
                PlacementKind::InCityRadius => {
                    first_city || self.footprint_in_city_radius(player, pos, bounds)
                }
                PlacementKind::OnTerrain => {
                    footprint_terrain().all(|terrain| terrain == rule.ref_def_id)
                }
                PlacementKind::AdjacentTerrain => geo
                    .footprint_surroundings(pos.face, bounds, rule.distance)
                    .any(|index| rule.ref_def_id == Some(surface.map.terrain[index])),
                PlacementKind::NearType => rule.ref_def_id.is_some_and(|near| {
                    self.own_of_type(player, near)
                        .any(|e| within(e.pos, rule.distance))
                }),
                PlacementKind::MinDistanceType => !self
                    .placed_and_queued(|t| Some(t) == rule.ref_def_id)
                    .any(|(_, _, p)| closer(p, rule.distance)),
                PlacementKind::MinDistanceEnemyCity => !self
                    .placed_and_queued(|t| self.is_city_type(t))
                    .any(|(owner, _, p)| self.enemies(player, owner) && closer(p, rule.distance)),
                PlacementKind::OnNode => self
                    .entities_of_types(|t| Some(t) == rule.ref_def_id)
                    .any(|e| {
                        geometry::footprint_contains(
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
                    PlacementKind::InOwnTerritory
                        if is_city && free_colonisation && !ignore_territory =>
                    {
                        SimError::InForeignTerritory
                    }
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

    /// Whether one complete city of the player has all four corners of the footprint inside its
    /// economic radius, so a building never stands half outside the city that serves it.
    fn footprint_in_city_radius(&self, player: PlayerId, pos: Pos, bounds: [i64; 4]) -> bool {
        let geo = self.geo(pos.body);
        let corners = [
            (bounds[0], bounds[1]),
            (bounds[2] - 1, bounds[1]),
            (bounds[0], bounds[3] - 1),
            (bounds[2] - 1, bounds[3] - 1),
        ]
        .map(|(x, y)| Pos {
            x: x as i32,
            y: y as i32,
            ..pos
        });
        self.territory.cities.iter().any(|id| {
            let Some(city) = self.entity(*id) else {
                return false;
            };
            let radius = self.city_economic_radius(*id);
            city.owner == player
                && city.build_progress.is_none()
                && !city.neutralised
                && city.pos.body == pos.body
                && corners.iter().all(|c| geo.within(city.pos, *c, radius))
        })
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
