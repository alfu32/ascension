//! The messages both ends speak (`app/docs/network.md`, "Protocol"): JSON in WebSocket text
//! frames, one object per frame tagged by `type`; a snapshot's save file follows its
//! [`ServerMessage::Snapshot`] as one binary frame.

use rom_mapgen::StartPlacement;
use rom_match::{ContentRequest, Entry, Journal, MapKind, MatchSettings, ScriptSetup, Spec};
use rom_sim::{BodyId, CityArea, Command, EconomyPace, EntityId, PlayerId, Pos};
use serde::{Deserialize, Serialize};

/// The protocol's version: both ends must speak the same one.
pub const PROTOCOL: u32 = 1;

/// The application version both ends must run.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Ticks between two hash checks.
pub const HASH_EVERY: u64 = 100;

/// The rules fingerprint as the handshake sends it.
pub fn rules() -> String {
    rom_match::rules_fingerprint_hex()
}

/// A message from a client to the server.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    /// Join with a one-time key, or rejoin a seat with its token.
    Hello {
        /// [`PROTOCOL`].
        protocol: u32,
        /// [`VERSION`].
        version: String,
        /// [`rules`].
        rules: String,
        /// The name the person plays under.
        name: String,
        /// A one-time key, to join.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        key: Option<String>,
        /// A seat token, to rejoin.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        seat_token: Option<String>,
    },
    /// Lobby: the faction (race or nation) this seat plays, by key.
    Choose {
        /// A playable faction's key.
        faction: String,
    },
    /// Lobby: ready, or not.
    Ready {
        /// Whether ready.
        ready: bool,
    },
    /// Lobby, host only: start now; empty human seats are played by computers.
    Start,
    /// An order; the server sets `command.player` to the seat's player.
    Order {
        /// The client's own number for it, echoed in [`ServerMessage::Rejected`].
        seq: u64,
        /// The order.
        command: WireCommand,
    },
    /// The camera looks at a planet: its surface is made if it has none.
    Visit {
        /// As for [`ClientMessage::Order`].
        #[serde(default)]
        seq: u64,
        /// The planet.
        body: u32,
    },
    /// The client's world hash after stepping to `tick` (every [`HASH_EVERY`] ticks).
    Hash {
        /// The tick the world stands at.
        tick: u64,
        /// `World::hash`.
        hash: u64,
    },
    /// The client stands at `tick` and has sent its orders for it; only when the server
    /// asked for it ([`ServerMessage::Start`]'s `wait_for_clients`).
    Done {
        /// The tick the world stands at.
        tick: u64,
    },
    /// The client is too far behind (or lost) and wants a snapshot.
    NeedSnapshot,
    /// After the match's end, a remaining human keeps playing (task 117).
    KeepPlaying,
    /// Pause the match (anyone).
    Pause,
    /// Resume the match (anyone).
    Resume,
    /// Set the speed, in percent of 20 ticks a second (host only).
    Speed {
        /// 1 to 10,000.
        pct: u32,
    },
    /// Save the match now (host only).
    Save {
        /// The save's name; the autosave's when absent.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    /// A chat line.
    Chat {
        /// To everyone or to allies only.
        channel: Channel,
        /// The text, at most [`CHAT_LIMIT`] characters.
        text: String,
    },
    /// Latency probe; answered with [`ServerMessage::Pong`].
    Ping {
        /// Echoed.
        nonce: u64,
    },
    /// Answer to [`ServerMessage::Ping`].
    Pong {
        /// Echoed.
        nonce: u64,
    },
}

/// Longest chat line, in characters.
pub const CHAT_LIMIT: usize = 500;

