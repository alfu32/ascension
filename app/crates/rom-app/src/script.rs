//! `--script`: replay pick events on the simulation thread, one per line with a tick number, so
//! a screenshot can show a selection, an open palette or a placement preview without a mouse.
//!
//! ```text
//! # tick command arguments
//! 5   box builders            # box-select the starting builders
//! 8   hud tab 0               # open the BUILD tab
//! 10  hud slot 0              # press the first palette slot
//! 12  hover @8,0              # hover a point relative to the city (tiles)
//! 14  click #7 shift          # click entity 7 with Shift held
//! ```
//!
//! Entities are `city`, `builder1`..`builderN`, `builders` (in `box`) or `#id`; points are
//! `@dx,dy` (tiles from the city centre, across face edges on a sphere) or `x,y` (tiles on the
//! city's face). Other commands: `rclick`,
//! `key <char> [ctrl] [shift]`, `escape`; `hud` takes `tab`, `arrow`, `slot`, `queue`, `tile`.
//!
//! Space commands: `ships` selects every ship of player one, `producers` its finished
//! training buildings (not cities), `orbit <rockN|=body>` and
//! `travel <systemN|=system>` order the selected ships (`rockN` is the N-th nearest resource body
//! of the home system, `systemN` the N-th nearest neighbouring system), `speed <percent>` sets
//! the game speed. Two test-only lines take effect before the match starts, whatever their tick:
//! `grant-age <age key>` and `grant <resource key> <amount>` for player one.

use rom_render::{HudRegionKind, Modifiers, PickEvent, PointerButton, SurfacePos};

use crate::match_setup::PlayerStart;

/// What the script may refer to by name.
#[derive(Clone, Debug)]
pub struct ScriptContext {
    /// The starting city's entity id.
    pub city: u32,
    /// The city centre in tiles.
    pub city_pos: (f32, f32),
    /// The face the city stands on (0 on a flat map).
    pub city_face: u8,
    /// The starting builders' entity ids.
    pub builders: Vec<u32>,
    /// Resource bodies of the home system, nearest to the home planet first (universe worlds).
    pub rocks: Vec<u32>,
    /// Systems joined to the home system by a route, shortest route first (universe worlds).
    pub systems: Vec<u32>,
}

/// What a script does to the match before it starts (test only).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScriptSetup {
    /// The age every player starts in, by key.
    pub age: Option<String>,
    /// Resources granted to player one: key and amount.
    pub grants: Vec<(String, i64)>,
}

