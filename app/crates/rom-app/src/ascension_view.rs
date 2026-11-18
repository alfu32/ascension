//! What the person sees of the ascension (task 76): the event lines when a player starts the
//! project, ascends or descends, the countdowns at the top of the screen, on the star map and in
//! the diplomacy tab, the notice to the last material power, and the celestial home's panel with
//! its residents and the chat channel of the ascended players (computer players included).
//!
//! The chat is the application's, not the simulation's: like a chat between people, it changes
//! nothing in the match and is not saved.

use std::collections::{BTreeMap, BTreeSet};

use rom_ai::ChatTopic;
use rom_render::{CountdownView, HudPanel, HudSnapshot, ResidenceView, owner_color};
use rom_rules::BodyKind;
use rom_sim::{PlayerId, PlayerStatus, World};

use crate::selection::PLAYER;

/// Most chat lines kept.
const CHAT_KEPT: usize = 40;

/// A line of the celestial home's channel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatLine {
    /// When it was said.
    pub tick: u64,
    /// Who said it.
    pub player: PlayerId,
    /// The words.
    pub text: String,
}

/// The name a player goes by in the event line, the diplomacy tab and the chat.
pub fn player_name(world: &World, id: PlayerId) -> String {
    if id == PLAYER {
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
fn project_name(world: &World, player: PlayerId) -> String {
    if player == PLAYER {
        "Your ascension".to_owned()
    } else {
        format!("Ascension of {}", player_name(world, player))
    }
}

/// The ascensions under way, for the star map: beside the system of each library.
pub fn countdowns(world: &World) -> Vec<CountdownView> {
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
                    project_name(world, p.player),
                    clock(world, p.remaining())
                ),
            })
        })
        .collect()
}

/// The ascended players living on the celestial home, by player.
pub fn residences(world: &World) -> Vec<ResidenceView> {
    world
        .players()
        .filter(|p| world.is_ascended(p.id))
        .map(|p| ResidenceView {
            owner: u8::try_from(p.id.0).unwrap_or(u8::MAX),
            name: player_name(world, p.id),
        })
        .collect()
}

/// What changed of the ascension since the last look, and the chat of the celestial home.
#[derive(Debug, Default)]
pub struct AscensionView {
    started: bool,
    /// Projects under way: player and the tick each was ordered.
    projects: BTreeSet<(PlayerId, u64)>,
    ascended: BTreeSet<PlayerId>,
    standing: Option<u32>,
    /// The players a computer plays: they speak in the channel.
    computers: BTreeSet<PlayerId>,
    /// The channel, oldest first.
    chat: Vec<ChatLine>,
    /// The line the person is typing, if any.
    pub draft: Option<String>,
}

impl AscensionView {
    /// A view of a match whose `computers` speak for themselves in the channel.
    pub fn new(computers: BTreeSet<PlayerId>) -> Self {
        Self {
            computers,
            ..Self::default()
        }
    }

    /// The channel, oldest first.
    #[cfg(test)]
    pub fn chat(&self) -> &[ChatLine] {
        &self.chat
    }

    fn post(&mut self, world: &World, player: PlayerId, text: impl Into<String>) {
        let text = text.into();
        if text.is_empty() {
            return;
        }
        self.chat.push(ChatLine {
            tick: world.tick(),
            player,
            text,
        });
        let over = self.chat.len().saturating_sub(CHAT_KEPT);
        self.chat.drain(..over);
    }