/// A message from the server to a client.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    /// Admitted: the seat, the token to rejoin it, and the lobby.
    Welcome {
        /// The seat, from 0.
        seat: u32,
        /// Keep it to rejoin the seat after a drop or a server restart.
        seat_token: String,
        /// Whether this seat may start the match and set the speed.
        host: bool,
        /// The lobby as it stands.
        lobby: Lobby,
    },
    /// Not admitted.
    Refused {
        /// Why.
        reason: RefuseReason,
        /// The same in words, naming both versions on a mismatch.
        detail: String,
    },
    /// The lobby changed.
    Lobby(Lobby),
    /// The match starts (or is under way, for a joiner): build it from `setup`.
    Start {
        /// Everything the match is built from.
        setup: MatchSetup,
        /// The player of every seat.
        seat_players: Vec<SeatPlayer>,
        /// Percent of 20 ticks a second.
        speed: u32,
        /// Whether the server waits for every client's [`ClientMessage::Done`] before a tick.
        #[serde(default)]
        wait_for_clients: bool,
        /// Whether a [`ServerMessage::Snapshot`] follows (a match under way or resumed): the
        /// world is then the snapshot's, not the one built from `setup`.
        #[serde(default)]
        snapshot: bool,
    },
    /// The journal of one tick; one per tick, in order, empty ones included.
    Turn {
        /// The tick the journal belongs to.
        tick: u64,
        /// What changed the world before it stepped.
        entries: Vec<WireEntry>,
    },
    /// An order or visit the leader refused.
    Rejected {
        /// The client's number for it.
        seq: u64,
        /// Why.
        reason: String,
    },
    /// A save of the leader's world at `tick` follows as one binary frame of `bytes` bytes;
    /// turns from `tick` on follow it.
    Snapshot {
        /// The tick the saved world stands at.
        tick: u64,
        /// Size of the binary frame.
        bytes: u64,
    },
    /// The match is paused.
    Pause {
        /// Who paused it.
        by: String,
    },
    /// The match runs again.
    Resume {
        /// Who resumed it.
        by: String,
    },
    /// The speed changed.
    Speed {
        /// Percent of 20 ticks a second.
        pct: u32,
        /// Who set it.
        by: String,
    },
    /// A chat line, relayed.
    Chat {
        /// Everyone or allies.
        channel: Channel,
        /// The text.
        text: String,
        /// The sender's name.
        from: String,
        /// The sender's seat; none for the server's operator.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        seat: Option<u32>,
    },
    /// A seat's connection state changed.
    Seat {
        /// The seat.
        seat: u32,
        /// Its state.
        state: SeatState,
        /// While waiting: seconds left before a computer takes the seat.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        wait_seconds: Option<u32>,
    },
    /// An action this seat may not take (speed or start by a non-host, …).
    Denied {
        /// The action, as the message type that asked for it.
        action: String,
        /// Why, in words.
        reason: String,
    },
    /// Something the players should know (a save made, a desync repaired).
    Notice {
        /// In words.
        text: String,
    },
    /// Latency probe; answer with [`ClientMessage::Pong`].
    Ping {
        /// Echoed.
        nonce: u64,
    },
    /// Answer to [`ClientMessage::Ping`].
    Pong {
        /// Echoed.
        nonce: u64,
    },
    /// The server closes this connection (it stops, or the seat was taken over by a newer
    /// connection).
    Bye {
        /// Why.
        reason: String,
    },
}

/// Why a `Hello` was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefuseReason {
    /// The key was never issued.
    UnknownKey,
    /// The key was used already.
    KeyUsed,
    /// The seat token belongs to no seat.
    UnknownToken,
    /// Another protocol version.
    ProtocolMismatch,
    /// Another application version.
    VersionMismatch,
    /// Other rules (schema, seed or model files).
    RulesMismatch,
    /// Every human seat is taken.
    MatchFull,
    /// Too many failed attempts from this address; try again later.
    TooManyAttempts,
    /// Neither key nor token, or a malformed message.
    BadHello,
}

/// A chat channel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    /// Everyone.
    All,
    /// The sender's team.
    Allies,
}

/// A seat's state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SeatState {
    /// A human seat nobody has taken.
    Open,
    /// Its person is connected.
    Connected,
    /// Its person dropped; the match waits for them.
    Waiting,
    /// A computer plays it.
    Computer,
    /// Its person dropped; the match goes on and the seat is silent.
    Idle,
}

/// Who a seat is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SeatKind {
    /// A person (a computer holds it while nobody does).
    Human,
    /// A computer player for the whole match.
    Computer,
}

/// A seat as the lobby shows it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeatInfo {
    /// The seat, from 0.
    pub seat: u32,
    /// Its player in the match.
    pub player: u32,
    /// A person's or a computer's.
    pub kind: SeatKind,
    /// The person's name, or the computer's profile and difficulty.
    pub name: String,
    /// The faction's key.
    pub faction: String,
    /// Ready to start.
    pub ready: bool,
    /// Its person is connected.
    pub connected: bool,
    /// Its state.
    pub state: SeatState,
    /// May start the match and set the speed.
    pub host: bool,
}

