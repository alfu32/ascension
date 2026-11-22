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
//! `key <char> [ctrl] [shift]`, `escape`; `hud` takes `tab`, `arrow`, `slot`, `queue`, `tile`,
//! `family` (a brush family listed beside the palette, task 114).
//!
//! Live names (resolved at the step, in entity id order): `relicN` = free carry nodes,
//! `templeN` = own completed keepers, `marketN` = own completed buildings training route
//! traders, `cityN` = own completed cities, `carrierN` = own claim-tag units,
//! `traderN` = own route traders, `gatherN` = own completed gather buildings, `shipN` = own orbiting ships, `landedN` = own landed ships, `enemyN` = enemy non-city buildings
//! on the home planet, `foecityN` = enemy cities there (task 75), `armyN` = own armed units that
//! do not build and `foeN` = enemy units on the home planet (task 125). `click` and `rclick`
//! support these names and Shift.
//! These component-based aliases replace a dependency on the last trained type.
//!
//! Space commands: `ships` selects every ship of player one, `producers` its finished
//! training buildings (not cities), `orbit <rockN|=body>` and
//! `travel <systemN|=system>` order the selected ships (`rockN` is the N-th nearest resource body
//! of the home system, `systemN` the N-th nearest neighbouring system), `speed <percent>` sets
//! the game speed. Test-only setup lines take effect before the match starts, whatever their tick:
//! `grant-age <age key>`, `grant <resource key> <amount>` for player one,
//! `place <owner id> <entity key> @dx,dy` to place a test scene before tick one, and
//! `place-space <owner id> <entity key> dx,dy` to place a ship in the plane of player one's home
//! system, `dx`, `dy` tiles from its star, and `place-orbit <owner id> <entity key> @dx,dy` to
//! put a ship into the orbit of player one's home planet above that point (an ordinary `orbit`
//! order given before tick one; an orbital craft is placed there and ordered to climb);
//! `jump-in <owner id> <entity key>` places a ship at the rim of the system nearest to player
//! one's home system by route, on its side, and orders it to player one's home planet along the
//! route (an ordinary `travel` order given before tick one: it charges, jumps and arrives beside
//! the planet, tasks 104 and 111);
//! `colony <owner id>` founds a city for that owner on the nearest other habitable planet of
//! player one's home system, generating the planet first (task 68's freighter route), and
//! `auto-colony <owner id>` puts a finished automated colony there instead (task 73). `craft` selects player one's orbital craft that are not
//! aboard a carrier; `craftN` names the N-th of them for `click` and `rclick`, `aircraftN` the N-th
//! of player one's aircraft (task 123).
//!
//! `save <name>`, `quicksave` and `menu [main|save|load]` write a save or open the in-match menu;
//! `end-page report` opens the match report instead of the end dialog when the match ends
//! (task 117).
//!
//! Camera commands replay input on the render thread exactly as the window would send it
//! (positions in canvas pixels): `cam wheel <steps> [x,y]` (each step glides, as one notch of
//! the wheel does), `cam drag x,y x,y [shift]` (middle-button drag; `shift`, or the older
//! `rotate`, holds Shift: since task 74 the plain drag turns the view in space and the Shift
//! drag pans it), `cam dblclick x,y`, `cam click x,y`, `cam point
//! x,y` (move the pointer only), `cam key <name> <down|up>` (a camera key by name: the arrows
//! `up`, `down`, `left`, `right`, `shift+up` and `shift+down` to fly up and down the star
//! map's screen, `f4` to level the star-map view, `end`, `home`, `f2`, `f3`), `cam home`,
//! `cam focus` and `cam follow` (the focus and follow keys), `cam sky` (F2, the "Sky from the
//! Sun" preset of the star map) and `cam lines` (F3, the constellation figures).

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
    /// The other habitable planets of the home system, nearest to the home planet first.
    pub planets: Vec<u32>,
}

pub use rom_match::ScriptSetup;