    /// The person's line in the channel; the first ascended computer player answers it.
    pub fn say(&mut self, world: &World, text: &str) {
        let text = text.trim();
        if text.is_empty() || !world.is_ascended(PLAYER) {
            return;
        }
        self.post(world, PLAYER, text);
        if let Some(&other) = self.computers.iter().find(|id| world.is_ascended(**id)) {
            self.post(
                world,
                other,
                rom_ai::chat_line(other, ChatTopic::Reply, world.tick()),
            );
        }
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
                lines.push(if player == PLAYER {
                    format!(
                        "You have begun the Ascension: {} left. Every player is told; hold your library",
                        clock(world, left)
                    )
                } else {
                    format!(
                        "{} has begun the Ascension: {} left. Defeat them first to stop it",
                        player_name(world, player),
                        clock(world, left)
                    )
                });
            }
            for &(player, _) in &self.projects {
                if projects.keys().any(|(p, _)| *p == player) || ascended.contains(&player) {
                    continue;
                }
                let name = player_name(world, player);
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
                lines.push(if player == PLAYER {
                    "You have ascended: your possessions left the material plane. Explore freely; \
                     Descend from the command panel when you wish"
                        .to_owned()
                } else {
                    format!(
                        "{} has ascended and left the material plane",
                        player_name(world, player)
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
                lines.push(if player == PLAYER {
                    format!("You have descended on {planet}: a city, citizens and the first age")
                } else {
                    format!("{} has descended on {planet}", player_name(world, player))
                });
                if self.computers.contains(&player) {
                    let line = rom_ai::chat_line(player, ChatTopic::Farewell, world.tick());
                    self.post(world, player, line);
                }
            }
            if standing.is_some() && standing != self.standing {
                let team = world.players().find(|p| p.id == PLAYER).map(|p| p.team);
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
                            .map(|p| player_name(world, p.id))
                            .collect();
                        format!("{} remain the last material power", names.join(", "))
                    },
                );
            }
        }
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
                        project_name(world, p.player).to_uppercase(),
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
                    (player_name(world, p.id), colour_of(p.id)),
                    (format!("  {state}"), [232, 201, 138, 255]),
                ]
            })
            .collect();
        let ascended = world.is_ascended(PLAYER);
        if ascended {
            hud.chat = ("[Celestial]".to_owned(), self.draft.clone());
        }
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
                lines.push((format!("  {}", player_name(world, *id)), colour_of(*id)));
            }
        }
        if ascended {
            lines.push((
                "Chat: Enter to write, Enter to send".to_owned(),
                [151, 165, 177, 255],
            ));
            for line in self.chat.iter().rev().take(8).rev() {
                lines.push((
                    format!("{}: {}", player_name(world, line.player), line.text),
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
    use rom_sim::{Command, EntityId, Pos};

    /// A universe match with player one ready to ascend and a library beside its city.
    fn ready() -> (World, BTreeSet<PlayerId>) {
        let options = crate::args::Options {
            universe: true,
            players: 2,
            seed: 1,
            ..crate::args::Options::default()
        };
        let content = crate::terrain::generate_universe_content(&options).expect("universe");
        let setup = ScriptSetup {
            age: Some("age.galactic".into()),
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
        (game.world, [PlayerId(2)].into())
    }

    #[test]
    fn everyone_is_told_of_the_project_and_the_ascension() {
        let (mut world, computers) = ready();
        let mut view = AscensionView::new(computers);
        view.update(&world);
        crate::script::ascend(&mut world, PLAYER).unwrap();
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
            countdowns(&world)[0].system,
            world.body(world.home_body(PLAYER).unwrap()).unwrap().system
        );
        // Everyone may look at the celestial home; only the ascended know every system.
        assert!(crate::space_view::known_systems(&world).contains(&celestial));
        crate::script::ascend(&mut world, PlayerId(2)).unwrap();
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
        assert_eq!(residences(&world).len(), 2);
        assert_eq!(
            crate::space_view::known_systems(&world).len(),
            world.systems().len()
        );
        // The computer player greets the channel and answers the person.
        assert!(view.chat().iter().any(|l| l.player == PlayerId(2)));
        let before = view.chat().len();
        view.say(&world, "Hello");
        assert_eq!(view.chat().len(), before + 2);
        let mut hud = HudSnapshot::default();
        view.fill(&world, &mut hud, true);
        let panel = hud.panel.expect("the celestial panel");
        assert!(panel.lines.iter().any(|(l, _)| l.contains("Hello")));
        assert_eq!(hud.chat.0, "[Celestial]");
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
