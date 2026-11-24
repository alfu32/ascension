//! The naval manager (task 97). Where an enemy can be reached by water from the home shore
//! (its ships seen on the same sea, or its buildings within a warship's reach of it), the player
//! keeps a small navy: `sea.navy_share_pct` of the military target, up to twice that when the
//! enemy fleet seen is larger, none below the difficulty rank `sea.min_difficulty_rank`.
//!
//! - **Class.** The warship trained is the one that gives the best exchange for its price (the
//!   production manager's measure, at the water layer) against the enemy ships and the aircraft
//!   that strike ships it has seen — before any is seen, against one of every warship it could
//!   train itself — and, once the enemy fleet is beaten, against the shore targets: a ship that
//!   outranges a building takes no fire from it.
//! - **Missions.** Enemy ships off the own coast are met by every sound ship; with a squadron
//!   stronger than the enemy fleet by the profile's attack ratio the navy hunts the enemy ships;
//!   with the enemy fleet beaten, the ships that outrange the defences there shell the nearest
//!   shore target while the others keep the coast. Otherwise the navy waits at the harbour.
//! - **Repair.** A damaged ship goes home to the harbour; a unit that heals ships is kept there
//!   when the player can train one.
//!
//! Warships are told apart by their rows (`Catalogue::sea_units`); nothing names a definition.

use crate::{
    Weights,
    memory::{Memory, Phase, Role},
    plan::{Act, Plan, Wish},
    production::{army_target, dearness},
    view::{self, View, operational},
};
use rom_rules::{AiDifficulty, DefId, Layer, ProductionMode, Rules, Weapon};
use rom_sim::{Entity, EntityId, Pos};
use std::collections::BTreeMap;

/// A ship below this share of its hit points goes home.
const DAMAGED_PCT: i64 = 40;
/// A ship at home stays there while a healer can mend it and it is below this share.
const REPAIRED_PCT: i64 = 85;
/// Sound ships the navy keeps together before it hunts or shells.
const SQUADRON: usize = 3;
/// Ships that outrange a shore target's defences needed to shell it.
const BOMBARDERS: usize = 2;
/// How near the harbour a ship counts as home, in milli-tiles.
const HOME_SLACK: u64 = 6000;
/// How far beyond a city's border enemy ships count as off the own coast, in milli-tiles.
const COAST_MARGIN: i64 = 8000;
/// Ticks an enemy ship seen counts as still there.
const RECENT_TICKS: u64 = 400;
/// Ticks a navy pulling back takes before it gathers again.
const RETREAT_TICKS: u64 = 1200;
/// Enemy buildings looked at as shore targets in one thought, nearest first.
const SHORE_LOOKS: usize = 48;
/// Reach assumed for shore targets before the player can train a warship, in milli-tiles.
const SHORE_REACH: i64 = 8000;
/// Tiles around a dock searched for the water its ships wait on.
const HARBOUR_TILES: i32 = 5;
/// How near a goal a ship already on its way counts as going there, in milli-tiles.
const GOAL_SLACK: u64 = 3000;
/// Score of a healer for ships while a damaged one waits at home.
const HEALER_SCORE: i64 = 450;
/// Sixteen directions, × 1000.
const DIRECTIONS: [(i64, i64); 16] = [
    (1000, 0),
    (924, 383),
    (707, 707),
    (383, 924),
    (0, 1000),
    (-383, 924),
    (-707, 707),
    (-924, 383),
    (-1000, 0),
    (-924, -383),
    (-707, -707),
    (-383, -924),
    (0, -1000),
    (383, -924),
    (707, -707),
    (924, -383),
];

/// An enemy building within a warship's reach of water the navy can sail to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Shore {
    /// The building, by entity id.
    pub id: u32,
    /// Its type.
    pub type_id: DefId,
    /// Where it stands.
    pub pos: Pos,
    /// Water within reach of it, as far from it as the reach allows.
    pub station: Pos,
}

