//! The simple launcher's four forms, window-independent: each one fills the same advanced
//! [`Form`] (and so the same `Options`) that the command line gives.

use std::collections::BTreeMap;

use serde_json::Value;

use super::form::Form;
use crate::args;

/// Whether Host, Join and Run a server may start (task 88 turns this on).
pub(super) const NETWORK_READY: bool = false;

/// Shown beside the disabled final button of the three network forms.
pub(super) const NETWORK_NOTE: &str = "Network play arrives with task 88";

/// The most players in one match.
const MAX_PLAYERS: u32 = 8;

/// The four things the simple launcher starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Mode {
    /// Standalone.
    Play,
    /// Standalone with a server: you play, others join.
    Host,
    /// A client.
    Join,
    /// A server without a game window.
    Serve,
}

impl Mode {
    pub const ALL: [Mode; 4] = [Mode::Play, Mode::Host, Mode::Join, Mode::Serve];

    /// The big choice's title.
    pub fn title(self) -> &'static str {
        match self {
            Mode::Play => "Play",
            Mode::Host => "Host a game",
            Mode::Join => "Join a game",
            Mode::Serve => "Run a server",
        }
    }

    /// The one line under the choice.
    pub fn blurb(self) -> &'static str {
        match self {
            Mode::Play => "Start a game on this computer against computer players.",
            Mode::Host => "You play here and others join over the network.",
            Mode::Join => "Join a game that someone else hosts or a server runs.",
            Mode::Serve => "Run a game server with no game window, only a status window.",
        }
    }

    /// The label of the form's final button.
    pub fn action(self) -> &'static str {
        match self {
            Mode::Play => "Start",
            Mode::Host => "Host",
            Mode::Join => "Join",
            Mode::Serve => "Start server",
        }
    }

    /// Whether this mode needs the network (and so waits for task 88).
    pub fn needs_network(self) -> bool {
        self != Mode::Play
    }
}

/// Every field of every simple form, with its default; the values are the text the form shows.
pub(super) const FIELDS: &[(&str, &str)] = &[
    // Play and Host: your side.
    ("race", ""),
    ("opponents", "3"),
    ("difficulty", ""),
    // Shared by Play, Host and Run a server.
    ("style", "map.many_worlds"),
    ("seed", ""),
    ("speed", "100"),
    ("drop", "pause-then-computer"),
    // Host and Run a server.
    ("remote", "1"),
    ("port", "7400"),
    // Join.
    ("url", ""),
    ("key", ""),
    ("name", ""),
    // Run a server.
    ("bind", "127.0.0.1"),
    ("humans", "2"),
    ("computers", "0"),
    ("autosave", "5"),
];

/// The values of the simple forms, remembered between runs in the launcher's settings file.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Simple {
    values: BTreeMap<String, String>,
}

impl Default for Simple {
    fn default() -> Self {
        Self {
            values: FIELDS
                .iter()
                .map(|(key, value)| ((*key).into(), (*value).into()))
                .collect(),
        }
    }
}

impl Simple {
    /// The text of a field.
    pub fn get(&self, key: &str) -> &str {
        self.values.get(key).map_or("", String::as_str)
    }

    /// Changes one field.
    pub fn set(&mut self, key: &str, value: impl Into<String>) {
        self.values.insert(key.into(), value.into());
    }

    /// Every field, for the settings file.
    pub fn values(&self) -> &BTreeMap<String, String> {
        &self.values
    }

    /// Reads known fields; others and mistyped ones keep their defaults.
    pub fn from_json(value: &Value) -> Self {
        let mut simple = Self::default();
        for (key, _) in FIELDS {
            if let Some(text) = value.get(*key).and_then(Value::as_str) {
                simple.set(key, text);
            }
        }
        simple
    }

    /// A whole number in `low..=high`, or the sentence that says what is wrong.
    fn number(&self, key: &str, label: &str, low: u32, high: u32) -> Result<u32, String> {
        self.get(key)
            .trim()
            .parse::<u32>()
            .ok()
            .filter(|n| (low..=high).contains(n))
            .ok_or_else(|| format!("{label}: a whole number from {low} to {high}"))
    }

