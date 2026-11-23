//! The ages as the game tells them (task 133): each age's subtitle, story and music, read by
//! ordinal from `assets/text/ages.toml` (kept in step with `docs/ages-story.md`), and the
//! new-age dialog the HUD draws from them and the rules.

use std::sync::OnceLock;

use rom_rules::{Era, Rules};

/// One age of `ages.toml`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgeText {
    /// The age's ordinal in the rules.
    pub ordinal: i64,
    /// Its short name as the document heads it ("Ancient").
    pub name: String,
    /// Its subtitle ("the first cities").
    pub subtitle: String,
    /// The music that announces it.
    pub theme: String,
    /// The music whose fragments follow it.
    pub interlude: String,
    /// The story the new-age dialog shows.
    pub story: String,
}

/// Every age of `ages.toml`, in ordinal order.
pub fn ages() -> &'static [AgeText] {
    static AGES: OnceLock<Vec<AgeText>> = OnceLock::new();
    AGES.get_or_init(|| {
        parse(include_str!("../../../assets/text/ages.toml")).unwrap_or_else(|e| {
            eprintln!("rise-of-man: assets/text/ages.toml: {e}");
            Vec::new()
        })
    })
}

/// The age with this ordinal.
pub fn age(ordinal: i64) -> Option<&'static AgeText> {
    ages().iter().find(|a| a.ordinal == ordinal)
}

/// Reads the `[[age]]` tables of `text`.
pub fn parse(text: &str) -> Result<Vec<AgeText>, String> {
    let table: toml::Table = text.parse().map_err(|e: toml::de::Error| e.to_string())?;
    let rows = table
        .get("age")
        .and_then(toml::Value::as_array)
        .ok_or("no [[age]] tables")?;
    let mut ages = rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let text = |key: &str| {
                row.get(key)
                    .and_then(toml::Value::as_str)
                    .map(str::to_owned)
                    .ok_or_else(|| format!("age {i}: no {key}"))
            };
            Ok(AgeText {
                ordinal: row
                    .get("ordinal")
                    .and_then(toml::Value::as_integer)
                    .ok_or_else(|| format!("age {i}: no ordinal"))?,
                name: text("name")?,
                subtitle: text("subtitle")?,
                theme: text("theme")?,
                interlude: text("interlude")?,
                story: text("story")?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    ages.sort_by_key(|a| a.ordinal);
    Ok(ages)
}

/// The era as the dialog names it.
pub fn era_name(era: Era) -> &'static str {
    match era {
        Era::History => "History",
        Era::NearFuture => "The near future",
        Era::Stellar => "The stellar era",
        Era::Galactic => "The galactic era",
    }
}

/// The new-age dialog for the age `ordinal` of `rules`: its number, the rules' name for it, its
/// era and subtitle, and its story.
pub fn dialog(rules: &Rules, ordinal: i64) -> Option<rom_render::HudAgeDialog> {
    let age = rules.ages.values().find(|a| a.ordinal == ordinal)?;
    let last = rules
        .ages
        .values()
        .map(|a| a.ordinal)
        .max()
        .unwrap_or(ordinal);
    let text = self::age(ordinal);
    let mut subtitle = text.map(|t| t.subtitle.clone()).unwrap_or_default();
    if let Some(first) = subtitle.get(..1) {
        subtitle = first.to_uppercase() + &subtitle[1..];
    }
    Some(rom_render::HudAgeDialog {
        // Counted from one for the player: the Ancient age is age 1 of 14.
        heading: format!(
            "AGE {} OF {}  ·  {}",
            ordinal + 1,
            last + 1,
            era_name(age.era).to_uppercase()
        ),
        name: rules.def(age.id).name.clone(),
        subtitle,
        story: text.map(|t| t.story.clone()).unwrap_or_default(),
        hint: "Click or press any key to continue".to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The numbered sections of the document: ordinal, name, subtitle and story.
    fn from_document() -> Vec<(i64, String, String, String)> {
        let doc = include_str!("../../../docs/ages-story.md");
        let mut out = Vec::new();
        let sections: Vec<&str> = doc.split("\n## ").skip(1).collect();
        for section in sections {
            let head = section.lines().next().unwrap_or_default();
            let Some((number, rest)) = head.split_once(". ") else {
                continue;
            };
            let Ok(ordinal) = number.parse::<i64>() else {
                continue;
            };
            let (name, subtitle) = rest.split_once(" — ").expect("name — subtitle");
            let story = section
                .split_once("*Story:* \"")
                .and_then(|(_, s)| s.split_once("\"\n"))
                .map(|(s, _)| s.split_whitespace().collect::<Vec<_>>().join(" "))
                .expect("a story");
            out.push((ordinal, name.to_owned(), subtitle.to_owned(), story));
        }
        out
    }

    #[test]
    fn the_file_keeps_in_step_with_the_document() {
        let doc = from_document();
        assert_eq!(doc.len(), 14);
        let ages = ages();
        assert_eq!(ages.len(), doc.len());
        for (age, (ordinal, name, subtitle, story)) in ages.iter().zip(&doc) {
            assert_eq!(
                (&age.ordinal, &age.name, &age.subtitle, &age.story),
                (ordinal, name, subtitle, story)
            );
            assert!(rom_audio::MusicId::from_name(&age.theme).is_some());
            assert!(rom_audio::MusicId::from_name(&age.interlude).is_some());
        }
    }

    #[test]
    fn every_age_of_the_rules_has_its_dialog() {
        let rules = crate::terrain::load_rules().unwrap();
        for age in rules.ages.values() {
            let dialog = dialog(&rules, age.ordinal).expect("a dialog");
            assert_eq!(dialog.name, rules.def(age.id).name);
            assert!(!dialog.story.is_empty() && !dialog.subtitle.is_empty());
            assert!(
                dialog
                    .heading
                    .starts_with(&format!("AGE {} OF 14", age.ordinal + 1))
            );
        }
        let first = dialog(&rules, 0).unwrap();
        assert_eq!(first.subtitle, "The first cities");
        assert!(first.heading.ends_with("HISTORY"));
        assert!(dialog(&rules, 99).is_none());
        assert!(parse("[[age]]\nordinal = 1").is_err());
    }
}
