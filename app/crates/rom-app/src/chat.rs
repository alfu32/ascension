//! In-game chat (task 94): the channels, the log, the keyboard input and the seam network play
//! will use.
//!
//! Chat is presentation. A line is never an order, is outside the simulation and its hash, and
//! changes nothing in the match. Lines leave through one [`ChatSink`] together with the players
//! they are meant for; the single-player sink is the [`ChatLog`] itself, and a network sink
//! (task 88) can relay the same call without touching the interface.

use rom_render::{CHAT_HISTORY_ROWS, HudChat, HudChatHistory, HudChatLine};
use rom_sim::{PlayerId, World};

use crate::ascension_view::{colour_of, player_name};
use crate::selection::PLAYER;

/// Most characters in a line.
pub const LINE_LENGTH: usize = 80;
/// Most lines the log keeps.
const KEPT: usize = 400;
/// Ticks a new line stays fully visible over the world, and the ticks it then takes to fade.
const SHOWN_TICKS: u64 = 200;
const FADE_TICKS: u64 = 60;
/// Lines shown over the world while typing, and otherwise.
const SHOWN_TYPING: usize = 6;
const SHOWN_IDLE: usize = 5;
/// Key of the settings entry a save keeps the log under.
pub const SAVE_KEY: &str = "chat.log";

/// Who a line is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Channel {
    /// Every player.
    All,
    /// The sender's team.
    Allies,
    /// The ascended players (task 76).
    Celestial,
}

impl Channel {
    /// The word scripts and saves use.
    pub fn key(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Allies => "allies",
            Self::Celestial => "celestial",
        }
    }

    /// The channel a word names.
    pub fn from_key(word: &str) -> Option<Self> {
        [Self::All, Self::Allies, Self::Celestial]
            .into_iter()
            .find(|c| c.key() == word.to_ascii_lowercase())
    }

    /// The coloured tag shown before a line.
    pub fn tag(self) -> &'static str {
        match self {
            Self::All => "[All]",
            Self::Allies => "[Allies]",
            Self::Celestial => "[Celestial]",
        }
    }

    /// The tag's colour.
    pub fn colour(self) -> [u8; 4] {
        match self {
            Self::All => [63, 208, 255, 255],
            Self::Allies => [120, 224, 120, 255],
            Self::Celestial => [232, 201, 138, 255],
        }
    }

    /// The next channel in the cycle; the celestial one only while `celestial` is open.
    pub fn next(self, celestial: bool) -> Self {
        match (self, celestial) {
            (Self::All, _) => Self::Allies,
            (Self::Allies, true) => Self::Celestial,
            _ => Self::All,
        }
    }
}

/// A line of chat.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatLine {
    /// When it was said.
    pub tick: u64,
    /// Who said it.
    pub player: PlayerId,
    /// Where it was said.
    pub channel: Channel,
    /// The words.
    pub text: String,
}

/// Where lines go. `recipients` are the players the line is for, the sender included; a sink
/// that serves one person keeps the line only if that person is among them.
pub trait ChatSink {
    /// Takes a line.
    fn deliver(&mut self, line: &ChatLine, recipients: &[PlayerId]);
}

/// The players a line to `channel` from `sender` is for, in id order (never nature, id 0).
pub fn recipients(world: &World, sender: PlayerId, channel: Channel) -> Vec<PlayerId> {
    let team = world.players().find(|p| p.id == sender).map(|p| p.team);
    world
        .players()
        .filter(|p| p.id != PlayerId(0))
        .filter(|p| match channel {
            Channel::All => true,
            Channel::Allies => Some(p.team) == team,
            Channel::Celestial => world.is_ascended(p.id),
        })
        .map(|p| p.id)
        .collect()
}

/// Cleans text for a line: control characters out, ends trimmed, cut to [`LINE_LENGTH`].
fn clean(text: &str) -> String {
    let text: String = text
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    text.trim().chars().take(LINE_LENGTH).collect()
}

/// Hands a line to the sink with its recipients, whatever the sender's standing: for lines the
/// application itself speaks (computer players arriving and leaving).
pub fn deliver(
    world: &World,
    sink: &mut dyn ChatSink,
    player: PlayerId,
    channel: Channel,
    text: &str,
) {
    let text = clean(text);
    if text.is_empty() {
        return;
    }
    let line = ChatLine {
        tick: world.tick(),
        player,
        channel,
        text,
    };
    sink.deliver(&line, &recipients(world, player, channel));
}

