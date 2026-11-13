//! The `rom universe` subcommand.

use std::{fs, path::PathBuf};

use rom_mapgen::{BodyKind, Universe, UniverseRequest, generate_universe};
use rom_rules::{DefId, Rules};

use crate::mapgen_cmd::ppm;

const USAGE: &str = "usage: rom universe --db <rules.db> --style <key> --seed <n> --factions <key,key,...> [--ppm <file>]";
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
    while let Some(flag) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_str() {
            "--db" => db = Some(PathBuf::from(value)),
            "--style" => style = Some(value),
            "--seed" => seed = Some(value.parse::<u64>()?),
            "--factions" => factions = Some(value),
            "--ppm" => ppm_path = Some(PathBuf::from(value)),
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
            player_factions,
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
    Ok(())
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
