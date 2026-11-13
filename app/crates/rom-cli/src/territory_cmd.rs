//! The `rom territory` subcommand: borders of the starting cities on a generated map.

use std::{collections::BTreeMap, fs, path::PathBuf, sync::Arc};

use rom_mapgen::{MapRequest, generate};
use rom_rules::{DefId, Rules};
use rom_sim::{Player, PlayerId, Pos, World};

use crate::mapgen_cmd::{first_style_planet, net_ppm, ppm, sphere_colours, terrain_pixels};

const USAGE: &str = "usage: rom territory --db <rules.db> --seed <n> --players <n> (--size <W>x<H> | --sphere <n>) [--ppm <file>]";

/// Owner colours, by player order; later players reuse them.
const COLOURS: [[u8; 3]; 8] = [
    [230, 40, 40],
    [40, 110, 240],
    [250, 210, 30],
    [200, 60, 220],
    [40, 220, 200],
    [250, 140, 30],
    [240, 240, 240],
    [120, 230, 60],
];

/// Runs the subcommand with the arguments after `territory`.
pub fn run(mut args: impl Iterator<Item = String>) -> Result<(), Box<dyn std::error::Error>> {
    let (mut db, mut seed, mut players, mut size, mut out) = (None, None, None, None, None);
    let mut sphere = None;
    while let Some(flag) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_str() {
            "--db" => db = Some(PathBuf::from(value)),
            "--seed" => seed = Some(value.parse::<u64>()?),
            "--players" => players = Some(value.parse::<u32>()?),
            "--size" => {
                let (w, h) = value.split_once('x').ok_or("size must look like 256x256")?;
                size = Some((w.parse::<u32>()?, h.parse::<u32>()?));
            }
            "--ppm" => out = Some(PathBuf::from(value)),
            "--sphere" => sphere = Some(value.parse::<u32>()?),
            _ => return Err(format!("unknown argument: {flag}").into()),
        }
    }
    let (db, seed, players) = (db.ok_or(USAGE)?, seed.ok_or(USAGE)?, players.ok_or(USAGE)?);
    let conn =
        rusqlite::Connection::open_with_flags(db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let rules = Arc::new(Rules::load(&conn)?);
    if let Some(n) = sphere {
        if size.is_some() {
            return Err("use either --size or --sphere".into());
        }
        return run_sphere(rules, seed, players, n, out);
    }
    let (width, height) = size.ok_or(USAGE)?;
    let style = *rules
        .map_styles
        .keys()
        .next()
        .ok_or("the rules have no map style")?;
    let setup = Setup::from_rules(&rules)?;
    let map = generate(
        &rules,
        &MapRequest::uniform(style, seed, width, height, players, setup.faction),
    )?;
    let mut world = World::new(
        rules,
        map.grid.clone(),
        seed,
        setup.roster(map.starts.len()),
    )?;
    for (n, start) in map.starts.iter().enumerate() {
        world.place_for_setup(setup.city, PlayerId(n as u32 + 1), *start)?;
    }
    world.step();
    report(&world, map.starts.len());
    if let Some(path) = out {
        let mut pixels = terrain_pixels(&map);
        for (index, pixel) in pixels.iter_mut().enumerate() {
            let (x, y) = (index as u32 % width, index as u32 / width);
            *pixel = tint(*pixel, world.territory_owner(x as i32, y as i32));
        }
        fs::write(path, ppm(width as usize, height as usize, &pixels))?;
    }
    Ok(())
}

/// The same borders on a sphere planet; `--ppm` writes the six-face net.
fn run_sphere(
    rules: Arc<Rules>,
    seed: u64,
    players: u32,
    n: u32,
    out: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let setup = Setup::from_rules(&rules)?;
    let planet = first_style_planet(&rules, seed, n, players)?;
    let (w, h) = {
        let et = &rules.entity_types[&setup.city];
        (et.footprint_w, et.footprint_h)
    };
    let mut world = World::from_sphere(
        rules,
        planet.surface,
        planet.terrain.clone(),
        &[],
        seed,
        setup.roster(planet.starts.len()),
    )?;
    let side = i64::from(n) * 1000;
    for (index, start) in planet.starts.iter().enumerate() {
        // A footprint stays on one face: move a start near an edge inwards just enough.
        let inside = |v: i64, size: i64| v.clamp(size * 500, side - size * 500) as i32;
        let at = Pos {
            x: inside(start.x, w),
            y: inside(start.y, h),
            face: start.face,
            ..Pos::default()
        };
        world.place_for_setup(setup.city, PlayerId(index as u32 + 1), at)?;
    }
    world.step();
    report(&world, planet.starts.len());
    if let Some(path) = out {
        let mut pixels = sphere_colours(&planet, world.rules());
        for (index, pixel) in pixels.iter_mut().enumerate() {
            let tile = planet.surface.tile(index);
            let owner = world.territory_owner_on(tile.face, tile.x as i32, tile.y as i32);
            *pixel = tint(*pixel, owner);
        }
        fs::write(path, net_ppm(&planet, &pixels))?;
    }
    Ok(())
}

/// The roster and city type shared by both topologies.
struct Setup {
    faction: DefId,
    age: DefId,
    city: DefId,
}

impl Setup {
    fn from_rules(rules: &Rules) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            faction: rules
                .factions
                .values()
                .find(|f| f.playable)
                .ok_or("the rules have no playable faction")?
                .id,
            age: rules
                .ages
                .values()
                .min_by_key(|a| (a.ordinal, a.id))
                .ok_or("the rules have no age")?
                .id,
            city: rules
                .entity_types
                .values()
                .find(|et| et.territory.as_ref().is_some_and(|t| t.is_city))
                .ok_or("the rules have no city type")?
                .id,
        })
    }

    fn roster(&self, players: usize) -> Vec<Player> {
        (1..=players as u32)
            .map(|n| Player {
                id: PlayerId(n),
                team: n,
                faction: self.faction,
                age: self.age,
                resources: BTreeMap::new(),
                techs: BTreeMap::new(),
            })
            .collect()
    }
}

fn report(world: &World, players: usize) {
    let owned: Vec<String> = (1..=players as u32)
        .map(|n| {
            world
                .owned_tile_counts()
                .get(&PlayerId(n))
                .copied()
                .unwrap_or(0)
                .to_string()
        })
        .collect();
    println!(
        "territory players={} owned={} hash={:016x}",
        players,
        owned.join(","),
        world.hash()
    );
}

/// Owner colour over the dimmed terrain, so relief stays readable.
fn tint(pixel: [u8; 3], owner: Option<PlayerId>) -> [u8; 3] {
    let dim = pixel.map(|c| c / 3);
    match owner {
        Some(owner) => {
            let colour = COLOURS[(owner.0 as usize - 1) % COLOURS.len()];
            [0, 1, 2].map(|i| (dim[i] / 2).saturating_add(colour[i] / 4 * 3))
        }
        None => dim,
    }
}