    fn port(&self) -> Result<u16, String> {
        self.get("port")
            .trim()
            .parse::<u16>()
            .ok()
            .filter(|p| *p > 0)
            .ok_or_else(|| "Port: a whole number from 1 to 65535".into())
    }

    fn url(&self) -> Result<String, String> {
        let url = self.get("url").trim();
        let host = url
            .strip_prefix("http://")
            .or_else(|| url.strip_prefix("https://"))
            .unwrap_or("");
        if host.is_empty() || host.starts_with('/') || url.contains(char::is_whitespace) {
            Err("Server address: it must start with http:// or https://".into())
        } else {
            Ok(url.into())
        }
    }

    /// The faction list for `players` players when the person chose a race: that race first,
    /// the others from the default mix (`mix`, longer than any match).
    fn factions(&self, players: u32, mix: &[String]) -> Option<String> {
        let race = self.get("race");
        if race.is_empty() {
            return None;
        }
        let mut others: Vec<&String> = mix.iter().collect();
        if let Some(at) = others.iter().position(|key| key.as_str() == race) {
            others.remove(at);
        }
        let rest = (players as usize).saturating_sub(1);
        let list: Vec<&str> = std::iter::once(race)
            .chain(
                others
                    .iter()
                    .cycle()
                    .take(if others.is_empty() { 0 } else { rest })
                    .map(|key| key.as_str()),
            )
            .collect();
        (list.len() == players as usize).then(|| list.join(","))
    }

    /// `profile:difficulty` for each of `count` computer seats; `none` when there are none.
    fn computers(&self, count: u32) -> String {
        if count == 0 {
            return "none".into();
        }
        let (profile, default) = crate::computer::DEFAULT_SPEC;
        let difficulty = match self.get("difficulty") {
            "" => default,
            chosen => chosen,
        };
        vec![format!("{profile}:{difficulty}"); count as usize].join(",")
    }

    /// The advanced form this mode's fields stand for. `mix` is the rules' default race mix and
    /// `random_seed` replaces a blank seed. Fails with a sentence naming the first bad field.
    pub fn form(&self, mode: Mode, mix: &[String], random_seed: u64) -> Result<Form, String> {
        let mut form = Form::default();
        if mode == Mode::Join {
            form.set("connect", self.url()?);
            if self.get("key").trim().is_empty() {
                return Err("Key: the server's key for you".into());
            }
            form.set("key", self.get("key").trim());
            self.name_into(&mut form)?;
            return Ok(form);
        }
        // The match: Play, Host and Run a server start universe matches.
        form.set("universe", "true");
        form.set("style", self.get("style"));
        let seed = self.get("seed").trim();
        if seed.is_empty() {
            form.set("seed", random_seed.to_string());
        } else {
            seed.parse::<u64>()
                .map_err(|_| "Seed: a whole number, or blank for a random one".to_string())?;
            form.set("seed", seed);
        }
        form.set("speed", self.get("speed").trim());
        let (people, computers) = match mode {
            Mode::Play => (
                1,
                self.number("opponents", "Opponents", 0, MAX_PLAYERS - 1)?,
            ),
            Mode::Host => {
                let remote = self.number("remote", "Remote players", 1, MAX_PLAYERS - 1)?;
                (
                    1 + remote,
                    self.number("opponents", "Opponents", 0, MAX_PLAYERS - 1)?,
                )
            }
            _ => (
                self.number("humans", "Human seats", 1, MAX_PLAYERS)?,
                self.number("computers", "Computer seats", 0, MAX_PLAYERS - 1)?,
            ),
        };
        let players = people + computers;
        if players > MAX_PLAYERS {
            return Err(format!(
                "A match has at most {MAX_PLAYERS} players; this one has {players}"
            ));
        }
        form.set("players", players.to_string());
        form.set("ai", self.computers(computers));
        if mode != Mode::Serve
            && let Some(list) = self.factions(players, mix)
        {
            form.set("factions", list);
        }
        match mode {
            Mode::Play => {}
            Mode::Host => {
                form.set("host", self.port()?.to_string());
                form.set("remote-players", (people - 1).to_string());
                form.set("bind", "0.0.0.0");
                form.set("on-disconnect", self.get("drop"));
            }
            _ => {
                form.set("serve", "true");
                form.set("host", self.port()?.to_string());
                let bind = self.get("bind").trim();
                if bind.is_empty() || bind.contains(char::is_whitespace) {
                    return Err("Listen address: for example 127.0.0.1 or 0.0.0.0".into());
                }
                form.set("bind", bind);
                form.set("remote-players", people.to_string());
                form.set("on-disconnect", self.get("drop"));
                form.set(
                    "autosave-minutes",
                    self.number("autosave", "Autosave minutes", 0, 1440)?
                        .to_string(),
                );
            }
        }
        Ok(form)
    }

