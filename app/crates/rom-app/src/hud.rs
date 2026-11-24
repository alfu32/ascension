//! Convert player one's world state to plain interface data, without graphics calls.
use crate::sim_thread::TICKS_PER_SECOND;
use rom_render::{
    ButtonState, HudResearch, HudResource, HudSnapshot, ResearchCell, ResearchOverview,
    ResearchTrack,
};
use rom_rules::{DefId, ResourceKind, Rules, TechEffectKind, TechKind};
use rom_sim::{OrderStatus, PlayerId, TradeRoute, World};

/// Resource artwork is presentation metadata keyed by the database abbreviation.
pub(crate) fn resource_icon(rules: &Rules, id: DefId) -> String {
    RESOURCE_ICONS
        .iter()
        .find(|(abbr, _)| *abbr == rules.resources[&id].abbrev)
        .map_or("icons/help".into(), |(_, name)| format!("icons/res_{name}"))
}

/// The colour of a base resource for markers in space: presentation metadata keyed by the
/// database abbreviation, like [`resource_icon`].
pub(crate) fn resource_colour(rules: &Rules, id: DefId) -> [u8; 3] {
    RESOURCE_COLOURS
        .iter()
        .find(|(abbr, _)| *abbr == rules.resources[&id].abbrev)
        .map_or([190, 190, 200], |(_, rgb)| *rgb)
}

/// Public entity queues omit submissions awaiting their first tick; the controller
/// supplies those locally until the queue becomes visible.
pub(crate) fn button_state(
    world: &World,
    player_id: PlayerId,
    def: DefId,
    pending: bool,
) -> ButtonState {
    let rules = world.rules();
    let player = world.players().find(|p| p.id == player_id).expect("player");
    if rules.techs.get(&def).is_some_and(|t| !t.repeatable)
        && player.techs.get(&def).is_some_and(|n| *n > 0)
    {
        return ButtonState::Done;
    }
    // An age advance to an age the player has reached is done, however it was reached.
    if rules
        .techs
        .get(&def)
        .is_some_and(|t| t.kind == TechKind::Age)
        && rules.tech_effects.get(&def).into_iter().flatten().any(|e| {
            e.effect == TechEffectKind::SetAge
                && e.target_def_id
                    .is_some_and(|id| rules.ages[&id].ordinal <= rules.ages[&player.age].ordinal)
        })
    {
        return ButtonState::Done;
    }
    if let Some(order) = world
        .entities()
        .filter(|e| e.owner == player.id)
        .flat_map(|e| e.orders())
        .find(|o| o.kind == "research" && o.subject == Some(def))
    {
        return ButtonState::InProgress {
            pct: (u128::from(order.progress_ticks) * 100
                / u128::from(order.total_ticks.unwrap_or(1).max(1)))
            .min(100) as u32,
        };
    }
    if pending {
        return ButtonState::InProgress { pct: 0 };
    }
    if !rules.is_available(def, player.faction) {
        return ButtonState::Locked {
            reason: "Unavailable to your faction".into(),
        };
    }
    if !rules.is_unlocked(def, &player.active_sources(), &world.owned_types(player.id)) {
        return ButtonState::Locked {
            reason: format!("Needs {}", unmet(world, player_id, def)),
        };
    }
    // The ascension waits for everything else at the library (task 76).
    if rules.ascends(def) && !world.ascension_ready(player.id) {
        let left = rules
            .ascension_missing(player.faction, player.age, &player.techs)
            .len();
        return ButtonState::Locked {
            reason: format!("Needs every library technology ({left} left)"),
        };
    }
    let missing: Vec<_> = world
        .price(player.id, def)
        .into_iter()
        .filter_map(|(r, amount)| {
            let short = amount - player.resources.get(&r).copied().unwrap_or(0);
            if short > 0 {
                Some((resource_icon(rules, r), short))
            } else {
                None
            }
        })
        .collect();
    if missing.is_empty() {
        ButtonState::Ready
    } else {
        ButtonState::Unaffordable { missing }
    }
}

