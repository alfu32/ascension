//! The `rom universe` subcommand.

use std::{collections::BTreeMap, fs, path::PathBuf, sync::Arc};

use rom_mapgen::{BodyKind, Universe, UniverseRequest, generate_universe};
use rom_rules::{DefId, Rules};
use rom_sim::{
    BodyId, BodyInfo, Command, EntityId, PlanetSurface, Player, PlayerId, Pos, RouteInfo, SimRng,
    SystemInfo, UniverseSetup, World,
};

use crate::mapgen_cmd::ppm;

const USAGE: &str = "usage: rom universe --db <rules.db> --style <key> --seed <n> --factions <key,key,...> [--ppm <file>] [--simulate <ticks>] [--space]";
/// Builders placed around each starting city.
const BUILDERS: usize = 3;
/// Ticks between rounds of random move orders for idle units.
const MOVE_PERIOD: u64 = 200;
/// Largest displacement of a random move along each local axis, in milli-tiles.
const MOVE_SPAN: i64 = 24_000;
/// Mixed into the seed so the move orders have their own random stream.
const MOVE_STREAM: u64 = 0x6d6f_7665_7321_2121;
/// Side of the star map image in pixels.
const SIDE: i64 = 1024;
/// Pixels from the image centre to the star map rim.
const RIM: i64 = 440;
/// Radius of a system disc in pixels.
const DISC: i64 = 14;
const BACKGROUND: [u8; 3] = [6, 6, 14];
const ROUTE: [u8; 3] = [96, 104, 132];
const NEUTRAL: [u8; 3] = [150, 150, 150];
const HABITABLE: [u8; 3] = [80, 200, 255];
const HOME_PLANET: [u8; 3] = [255, 255, 255];
const RESOURCE: [u8; 3] = [255, 200, 0];
/// Home system colours, by the race's place among the races present.
const RACE_COLOURS: [[u8; 3]; 6] = [
    [230, 70, 60],
    [70, 200, 90],
    [90, 120, 255],
    [230, 150, 40],
    [190, 80, 220],
    [40, 210, 210],
];

