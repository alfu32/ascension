//! Task 144: cities have names. A new city (a site included) takes the first name of its
//! owner's list that no city in the match carries; the list is the `city` pool of the
//! player's starting nation (`faction_name`), or of the first faction up its ladder that has
//! one, so blocs and united humanity go on with the nation's names. When the list runs out the
//! names come round again as "New <name>", then "New <name> 2", "New <name> 3" and so on. A
//! faction without any list names its cities after the city type: "City 1", "City 2", ….
//!
//! The capital keeps the first name; a captured city keeps its name; a city that is gone frees
//! its name. The `rename_city` order gives an own city the name in the order's text at once.
//! Names are world state: hashed, saved and the same on every machine.

use crate::{Command, EntityId, OrderId, PlayerId, SimError, World};
use rom_rules::{DefId, FactionNameKind};
use std::collections::BTreeSet;

/// The order that renames an own city.
pub const RENAME_CITY: &str = "rename_city";

/// Longest city name when the rules give no `city.name_max_chars`.
const DEFAULT_MAX_CHARS: usize = 24;

/// A name as a player gave it, made fit to show: control characters dropped, every run of
/// white space one space, the ends trimmed, at most `max` characters.
pub fn clean_city_name(text: &str, max: usize) -> String {
    let mut out = String::new();
    let mut space = false;
    for c in text.chars().filter(|c| printable(*c)) {
        if c.is_whitespace() {
            space = !out.is_empty();
            continue;
        }
        if out.chars().count() >= max {
            break;
        }
        if space {
            if out.chars().count() + 1 >= max {
                break;
            }
            out.push(' ');
            space = false;
        }
        out.push(c);
    }
    out
}

/// Whether a character may stand in a name: not a control character (white space aside, which
/// becomes a space), not an invisible format mark (zero-width or direction override).
fn printable(c: char) -> bool {
    let format = matches!(c, '\u{200b}'..='\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2060}'..='\u{206f}' | '\u{feff}');
    (!c.is_control() || c.is_whitespace()) && !format
}

impl World {
    /// A city's name, for cities and city sites.
    pub fn city_name(&self, id: EntityId) -> Option<&str> {
        self.territory.names.get(&id).map(String::as_str)
    }

    /// Every city's name, in city id order.
    pub fn city_names(&self) -> impl Iterator<Item = (EntityId, &str)> + '_ {
        self.territory
            .names
            .iter()
            .map(|(id, name)| (*id, name.as_str()))
    }

    /// The longest name `rename_city` keeps, in characters (`city.name_max_chars`).
    pub fn city_name_max_chars(&self) -> usize {
        self.rules
            .constant("city.name_max_chars")
            .and_then(|n| usize::try_from(n).ok())
            .filter(|n| *n > 0)
            .unwrap_or(DEFAULT_MAX_CHARS)
    }

    /// The city names of the first faction with a list: the player's starting faction and
    /// those above it, then its present faction and those above it.
    fn city_name_list(&self, player: PlayerId) -> Vec<&str> {
        let present = self.players.get(&player).map(|p| p.faction);
        let mut seen = BTreeSet::new();
        for start in [self.origin_faction(player), present].into_iter().flatten() {
            let mut current = Some(start);
            while let Some(faction) = current.filter(|f| seen.insert(*f)) {
                let names: Vec<&str> = self
                    .rules
                    .faction_names
                    .get(&faction)
                    .into_iter()
                    .flatten()
                    .filter(|row| row.kind == FactionNameKind::City)
                    .map(|row| row.name.as_str())
                    .collect();
                if !names.is_empty() {
                    return names;
                }
                current = self.rules.factions.get(&faction).and_then(|f| f.parent_id);
            }
        }
        Vec::new()
    }

    /// The first name of `player`'s list that no city carries; `type_id` is the city's type,
    /// whose name numbers the cities of a faction without a list.
    fn next_city_name(&self, player: PlayerId, type_id: DefId) -> String {
        let taken: BTreeSet<&str> = self.territory.names.values().map(String::as_str).collect();
        let list = self.city_name_list(player);
        if list.is_empty() {
            let faction = self.players.get(&player).map_or(DefId(0), |p| p.faction);
            let base = self.rules.display_name(type_id, faction);
            return (1u64..)
                .map(|n| format!("{base} {n}"))
                .find(|name| !taken.contains(name.as_str()))
                .expect("a free number");
        }
        (0u64..)
            .flat_map(|round| {
                list.iter().map(move |name| match round {
                    0 => (*name).to_owned(),
                    1 => format!("New {name}"),
                    n => format!("New {name} {n}"),
                })
            })
            .find(|name| !taken.contains(name.as_str()))
            .expect("a free name")
    }

    /// Names a city that has none yet, from its owner's list.
    pub(crate) fn name_city(&mut self, id: EntityId) {
        if self.territory.names.contains_key(&id) {
            return;
        }
        let Some((owner, type_id)) = self.entity(id).map(|e| (e.owner, e.type_id)) else {
            return;
        };
        let name = self.next_city_name(owner, type_id);
        self.territory.names.insert(id, name);
    }

    /// Names every city without a name, in id order: a save from before city names.
    pub(crate) fn name_unnamed_cities(&mut self) {
        let unnamed: Vec<EntityId> = self
            .territory
            .cities
            .iter()
            .copied()
            .filter(|id| !self.territory.names.contains_key(id))
            .collect();
        for id in unnamed {
            self.name_city(id);
        }
    }

    /// Checks a `rename_city` order and renames the city at once: the executor is an own city
    /// (a site too), the text a name that keeps at least one character once cleaned.
    pub(crate) fn submit_rename(&mut self, command: Command) -> Result<OrderId, SimError> {
        if !self.rules.order_kinds.contains_key(&command.kind) {
            return Err(SimError::UnknownKind(command.kind));
        }
        self.check_orders_open(command.player)?;
        if command.subject.is_some()
            || command.target_entity.is_some()
            || command.target_pos.is_some()
        {
            return Err(SimError::WrongTargetShape);
        }
        let entity = self
            .entity(command.executor)
            .ok_or(SimError::UnknownExecutor(command.executor))?;
        if entity.owner != command.player {
            return Err(SimError::ForeignOwner(command.executor));
        }
        if !self.territory.cities.contains(&command.executor) {
            return Err(SimError::NotACity);
        }
        let name = clean_city_name(
            command.text.as_deref().unwrap_or_default(),
            self.city_name_max_chars(),
        );
        if name.is_empty() {
            return Err(SimError::EmptyName);
        }
        let next = self
            .next_order_id
            .checked_add(1)
            .ok_or(SimError::IdExhausted)?;
        let id = OrderId(self.next_order_id);
        self.next_order_id = next;
        self.territory.names.insert(command.executor, name);
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::clean_city_name;

    #[test]
    fn names_are_cleaned() {
        assert_eq!(clean_city_name("  Nova\tRoma \n", 24), "Nova Roma");
        assert_eq!(clean_city_name("Ro\u{7}ma", 24), "Roma");
        assert_eq!(clean_city_name("\u{1b}[31m", 24), "[31m");
        assert_eq!(clean_city_name("abcdefgh", 5), "abcde");
        assert_eq!(clean_city_name("abcd efgh", 5), "abcd");
        assert_eq!(clean_city_name("ab   cd", 5), "ab cd");
        assert_eq!(clean_city_name(" \t\r\n", 24), "");
        assert_eq!(clean_city_name("Ōsaka", 24), "Ōsaka");
        assert_eq!(clean_city_name("Ro\u{202e}ma\u{200b}", 24), "Roma");
    }
}