/// The lobby: the match's settings and its seats.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lobby {
    /// The match as it will be built (factions as chosen so far).
    pub settings: MatchSetup,
    /// Every seat, in order.
    pub seats: Vec<SeatInfo>,
    /// `lobby`, `countdown`, `playing` or `stopped`.
    pub phase: String,
    /// Seconds before the start, during a countdown.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub countdown: Option<u32>,
    /// Percent of 20 ticks a second.
    pub speed: u32,
}

/// A seat and its player.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeatPlayer {
    /// The seat.
    pub seat: u32,
    /// Its player.
    pub player: u32,
}

/// What a match is played on, on the wire.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MapSpec {
    /// A universe of star systems (the game).
    Universe,
    /// One sphere planet of `n` tiles per face edge.
    Sphere {
        /// Tiles per face edge.
        n: u32,
    },
    /// A flat map.
    Flat {
        /// Width in tiles.
        width: u32,
        /// Height in tiles.
        height: u32,
    },
}

/// A match's whole setup as the wire carries it: [`MatchSettings`] in plain data. Every
/// machine builds the same world from it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchSetup {
    /// The map seed.
    pub seed: u64,
    /// The map style's key; the rules' default when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    /// The faction key of every player, in player order.
    pub factions: Vec<String>,
    /// Universe, sphere or flat map.
    pub map: MapSpec,
    /// The home planet's face size in a universe.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub home_size: Option<u32>,
    /// `separate` or `together`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_placement: Option<String>,
    /// Whether the city area limits building (task 158): `on` or `off`; absent is on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub city_area: Option<String>,
    /// Percent of every income drawn from the world (task 159): one of
    /// [`EconomyPace::GATHER_RATES`].
    pub gather_rate: u32,
    /// Percent of every research's time (task 159): one of [`EconomyPace::RESEARCH_TIMES`].
    pub research_time: u32,
    /// Every player starts in the age of this ordinal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub age: Option<i64>,
    /// A script's setup lines (test setups), scanned on every machine.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub script: String,
    /// `profile:difficulty` of every player no person plays, in player order.
    pub ai: Vec<String>,
    /// The players people play.
    pub humans: Vec<u32>,
}

impl MatchSetup {
    /// The settings every machine builds the match from.
    pub fn to_settings(&self) -> Result<MatchSettings, String> {
        let (map, map_size) = match self.map {
            MapSpec::Universe => (MapKind::Universe, ContentRequest::default().map_size),
            MapSpec::Sphere { n } => (MapKind::Sphere(n), ContentRequest::default().map_size),
            MapSpec::Flat { width, height } => (MapKind::Flat, (width, height)),
        };
        let start_placement = match &self.start_placement {
            Some(text) => Some(
                StartPlacement::parse(text)
                    .ok_or_else(|| format!("unknown start placement {text:?}"))?,
            ),
            None => None,
        };
        let city_area = match &self.city_area {
            Some(text) => {
                CityArea::parse(text).ok_or_else(|| format!("unknown city area {text:?}"))?
            }
            None => CityArea::default(),
        };
        let pace = EconomyPace {
            gather_rate: self.gather_rate,
            research_time: self.research_time,
        };
        if !pace.is_valid() {
            return Err(format!(
                "unknown economy pace: gathering {} %, research time {} %",
                self.gather_rate, self.research_time
            ));
        }
        let ai = self
            .ai
            .iter()
            .map(|spec| Spec::parse(spec))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(MatchSettings {
            content: ContentRequest {
                style: self.style.clone(),
                factions: Some(self.factions.clone()),
                players: self.factions.len() as u32,
                seed: self.seed,
                map_size,
                home_size: self.home_size,
                start_placement,
            },
            map,
            age: self.age,
            script: ScriptSetup::scan(&self.script)?,
            ai: Some(ai),
            humans: self.humans.iter().map(|p| PlayerId(*p)).collect(),
            city_area,
            pace,
        })
    }
}

/// A position on the wire.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WirePos {
    /// Milli-tiles.
    pub x: i32,
    /// Milli-tiles.
    pub y: i32,
    /// Cube face.
    pub face: u8,
    /// The body.
    pub body: u32,
}

