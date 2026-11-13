//! The `rom mapgen` subcommand.

use std::{fs, path::PathBuf};

use rom_geo::{SPHERE_SCALE, Tile};
use rom_mapgen::{
    BaseTerrains, GeneratedMap, MapRequest, SpherePlanet, SphereRequest, generate, generate_sphere,
};
use rom_rules::{DefId, Layer, Rules};

const USAGE: &str = "usage: rom mapgen --db <rules.db> --style <key> --seed <n> (--players <n> | --factions <key,key,...>) (--size <W>x<H> | --sphere <n>) [--ppm <file>] [--ppm-height <h>] [--net <file>]";

/// Runs the subcommand with the arguments after `mapgen`.
pub fn run(mut args: impl Iterator<Item = String>) -> Result<(), Box<dyn std::error::Error>> {
    let (mut db, mut style, mut seed, mut players, mut size, mut ppm) =
        (None, None, None, None, None, None);
    let mut factions: Option<Vec<String>> = None;
    let (mut sphere, mut net) = (None, None);
    let mut ppm_height = 512_u32;
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
            "--sphere" => sphere = Some(value.parse::<u32>()?),
            "--net" => net = Some(PathBuf::from(value)),
            "--ppm-height" => ppm_height = value.parse::<u32>()?,
            _ => return Err(format!("unknown argument: {flag}").into()),
        }
    }
    let (db, style, seed) = (db.ok_or(USAGE)?, style.ok_or(USAGE)?, seed.ok_or(USAGE)?);
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
    if !(1..=4096).contains(&ppm_height) {
        return Err("ppm height must be 1..=4096".into());
    }
    if let Some(n) = sphere {
        if size.is_some() {
            return Err("use either --size or --sphere".into());
        }
        let map = generate_sphere(
            &rules,
            &SphereRequest {
                style: style_id,
                seed,
                n,
                player_factions,
            },
        )?;
        let fairness = map.fairness(&rules);
        println!(
            "sphere n={n} tiles={} land={}% sea={}% starts={} nodes={} fair={} hash={:016x}",
            map.surface.len(),
            map.layer_share(&rules, Layer::Ground),
            map.layer_share(&rules, Layer::Fluid),
            map.starts.len(),
            map.nodes.len(),
            if fairness.is_empty() { "yes" } else { "no" },
            map.hash()
        );
        for problem in &fairness {
            eprintln!("unfair: {problem}");
        }
        let pixels = sphere_colours(&map);
        if let Some(path) = ppm {
            fs::write(path, sphere_ppm(&map, &pixels, ppm_height))?;
        }
        if let Some(path) = net {
            fs::write(path, net_ppm(&map, &pixels))?;
        }
        return Ok(());
    }
    if net.is_some() {
        return Err("--net requires --sphere".into());
    }
    let (width, height) = size.ok_or(USAGE)?;
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
        .map(|(t, height)| terrain_colour(*t, *height, map.base))
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

fn terrain_colour(t: DefId, height: u8, b: BaseTerrains) -> [u8; 3] {
    if t == b.land {
        let g = (70 + u32::from(height) * 3 / 4).min(255) as u8;
        [g / 3, g, g / 4]
    } else if t == b.shallow {
        [0x6a, 0xb8, 0xe8]
    } else if t == b.deep {
        [0x1a, 0x3a, 0x8a]
    } else if t == b.space {
        [6, 6, 14]
    } else {
        feature_colour(t)
    }
}

fn sphere_colours(map: &SpherePlanet) -> Vec<[u8; 3]> {
    let mut pixels: Vec<_> = map
        .terrain
        .iter()
        .zip(&map.height)
        .map(|(&t, &h)| terrain_colour(t, h, map.base))
        .collect();
    for &(_, p) in &map.nodes {
        pixels[map.surface.index(p.tile())] = [255, 220, 0];
    }
    for p in &map.starts {
        pixels[map.surface.index(p.tile())] = [255; 3];
    }
    pixels
}