/// Runs the subcommand with the arguments after `universe`.
pub fn run(mut args: impl Iterator<Item = String>) -> Result<(), Box<dyn std::error::Error>> {
    let (mut db, mut style, mut seed, mut factions, mut ppm_path) = (None, None, None, None, None);
    let mut simulate = None;
    let mut space = false;
    while let Some(flag) = args.next() {
        if flag == "--space" {
            space = true;
            continue;
        }
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_str() {
            "--db" => db = Some(PathBuf::from(value)),
            "--style" => style = Some(value),
            "--seed" => seed = Some(value.parse::<u64>()?),
            "--factions" => factions = Some(value),
            "--ppm" => ppm_path = Some(PathBuf::from(value)),
            "--simulate" => simulate = Some(value.parse::<u64>()?),
            _ => return Err(format!("unknown argument: {flag}").into()),
        }
    }
    let (db, style, seed, factions) = (
        db.ok_or(USAGE)?,
        style.ok_or(USAGE)?,
        seed.ok_or(USAGE)?,
        factions.ok_or(USAGE)?,
    );
    let conn =
        rusqlite::Connection::open_with_flags(db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let rules = Rules::load(&conn)?;
    let style_id = rules
        .id_of(&style)
        .filter(|id| rules.map_styles.contains_key(id))
        .ok_or_else(|| format!("unknown style {style}"))?;
    let player_factions = factions
        .split(',')
        .map(|key| {
            rules
                .id_of(key)
                .filter(|id| rules.factions.contains_key(id))
                .ok_or_else(|| format!("unknown faction {key}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let universe = generate_universe(
        &rules,
        &UniverseRequest {
            style: style_id,
            seed,
            player_factions: player_factions.clone(),
        },
    )?;
    println!("{}", summary(&rules, &universe));
    for problem in universe.fairness(&rules) {
        eprintln!("unfair: {problem}");
    }
    if let Some(path) = ppm_path {
        let radius = rules.map_styles[&style_id].star_map_radius;
        fs::write(path, star_map_ppm(&universe, radius))?;
    }
    if let Some(ticks) = simulate {
        let world = simulate_universe(
            Arc::new(rules),
            &universe,
            &player_factions,
            seed,
            ticks,
            space,
        )?;
        println!("{}", world_line(&world));
        if space {
            println!("{}", space_line(&world));
        }
    }
    Ok(())
}

/// The universe as plain data for the simulation, with the generated (home) planets.
pub fn universe_setup(universe: &Universe) -> UniverseSetup {
    UniverseSetup {
        systems: universe
            .systems
            .iter()
            .map(|s| SystemInfo {
                id: s.id,
                name: s.name.clone(),
                x: s.x,
                y: s.y,
            })
            .collect(),
        bodies: universe
            .bodies
            .iter()
            .map(|b| BodyInfo {
                id: BodyId(b.id),
                system: b.system_id,
                kind: b.kind,
                name: b.name.clone(),
                orbit_radius: b.orbit_radius,
                orbit_angle_mdeg: b.orbit_angle_mdeg,
                n: b.n,
                home_race: b.home_race,
                node_type: b.node_entity_type,
            })
            .collect(),
        routes: universe
            .routes
            .iter()
            .map(|r| RouteInfo {
                system_a: r.system_a,
                system_b: r.system_b,
                length: r.length,
            })
            .collect(),
        planets: universe
            .planets
            .iter()
            .map(|(body, planet)| {
                (
                    BodyId(*body),
                    PlanetSurface {
                        surface: planet.surface,
                        terrain: planet.terrain.clone(),
                        nodes: planet.nodes.clone(),
                    },
                )
            })
            .collect(),
    }
}

/// Neutral player 0 and one player per faction, each on its own team, in the faction's
/// first age (or the earliest age).
fn players(rules: &Rules, factions: &[DefId]) -> Result<Vec<Player>, Box<dyn std::error::Error>> {
    let earliest = rules
        .ages
        .values()
        .min_by_key(|a| (a.ordinal, a.id))
        .ok_or("the rules have no age")?
        .id;
    let player = |id: u32, faction: DefId| Player {
        id: PlayerId(id),
        team: id,
        faction,
        age: rules
            .factions
            .get(&faction)
            .and_then(|f| f.first_age_id)
            .filter(|age| rules.ages.contains_key(age))
            .unwrap_or(earliest),
        resources: BTreeMap::new(),
        techs: BTreeMap::new(),
    };
    let first = *factions.first().ok_or("no factions")?;
    Ok(std::iter::once(player(0, first))
        .chain(
            factions
                .iter()
                .enumerate()
                .map(|(i, f)| player(i as u32 + 1, *f)),
        )
        .collect())
}

/// Builds the world from the universe's home planets, reveals each player's home planet,
/// places each player's starting city with `find_city_site` and builders around it, and
/// runs `ticks` ticks in which idle units are sent to random points on their planet.
pub fn simulate_universe(
    rules: Arc<Rules>,
    universe: &Universe,
    factions: &[DefId],
    seed: u64,
    ticks: u64,
    space: bool,
) -> Result<World, Box<dyn std::error::Error>> {
    let builder = rules
        .entity_types
        .values()
        .find(|et| et.builder.is_some() && et.movement.is_some())
        .ok_or("the rules have no builder")?
        .id;
    let homes: BTreeMap<PlayerId, BodyId> = universe
        .starts
        .iter()
        .enumerate()
        .map(|(i, (body, _))| (PlayerId(i as u32 + 1), BodyId(*body)))
        .collect();
    let mut players = players(&rules, factions)?;
    if space {
        let age = rules
            .ages
            .values()
            .filter(|a| {
                let rank = rules.map_scales[&a.map_scale].rank;
                rules.map_scales.values().any(|s| s.rank < rank)
            })
            .min_by_key(|a| (a.ordinal, a.id))
            .ok_or("no space age")?
            .id;
        for player in players.iter_mut().filter(|p| p.id != PlayerId(0)) {
            player.age = age;
        }
    }
    let mut world = World::from_universe(
        Arc::clone(&rules),
        universe_setup(universe),
        seed,
        players,
        homes,
    )?;
    let mut units = Vec::new();
    for (i, (body, start)) in universe.starts.iter().enumerate() {
        let (player, body) = (PlayerId(i as u32 + 1), BodyId(*body));
        world.reveal_terrain(player, body);
        let mut near = World::pos_of(*start)?;
        near.body = body;
        let city = world.city_type(player).ok_or("no city type")?;
        let site = world
            .find_city_site(player, body, near)
            .ok_or("no room for a starting city")?;
        world.place_for_setup(city, player, site)?;
        let et = &rules.entity_types[&city];
        let reach = et.footprint_w.max(et.footprint_h) * 500 + 1500;
        for n in 0..BUILDERS {
            let at = ring_position(&world, site, n, reach);
            units.push(world.spawn(builder, player, at)?);
        }
    }
    let mut scouts = if space {
        setup_scouts(&mut world)?
    } else {
        Vec::new()
    };
    let mut rng = SimRng::new(seed ^ MOVE_STREAM);
    for tick in 0..ticks {
        if tick % MOVE_PERIOD == 0 {
            send_idle(&mut world, &units, &mut rng);
        }
        send_scouts(&mut world, &mut scouts)?;
        world.step();
    }
    Ok(world)
}

/// A scout's first claim, followed by one voyage and a destination orbit.
struct Scout {
    ship: EntityId,
    first: BodyId,
    destination: Option<u32>,
    arrived: bool,
}

fn nearest_resource(world: &World, pos: Pos) -> Option<BodyId> {
    world
        .bodies()
        .iter()
        .filter(|b| b.kind == BodyKind::Resource)
        .filter_map(|b| {
            world
                .body_position(b.id)
                .and_then(|at| world.distance(pos, at))
                .map(|d| (d, b.id))
        })
        .min()
        .map(|(_, b)| b)
}

fn orbit(world: &mut World, ship: EntityId, body: BodyId) -> Result<(), rom_sim::SimError> {
    world.submit(Command {
        player: world.entity(ship).expect("scout").owner,
        executor: ship,
        kind: "orbit".into(),
        subject: None,
        target_entity: world.body_marker(body),
        target_pos: None,
        queue: false,
    })?;
    Ok(())
}

fn setup_scouts(world: &mut World) -> Result<Vec<Scout>, Box<dyn std::error::Error>> {
    let terrain = world
        .rules()
        .terrains
        .values()
        .find(|t| t.layer == rom_rules::Layer::Space && !t.blocks_los)
        .ok_or("no space terrain")?
        .id;
    let ship_type = world
        .rules()
        .entity_types
        .values()
        .find(|e| {
            e.movement.as_ref().is_some_and(|m| {
                world
                    .rules()
                    .movement_class_terrain
                    .get(&m.movement_class_id)
                    .into_iter()
                    .flatten()
                    .any(|r| r.terrain_id == terrain && r.speed_pct > 0)
            })
        })
        .ok_or("no starship")?
        .id;
    let players: Vec<_> = world
        .players()
        .filter(|p| p.id != PlayerId(0))
        .map(|p| p.id)
        .collect();
    let mut scouts = Vec::new();
    let mut per_system = BTreeMap::<u32, usize>::new();
    for player in players {
        let home = world.home_body(player).ok_or("no home")?;
        let system = world.body(home).ok_or("no body")?.system;
        let resources: Vec<_> = world
            .bodies()
            .iter()
            .filter(|b| b.system == system && b.kind == BodyKind::Resource)
            .map(|b| b.id)
            .collect();
        let index = per_system.entry(system).or_default();
        let body = *resources
            .get(*index % resources.len().max(1))
            .ok_or("no resource body")?;
        *index += 1;
        // Separate starting scouts in a shared home system so each can make a claim.
        let pos = world.body_position(body).ok_or("no marker position")?;
        let ship = world.spawn(ship_type, player, pos)?;
        let first = nearest_resource(world, pos).ok_or("no nearest resource")?;
        orbit(world, ship, first)?;
        scouts.push(Scout {
            ship,
            first,
            destination: None,
            arrived: false,
        });
    }
    Ok(scouts)
}

fn send_scouts(world: &mut World, scouts: &mut [Scout]) -> Result<(), rom_sim::SimError> {
    for scout in scouts {
        let Some(ship) = world.entity(scout.ship) else {
            continue;
        };
        let (pos, owner) = (ship.pos, ship.owner);
        if scout.destination.is_none()
            && world.claims().any(|(b, p)| b == scout.first && p == owner)
        {
            let system = world.body(pos.body).expect("space").system;
            let origin = world
                .systems()
                .iter()
                .find(|s| s.id == system)
                .expect("origin");
            let destination = world
                .systems()
                .iter()
                .filter(|s| s.id != system)
                .min_by_key(|s| {
                    let dx = i64::from(s.x) - i64::from(origin.x);
                    let dy = i64::from(s.y) - i64::from(origin.y);
                    (
                        i128::from(dx) * i128::from(dx) + i128::from(dy) * i128::from(dy),
                        s.id,
                    )
                })
                .map(|s| s.id);
            if let Some(destination) = destination {
                world.submit(Command {
                    player: owner,
                    executor: scout.ship,
                    kind: "travel".into(),
                    subject: None,
                    target_entity: None,
                    target_pos: Some(Pos {
                        body: world.space_of(destination).expect("space"),
                        ..Pos::default()
                    }),
                    queue: false,
                })?;
                scout.destination = Some(destination);
            }
        } else if !scout.arrived
            && scout
                .destination
                .is_some_and(|s| world.space_of(s) == Some(pos.body))
        {
            if let Some(body) = nearest_resource(world, pos) {
                orbit(world, scout.ship, body)?;
            }
            scout.arrived = true;
        }
    }
    Ok(())
}

fn space_line(world: &World) -> String {
    format!(
        "space claims={} transits_done={} hash={:016x}",
        world.claims().count(),
        world.transits_done(),
        world.hash()
    )
}

/// The `n`-th of `BUILDERS` points on a ring around `centre`, moved outwards until the
/// ground is buildable; the centre when none is.
fn ring_position(world: &World, centre: Pos, n: usize, reach: i64) -> Pos {
    const DIRECTIONS: [(i64, i64); 3] = [(1000, 0), (-500, 866), (-500, -866)];
    let (dx, dy) = DIRECTIONS[n % DIRECTIONS.len()];
    (0..6)
        .map(|step| {
            let distance = reach + step * 700;
            world.offset(centre, dx * distance / 1000, dy * distance / 1000)
        })
        .find(|at| {
            world
                .terrain_at(*at)
                .is_some_and(|t| world.rules().terrains[&t].buildable)
        })
        .unwrap_or(centre)
}

/// Sends every idle unit of `units` to a random point near it on its own planet.
fn send_idle(world: &mut World, units: &[EntityId], rng: &mut SimRng) {
    for id in units {
        let Some(unit) = world.entity(*id) else {
            continue;
        };
        if !unit.orders().is_empty() {
            continue;
        }
        let (owner, pos) = (unit.owner, unit.pos);
        let span = 2 * MOVE_SPAN as u32 + 1;
        let dx = i64::from(rng.range(0, span)) - MOVE_SPAN;
        let dy = i64::from(rng.range(0, span)) - MOVE_SPAN;
        let target = world.offset(pos, dx, dy);
        // A refused order (an unreachable point) just leaves the unit idle.
        let _ = world.submit(Command {
            player: owner,
            executor: *id,
            kind: "move".into(),
            subject: None,
            target_entity: None,
            target_pos: Some(target),
            queue: false,
        });
    }
}

/// The line printed after a simulation.
pub fn world_line(world: &World) -> String {
    format!(
        "world bodies={}/{} entities={} tick={} hash={:016x}",
        world
            .surfaces()
            .iter()
            .filter(|s| world
                .body(s.body())
                .is_none_or(|b| b.kind == BodyKind::Habitable))
            .count(),
        world.habitable_count(),
        world.entities().count(),
        world.tick(),
        world.hash()
    )
}

/// The one-line summary printed by the command.
fn summary(rules: &Rules, universe: &Universe) -> String {
    let count = |kind: BodyKind| universe.bodies.iter().filter(|b| b.kind == kind).count();
    let homes: Vec<String> = universe
        .home_systems()
        .iter()
        .map(|&(race, system)| {
            format!(
                "{}:{}",
                rules.key(race),
                universe.systems[system as usize - 1].name
            )
        })
        .collect();
    format!(
        "universe systems={} bodies={} habitable={} resource={} routes={} homes={} fair={} hash={:016x}",
        universe.systems.len(),
        universe.bodies.len(),
        count(BodyKind::Habitable),
        count(BodyKind::Resource),
        universe.routes.len(),
        homes.join(","),
        if universe.fairness(rules).is_empty() {
            "yes"
        } else {
            "no"
        },
        universe.hash()
    )
}

struct Canvas {
    pixels: Vec<[u8; 3]>,
}

impl Canvas {
    fn set(&mut self, x: i64, y: i64, colour: [u8; 3]) {
        if (0..SIDE).contains(&x) && (0..SIDE).contains(&y) {
            self.pixels[(y * SIDE + x) as usize] = colour;
        }
    }

    fn disc(&mut self, cx: i64, cy: i64, r: i64, colour: [u8; 3]) {
        for dy in -r..=r {
            for dx in -r..=r {
                if dx * dx + dy * dy <= r * r {
                    self.set(cx + dx, cy + dy, colour);
                }
            }
        }
    }

    /// Bresenham line, two pixels wide.
    fn line(&mut self, (x0, y0): (i64, i64), (x1, y1): (i64, i64), colour: [u8; 3]) {
        let (dx, dy) = ((x1 - x0).abs(), -(y1 - y0).abs());
        let (sx, sy) = ((x1 - x0).signum(), (y1 - y0).signum());
        let (mut x, mut y, mut err) = (x0, y0, dx + dy);
        loop {
            self.set(x, y, colour);
            if dx >= -dy {
                self.set(x, y + 1, colour);
            } else {
                self.set(x + 1, y, colour);
            }
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }
}

/// The star map: routes as lines, systems as discs coloured by home race (grey
/// when neutral), and a row of small dots under each system, one per body:
/// white for the home planet, blue for other habitable planets, yellow for resource bodies.
fn star_map_ppm(universe: &Universe, radius: i64) -> Vec<u8> {
    let radius = radius.max(1);
    let px = |s: &rom_mapgen::StarSystem| {
        (
            SIDE / 2 + i64::from(s.x) * RIM / radius,
            SIDE / 2 + i64::from(s.y) * RIM / radius,
        )
    };
    let mut canvas = Canvas {
        pixels: vec![BACKGROUND; (SIDE * SIDE) as usize],
    };
    for r in &universe.routes {
        let a = &universe.systems[r.system_a as usize - 1];
        let b = &universe.systems[r.system_b as usize - 1];
        canvas.line(px(a), px(b), ROUTE);
    }
    let homes = universe.home_systems();
    let races: Vec<DefId> = homes.iter().map(|&(race, _)| race).collect();
    for system in &universe.systems {
        let (cx, cy) = px(system);
        let colour = homes
            .iter()
            .find(|&&(_, s)| s == system.id)
            .and_then(|&(race, _)| races.iter().position(|&r| r == race))
            .map_or(NEUTRAL, |i| RACE_COLOURS[i % RACE_COLOURS.len()]);
        canvas.disc(cx, cy, DISC, colour);
        let mut dots: Vec<[u8; 3]> = universe
            .bodies
            .iter()
            .filter(|b| b.system_id == system.id)
            .filter_map(|b| match b.kind {
                BodyKind::Star => None,
                BodyKind::Habitable if b.home_race.is_some() => Some(HOME_PLANET),
                BodyKind::Habitable => Some(HABITABLE),
                BodyKind::Resource => Some(RESOURCE),
            })
            .collect();
        dots.sort_by_key(|c| std::cmp::Reverse(*c == HOME_PLANET || *c == HABITABLE));
        let width = dots.len() as i64 * 10;
        for (i, &c) in dots.iter().enumerate() {
            canvas.disc(cx - width / 2 + 5 + i as i64 * 10, cy + DISC + 9, 3, c);
        }
    }
    ppm(SIDE as usize, SIDE as usize, &canvas.pixels)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(ticks: u64) -> String {
        let conn = rom_rules::build_rules_db(
            include_str!("../../../db/schema.sql"),
            include_str!("../../../db/seed.sql"),
        )
        .unwrap();
        let rules = Rules::load(&conn).unwrap();
        let factions: Vec<DefId> = ["nation.romans", "nation.ashscale"]
            .iter()
            .map(|key| rules.id_of(key).unwrap())
            .collect();
        let request = UniverseRequest {
            style: rules.id_of("map.many_worlds").unwrap(),
            seed: 3,
            player_factions: factions.clone(),
        };
        let universe = generate_universe(&rules, &request).unwrap();
        let world =
            simulate_universe(Arc::new(rules), &universe, &factions, 3, ticks, false).unwrap();
        // Every home planet has a surface, and each player's start is on its home planet.
        assert_eq!(
            world.surfaces().len(),
            universe.planets.len() + universe.systems.len()
        );
        for player in [PlayerId(1), PlayerId(2)] {
            let home = world.home_body(player).unwrap();
            assert!(
                world
                    .entities()
                    .filter(|e| e.owner == player)
                    .all(|e| e.pos.body == home)
            );
            assert!(world.visibility_tiles(player, home).iter().all(|t| *t > 0));
        }
        world_line(&world)
    }

    #[test]
    fn space_scenario_same_seed_same_hash() {
        fn scenario() -> (String, String) {
            let conn = rom_rules::build_rules_db(
                include_str!("../../../db/schema.sql"),
                include_str!("../../../db/seed.sql"),
            )
            .unwrap();
            conn.execute_batch(
                "UPDATE map_style SET planet_n_min=64,planet_n_median=64,planet_n_max=64;",
            )
            .unwrap();
            let rules = Rules::load(&conn).unwrap();
            let factions: Vec<_> = [
                "nation.romans",
                "nation.egyptians",
                "nation.ashscale",
                "nation.highwind",
            ]
            .iter()
            .map(|k| rules.id_of(k).unwrap())
            .collect();
            let universe = generate_universe(
                &rules,
                &UniverseRequest {
                    style: rules.id_of("map.many_worlds").unwrap(),
                    seed: 1,
                    player_factions: factions.clone(),
                },
            )
            .unwrap();
            let world =
                simulate_universe(Arc::new(rules), &universe, &factions, 1, 6000, true).unwrap();
            assert!(world.claims().count() >= 4);
            assert!(world.transits_done() >= 4);
            (world_line(&world), space_line(&world))
        }
        assert_eq!(scenario(), scenario());
    }

    #[test]
    fn universe_world_same_seed_same_hash() {
        let first = run(600);
        assert!(first.contains("tick=600"), "{first}");
        assert_eq!(first, run(600));
    }
}