/// First unmet OR-group, excluding alternatives of groups that are already met.
pub(crate) fn unmet(world: &World, player_id: PlayerId, def: DefId) -> String {
    let rules = world.rules();
    let player = world.players().find(|p| p.id == player_id).expect("player");
    let active = player.active_sources();
    let owned = world.owned_types(player.id);
    let mut groups = std::collections::BTreeMap::<_, Vec<_>>::new();
    for row in rules.prerequisites.get(&def).into_iter().flatten() {
        groups
            .entry(row.alt_group)
            .or_default()
            .push(row.requires_def_id);
    }
    let met = |id: &DefId| {
        if let Some(age) = rules.ages.get(id) {
            rules.ages[&player.age].ordinal >= age.ordinal
        } else if rules.techs.contains_key(id) {
            active.contains(id)
        } else {
            owned.contains(id)
        }
    };
    groups
        .values()
        .find(|ids| !ids.iter().any(met))
        .map_or("a prerequisite".into(), |ids| {
            ids.iter()
                .map(|id| rules.display_name(*id, player.faction))
                .collect::<Vec<_>>()
                .join(" or ")
        })
}

/// Whether a producer should show this technology as the next step in its track or age.
pub(crate) fn next_research(world: &World, player_id: PlayerId, def: DefId) -> bool {
    let rules = world.rules();
    let player = world.players().find(|p| p.id == player_id).expect("player");
    let tech = &rules.techs[&def];
    let done = |id: &DefId| player.techs.get(id).is_some_and(|n| *n > 0);
    if !tech.repeatable && done(&def) {
        return false;
    }
    match tech.kind {
        TechKind::Track => !rules.techs.values().any(|t| {
            t.track == tech.track
                && t.kind == TechKind::Track
                && t.level < tech.level
                && !done(&t.id)
        }),
        TechKind::Age => rules.tech_effects.get(&def).into_iter().flatten().any(|e| {
            e.effect == TechEffectKind::SetAge
                && e.target_def_id.is_some_and(|id| {
                    rules.ages[&id].ordinal == rules.ages[&player.age].ordinal + 1
                })
        }),
        _ => true,
    }
}

/// Completed levels, queued research and the next step of each track and age.
pub(crate) fn research_overview(
    world: &World,
    player_id: PlayerId,
    pending: &[DefId],
) -> ResearchOverview {
    let rules = world.rules();
    let player = world.players().find(|p| p.id == player_id).expect("player");
    let done = |id: &DefId| player.techs.get(id).is_some_and(|n| *n > 0);
    let queued: std::collections::BTreeSet<_> = world
        .entities()
        .filter(|e| e.owner == player.id)
        .flat_map(|e| e.orders())
        .filter(|o| o.kind == "research")
        .filter_map(|o| o.subject)
        .chain(pending.iter().copied())
        .collect();
    let mut tracks = std::collections::BTreeMap::<_, Vec<_>>::new();
    for tech in rules.techs.values().filter(|t| t.kind == TechKind::Track) {
        tracks.entry(&tech.track).or_default().push(tech);
    }
    let tracks = tracks
        .into_iter()
        .map(|(label, mut techs)| {
            techs.sort_by_key(|t| (t.level, t.id));
            let next = techs.iter().find(|t| !done(&t.id)).map(|t| t.id);
            ResearchTrack {
                label: label.to_uppercase(),
                levels: techs
                    .iter()
                    .map(|t| {
                        if done(&t.id) {
                            ResearchCell::Done
                        } else if queued.contains(&t.id) {
                            ResearchCell::InProgress
                        } else if Some(t.id) == next {
                            ResearchCell::Next
                        } else {
                            ResearchCell::Locked
                        }
                    })
                    .collect(),
            }
        })
        .collect();
    let current_age = rules.ages[&player.age].ordinal;
    let mut ages: Vec<_> = rules.ages.values().collect();
    ages.sort_by_key(|a| (a.ordinal, a.id));
    ResearchOverview {
        tracks,
        ages: ages
            .iter()
            .map(|a| {
                if a.ordinal <= current_age {
                    return ResearchCell::Done;
                }
                let in_progress = queued.iter().any(|id| {
                    rules.tech_effects.get(id).into_iter().flatten().any(|e| {
                        e.effect == TechEffectKind::SetAge && e.target_def_id == Some(a.id)
                    })
                });
                if in_progress {
                    ResearchCell::InProgress
                } else if a.ordinal == current_age + 1 {
                    ResearchCell::Next
                } else {
                    ResearchCell::Locked
                }
            })
            .collect(),
    }
}

