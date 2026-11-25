//! City names as the person sees them (task 144): over the cities on the shown surface, on the
//! minimap's tooltip, and the rename field on the selection panel.

use rom_render::{HudCapital, HudCityName, HudRename};
use rom_sim::{BodyId, Command, EntityId, PlayerId, World};

use crate::chat::ChatKey;

/// The names over the cities on `body` that `me` sees: its own and its allies' always, the
/// others' while the city's ground is in sight. `capitals` are the capitals shown there.
pub fn shown_names(
    world: &World,
    me: PlayerId,
    body: BodyId,
    capitals: &[HudCapital],
) -> Vec<HudCityName> {
    let team = |id: PlayerId| world.players().find(|p| p.id == id).map(|p| p.team);
    world
        .city_names()
        .filter_map(|(id, name)| {
            let e = world.entity(id)?;
            let seen = e.owner == me
                || team(e.owner) == team(me)
                || world.is_visible_on_body(
                    me,
                    body,
                    e.pos.face,
                    e.pos.x.div_euclid(1000),
                    e.pos.y.div_euclid(1000),
                );
            (e.pos.body == body && seen).then(|| {
                let capital = capitals.iter().any(|c| c.city == id.0);
                HudCityName {
                    city: id.0,
                    owner: u8::try_from(e.owner.0).unwrap_or(u8::MAX),
                    face: e.pos.face,
                    x: e.pos.x,
                    y: e.pos.y,
                    name: name.to_owned(),
                    capital,
                    about: about(world, me, e.owner, capital),
                }
            })
        })
        .collect()
}

/// Whose a city is, in words: "Your capital", "City of Greeks (2)".
fn about(world: &World, me: PlayerId, owner: PlayerId, capital: bool) -> String {
    let kind = if capital { "capital" } else { "city" };
    if owner == me {
        return format!("Your {kind}");
    }
    let who = crate::ascension_view::player_name(world, me, owner);
    if capital {
        format!("Capital of {who}")
    } else {
        format!("City of {who}")
    }
}

/// The rename field (task 144): the city it renames, the name being typed and the caret. It is
/// shared between the window, which types into it, and the simulation thread, which opens it
/// and gives the order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RenameField {
    city: Option<EntityId>,
    draft: String,
    caret: usize,
    max: usize,
    done: Option<(EntityId, String)>,
}

impl RenameField {
    /// Opens the field on `city` with its present name, the caret at its end; `max` is the
    /// longest name the world keeps.
    pub fn open(&mut self, city: EntityId, name: &str, max: usize) {
        self.city = Some(city);
        self.max = max.max(1);
        self.draft = name.chars().take(self.max).collect();
        self.caret = self.draft.chars().count();
    }

    /// Closes the field without renaming.
    pub fn close(&mut self) {
        self.city = None;
        self.draft.clear();
        self.caret = 0;
    }

    /// Whether the field has the keyboard.
    pub fn focused(&self) -> bool {
        self.city.is_some()
    }

    /// The city being renamed.
    pub fn city(&self) -> Option<EntityId> {
        self.city
    }

    /// The name chosen with Enter since the last call, with its city.
    pub fn take_done(&mut self) -> Option<(EntityId, String)> {
        self.done.take()
    }

    /// What the selection panel draws while the field is open.
    pub fn view(&self) -> Option<HudRename> {
        self.focused().then(|| HudRename {
            draft: self.draft.clone(),
            caret: self.caret,
        })
    }

    fn insert(&mut self, text: &str) {
        for c in text.chars().filter(|c| !c.is_control()) {
            if self.draft.chars().count() >= self.max {
                break;
            }
            let at = self.byte(self.caret);
            self.draft.insert(at, c);
            self.caret += 1;
        }
    }

    fn byte(&self, chars: usize) -> usize {
        self.draft
            .char_indices()
            .nth(chars)
            .map_or(self.draft.len(), |(i, _)| i)
    }

    /// Takes a key; returns whether the field took it (every key while it is open).
    pub fn key(&mut self, key: ChatKey) -> bool {
        let Some(city) = self.city else {
            return false;
        };
        let len = self.draft.chars().count();
        match key {
            ChatKey::Char(c) => self.insert(&c.to_string()),
            ChatKey::Paste(text) => self.insert(&text),
            ChatKey::Enter => {
                let name = rom_sim::clean_city_name(&self.draft, self.max);
                if !name.is_empty() {
                    self.done = Some((city, name));
                }
                self.close();
            }
            ChatKey::Escape => self.close(),
            ChatKey::Backspace if self.caret > 0 => {
                let at = self.byte(self.caret - 1);
                self.draft.remove(at);
                self.caret -= 1;
            }
            ChatKey::Delete if self.caret < len => {
                let at = self.byte(self.caret);
                self.draft.remove(at);
            }
            ChatKey::Left => self.caret = self.caret.saturating_sub(1),
            ChatKey::Right => self.caret = (self.caret + 1).min(len),
            ChatKey::Home => self.caret = 0,
            ChatKey::End => self.caret = len,
            _ => {}
        }
        true
    }
}

/// The order that gives `city` the name `name`, in `me`'s name.
pub fn rename_order(me: PlayerId, city: EntityId, name: String) -> Command {
    Command {
        player: me,
        executor: city,
        kind: rom_sim::RENAME_CITY.into(),
        subject: None,
        target_entity: None,
        target_pos: None,
        queue: false,
        text: Some(name),
        facing: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn typed(field: &mut RenameField, text: &str) {
        for c in text.chars() {
            assert!(field.key(ChatKey::Char(c)));
        }
    }

    #[test]
    fn the_field_edits_the_name_and_gives_it_on_enter() {
        let mut field = RenameField::default();
        assert!(!field.key(ChatKey::Char('a')), "closed, it takes no key");
        field.open(EntityId(7), "Roma", 24);
        assert_eq!(field.view().unwrap().caret, 4);
        field.key(ChatKey::Home);
        typed(&mut field, "Nova ");
        field.key(ChatKey::End);
        field.key(ChatKey::Backspace);
        typed(&mut field, "a\u{7}");
        assert_eq!(field.view().unwrap().draft, "Nova Roma");
        assert!(field.key(ChatKey::Enter));
        assert!(!field.focused());
        assert_eq!(
            field.take_done(),
            Some((EntityId(7), "Nova Roma".to_owned()))
        );
        assert_eq!(field.take_done(), None);
    }

    #[test]
    fn escape_and_an_empty_name_rename_nothing() {
        let mut field = RenameField::default();
        field.open(EntityId(3), "Ostia", 24);
        typed(&mut field, "x");
        field.key(ChatKey::Escape);
        assert_eq!(field.take_done(), None);
        field.open(EntityId(3), "Ostia", 24);
        for _ in 0..5 {
            field.key(ChatKey::Backspace);
        }
        typed(&mut field, "   ");
        field.key(ChatKey::Enter);
        assert_eq!(field.take_done(), None);
        // The field holds no more than the world keeps.
        field.open(EntityId(3), "", 6);
        typed(&mut field, "Mediolanum");
        assert_eq!(field.view().unwrap().draft, "Mediol");
    }
}
