//! What the person sees of the ascension (task 76): the event lines when a player starts the
//! project, ascends or descends, the countdowns at the top of the screen, on the star map and in
//! the diplomacy tab, the notice to the last material power, and the celestial home's panel with
//! its residents and the chat channel of the ascended players (computer players included).
//!
//! The chat is the application's, not the simulation's: like a chat between people, it changes
//! nothing in the match. Its log lives here (see `chat.rs`) and a save keeps it beside the
//! match settings, outside the simulation state and its hash.

use std::collections::{BTreeMap, BTreeSet};

use rom_ai::ChatTopic;
use rom_render::{CountdownView, HudPanel, HudSnapshot, ResidenceView, owner_color};
use rom_rules::BodyKind;
use rom_sim::{PlayerId, PlayerStatus, World};

use crate::chat::{self, Channel, ChatInput, ChatLine, ChatLog, ChatSink};

/// The name a player goes by in the event line, the diplomacy tab and the chat.
pub fn player_name(world: &World, me: PlayerId, id: PlayerId) -> String {
    if id == me {
        return "You".to_owned();
    }
    world.players().find(|p| p.id == id).map_or_else(
        || format!("Player {}", id.0),
        |p| format!("{} ({})", world.rules().def(p.faction).name, id.0),
    )
}

/// A player's colour, opaque.
pub fn colour_of(id: PlayerId) -> [u8; 4] {
    let [r, g, b] = owner_color(u8::try_from(id.0).unwrap_or(u8::MAX))
        .map(|c| (c * 255.0).round().clamp(0.0, 255.0) as u8);
    [r, g, b, 255]
}

/// Game ticks as minutes and seconds, "04:10".
pub fn clock(world: &World, ticks: u64) -> String {
    let rate = world
        .rules()
        .constant("sim.ticks_per_second")
        .unwrap_or(20)
        .max(1) as u64;
    let seconds = ticks.div_ceil(rate);
    format!("{:02}:{:02}", seconds / 60, seconds % 60)
}

/// The celestial home's system, if the universe has one.
pub fn celestial_system(world: &World) -> Option<u32> {
    world
        .bodies()
        .iter()
        .find(|b| b.kind == BodyKind::BlackHole)
        .map(|b| b.system)
}

/// "Your ascension" or "Ascension of Romans (2)".
fn project_name(world: &World, me: PlayerId, player: PlayerId) -> String {
    if player == me {
        "Your ascension".to_owned()
    } else {
        format!("Ascension of {}", player_name(world, me, player))
    }
}

/// The ascensions under way, for the star map: beside the system of each library.
pub fn countdowns(world: &World, me: PlayerId) -> Vec<CountdownView> {
    world
        .ascension_projects()
        .into_iter()
        .filter_map(|p| {
            let library = world.entity(p.library)?;
            let system = world.body(library.pos.body)?.system;
            Some(CountdownView {
                system,
                owner: u8::try_from(p.player.0).unwrap_or(u8::MAX),
                text: format!(
                    "{} {}",
                    project_name(world, me, p.player),
                    clock(world, p.remaining())
                ),
            })
        })
        .collect()
}

/// The ascended players living on the celestial home, by player.
pub fn residences(world: &World, me: PlayerId) -> Vec<ResidenceView> {
    world
        .players()
        .filter(|p| world.is_ascended(p.id))
        .map(|p| ResidenceView {
            owner: u8::try_from(p.id.0).unwrap_or(u8::MAX),
            name: player_name(world, me, p.id),
        })
        .collect()
}

/// What changed of the ascension since the last look, and the chat of the celestial home.
#[derive(Debug)]
pub struct AscensionView {
    started: bool,
    /// Projects under way: player and the tick each was ordered.
    projects: BTreeSet<(PlayerId, u64)>,
    ascended: BTreeSet<PlayerId>,
    standing: Option<u32>,
    /// Bodies brought to life (task 77).
    living: BTreeSet<rom_sim::BodyId>,
    /// Who held each resource body at the last look.
    claims: BTreeMap<rom_sim::BodyId, PlayerId>,
    /// The players a computer plays: they speak in the channel.
    computers: BTreeSet<PlayerId>,
    /// Every line the person has seen, oldest first.
    chat: ChatLog,
    /// The keyboard side of the chat, as the window left it.
    pub input: ChatInput,
    /// The person's player.
    me: PlayerId,
}