// Presentation metadata only, keyed by the database resource abbreviation.
const RESOURCE_COLOURS: &[(&str, [u8; 3])] = &[
    ("F", [120, 200, 80]),
    ("T", [170, 115, 60]),
    ("M", [175, 190, 210]),
    ("W", [240, 205, 80]),
    ("K", [130, 170, 245]),
    ("O", [230, 140, 50]),
    ("E", [90, 225, 240]),
];

const RESOURCE_ICONS: &[(&str, &str)] = &[
    ("F", "food"),
    ("T", "timber"),
    ("M", "metal"),
    ("W", "wealth"),
    ("K", "knowledge"),
    ("O", "oil"),
    ("E", "energy"),
];

/// The player's route slots after active modifiers.
pub(crate) fn caravan_limit(world: &World, player: PlayerId) -> i64 {
    let rules = world.rules();
    world.players().find(|p| p.id == player).map_or(0, |p| {
        rules.player_stat(
            "caravan_limit",
            rules.constant("player.caravan_limit.base").unwrap_or(0),
            &p.active_sources(),
        )
    })
}

/// Keeper income shares or the trader's accepted city names and payment.
/// The name of the technology that enables the rule `key` (`tech_effect.rule_key`), for help
/// and refusal lines; "a research" when no technology does.
pub(crate) fn rule_tech_name(world: &World, key: &str) -> String {
    let rules = world.rules();
    rules
        .tech_effects
        .iter()
        .find(|(_, effects)| {
            effects
                .iter()
                .any(|e| e.effect == rom_rules::TechEffectKind::EnableRule && e.rule_key == key)
        })
        .map_or_else(
            || "a research".to_owned(),
            |(tech, _)| rules.def(*tech).name.clone(),
        )
}

/// "Path: Vega → Tarf → Sirius, 2 jumps (1 to chart)" for a trade route between cities of two
/// star systems: every system the freighter passes, in order.
fn interstellar_line(
    world: &World,
    player: rom_sim::PlayerId,
    home: rom_sim::EntityId,
    target: rom_sim::EntityId,
) -> Option<String> {
    let system = |id| {
        world
            .entity(id)
            .and_then(|e| world.body(e.pos.body))
            .map(|b| b.system)
    };
    let (a, b) = (system(home)?, system(target)?);
    if a == b {
        return None;
    }
    let path = world.travel_path(player, a, b)?;
    let jumps = path.len() - 1;
    let unknown = path
        .windows(2)
        .filter(|leg| world.charting(player, leg[0], leg[1]))
        .count();
    let names: Vec<String> = path.iter().map(|id| system_name(world, *id)).collect();
    Some(format!(
        "Path: {}, {jumps} jump{}{}",
        names.join(" → "),
        if jumps == 1 { "" } else { "s" },
        if unknown > 0 {
            format!(" ({unknown} to chart)")
        } else {
            String::new()
        }
    ))
}

/// The name of star system `id`.
fn system_name(world: &World, id: u32) -> String {
    world
        .systems()
        .iter()
        .find(|s| s.id == id)
        .map_or_else(|| format!("system {id}"), |s| s.name.clone())
}