/// A person's line: refused when empty, or for the celestial channel while not ascended.
/// Returns whether it was sent.
pub fn send(
    world: &World,
    sink: &mut dyn ChatSink,
    player: PlayerId,
    channel: Channel,
    text: &str,
) -> bool {
    if channel == Channel::Celestial && !world.is_ascended(player) {
        return false;
    }
    if clean(text).is_empty() {
        return false;
    }
    deliver(world, sink, player, channel, text);
    true
}

/// The lines this person has seen, oldest first. It is the single-player [`ChatSink`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatLog {
    lines: Vec<ChatLine>,
    /// The person whose log it is.
    me: PlayerId,
}

impl Default for ChatLog {
    /// The standalone player's log.
    fn default() -> Self {
        Self::for_player(PLAYER)
    }
}

impl ChatSink for ChatLog {
    fn deliver(&mut self, line: &ChatLine, recipients: &[PlayerId]) {
        if recipients.contains(&self.me) {
            self.lines.push(line.clone());
            let over = self.lines.len().saturating_sub(KEPT);
            self.lines.drain(..over);
        }
    }
}

impl ChatLog {
    /// An empty log of the person playing `me`.
    pub fn for_player(me: PlayerId) -> Self {
        Self {
            lines: Vec::new(),
            me,
        }
    }

    /// The same lines, as the log of the person playing `me`.
    pub fn with_player(mut self, me: PlayerId) -> Self {
        self.me = me;
        self
    }

    /// The lines, oldest first.
    pub fn lines(&self) -> &[ChatLine] {
        &self.lines
    }

