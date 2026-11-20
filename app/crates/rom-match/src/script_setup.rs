//! What a script does to a match before its first tick (test setups): ages, grants, test
//! scenes, ships, colonies. Script lines that run during play are inputs, not setup.

use crate::setup::PLAYER_ONE;

/// What a script does to the match before it starts (test only).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScriptSetup {
    /// The age every player starts in, by key.
    pub age: Option<String>,
    /// Resources granted to player one: key and amount.
    pub grants: Vec<(String, i64)>,
    /// Test scene entities: owner, type key and milli-tile offsets from player one's city.
    pub placements: Vec<(u32, String, i64, i64)>,
    /// Test ships: owner, type key and milli-tile offsets from the star of player one's home
    /// system, in that system's plane.
    pub space_placements: Vec<(u32, String, i64, i64)>,
    /// Test ships ordered into the orbit of player one's home planet: owner, type key and
    /// milli-tile offsets from player one's city of the point they orbit above.
    pub orbit_placements: Vec<(u32, String, i64, i64)>,
    /// Test ships placed at the rim of the nearest system by route and sent into player one's
    /// home system (`jump-in <owner id> <entity key>`): owner and type key.
    pub jump_ins: Vec<(u32, String)>,
    /// Technologies player one starts with, by key.
    pub techs: Vec<String>,
    /// Owners given a city on the nearest other habitable planet of player one's home system
    /// (`colony <owner id>`; the planet is generated before tick one).
    pub colonies: Vec<u32>,
    /// Owners given a city on the nearest habitable planet of a star system one route from
    /// player one's home system (`colony-far <owner id>`).
    pub far_colonies: Vec<u32>,
    /// Owners given a finished automated colony on the nearest other habitable planet of
    /// player one's home system (`auto-colony <owner id>`).
    pub auto_colonies: Vec<u32>,
    /// Players that start with every technology the library offers and the price of the
    /// ascension (`ascension-ready <owner id>`, task 76).
    pub ascension_ready: Vec<u32>,
    /// Players that start with every technology they could research in their start age
    /// already researched (`research-all <owner id>`; ascensions and later age advances
    /// excluded).
    pub research_all: Vec<u32>,
}

impl ScriptSetup {
    /// Applies parsed resource grants before the first simulation tick.
    pub fn apply_grants(&self, world: &mut rom_sim::World) -> Result<(), String> {
        for owner in &self.research_all {
            world
                .complete_research_for_setup(rom_sim::PlayerId(*owner))
                .map_err(|e| e.to_string())?;
        }
        for (key, amount) in &self.grants {
            let resource = world
                .rules()
                .id_of(key)
                .ok_or_else(|| format!("unknown resource {key:?}"))?;
            world
                .grant(PLAYER_ONE, resource, *amount)
                .map_err(|e| e.to_string())?;
        }
        self.apply_space_placements(world)?;
        if !self.placements.is_empty() {
            let origin = world
                .entities()
                .find(|e| {
                    e.owner == PLAYER_ONE
                        && world.rules().entity_types[&e.type_id]
                            .territory
                            .as_ref()
                            .is_some_and(|t| t.is_city)
                })
                .map(|e| e.pos)
                .ok_or("test scene needs a starting city")?;
            self.apply_placements(world, origin)?;
        }
        Ok(())
    }

    /// Places explicitly requested test-scene entities before the first tick.
    pub fn apply_placements(
        &self,
        world: &mut rom_sim::World,
        origin: rom_sim::Pos,
    ) -> Result<(), String> {
        for (owner, key, dx, dy) in &self.placements {
            let kind = world
                .rules()
                .id_of(key)
                .filter(|id| world.rules().entity_types.contains_key(id))
                .ok_or_else(|| format!("unknown entity type {key:?}"))?;
            let pos = world.offset(origin, *dx, *dy);
            world
                .place_for_setup(kind, rom_sim::PlayerId(*owner), pos)
                .map_err(|e| e.to_string())?;
        }
        self.apply_orbit_placements(world, origin)
    }