/// A city by its name (task 144); "City 3", numbered by type in id order, for one without a
/// name; "City lost" when it is gone.
pub(crate) fn city_label(world: &World, id: rom_sim::EntityId) -> String {
    if let Some(name) = world.city_name(id) {
        return name.to_owned();
    }
    let rules = world.rules();
    world.entity(id).map_or_else(
        || "City lost".into(),
        |e| {
            let faction = world
                .players()
                .find(|p| p.id == e.owner)
                .map_or(DefId(0), |p| p.faction);
            let number = world
                .entities()
                .filter(|c| c.type_id == e.type_id && c.id <= e.id)
                .count();
            format!("{} {number}", rules.display_name(e.type_id, faction))
        },
    )
}

/// What a space freighter on its route does next, from the leg in front of its trade order:
/// "Next: land beside City 3", "Next: jump toward Tarf" and so on.
pub(crate) fn next_leg(
    world: &World,
    entity: &rom_sim::Entity,
    route: &TradeRoute,
) -> Option<String> {
    let leg = entity.orders().front()?;
    let city_on = |body: rom_sim::BodyId| {
        [route.target, route.home]
            .into_iter()
            .find(|c| world.entity(*c).is_some_and(|e| e.pos.body == body))
    };
    let at = leg.target_pos;
    let text = match leg.kind.as_str() {
        "lift" => "lift off".to_owned(),
        "travel" => {
            let system = at.and_then(|p| world.body(p.body)).map(|b| b.system)?;
            format!("jump toward {}", system_name(world, system))
        }
        "orbit" => format!(
            "fly to the planet of {}",
            city_label(world, city_on(at?.body)?)
        ),
        "land" => format!("land beside {}", city_label(world, city_on(at?.body)?)),
        "trade" if world.is_landed(entity.id) => "lift off".to_owned(),
        _ => return None,
    };
    Some(format!("Next: {text}"))
}