impl From<Pos> for WirePos {
    fn from(pos: Pos) -> Self {
        Self {
            x: pos.x,
            y: pos.y,
            face: pos.face,
            body: pos.body.0,
        }
    }
}

impl From<WirePos> for Pos {
    fn from(pos: WirePos) -> Self {
        Pos::on(BodyId(pos.body), pos.face, pos.x, pos.y)
    }
}

/// A `rom_sim::Command` on the wire (the simulation has no serde).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireCommand {
    /// The issuing player; the server overwrites it with the seat's.
    #[serde(default)]
    pub player: u32,
    /// The entity given the order.
    pub executor: u32,
    /// The order's key.
    pub kind: String,
    /// A definition (what to build, research or trade).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<u32>,
    /// A target entity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_entity: Option<u32>,
    /// A target point.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_pos: Option<WirePos>,
    /// Appended to the executor's queue rather than replacing it.
    #[serde(default)]
    pub queue: bool,
    /// Free text: a city's new name (task 144).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// A construction's facing in degrees (task 147).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub facing: Option<u16>,
}

impl From<&Command> for WireCommand {
    fn from(command: &Command) -> Self {
        Self {
            player: command.player.0,
            executor: command.executor.0,
            kind: command.kind.clone(),
            subject: command.subject.map(|d| d.0),
            target_entity: command.target_entity.map(|e| e.0),
            target_pos: command.target_pos.map(WirePos::from),
            queue: command.queue,
            text: command.text.clone(),
            facing: command.facing,
        }
    }
}

impl From<WireCommand> for Command {
    fn from(command: WireCommand) -> Self {
        Self {
            player: PlayerId(command.player),
            executor: EntityId(command.executor),
            kind: command.kind,
            subject: command.subject.map(rom_rules::DefId),
            target_entity: command.target_entity.map(EntityId),
            target_pos: command.target_pos.map(Pos::from),
            queue: command.queue,
            text: command.text,
            facing: command.facing,
        }
    }
}

/// A journal entry on the wire.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WireEntry {
    /// An accepted command.
    Order(WireCommand),
    /// A planet's surface made, its terrain revealed to these players.
    Surface {
        /// The planet.
        body: u32,
        /// Who learns its terrain.
        revealed_for: Vec<u32>,
    },
}

impl From<&Entry> for WireEntry {
    fn from(entry: &Entry) -> Self {
        match entry {
            Entry::Order(command) => Self::Order(command.into()),
            Entry::SurfaceAdded { body, revealed_for } => Self::Surface {
                body: body.0,
                revealed_for: revealed_for.iter().map(|p| p.0).collect(),
            },
        }
    }
}

impl From<WireEntry> for Entry {
    fn from(entry: WireEntry) -> Self {
        match entry {
            WireEntry::Order(command) => Self::Order(command.into()),
            WireEntry::Surface { body, revealed_for } => Self::SurfaceAdded {
                body: BodyId(body),
                revealed_for: revealed_for.into_iter().map(PlayerId).collect(),
            },
        }
    }
}

/// The `Turn` message of a journal.
pub fn turn(journal: &Journal) -> ServerMessage {
    ServerMessage::Turn {
        tick: journal.tick,
        entries: journal.entries.iter().map(WireEntry::from).collect(),
    }
}

/// The journal a `Turn` carries.
pub fn journal(tick: u64, entries: Vec<WireEntry>) -> Journal {
    Journal {
        tick,
        entries: entries.into_iter().map(Entry::from).collect(),
    }
}

/// A message as the text of a frame.
pub fn encode<T: Serialize>(message: &T) -> String {
    serde_json::to_string(message).expect("protocol messages always serialise")
}

/// Reads a client's message.
pub fn decode_client(text: &str) -> Result<ClientMessage, serde_json::Error> {
    serde_json::from_str(text)
}