impl ScriptSetup {
    /// Applies parsed resource grants before the first simulation tick.
    pub fn apply_grants(&self, world: &mut rom_sim::World) -> Result<(), String> {
        for (key, amount) in &self.grants {
            let resource = world
                .rules()
                .id_of(key)
                .ok_or_else(|| format!("unknown resource {key:?}"))?;
            world
                .grant(crate::selection::PLAYER, resource, *amount)
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

/// A script step that is not a pick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptAction {
    /// Select all of player one's ships.
    SelectShips,
    /// Select player one's finished buildings that train units other than from a city.
    SelectProducers,
    /// Select finished producers that offer track research.
    SelectResearchers,
    /// Set the game speed in percent.
    Speed(u32),
}

impl ScriptContext {
    /// The names of one player's start.
    pub fn of(start: &PlayerStart) -> Self {
        Self {
            city: start.city.0,
            city_pos: (
                start.city_pos.x as f32 / 1000.0,
                start.city_pos.y as f32 / 1000.0,
            ),
            city_face: start.city_pos.face,
            builders: start.builders.iter().map(|b| b.0).collect(),
            rocks: Vec::new(),
            systems: Vec::new(),
        }
    }
}

/// A parsed script: pick events in tick order.
#[derive(Clone, Debug, Default)]
pub struct Script {
    events: Vec<(u64, PickEvent)>,
    next: usize,
    actions: Vec<(u64, ScriptAction)>,
    next_action: usize,
}

impl Script {
    /// Parses a script; an error names the line.
    pub fn parse(
        text: &str,
        context: &ScriptContext,
        offset: &dyn Fn(i64, i64) -> SurfacePos,
    ) -> Result<Self, String> {
        let mut events = Vec::new();
        let mut actions = Vec::new();
        for (number, line) in text.lines().enumerate() {
            let line = line.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let words: Vec<&str> = line.split_whitespace().collect();
            let tick = words.first().and_then(|t| t.parse::<u64>().ok());
            match (tick, words.get(1).copied()) {
                (Some(_), Some("grant-age" | "grant")) => continue,
                (Some(tick), Some("ships")) => {
                    actions.push((tick, ScriptAction::SelectShips));
                    continue;
                }
                (Some(tick), Some("researchers")) => {
                    actions.push((tick, ScriptAction::SelectResearchers));
                    continue;
                }
                (Some(tick), Some("producers")) => {
                    actions.push((tick, ScriptAction::SelectProducers));
                    continue;
                }
                (Some(tick), Some("speed")) => {
                    let percent = words
                        .get(2)
                        .and_then(|p| p.parse::<u32>().ok())
                        .filter(|p| (1..=10_000).contains(p))
                        .ok_or_else(|| format!("line {}: speed takes 1 to 10000", number + 1))?;
                    actions.push((tick, ScriptAction::Speed(percent)));
                    continue;
                }
                _ => {}
            }
            let event = parse_line(line, context, offset)
                .map_err(|error| format!("line {}: {error}", number + 1))?;
            events.push(event);
        }
        events.sort_by_key(|(tick, _)| *tick);
        actions.sort_by_key(|(tick, _)| *tick);
        Ok(Self {
            events,
            next: 0,
            actions,
            next_action: 0,
        })
    }

    /// Non-pick steps whose tick has come, each returned once.
    pub fn due_actions(&mut self, tick: u64) -> Vec<ScriptAction> {
        let mut out = Vec::new();
        while let Some((at, action)) = self.actions.get(self.next_action) {
            if *at > tick {
                break;
            }
            out.push(*action);
            self.next_action += 1;
        }
        out
    }

    /// Events whose tick has come, each returned once.
    pub fn due(&mut self, tick: u64) -> Vec<PickEvent> {
        let mut out = Vec::new();
        while let Some((at, event)) = self.events.get(self.next) {
            if *at > tick {
                break;
            }
            out.push(event.clone());
            self.next += 1;
        }
        out
    }
}

fn entity(word: &str, context: &ScriptContext) -> Result<u32, String> {
    if word == "city" {
        return Ok(context.city);
    }
    if let Some(n) = word.strip_prefix("builder") {
        let n: usize = n
            .parse()
            .map_err(|_| format!("bad builder number in {word:?}"))?;
        return context
            .builders
            .get(n.wrapping_sub(1))
            .copied()
            .ok_or_else(|| format!("there is no {word}"));
    }
    word.strip_prefix('#')
        .and_then(|n| n.parse().ok())
        .ok_or_else(|| format!("expected an entity (city, builderN, #id), got {word:?}"))
}

fn point(
    word: &str,
    context: &ScriptContext,
    offset: &dyn Fn(i64, i64) -> SurfacePos,
) -> Result<((f32, f32), SurfacePos), String> {
    let (relative, rest) = match word.strip_prefix('@') {
        Some(rest) => (true, rest),
        None => (false, word),
    };
    let (x, y) = rest
        .split_once(',')
        .ok_or_else(|| format!("expected a point x,y or @dx,dy, got {word:?}"))?;
    let parse = |t: &str| {
        t.parse::<f32>()
            .map_err(|_| format!("bad number {t:?} in {word:?}"))
    };
    let (x, y) = (parse(x)?, parse(y)?);
    if relative {
        let surface = offset((x * 1000.0).round() as i64, (y * 1000.0).round() as i64);
        Ok((
            (surface.x as f32 / 1000.0, surface.y as f32 / 1000.0),
            surface,
        ))
    } else {
        let surface = SurfacePos {
            face: context.city_face,
            x: (x * 1000.0).round() as i64,
            y: (y * 1000.0).round() as i64,
        };
        Ok(((x, y), surface))
    }
}

fn parse_line(
    line: &str,
    context: &ScriptContext,
    offset: &dyn Fn(i64, i64) -> SurfacePos,
) -> Result<(u64, PickEvent), String> {
    let mut words = line.split_whitespace();
    let tick: u64 = words
        .next()
        .and_then(|t| t.parse().ok())
        .ok_or("a line starts with a tick number")?;
    let command = words.next().ok_or("missing command")?;
    let rest: Vec<&str> = words.collect();
    let mut modifiers = Modifiers::default();
    let mut double = false;
    let mut button = PointerButton::Left;
    let mut args = Vec::new();
    for word in &rest {
        match *word {
            "shift" => modifiers.shift = true,
            "ctrl" => modifiers.ctrl = true,
            "double" => double = true,
            "left" => button = PointerButton::Left,
            "right" => button = PointerButton::Right,
            other => args.push(other),
        }
    }
    let arg = |n: usize| {
        args.get(n)
            .copied()
            .ok_or_else(|| format!("{command} needs more arguments"))
    };
    let event = match command {
        "box" => {
            let list = arg(0)?;
            let entities = if list == "builders" {
                context.builders.clone()
            } else {
                list.split(',')
                    .map(|w| entity(w, context))
                    .collect::<Result<_, _>>()?
            };
            PickEvent::BoxSelect {
                entities,
                modifiers,
            }
        }
        "click" | "rclick" => {
            let target = arg(0)?;
            let ((world_point, surface), entity) = match entity(target, context) {
                Ok(id) => (
                    (
                        context.city_pos,
                        SurfacePos {
                            face: context.city_face,
                            x: (context.city_pos.0 * 1000.0).round() as i64,
                            y: (context.city_pos.1 * 1000.0).round() as i64,
                        },
                    ),
                    Some(id),
                ),
                Err(_) => (point(target, context, offset)?, None),
            };
            if command == "click" {
                PickEvent::Click {
                    world_point,
                    surface,
                    entity,
                    modifiers,
                    double,
                    on_screen: Vec::new(),
                }
            } else {
                PickEvent::RightClick {
                    world_point,
                    surface,
                    entity,
                    modifiers,
                }
            }
        }
        "hud" => {
            let region = match arg(0)? {
                "tab" => HudRegionKind::PaletteTab,
                "data" => HudRegionKind::DataTab,
                "arrow" => HudRegionKind::PaletteArrow,
                "slot" => HudRegionKind::CommandSlot,
                "queue" => HudRegionKind::QueueItem,
                "tile" => HudRegionKind::SelectionTile,
                other => return Err(format!("unknown HUD region {other:?}")),
            };
            let index = arg(1)?
                .parse()
                .map_err(|_| "the HUD index must be a number".to_owned())?;
            PickEvent::HudClick {
                region,
                index,
                button,
                modifiers,
            }
        }
        "hover" => {
            let (world_point, surface) = point(arg(0)?, context, offset)?;
            PickEvent::Hover {
                world_point,
                surface,
            }
        }
        "key" => {
            let mut chars = arg(0)?.chars();
            match (chars.next(), chars.next()) {
                (Some(ch), None) => PickEvent::Hotkey {
                    ch: ch.to_ascii_lowercase(),
                    modifiers,
                },
                _ => return Err("key takes one character".into()),
            }
        }
        "escape" => PickEvent::Escape,
        "orbit" | "travel" => {
            let word = arg(0)?;
            let (prefix, list) = if command == "orbit" {
                ("rock", &context.rocks)
            } else {
                ("system", &context.systems)
            };
            let id = match word.strip_prefix('=') {
                Some(n) => n.parse().ok(),
                None => word
                    .strip_prefix(prefix)
                    .and_then(|n| n.parse::<usize>().ok())
                    .and_then(|n| list.get(n.wrapping_sub(1)).copied()),
            }
            .ok_or_else(|| format!("{command} needs {prefix}N or =id, got {word:?}"))?;
            PickEvent::SpaceRightClick {
                surface: SurfacePos {
                    face: 0,
                    x: 0,
                    y: 0,
                },
                entity: None,
                body: (command == "orbit").then_some(id),
                system: (command == "travel").then_some(id),
                modifiers,
            }
        }
        other => return Err(format!("unknown command {other:?}")),
    };
    Ok((tick, event))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> ScriptContext {
        ScriptContext {
            city: 1,
            city_pos: (100.0, 50.0),
            city_face: 0,
            builders: vec![2, 3, 4],
            rocks: vec![40, 41],
            systems: vec![7],
        }
    }

    /// A flat map: offsets from the city at (100, 50) tiles.
    fn offset(dx: i64, dy: i64) -> SurfacePos {
        SurfacePos {
            face: 0,
            x: 100_000 + dx,
            y: 50_000 + dy,
        }
    }

    #[test]
    fn parses_events_and_replays_each_once_in_tick_order() {
        let text = "
            # comment
            12 hover @8,-2
            5 box builders shift
            8 hud slot 3 right
            9 click city double
            10 key Q ctrl
            11 escape
        ";
        let mut script = Script::parse(text, &context(), &offset).expect("script");
        assert!(script.due(4).is_empty());
        assert_eq!(
            script.due(5),
            vec![PickEvent::BoxSelect {
                entities: vec![2, 3, 4],
                modifiers: Modifiers {
                    shift: true,
                    ..Modifiers::default()
                },
            }]
        );
        let later = script.due(100);
        assert_eq!(later.len(), 5);
        assert_eq!(
            later[0],
            PickEvent::HudClick {
                region: HudRegionKind::CommandSlot,
                index: 3,
                button: PointerButton::Right,
                modifiers: Modifiers::default(),
            }
        );
        assert!(matches!(
            later[1],
            PickEvent::Click {
                entity: Some(1),
                double: true,
                ..
            }
        ));
        assert_eq!(
            later[4],
            PickEvent::Hover {
                world_point: (108.0, 48.0),
                surface: SurfacePos {
                    face: 0,
                    x: 108_000,
                    y: 48_000,
                },
            }
        );
        assert!(script.due(1000).is_empty());
    }

    #[test]
    fn space_commands_and_setup_lines() {
        let text = "
            0 grant-age age.starfaring
            0 grant res.metal 900
            5 ships
            6 speed 400
            7 orbit rock2
            8 travel system1
            9 orbit =12
        ";
        let setup = ScriptSetup::scan(text).unwrap();
        assert_eq!(setup.age.as_deref(), Some("age.starfaring"));
        assert_eq!(setup.grants, vec![("res.metal".to_owned(), 900)]);
        let mut script = Script::parse(text, &context(), &offset).unwrap();
        assert_eq!(
            script.due_actions(100),
            vec![ScriptAction::SelectShips, ScriptAction::Speed(400)]
        );
        let events = script.due(100);
        assert!(matches!(
            events[0],
            PickEvent::SpaceRightClick {
                body: Some(41),
                system: None,
                ..
            }
        ));
        assert!(matches!(
            events[1],
            PickEvent::SpaceRightClick {
                body: None,
                system: Some(7),
                ..
            }
        ));
        assert!(matches!(
            events[2],
            PickEvent::SpaceRightClick { body: Some(12), .. }
        ));
        assert!(Script::parse("1 orbit rock9", &context(), &offset).is_err());
        assert!(ScriptSetup::scan("1 grant res.food").is_err());
    }

    #[test]
    fn errors_name_the_line() {
        let error = Script::parse("1 escape\n2 frobnicate", &context(), &offset).unwrap_err();
        assert!(error.starts_with("line 2:"), "{error}");
        assert!(Script::parse("x escape", &context(), &offset).is_err());
        assert!(Script::parse("3 click builder9", &context(), &offset).is_err());
    }
}