impl AscensionView {
    /// A view of a match whose `computers` speak for themselves in the channel.
    pub fn new(computers: BTreeSet<PlayerId>, me: PlayerId) -> Self {
        Self {
            started: false,
            projects: BTreeSet::new(),
            ascended: BTreeSet::new(),
            standing: None,
            living: BTreeSet::new(),
            claims: BTreeMap::new(),
            computers,
            chat: ChatLog::for_player(me),
            input: ChatInput::default(),
            me,
        }
    }

    /// The lines the person has seen, oldest first.
    pub fn chat(&self) -> &[ChatLine] {
        self.chat.lines()
    }

    /// The log, to save.
    pub fn log(&self) -> &ChatLog {
        &self.chat
    }

    /// Takes back the log a save kept.
    pub fn restore(&mut self, log: ChatLog) {
        self.chat = log.with_player(self.me);
    }

    /// A line the application speaks into the celestial channel (computer players).
    fn post(&mut self, world: &World, player: PlayerId, text: impl AsRef<str>) {
        chat::deliver(
            world,
            &mut self.chat,
            player,
            Channel::Celestial,
            text.as_ref(),
        );
    }

    /// The person's line to `channel`. Only the celestial channel gets an answer, from the first
    /// ascended computer player. Returns whether it was sent.
    pub fn say(&mut self, world: &World, channel: Channel, text: &str) -> bool {
        self.say_with(world, channel, text, None)
    }

    /// [`Self::say`], also handing the line to `sink` (network play relays it there).
    pub fn say_with(
        &mut self,
        world: &World,
        channel: Channel,
        text: &str,
        sink: Option<&mut dyn ChatSink>,
    ) -> bool {
        if !chat::send(world, &mut self.chat, self.me, channel, text) {
            return false;
        }
        if let Some(sink) = sink {
            chat::send(world, sink, self.me, channel, text);
        }
        if channel == Channel::Celestial
            && let Some(&other) = self.computers.iter().find(|id| world.is_ascended(**id))
        {
            self.post(
                world,
                other,
                rom_ai::chat_line(other, ChatTopic::Reply, world.tick()),
            );
        }
        true
    }

    /// A line arriving from another player, as the network will deliver it.
    pub fn hear(&mut self, world: &World, from: PlayerId, channel: Channel, text: &str) {
        chat::deliver(world, &mut self.chat, from, channel, text);
    }