    /// Whether the log is empty.
    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// The log as text for a save: one line each, `tick TAB player TAB channel TAB words`.
    pub fn encode(&self) -> String {
        self.lines
            .iter()
            .map(|l| {
                format!(
                    "{}\t{}\t{}\t{}",
                    l.tick,
                    l.player.0,
                    l.channel.key(),
                    l.text
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The log a save kept; lines that do not read are skipped, so a save without a log (or
    /// with a damaged one) gives an empty or shorter log and never an error.
    pub fn decode(text: &str) -> Self {
        let lines = text
            .lines()
            .filter_map(|row| {
                let mut parts = row.splitn(4, '\t');
                let tick = parts.next()?.parse().ok()?;
                let player = PlayerId(parts.next()?.parse().ok()?);
                let channel = Channel::from_key(parts.next()?)?;
                let text = clean(parts.next()?);
                (!text.is_empty()).then_some(ChatLine {
                    tick,
                    player,
                    channel,
                    text,
                })
            })
            .collect();
        let mut log = Self { lines, me: PLAYER };
        let over = log.lines.len().saturating_sub(KEPT);
        log.lines.drain(..over);
        log
    }

    fn row(&self, world: &World, line: &ChatLine, alpha: u8) -> HudChatLine {
        HudChatLine {
            name: player_name(world, self.me, line.player),
            name_colour: colour_of(line.player),
            tag: line.channel.tag().to_owned(),
            tag_colour: line.channel.colour(),
            text: line.text.clone(),
            alpha,
        }
    }

    /// What the HUD draws: the message line, the latest lines (fading) and the open history.
    pub fn view(&self, world: &World, input: &ChatInput) -> HudChat {
        let celestial = world.is_ascended(self.me);
        let channel = input.channel(celestial);
        let now = world.tick();
        let typing = input.focused();
        let alpha = |line: &ChatLine| -> u8 {
            let age = now.saturating_sub(line.tick);
            if typing || age <= SHOWN_TICKS {
                255
            } else if age >= SHOWN_TICKS + FADE_TICKS {
                0
            } else {
                (255 - (age - SHOWN_TICKS) * 255 / FADE_TICKS) as u8
            }
        };
        let take = if typing { SHOWN_TYPING } else { SHOWN_IDLE };
        let mut recent: Vec<HudChatLine> = self
            .lines
            .iter()
            .rev()
            .take(take)
            .map(|l| self.row(world, l, alpha(l)))
            .filter(|l| l.alpha > 0)
            .collect();
        recent.reverse();
        let history = input.history_open().then(|| {
            let scroll = input.scroll().min(self.lines.len().saturating_sub(1));
            let end = self.lines.len() - scroll.min(self.lines.len());
            let start = end.saturating_sub(CHAT_HISTORY_ROWS);
            HudChatHistory {
                lines: self.lines[start..end]
                    .iter()
                    .map(|l| self.row(world, l, 255))
                    .collect(),
                above: start,
                below: self.lines.len() - end,
            }
        });
        HudChat {
            channel: channel.tag().to_owned(),
            channel_colour: channel.colour(),
            draft: input.draft(),
            caret: input.caret(),
            recent,
            history,
        }
    }
}

/// A key as the chat takes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChatKey {
    /// A typed character.
    Char(char),
    /// Pasted text.
    Paste(String),
    /// Enter: opens the line, or sends it.
    Enter,
    /// Escape: drops the line, or closes the history.
    Escape,
    /// Tab, or Shift+Enter: the next channel.
    Tab,
    /// Takes back the character before the caret.
    Backspace,
    /// Removes the character after the caret.
    Delete,
    /// Caret left.
    Left,
    /// Caret right.
    Right,
    /// Caret to the start.
    Home,
    /// Caret to the end.
    End,
    /// Scroll the history one line towards older lines.
    Up,
    /// Scroll the history one line towards newer lines.
    Down,
    /// Scroll the history a page towards older lines.
    PageUp,
    /// Scroll the history a page towards newer lines.
    PageDown,
    /// Open or close the history.
    History,
}

impl ChatKey {
    /// The key a script or keystroke name stands for; one character is a typed character.
    pub fn from_name(name: &str) -> Option<Self> {
        let mut chars = name.chars();
        if let (Some(c), None) = (chars.next(), chars.next()) {
            return Some(Self::Char(c));
        }
        Some(match name.to_ascii_lowercase().as_str() {
            "enter" => Self::Enter,
            "escape" | "esc" => Self::Escape,
            "tab" | "shift+enter" => Self::Tab,
            "backspace" => Self::Backspace,
            "delete" => Self::Delete,
            "left" => Self::Left,
            "right" => Self::Right,
            "home" => Self::Home,
            "end" => Self::End,
            "up" => Self::Up,
            "down" => Self::Down,
            "pageup" => Self::PageUp,
            "pagedown" => Self::PageDown,
            "space" => Self::Char(' '),
            "f7" | "history" => Self::History,
            _ => return None,
        })
    }
}

/// Lines a page key scrolls the history.
const PAGE: usize = 10;

/// The keyboard side of the chat: whether it has the keys, the line being typed, the caret, the
/// channel the person chose and the history's scroll. Shared between the window and the
/// simulation thread.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChatInput {
    focus: bool,
    draft: String,
    caret: usize,
    /// `None` until the person picks one: then All, or the celestial channel once ascended.
    chosen: Option<Channel>,
    history: bool,
    scroll: usize,
    sent: Vec<(Channel, String)>,
}

impl ChatInput {
    /// Whether the chat has the keyboard (game hotkeys and camera keys wait).
    pub fn focused(&self) -> bool {
        self.focus
    }

    /// The line being typed, while the chat has the keyboard.
    pub fn draft(&self) -> Option<String> {
        self.focus.then(|| self.draft.clone())
    }

    /// The caret's place in the line, in characters.
    pub fn caret(&self) -> usize {
        self.caret
    }

    /// The channel lines go to; the celestial one only while `celestial` is open.
    pub fn channel(&self, celestial: bool) -> Channel {
        match self.chosen {
            Some(Channel::Celestial) if !celestial => Channel::All,
            Some(channel) => channel,
            None if celestial => Channel::Celestial,
            None => Channel::All,
        }
    }

    /// Whether the history is open.
    pub fn history_open(&self) -> bool {
        self.history
    }

    /// How many lines the history is scrolled back.
    pub fn scroll(&self) -> usize {
        self.scroll
    }

    /// Keeps the scroll within a log of `len` lines.
    pub fn clamp_scroll(&mut self, len: usize) {
        self.scroll = self.scroll.min(len.saturating_sub(1));
    }

    /// Gives the chat the keyboard (a click on the chat line).
    pub fn open(&mut self) {
        self.focus = true;
    }

    /// Opens or closes the history (the chat button, or its key).
    pub fn toggle_history(&mut self) {
        self.history = !self.history;
        self.scroll = 0;
    }

    /// The lines sent since the last call, each with its channel.
    pub fn take_sent(&mut self) -> Vec<(Channel, String)> {
        std::mem::take(&mut self.sent)
    }

    fn insert(&mut self, text: &str) {
        let room = LINE_LENGTH.saturating_sub(self.draft.chars().count());
        let text: String = text
            .chars()
            .map(|c| if c.is_control() { ' ' } else { c })
            .take(room)
            .collect();
        let at = self
            .draft
            .char_indices()
            .nth(self.caret)
            .map_or(self.draft.len(), |(i, _)| i);
        self.draft.insert_str(at, &text);
        self.caret += text.chars().count();
    }

    fn byte(&self, caret: usize) -> usize {
        self.draft
            .char_indices()
            .nth(caret)
            .map_or(self.draft.len(), |(i, _)| i)
    }

    /// Applies a key. Returns whether the chat took it; while it has the keyboard it takes every
    /// key, so game hotkeys and camera keys never see one. `celestial` says whether the person
    /// may use the celestial channel.
    pub fn key(&mut self, key: ChatKey, celestial: bool) -> bool {
        if !self.focus {
            return self.idle_key(&key);
        }
        match key {
            ChatKey::Char(c) => self.insert(&c.to_string()),
            ChatKey::Paste(text) => self.insert(&text),
            ChatKey::Enter => {
                let text = std::mem::take(&mut self.draft);
                if !clean(&text).is_empty() {
                    let channel = self.channel(celestial);
                    self.sent.push((channel, text));
                }
                self.caret = 0;
                self.focus = false;
            }
            ChatKey::Escape => {
                self.draft.clear();
                self.caret = 0;
                self.focus = false;
            }
            ChatKey::Tab => {
                self.chosen = Some(self.channel(celestial).next(celestial));
            }
            ChatKey::Backspace => {
                if self.caret > 0 {
                    let from = self.byte(self.caret - 1);
                    let to = self.byte(self.caret);
                    self.draft.replace_range(from..to, "");
                    self.caret -= 1;
                }
            }
            ChatKey::Delete => {
                if self.caret < self.draft.chars().count() {
                    let from = self.byte(self.caret);
                    let to = self.byte(self.caret + 1);
                    self.draft.replace_range(from..to, "");
                }
            }
            ChatKey::Left => self.caret = self.caret.saturating_sub(1),
            ChatKey::Right => self.caret = (self.caret + 1).min(self.draft.chars().count()),
            ChatKey::Home => self.caret = 0,
            ChatKey::End => self.caret = self.draft.chars().count(),
            ChatKey::History => self.toggle_history(),
            ChatKey::Up => self.scroll = self.scroll.saturating_add(1),
            ChatKey::Down => self.scroll = self.scroll.saturating_sub(1),
            ChatKey::PageUp => self.scroll = self.scroll.saturating_add(PAGE),
            ChatKey::PageDown => self.scroll = self.scroll.saturating_sub(PAGE),
        }
        true
    }

    /// Keys while the chat has no focus: Enter opens the line; with the history open it also
    /// scrolls and Escape closes it. Anything else belongs to the game.
    fn idle_key(&mut self, key: &ChatKey) -> bool {
        match key {
            ChatKey::Enter => self.focus = true,
            ChatKey::History => self.toggle_history(),
            ChatKey::Escape if self.history => self.history = false,
            ChatKey::Up if self.history => self.scroll = self.scroll.saturating_add(1),
            ChatKey::Down if self.history => self.scroll = self.scroll.saturating_sub(1),
            ChatKey::PageUp if self.history => self.scroll = self.scroll.saturating_add(PAGE),
            ChatKey::PageDown if self.history => self.scroll = self.scroll.saturating_sub(PAGE),
            _ => return false,
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rom_sim::{MapGrid, Player};
    use std::sync::Arc;

    fn typed(input: &mut ChatInput, text: &str) {
        for c in text.chars() {
            assert!(input.key(ChatKey::Char(c), false));
        }
    }

    #[test]
    fn focus_takes_the_keys_from_the_game_and_gives_them_back() {
        let mut input = ChatInput::default();
        // Without focus a letter, the arrows and Escape belong to the game.
        for key in [
            ChatKey::Char('a'),
            ChatKey::Left,
            ChatKey::Escape,
            ChatKey::Tab,
        ] {
            assert!(!input.key(key, false));
        }
        assert!(input.key(ChatKey::Enter, false));
        assert!(input.focused());
        // With it every key is the chat's, including ones that move the camera.
        for key in [
            ChatKey::Char('w'),
            ChatKey::Left,
            ChatKey::Up,
            ChatKey::Home,
            ChatKey::Char(' '),
        ] {
            assert!(input.key(key, false));
        }
        assert!(input.key(ChatKey::Escape, false));
        assert!(!input.focused());
        assert!(!input.key(ChatKey::Char('w'), false));
    }

    #[test]
    fn enter_sends_to_the_chosen_channel_and_escape_keeps_nothing() {
        let mut input = ChatInput::default();
        input.key(ChatKey::Enter, false);
        typed(&mut input, "hello");
        assert!(input.key(ChatKey::Tab, false));
        input.key(ChatKey::Enter, false);
        assert_eq!(input.take_sent(), [(Channel::Allies, "hello".to_owned())]);
        assert!(!input.focused());
        // The choice is remembered for the next line.
        input.key(ChatKey::Enter, false);
        typed(&mut input, "again");
        input.key(ChatKey::Enter, false);
        assert_eq!(input.take_sent(), [(Channel::Allies, "again".to_owned())]);
        // Escape drops the line, an empty one is not sent.
        input.key(ChatKey::Enter, false);
        typed(&mut input, "never");
        input.key(ChatKey::Escape, false);
        input.key(ChatKey::Enter, false);
        assert_eq!(input.draft().as_deref(), Some(""));
        input.key(ChatKey::Enter, false);
        assert!(input.take_sent().is_empty());
    }

    #[test]
    fn the_celestial_channel_opens_with_the_ascension() {
        let mut input = ChatInput::default();
        assert_eq!(input.channel(false), Channel::All);
        assert_eq!(input.channel(true), Channel::Celestial);
        input.key(ChatKey::Enter, true);
        input.key(ChatKey::Tab, true);
        assert_eq!(input.channel(true), Channel::All);
        input.key(ChatKey::Tab, true);
        input.key(ChatKey::Tab, true);
        assert_eq!(input.channel(true), Channel::Celestial);
        // Back among the material: the channel is closed.
        assert_eq!(input.channel(false), Channel::All);
    }

    #[test]
    fn the_caret_edits_in_place() {
        let mut input = ChatInput::default();
        input.key(ChatKey::Enter, false);
        typed(&mut input, "héllo");
        input.key(ChatKey::Left, false);
        input.key(ChatKey::Left, false);
        input.key(ChatKey::Backspace, false);
        assert_eq!(input.draft().as_deref(), Some("hélo"));
        assert_eq!(input.caret(), 2);
        input.key(ChatKey::Delete, false);
        assert_eq!(input.draft().as_deref(), Some("héo"));
        input.key(ChatKey::Paste("X\nY".into()), false);
        assert_eq!(input.draft().as_deref(), Some("héX Yo"));
        input.key(ChatKey::Home, false);
        input.key(ChatKey::Char('>'), false);
        assert_eq!(input.draft().as_deref(), Some(">héX Yo"));
        input.key(ChatKey::End, false);
        assert_eq!(input.caret(), 7);
        input.key(ChatKey::Paste("z".repeat(200)), false);
        assert_eq!(input.draft().unwrap().chars().count(), LINE_LENGTH);
    }

    #[test]
    fn the_history_scrolls_without_the_keyboard() {
        let mut input = ChatInput::default();
        assert!(!input.key(ChatKey::Up, false));
        assert!(input.key(ChatKey::History, false));
        assert!(input.key(ChatKey::Up, false));
        assert_eq!(input.scroll(), 1);
        assert!(input.key(ChatKey::PageUp, false));
        assert_eq!(input.scroll(), 11);
        input.clamp_scroll(5);
        assert_eq!(input.scroll(), 4);
        assert!(input.key(ChatKey::Escape, false));
        assert!(!input.history_open());
    }

    /// Records what a sink was given.
    #[derive(Default)]
    struct Recorder(Vec<(ChatLine, Vec<PlayerId>)>);

    impl ChatSink for Recorder {
        fn deliver(&mut self, line: &ChatLine, recipients: &[PlayerId]) {
            self.0.push((line.clone(), recipients.to_vec()));
        }
    }

    /// Nature, the person (team 1), an opponent (team 2) and the person's ally (team 1).
    fn match_of_three() -> World {
        let rules = Arc::new(crate::terrain::load_rules().expect("rules"));
        let id = |key: &str| rules.id_of(key).expect("definition");
        let player = |n: u32, team: u32| Player {
            id: PlayerId(n),
            team,
            age: id("age.ancient"),
            faction: id("nation.romans"),
            resources: Default::default(),
            techs: Default::default(),
        };
        let map = MapGrid::new(16, 16, vec![id("ter.plain"); 16 * 16]).expect("map");
        let players = vec![player(0, 0), player(1, 1), player(2, 2), player(3, 1)];
        World::new(Arc::clone(&rules), map, 1, players).expect("world")
    }

    #[test]
    fn allies_reach_only_the_team() {
        let world = match_of_three();
        let mut sink = Recorder::default();
        assert!(send(&world, &mut sink, PLAYER, Channel::Allies, "  hold  "));
        assert!(send(&world, &mut sink, PLAYER, Channel::All, "hi all"));
        let mates = vec![PlayerId(1), PlayerId(3)];
        assert_eq!(sink.0[0].0.text, "hold");
        assert_eq!(sink.0[0].1, mates);
        assert!(mates.contains(&PLAYER));
        let everyone: Vec<PlayerId> = world
            .players()
            .filter(|p| p.id != PlayerId(0))
            .map(|p| p.id)
            .collect();
        assert_eq!(sink.0[1].1, everyone);
        assert!(everyone.len() > mates.len(), "the match has opponents");
        // The celestial channel is closed to the material.
        assert!(!send(&world, &mut sink, PLAYER, Channel::Celestial, "x"));
        assert!(!send(&world, &mut sink, PLAYER, Channel::All, "   "));
        assert_eq!(sink.0.len(), 2);
    }

    #[test]
    fn the_log_keeps_order_and_only_what_is_for_the_person() {
        let world = match_of_three();
        let mut log = ChatLog::default();
        let other = PlayerId(2);
        for (i, text) in ["one", "two", "three", "four"].into_iter().enumerate() {
            let sender = if i % 2 == 0 { PLAYER } else { other };
            deliver(&world, &mut log, sender, Channel::All, text);
        }
        // An allies line from an opponent does not reach the person.
        deliver(&world, &mut log, other, Channel::Allies, "secret");
        let texts: Vec<&str> = log.lines().iter().map(|l| l.text.as_str()).collect();
        assert_eq!(texts, ["one", "two", "three", "four"]);
        let again = ChatLog::decode(&log.encode());
        assert_eq!(again, log);
        // A save made before there was a log, or a damaged one, still reads.
        assert!(ChatLog::decode("").is_empty());
        assert_eq!(
            ChatLog::decode("junk\n1\t1\tall\tok\n2\tx").lines().len(),
            1
        );
    }

    #[test]
    fn lines_fade_after_a_while() {
        let world = match_of_three();
        let mut log = ChatLog::default();
        deliver(&world, &mut log, PLAYER, Channel::All, "hello");
        let input = ChatInput::default();
        let view = log.view(&world, &input);
        assert_eq!(view.recent.len(), 1);
        assert_eq!(view.recent[0].alpha, 255);
        assert_eq!(view.channel, "[All]");
        assert!(view.history.is_none() && view.draft.is_none());
        let mut faded = log.clone();
        faded.lines[0].tick = 0;
        let mut later = match_of_three();
        while later.tick() < SHOWN_TICKS + FADE_TICKS + 1 {
            later.step();
        }
        assert!(faded.view(&later, &input).recent.is_empty());
        let mut typing = ChatInput::default();
        typing.key(ChatKey::Enter, false);
        assert_eq!(faded.view(&later, &typing).recent.len(), 1);
    }
}