/// What the naval manager knows in one thought.
#[derive(Debug, Default)]
pub(crate) struct Sea {
    /// The player goes to sea: an enemy can be reached by water and the difficulty allows it.
    pub warranted: bool,
    /// Water beside an own dock, where ships wait.
    pub harbour: Option<Pos>,
    /// The harbour, or before a dock the water nearest the capital.
    pub home: Option<Pos>,
    /// A warship type the player's race could field: what reachability is measured for.
    pub probe: Option<DefId>,
    /// Warship types the player can train now.
    pub trainable: Vec<DefId>,
    /// Enemy warships remembered on water the navy can reach: id, type, place, tick seen.
    pub fleet: Vec<(u32, DefId, Pos, u64)>,
    /// Enemy aircraft that strike ships, by type.
    pub air: BTreeMap<DefId, i64>,
    /// Enemy buildings the navy can shell, those that fire at ships first, then cities.
    pub shore: Vec<Shore>,
    /// Population of own warships, with those in training.
    pub pop: i64,
    /// The population to keep at sea.
    pub target: i64,
}

/// Damage per second (× 100) of `attacker` against `target` standing on `layer`, from the
/// weapons that reach that layer and pass `keep`.
fn dps(
    rules: &Rules,
    attacker: DefId,
    target: DefId,
    layer: Layer,
    keep: impl Fn(&Weapon) -> bool,
) -> i64 {
    let t = &rules.entity_types[&target];
    rules.entity_types[&attacker]
        .weapons
        .iter()
        .filter(|w| !w.orbit_only && w.targets.iter().any(|l| l.layer == layer) && keep(w))
        .map(|w| {
            let pct = t
                .armor_class_id
                .map_or(100, |a| rules.damage_pct(w.damage_class_id, a));
            let hit = (w.damage * pct / 100 - t.armor).max(1);
            hit * w.shots.max(1) * 2000 / w.cooldown_ticks.max(1)
        })
        .sum()
}

/// The farthest a type's weapons that reach `layer` reach, in milli-tiles.
fn reach_at(rules: &Rules, type_id: DefId, layer: Layer) -> i64 {
    rules.entity_types[&type_id]
        .weapons
        .iter()
        .filter(|w| !w.orbit_only && w.targets.iter().any(|l| l.layer == layer))
        .map(|w| w.range_max)
        .max()
        .unwrap_or(0)
}

/// The layer a type fights on: water for a warship, the air for what flies, else the ground.
fn layer_of(view: &View<'_>, type_id: DefId) -> Layer {
    let rules = view.rules;
    if view.cat.is_sea_unit(type_id) {
        return Layer::Fluid;
    }
    let flies = rules.entity_types[&type_id]
        .movement
        .as_ref()
        .is_some_and(|m| rules.movement_classes[&m.movement_class_id].flies);
    if flies { Layer::Air } else { Layer::Ground }
}

/// How good a warship type is for its price against `foes` (type and count): the production
/// manager's exchange (damage dealt, times staying power, over damage suffered), with the ship
/// on the water. Ships fight at range in numbers, where a force's strength grows with the
/// square of its count, so damage and staying power both count per resource spent: the
/// exchange is divided by the price twice (once, as on land, the line ship came out ahead of
/// the escort against line ships, which the fight matrix measures the escort to beat at equal
/// cost). A building it outranges cannot hurt it; a mobile foe whose reach is short of a
/// weapon's least range closes in under that weapon.
pub(crate) fn value(view: &View<'_>, foes: &BTreeMap<DefId, i64>, unit: DefId) -> i64 {
    let rules = view.rules;
    let et = &rules.entity_types[&unit];
    let reach = et.weapons.iter().map(|w| w.range_max).max().unwrap_or(0);
    let staying = et.hp + reach / 50;
    let mut score: i128 = 0;
    let mut weight = 0_i64;
    for (foe, count) in foes {
        let layer = layer_of(view, *foe);
        let still = rules.entity_types[foe].movement.is_none();
        let foe_reach = rules.entity_types[foe]
            .weapons
            .iter()
            .map(|w| w.range_max)
            .max()
            .unwrap_or(0);
        let dealt = dps(rules, unit, *foe, layer, |w| {
            still || w.range_min <= foe_reach
        });
        let at_sea = reach_at(rules, *foe, Layer::Fluid);
        let suffered = if still && reach_at(rules, unit, layer) > at_sea {
            0
        } else {
            dps(rules, *foe, unit, Layer::Fluid, |w| w.range_min <= reach)
        };
        // A floor keeps a foe that cannot hit back from counting without limit.
        let floor = if still {
            0
        } else {
            crate::production::dps_against(rules, *foe, None, 0) / 4
        };
        let suffered = suffered.max(floor).max(50);
        score += i128::from(dealt) * i128::from(staying) * 1000 / i128::from(suffered)
            * i128::from(*count);
        weight += count;
    }
    let price = i128::from(dearness(view, unit).max(1));
    (score * 1000 / i128::from(weight.max(1)) / price / price).clamp(0, i128::from(i64::MAX)) as i64
}

