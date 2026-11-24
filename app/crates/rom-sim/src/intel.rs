//! What each player knows of the others (task 130): first contact, the planet-wide survey of
//! buildings, and every capital known at the level research and sight give.

use crate::{BodyId, Entity, EntityId, PlayerId, Pos, World};
use std::collections::{BTreeMap, BTreeSet};

/// `tech_effect.rule_key` (task 130): every building on the player's home planet is known and
/// kept current, as if seen; units stay in the fog. Shared with the player's team, as sight is.
pub const RULE_PLANET_SURVEY: &str = "planet_survey";
/// Each player met: its capital's star system.
pub const RULE_CAPITAL_SYSTEM: &str = "capital_system";
/// Each capital whose system is known: its planet.
pub const RULE_CAPITAL_PLANET: &str = "capital_planet";
/// Each capital whose planet is known: the city.
pub const RULE_CAPITAL_CITY: &str = "capital_city";
/// Every capital, met or not, at city level.
pub const RULE_ALL_CAPITALS: &str = "all_capitals";

/// The nature player: it owns nodes, meets nobody and has no capital.
const NATURE: PlayerId = PlayerId(0);

/// How much a player knows of a capital; each level implies the coarser ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CapitalLevel {
    /// The star system it is in.
    System = 1,
    /// The planet it stands on.
    Planet = 2,
    /// The city itself and where it stands.
    City = 3,
}

impl CapitalLevel {
    /// The level saved as `n` (1 system, 2 planet, 3 city).
    pub fn from_number(n: u8) -> Option<Self> {
        match n {
            1 => Some(Self::System),
            2 => Some(Self::Planet),
            3 => Some(Self::City),
            _ => None,
        }
    }

    /// The level's saved number.
    pub fn number(self) -> u8 {
        self as u8
    }
}

/// What a player keeps of another player's capital: the level known, the city, and where it
/// stood when last known.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapitalMemory {
    /// How much is known.
    pub level: CapitalLevel,
    /// The capital city's entity id.
    pub city: EntityId,
    /// Where it stood when last known.
    pub pos: Pos,
}

/// A capital as one player knows it ([`World::known_capitals`]). Fields finer than `level` are
/// `None`, so a view can draw exactly what is known.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KnownCapital {
    /// Whose capital it is.
    pub owner: PlayerId,
    /// How much is known.
    pub level: CapitalLevel,
    /// Its star system (`None` in a world without star systems).
    pub system: Option<u32>,
    /// Its planet, from [`CapitalLevel::Planet`].
    pub body: Option<BodyId>,
    /// The city, at [`CapitalLevel::City`].
    pub city: Option<EntityId>,
    /// Where the city stood when last known, at [`CapitalLevel::City`].
    pub pos: Option<Pos>,
    /// Whether the viewer sees the city now.
    pub visible: bool,
}

/// First contacts and remembered capitals (hashed, saved); the surveys are derived.
#[derive(Debug, Default)]
pub(crate) struct IntelState {
    /// (viewer, other): the viewer, or its team, has seen a unit or building of the other.
    pub contacts: BTreeSet<(PlayerId, PlayerId)>,
    /// (viewer, owner): the owner's capital as the viewer knows it; enemies only (own and
    /// allied capitals are always known in full).
    pub capitals: BTreeMap<(PlayerId, PlayerId), CapitalMemory>,
    /// (player, surface slot): surfaces whose buildings the player knows by a survey.
    pub surveys: BTreeSet<(PlayerId, usize)>,
}

/// Which steps of the capital ladder a player has researched.
#[derive(Debug, Clone, Copy)]
struct Ladder {
    all: bool,
    system: bool,
    planet: bool,
    city: bool,
}

impl Ladder {
    /// The level research gives of the capital of a player `met` or not.
    fn level(self, met: bool) -> Option<CapitalLevel> {
        if self.all {
            return Some(CapitalLevel::City);
        }
        if !(met && self.system) {
            return None;
        }
        Some(if self.planet && self.city {
            CapitalLevel::City
        } else if self.planet {
            CapitalLevel::Planet
        } else {
            CapitalLevel::System
        })
    }
}

impl World {
    /// Whether `viewer` has met `other`: it or its team has seen a unit or building of the
    /// other (or knew its buildings by a survey). Allies have always met.
    pub fn has_met(&self, viewer: PlayerId, other: PlayerId) -> bool {
        self.allied(viewer, other) || self.intel.contacts.contains(&(viewer, other))
    }