/// Reads the server's message.
pub fn decode_server(text: &str) -> Result<ServerMessage, serde_json::Error> {
    serde_json::from_str(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_and_entries_cross_the_wire_unchanged() {
        let command = Command {
            player: PlayerId(2),
            executor: EntityId(77),
            kind: "travel".into(),
            subject: Some(rom_rules::DefId(9)),
            target_entity: None,
            target_pos: Some(Pos::on(BodyId(4), 3, -1200, 560)),
            queue: true,
            text: None,
            facing: None,
        };
        // A city's new name crosses the wire as the order's text (task 144).
        let rename = Command {
            player: PlayerId(2),
            executor: EntityId(5),
            kind: rom_sim::RENAME_CITY.into(),
            subject: None,
            target_entity: None,
            target_pos: None,
            queue: false,
            text: Some("Nova Roma".into()),
            facing: None,
        };
        let journal = Journal {
            tick: 41,
            entries: vec![
                Entry::Order(command.clone()),
                Entry::Order(rename),
                Entry::SurfaceAdded {
                    body: BodyId(12),
                    revealed_for: vec![PlayerId(1), PlayerId(3)],
                },
            ],
        };
        let text = encode(&turn(&journal));
        let ServerMessage::Turn { tick, entries } = decode_server(&text).unwrap() else {
            panic!("{text}")
        };
        assert_eq!(super::journal(tick, entries), journal);
        let empty = encode(&turn(&Journal {
            tick: 123_456,
            entries: Vec::new(),
        }));
        // About 30 bytes on the wire (network.md, "Sizes and costs").
        assert!(empty.len() <= 48, "{empty}");
    }

    #[test]
    fn messages_are_tagged_json() {
        let hello = ClientMessage::Hello {
            protocol: PROTOCOL,
            version: "0.0.1".into(),
            rules: "ab".into(),
            name: "Ada".into(),
            key: Some("K".into()),
            seat_token: None,
        };
        let text = encode(&hello);
        assert!(text.starts_with("{\"type\":\"Hello\""), "{text}");
        assert_eq!(decode_client(&text).unwrap(), hello);
        assert_eq!(
            decode_client("{\"type\":\"Pause\"}").unwrap(),
            ClientMessage::Pause
        );
        let refused = ServerMessage::Refused {
            reason: RefuseReason::KeyUsed,
            detail: "used".into(),
        };
        assert!(encode(&refused).contains("\"key_used\""));
        assert!(decode_client("{\"type\":\"Nope\"}").is_err());
    }

    #[test]
    fn a_setup_gives_the_settings() {
        let setup = MatchSetup {
            seed: 7,
            style: Some("map.solar".into()),
            factions: vec!["a".into(), "b".into(), "c".into()],
            map: MapSpec::Universe,
            home_size: None,
            start_placement: Some("together".into()),
            city_area: Some("off".into()),
            gather_rate: 10,
            research_time: 400,
            age: Some(2),
            script: String::new(),
            ai: vec!["builder:tough".into()],
            humans: vec![1, 2],
        };
        let text = encode(&setup);
        let back: MatchSetup = serde_json::from_str(&text).unwrap();
        assert_eq!(back, setup);
        let settings = setup.to_settings().unwrap();
        assert_eq!(settings.map, MapKind::Universe);
        assert_eq!(settings.content.players, 3);
        assert_eq!(settings.humans, [PlayerId(1), PlayerId(2)]);
        assert_eq!(settings.ai.unwrap()[0].to_string(), "builder:tough");
        assert_eq!(
            settings.content.start_placement,
            Some(StartPlacement::Together)
        );
        assert_eq!(settings.city_area, CityArea::Free);
        assert_eq!(
            settings.pace,
            EconomyPace {
                gather_rate: 10,
                research_time: 400
            }
        );
        // Task 159: a pace outside the lists is refused.
        assert!(
            MatchSetup {
                gather_rate: 30,
                ..setup.clone()
            }
            .to_settings()
            .is_err()
        );
        assert!(
            MatchSetup {
                research_time: 300,
                ..setup.clone()
            }
            .to_settings()
            .is_err()
        );
        // A setup without the option (an older host's) keeps the city area limiting building.
        let older: MatchSetup =
            serde_json::from_str(&text.replace(",\"city_area\":\"off\"", "")).unwrap();
        assert_eq!(older.city_area, None);
        assert_eq!(
            older.to_settings().unwrap().city_area,
            CityArea::LimitsBuilding
        );
        assert!(
            MatchSetup {
                city_area: Some("maybe".into()),
                ..setup
            }
            .to_settings()
            .is_err()
        );
    }
}