/// The centre of the tile a position lies in.
fn tile_centre(pos: Pos) -> Pos {
    Pos {
        x: pos.x.div_euclid(1000) * 1000 + 500,
        y: pos.y.div_euclid(1000) * 1000 + 500,
        ..pos
    }
}

/// Whether the tile at a position is water a probe could sail on.
fn is_water(view: &View<'_>, probe: DefId, pos: Pos) -> bool {
    let rules = view.rules;
    let Some(class) = rules.entity_types[&probe]
        .movement
        .as_ref()
        .map(|m| m.movement_class_id)
    else {
        return false;
    };
    view.world.terrain_at(pos).is_some_and(|t| {
        rules
            .terrains
            .get(&t)
            .is_some_and(|t| t.layer == Layer::Fluid)
            && rules
                .movement_class_terrain
                .get(&class)
                .into_iter()
                .flatten()
                .any(|row| row.terrain_id == t && row.speed_pct > 0)
    })
}

/// The nearest water to `centre` within `tiles`, as a tile centre.
fn water_near(view: &View<'_>, probe: DefId, centre: Pos, tiles: i32) -> Option<Pos> {
    view.cat
        .spiral
        .iter()
        .take_while(|(dx, dy)| dx * dx + dy * dy <= tiles * tiles)
        .map(|(dx, dy)| {
            view.world
                .offset(centre, i64::from(*dx) * 1000, i64::from(*dy) * 1000)
        })
        .find(|at| is_water(view, probe, *at))
        .map(tile_centre)
}

/// Whether a ship of `probe` could sail from `from` to `to`.
fn sails(view: &View<'_>, probe: DefId, from: Pos, to: Pos) -> bool {
    view.world.reachable(probe, from, to) == Some(true)
}

/// Own producers that train warships, operational, in id order.
fn docks<'a>(view: &View<'a>) -> Vec<&'a Entity> {
    view.own
        .iter()
        .copied()
        .filter(|e| {
            operational(e)
                && view
                    .rules
                    .production
                    .get(&e.type_id)
                    .into_iter()
                    .flatten()
                    .any(|row| {
                        row.mode == ProductionMode::Train
                            && view.cat.is_sea_unit(row.product_def_id)
                    })
        })
        .collect()
}

/// Own warships that can take orders now, in id order.
pub(crate) fn ships<'a>(view: &View<'a>) -> Vec<&'a Entity> {
    view.own
        .iter()
        .copied()
        .filter(|e| {
            view.cat.is_sea_unit(e.type_id)
                && operational(e)
                && e.hp > 0
                && e.container_id.is_none()
        })
        .collect()
}

/// Fighting strength of own ships now.
fn strength_of(view: &View<'_>, ships: &[&Entity]) -> i64 {
    ships.iter().map(|e| crate::army::power(view, e)).sum()
}

/// Strength of the enemy fleet remembered.
fn fleet_strength(view: &View<'_>, sea: &Sea) -> i64 {
    sea.fleet
        .iter()
        .map(|(_, t, _, _)| view::strength(view.rules, *t))
        .sum()
}