/// A script step that is not a pick.
#[derive(Clone, Debug, PartialEq)]
pub enum ScriptAction {
    /// Select all of player one's ships.
    SelectShips,
    /// Select player one's orbital craft that are not aboard a carrier.
    SelectCraft,
    /// Select player one's finished buildings that train units other than from a city.
    SelectProducers,
    /// Select finished producers that offer track research.
    SelectResearchers,
    /// Resolve a named entity against the live world and replay a click.
    EntityPick {
        target: String,
        right: bool,
        shift: bool,
    },
    /// Set the game speed in percent.
    Speed(u32),
    /// Write a named save.
    Save(String),
    /// Write a quick save.
    QuickSave,
    /// Open a page of the in-match menu.
    Menu(crate::sim_thread::MenuPage),
    /// Input for the render thread, as the window would send it.
    Input(Vec<rom_render::InputEvent>),
    /// The player orders the ascension at its library (task 76).
    Ascend(rom_sim::PlayerId),
    /// The person writes a line to a channel (tasks 76 and 94).
    Chat(crate::chat::Channel, String),
    /// A line arrives from another player: sender, channel, words (task 94).
    ChatFrom(rom_sim::PlayerId, crate::chat::Channel, String),
    /// Keys for the chat: named keys, or the characters of a text (task 94).
    ChatKeys(Vec<crate::chat::ChatKey>),
    /// The camera flies to a body and down to its globe (`visit rockN|planetN|=id`, task 77).
    Visit(u32),
    /// The camera turns the planet shown to centre a tile (`look F:x,y`, task 77).
    Look(SurfacePos),
}

