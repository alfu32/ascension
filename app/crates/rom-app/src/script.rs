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
//! `@dx,dy` (tiles from the city centre) or `x,y` (tiles). Other commands: `rclick`,
//! `key <char> [ctrl] [shift]`, `escape`; `hud` takes `tab`, `arrow`, `slot`, `queue`, `tile`.

use rom_render::{HudRegionKind, Modifiers, PickEvent, PointerButton};

use crate::match_setup::PlayerStart;

/// What the script may refer to by name.
#[derive(Clone, Debug)]
pub struct ScriptContext {
    /// The starting city's entity id.
    pub city: u32,
    /// The city centre in tiles.
    pub city_pos: (f32, f32),
    /// The starting builders' entity ids.
    pub builders: Vec<u32>,
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
            builders: start.builders.iter().map(|b| b.0).collect(),
        }
    }
}

/// A parsed script: pick events in tick order.
#[derive(Clone, Debug, Default)]
pub struct Script {
    events: Vec<(u64, PickEvent)>,
    next: usize,
}

impl Script {
    /// Parses a script; an error names the line.
    pub fn parse(text: &str, context: &ScriptContext) -> Result<Self, String> {
        let mut events = Vec::new();
        for (number, line) in text.lines().enumerate() {
            let line = line.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let event = parse_line(line, context)
                .map_err(|error| format!("line {}: {error}", number + 1))?;
            events.push(event);
        }
        events.sort_by_key(|(tick, _)| *tick);
        Ok(Self { events, next: 0 })
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

fn point(word: &str, context: &ScriptContext) -> Result<(f32, f32), String> {
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
    Ok(if relative {
        (context.city_pos.0 + x, context.city_pos.1 + y)
    } else {
        (x, y)
    })
}

fn parse_line(line: &str, context: &ScriptContext) -> Result<(u64, PickEvent), String> {
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
            let (world_point, entity) = match entity(target, context) {
                Ok(id) => (context.city_pos, Some(id)),
                Err(_) => (point(target, context)?, None),
            };
            if command == "click" {
                PickEvent::Click {
                    world_point,
                    entity,
                    modifiers,
                    double,
                    on_screen: Vec::new(),
                }
            } else {
                PickEvent::RightClick {
                    world_point,
                    entity,
                    modifiers,
                }
            }
        }
        "hud" => {
            let region = match arg(0)? {
                "tab" => HudRegionKind::PaletteTab,
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
        "hover" => PickEvent::Hover {
            world_point: point(arg(0)?, context)?,
        },
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
            builders: vec![2, 3, 4],
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
        let mut script = Script::parse(text, &context()).expect("script");
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
                world_point: (108.0, 48.0)
            }
        );
        assert!(script.due(1000).is_empty());
    }

    #[test]
    fn errors_name_the_line() {
        let error = Script::parse("1 escape\n2 frobnicate", &context()).unwrap_err();
        assert!(error.starts_with("line 2:"), "{error}");
        assert!(Script::parse("x escape", &context()).is_err());
        assert!(Script::parse("3 click builder9", &context()).is_err());
    }
}