/// Whether the enemy fleet remembered is beaten: weaker than the own sound ships by the
/// profile's attack ratio, with a squadron of them at sea.
fn fleet_beaten(view: &View<'_>, sea: &Sea, weights: &Weights, own: &[&Entity]) -> bool {
    own.len() >= SQUADRON
        && fleet_strength(view, sea) * weights.attack_ratio.max(1) / 100 <= strength_of(view, own)
}

impl Sea {
    /// Reads the sea around the player's capital, the enemy at sea and on the shore, and the
    /// navy it keeps.
    pub fn new(
        view: &View<'_>,
        memory: &Memory,
        weights: &Weights,
        difficulty: &AiDifficulty,
    ) -> Self {
        let rules = view.rules;
        let mut sea = Sea {
            pop: view
                .have
                .iter()
                .filter(|(t, _)| view.cat.is_sea_unit(**t))
                .map(|(t, n)| rules.entity_types[t].pop_cost.max(0) * n)
                .sum(),
            ..Sea::default()
        };
        if difficulty.rank < weights.sea_min_rank || weights.sea_share <= 0 {
            return sea;
        }
        let Some(capital) = view.cities.first() else {
            return sea;
        };
        sea.probe = view
            .cat
            .sea_units
            .iter()
            .copied()
            .find(|t| rules.is_available(*t, view.faction));
        let Some(probe) = sea.probe else {
            return sea;
        };
        sea.harbour = docks(view)
            .iter()
            .filter(|d| d.pos.body == capital.pos.body)
            .find_map(|d| water_near(view, probe, d.pos, HARBOUR_TILES));
        sea.home = sea.harbour.or_else(|| {
            let tiles = ((capital.border / 1000) as i32).clamp(1, crate::catalogue::SEARCH_TILES);
            water_near(view, probe, capital.pos, tiles)
        });
        let Some(home) = sea.home else {
            return sea;
        };
        let mut trainable: Vec<DefId> = view
            .owned
            .iter()
            .flat_map(|p| rules.production.get(p).into_iter().flatten())
            .filter(|row| row.mode == ProductionMode::Train)
            .map(|row| row.product_def_id)
            .filter(|t| {
                view.cat.is_sea_unit(*t)
                    && view.unlocked(*t)
                    && !view.obsolete(*t)
                    && memory
                        .blocked
                        .get(&t.0)
                        .is_none_or(|until| *until <= view.tick)
            })
            .collect();
        trainable.sort_unstable();
        trainable.dedup();
        sea.trainable = trainable;
        for (id, e) in &memory.enemies {
            if e.building || e.pos.body != home.body {
                continue;
            }
            if view.cat.is_sea_unit(e.type_id) {
                if sails(view, probe, home, e.pos) {
                    sea.fleet.push((*id, e.type_id, e.pos, e.tick));
                }
            } else if layer_of(view, e.type_id) == Layer::Air
                && reach_at(rules, e.type_id, Layer::Fluid) > 0
            {
                *sea.air.entry(e.type_id).or_default() += 1;
            }
        }
        // Shore targets: the reach of the ships the player can train, or a guess before.
        let reach = sea
            .trainable
            .iter()
            .map(|t| reach_at(rules, *t, Layer::Ground))
            .max()
            .unwrap_or(SHORE_REACH)
            .max(2000);
        let mut near: Vec<(u64, u32)> = memory
            .enemies
            .iter()
            .filter(|(_, e)| e.building && e.pos.body == home.body)
            .map(|(id, e)| (view.distance(home, e.pos), *id))
            .collect();
        near.sort_unstable();
        let rank = |t: DefId| {
            if reach_at(rules, t, Layer::Fluid) > 0 {
                0
            } else if view.is_city(t) {
                1
            } else {
                2
            }
        };
        let mut shore: Vec<(i64, u64, Shore)> = Vec::new();
        for (distance, id) in near.into_iter().take(SHORE_LOOKS) {
            let e = memory.enemies[&id];
            let station = station(view, probe, home, e.pos, reach);
            if let Some(station) = station {
                shore.push((
                    rank(e.type_id),
                    distance,
                    Shore {
                        id,
                        type_id: e.type_id,
                        pos: e.pos,
                        station,
                    },
                ));
            }
        }
        shore.sort_by_key(|(rank, distance, s)| (*rank, *distance, s.id));
        sea.shore = shore.into_iter().map(|(_, _, s)| s).collect();
        sea.warranted = !sea.fleet.is_empty() || !sea.shore.is_empty();
        if sea.warranted {
            let army = army_target(view, weights, difficulty, memory);
            let share = army * weights.sea_share / 100;
            let fleet: i64 = sea
                .fleet
                .iter()
                .map(|(_, t, _, _)| rules.entity_types[t].pop_cost.max(0))
                .sum();
            // Twice the share at most: an arms race at sea leaves no army to win on land.
            let answer = (fleet * weights.attack_ratio / 100).min(share * 2);
            sea.target = share.max(answer);
        }
        sea
    }
}