/// Orders the ascension at a finished library of `player` (task 76): what pressing the button
/// does.
pub fn ascend(
    world: &mut crate::selection::OrderSink<'_>,
    player: rom_sim::PlayerId,
) -> Result<(), String> {
    let rules = std::sync::Arc::clone(world.rules());
    let (library, tech) = world
        .entities()
        .filter(|e| e.owner == player && e.build_progress.is_none())
        .find_map(|e| {
            let row = rules
                .production
                .get(&e.type_id)?
                .iter()
                .find(|p| rules.ascends(p.product_def_id))?;
            Some((e.id, row.product_def_id))
        })
        .ok_or("ascend: the player has no library")?;
    world
        .submit(rom_sim::Command {
            player,
            executor: library,
            kind: "research".into(),
            subject: Some(tech),
            target_entity: None,
            target_pos: None,
            queue: false,
        })
        .map(|_| ())
        .map_err(|e| format!("ascend: {e}"))
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
            planets: Vec::new(),
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
    /// When the match ends, open the report at once instead of the end dialog (task 117).
    pub end_report: bool,
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
        let mut end_report = false;
        for (number, line) in text.lines().enumerate() {
            let line = line.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let words: Vec<&str> = line.split_whitespace().collect();
            let tick = words.first().and_then(|t| t.parse::<u64>().ok());
            match (tick, words.get(1).copied()) {
                (
                    Some(_),
                    Some(
                        "grant-age" | "grant" | "grant-tech" | "place" | "place-space"
                        | "place-orbit" | "colony" | "colony-far" | "auto-colony"
                        | "ascension-ready" | "research-all" | "jump-in",
                    ),
                ) => {
                    continue;
                }
                (Some(_), Some("end-page")) => {
                    end_report = match words.get(2).copied() {
                        Some("report") => true,
                        Some("dialog") => false,
                        other => {
                            return Err(format!(
                                "line {}: end-page takes dialog or report, not {other:?}",
                                number + 1
                            ));
                        }
                    };
                    continue;
                }
                (Some(tick), Some("cam")) => {
                    let input = camera_input(&words[2..])
                        .map_err(|error| format!("line {}: {error}", number + 1))?;
                    actions.push((tick, ScriptAction::Input(input)));
                    continue;
                }
                (Some(tick), Some("ships")) => {
                    actions.push((tick, ScriptAction::SelectShips));
                    continue;
                }
                (Some(tick), Some("craft")) => {
                    actions.push((tick, ScriptAction::SelectCraft));
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
                (Some(tick), Some("save")) => {
                    let name = words[2..].join(" ");
                    actions.push((tick, ScriptAction::Save(name)));
                    continue;
                }
                (Some(tick), Some("menu")) => {
                    use crate::sim_thread::MenuPage;
                    let page = match words.get(2).copied() {
                        None | Some("main") => MenuPage::Main,
                        Some("save") => MenuPage::Save,
                        Some("load") => MenuPage::Load,
                        Some(other) => {
                            return Err(format!(
                                "line {}: unknown menu page {other:?}",
                                number + 1
                            ));
                        }
                    };
                    actions.push((tick, ScriptAction::Menu(page)));
                    continue;
                }
                (Some(tick), Some("ascend")) => {
                    let player = words
                        .get(2)
                        .and_then(|p| p.parse::<u32>().ok())
                        .ok_or_else(|| format!("line {}: ascend needs a player id", number + 1))?;
                    actions.push((tick, ScriptAction::Ascend(rom_sim::PlayerId(player))));
                    continue;
                }
                (Some(tick), Some("look")) => {
                    let (_, at) = point(words.get(2).copied().unwrap_or(""), context, offset)
                        .map_err(|error| format!("line {}: {error}", number + 1))?;
                    actions.push((tick, ScriptAction::Look(at)));
                    continue;
                }
                (Some(tick), Some("visit")) => {
                    let word = words.get(2).copied().unwrap_or("");
                    let nth = |prefix: &str, list: &[u32]| {
                        word.strip_prefix(prefix)
                            .and_then(|n| n.parse::<usize>().ok())
                            .and_then(|n| list.get(n.wrapping_sub(1)).copied())
                    };
                    let body = word
                        .strip_prefix('=')
                        .and_then(|n| n.parse().ok())
                        .or_else(|| nth("rock", &context.rocks))
                        .or_else(|| nth("planet", &context.planets))
                        .ok_or_else(|| {
                            format!("line {}: visit needs rockN, planetN or =id", number + 1)
                        })?;
                    actions.push((tick, ScriptAction::Visit(body)));
                    continue;
                }
                (Some(tick), Some("chat")) => {
                    // `chat [all|allies|celestial] words`; without a channel, the celestial one.
                    let (channel, rest) =
                        match words.get(2).and_then(|w| crate::chat::Channel::from_key(w)) {
                            Some(channel) => (channel, &words[3..]),
                            None => (crate::chat::Channel::Celestial, &words[2..]),
                        };
                    actions.push((tick, ScriptAction::Chat(channel, rest.join(" "))));
                    continue;
                }
                (Some(tick), Some("chatfrom")) => {
                    let from = words.get(2).and_then(|p| p.parse::<u32>().ok());
                    let channel = words.get(3).and_then(|w| crate::chat::Channel::from_key(w));
                    let (Some(from), Some(channel)) = (from, channel) else {
                        return Err(format!(
                            "line {}: chatfrom needs a player id, a channel and words",
                            number + 1
                        ));
                    };
                    actions.push((
                        tick,
                        ScriptAction::ChatFrom(
                            rom_sim::PlayerId(from),
                            channel,
                            words[4..].join(" "),
                        ),
                    ));
                    continue;
                }
                // `type words`: the characters go to the chat as key presses.
                (Some(tick), Some("type")) => {
                    let keys = words[2..]
                        .join(" ")
                        .chars()
                        .map(crate::chat::ChatKey::Char)
                        .collect();
                    actions.push((tick, ScriptAction::ChatKeys(keys)));
                    continue;
                }
                // `key enter|escape|tab|backspace|left|right|home|end|f7|...`: a named key.
                (Some(tick), Some("key"))
                    if words.get(2).is_some_and(|w| {
                        w.chars().count() > 1 && crate::chat::ChatKey::from_name(w).is_some()
                    }) =>
                {
                    let key = crate::chat::ChatKey::from_name(words[2]).expect("checked above");
                    actions.push((tick, ScriptAction::ChatKeys(vec![key])));
                    continue;
                }
                (Some(tick), Some("quicksave")) => {
                    actions.push((tick, ScriptAction::QuickSave));
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
            if let (Some(tick), Some(command @ ("click" | "rclick"))) =
                (tick, words.get(1).copied())
                && let Some(target) = words.get(2).filter(|word| dynamic_name(word))
            {
                actions.push((
                    tick,
                    ScriptAction::EntityPick {
                        target: (*target).to_owned(),
                        right: command == "rclick",
                        shift: words.contains(&"shift"),
                    },
                ));
                continue;
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
            end_report,
        })
    }

    /// Non-pick steps whose tick has come, each returned once.
    pub fn due_actions(&mut self, tick: u64) -> Vec<ScriptAction> {
        let mut out = Vec::new();
        while let Some((at, action)) = self.actions.get(self.next_action) {
            if *at > tick {
                break;
            }
            out.push(action.clone());
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
    // `F:x,y`: tile x, y of face F of the planet shown (task 77).
    let (face, rest) = match rest.split_once(':') {
        Some((face, rest)) if !relative => (
            Some(
                face.parse::<u8>()
                    .ok()
                    .filter(|f| *f < 6)
                    .ok_or_else(|| format!("bad face in {word:?}"))?,
            ),
            rest,
        ),
        _ => (None, rest),
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
            face: face.unwrap_or(context.city_face),
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
                "top" => HudRegionKind::TopButton,
                "chat" => HudRegionKind::ChatLine,
                "arrow" => HudRegionKind::PaletteArrow,
                "slot" => HudRegionKind::CommandSlot,
                "queue" => HudRegionKind::QueueItem,
                "family" => HudRegionKind::PaletteGroup,
                "tile" => HudRegionKind::SelectionTile,
                "inside" => HudRegionKind::InsideWorker,
                "insideall" => HudRegionKind::InsideWorkerAll,
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
                entity: None,
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
            let nth = |prefix: &str, list: &[u32]| {
                word.strip_prefix(prefix)
                    .and_then(|n| n.parse::<usize>().ok())
                    .and_then(|n| list.get(n.wrapping_sub(1)).copied())
            };
            let id = match word.strip_prefix('=') {
                Some(n) => n.parse().ok(),
                None if command == "orbit" && word.starts_with("planet") => {
                    nth("planet", &context.planets)
                }
                None => nth(prefix, list),
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

/// The input events of a `cam` line.
fn camera_input(words: &[&str]) -> Result<Vec<rom_render::InputEvent>, String> {
    use rom_render::{InputEvent, Key};
    let pixel = |word: &str| -> Result<(f32, f32), String> {
        let (x, y) = word
            .split_once(',')
            .ok_or_else(|| format!("expected x,y in pixels, got {word:?}"))?;
        let parse = |t: &str| {
            t.parse::<f32>()
                .map_err(|_| format!("bad number {t:?} in {word:?}"))
        };
        Ok((parse(x)?, parse(y)?))
    };
    let shift = Modifiers {
        shift: words.contains(&"shift") || words.contains(&"rotate"),
        ..Modifiers::default()
    };
    let at = |n: usize| {
        words
            .get(n)
            .copied()
            .ok_or_else(|| "cam needs more arguments".to_owned())
            .and_then(pixel)
    };
    Ok(match words.first().copied() {
        Some("wheel") => {
            let steps = words
                .get(1)
                .and_then(|w| w.parse::<f32>().ok())
                .ok_or("cam wheel needs a number of steps")?;
            let mut events = Vec::new();
            if words.len() > 2 {
                let (x, y) = at(2)?;
                events.push(InputEvent::PointerMoved { x, y });
            }
            events.push(InputEvent::Wheel { steps });
            events
        }
        Some("drag") => {
            let ((x0, y0), (x1, y1)) = (at(1)?, at(2)?);
            let mut events = vec![
                InputEvent::PointerMoved { x: x0, y: y0 },
                InputEvent::ButtonDown {
                    button: PointerButton::Middle,
                    x: x0,
                    y: y0,
                    mods: shift,
                },
            ];
            for step in 1..=8 {
                let t = step as f32 / 8.0;
                events.push(InputEvent::PointerMoved {
                    x: x0 + (x1 - x0) * t,
                    y: y0 + (y1 - y0) * t,
                });
            }
            events.push(InputEvent::ButtonUp {
                button: PointerButton::Middle,
                x: x1,
                y: y1,
                mods: shift,
            });
            events
        }
        Some("dblclick") => {
            let (x, y) = at(1)?;
            let mods = Modifiers::default();
            let button = PointerButton::Left;
            vec![
                InputEvent::PointerMoved { x, y },
                InputEvent::ButtonDown { button, x, y, mods },
                InputEvent::ButtonUp { button, x, y, mods },
                InputEvent::ButtonDown { button, x, y, mods },
                InputEvent::ButtonUp { button, x, y, mods },
            ]
        }
        Some("key") => {
            let key = words.get(1).and_then(|w| Key::from_name(w)).ok_or(
                "cam key needs a camera key: up, down, left, right, shift+up, shift+down, f4",
            )?;
            match words.get(2).copied() {
                Some("down") => vec![InputEvent::KeyDown(key)],
                Some("up") => vec![InputEvent::KeyUp(key)],
                _ => return Err("cam key needs down or up".into()),
            }
        }
        Some("point") => {
            let (x, y) = at(1)?;
            vec![InputEvent::PointerMoved { x, y }]
        }
        Some("click") => {
            let (x, y) = at(1)?;
            let button = PointerButton::Left;
            vec![
                InputEvent::PointerMoved { x, y },
                InputEvent::ButtonDown {
                    button,
                    x,
                    y,
                    mods: shift,
                },
                InputEvent::ButtonUp {
                    button,
                    x,
                    y,
                    mods: shift,
                },
            ]
        }
        Some("home") => vec![InputEvent::KeyDown(Key::FrameMap)],
        Some("focus") => vec![InputEvent::KeyDown(Key::FocusSelection)],
        Some("follow") => vec![InputEvent::KeyDown(Key::FollowSelection)],
        Some("sky") => vec![InputEvent::KeyDown(Key::SkyFromSun)],
        Some("lines") => vec![InputEvent::KeyDown(Key::ToggleConstellations)],
        other => return Err(format!("unknown camera command {other:?}")),
    })
}

fn dynamic_name(word: &str) -> bool {
    [
        "relic", "temple", "market", "city", "carrier", "trader", "ship", "enemy", "gather",
        "landing", "colony", "landed", "foecity", "aircraft", "army", "foe",
    ]
    .iter()
    .any(|prefix| {
        word.strip_prefix(prefix)
            .is_some_and(|n| n.parse::<usize>().is_ok())
    })
}

/// Ground points named by the live world: `landingN` is open ground near the point below
/// player one's N-th carrier in orbit (within forty tiles), `colonyN` a city site nearest its N-th landed ship.
fn live_point(
    world: &rom_sim::World,
    player: rom_sim::PlayerId,
    target: &str,
) -> Result<Option<rom_sim::Pos>, String> {
    let nth = |prefix: &str, pick: &dyn Fn(&rom_sim::Entity) -> bool| {
        let n: usize = target.strip_prefix(prefix)?.parse().ok()?;
        world
            .entities()
            .filter(|e| e.owner == player && pick(e))
            .nth(n.wrapping_sub(1))
            .map(|e| e.id)
    };
    if target.starts_with("landing") {
        let ship = nth("landing", &|e| {
            world.is_carrier_type(e.type_id) && world.orbit(e.id).is_some()
        })
        .ok_or_else(|| format!("there is no {target}"))?;
        let below = world.orbit(ship).expect("orbiting carrier").pos;
        // Below it, else the nearest open ground up to forty tiles away (the ship moves there
        // within its orbit before it lands).
        let rings = [(0, 0)].into_iter().chain((1..=4).flat_map(|r| {
            let d = r * 10_000;
            [
                (d, 0),
                (-d, 0),
                (0, d),
                (0, -d),
                (d, d),
                (-d, d),
                (d, -d),
                (-d, -d),
            ]
        }));
        return rings
            .filter_map(|(dx, dy)| world.landing_site(ship, world.offset(below, dx, dy)))
            .next()
            .map(Some)
            .ok_or_else(|| format!("no open ground below {target}"));
    }
    if target.starts_with("colony") {
        let ship = nth("colony", &|e| world.is_landed(e.id))
            .ok_or_else(|| format!("there is no {target}"))?;
        let at = world.entity(ship).expect("landed ship").pos;
        return world
            .find_city_site(player, at.body, at)
            .map(Some)
            .ok_or_else(|| format!("no city site near {target}"));
    }
    Ok(None)
}

/// Resolve a screenshot alias using components, without bypassing normal pick/order handling.
pub fn live_pick(
    world: &rom_sim::World,
    player: rom_sim::PlayerId,
    target: &str,
    right: bool,
    shift: bool,
) -> Result<PickEvent, String> {
    use rom_rules::{ClaimMode, EntityClass, ProductionMode};
    let rules = world.rules();
    let modifiers = Modifiers {
        shift,
        ..Modifiers::default()
    };
    if let Some(pos) = live_point(world, player, target)? {
        let surface = SurfacePos {
            face: pos.face,
            x: i64::from(pos.x),
            y: i64::from(pos.y),
        };
        let world_point = (pos.x as f32 / 1000.0, pos.y as f32 / 1000.0);
        return Ok(if right {
            PickEvent::RightClick {
                surface,
                world_point,
                entity: None,
                modifiers,
            }
        } else {
            PickEvent::Click {
                surface,
                world_point,
                entity: None,
                modifiers,
                double: false,
                on_screen: Vec::new(),
            }
        });
    }
    let (name, n) = [
        "relic", "temple", "market", "city", "carrier", "trader", "ship", "enemy", "gather",
        "craft", "landed", "foecity", "aircraft", "army", "foe",
    ]
    .iter()
    .find_map(|name| {
        target
            .strip_prefix(name)
            .and_then(|n| n.parse::<usize>().ok())
            .map(|n| (*name, n))
    })
    .ok_or_else(|| format!("unknown live entity {target}"))?;
    let trader = |id| {
        rules.entity_types[&id]
            .trader
            .as_ref()
            .is_some_and(|t| t.runs_routes)
    };
    let entity = world
        .entities()
        .filter(|e| e.container_id.is_none())
        .filter(|e| {
            let et = &rules.entity_types[&e.type_id];
            if name == "relic" {
                return et
                    .node
                    .as_ref()
                    .is_some_and(|n| n.claim_mode == ClaimMode::Carry);
            }
            if name == "enemy" || name == "foecity" || name == "foe" {
                let own = world.players().find(|p| p.id == player).map(|p| p.team);
                let city = et.territory.as_ref().is_some_and(|t| t.is_city);
                let hostile = Some(e.pos.body) == world.home_body(player)
                    && e.owner != rom_sim::PlayerId(0)
                    && world
                        .players()
                        .find(|p| p.id == e.owner)
                        .is_some_and(|p| Some(p.team) != own);
                if name == "foe" {
                    return hostile && et.class == EntityClass::Unit;
                }
                return hostile && et.class == EntityClass::Building && city == (name == "foecity");
            }
            if e.owner != player || e.build_progress.is_some() {
                return false;
            }
            match name {
                "ship" => world.orbit(e.id).is_some(),
                "landed" => world.is_landed(e.id),
                "craft" => world.is_craft_type(e.type_id),
                "aircraft" => world.is_aircraft_type(e.type_id),
                "temple" => et.keeper.is_some(),
                "gather" => et.gather_site.is_some() && et.class == EntityClass::Building,
                "market" => {
                    et.class == EntityClass::Building
                        && rules
                            .production
                            .get(&et.id)
                            .into_iter()
                            .flatten()
                            .any(|r| r.mode == ProductionMode::Train && trader(r.product_def_id))
                }
                "city" => et.territory.as_ref().is_some_and(|t| t.is_city),
                "army" => {
                    et.class == EntityClass::Unit && !et.weapons.is_empty() && et.builder.is_none()
                }
                "trader" => trader(et.id),
                "carrier" => {
                    et.class == EntityClass::Unit
                        && rules.entity_types.values().any(|node| {
                            node.node.as_ref().is_some_and(|node| {
                                node.claim_mode == ClaimMode::Carry
                                    && node
                                        .claim_tag_id
                                        .is_some_and(|tag| rules.has_tag(et.id, tag))
                            })
                        })
                }
                _ => false,
            }
        })
        .nth(n.wrapping_sub(1))
        .ok_or_else(|| format!("there is no {target}"))?;
    let pos = world.orbit(entity.id).map_or(entity.pos, |orbit| orbit.pos);
    let surface = SurfacePos {
        face: pos.face,
        x: i64::from(pos.x),
        y: i64::from(pos.y),
    };
    let world_point = (pos.x as f32 / 1000.0, pos.y as f32 / 1000.0);
    Ok(if right {
        PickEvent::RightClick {
            surface,
            world_point,
            entity: Some(entity.id.0),
            modifiers,
        }
    } else {
        PickEvent::Click {
            surface,
            world_point,
            entity: Some(entity.id.0),
            modifiers,
            double: false,
            on_screen: Vec::new(),
        }
    })
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
            planets: vec![9],
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
                entity: None,
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
        // Task 104: ships sent to jump into the home system are set up, not replayed.
        let jump = "0 jump-in 2 unit.corvette";
        assert_eq!(
            ScriptSetup::scan(jump).unwrap().jump_ins,
            vec![(2, "unit.corvette".to_owned())]
        );
        assert!(Script::parse(jump, &context(), &offset).is_ok());
        assert!(ScriptSetup::scan("0 jump-in 2").is_err());
    }

    #[test]
    fn camera_lines_replay_window_input_and_space_ships_are_set_up() {
        use rom_render::{InputEvent, Key};
        let text = "
            0 place-space 1 unit.corvette 30,-5
            3 cam wheel -2 640,360
            4 cam drag 100,100 300,100 rotate
            5 cam dblclick 50,60
            6 cam key left down
            7 cam focus
        ";
        let setup = ScriptSetup::scan(text).unwrap();
        assert_eq!(
            setup.space_placements,
            vec![(1, "unit.corvette".to_owned(), 30_000, -5_000)]
        );
        let mut script = Script::parse(text, &context(), &offset).unwrap();
        let input = |actions: Vec<ScriptAction>| match actions.as_slice() {
            [ScriptAction::Input(events)] => events.clone(),
            other => panic!("{other:?}"),
        };
        assert_eq!(
            input(script.due_actions(3)),
            vec![
                InputEvent::PointerMoved { x: 640.0, y: 360.0 },
                InputEvent::Wheel { steps: -2.0 }
            ]
        );
        let drag = input(script.due_actions(4));
        assert!(matches!(
            drag.first(),
            Some(InputEvent::PointerMoved { x: 100.0, .. })
        ));
        assert!(matches!(
            drag[1],
            InputEvent::ButtonDown {
                button: PointerButton::Middle,
                mods: Modifiers { shift: true, .. },
                ..
            }
        ));
        assert!(matches!(
            drag.last(),
            Some(InputEvent::ButtonUp { x: 300.0, .. })
        ));
        let presses = input(script.due_actions(5))
            .iter()
            .filter(|e| matches!(e, InputEvent::ButtonDown { .. }))
            .count();
        assert_eq!(presses, 2);
        assert_eq!(
            input(script.due_actions(6)),
            vec![InputEvent::KeyDown(Key::PanLeft)]
        );
        assert_eq!(
            input(script.due_actions(7)),
            vec![InputEvent::KeyDown(Key::FocusSelection)]
        );
        assert!(Script::parse("1 cam spin", &context(), &offset).is_err());
        assert!(ScriptSetup::scan("0 place-space 1 unit.corvette 30").is_err());
    }

    #[test]
    fn errors_name_the_line() {
        let error = Script::parse("1 escape\n2 frobnicate", &context(), &offset).unwrap_err();
        assert!(error.starts_with("line 2:"), "{error}");
        assert!(Script::parse("x escape", &context(), &offset).is_err());
        assert!(Script::parse("3 click builder9", &context(), &offset).is_err());
    }
}