/// An automated colony's panel: each kind of site against its cap, then whether its governor
/// runs and what it waits for ("Needs 200 Metal"). Two lines; empty for anything else.
pub(crate) fn colony_lines(world: &World, entity: &rom_sim::Entity) -> Vec<String> {
    let Some(status) = rom_ai::colony_status(world, entity.id) else {
        return Vec::new();
    };
    let rules = world.rules();
    let faction = world
        .players()
        .find(|p| p.id == entity.owner)
        .map_or(entity.type_id, |p| p.faction);
    // The last word of a site's name is enough beside its count ("Farm 5/5").
    let short = |def| {
        let name = rules.display_name(def, faction);
        name.rsplit(' ').next().unwrap_or(name).to_owned()
    };
    let counts = status
        .sites
        .iter()
        .map(|s| format!("{} {}/{}", short(s.site_type), s.count, s.cap))
        .collect::<Vec<_>>()
        .join(" · ");
    let resource = |id| rules.def(id).name.clone();
    let doing = match &status.waiting {
        rom_ai::Waiting::Nothing => "every site stands".to_owned(),
        rom_ai::Waiting::Paused => String::new(),
        rom_ai::Waiting::Colony => "waits for the colony".to_owned(),
        rom_ai::Waiting::Building(def) => format!("building {}", short(*def)),
        rom_ai::Waiting::Locked(def) => format!("{} not unlocked", short(*def)),
        rom_ai::Waiting::Placing(def) => format!("placing {}", short(*def)),
        rom_ai::Waiting::Resources(_, missing) => format!(
            "needs {}",
            missing
                .iter()
                .map(|(r, n)| format!("{n} {}", resource(*r)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    };
    let state = if status.paused {
        "Governor paused".to_owned()
    } else {
        format!("Governor on: {doing}")
    };
    vec![counts, state]
}

/// The help line of a selected market (task 107): how buying and selling are priced.
pub(crate) fn market_help(world: &World, player: PlayerId) -> String {
    if !world.can_trade(player) {
        return format!(
            "Trading at the market needs {}",
            rule_tech_name(world, rom_sim::RULE_MARKET_TRADING)
        );
    }
    let spread = world.market_spread(player).unwrap_or(0);
    let step = world.rules().constant("market.price_step_pct").unwrap_or(0);
    format!(
        "Buy {}% over the price, sell {}% under; a lot moves it {step}%, then it drifts back",
        (spread + 1) / 2,
        spread / 2
    )
}

/// A market's card lines (task 107): what one lot of each resource costs and pays now.
pub(crate) fn market_lines(world: &World, entity: &rom_sim::Entity) -> Vec<String> {
    let rules = world.rules();
    let Some(currency) = world.market_currency(entity.id) else {
        return Vec::new();
    };
    let quotes: Vec<_> = world
        .tradeable_resources(entity.owner)
        .into_iter()
        .filter(|r| *r != currency)
        .filter_map(|r| world.market_quote(entity.owner, r))
        .collect();
    let Some(lot) = quotes.first().map(|q| q.lot) else {
        return Vec::new();
    };
    let line = |label: &str, price: fn(&rom_sim::MarketQuote) -> i64| {
        let prices = quotes
            .iter()
            .map(|q| format!("{} {}", rules.def(q.resource).name, price(q)))
            .collect::<Vec<_>>()
            .join(" · ");
        format!("{label} {lot}: {prices}")
    };
    vec![line("Buy", |q| q.buy), line("Sell", |q| q.sell)]
}

pub(crate) fn wealth_card(world: &World, entity: &rom_sim::Entity) -> Vec<String> {
    let rules = world.rules();
    let et = &rules.entity_types[&entity.type_id];
    if let Some(keeper) = &et.keeper {
        let mut raw = std::collections::BTreeMap::<_, i64>::new();
        let mut kept = 0;
        for node in world.kept_nodes(entity.id) {
            if let Some(n) = &rules.entity_types[&node.type_id].node {
                kept += 1;
                *raw.entry(n.resource_id).or_default() += n.income;
            }
        }
        let income: i64 = if entity.build_progress.is_none()
            && !entity.neutralised
            && entity.assimilation_until.is_none()
        {
            raw.into_iter()
                .map(|(resource, amount)| {
                    let total = world.income(entity.owner, resource);
                    (i128::from(total.effective) * i128::from(amount)
                        / i128::from(total.raw.max(1))) as i64
                })
                .sum()
        } else {
            0
        };
        let seconds = rules.constant("economy.income_period_ticks").unwrap_or(600)
            / i64::from(TICKS_PER_SECOND);
        return vec![
            format!("Relics {kept} / {}", keeper.capacity),
            format!("+{income} wealth per {seconds} s"),
        ];
    }
    if et.trader.as_ref().is_some_and(|t| t.runs_routes) {
        if let Some(route) = world
            .trade_routes(entity.owner)
            .find(|r| r.trader == entity.id)
        {
            let mut lines = vec![format!(
                "Route: {} → {}, +{} per trip",
                city_label(world, route.home),
                city_label(world, route.target),
                route.per_trip_income
            )];
            lines.extend(interstellar_line(
                world,
                entity.owner,
                route.home,
                route.target,
            ));
            lines.extend(next_leg(world, entity, &route));
            return lines;
        }
        return vec!["No route".into()];
    }
    Vec::new()
}

/// Reads stock, effective income per configured income period, caps and active research.
pub fn snapshot_of(world: &World, player_id: PlayerId) -> HudSnapshot {
    let rules = world.rules();
    let mut result = HudSnapshot {
        game_ticks: world.tick(),
        ticks_per_second: TICKS_PER_SECOND,
        ..HudSnapshot::default()
    };
    let Some(player) = world.players().find(|p| p.id == player_id) else {
        return result;
    };
    result.wealth = rom_render::HudWealth {
        relics: world
            .entities()
            .filter(|e| e.owner == player.id && rules.entity_types[&e.type_id].keeper.is_some())
            .map(|e| world.kept_nodes(e.id).count())
            .sum(),
        routes: world.trade_routes(player.id).count(),
        limit: caravan_limit(world, player.id),
        per_trip: world
            .trade_routes(player.id)
            .map(|r| r.per_trip_income)
            .sum(),
        per_period: 0,
    };
    let sources = player.active_sources();
    let ordinal = rules.ages[&player.age].ordinal;
    result.resources = rules
        .resources
        .values()
        .filter(|r| r.kind == ResourceKind::Base)
        .map(|r| {
            let income = world.income(player.id, r.id);
            let icon = resource_icon(rules, r.id);
            HudResource {
                icon,
                label: rules.def(r.id).name.clone(),
                amount: player.resources.get(&r.id).copied().unwrap_or(0),
                rate: income.effective + income.trade,
                capped: income.capped,
                visible: r
                    .first_age_id
                    .is_none_or(|age| rules.ages[&age].ordinal <= ordinal),
            }
        })
        .collect();
    result.wealth.per_period = rules
        .resources
        .values()
        .filter(|r| r.kind == ResourceKind::Base)
        .map(|r| world.income(player.id, r.id).trade)
        .sum();
    let cap = rules.player_stat(
        "commerce_cap",
        rules
            .constant("player.commerce_cap.base")
            .expect("validated commerce cap"),
        &sources,
    );
    let commerce = rules
        .resources
        .values()
        .filter(|r| r.kind == ResourceKind::Base && r.rate_capped)
        .map(|r| world.income(player.id, r.id).effective)
        .max()
        .unwrap_or(0);
    result.commerce = (commerce, cap);
    result.population = (
        world.population_used(player.id),
        world.population_cap(player.id),
    );
    result.cities = (
        world
            .entities()
            .filter(|e| {
                // Automated colonies stand outside the city limit, so they are not counted.
                e.owner == player.id && world.counts_as_city(e.type_id)
            })
            .count() as i64,
        rules.player_stat(
            "city_limit",
            rules.constant("player.city_limit.base").unwrap_or(0),
            &sources,
        ),
    );
    result.research = world
        .entities()
        .filter(|e| e.owner == player.id)
        .filter_map(|e| e.orders().front())
        .filter(|o| o.kind == "research" && o.status == OrderStatus::Active)
        .filter_map(|o| {
            let id = o.subject?;
            rules.techs.get(&id)?;
            let total = o.total_ticks.unwrap_or(1).max(1);
            Some(HudResearch {
                label: rules.def(id).name.clone(),
                progress_pct: (u128::from(o.progress_ticks) * 100 / u128::from(total)).min(100)
                    as u32,
            })
        })
        .collect();
    // No simulation event feed is exposed yet; leave messages empty until that API exists.
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use rom_rules::{DefId, Rules};
    use rom_sim::{Command, MapGrid, Player, Pos};
    use std::collections::BTreeMap;
    use std::sync::Arc;

    fn scenario() -> (World, Arc<Rules>) {
        let rules = Arc::new(crate::terrain::load_rules().expect("rules"));
        let id = |key: &str| rules.id_of(key).expect("definition");
        let player = Player {
            id: PlayerId(1),
            team: 1,
            age: id("age.ancient"),
            faction: id("nation.romans"),
            resources: rules.resources.keys().map(|id| (*id, 10000)).collect(),
            techs: BTreeMap::new(),
        };
        let map = MapGrid::new(64, 64, vec![id("ter.plain"); 64 * 64]).expect("map");
        (
            World::new(rules.clone(), map, 1, vec![player]).expect("world"),
            rules,
        )
    }
    fn command(
        executor: rom_sim::EntityId,
        kind: &str,
        subject: Option<DefId>,
        target_entity: Option<rom_sim::EntityId>,
    ) -> Command {
        Command {
            player: PlayerId(1),
            executor,
            kind: kind.into(),
            subject,
            target_entity,
            target_pos: None,
            queue: false,
            text: None,
        }
    }
    #[test]
    fn hud_snapshot_reports_stock_rates_and_caps() {
        let (mut world, rules) = scenario();
        let id = |key: &str| rules.id_of(key).unwrap();
        world
            .spawn(id("bld.city"), PlayerId(1), Pos::flat(30000, 30000))
            .unwrap();
        let farm = world
            .spawn(id("bld.farm"), PlayerId(1), Pos::flat(10000, 10000))
            .unwrap();
        let worker = world
            .spawn(id("unit.citizen"), PlayerId(1), Pos::flat(10000, 12000))
            .unwrap();
        world
            .submit(command(worker, "harvest", None, Some(farm)))
            .unwrap();
        for _ in 0..100 {
            world.step();
        }
        let h = snapshot_of(&world, PlayerId(1));
        let food = h
            .resources
            .iter()
            .find(|r| r.icon == "icons/res_food")
            .unwrap();
        assert_eq!(food.rate, 10, "income per configured period");
        assert_eq!(
            food.amount,
            world.players().next().unwrap().resources[&id("res.food")]
        );
        assert!(!food.capped);
        assert_eq!(h.commerce, (10, 70));
        assert_eq!(h.population, (1, 50));
        assert_eq!(h.cities, (1, 1));
        assert_eq!(h.game_ticks, world.tick());
        // Multiple workers must report the same capped rate as the simulation.
        for n in 0..8 {
            let farm = world
                .spawn(
                    id("bld.farm"),
                    PlayerId(1),
                    Pos {
                        x: 2000 + n * 6000,
                        y: 50000,
                        face: 0,
                        ..Pos::default()
                    },
                )
                .unwrap();
            let worker = world
                .spawn(
                    id("unit.citizen"),
                    PlayerId(1),
                    Pos {
                        x: 2000 + n * 6000,
                        y: 52000,
                        face: 0,
                        ..Pos::default()
                    },
                )
                .unwrap();
            world
                .submit(command(worker, "harvest", None, Some(farm)))
                .unwrap();
        }
        for _ in 0..100 {
            world.step();
        }
        let capped = snapshot_of(&world, PlayerId(1))
            .resources
            .into_iter()
            .find(|r| r.icon == "icons/res_food")
            .unwrap();
        assert_eq!(capped.rate, 70);
        assert!(capped.capped);
    }
    #[test]
    fn hidden_resource_before_its_age() {
        let (world, rules) = scenario();
        let h = snapshot_of(&world, PlayerId(1));
        let player = world.players().next().unwrap();
        for (resource, view) in rules
            .resources
            .values()
            .filter(|r| r.kind == ResourceKind::Base)
            .zip(&h.resources)
        {
            assert_eq!(
                view.visible,
                resource
                    .first_age_id
                    .is_none_or(|a| rules.ages[&a].ordinal <= rules.ages[&player.age].ordinal)
            );
        }
        assert!(
            h.resources
                .iter()
                .find(|r| r.icon == "icons/res_food")
                .unwrap()
                .visible
        );
        assert!(
            !h.resources
                .iter()
                .find(|r| r.icon == "icons/res_oil")
                .unwrap()
                .visible
        );
    }
    #[test]
    fn research_progress_appears_while_researching() {
        let (mut world, rules) = scenario();
        let library = world
            .spawn(
                rules.id_of("bld.library").unwrap(),
                PlayerId(1),
                Pos::flat(20000, 20000),
            )
            .unwrap();
        let tech = rules.id_of("tech.military.1").unwrap();
        assert!(snapshot_of(&world, PlayerId(1)).research.is_empty());
        world
            .submit(command(library, "research", Some(tech), None))
            .unwrap();
        for _ in 0..20 {
            world.step();
        }
        let h = snapshot_of(&world, PlayerId(1));
        assert_eq!(h.research.len(), 1);
        assert_eq!(h.research[0].label, rules.def(tech).name);
        let order = world.entity(library).unwrap().orders().front().unwrap();
        assert_eq!(
            u64::from(h.research[0].progress_pct),
            order.progress_ticks * 100 / order.total_ticks.unwrap()
        );
        assert!(h.research[0].progress_pct > 0 && h.research[0].progress_pct < 100);
        for _ in 0..2000 {
            world.step();
        }
        assert!(snapshot_of(&world, PlayerId(1)).research.is_empty());
    }
}