/// Water the navy can sail to within `reach` of `at`, as far out as the reach allows.
fn station(view: &View<'_>, probe: DefId, home: Pos, at: Pos, reach: i64) -> Option<Pos> {
    let reach = reach - 1000;
    [reach, reach * 3 / 4, reach / 2, 2000]
        .into_iter()
        .filter(|r| *r >= 1000)
        .find_map(|r| {
            DIRECTIONS.iter().find_map(|(dx, dy)| {
                let p = view.world.offset(at, dx * r / 1000, dy * r / 1000);
                (is_water(view, probe, p) && sails(view, probe, home, p)).then(|| tile_centre(p))
            })
        })
}

/// The foes a warship is chosen against: the enemy fleet and the aircraft that strike ships,
/// or before any is seen one of every warship the player could train; with the enemy fleet
/// beaten, the shore targets.
fn foes(view: &View<'_>, sea: &Sea, weights: &Weights, own: &[&Entity]) -> BTreeMap<DefId, i64> {
    let mut foes: BTreeMap<DefId, i64> = BTreeMap::new();
    if fleet_beaten(view, sea, weights, own) && !sea.shore.is_empty() {
        for s in sea.shore.iter().take(SQUADRON * 4) {
            *foes.entry(s.type_id).or_default() += 1;
        }
        return foes;
    }
    for (_, t, _, _) in &sea.fleet {
        *foes.entry(*t).or_default() += 1;
    }
    for (t, n) in &sea.air {
        *foes.entry(*t).or_default() += n;
    }
    if foes.is_empty() {
        for t in &sea.trainable {
            foes.insert(*t, 1);
        }
    }
    foes
}

/// The warship type to train next.
fn choice(view: &View<'_>, sea: &Sea, weights: &Weights) -> Option<DefId> {
    let own = ships(view);
    let foes = foes(view, sea, weights, &own);
    sea.trainable
        .iter()
        .copied()
        .max_by_key(|t| (value(view, &foes, *t), std::cmp::Reverse(*t)))
}

/// A type that heals ships and can reach them on the water, trainable now.
fn healer(view: &View<'_>, sea: &Sea) -> Option<DefId> {
    let rules = view.rules;
    let ship = *sea.trainable.first()?;
    view.owned
        .iter()
        .flat_map(|p| rules.production.get(p).into_iter().flatten())
        .filter(|row| row.mode == ProductionMode::Train)
        .map(|row| row.product_def_id)
        .filter(|t| {
            let et = &rules.entity_types[t];
            et.healer
                .as_ref()
                .is_some_and(|h| rules.has_tag(ship, h.target_tag_id))
                && et.movement.as_ref().is_some_and(|m| {
                    rules.movement_classes[&m.movement_class_id].flies || view.cat.is_sea_unit(*t)
                })
                && view.unlocked(*t)
                && !view.obsolete(*t)
        })
        .min()
}

