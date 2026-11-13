//! The `rom mapgen` subcommand.

use std::{fs, path::PathBuf};

use rom_mapgen::{GeneratedMap, MapRequest, generate};
use rom_rules::{DefId, Layer, Rules};

const USAGE: &str = "usage: rom mapgen --db <rules.db> --style <key> --seed <n> (--players <n> | --factions <key,key,...>) --size <W>x<H> [--ppm <file>]";

/// Runs the subcommand with the arguments after `mapgen`.
pub fn run(mut args: impl Iterator<Item = String>) -> Result<(), Box<dyn std::error::Error>> {
    let (mut db, mut style, mut seed, mut players, mut size, mut ppm) =
        (None, None, None, None, None, None);
    let mut factions: Option<Vec<String>> = None;
    while let Some(flag) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_str() {
            "--db" => db = Some(PathBuf::from(value)),
            "--style" => style = Some(value),
            "--seed" => seed = Some(value.parse::<u64>()?),
            "--players" => players = Some(value.parse::<u32>()?),
            "--factions" => factions = Some(value.split(',').map(str::to_owned).collect()),
            "--size" => {
                let (w, h) = value.split_once('x').ok_or("size must look like 256x256")?;
                size = Some((w.parse::<u32>()?, h.parse::<u32>()?));
            }
            "--ppm" => ppm = Some(PathBuf::from(value)),
            _ => return Err(format!("unknown argument: {flag}").into()),
        }
    }
    let (db, style, seed, (width, height)) = (
        db.ok_or(USAGE)?,
        style.ok_or(USAGE)?,
        seed.ok_or(USAGE)?,
        size.ok_or(USAGE)?,
    );
    let conn =
        rusqlite::Connection::open_with_flags(db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let rules = Rules::load(&conn)?;
    let style_id = rules
        .id_of(&style)
        .ok_or_else(|| format!("unknown style {style}"))?;
    let player_factions = match (factions, players) {
        (Some(_), Some(_)) => return Err("use either --players or --factions".into()),
        (Some(keys), None) => keys
            .iter()
            .map(|key| {
                rules
                    .id_of(key)
                    .filter(|id| rules.factions.contains_key(id))
                    .ok_or_else(|| format!("unknown faction {key}"))
            })
            .collect::<Result<Vec<_>, _>>()?,
        (None, Some(n)) => {
            let first = rules
                .factions
                .values()
                .find(|f| f.playable)
                .ok_or("the rules have no playable faction")?;
            vec![first.id; n as usize]
        }
        (None, None) => return Err(USAGE.into()),
    };
    let map = generate(
        &rules,
        &MapRequest {
            style: style_id,
            seed,
            width,
            height,
            player_factions: player_factions.clone(),
        },
    )?;
    let mut homes: Vec<(rom_rules::DefId, Vec<usize>)> = Vec::new();
    for (index, faction) in player_factions.iter().enumerate() {
        let race = rules.race_of(*faction);
        match homes.iter_mut().find(|(r, _)| *r == race) {
            Some((_, players)) => players.push(index + 1),
            None => homes.push((race, vec![index + 1])),
        }
    }
    homes.sort_by_key(|(race, _)| *race);
    let homes: Vec<String> = homes
        .iter()
        .map(|(race, players)| {
            let list: Vec<String> = players.iter().map(usize::to_string).collect();
            format!("{}:{}", rules.key(*race), list.join("+"))
        })
        .collect();
    let fairness = map.fairness(&rules);
    let total = u64::from(width) * u64::from(height);
    let share = |layer: Layer| {
        let n = map
            .grid
            .terrain
            .iter()
            .filter(|t| rules.terrains.get(t).is_some_and(|t| t.layer == layer))
            .count() as u64;
        n * 100 / total
    };
    println!(
        "map {width}x{height} regions={} starts={} nodes={} land={}% sea={}% space={}% fair={} hash={:016x} homes={}",
        map.regions.len(),
        map.starts.len(),
        map.nodes.len(),
        share(Layer::Ground),
        share(Layer::Fluid),
        share(Layer::Space),
        if fairness.is_empty() { "yes" } else { "no" },
        map.hash(),
        homes.join(",")
    );
    for problem in &fairness {
        eprintln!("unfair: {problem}");
    }
    if let Some(path) = ppm {
        fs::write(path, ppm_bytes(&map))?;
    }
    Ok(())
}

fn feature_colour(id: DefId) -> [u8; 3] {
    let n = id.0.wrapping_mul(2_654_435_761);
    [
        (n >> 24) as u8 | 0x40,
        (n >> 16) as u8 | 0x40,
        (n >> 8) as u8 | 0x40,
    ]
}

/// One colour per tile from the terrain and its height.
pub(crate) fn terrain_pixels(map: &GeneratedMap) -> Vec<[u8; 3]> {
    map.grid
        .terrain
        .iter()
        .zip(&map.height)
        .map(|(t, height)| {
            let b = map.base;
            if *t == b.land {
                let g = (70 + u32::from(*height) * 3 / 4).min(255) as u8;
                [g / 3, g, g / 4]
            } else if *t == b.shallow {
                [0x6a, 0xb8, 0xe8]
            } else if *t == b.deep {
                [0x1a, 0x3a, 0x8a]
            } else if *t == b.space {
                [6, 6, 14]
            } else {
                feature_colour(*t)
            }
        })
        .collect()
}

/// A binary PPM image of the given pixels.
pub(crate) fn ppm(width: usize, height: usize, pixels: &[[u8; 3]]) -> Vec<u8> {
    let mut out = format!("P6\n{width} {height}\n255\n").into_bytes();
    for p in pixels {
        out.extend_from_slice(p);
    }
    out
}

fn ppm_bytes(map: &GeneratedMap) -> Vec<u8> {
    let (w, h) = (map.grid.width as usize, map.grid.height as usize);
    let mut pixels = terrain_pixels(map);
    let mut mark = |x: i32, y: i32, colour: [u8; 3]| {
        let (tx, ty) = (x / 1000, y / 1000);
        pixels[ty as usize * w + tx as usize] = colour;
    };
    for n in &map.nodes {
        mark(n.pos.x, n.pos.y, [255, 220, 0]);
    }
    for s in &map.starts {
        mark(s.x, s.y, [255, 255, 255]);
    }
    ppm(w, h, &pixels)
}