    /// Puts the requested test ships beside the home planet and orders them into its orbit.
    fn apply_orbit_placements(
        &self,
        world: &mut rom_sim::World,
        origin: rom_sim::Pos,
    ) -> Result<(), String> {
        for (owner, key, dx, dy) in &self.orbit_placements {
            let kind = world
                .rules()
                .id_of(key)
                .filter(|id| world.rules().entity_types.contains_key(id))
                .ok_or_else(|| format!("unknown entity type {key:?}"))?;
            let marker = world
                .body_position(origin.body)
                .ok_or("place-orbit needs a universe match")?;
            let owner = rom_sim::PlayerId(*owner);
            let over = world.offset(origin, *dx, *dy);
            // An orbital craft starts in the air at the point and climbs; a ship flies in.
            let (ship, kind, target) = if world.is_craft_type(kind) {
                let craft = world
                    .place_for_setup(kind, owner, over)
                    .map_err(|e| e.to_string())?;
                (craft, "lift", None)
            } else {
                let ship = world
                    .spawn(kind, owner, marker)
                    .map_err(|e| e.to_string())?;
                (ship, "orbit", Some(over))
            };
            world
                .submit(rom_sim::Command {
                    player: owner,
                    executor: ship,
                    kind: kind.into(),
                    subject: None,
                    target_entity: None,
                    target_pos: target,
                    queue: false,
                })
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Places the requested test ships in the plane of player one's home system.
    pub fn apply_space_placements(&self, world: &mut rom_sim::World) -> Result<(), String> {
        self.apply_jump_ins(world)?;
        if self.space_placements.is_empty() {
            return Ok(());
        }
        let star = world
            .home_body(PLAYER_ONE)
            .and_then(|home| world.body(home))
            .and_then(|home| world.space_of(home.system))
            .ok_or("place-space needs a universe match")?;
        let centre = world
            .body_position(star)
            .ok_or("the home star has no position")?;
        for (owner, key, dx, dy) in &self.space_placements {
            let kind = world
                .rules()
                .id_of(key)
                .filter(|id| world.rules().entity_types.contains_key(id))
                .ok_or_else(|| format!("unknown entity type {key:?}"))?;
            let at = rom_sim::Pos {
                x: centre.x + i32::try_from(*dx).map_err(|_| "offset too large")?,
                y: centre.y + i32::try_from(*dy).map_err(|_| "offset too large")?,
                ..centre
            };
            world
                .spawn(kind, rom_sim::PlayerId(*owner), at)
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Places the `jump-in` ships at the rim of the system nearest to player one's home system
    /// by route (the shortest route, then the lowest id), toward home, and sends them to the
    /// home planet.
    fn apply_jump_ins(&self, world: &mut rom_sim::World) -> Result<(), String> {
        if self.jump_ins.is_empty() {
            return Ok(());
        }
        let home = world
            .home_body(PLAYER_ONE)
            .and_then(|home| world.body(home))
            .map(|home| home.system)
            .ok_or("jump-in needs a universe match")?;
        let from = world
            .routes()
            .iter()
            .filter_map(|r| {
                let other = if r.system_a == home {
                    r.system_b
                } else if r.system_b == home {
                    r.system_a
                } else {
                    return None;
                };
                Some((r.length, other))
            })
            .min()
            .map(|(_, other)| other)
            .ok_or("jump-in needs a route from the home system")?;
        let rim = world
            .system_edge(from, home)
            .ok_or("jump-in: no rim toward the home system")?;
        // Into player one's explored home system they come out beside its home planet (task 111).
        let target = world
            .home_body(PLAYER_ONE)
            .and_then(|planet| world.body_position(planet))
            .ok_or("the home planet has no position")?;
        for (owner, key) in &self.jump_ins {
            let kind = world
                .rules()
                .id_of(key)
                .filter(|id| world.rules().entity_types.contains_key(id))
                .ok_or_else(|| format!("unknown entity type {key:?}"))?;
            let owner = rom_sim::PlayerId(*owner);
            let ship = world.spawn(kind, owner, rim).map_err(|e| e.to_string())?;
            world
                .submit(rom_sim::Command {
                    player: owner,
                    executor: ship,
                    kind: "travel".into(),
                    subject: None,
                    target_entity: None,
                    target_pos: Some(target),
                    queue: false,
                })
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Reads the `grant-age` and `grant` lines of a script.
    pub fn scan(text: &str) -> Result<Self, String> {
        let mut setup = Self::default();
        for (number, line) in text.lines().enumerate() {
            let line = line.split('#').next().unwrap_or("").trim();
            let words: Vec<&str> = line.split_whitespace().collect();
            let at = || format!("line {}", number + 1);
            match words.get(1).copied() {
                Some("jump-in") => {
                    let (Some(owner), Some(key)) = (words.get(2), words.get(3)) else {
                        return Err(format!("{}: jump-in needs owner and entity key", at()));
                    };
                    let owner = owner.parse().map_err(|_| format!("{}: bad owner", at()))?;
                    setup.jump_ins.push((owner, (*key).to_owned()));
                }
                Some("place-space") => {
                    let (Some(owner), Some(key), Some(offset)) =
                        (words.get(2), words.get(3), words.get(4))
                    else {
                        return Err(format!(
                            "{}: place-space needs owner, entity key and dx,dy",
                            at()
                        ));
                    };
                    let owner = owner.parse().map_err(|_| format!("{}: bad owner", at()))?;
                    let (dx, dy) = offset
                        .split_once(',')
                        .ok_or_else(|| format!("{}: place-space needs dx,dy", at()))?;
                    let parse = |v: &str| {
                        v.parse::<i64>()
                            .ok()
                            .and_then(|v| v.checked_mul(1000))
                            .ok_or_else(|| format!("{}: bad tile offset", at()))
                    };
                    setup
                        .space_placements
                        .push((owner, (*key).to_owned(), parse(dx)?, parse(dy)?));
                }
                Some(command @ ("place" | "place-orbit")) => {
                    let (Some(owner), Some(key), Some(offset)) =
                        (words.get(2), words.get(3), words.get(4))
                    else {
                        return Err(format!(
                            "{}: place needs owner, entity key and @dx,dy",
                            at()
                        ));
                    };
                    let owner = owner.parse().map_err(|_| format!("{}: bad owner", at()))?;
                    let (dx, dy) = offset
                        .strip_prefix('@')
                        .and_then(|v| v.split_once(','))
                        .ok_or_else(|| format!("{}: place needs @dx,dy", at()))?;
                    let parse = |v: &str| {
                        v.parse::<i64>()
                            .ok()
                            .and_then(|v| v.checked_mul(1000))
                            .ok_or_else(|| format!("{}: bad tile offset", at()))
                    };
                    let list = if command == "place" {
                        &mut setup.placements
                    } else {
                        &mut setup.orbit_placements
                    };
                    list.push((owner, (*key).to_owned(), parse(dx)?, parse(dy)?));
                }
                Some("ascension-ready") => {
                    let owner = words
                        .get(2)
                        .and_then(|o| o.parse().ok())
                        .ok_or_else(|| format!("{}: ascension-ready needs an owner id", at()))?;
                    setup.ascension_ready.push(owner);
                }
                Some("research-all") => {
                    let owner = words
                        .get(2)
                        .and_then(|o| o.parse().ok())
                        .ok_or_else(|| format!("{}: research-all needs an owner id", at()))?;
                    setup.research_all.push(owner);
                }
                Some(kind @ ("colony" | "colony-far" | "auto-colony")) => {
                    let owner = words
                        .get(2)
                        .and_then(|o| o.parse().ok())
                        .ok_or_else(|| format!("{}: {kind} needs an owner id", at()))?;
                    match kind {
                        "colony" => setup.colonies.push(owner),
                        "colony-far" => setup.far_colonies.push(owner),
                        _ => setup.auto_colonies.push(owner),
                    }
                }
                Some("grant-tech") => {
                    setup.techs.push(
                        words
                            .get(2)
                            .ok_or_else(|| format!("{}: grant-tech needs a technology key", at()))?
                            .to_string(),
                    );
                }
                Some("grant-age") => {
                    setup.age = Some(
                        words
                            .get(2)
                            .ok_or_else(|| format!("{}: grant-age needs an age key", at()))?
                            .to_string(),
                    );
                }
                Some("grant") => {
                    let (Some(key), Some(amount)) = (words.get(2), words.get(3)) else {
                        return Err(format!(
                            "{}: grant needs a resource key and an amount",
                            at()
                        ));
                    };
                    let amount = amount
                        .parse()
                        .map_err(|_| format!("{}: bad amount {amount:?}", at()))?;
                    setup.grants.push(((*key).to_owned(), amount));
                }
                _ => {}
            }
        }
        Ok(setup)
    }
}
