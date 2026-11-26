//! Match presets (task 112): a name, a line, an icon, a picture and the Play form's fields they
//! set. The bundled presets are `app/assets/launcher/presets.json`; the person's own are kept in
//! the launcher's settings; both export to and import from a file in the same format:
//!
//! ```json
//! { "format": "rise-of-man presets 1",
//!   "presets": [ { "name": "Classic", "line": "…", "icon": "tab_political",
//!                  "picture": "tiles/classic",
//!                  "fields": { "style": "map.many_worlds", "opponents": "3" } } ] }
//! ```
//!
//! `fields` are the Play form's (`simple::FIELDS`: style, opponents, difficulty, placement,
//! city_area, gather_rate, research_time, seed, speed, age, script, watch); `script` names a bundled script or a file. `icon` and
//! `picture` are optional.

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::{Value, json};

use super::scripts;
use super::simple::{FIELDS, Simple};

/// The format line every presets file starts with.
pub(super) const FORMAT: &str = "rise-of-man presets 1";

/// The fields a preset may set: the match, never the person's race, name or server.
pub(super) const MATCH_FIELDS: [&str; 12] = [
    "style",
    "opponents",
    "difficulty",
    "placement",
    "city_area",
    "gather_rate",
    "research_time",
    "seed",
    "speed",
    "age",
    "script",
    "watch",
];

/// One preset.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Preset {
    pub name: String,
    pub line: String,
    pub icon: String,
    pub picture: String,
    pub fields: BTreeMap<String, String>,
}

/// The presets that come with the game.
pub(super) fn bundled() -> Vec<Preset> {
    parse(include_str!("../../../../assets/launcher/presets.json")).unwrap_or_default()
}

/// Reads a presets file; refuses another format or a field the Play form does not have.
pub(super) fn parse(text: &str) -> Result<Vec<Preset>, String> {
    let value: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    if value["format"].as_str() != Some(FORMAT) {
        return Err(format!(
            "not a presets file (the format line is \"{FORMAT}\")"
        ));
    }
    let list = value["presets"]
        .as_array()
        .ok_or("the file has no \"presets\" list")?;
    list.iter().map(from_json).collect()
}

fn from_json(value: &Value) -> Result<Preset, String> {
    let text = |key: &str| value[key].as_str().unwrap_or_default().to_owned();
    let name = text("name");
    if name.trim().is_empty() {
        return Err("a preset has no name".into());
    }
    let mut fields = BTreeMap::new();
    for (key, field) in value["fields"].as_object().into_iter().flatten() {
        if !MATCH_FIELDS.contains(&key.as_str()) {
            return Err(format!("{name}: \"{key}\" is not a field a preset sets"));
        }
        let field = field
            .as_str()
            .ok_or_else(|| format!("{name}: \"{key}\" must be text"))?;
        fields.insert(key.clone(), field.to_owned());
    }
    Ok(Preset {
        name,
        line: text("line"),
        icon: text("icon"),
        picture: text("picture"),
        fields,
    })
}

/// Writes presets in the file format.
pub(super) fn to_json(presets: &[Preset]) -> String {
    let list: Vec<Value> = presets.iter().map(Preset::json).collect();
    serde_json::to_string_pretty(&json!({ "format": FORMAT, "presets": list })).unwrap_or_default()
}

impl Preset {
    /// The preset as one entry of a file (or the settings).
    pub fn json(&self) -> Value {
        json!({
            "name": self.name,
            "line": self.line,
            "icon": self.icon,
            "picture": self.picture,
            "fields": self.fields,
        })
    }

    /// Reads one entry as [`Preset::json`] writes it.
    pub fn from_value(value: &Value) -> Option<Self> {
        from_json(value).ok()
    }

    /// A preset of the Play form's current match fields.
    pub fn of(simple: &Simple, name: &str, line: &str) -> Self {
        Self {
            name: name.to_owned(),
            line: line.to_owned(),
            icon: String::new(),
            picture: String::new(),
            fields: MATCH_FIELDS
                .iter()
                .filter(|key| {
                    let default = FIELDS.iter().find(|(k, _)| k == *key).map(|(_, v)| *v);
                    Some(simple.get(key)) != default
                })
                .map(|key| ((*key).to_owned(), simple.get(key).to_owned()))
                .collect(),
        }
    }

    /// The Play form's fields with this preset applied: the match fields start from their
    /// defaults, the person's own (race, name, server) stay. A bundled script's name becomes
    /// its file beside the settings at `settings`.
    pub fn apply(&self, base: &Simple, settings: &Path) -> Simple {
        let mut simple = base.clone();
        let defaults = Simple::default();
        for key in MATCH_FIELDS {
            simple.set(key, defaults.get(key));
        }
        for (key, value) in &self.fields {
            let value = if key == "script" && scripts::BUNDLED.iter().any(|(name, _)| name == value)
            {
                scripts::path(settings, value)
                    .to_string_lossy()
                    .into_owned()
            } else {
                value.clone()
            };
            simple.set(key, value);
        }
        simple
    }
}

#[cfg(test)]
mod tests {
    use super::super::simple::Mode;
    use super::*;
    use crate::args;