    /// Event lines for what changed since the last call (the first call only remembers); the
    /// computer players' lines go to the channel.
    pub fn update(&mut self, world: &World) -> Vec<String> {
        let mut lines = Vec::new();
        let projects: BTreeMap<(PlayerId, u64), u64> = world
            .ascension_projects()
            .into_iter()
            .map(|p| ((p.player, p.issued), p.remaining()))
            .collect();
        let ascended: BTreeSet<PlayerId> = world
            .players()
            .filter(|p| world.is_ascended(p.id))
            .map(|p| p.id)
            .collect();
        let standing = world.last_standing();
        if self.started {
            for (&(player, _), &left) in &projects {
                if self.projects.iter().any(|(p, _)| *p == player) {
                    continue;
                }
                lines.push(if player == self.me {
                    format!(
                        "You have begun the Ascension: {} left. Every player is told; hold your library",
                        clock(world, left)
                    )
                } else {
                    format!(
                        "{} has begun the Ascension: {} left. Defeat them first to stop it",
                        player_name(world, self.me, player),
                        clock(world, left)
                    )
                });
            }
            for &(player, _) in &self.projects {
                if projects.keys().any(|(p, _)| *p == player) || ascended.contains(&player) {
                    continue;
                }
                let name = player_name(world, self.me, player);
                lines.push(match world.player_status(player) {
                    Some(PlayerStatus::Defeated) => {
                        format!("{name} fell before the Ascension was complete")
                    }
                    _ => format!("The Ascension of {name} was stopped"),
                });
            }
            let arrived: Vec<PlayerId> = ascended.difference(&self.ascended).copied().collect();
            let left: Vec<PlayerId> = self.ascended.difference(&ascended).copied().collect();
            for player in arrived {
                lines.push(if player == self.me {
                    "You have ascended: your possessions left the material plane. Explore freely; \
                     Descend from the command panel when you wish"
                        .to_owned()
                } else {
                    format!(
                        "{} has ascended and left the material plane",
                        player_name(world, self.me, player)
                    )
                });
                if self.computers.contains(&player) {
                    let line = rom_ai::chat_line(player, ChatTopic::Arrived, world.tick());
                    self.post(world, player, line);
                }
                let greeters: Vec<PlayerId> = self
                    .computers
                    .iter()
                    .copied()
                    .filter(|id| *id != player && self.ascended.contains(id))
                    .collect();
                for greeter in greeters {
                    let line = rom_ai::chat_line(greeter, ChatTopic::Welcome, world.tick());
                    self.post(world, greeter, line);
                }
            }
            for player in left {
                let planet = world
                    .home_body(player)
                    .and_then(|b| world.body(b))
                    .map_or_else(|| "the ground".to_owned(), |b| b.name.clone());
                lines.push(if player == self.me {
                    format!("You have descended on {planet}: a city, citizens and the first age")
                } else {
                    format!(
                        "{} has descended on {planet}",
                        player_name(world, self.me, player)
                    )
                });
                if self.computers.contains(&player) {
                    let line = rom_ai::chat_line(player, ChatTopic::Farewell, world.tick());
                    self.post(world, player, line);
                }
            }
            if standing.is_some() && standing != self.standing {
                let team = world.players().find(|p| p.id == self.me).map(|p| p.team);
                // First, so the news of an ascension or a descent stays on the event line.
                lines.insert(
                    0,
                    if standing == team {
                        "Every rival is defeated or ascended: you are the last material power. The \
                     match goes on: keep playing, ascend at the library, or leave"
                            .to_owned()
                    } else {
                        let names: Vec<String> = world
                            .players()
                            .filter(|p| p.id != PlayerId(0) && Some(p.team) == standing)
                            .map(|p| player_name(world, self.me, p.id))
                            .collect();
                        format!("{} remain the last material power", names.join(", "))
                    },
                );
            }
        }
        // Task 77: a moon or asteroid brought to life; whoever held it is told.
        let living: BTreeSet<rom_sim::BodyId> = world.living_bodies().map(|(b, _)| b).collect();
        if self.started {
            for &body in living.difference(&self.living) {
                let Some(life) = world.living(body) else {
                    continue;
                };
                let name = world
                    .body(body)
                    .map_or_else(|| "a moon".to_owned(), |b| b.name.clone());
                let who = if life.by == self.me {
                    "You have".to_owned()
                } else {
                    format!("{} has", player_name(world, self.me, life.by))
                };
                let claim = match self.claims.get(&body) {
                    Some(&holder) if holder == self.me => ": your claim on it has ended".to_owned(),
                    Some(&holder) => {
                        format!(
                            ": the claim of {} on it has ended",
                            player_name(world, self.me, holder)
                        )
                    }
                    None => ": its resources are gone, its ground is open".to_owned(),
                };
                lines.push(format!("{who} brought {name} to life{claim}"));
            }
        }
        self.living = living;
        self.claims = world.claims().collect();
        self.projects = projects.into_keys().collect();
        self.ascended = ascended;
        self.standing = standing;
        self.started = true;
        lines
    }

