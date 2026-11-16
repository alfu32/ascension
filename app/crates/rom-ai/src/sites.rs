//! Where to build: candidate tiles around own cities, filtered cheaply against what the
//! player knows, then confirmed with the simulation's own placement preview.

use crate::{
    catalogue::SEARCH_TILES,
    memory::EnemySighting,
    view::{Cell, City, View, cell_of, threat_map},
};
use rom_rules::{DefId, EntityType, LimitScope, PlacementKind};
use rom_sim::{EntityId, Pos};
use std::collections::{BTreeMap, BTreeSet};

/// Placement previews one thought may ask the simulation for; each costs the simulation a
/// pass over every entity.
const PREVIEWS: usize = 4;
/// Tile lookups one thought may spend on looking for sites.
const LOOKUPS: usize = 60_000;
/// Radius, in tiles, in which a new city's surroundings are valued.
const CITY_VALUE_TILES: i32 = 10;

/// The result of a site search.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Site {
    /// A confirmed site.
    At(Pos),
    /// Nothing the player knows of will do.
    Nowhere,
    /// The thought's preview budget ran out before the search was finished.
    Later,
}

/// A tile: body, face, x, y.
type Tile = (u32, u8, i32, i32);

/// Known obstacles and dangers, plus what this thought has already planned to build.
#[derive(Debug, Default)]
pub(crate) struct Ground {
    /// Tiles under known buildings.
    taken: BTreeSet<Tile>,
    /// The same tiles and a one-tile lane around them.
    lanes: BTreeSet<Tile>,
    /// Known enemy strength per coarse cell.
    pub threat: BTreeMap<Cell, i64>,
    /// Buildings planned during this thought: line root and city.
    planned: Vec<(DefId, Option<EntityId>)>,
    /// Footprints kept free for the first gather site of a resource drawn from a node
    /// terrain: other buildings must not take the only spot such a site has.
    reserved: BTreeSet<Tile>,
    /// Whether the reservations of this thought have been made.
    pub reserved_done: bool,
    /// Placement previews asked of the simulation during this thought.
    previews: std::cell::Cell<usize>,
    /// Tile lookups spent on site searches during this thought.
    lookups: std::cell::Cell<usize>,
}

/// The snapped centre of a footprint of `size` tiles requested at `coordinate`, as the
/// simulation snaps it.
fn snap(coordinate: i32, size: i64) -> i32 {
    ((i64::from(coordinate) - size * 500 + 500).div_euclid(1000) * 1000 + size * 500) as i32
}

/// First tile and extent of a footprint centred at `pos`.
fn footprint(et: &EntityType, pos: Pos) -> (i32, i32, i32, i32) {
    let (w, h) = (et.footprint_w.max(1), et.footprint_h.max(1));
    (
        (i64::from(pos.x) - w * 500).div_euclid(1000) as i32,
        (i64::from(pos.y) - h * 500).div_euclid(1000) as i32,
        w as i32,
        h as i32,
    )
}

impl Ground {
    /// Collects obstacles from own buildings and remembered enemy buildings.
    pub fn new(view: &View<'_>, enemies: &BTreeMap<u32, EnemySighting>) -> Self {
        let mut ground = Self {
            threat: threat_map(view.rules, enemies),
            ..Self::default()
        };
        for e in &view.own {
            if view.is_building(e.type_id) {
                ground.occupy(&view.rules.entity_types[&e.type_id], e.pos);
            }
        }
        for enemy in enemies.values().filter(|e| e.building) {
            if let Some(et) = view.rules.entity_types.get(&enemy.type_id) {
                ground.occupy(et, enemy.pos);
            }
        }
        ground
    }

    fn occupy(&mut self, et: &EntityType, pos: Pos) {
        let (x0, y0, w, h) = footprint(et, pos);
        for y in y0 - 1..=y0 + h {
            for x in x0 - 1..=x0 + w {
                let tile = (pos.body.0, pos.face, x, y);
                self.lanes.insert(tile);
                if x >= x0 && x < x0 + w && y >= y0 && y < y0 + h {
                    self.taken.insert(tile);
                }
            }
        }
    }

    /// Keeps the footprint of a planned node gather site free of other buildings.
    pub fn reserve(&mut self, et: &EntityType, pos: Pos) {
        let (x0, y0, w, h) = footprint(et, pos);
        for y in y0..y0 + h {
            for x in x0..x0 + w {
                self.reserved.insert((pos.body.0, pos.face, x, y));
            }
        }
    }

    /// Records a building this thought has ordered, so later choices keep clear of it.
    pub fn plan(&mut self, view: &View<'_>, type_id: DefId, pos: Pos) {
        self.occupy(&view.rules.entity_types[&type_id], pos);
        self.planned
            .push((view.cat.root_of(type_id), covering_city(view, pos)));
    }