/// Whether a ship is damaged enough to go home.
fn damaged(e: &Entity) -> bool {
    e.hp * 100 < e.max_hp * DAMAGED_PCT
}

/// Wishes of the naval manager: warships while the navy is below its share, and a healer
/// while a damaged ship waits at home.
pub(crate) fn wishes(view: &View<'_>, weights: &Weights, sea: &Sea, wishes: &mut Vec<Wish>) {
    if !sea.warranted {
        return;
    }
    if sea.pop < sea.target
        && let Some(ship) = choice(view, sea, weights)
    {
        let coast = !off_the_coast(view, sea).is_empty();
        let score = if coast {
            720
        } else {
            430 + weights.army_share + ((sea.target - sea.pop) * 8).min(120)
        };
        wishes.push(Wish {
            score,
            act: Act::Train(ship),
            reserve: false,
            opening: false,
        });
    }
    if let (Some(home), Some(mender)) = (sea.home, healer(view, sea))
        && view.count(mender) == 0
        && ships(view)
            .iter()
            .any(|e| damaged(e) && view.distance(e.pos, home) <= HOME_SLACK)
    {
        wishes.push(Wish {
            score: HEALER_SCORE,
            act: Act::Train(mender),
            reserve: false,
            opening: false,
        });
    }
}

/// Enemy ships seen lately off the own coast: within a city's border and a margin.
fn off_the_coast(view: &View<'_>, sea: &Sea) -> Vec<(u32, Pos)> {
    sea.fleet
        .iter()
        .filter(|(_, _, pos, tick)| {
            tick + RECENT_TICKS >= view.tick
                && view.cities.iter().any(|c| {
                    c.pos.body == pos.body
                        && view.distance(c.pos, *pos)
                            <= (c.border.max(c.radius) + COAST_MARGIN) as u64
                })
        })
        .map(|(id, _, pos, _)| (*id, *pos))
        .collect()
}

/// Whether a ship's first order already takes it to `goal` (or fights on the way there).
fn going(e: &Entity, kinds: &[&str], goal: Pos, slack: u64, view: &View<'_>) -> bool {
    e.orders().front().is_some_and(|o| {
        kinds.contains(&o.kind.as_str())
            && o.target_pos
                .is_some_and(|p| view.distance(p, goal) <= slack)
    }) || e
        .orders()
        .front()
        .is_some_and(|o| o.automatic && o.kind == "attack")
}

/// Sends ships toward a point they can sail to with orders of `kind`, those not on their
/// way already. Returns false when the order budget ran out.
fn send(plan: &mut Plan<'_>, probe: DefId, ships: &[&Entity], goal: Pos, kind: &str) -> bool {
    let mut orders: Vec<EntityId> = Vec::new();
    for e in ships {
        let view = &plan.view;
        if view.busy.contains(&e.id)
            || view.distance(e.pos, goal) <= GOAL_SLACK
            || going(e, &["move", "attack_move"], goal, GOAL_SLACK, view)
            || !sails(view, probe, e.pos, goal)
        {
            continue;
        }
        orders.push(e.id);
    }
    for id in orders {
        if !plan.order(id, kind, None, None, Some(goal)) {
            return false;
        }
    }
    true
}

/// The ships of `own` that outrange the defences around a shore target: their reach at the
/// ground beyond the farthest any armed enemy building near it fires at ships.
fn bombarders<'a>(
    view: &View<'_>,
    memory: &Memory,
    own: &[&'a Entity],
    target: &Shore,
) -> Vec<&'a Entity> {
    let rules = view.rules;
    let defence = memory
        .enemies
        .values()
        .filter(|e| e.building && e.pos.body == target.pos.body)
        .map(|e| (reach_at(rules, e.type_id, Layer::Fluid), e.pos))
        .filter(|(reach, pos)| *reach > 0 && view.distance(*pos, target.pos) <= 2 * *reach as u64)
        .map(|(reach, _)| reach)
        .max()
        .unwrap_or(0);
    own.iter()
        .copied()
        .filter(|e| reach_at(rules, e.type_id, Layer::Ground) > defence)
        .collect()
}

