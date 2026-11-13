//! The `rom territory` subcommand: borders of the starting cities on a generated map.

use std::{collections::BTreeMap, fs, path::PathBuf, sync::Arc};

use rom_mapgen::{MapRequest, generate};
use rom_rules::Rules;
use rom_sim::{Player, PlayerId, World};

use crate::mapgen_cmd::{ppm, terrain_pixels};

const USAGE: &str =
    "usage: rom territory --db <rules.db> --seed <n> --players <n> --size <W>x<H> [--ppm <file>]";

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
            _ => return Err(format!("unknown argument: {flag}").into()),
        }
    }
    let (db, seed, players, (width, height)) = (
        db.ok_or(USAGE)?,
        seed.ok_or(USAGE)?,
        players.ok_or(USAGE)?,
        size.ok_or(USAGE)?,
    );
    let conn =
        rusqlite::Connection::open_with_flags(db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let rules = Arc::new(Rules::load(&conn)?);
    let style = *rules
        .map_styles
        .keys()
        .next()
        .ok_or("the rules have no map style")?;
    let faction = rules
        .factions
        .values()
        .find(|f| f.playable)
        .ok_or("the rules have no playable faction")?
        .id;
    let age = rules
        .ages
        .values()
        .min_by_key(|a| (a.ordinal, a.id))
        .ok_or("the rules have no age")?
        .id;
    let city = rules
        .entity_types
        .values()
        .find(|et| et.territory.as_ref().is_some_and(|t| t.is_city))
        .ok_or("the rules have no city type")?
        .id;
    let map = generate(
        &rules,
        &MapRequest::uniform(style, seed, width, height, players, faction),
    )?;
    let roster = (1..=map.starts.len() as u32)
        .map(|n| Player {
            id: PlayerId(n),
            team: n,
            faction,
            age,
            resources: BTreeMap::new(),
            techs: BTreeMap::new(),
        })
        .collect();
    let mut world = World::new(rules, map.grid.clone(), seed, roster)?;
    for (n, start) in map.starts.iter().enumerate() {
        world.place_for_setup(city, PlayerId(n as u32 + 1), *start)?;
    }
    world.step();
    let owned: Vec<String> = (1..=map.starts.len() as u32)
        .map(|n| {
            world
                .territory_tiles()
                .get(&PlayerId(n))
                .copied()
                .unwrap_or(0)
                .to_string()
        })
        .collect();
    println!(
        "territory players={} owned={} hash={:016x}",
        map.starts.len(),
        owned.join(","),
        world.hash()
    );
    if let Some(path) = out {
        let mut pixels = terrain_pixels(&map);
        for (index, pixel) in pixels.iter_mut().enumerate() {
            let (x, y) = (index as u32 % width, index as u32 / width);
            let dim = pixel.map(|c| c / 3);
            *pixel = match world.territory_owner(x as i32, y as i32) {
                Some(owner) => {
                    let colour = COLOURS[(owner.0 as usize - 1) % COLOURS.len()];
                    // Owner colour over the dimmed terrain, so relief stays readable.
                    [0, 1, 2].map(|i| (dim[i] / 2).saturating_add(colour[i] / 4 * 3))
                }
                None => dim,
            };
        }
        fs::write(path, ppm(width as usize, height as usize, &pixels))?;
    }
    Ok(())
}