    /// Asks the simulation whether a building may stand at `pos`, within the thought's
    /// preview budget.
    fn preview(&self, view: &View<'_>, subject: DefId, pos: Pos) -> Site {
        if self.previews.get() >= PREVIEWS {
            return Site::Later;
        }
        self.previews.set(self.previews.get() + 1);
        match view.world.check_placement(view.me, subject, pos) {
            Ok(pos) => Site::At(pos),
            Err(_) => Site::Nowhere,
        }
    }

    /// Counts tile lookups; false once the thought's allowance is spent.
    fn spend(&self, lookups: usize) -> bool {
        self.lookups.set(self.lookups.get() + lookups);
        self.lookups.get() <= LOOKUPS
    }

    /// Known enemy strength in the cell of a position and the cells around it.
    pub fn danger(&self, pos: Pos) -> i64 {
        let (body, face, cx, cy) = cell_of(pos);
        (-1..=1)
            .flat_map(|dy| (-1..=1).map(move |dx| (dx, dy)))
            .filter_map(|(dx, dy)| self.threat.get(&(body, face, cx + dx, cy + dy)))
            .sum()
    }
}

/// The own complete city whose economic radius covers a position; nearest first.
fn covering_city(view: &View<'_>, pos: Pos) -> Option<EntityId> {
    view.cities
        .iter()
        .filter(|c| c.complete && c.pos.body == pos.body)
        .filter_map(|c| {
            let d = view.world.distance(c.pos, pos)?;
            (d <= c.radius.max(0) as u64).then_some((d, c.id))
        })
        .min()
        .map(|(_, id)| id)
}

/// What the placement rules of a type ask of the ground, read once per search.
struct Needs<'a> {
    et: &'a EntityType,
    /// A gather site whose slots come from node terrain: it may use reserved ground.
    node_site: bool,
    on_terrain: Vec<DefId>,
    adjacent: Vec<(DefId, i64)>,
    own_territory: bool,
    in_city: bool,
}

impl<'a> Needs<'a> {
    fn of(view: &View<'a>, subject: DefId) -> Option<Self> {
        let et = view.rules.entity_types.get(&subject)?;
        let rules = &et.placement_rules;
        Some(Self {
            et,
            node_site: et
                .gather_site
                .as_ref()
                .is_some_and(|s| s.slots_per_node > 0 && s.needs_node_terrain_id.is_some()),
            on_terrain: rules
                .iter()
                .filter(|r| r.rule == PlacementKind::OnTerrain)
                .filter_map(|r| r.ref_def_id)
                .collect(),
            adjacent: rules
                .iter()
                .filter(|r| r.rule == PlacementKind::AdjacentTerrain)
                .filter_map(|r| r.ref_def_id.map(|t| (t, r.distance)))
                .collect(),
            own_territory: rules
                .iter()
                .any(|r| r.rule == PlacementKind::InOwnTerritory),
            in_city: rules.iter().any(|r| r.rule == PlacementKind::InCityRadius),
        })
    }
}

/// The snapped centre for a building requested at `at`, if everything the player knows
/// allows it: explored buildable ground of the required terrain, own territory when the
/// type needs it, and no known building (or its lane) in the way.
fn fits(
    view: &View<'_>,
    ground: &Ground,
    needs: &Needs<'_>,
    at: Pos,
    lane: bool,
    territory: bool,
) -> Option<Pos> {
    let et = needs.et;
    let pos = Pos {
        x: snap(at.x, et.footprint_w.max(1)),
        y: snap(at.y, et.footprint_h.max(1)),
        ..at
    };
    let (x0, y0, w, h) = footprint(et, pos);
    let obstacles = if lane { &ground.lanes } else { &ground.taken };
    for y in y0..y0 + h {
        for x in x0..x0 + w {
            ground.spend(1);
            if obstacles.contains(&(pos.body.0, pos.face, x, y))
                || (!needs.node_site && ground.reserved.contains(&(pos.body.0, pos.face, x, y)))
            {
                return None;
            }
            if !view
                .world
                .is_explored_on_body(view.me, pos.body, pos.face, x, y)
            {
                return None;
            }
            let terrain = view.world.terrain_on_body(pos.body, pos.face, x, y)?;
            if !view
                .rules
                .terrains
                .get(&terrain)
                .is_some_and(|t| t.buildable)
                || needs.on_terrain.iter().any(|t| *t != terrain)
            {
                return None;
            }
            if territory
                && view.world.territory_owner_on_body(pos.body, pos.face, x, y) != Some(view.me)
            {
                return None;
            }
        }
    }
    for (terrain, distance) in &needs.adjacent {
        let reach = (distance / 1000).max(1) as i32;
        ground.spend(((w + 2 * reach) * (h + 2 * reach)) as usize);
        let near = (y0 - reach..y0 + h + reach).any(|y| {
            (x0 - reach..x0 + w + reach)
                .any(|x| view.world.terrain_on_body(pos.body, pos.face, x, y) == Some(*terrain))
        });
        if !near {
            return None;
        }
    }
    Some(pos)
}