/// Leads the navy for one thought.
pub(crate) fn command(
    plan: &mut Plan<'_>,
    memory: &mut Memory,
    sea: &Sea,
    weights: &Weights,
    difficulty: &AiDifficulty,
) {
    let all = ships(&plan.view);
    let mut army = memory.armies.remove(&Role::Navy).unwrap_or_default();
    army.members = all.iter().map(|e| e.id.0).collect();
    let (Some(home), Some(probe)) = (sea.harbour.or(sea.home), sea.probe) else {
        memory.armies.insert(Role::Navy, army);
        return;
    };
    if all.is_empty() {
        memory.armies.insert(Role::Navy, army);
        return;
    }
    let tick = plan.view.tick;
    let mender = healer(&plan.view, sea);
    // Damaged ships go home; at home they stay while something can mend them.
    let mendable = |e: &Entity| {
        mender.is_some()
            && e.hp * 100 < e.max_hp * REPAIRED_PCT
            && plan.view.distance(e.pos, home) <= HOME_SLACK
    };
    let (hurt, sound): (Vec<&Entity>, Vec<&Entity>) =
        all.iter().copied().partition(|e| damaged(e) || mendable(e));
    if !send(plan, probe, &hurt, home, "move") {
        memory.armies.insert(Role::Navy, army);
        return;
    }
    mend(plan, &hurt, home);
    let strength = strength_of(&plan.view, &sound);
    // A beaten navy pulls back and gathers again at home.
    if army.phase == Phase::Advance
        && difficulty.retreats
        && strength * 100 < army.peak * weights.retreat_ratio.clamp(1, 100) / 100
    {
        army.phase = Phase::Retreat;
        army.target = None;
        army.goal = Some(home);
        army.since = tick;
    }
    if army.phase == Phase::Retreat {
        if tick < army.since + RETREAT_TICKS {
            send(plan, probe, &sound, home, "move");
            memory.armies.insert(Role::Navy, army);
            return;
        }
        army.phase = Phase::Gather;
        army.goal = None;
    }
    let coast = off_the_coast(&plan.view, sea);
    let grace = tick < difficulty.grace_ticks.max(0) as u64;
    let beaten = fleet_beaten(&plan.view, sea, weights, &sound);
    let hunt = sound.len() >= SQUADRON
        && !sea.fleet.is_empty()
        && strength * 100 >= fleet_strength(&plan.view, sea) * weights.attack_ratio.max(1);
    let (phase, target, goal) = if let Some((id, at)) = coast
        .iter()
        .min_by_key(|(id, at)| (plan.view.distance(home, *at), *id))
    {
        (Phase::Respond, Some(*id), Some(*at))
    } else if hunt && !grace {
        let (id, _, at, _) = sea
            .fleet
            .iter()
            .min_by_key(|(id, _, at, _)| (plan.view.distance(home, *at), *id))
            .copied()
            .expect("a fleet to hunt");
        (Phase::Advance, Some(id), Some(at))
    } else if beaten && !grace {
        match sea
            .shore
            .iter()
            .find(|s| {
                memory
                    .unreachable
                    .get(&s.id)
                    .is_none_or(|until| *until <= tick)
            })
            .filter(|s| bombarders(&plan.view, memory, &sound, s).len() >= BOMBARDERS)
        {
            Some(s) => (Phase::Advance, Some(s.id), Some(s.station)),
            None => (Phase::Gather, None, None),
        }
    } else {
        (Phase::Gather, None, None)
    };
    if phase != army.phase || target != army.target {
        if phase == Phase::Advance && army.phase != Phase::Advance {
            army.peak = strength;
        }
        army.phase = phase;
        army.target = target;
        army.since = tick;
    }
    army.goal = goal;
    match (phase, goal) {
        (Phase::Respond, Some(at)) => {
            send(plan, probe, &sound, at, "attack_move");
        }
        (Phase::Advance, Some(at)) if sea.fleet.iter().any(|(id, ..)| Some(*id) == target) => {
            send(plan, probe, &sound, at, "attack_move");
        }
        (Phase::Advance, Some(station)) => {
            let shore = sea
                .shore
                .iter()
                .find(|s| Some(s.id) == target)
                .copied()
                .expect("the shore target");
            let guns = bombarders(&plan.view, memory, &sound, &shore);
            let others: Vec<&Entity> = sound
                .iter()
                .copied()
                .filter(|e| !guns.iter().any(|g| g.id == e.id))
                .collect();
            shell(plan, memory, probe, &guns, &shore, station);
            send(plan, probe, &others, home, "move");
        }
        _ => {
            send(plan, probe, &sound, home, "move");
        }
    }
    memory.armies.insert(Role::Navy, army);
}