    fn mix() -> Vec<String> {
        ["a", "b", "c", "d", "e", "f", "g", "h"]
            .iter()
            .map(|s| (*s).to_owned())
            .collect()
    }

    fn parse_args(list: &[&str]) -> args::Options {
        args::parse(list.iter().map(|s| (*s).to_owned())).unwrap()
    }

    #[test]
    fn bundled_presets_give_their_command_lines() {
        let settings = Path::new("/cfg/rise-of-man/launcher.json");
        let presets = bundled();
        assert!(presets.len() >= 7);
        let line = |name: &str| {
            let preset = presets.iter().find(|p| p.name == name).unwrap();
            preset
                .apply(&Simple::default(), settings)
                .options(Mode::Play, &mix(), 9)
                .unwrap()
        };
        let base = ["--universe", "--seed", "9"];
        let with = |extra: &[&str]| {
            let mut all: Vec<&str> = base.to_vec();
            all.extend_from_slice(extra);
            parse_args(&all)
        };
        assert_eq!(
            line("Quick match"),
            with(&[
                "--style",
                "map.one_world",
                "--players",
                "2",
                "--ai",
                "balanced:easy"
            ])
        );
        assert_eq!(
            line("Classic"),
            with(&[
                "--style",
                "map.many_worlds",
                "--players",
                "4",
                "--ai",
                "balanced:moderate,balanced:moderate,balanced:moderate"
            ])
        );
        assert_eq!(line("Galactic war").players, 8);
        assert_eq!(line("Galactic war").ai[6].to_string(), "balanced:tougher");
        assert_eq!(
            line("Start in space"),
            with(&[
                "--style",
                "map.many_worlds",
                "--players",
                "4",
                "--ai",
                "balanced:moderate,balanced:moderate,balanced:moderate",
                "--age",
                "10",
                "--script",
                "/cfg/rise-of-man/scripts/rich-start-10-starfaring.txt",
            ])
        );
        assert_eq!(
            line("Sandbox"),
            with(&[
                "--style",
                "map.many_worlds",
                "--players",
                "1",
                "--ai",
                "none",
                "--age",
                "14",
                "--script",
                "/cfg/rise-of-man/scripts/rich-start-14-federation.txt",
            ])
        );
        assert_eq!(
            line("Peaceful build"),
            with(&[
                "--style",
                "map.many_worlds",
                "--players",
                "1",
                "--ai",
                "none"
            ])
        );
        assert_eq!(
            line("Big battle").script.as_deref(),
            Some(Path::new("/cfg/rise-of-man/scripts/big-battle.txt"))
        );
        let watch = line("Watch the computers");
        assert_eq!(
            watch,
            with(&[
                "--style",
                "map.one_world",
                "--players",
                "4",
                "--watch",
                "--ai",
                "balanced:moderate,balanced:moderate,balanced:moderate,balanced:moderate",
                "--speed",
                "200",
            ])
        );
        // Every named icon and picture exists, every named script is bundled.
        let chrome = crate::style::chrome();
        for preset in &presets {
            assert!(chrome.picture(&preset.picture).is_some(), "{}", preset.name);
            assert!(crate::style::has_icon(&preset.icon), "{}", preset.name);
        }
    }

    #[test]
    fn a_preset_keeps_the_persons_own_fields() {
        let mut mine = Simple::default();
        mine.set("race", "c");
        mine.set("opponents", "6");
        mine.set("script", "/old.txt");
        let preset = Preset {
            name: "x".into(),
            line: String::new(),
            icon: String::new(),
            picture: String::new(),
            fields: BTreeMap::from([("style".to_owned(), "map.solar".to_owned())]),
        };
        let applied = preset.apply(&mine, Path::new("/cfg/launcher.json"));
        assert_eq!(applied.get("race"), "c");
        assert_eq!(applied.get("opponents"), "3");
        assert_eq!(applied.get("script"), "");
        assert_eq!(applied.get("style"), "map.solar");
    }

    #[test]
    fn export_then_import_round_trips() {
        let mut mine = Simple::default();
        mine.set("style", "map.sector");
        mine.set("opponents", "5");
        mine.set("script", "/home/me/my script.txt");
        let own = Preset::of(&mine, "Mine", "Five in a sector");
        assert_eq!(own.fields.len(), 3);
        let mut all = bundled();
        all.push(own.clone());
        let text = to_json(&all);
        assert_eq!(parse(&text).unwrap(), all);
        let back = parse(&text).unwrap().pop().unwrap();
        let applied = back.apply(&Simple::default(), Path::new("/cfg/launcher.json"));
        assert_eq!(applied.get("script"), "/home/me/my script.txt");
        // Not a presets file, or a field no preset sets.
        assert!(parse("{}").is_err());
        assert!(
            parse(&format!(
                r#"{{"format":"{FORMAT}","presets":[{{"name":"x","fields":{{"port":"1"}}}}]}}"#
            ))
            .is_err()
        );
        assert!(
            parse(&format!(
                r#"{{"format":"{FORMAT}","presets":[{{"fields":{{}}}}]}}"#
            ))
            .is_err()
        );
    }
}