// Presentation only: floating-point longitude/latitude never enters generation
// or the deterministic planet hash. Pixel centres avoid duplicated seam columns.
fn projection_direction(x: u32, y: u32, height: u32) -> [i32; 3] {
    let longitude =
        ((f64::from(x) + 0.5) / f64::from(2 * height) * 2.0 - 1.0) * std::f64::consts::PI;
    let latitude = ((f64::from(y) + 0.5) / f64::from(height) - 0.5) * std::f64::consts::PI;
    let (slon, clon) = longitude.sin_cos();
    let (slat, clat) = latitude.sin_cos();
    [slon * clat, slat, clon * clat].map(|v| (v * f64::from(SPHERE_SCALE)).round() as i32)
}

fn sphere_ppm(map: &SpherePlanet, pixels: &[[u8; 3]], height: u32) -> Vec<u8> {
    let width = 2 * height;
    let mut out = format!("P6\n{width} {height}\n255\n").into_bytes();
    for y in 0..height {
        for x in 0..width {
            let tile = map
                .surface
                .tile_at_point(projection_direction(x, y, height));
            out.extend_from_slice(&pixels[map.surface.index(tile)]);
        }
    }
    out
}

fn net_ppm(map: &SpherePlanet, pixels: &[[u8; 3]]) -> Vec<u8> {
    let n = map.surface.n;
    let (width, height) = (4 * n, 3 * n);
    let mut out = format!("P6\n{width} {height}\n255\n").into_bytes();
    for y in 0..height {
        for x in 0..width {
            let face = match (x / n, y / n) {
                (1, 0) => Some(4),
                (0, 1) => Some(3),
                (1, 1) => Some(0),
                (2, 1) => Some(1),
                (3, 1) => Some(2),
                (1, 2) => Some(5),
                _ => None,
            };
            let colour = face
                .map(|face| {
                    pixels[map.surface.index(Tile {
                        face,
                        x: x % n,
                        y: y % n,
                    })]
                })
                .unwrap_or([6, 6, 14]);
            out.extend_from_slice(&colour);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use rom_geo::CubeSurface;
    use rom_rules::build_rules_db;

    #[test]
    fn sphere_exports_have_correct_projection_and_net_layout() {
        let rules = Rules::load(
            &build_rules_db(
                include_str!("../../../db/schema.sql"),
                include_str!("../../../db/seed.sql"),
            )
            .unwrap(),
        )
        .unwrap();
        let mut map = generate_sphere(
            &rules,
            &SphereRequest {
                style: rules.id_of("map.one_world").unwrap(),
                seed: 1,
                n: 32,
                player_factions: vec![rules.id_of("nation.romans").unwrap()],
            },
        )
        .unwrap();
        map.starts.clear();
        map.nodes.clear();
        let surface = CubeSurface { n: 32 };
        let pixels: Vec<_> = (0..surface.len())
            .map(|i| [surface.tile(i).face, 0, 0])
            .collect();
        let h = 20;
        let image = sphere_ppm(&map, &pixels, h);
        let header = format!("P6\n{} {h}\n255\n", 2 * h);
        assert!(image.starts_with(header.as_bytes()));
        assert_eq!(image.len(), header.len() + (2 * h * h * 3) as usize);
        let face_at = |x, y| image[header.len() + ((y * 2 * h + x) * 3) as usize];
        assert_eq!(face_at(h, h / 2), 0);
        assert_eq!(face_at(h + h / 2, h / 2), 1);
        assert_eq!(face_at(h / 2, h / 2), 3);
        assert_eq!(face_at(0, h / 2), 2);
        assert_eq!(face_at(h, 0), 4);
        assert_eq!(face_at(h, h - 1), 5);

        let image = net_ppm(&map, &pixels);
        let header = "P6\n128 96\n255\n";
        assert!(image.starts_with(header.as_bytes()));
        assert_eq!(image.len(), header.len() + 128 * 96 * 3);
        for (face, (cx, cy)) in [(1, 1), (2, 1), (3, 1), (0, 1), (1, 0), (1, 2)]
            .into_iter()
            .enumerate()
        {
            assert_eq!(
                image[header.len() + ((cy * 32 + 16) * 128 + cx * 32 + 16) * 3],
                face as u8
            );
        }
    }
}