/// Tiles of `terrain` whose centres lie within `radius` milli-tiles of `centre`.
fn count_terrain(view: &View<'_>, centre: Pos, terrain: DefId, radius: i64) -> i64 {
    let r = (radius / 1000) as i32;
    let limit = radius * radius;
    let mut count = 0;
    for dy in -r..=r {
        for dx in -r..=r {
            let (mx, my) = (i64::from(dx) * 1000, i64::from(dy) * 1000);
            if mx * mx + my * my > limit {
                continue;
            }
            let at = view.world.offset(centre, mx, my);
            if view.world.terrain_at(at) == Some(terrain) {
                count += 1;
            }
        }
    }
    count
}

/// Buildings of a line a city already holds, with those planned in this thought.
fn line_in_city(view: &View<'_>, ground: &Ground, root: DefId, city: EntityId) -> i64 {
    let standing = view
        .own
        .iter()
        .filter(|e| e.city_id == Some(city) && view.cat.root_of(e.type_id) == root)
        .count();
    let planned = ground
        .planned
        .iter()
        .filter(|(r, c)| *r == root && *c == Some(city))
        .count();
    (standing + planned) as i64
}

/// Tiles from a city centre within which a type may be placed.
fn search_radius(needs: &Needs<'_>, city: &City) -> i32 {
    let size = needs.et.footprint_w.max(needs.et.footprint_h).max(1);
    let radius = if needs.in_city {
        // All four corners of the footprint must lie inside the economic radius.
        city.radius - size * 710
    } else {
        city.border
    };
    ((radius / 1000) as i32).clamp(0, SEARCH_TILES)
}

/// A site for a building of type `subject` near one of the player's cities, or `None`.
///
/// Gather sites that take their slots from a node terrain go where the most of it is in
/// reach; everything else goes as near to a city centre as the ground allows. A free lane is
/// kept around other buildings when there is room.
pub(crate) fn find_site(view: &View<'_>, ground: &Ground, subject: DefId) -> Site {
    let Some(needs) = Needs::of(view, subject) else {
        return Site::Nowhere;
    };
    if view
        .world
        .placement_discovery_age(view.me, subject)
        .is_some()
    {
        return Site::Nowhere;
    }
    let root = view.cat.root_of(subject);
    let node = needs
        .et
        .gather_site
        .as_ref()
        .filter(|s| s.slots_per_node > 0)
        .and_then(|s| s.needs_node_terrain_id.map(|t| (t, s)));
    for lane in [true, false] {
        for city in view.cities.iter().filter(|c| c.complete) {
            if needs.et.limit_scope == LimitScope::PerCity
                && line_in_city(view, ground, root, city.id) >= needs.et.limit_count
            {
                continue;
            }
            // The building counts for the nearest city that covers it, which near another
            // city is not always the one being searched around.
            let full = |pos: Pos| {
                needs.et.limit_scope == LimitScope::PerCity
                    && covering_city(view, pos).is_some_and(|c| {
                        c != city.id && line_in_city(view, ground, root, c) >= needs.et.limit_count
                    })
            };
            let reach = search_radius(&needs, city);
            let spots = view
                .cat
                .spiral
                .iter()
                .take_while(|(dx, dy)| dx * dx + dy * dy <= reach * reach)
                .map(|(dx, dy)| {
                    view.world
                        .offset(city.pos, i64::from(*dx) * 1000, i64::from(*dy) * 1000)
                });
            if let Some((terrain, site)) = node {
                // The most node tiles in reach first, then the nearest to the city. The node
                // tiles around the city are found once; only spots near one are tried.
                let radius = site.node_radius.max(0);
                let scan = reach + (radius / 1000) as i32 + 3;
                let node_tiles: Vec<Pos> = view
                    .cat
                    .spiral
                    .iter()
                    .take_while(|(dx, dy)| dx * dx + dy * dy <= scan * scan)
                    .map(|(dx, dy)| {
                        view.world
                            .offset(city.pos, i64::from(*dx) * 1000, i64::from(*dy) * 1000)
                    })
                    .filter(|at| view.world.terrain_at(*at) == Some(terrain))
                    .collect();
                ground.spend((scan * scan * 3) as usize);
                let mut ranked: Vec<(i64, usize, Pos)> = Vec::new();
                for (order, at) in spots.enumerate() {
                    if !ground.spend(0) {
                        break;
                    }
                    let near = node_tiles
                        .iter()
                        .any(|n| view.distance(at, *n) <= radius as u64 + 3000);
                    if !near {
                        continue;
                    }
                    let Some(pos) = fits(view, ground, &needs, at, lane, needs.own_territory)
                    else {
                        continue;
                    };
                    if full(pos) {
                        continue;
                    }
                    let nodes = count_terrain(view, pos, terrain, radius);
                    ground.spend(120);
                    let slots = (site.slots_base + site.slots_per_node * nodes).min(site.slots_max);
                    if slots > 0 {
                        ranked.push((-slots, order, pos));
                    }
                    if slots >= site.slots_max {
                        // Nothing farther out can be better.
                        break;
                    }
                }
                ranked.sort_unstable_by_key(|(worth, order, _)| (*worth, *order));
                for (_, _, pos) in ranked {
                    match ground.preview(view, subject, pos) {
                        Site::Nowhere => {}
                        found => return found,
                    }
                }
            } else {
                for at in spots {
                    if !ground.spend(0) {
                        return Site::Later;
                    }
                    let Some(pos) = fits(view, ground, &needs, at, lane, needs.own_territory)
                    else {
                        continue;
                    };
                    if full(pos) {
                        continue;
                    }
                    if !needs.in_city && ground.danger(pos) > 0 && lane {
                        continue;
                    }
                    match ground.preview(view, subject, pos) {
                        Site::Nowhere => {}
                        found => return found,
                    }
                }
            }
        }
    }
    Site::Nowhere
}