    /// The countdowns at the top, the diplomacy tab, the chat line and, while the camera is in
    /// the celestial home's system (`celestial_in_view`), its panel.
    pub fn fill(&self, world: &World, hud: &mut HudSnapshot, celestial_in_view: bool) {
        let projects = world.ascension_projects();
        hud.banner = projects
            .iter()
            .map(|p| {
                (
                    format!(
                        "{}  {}",
                        project_name(world, self.me, p.player).to_uppercase(),
                        clock(world, p.remaining())
                    ),
                    colour_of(p.player),
                )
            })
            .collect();
        hud.diplomacy = world
            .players()
            .filter(|p| p.id != PlayerId(0))
            .flat_map(|p| {
                let state = match (
                    world.player_status(p.id),
                    projects.iter().find(|q| q.player == p.id),
                ) {
                    (_, Some(q)) => format!("ascending {}", clock(world, q.remaining())),
                    (Some(PlayerStatus::Ascended), _) => format!(
                        "ascended{}",
                        match world.ascensions(p.id) {
                            1 => String::new(),
                            n => format!(" ({n}x)"),
                        }
                    ),
                    (Some(PlayerStatus::Defeated), _) => "defeated".to_owned(),
                    (Some(PlayerStatus::Won), _) => "victorious".to_owned(),
                    _ => "playing".to_owned(),
                };
                [
                    (player_name(world, self.me, p.id), colour_of(p.id)),
                    (format!("  {state}"), [232, 201, 138, 255]),
                ]
            })
            .collect();
        let ascended = world.is_ascended(self.me);
        hud.chat = self.chat.view(world, &self.input);
        if !celestial_in_view {
            return;
        }
        let mut lines: Vec<(String, [u8; 4])> = Vec::new();
        let residents: Vec<PlayerId> = world
            .players()
            .filter(|p| world.is_ascended(p.id))
            .map(|p| p.id)
            .collect();
        if residents.is_empty() {
            lines.push(("No one lives here yet".to_owned(), [151, 165, 177, 255]));
        } else {
            lines.push(("Residents:".to_owned(), [151, 165, 177, 255]));
            for id in &residents {
                lines.push((
                    format!("  {}", player_name(world, self.me, *id)),
                    colour_of(*id),
                ));
            }
        }
        if ascended {
            lines.push((
                "Chat: Enter to write, Tab for channels, Enter to send".to_owned(),
                [151, 165, 177, 255],
            ));
            let spoken: Vec<&ChatLine> = self
                .chat
                .lines()
                .iter()
                .filter(|l| l.channel == Channel::Celestial)
                .collect();
            for line in &spoken[spoken.len().saturating_sub(8)..] {
                lines.push((
                    format!(
                        "{}: {}",
                        player_name(world, self.me, line.player),
                        line.text
                    ),
                    colour_of(line.player),
                ));
            }
        } else {
            lines.push((
                "Only the ascended reach it: no route leads here".to_owned(),
                [151, 165, 177, 255],
            ));
        }
        hud.panel = Some(HudPanel {
            title: "EMPYREAN · THE CELESTIAL HOME".to_owned(),
            lines,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::match_setup::build_universe_match;
    use crate::script::ScriptSetup;
    use crate::selection::PLAYER;
    use rom_sim::{Command, EntityId, Pos};

    /// A universe match with player one ready to ascend and a library beside its city.
    fn ready() -> (World, BTreeSet<PlayerId>) {
        let (world, computers, _) = ready_with_universe();
        (world, computers)
    }

    /// [`ready`] with the generated universe.
    fn ready_with_universe() -> (World, BTreeSet<PlayerId>, rom_mapgen::Universe) {
        let options = crate::args::Options {
            universe: true,
            players: 2,
            seed: 1,
            ..crate::args::Options::default()
        };
        let content = crate::terrain::generate_universe_content(&options).expect("universe");
        // The last age by ordinal (task 164), where the ascension is researched.
        let last = content
            .rules
            .ages
            .values()
            .max_by_key(|a| a.ordinal)
            .unwrap();
        let setup = ScriptSetup {
            age: Some(content.rules.key(last.id).to_owned()),
            ascension_ready: vec![1, 2],
            ..ScriptSetup::default()
        };
        let mut game = build_universe_match(
            &content.rules,
            &content.universe,
            &content.factions,
            1,
            None,
            &setup,
        )
        .expect("match");
        let library = content.rules.id_of("bld.library").unwrap();
        for start in &game.starts {
            let at = game.world.offset(start.city_pos, 12_000, 0);
            game.world
                .place_for_setup(library, start.player, at)
                .expect("a library");
        }
        (game.world, [PlayerId(2)].into(), content.universe)
    }

    /// Task 77: the person ascends, presses Bring life and right-clicks a rock of its home
    /// system; the rock lives, the event line says so; on its globe the Build tab holds the
    /// brushes and a click paints.
    #[test]
    fn the_ascended_person_brings_a_rock_to_life_and_paints_it() {
        use crate::selection::{Controller, ViewCtx};
        use rom_render::{HudRegionKind, Modifiers, PickEvent, PointerButton, SurfacePos};
        let (mut world, computers, universe) = ready_with_universe();
        let mut view = AscensionView::new(computers, PLAYER);
        view.update(&world);
        crate::script::ascend(&mut crate::selection::OrderSink::Lead(&mut world), PLAYER).unwrap();
        while !world.is_ascended(PLAYER) {
            world.step();
            assert!(world.tick() < 20_000);
        }
        view.update(&world);
        let home = world.home_body(PLAYER).unwrap();
        let system = world.body(home).unwrap().system;
        let rock = world
            .dormant_bodies()
            .map(|(b, _)| b)
            .find(|b| world.body(*b).is_some_and(|i| i.system == system))
            .expect("a rock at home");
        let mut ctl = Controller::new();
        let press = |ctl: &mut Controller, world: &mut World, region, index| {
            ctl.handle(
                world,
                PickEvent::HudClick {
                    region,
                    index,
                    button: PointerButton::Left,
                    modifiers: Modifiers::default(),
                },
            );
        };
        ctl.set_view(ViewCtx {
            planet: home,
            system: Some(system),
            in_space: true,
            frame: None,
        });
        press(&mut ctl, &mut world, HudRegionKind::CommandSlot, 1);
        ctl.handle(
            &mut world,
            PickEvent::SpaceRightClick {
                surface: SurfacePos {
                    face: 0,
                    x: 0,
                    y: 0,
                },
                entity: None,
                body: Some(rock.0),
                system: None,
                modifiers: Modifiers::default(),
            },
        );
        world.step();
        assert!(world.living(rock).is_some());
        let lines = view.update(&world);
        assert!(
            lines.iter().any(|l| l.starts_with("You have brought")),
            "{lines:?}"
        );
        let barren = crate::terrain::living_scene(&world, &universe, rock).expect("a scene");
        assert_eq!(barren.sea_level, Some(world.sea_height()));
        // On its globe: the Build tab's first page, "Land, continent", one click.
        ctl.set_view(ViewCtx {
            planet: rock,
            system: Some(system),
            in_space: false,
            frame: None,
        });
        press(&mut ctl, &mut world, HudRegionKind::PaletteTab, 0);
        press(&mut ctl, &mut world, HudRegionKind::CommandSlot, 2);
        ctl.handle(
            &mut world,
            PickEvent::Click {
                world_point: (32.0, 32.0),
                surface: SurfacePos {
                    face: 0,
                    x: 32_000,
                    y: 32_000,
                },
                entity: None,
                modifiers: Modifiers::default(),
                double: false,
                on_screen: Vec::new(),
            },
        );
        world.step();
        assert_eq!(world.living(rock).unwrap().strokes, 1);
        let painted = crate::terrain::living_scene(&world, &universe, rock).expect("a scene");
        let changed = painted
            .kinds
            .iter()
            .zip(&barren.kinds)
            .filter(|(a, b)| a != b)
            .count();
        assert!(changed > 400, "{changed} tiles painted");
        // Task 150: with the brush still in hand a press, a drag and a release lay a dab every
        // brush size along the path: no gaps, and going back over it gives no repeats.
        let brush = ctl.brush_in_hand().expect("the brush stays in hand");
        let spacing = world.rules().brushes[&brush].radius.max(1000);
        let first = 4_000;
        let end = first + 2 * spacing + 500;
        let at = |x: i64| PickEvent::Stroke {
            world_point: (x as f32 / 1000.0, 32.0),
            surface: SurfacePos {
                face: 0,
                x,
                y: 32_000,
            },
            start: x == first,
        };
        let before = world.living(rock).unwrap().strokes;
        for event in [
            at(first),
            at(end),
            at(end),
            at(first + 1),
            PickEvent::StrokeEnd,
        ] {
            ctl.handle(&mut world, event);
        }
        world.step();
        let laid = world.living(rock).unwrap().strokes - before;
        assert_eq!(laid, 3, "the press and one dab per {spacing}");
        assert!(!ctl.stroking(), "the release ends the stroke");
    }

    #[test]
    fn everyone_is_told_of_the_project_and_the_ascension() {
        let (mut world, computers) = ready();
        let mut view = AscensionView::new(computers, PLAYER);
        view.update(&world);
        crate::script::ascend(&mut crate::selection::OrderSink::Lead(&mut world), PLAYER).unwrap();
        world.step();
        let lines = view.update(&world);
        assert!(
            lines[0].starts_with("You have begun the Ascension"),
            "{lines:?}"
        );
        let mut hud = HudSnapshot::default();
        view.fill(&world, &mut hud, false);
        assert_eq!(hud.banner.len(), 1);
        assert!(
            hud.diplomacy[1].0.contains("ascending"),
            "{:?}",
            hud.diplomacy
        );
        let celestial = celestial_system(&world).expect("a celestial home");
        assert_eq!(
            countdowns(&world, PLAYER)[0].system,
            world.body(world.home_body(PLAYER).unwrap()).unwrap().system
        );
        // Everyone may look at the celestial home; only the ascended know every system.
        assert!(crate::space_view::known_systems(PLAYER, &world).contains(&celestial));
        crate::script::ascend(
            &mut crate::selection::OrderSink::Lead(&mut world),
            PlayerId(2),
        )
        .unwrap();
        let mut told = Vec::new();
        while !(world.is_ascended(PLAYER) && world.is_ascended(PlayerId(2))) {
            world.step();
            told.extend(view.update(&world));
            assert!(world.tick() < 20_000);
        }
        assert!(
            told.iter().any(|l| l.starts_with("You have ascended")),
            "{told:?}"
        );
        assert_eq!(residences(&world, PLAYER).len(), 2);
        assert_eq!(
            crate::space_view::known_systems(PLAYER, &world).len(),
            world.systems().len()
        );
        // The computer player greets the channel and answers the person.
        assert!(view.chat().iter().any(|l| l.player == PlayerId(2)));
        let before = view.chat().len();
        assert!(view.say(&world, Channel::Celestial, "Hello"));
        assert_eq!(view.chat().len(), before + 2);
        let mut hud = HudSnapshot::default();
        view.fill(&world, &mut hud, true);
        let panel = hud.panel.expect("the celestial panel");
        assert!(panel.lines.iter().any(|(l, _)| l.contains("Hello")));
        assert_eq!(hud.chat.channel, "[Celestial]");
        // Then the descent where its city stood.
        let home = world.home_body(PLAYER).unwrap();
        let site = world
            .ascension_projects()
            .first()
            .map(|p| p.library)
            .and_then(|id| world.entity(id))
            .map(|e| e.pos);
        assert!(site.is_none(), "no project is left");
        let at = (0..6)
            .find_map(|face| {
                let n = world.body(home)?.n? as i32;
                world.find_city_site(PLAYER, home, Pos::on(home, face, n * 500, n * 500))
            })
            .expect("open land");
        world
            .submit(Command {
                player: PLAYER,
                executor: EntityId(0),
                kind: "descend".into(),
                subject: None,
                target_entity: None,
                target_pos: Some(at),
                queue: false,
                text: None,
                facing: None,
            })
            .unwrap();
        world.step();
        let lines = view.update(&world);
        assert!(
            lines.iter().any(|l| l.starts_with("You have descended")),
            "{lines:?}"
        );
        assert_eq!(world.cities(PLAYER).len(), 1);
    }
}