/// Ships shelling a shore target: on their way to the station they fight what they meet;
/// within reach of the target and seeing it, they fire on it.
fn shell(
    plan: &mut Plan<'_>,
    memory: &Memory,
    probe: DefId,
    guns: &[&Entity],
    target: &Shore,
    station: Pos,
) {
    let view = &plan.view;
    let seen = memory
        .enemies
        .get(&target.id)
        .is_some_and(|e| e.tick == view.tick)
        && view
            .world
            .entity(EntityId(target.id))
            .is_some_and(|e| e.hp > 0);
    let mut fire: Vec<EntityId> = Vec::new();
    let mut sail: Vec<&Entity> = Vec::new();
    for e in guns {
        let reach = reach_at(view.rules, e.type_id, Layer::Ground) as u64;
        let firing = e
            .orders()
            .front()
            .is_some_and(|o| o.kind == "attack" && o.target_entity == Some(EntityId(target.id)));
        if firing || view.busy.contains(&e.id) {
            continue;
        }
        if seen && view.distance(e.pos, target.pos) <= reach + GOAL_SLACK {
            fire.push(e.id);
        } else {
            sail.push(e);
        }
    }
    for id in fire {
        if !plan.order(id, "attack", None, Some(EntityId(target.id)), None) {
            return;
        }
    }
    send(plan, probe, &sail, station, "attack_move");
}

/// Healers of ships at home mend the most damaged ship there, or wait at the harbour.
fn mend(plan: &mut Plan<'_>, hurt: &[&Entity], home: Pos) {
    let view = &plan.view;
    let rules = view.rules;
    let menders: Vec<&Entity> = view
        .own
        .iter()
        .copied()
        .filter(|e| {
            operational(e)
                && e.container_id.is_none()
                && e.orders().is_empty()
                && !view.busy.contains(&e.id)
                && rules.entity_types[&e.type_id]
                    .healer
                    .as_ref()
                    .is_some_and(|h| {
                        hurt.iter()
                            .any(|s| rules.has_tag(s.type_id, h.target_tag_id))
                    })
                // Only a mender that reaches ships on the water: not a supply cart (task 140).
                && rules.entity_types[&e.type_id].movement.as_ref().is_some_and(|m| {
                    rules.movement_classes[&m.movement_class_id].flies
                        || view.cat.is_sea_unit(e.type_id)
                })
        })
        .collect();
    let mut orders: Vec<(EntityId, &str, Option<EntityId>, Option<Pos>)> = Vec::new();
    for m in menders {
        let patient = hurt
            .iter()
            .filter(|s| view.distance(s.pos, home) <= HOME_SLACK)
            .min_by_key(|s| (s.hp * 100 / s.max_hp.max(1), s.id))
            .map(|s| s.id);
        match patient {
            Some(p) => orders.push((m.id, "heal", Some(p), None)),
            None if view.distance(m.pos, home) > HOME_SLACK => {
                orders.push((m.id, "move", None, Some(home)));
            }
            None => {}
        }
    }
    for (id, kind, target, at) in orders {
        if !plan.order(id, kind, None, target, at) {
            return;
        }
    }
}