/// A site for another city: inside the player's borders (or anywhere not foreign once
/// the age allows free colonisation), at least the city spacing from every own city, and
/// with the most resources and open ground around it.
pub(crate) fn find_city_site(view: &View<'_>, ground: &Ground, city_type: DefId) -> Site {
    let Some(needs) = Needs::of(view, city_type) else {
        return Site::Nowhere;
    };
    let spacing = view.rules.constant("city.spacing").unwrap_or(0);
    let free = view
        .rules
        .ages
        .get(&view.age)
        .is_some_and(|a| a.free_colonisation);
    let node_terrains: Vec<DefId> = view
        .cat
        .node_terrains
        .iter()
        .copied()
        .filter(|t| view.world.knows_terrain(view.me, *t))
        .collect();
    let mut ranked: Vec<(i64, usize, Pos)> = Vec::new();
    let mut order = 0;
    for city in view.cities.iter().filter(|c| c.complete) {
        let reach = ((city.border / 1000) as i32).clamp(0, SEARCH_TILES);
        let near = (spacing / 1000) as i32;
        for (dx, dy) in &view.cat.spiral {
            let d2 = dx * dx + dy * dy;
            if d2 > reach * reach || !ground.spend(0) {
                break;
            }
            // Cities stand a spacing apart; a lattice of every third tile is fine enough.
            if d2 <= near * near || dx.rem_euclid(3) != 0 || dy.rem_euclid(3) != 0 {
                continue;
            }
            let at = view
                .world
                .offset(city.pos, i64::from(*dx) * 1000, i64::from(*dy) * 1000);
            let Some(pos) = fits(view, ground, &needs, at, true, needs.own_territory && !free)
            else {
                continue;
            };
            if view.cities.iter().any(|c| {
                c.pos.body == pos.body && view.distance(c.pos, pos) < spacing.max(0) as u64
            }) || ground.danger(pos) > 0
            {
                continue;
            }
            order += 1;
            ranked.push((0, order, pos));
        }
    }
    // Value the surroundings of a thinned-out set of candidates only.
    let step = (ranked.len() / 24).max(1);
    let mut valued: Vec<(i64, usize, Pos)> = ranked
        .into_iter()
        .step_by(step)
        .map(|(_, order, pos)| {
            let mut value = 0;
            for dy in (-CITY_VALUE_TILES..=CITY_VALUE_TILES).step_by(2) {
                for dx in (-CITY_VALUE_TILES..=CITY_VALUE_TILES).step_by(2) {
                    let at = view
                        .world
                        .offset(pos, i64::from(dx) * 1000, i64::from(dy) * 1000);
                    let Some(terrain) = view.world.terrain_at(at) else {
                        continue;
                    };
                    if node_terrains.contains(&terrain) {
                        value += 3;
                    } else if needs.on_terrain.contains(&terrain) {
                        value += 1;
                    }
                }
            }
            (-value, order, pos)
        })
        .collect();
    valued.sort_unstable_by_key(|(value, order, _)| (*value, *order));
    for (_, _, pos) in valued {
        match ground.preview(view, city_type, pos) {
            Site::Nowhere => {}
            found => return found,
        }
    }
    Site::Nowhere
}