    /// The players `viewer` has met, in id order (allies not included).
    pub fn contacts(&self, viewer: PlayerId) -> impl Iterator<Item = PlayerId> + '_ {
        self.intel
            .contacts
            .range((viewer, PlayerId(0))..=(viewer, PlayerId(u32::MAX)))
            .map(|(_, other)| *other)
    }

    /// Whether `player` knows every building on the surface of `body` by a survey
    /// (`tech_effect.rule_key = 'planet_survey'` of its own or a team member's home planet).
    pub fn surveys(&self, player: PlayerId, body: BodyId) -> bool {
        self.slot_of(body)
            .is_some_and(|slot| self.intel.surveys.contains(&(player, slot)))
    }

    /// Every capital `viewer` knows, by owner: its own and its allies' in full; an enemy's at
    /// the level its research and its sight give, where it stood when last known.
    pub fn known_capitals(&self, viewer: PlayerId) -> Vec<KnownCapital> {
        let mut out = Vec::new();
        for owner in self.players.keys().copied().filter(|p| *p != NATURE) {
            let memory = if self.allied(viewer, owner) {
                self.capital(owner)
                    .and_then(|id| self.entity(id))
                    .map(|e| CapitalMemory {
                        level: CapitalLevel::City,
                        city: e.id,
                        pos: e.pos,
                    })
            } else {
                self.intel.capitals.get(&(viewer, owner)).copied()
            };
            let Some(m) = memory else { continue };
            let planet = m.level >= CapitalLevel::Planet;
            let city = m.level == CapitalLevel::City;
            out.push(KnownCapital {
                owner,
                level: m.level,
                system: self.body(m.pos.body).map(|b| b.system),
                body: planet.then_some(m.pos.body),
                city: city.then_some(m.city),
                pos: city.then_some(m.pos),
                visible: self
                    .entity(m.city)
                    .is_some_and(|e| self.sees_entity(viewer, e)),
            });
        }
        out
    }

    /// The surface slot of a player's home planet; the only surface of a one-surface world.
    fn home_slot(&self, player: PlayerId) -> Option<usize> {
        match self.home_body(player) {
            Some(body) => self.slot_of(body),
            None => (self.surfaces.len() == 1).then_some(0),
        }
    }

    /// End of a tick: surveys, first contacts, then what each player knows of each capital.
    pub(crate) fn update_intel(&mut self) {
        self.refresh_surveys();
        self.make_contacts();
        self.update_capitals();
    }

    /// The surveys the research of every player gives now; a surface newly surveyed has every
    /// building on it remembered at once.
    pub(crate) fn refresh_surveys(&mut self) {
        let now = self.surveys_now();
        if now == self.intel.surveys {
            return;
        }
        let fresh: BTreeSet<usize> = now
            .difference(&self.intel.surveys)
            .map(|(_, slot)| *slot)
            .collect();
        self.intel.surveys = now;
        for slot in fresh {
            let tiles: Vec<u32> = self.surfaces[slot]
                .vision
                .buildings
                .keys()
                .copied()
                .collect();
            for tile in tiles {
                self.remember_tile(slot, tile);
            }
        }
    }

    /// The (player, surface slot) pairs surveyed by the research every player holds now.
    pub(crate) fn surveys_now(&self) -> BTreeSet<(PlayerId, usize)> {
        let mut now = BTreeSet::new();
        for p in self.players.values().filter(|p| p.id != NATURE) {
            let Some(slot) = self.home_slot(p.id) else {
                continue;
            };
            if !self.rule_enabled(p.id, RULE_PLANET_SURVEY) {
                continue;
            }
            for q in self.players.values().filter(|q| q.team == p.team) {
                now.insert((q.id, slot));
            }
        }
        now
    }

    /// Whether `player` knows the buildings of the surface at `slot` by a survey.
    pub(crate) fn surveyed(&self, player: PlayerId, slot: usize) -> bool {
        !self.intel.surveys.is_empty() && self.intel.surveys.contains(&(player, slot))
    }

    /// Records the first contacts of this tick: a player meets another once it sees a unit or
    /// building of it, or knows one of its buildings by a survey.
    fn make_contacts(&mut self) {
        let ids: Vec<PlayerId> = self
            .players
            .keys()
            .copied()
            .filter(|p| *p != NATURE)
            .collect();
        // Owner → players that have yet to meet it.
        let mut pending: BTreeMap<PlayerId, Vec<PlayerId>> = BTreeMap::new();
        for &owner in &ids {
            for &viewer in &ids {
                if !self.has_met(viewer, owner) {
                    pending.entry(owner).or_default().push(viewer);
                }
            }
        }
        if pending.is_empty() {
            return;
        }
        // Surfaces where each of those viewers sees something or surveys: elsewhere it can
        // meet nobody.
        let mut watched: BTreeMap<PlayerId, Vec<bool>> = BTreeMap::new();
        for viewer in pending.values().flatten().copied() {
            if watched.contains_key(&viewer) {
                continue;
            }
            let ascended = self.is_ascended(viewer);
            let slots = (0..self.surfaces.len())
                .map(|slot| {
                    ascended
                        || self.surveyed(viewer, slot)
                        || self
                            .team_vision(viewer, slot)
                            .any(|v| v.count.uniform_value() != Some(0))
                })
                .collect();
            watched.insert(viewer, slots);
        }
        let mut met: Vec<(PlayerId, PlayerId)> = Vec::new();
        for e in self.entities() {
            let Some(viewers) = pending.get(&e.owner) else {
                continue;
            };
            for &viewer in viewers {
                if met.contains(&(viewer, e.owner)) {
                    continue;
                }
                let slots = &watched[&viewer];
                if !self.places(e).any(|slot| slots[slot]) {
                    continue;
                }
                if self.survey_shows(viewer, e) || self.sees_entity(viewer, e) {
                    met.push((viewer, e.owner));
                }
            }
        }
        self.intel.contacts.extend(met);
    }

    /// The surface slots an entity may be seen on: where it stands, the ground below its
    /// orbit, its drone grid.
    fn places<'a>(&'a self, e: &'a Entity) -> impl Iterator<Item = usize> + 'a {
        [
            Some(e.pos),
            self.orbit(e.id).map(|o| o.pos),
            self.drone_cloud(e.id).map(|c| c.pos),
        ]
        .into_iter()
        .flatten()
        .filter_map(|pos| self.slot_of(pos.body))
    }

    /// Whether `viewer` knows a building by a survey of the surface it stands on.
    fn survey_shows(&self, viewer: PlayerId, e: &Entity) -> bool {
        e.container_id.is_none()
            && self.is_building(e.type_id)
            && !self.hidden_mine(viewer, e)
            && self
                .slot_of(e.pos.body)
                .is_some_and(|slot| self.surveyed(viewer, slot))
    }

    /// The capital ladder `player` has researched.
    fn ladder(&self, player: PlayerId) -> Ladder {
        Ladder {
            all: self.rule_enabled(player, RULE_ALL_CAPITALS),
            system: self.rule_enabled(player, RULE_CAPITAL_SYSTEM),
            planet: self.rule_enabled(player, RULE_CAPITAL_PLANET),
            city: self.rule_enabled(player, RULE_CAPITAL_CITY),
        }
    }

    /// Brings every player's knowledge of every enemy capital up to date. Research keeps the
    /// levels it gives current at once (a capital taken keeps its place; one destroyed or
    /// replaced is updated); sight raises a capital to city level, and what was seen stays
    /// known where it stood until the viewer sees that place again.
    fn update_capitals(&mut self) {
        let ids: Vec<PlayerId> = self
            .players
            .keys()
            .copied()
            .filter(|p| *p != NATURE)
            .collect();
        for &viewer in &ids {
            let ladder = self.ladder(viewer);
            for &owner in &ids {
                if self.allied(viewer, owner) {
                    continue;
                }
                let key = (viewer, owner);
                let old = self.intel.capitals.get(&key).copied();
                let live = self
                    .capital(owner)
                    .and_then(|id| self.entity(id))
                    .map(|e| (e.id, e.pos));
                let researched = ladder.level(self.has_met(viewer, owner));
                let mut new = old;
                if let Some(level) = researched {
                    new = live.map(|(city, pos)| CapitalMemory {
                        level: match old {
                            Some(m) if m.city == city => m.level.max(level),
                            _ => level,
                        },
                        city,
                        pos,
                    });
                }
                if let Some((city, pos)) = live
                    && self.knows_by_sight(viewer, city, pos)
                {
                    new = Some(CapitalMemory {
                        level: CapitalLevel::City,
                        city,
                        pos,
                    });
                } else if researched.is_none()
                    && let Some(m) = new
                    && live.is_none_or(|(city, _)| city != m.city)
                    && self.place_visible(viewer, m.pos)
                {
                    // Its place is in view and the capital is not there any more.
                    new = None;
                }
                if new != old {
                    match new {
                        Some(m) => self.intel.capitals.insert(key, m),
                        None => self.intel.capitals.remove(&key),
                    };
                }
            }
        }
    }

    /// Whether `viewer` sees the city now or remembers it standing (it saw it, or knows it by
    /// a survey).
    fn knows_by_sight(&self, viewer: PlayerId, city: EntityId, pos: Pos) -> bool {
        if self
            .entity(city)
            .is_some_and(|e| self.sees_entity(viewer, e))
        {
            return true;
        }
        let Some(slot) = self.slot_of(pos.body) else {
            return false;
        };
        let surface = &self.surfaces[slot];
        surface.geo.pos_index(pos).is_some_and(|tile| {
            surface
                .vision
                .players
                .get(&viewer)
                .is_some_and(|v| v.memory.contains_key(&(tile as u32, city)))
        })
    }

    /// Whether `viewer` sees the tile of `pos` now.
    fn place_visible(&self, viewer: PlayerId, pos: Pos) -> bool {
        self.slot_of(pos.body).is_some_and(|slot| {
            self.surfaces[slot]
                .geo
                .pos_index(pos)
                .is_some_and(|tile| self.visible_index(viewer, slot, tile))
        })
    }

    /// Every remembered capital, by viewer and owner, for a save.
    pub(crate) fn capital_memories(
        &self,
    ) -> impl Iterator<Item = ((PlayerId, PlayerId), CapitalMemory)> + '_ {
        self.intel.capitals.iter().map(|(k, m)| (*k, *m))
    }
}