    fn name_into(&self, form: &mut Form) -> Result<(), String> {
        let name = self.get("name").trim();
        if name.chars().count() > 32 {
            return Err("Your name: at most 32 characters".into());
        }
        if !name.is_empty() {
            form.set("name", name);
        }
        Ok(())
    }

    /// The form that takes the seat a server remembered for this address (`token`).
    pub fn rejoin_form(&self, token: &str) -> Result<Form, String> {
        let mut form = Form::default();
        form.set("connect", self.url()?);
        form.set("token", token);
        self.name_into(&mut form)?;
        Ok(form)
    }

    /// The options a Start press runs with, or the first thing wrong with the fields.
    #[cfg(test)]
    pub fn options(
        &self,
        mode: Mode,
        mix: &[String],
        random_seed: u64,
    ) -> Result<args::Options, String> {
        self.form(mode, mix, random_seed)?.validate()
    }
}

/// The disconnect choices the forms offer: command-line value and words.
pub(super) fn drop_choices() -> [(&'static str, &'static str); 3] {
    use args::DisconnectPolicy::*;
    [
        (PauseThenComputer.name(), "Pause, then a computer plays"),
        (Idle.name(), "Keep playing, their units stand idle"),
        (Computer.name(), "A computer plays at once"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::Options;

    fn parse(list: &[&str]) -> Options {
        args::parse(list.iter().map(|s| (*s).to_owned())).unwrap()
    }

    fn mix() -> Vec<String> {
        ["a", "b", "c", "d", "e", "f", "g", "h"]
            .iter()
            .map(|s| (*s).to_owned())
            .collect()
    }

    #[test]
    fn defaults_match_the_table_and_port() {
        assert_eq!(Simple::default().get("port"), "7400");
        assert_eq!(Simple::default().get("opponents"), "3");
    }

    #[test]
    fn play_gives_the_command_line_options() {
        let simple = Simple::default();
        let options = simple.options(Mode::Play, &mix(), 77).unwrap();
        let ai = "balanced:moderate,balanced:moderate,balanced:moderate";
        assert_eq!(
            options,
            parse(&[
                "--universe",
                "--style",
                "map.many_worlds",
                "--seed",
                "77",
                "--players",
                "4",
                "--ai",
                ai,
            ])
        );
        let mut chosen = Simple::default();
        chosen.set("race", "c");
        chosen.set("opponents", "2");
        chosen.set("difficulty", "tough");
        chosen.set("style", "map.solar");
        chosen.set("seed", "12");
        chosen.set("speed", "200");
        assert_eq!(
            chosen.options(Mode::Play, &mix(), 77).unwrap(),
            parse(&[
                "--universe",
                "--style",
                "map.solar",
                "--seed",
                "12",
                "--factions",
                "c,a,b",
                "--ai",
                "balanced:tough,balanced:tough",
                "--speed",
                "200",
            ])
        );
        chosen.set("opponents", "0");
        let alone = chosen.options(Mode::Play, &mix(), 1).unwrap();
        assert_eq!((alone.players, alone.no_ai), (1, true));
    }

    #[test]
    fn host_adds_the_network_options() {
        let mut simple = Simple::default();
        simple.set("remote", "2");
        simple.set("opponents", "1");
        simple.set("port", "7401");
        simple.set("drop", "idle");
        simple.set("seed", "5");
        assert_eq!(
            simple.options(Mode::Host, &mix(), 1).unwrap(),
            parse(&[
                "--universe",
                "--style",
                "map.many_worlds",
                "--seed",
                "5",
                "--players",
                "4",
                "--ai",
                "balanced:moderate",
                "--host",
                "7401",
                "--remote-players",
                "2",
                "--bind",
                "0.0.0.0",
                "--on-disconnect",
                "idle",
            ])
        );
        simple.set("remote", "7");
        assert!(simple.options(Mode::Host, &mix(), 1).is_err());
    }

    #[test]
    fn join_and_rejoin() {
        let mut simple = Simple::default();
        assert!(simple.options(Mode::Join, &mix(), 1).is_err());
        simple.set("url", "https://play.example.org/ascension/");
        assert!(simple.options(Mode::Join, &mix(), 1).is_err(), "no key yet");
        simple.set("key", " k-1 ");
        simple.set("name", "Ana");
        assert_eq!(
            simple.options(Mode::Join, &mix(), 1).unwrap(),
            parse(&[
                "--connect",
                "https://play.example.org/ascension/",
                "--key",
                "k-1",
                "--name",
                "Ana",
            ])
        );
        assert_eq!(
            simple.rejoin_form("tok").unwrap().validate().unwrap(),
            parse(&[
                "--connect",
                "https://play.example.org/ascension/",
                "--token",
                "tok",
                "--name",
                "Ana",
            ])
        );
    }

    #[test]
    fn serve_gives_the_server_options() {
        let mut simple = Simple::default();
        simple.set("bind", "0.0.0.0");
        simple.set("port", "7402");
        simple.set("humans", "3");
        simple.set("computers", "2");
        simple.set("difficulty", "easy");
        simple.set("autosave", "10");
        simple.set("seed", "9");
        simple.set("race", "c");
        assert_eq!(
            simple.options(Mode::Serve, &mix(), 1).unwrap(),
            parse(&[
                "--universe",
                "--style",
                "map.many_worlds",
                "--seed",
                "9",
                "--players",
                "5",
                "--ai",
                "balanced:easy,balanced:easy",
                "--serve",
                "--host",
                "7402",
                "--bind",
                "0.0.0.0",
                "--remote-players",
                "3",
                "--autosave-minutes",
                "10",
            ])
        );
    }

    #[test]
    fn invalid_fields_are_refused() {
        let bad = |mode: Mode, key: &str, value: &str| {
            let mut simple = Simple::default();
            simple.set("url", "http://127.0.0.1:7400/");
            simple.set("key", "k");
            simple.set(key, value);
            simple.options(mode, &mix(), 1).is_err()
        };
        for url in ["ftp://x", "play.example.org", "http://", "https:///x", ""] {
            assert!(bad(Mode::Join, "url", url), "{url}");
        }
        assert!(bad(Mode::Join, "key", "  "));
        for port in ["0", "65536", "abc", ""] {
            assert!(bad(Mode::Host, "port", port), "{port}");
            assert!(bad(Mode::Serve, "port", port), "{port}");
        }
        assert!(bad(Mode::Play, "opponents", "8"));
        assert!(bad(Mode::Play, "seed", "-1"));
        assert!(bad(Mode::Play, "speed", "0"));
        assert!(bad(Mode::Serve, "bind", ""));
        assert!(bad(Mode::Serve, "humans", "0"));
        assert!(bad(Mode::Serve, "autosave", "x"));
    }

    #[test]
    fn advanced_gets_the_same_options() {
        let mut simple = Simple::default();
        simple.set("humans", "2");
        simple.set("seed", "3");
        for mode in Mode::ALL {
            simple.set("url", "http://127.0.0.1:7400/");
            simple.set("key", "k");
            let form = simple.form(mode, &mix(), 1).unwrap();
            let again = args::parse(form.arguments()).unwrap();
            assert_eq!(again, simple.options(mode, &mix(), 1).unwrap(), "{mode:?}");
        }
    }

    #[test]
    fn remembered_values_come_back() {
        let mut simple = Simple::default();
        simple.set("seed", "31");
        simple.set("url", "http://x/");
        let json: Value = serde_json::to_value(simple.values()).unwrap();
        assert_eq!(Simple::from_json(&json), simple);
        let partial = serde_json::json!({"seed": "4", "future": "x", "port": 3});
        let loaded = Simple::from_json(&partial);
        assert_eq!((loaded.get("seed"), loaded.get("port")), ("4", "7400"));
    }
}
