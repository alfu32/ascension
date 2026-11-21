//! The launcher's screens drawn in the game's look (task 112): home, the four forms and the
//! advanced launcher. Every piece comes from `crate::style`.

use std::path::PathBuf;
use std::rc::Rc;

use gpui::prelude::FluentBuilder;
use gpui::{
    AnyElement, ClipboardItem, Context, InteractiveElement, IntoElement, ParentElement,
    SharedString, StatefulInteractiveElement, Styled, Window, div, px, rgb,
};
use gpui_component::input::Input;
use gpui_component::select::Select;
use gpui_component::{Sizable, StyledExt};

use super::super::form::{
    CONTROLS, Form, GROUPS, HIDDEN, MODELS_GROUP, PRESETS, SAVES_GROUP, commands, preset,
};
use super::super::models;
use super::super::presets::Preset;
use super::super::scripts;
use super::super::simple::{self, Mode, Simple};
use super::{Launcher, SEGMENTS, Screen};
use crate::style::{self, Look};

/// What a choice does when picked: the launcher, the chosen key, the window.
type Pick = Rc<dyn Fn(&mut Launcher, &str, &mut Window, &mut Context<Launcher>)>;

/// How the advanced launcher edits one option.
enum Editor {
    /// A flag without a value: a switch.
    Switch,
    /// `on` or `off`: a switch.
    OnOff,
    /// A short list: a row of choices; `custom` keeps a text field for other values.
    Row { custom: bool },
    /// A text field, with a dropdown when the list is long.
    Text,
}

impl Launcher {
    /// A row of choices, `current` glowing; picking one calls `pick`.
    fn segmented(
        &self,
        id: &'static str,
        items: Vec<(String, String)>,
        current: &str,
        pick: Pick,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut row = style::choices();
        for (index, (key, text)) in items.into_iter().enumerate() {
            let pick = pick.clone();
            let chosen = key == current;
            row = row
                .child(style::choice((id, index), text, chosen).on_click(
                    cx.listener(move |this, _, window, cx| pick(this, &key, window, cx)),
                ));
        }
        row.into_any_element()
    }

    /// A count between − and +, kept within its bounds.
    fn stepper(&self, key: &'static str, cx: &mut Context<Self>) -> AnyElement {
        let (low, high) = Simple::bounds(key).unwrap_or((0, 0));
        let value = self.config.simple.get(key).trim().parse::<u32>().ok();
        let less = value.is_none_or(|v| v > low);
        let more = value.is_none_or(|v| v < high);
        style::stepper(
            style::small_button((key, 0usize), "−", less).when(less, |b| {
                b.on_click(cx.listener(move |this, _, _, cx| this.simple_step(key, -1, cx)))
            }),
            SharedString::from(value.map_or("?".to_owned(), |v| v.to_string())),
            style::small_button((key, 1usize), "+", more).when(more, |b| {
                b.on_click(cx.listener(move |this, _, _, cx| this.simple_step(key, 1, cx)))
            }),
        )
        .into_any_element()
    }

    fn text_input(&self, key: &'static str, width: Option<f32>) -> AnyElement {
        let input = Input::new(&self.simple_inputs[key]);
        match width {
            Some(w) => div().w(px(w)).child(input).into_any_element(),
            None => div().w_full().child(input).into_any_element(),
        }
    }

    /// Race cards: the default mix and each race with its count of nations.
    fn race_cards(&self, cx: &mut Context<Self>) -> AnyElement {
        let current = self.config.simple.get("race").to_owned();
        let at = self
            .catalog
            .races
            .iter()
            .position(|r| r.nations.iter().any(|n| n.key == current));
        let mut row =
            div().flex().gap(px(8.)).child(
                div().flex_1().min_w_0().child(
                    style::option_card(
                        ("race", 0usize),
                        "Any",
                        "The default mix",
                        current.is_empty(),
                    )
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.choose_nation(String::new(), window, cx)
                    })),
                ),
            );
        for (index, race) in self.catalog.races.iter().enumerate() {
            let first = race
                .nations
                .first()
                .map(|n| n.key.clone())
                .unwrap_or_default();
            let line = match race.nations.len() {
                1 => "1 nation".to_owned(),
                n => format!("{n} nations"),
            };
            row = row.child(
                div().flex_1().min_w_0().child(
                    style::option_card(
                        ("race", index + 1),
                        race.name.clone(),
                        line,
                        at == Some(index),
                    )
                    .on_click(cx.listener(move |this, _, window, cx| {
                        if at != Some(index) {
                            this.choose_nation(first.clone(), window, cx);
                        }
                    })),
                ),
            );
        }
        row.into_any_element()
    }

    /// The nation of the chosen race: a row of choices, or a dropdown when it has many.
    fn nation_chooser(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let current = self.config.simple.get("race").to_owned();
        let race = self
            .catalog
            .races
            .iter()
            .find(|r| r.nations.iter().any(|n| n.key == current))?;
        if race.nations.len() > SEGMENTS {
            let select = self.nation_selects.get(&race.key)?;
            return Some(
                div()
                    .w(px(300.))
                    .child(Select::new(select))
                    .into_any_element(),
            );
        }
        let items = race
            .nations
            .iter()
            .map(|n| (n.key.clone(), n.label.clone()))
            .collect();
        Some(self.segmented(
            "nation",
            items,
            &current,
            Rc::new(|this, key, window, cx| this.choose_nation(key.to_owned(), window, cx)),
            cx,
        ))
    }

    fn difficulty(&self, cx: &mut Context<Self>) -> AnyElement {
        let current = match self.config.simple.get("difficulty") {
            "" => crate::computer::DEFAULT_SPEC.1.to_owned(),
            chosen => chosen.to_owned(),
        };
        let items = self
            .catalog
            .difficulties
            .iter()
            .map(|c| (c.key.clone(), c.label.clone()))
            .collect();
        self.segmented(
            "difficulty",
            items,
            &current,
            Rc::new(|this, key, _, cx| this.simple_change("difficulty", key.to_owned(), cx)),
            cx,
        )
    }

    fn placement(&self, cx: &mut Context<Self>) -> AnyElement {
        let default = self.catalog.placement_default.clone();
        let current = match self.config.simple.get("placement") {
            "" => default.clone(),
            chosen => chosen.to_owned(),
        };
        let items = self.catalog.lists["start-placement"]
            .iter()
            .map(|c| (c.key.clone(), c.label.clone()))
            .collect();
        self.segmented(
            "placement",
            items,
            &current,
            Rc::new(move |this, key, _, cx| {
                // The rules' default is left off the command line.
                let value = if key == default { "" } else { key };
                this.simple_change("placement", value.to_owned(), cx)
            }),
            cx,
        )
    }

    fn drop_field(&self, cx: &mut Context<Self>) -> AnyElement {
        let current = self.config.simple.get("drop").to_owned();
        let items = simple::drop_choices()
            .iter()
            .map(|(key, short, _)| ((*key).to_owned(), (*short).to_owned()))
            .collect();
        let sentence = simple::drop_choices()
            .iter()
            .find(|(key, _, _)| *key == current)
            .map(|(_, _, long)| *long)
            .unwrap_or("");
        style::field(
            "When a player drops",
            Some(sentence.into()),
            self.segmented(
                "drop",
                items,
                &current,
                Rc::new(|this, key, _, cx| this.simple_change("drop", key.to_owned(), cx)),
                cx,
            ),
        )
        .into_any_element()
    }

    /// The map styles as picture cards, three to a row.
    fn map_cards(&self, cx: &mut Context<Self>) -> AnyElement {
        let current = self.config.simple.get("style").to_owned();
        let mut grid = div().flex().flex_col().gap(px(12.));
        for (row_index, row) in self.catalog.styles.chunks(3).enumerate() {
            let mut line = div().flex().gap(px(12.));
            for (index, card) in row.iter().enumerate() {
                let key = card.key.clone();
                line = line.child(
                    div().flex_1().min_w_0().child(
                        style::card(
                            ("map-style", row_index * 3 + index),
                            Some(&format!("maps/{}", card.key)),
                            None,
                            card.name.clone(),
                            card.line.clone(),
                            card.key == current,
                            70.,
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.simple_change("style", key.clone(), cx)
                        })),
                    ),
                );
            }
            for _ in row.len()..3 {
                line = line.child(div().flex_1());
            }
            grid = grid.child(line);
        }
        grid.into_any_element()
    }

    /// The forms' setup: starting age and a script (bundled, or a file of the person's).
    fn setup_fields(&self, cx: &mut Context<Self>) -> AnyElement {
        let chosen = self.config.simple.get("script").to_owned();
        let about = if chosen.is_empty() {
            Ok(None)
        } else {
            scripts::describe(&self.path, &chosen).map(Some)
        };
        let note = match &about {
            Ok(Some(about)) => style::hint(about.line.clone()),
            Ok(None) => style::hint("No script: the match starts as the rules set it."),
            Err(error) => style::status(error.clone(), true),
        };
        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(
                div()
                    .flex()
                    .gap(px(16.))
                    .items_start()
                    .child(style::field(
                        "Starting age",
                        None,
                        div()
                            .w(px(190.))
                            .children(self.age_select.as_ref().map(Select::new)),
                    ))
                    .child(
                        div().flex_1().min_w_0().child(style::field(
                            "Start script",
                            None,
                            div()
                                .flex()
                                .gap(px(8.))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .children(self.script_select.as_ref().map(Select::new)),
                                )
                                .child(
                                    style::button("script-file", "Open file …", Look::Plain, true)
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.pick_script(window, cx)
                                        })),
                                ),
                        )),
                    ),
            )
            .child(note)
            .into_any_element()
    }

    /// The map section shared by Play, Host and Run a server.
    fn map_section(&self, with_speed: bool, cx: &mut Context<Self>) -> AnyElement {
        div()
            .flex()
            .flex_col()
            .gap(px(14.))
            .child(style::heading("The map"))
            .child(self.map_cards(cx))
            .child(
                div()
                    .flex()
                    .gap(px(16.))
                    .child(style::field("Placement", None, self.placement(cx)))
                    .child(style::field(
                        "Seed",
                        None,
                        self.text_input("seed", Some(if with_speed { 112. } else { 150. })),
                    ))
                    .when(with_speed, |row| {
                        row.child(style::field(
                            "Speed %",
                            None,
                            self.text_input("speed", Some(72.)),
                        ))
                    }),
            )
            .child(style::heading("Setup"))
            .child(self.setup_fields(cx))
            .into_any_element()
    }

    /// The side section of a form: who plays.
    fn side_section(&self, mode: Mode, cx: &mut Context<Self>) -> AnyElement {
        let mut column = div().flex().flex_col().gap(px(14.));
        let difficulty = |this: &Self, cx: &mut Context<Self>| {
            style::field(
                "Difficulty",
                Some("How hard the computer players play.".into()),
                this.difficulty(cx),
            )
        };
        match mode {
            Mode::Play | Mode::Host => {
                column = column
                    .child(style::heading("Your side"))
                    .child(self.race_cards(cx));
                if let Some(nation) = self.nation_chooser(cx) {
                    column = column.child(style::field("Nation", None, nation));
                }
                column = column
                    .child(style::heading("Opponents"))
                    .child(style::field(
                        "Computer opponents",
                        Some("With none, you are alone on the map.".into()),
                        self.stepper("opponents", cx),
                    ))
                    .child(difficulty(self, cx));
                if mode == Mode::Host {
                    column = column
                        .child(style::heading("Network"))
                        .child(
                            div()
                                .flex()
                                .gap(px(20.))
                                .child(style::field(
                                    "Remote players",
                                    Some("People on other computers.".into()),
                                    self.stepper("remote", cx),
                                ))
                                .child(style::field(
                                    "Port",
                                    Some("Usually 7400.".into()),
                                    self.text_input("port", Some(110.)),
                                )),
                        )
                        .child(self.drop_field(cx));
                }
            }
            Mode::Serve => {
                column = column
                    .child(style::heading("The server"))
                    .child(
                        div()
                            .flex()
                            .gap(px(20.))
                            .child(style::field(
                                "Listen address",
                                Some("0.0.0.0 lets the network in.".into()),
                                self.text_input("bind", Some(220.)),
                            ))
                            .child(style::field(
                                "Port",
                                None,
                                self.text_input("port", Some(110.)),
                            )),
                    )
                    .child(style::heading("Seats"))
                    .child(
                        div()
                            .flex()
                            .gap(px(28.))
                            .child(style::field(
                                "Human seats",
                                Some("Join with keys.".into()),
                                self.stepper("humans", cx),
                            ))
                            .child(style::field(
                                "Computer seats",
                                Some("At most 8 in all.".into()),
                                self.stepper("computers", cx),
                            )),
                    )
                    .child(difficulty(self, cx))
                    .child(self.drop_field(cx))
                    .child(style::field(
                        "Autosave",
                        Some("Game minutes; 0 turns it off.".into()),
                        self.text_input("autosave", Some(110.)),
                    ));
            }
            Mode::Join => {
                column = column
                    .child(style::heading("The server"))
                    .child(style::field(
                        "Server address",
                        Some("Starts with http:// or https://, as the host gave it.".into()),
                        self.text_input("url", None),
                    ))
                    .child(style::field(
                        "Key",
                        Some("The key the host gave you.".into()),
                        self.text_input("key", None),
                    ))
                    .child(style::field(
                        "Your name",
                        Some("What the other players see.".into()),
                        self.text_input("name", Some(320.)),
                    ));
            }
        }
        column.into_any_element()
    }

    /// The last message and the started processes.
    fn status_lines(&self) -> AnyElement {
        div()
            .flex()
            .flex_col()
            .gap_1()
            .children(
                self.message
                    .iter()
                    .map(|message| style::status(message.clone(), true)),
            )
            .children(
                self.processes
                    .iter()
                    .map(|p| style::hint(format!("Started PID {} · {}", p.child.id(), p.status))),
            )
            .into_any_element()
    }

    /// A screen over the backdrop, its content centred and scrolling when the window is small.
    fn screen(id: &'static str, content: impl IntoElement) -> AnyElement {
        style::ground()
            .child(style::backdrop())
            .child(
                div()
                    .id(id)
                    .absolute()
                    .size_full()
                    .overflow_y_scroll()
                    .child(
                        div()
                            .min_h_full()
                            .flex()
                            .flex_col()
                            .items_center()
                            .justify_center()
                            .p(px(20.))
                            .child(content),
                    ),
            )
            .child(
                div()
                    .absolute()
                    .bottom(px(8.))
                    .right(px(14.))
                    .child(style::version()),
            )
            .into_any_element()
    }

    /// The preset tiles: the game's and the person's own, four to a row; the chosen one glows.
    fn preset_tiles(&self, cx: &mut Context<Self>) -> AnyElement {
        let tiles: Vec<(Preset, bool)> = self
            .bundled
            .iter()
            .map(|p| (p.clone(), false))
            .chain(self.config.play_presets.iter().map(|p| (p.clone(), true)))
            .collect();
        let mut grid = div().flex().flex_col().gap(px(14.));
        for (row_index, row) in tiles.chunks(4).enumerate() {
            let mut line = div().flex().gap(px(14.));
            for (index, (preset, own)) in row.iter().enumerate() {
                let id = row_index * 4 + index;
                let name = preset.name.clone();
                let picture = if preset.picture.is_empty() {
                    "tiles/custom"
                } else {
                    preset.picture.as_str()
                };
                let mut tile = div().relative().flex_1().min_w_0().child(
                    style::card(
                        ("preset-tile", id),
                        Some(picture),
                        Some(if preset.icon.is_empty() {
                            "flag"
                        } else {
                            preset.icon.as_str()
                        }),
                        preset.name.clone(),
                        preset.line.clone(),
                        preset.name == self.config.chosen,
                        104.,
                    )
                    .on_click(cx.listener(
                        move |this, event: &gpui::ClickEvent, _, cx| {
                            this.choose_preset(name.clone(), cx);
                            if event.click_count() > 1 {
                                this.play_chosen(cx);
                            }
                        },
                    )),
                );
                if *own {
                    let name = preset.name.clone();
                    tile = tile.child(div().absolute().top(px(6.)).right(px(6.)).child(
                        style::small_button(("remove-tile", id), "×", true).on_click(
                            cx.listener(move |this, _, _, cx| this.remove_play_preset(&name, cx)),
                        ),
                    ));
                }
                line = line.child(tile);
            }
            for _ in row.len()..4 {
                line = line.child(div().flex_1());
            }
            grid = grid.child(line);
        }
        grid.into_any_element()
    }

    pub(super) fn render_home(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let chosen = self.find_preset(&self.config.chosen);
        let hero = style::hero(250.).child(
            div()
                .absolute()
                .size_full()
                .flex()
                .items_center()
                .gap(px(36.))
                .px(px(36.))
                .child(style::emblem(320.))
                .child(div().flex_1())
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(10.))
                        .relative()
                        .p(px(18.))
                        .child(style::frame("panel"))
                        .child(
                            style::play_button("play", "Play", chosen.is_some())
                                .on_click(cx.listener(|this, _, _, cx| this.play_chosen(cx))),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .w(px(260.))
                                .child(style::heading(
                                    chosen.as_ref().map_or(String::new(), |p| p.name.clone()),
                                ))
                                .child(style::hint(
                                    chosen.as_ref().map_or(String::new(), |p| p.line.clone()),
                                )),
                        )
                        .child(
                            div()
                                .flex()
                                .gap(px(10.))
                                .child(
                                    style::button("customise", "Change …", Look::Framed, true)
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.customise(window, cx)
                                        })),
                                )
                                .child(
                                    style::button("home-continue", "Continue", Look::Framed, true)
                                        .on_click(
                                            cx.listener(|this, _, _, cx| this.continue_latest(cx)),
                                        ),
                                ),
                        ),
                ),
        );
        let mut others = div()
            .flex()
            .flex_wrap()
            .items_center()
            .justify_center()
            .gap(px(10.));
        for mode in [Mode::Host, Mode::Join, Mode::Serve] {
            others = others.child(
                style::button(("home", mode as usize), mode.title(), Look::Plain, true).on_click(
                    cx.listener(move |this, _, window, cx| this.go(Screen::Form(mode), window, cx)),
                ),
            );
        }
        others = others
            .child(div().w(px(16.)))
            .child(
                style::button("home-advanced", "Advanced …", Look::Plain, true).on_click(
                    cx.listener(|this, _, window, cx| this.go(Screen::Advanced, window, cx)),
                ),
            )
            .child(
                style::button("home-quit", "Quit", Look::Plain, true)
                    .on_click(cx.listener(|_, _, _, cx| cx.quit())),
            );
        let tiles = style::panel()
            .w_full()
            .gap(px(12.))
            .child(
                div()
                    .relative()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .child(style::heading("Choose a match"))
                    .child(style::hint(
                        "Pick a tile, then Play. Double-click plays at once.",
                    ))
                    .child(div().flex_1())
                    .child(
                        style::button("import-presets", "Import …", Look::Plain, true)
                            .on_click(cx.listener(|this, _, _, cx| this.import_presets(cx))),
                    )
                    .child(
                        style::button("export-presets", "Export …", Look::Plain, true)
                            .on_click(cx.listener(|this, _, _, cx| this.export_presets(cx))),
                    ),
            )
            .child(div().relative().child(self.preset_tiles(cx)));
        let content = div()
            .w_full()
            .max_w(px(1220.))
            .flex()
            .flex_col()
            .items_center()
            .gap(px(18.))
            .child(hero)
            .child(tiles)
            .child(others)
            .child(div().w_full().child(self.status_lines()));
        Self::screen("home", content)
    }

    pub(super) fn render_form(
        &mut self,
        mode: Mode,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let check = self.simple_check(mode);
        let token = mode == Mode::Join
            && self
                .config
                .tokens
                .contains_key(self.config.simple.get("url").trim());
        let (note, wrong) = match &check {
            Err(error) => (error.clone(), true),
            Ok(_) => ("Ready".to_owned(), false),
        };
        let body = if mode == Mode::Join {
            div()
                .w(px(620.))
                .child(self.side_section(mode, cx))
                .into_any_element()
        } else {
            div()
                .flex()
                .gap(px(36.))
                .child(div().flex_1().min_w_0().child(self.side_section(mode, cx)))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .child(self.map_section(mode != Mode::Play, cx)),
                )
                .into_any_element()
        };
        let footer = div()
            .flex()
            .items_center()
            .gap(px(12.))
            .pt(px(16.))
            .border_t_1()
            .border_color(rgb(style::LINE))
            .child(
                style::button("form-back", "Back", Look::Plain, true)
                    .on_click(cx.listener(|this, _, window, cx| this.go(Screen::Home, window, cx))),
            )
            .child(
                style::button("form-advanced", "Open in Advanced …", Look::Plain, true).on_click(
                    cx.listener(move |this, _, window, cx| this.open_in_advanced(mode, window, cx)),
                ),
            )
            .when(mode == Mode::Play, |footer| {
                footer
                    .child(
                        div()
                            .w(px(160.))
                            .child(Input::new(&self.play_preset_name).small()),
                    )
                    .child(
                        style::button("save-play-preset", "Save as preset", Look::Plain, true)
                            .on_click(cx.listener(|this, _, _, cx| this.save_play_preset(cx))),
                    )
            })
            .child(div().flex_1().min_w_0().child(style::status(note, wrong)))
            .when(token, |footer| {
                footer.child(
                    style::button("form-rejoin", "Rejoin", Look::Framed, true)
                        .on_click(cx.listener(|this, _, _, cx| this.rejoin(cx))),
                )
            })
            .child(
                style::button("form-start", mode.action(), Look::Gold, check.is_ok())
                    .when(check.is_ok(), |b| {
                        b.on_click(cx.listener(move |this, _, _, cx| this.start(mode, cx)))
                    }),
            );
        let panel = style::main_panel()
            .w_full()
            .max_w(px(if mode == Mode::Join { 760. } else { 1180. }))
            .gap(px(18.))
            .child(
                div()
                    .relative()
                    .flex()
                    .items_end()
                    .gap(px(16.))
                    .child(style::title(mode.title()))
                    .child(style::hint(mode.blurb()).pb(px(3.))),
            )
            .child(div().relative().child(body))
            .child(div().relative().child(footer))
            .child(div().relative().child(self.status_lines()));
        Self::screen("form", panel)
    }
}

impl Launcher {
    /// How option `flag` is edited in the advanced launcher.
    fn editor(&self, flag: &str, toggle: bool) -> Editor {
        if toggle {
            return Editor::Switch;
        }
        match self.catalog.lists.get(flag) {
            Some(list) if list.iter().map(|c| c.key.as_str()).eq(["on", "off"]) => Editor::OnOff,
            Some(list) if list.len() <= SEGMENTS => Editor::Row {
                custom: list.iter().any(|c| c.key.contains('x')),
            },
            _ => Editor::Text,
        }
    }

    /// Sets option `index` from a choice, keeping its text field in step.
    fn set_option(
        &mut self,
        index: usize,
        value: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let flag = CONTROLS[index].flag;
        self.change(flag, value.to_owned(), cx);
        if let Some(input) = self.fields[index].input.clone() {
            input.update(cx, |input, cx| {
                input.set_value(value.to_owned(), window, cx)
            });
        }
    }

    /// One option of the advanced launcher: its flag, editor and help.
    fn row(&self, index: usize, cx: &mut Context<Self>) -> AnyElement {
        let control = &CONTROLS[index];
        let flag = control.flag;
        let field = &self.fields[index];
        let value = self.config.form.get(flag).to_owned();
        let default = if control.default.is_empty() {
            "automatic / off"
        } else {
            control.default
        };
        let editor: AnyElement = if flag == "style" {
            self.advanced_map_cards(index, &value, cx)
        } else if flag == "script" {
            self.advanced_script(index, &value, cx)
        } else {
            match self.editor(flag, control.toggle) {
                Editor::Switch => {
                    let on = value == "true";
                    style::toggle(("switch", index), on, if on { "On" } else { "Off" })
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.change(flag, (!on).to_string(), cx)
                        }))
                        .into_any_element()
                }
                Editor::OnOff => {
                    let on = value != "off";
                    style::toggle(("switch", index), on, if on { "On" } else { "Off" })
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.set_option(index, if on { "off" } else { "on" }, window, cx)
                        }))
                        .into_any_element()
                }
                Editor::Row { custom } => {
                    let mut items: Vec<(String, String)> = Vec::new();
                    if control.default.is_empty() {
                        items.push((String::new(), "Default".into()));
                    }
                    items.extend(self.catalog.lists[flag].iter().map(|c| {
                        let text = c.label.split(" · ").next().unwrap_or(&c.label);
                        (c.key.clone(), text.to_owned())
                    }));
                    let current = if value.is_empty() { "" } else { value.as_str() };
                    let row = self.segmented(
                        "option",
                        items,
                        current,
                        Rc::new(move |this, key, window, cx| {
                            this.set_option(index, key, window, cx)
                        }),
                        cx,
                    );
                    if custom {
                        div()
                            .flex()
                            .flex_wrap()
                            .gap(px(10.))
                            .items_center()
                            .child(row)
                            .children(
                                field
                                    .input
                                    .as_ref()
                                    .map(|i| div().w(px(150.)).child(Input::new(i).small())),
                            )
                            .into_any_element()
                    } else {
                        row
                    }
                }
                Editor::Text => {
                    let mut line = div().flex().items_center().gap_2().w_full().child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .children(field.input.as_ref().map(|i| Input::new(i).small())),
                    );
                    if let Some(select) = &field.select {
                        line = line.child(
                            div()
                                .w(px(280.))
                                .child(Select::new(select).small().placeholder("Choose…")),
                        );
                    }
                    if flag == "seed" {
                        line = line.child(
                            style::button("random", "Random", Look::Plain, true).on_click(
                                cx.listener(|this, _, window, cx| {
                                    let seed = Self::random_seed();
                                    let mut form = this.config.form.clone();
                                    form.set("seed", seed.to_string());
                                    this.replace(form, window, cx);
                                }),
                            ),
                        );
                    }
                    if flag == "ai" {
                        line = line.child(
                            style::button("choose-computers", "Per seat …", Look::Plain, true)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.computers_open = !this.computers_open;
                                    cx.notify();
                                })),
                        );
                    }
                    if flag == "factions" {
                        line = line.child(
                            style::button("choose-factions", "Choose …", Look::Plain, true)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.factions_open = !this.factions_open;
                                    cx.notify();
                                })),
                        );
                    }
                    line.into_any_element()
                }
            }
        };
        let mut row = div()
            .flex()
            .flex_col()
            .gap(px(6.))
            .py(px(10.))
            .border_b_1()
            .border_color(rgb(style::LINE))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(14.))
                    .child(
                        div()
                            .w(px(190.))
                            .flex_shrink_0()
                            .text_size(px(13.))
                            .font_family(style::MONO)
                            .text_color(rgb(style::GOLD))
                            .child(format!("--{flag}")),
                    )
                    .child(div().flex_1().min_w_0().child(editor)),
            )
            .child(
                div()
                    .pl(px(204.))
                    .child(style::hint(format!("{}  Default: {default}", control.help))),
            );
        if flag == "factions" && self.factions_open {
            let chosen: Vec<String> = self
                .config
                .form
                .get("factions")
                .split(',')
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect();
            let mut list = style::choices().pl(px(204.));
            for (i, choice) in self.catalog.factions.iter().enumerate() {
                let key = choice.key.clone();
                let on = chosen.contains(&key);
                let name = choice.label.split(" · ").next().unwrap_or(&choice.label);
                list = list.child(style::choice(("faction", i), name.to_owned(), on).on_click(
                    cx.listener(move |this, _, window, cx| {
                        let mut keys = this
                            .config
                            .form
                            .get("factions")
                            .split(',')
                            .filter(|s| !s.is_empty() && *s != key)
                            .map(str::to_owned)
                            .collect::<Vec<_>>();
                        if !on {
                            keys.push(key.clone());
                        }
                        let mut form = this.config.form.clone();
                        form.set("factions", keys.join(","));
                        this.replace(form, window, cx);
                    }),
                ));
            }
            row = row.child(list);
        }
        if flag == "ai" && self.computers_open {
            row = row.child(self.computer_slots(cx));
        }
        if flag == "look-at" {
            row = row.child(
                div()
                    .pl(px(204.))
                    .flex()
                    .gap_2()
                    .children(
                        self.look_selects
                            .iter()
                            .map(|s| div().flex_1().min_w_0().child(Select::new(s).small())),
                    )
                    .child(
                        div()
                            .w(px(155.))
                            .children(self.distance.as_ref().map(|d| Input::new(d).small())),
                    ),
            );
        }
        row.into_any_element()
    }

    /// The style option as the map styles' picture cards, in one row.
    fn advanced_map_cards(&self, index: usize, value: &str, cx: &mut Context<Self>) -> AnyElement {
        let mut row = div().flex().gap(px(10.));
        for (i, card) in self.catalog.styles.iter().enumerate() {
            let key = card.key.clone();
            let chosen = card.key == value;
            row = row.child(
                div().flex_1().min_w_0().child(
                    style::card(
                        ("advanced-style", i),
                        Some(&format!("maps/{}", card.key)),
                        None,
                        card.name.clone(),
                        card.line.clone(),
                        chosen,
                        58.,
                    )
                    .on_click(cx.listener(move |this, _, window, cx| {
                        let next = if chosen { String::new() } else { key.clone() };
                        this.set_option(index, &next, window, cx)
                    })),
                ),
            );
        }
        row.into_any_element()
    }

    /// The script option: a bundled script or a file, with what it does.
    fn advanced_script(&self, index: usize, value: &str, cx: &mut Context<Self>) -> AnyElement {
        let field = &self.fields[index];
        let note = if value.is_empty() {
            style::hint("None: the match starts as the rules set it.")
        } else {
            match scripts::describe(&self.path, value) {
                Ok(about) => style::hint(format!("{} · {}", about.title, about.line)),
                Err(error) => style::status(error, true),
            }
        };
        div()
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .w(px(300.))
                            .children(field.select.as_ref().map(|s| Select::new(s).small())),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .children(field.input.as_ref().map(|i| Input::new(i).small())),
                    )
                    .child(
                        style::button("advanced-script-file", "Open file …", Look::Plain, true)
                            .on_click(cx.listener(move |_, _, window, cx| {
                                let chosen = cx.prompt_for_paths(gpui::PathPromptOptions {
                                    files: true,
                                    directories: false,
                                    multiple: false,
                                    prompt: Some("Use this script".into()),
                                });
                                cx.spawn_in(window, async move |this, cx| {
                                    let picked = chosen.await;
                                    let _ = this.update_in(cx, |this, window, cx| {
                                        if let Ok(Ok(Some(paths))) = picked
                                            && let Some(path) = paths.first()
                                        {
                                            let text = path.to_string_lossy().into_owned();
                                            this.set_option(index, &text, window, cx);
                                        }
                                    });
                                })
                                .detach();
                            })),
                    ),
            )
            .child(note)
            .into_any_element()
    }

    /// One line per computer seat: its profile and its difficulty as rows of choices.
    fn computer_slots(&self, cx: &mut Context<Self>) -> AnyElement {
        let form = &self.config.form;
        let mut list = div().flex().flex_col().gap(px(8.)).pl(px(204.));
        for slot in 0..form.computer_slots() {
            let (profile, difficulty) = form.computer(slot);
            let mut line =
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(6.))
                    .child(div().w(px(80.)).child(style::label(format!(
                        "Player {}",
                        slot + form.first_computer()
                    ))));
            for (index, choice) in self.catalog.profiles.iter().enumerate() {
                let (key, difficulty) = (choice.key.clone(), difficulty.clone());
                line = line.child(
                    style::choice(
                        ("ai-profile", slot * 64 + index),
                        choice.label.clone(),
                        choice.key == profile,
                    )
                    .on_click(cx.listener(move |this, _, window, cx| {
                        let mut form = this.config.form.clone();
                        form.set_computer(slot, &key, &difficulty);
                        this.replace(form, window, cx);
                    })),
                );
            }
            line = line.child(div().w(px(10.)));
            for (index, choice) in self.catalog.difficulties.iter().enumerate() {
                let (key, profile) = (choice.key.clone(), profile.clone());
                line = line.child(
                    style::choice(
                        ("ai-difficulty", slot * 64 + index),
                        choice.label.clone(),
                        choice.key == difficulty,
                    )
                    .on_click(cx.listener(move |this, _, window, cx| {
                        let mut form = this.config.form.clone();
                        form.set_computer(slot, &profile, &key);
                        this.replace(form, window, cx);
                    })),
                );
            }
            list = list.child(line);
        }
        list.into_any_element()
    }

    /// The Saved games group: every save, newest first, with a Load button each.
    fn saves_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        let mut panel = div().flex().flex_col().gap(px(10.)).py(px(10.)).child(
            div()
                .flex()
                .items_center()
                .gap(px(14.))
                .child(
                    style::button("saves-refresh", "Refresh", Look::Plain, true).on_click(
                        cx.listener(|this, _, _, cx| {
                            this.refresh_saves();
                            cx.notify();
                        }),
                    ),
                )
                .child(style::hint(self.saves_note.clone().unwrap_or_default())),
        );
        for (index, entry) in self.saves.iter().enumerate() {
            let path = entry.path.clone();
            let refused = entry.refusal.is_some();
            panel =
                panel.child(
                    div()
                        .relative()
                        .flex()
                        .items_center()
                        .gap(px(14.))
                        .px(px(14.))
                        .py(px(10.))
                        .child(style::frame("cell"))
                        .child(
                            div()
                                .relative()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .gap(px(2.))
                                .child(
                                    div()
                                        .font_bold()
                                        .text_color(rgb(if refused {
                                            style::MUTED
                                        } else {
                                            style::GOLD
                                        }))
                                        .child(format!("{} ({})", entry.name, entry.kind.label())),
                                )
                                .child(style::hint(entry.summary()))
                                .children(entry.refusal.as_ref().map(|why| {
                                    div()
                                        .text_size(px(12.))
                                        .text_color(rgb(style::WARN))
                                        .child(why.clone())
                                })),
                        )
                        .child({
                            let path = path.clone();
                            style::button(("save-report", index), "Report", Look::Plain, !refused)
                                .when(!refused, |b| {
                                    b.on_click(cx.listener(move |this, _, window, cx| {
                                        this.show_report(&path, window, cx)
                                    }))
                                })
                        })
                        .child(
                            style::button(("load-save", index), "Load", Look::Framed, !refused)
                                .when(!refused, |b| {
                                    b.on_click(
                                        cx.listener(move |this, _, _, cx| {
                                            this.load_save(&path, cx)
                                        }),
                                    )
                                }),
                        ),
                );
        }
        panel.into_any_element()
    }

    /// The Models group: the export folder, the two model tools and their status line.
    fn models_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        let busy = self.model_busy;
        let folder_row = div()
            .flex()
            .items_center()
            .gap(px(10.))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(Input::new(&self.model_folder).small()),
            )
            .child(
                style::button("model-folder", "Choose folder …", Look::Plain, true).on_click(
                    cx.listener(|_, _, window, cx| {
                        let chosen = cx.prompt_for_paths(gpui::PathPromptOptions {
                            files: false,
                            directories: true,
                            multiple: false,
                            prompt: Some("Export here".into()),
                        });
                        cx.spawn_in(window, async move |this, cx| {
                            let picked = chosen.await;
                            let _ = this.update_in(cx, |this, window, cx| {
                            match picked {
                                Ok(Ok(Some(paths))) => {
                                    if let Some(path) = paths.first() {
                                        let text = path.display().to_string();
                                        this.model_folder.update(cx, |input, cx| {
                                            input.set_value(text, window, cx)
                                        });
                                    }
                                }
                                Ok(Ok(None)) => {}
                                Ok(Err(e)) => {
                                    this.model_status = format!(
                                        "Folder picker unavailable ({e}); type the folder instead."
                                    )
                                }
                                Err(_) => {}
                            }
                            cx.notify();
                        });
                        })
                        .detach();
                    }),
                ),
            );
        let unlock = self.model_unlock;
        let actions = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(14.))
            .child(
                style::button("export-models", "Export all models", Look::Gold, !busy).when(
                    !busy,
                    |b| {
                        b.on_click(cx.listener(|this, _, _, cx| {
                            let folder = PathBuf::from(this.model_folder.read(cx).value().trim());
                            this.model_job(
                                "Exporting every model…",
                                move || models::export_into(&folder),
                                cx,
                            );
                        }))
                    },
                ),
            )
            .child(
                style::button("import-model", "Import model …", Look::Framed, !busy).when(
                    !busy,
                    |b| {
                        b.on_click(cx.listener(|this, _, _, cx| {
                            let chosen = cx.prompt_for_paths(gpui::PathPromptOptions {
                                files: true,
                                directories: false,
                                multiple: false,
                                prompt: Some("Import".into()),
                            });
                            let unlock = this.model_unlock;
                            cx.spawn(async move |this, cx| {
                                let picked = chosen.await;
                                let _ = this.update(cx, |this, cx| match picked {
                                    Ok(Ok(Some(paths))) => {
                                        if let Some(path) = paths.first().cloned() {
                                            this.model_job(
                                                "Importing…",
                                                move || models::import_file(&path, unlock),
                                                cx,
                                            );
                                        }
                                    }
                                    Ok(Ok(None)) => {}
                                    Ok(Err(e)) => {
                                        this.model_status = format!(
                                            "File picker unavailable ({e}); use rom model import <file>."
                                        );
                                        cx.notify();
                                    }
                                    Err(_) => {}
                                });
                            })
                            .detach();
                        }))
                    },
                ),
            )
            .child(
                style::toggle(
                    "model-unlock",
                    unlock,
                    "Allow changing approved models (--unlock)",
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.model_unlock = !unlock;
                    cx.notify();
                })),
            );
        div()
            .flex()
            .flex_col()
            .gap(px(14.))
            .py(px(12.))
            .child(style::hint(
                "Edit building and unit models in Meshcraft · docs/models-in-meshcraft.md",
            ))
            .child(style::field("Export folder", None, folder_row))
            .child(actions)
            .child(
                div()
                    .id("model-status")
                    .text_size(px(13.))
                    .max_h(px(160.))
                    .overflow_y_scroll()
                    .text_color(rgb(if busy { style::WARN } else { style::CYAN }))
                    .child(self.model_status.clone()),
            )
            .into_any_element()
    }

    /// The advanced launcher's workflows as a carousel of picture cards.
    fn carousel(&self, visible: usize, cx: &mut Context<Self>) -> AnyElement {
        let count = PRESETS.len();
        let visible = visible.clamp(1, count);
        let start = self.carousel.min(count - visible);
        let mut row = div().flex().items_start().gap(px(12.)).child(
            style::small_button("carousel-back", "‹", start > 0)
                .mt(px(22.))
                .when(start > 0, |b| {
                    b.on_click(cx.listener(|this, _, _, cx| {
                        this.carousel = this.carousel.saturating_sub(1);
                        cx.notify();
                    }))
                }),
        );
        for (index, (name, line, picture)) in PRESETS.iter().enumerate().skip(start).take(visible) {
            row = row.child(
                div().flex_1().min_w_0().child(
                    style::card(
                        ("workflow", index),
                        Some(picture),
                        None,
                        *name,
                        *line,
                        false,
                        74.,
                    )
                    .on_click(cx.listener(move |this, _, window, cx| {
                        let form = preset(
                            index,
                            this.catalog.first_model(),
                            &this.catalog.planet_style,
                        );
                        this.group = match index {
                            3 | 4 => 3,
                            5 => 4,
                            _ => 0,
                        };
                        this.replace(form, window, cx);
                    })),
                ),
            );
        }
        let more = start + visible < count;
        row.child(
            style::small_button("carousel-next", "›", more)
                .mt(px(22.))
                .when(more, |b| {
                    b.on_click(cx.listener(move |this, _, _, cx| {
                        this.carousel = (start + 1).min(count - visible);
                        cx.notify();
                    }))
                }),
        )
        .into_any_element()
    }

    /// The "…" menu: the person's saved forms, saving the current one, and reset.
    fn advanced_menu(&self, cx: &mut Context<Self>) -> AnyElement {
        let mut saved = style::choices();
        for (index, name) in self.config.presets.keys().enumerate() {
            let name = name.clone();
            let remove_name = name.clone();
            saved =
                saved
                    .child(
                        style::choice(("custom", index), name.clone(), false).on_click(
                            cx.listener(move |this, _, window, cx| {
                                let form = this.config.presets[&name].clone();
                                this.menu_open = false;
                                this.replace(form, window, cx);
                            }),
                        ),
                    )
                    .child(style::small_button(("remove", index), "×", true).on_click(
                        cx.listener(move |this, _, _, cx| {
                            this.config.presets.remove(&remove_name);
                            this.save();
                            cx.notify();
                        }),
                    ));
        }
        style::panel()
            .w(px(440.))
            .gap(px(12.))
            .child(div().relative().child(style::heading("My forms")))
            .child(
                div()
                    .relative()
                    .flex()
                    .gap(px(8.))
                    .child(div().flex_1().child(Input::new(&self.preset_name).small()))
                    .child(
                        style::button("save-preset", "Save", Look::Framed, true).on_click(
                            cx.listener(|this, _, _, cx| {
                                let name = this.preset_name.read(cx).value().trim().to_owned();
                                if name.is_empty() {
                                    this.message = Some("Enter a preset name first.".into());
                                } else if this.config.presets.len() >= 12
                                    && !this.config.presets.contains_key(&name)
                                {
                                    this.message = Some(
                                        "Twelve custom presets saved; remove one first.".into(),
                                    );
                                } else {
                                    this.config.presets.insert(name, this.config.form.clone());
                                    this.message = None;
                                    this.save();
                                }
                                cx.notify();
                            }),
                        ),
                    ),
            )
            .when(!self.config.presets.is_empty(), |panel| {
                panel.child(div().relative().child(saved))
            })
            .child(div().relative().flex().gap(px(8.)).child(
                style::button("reset", "Reset to defaults", Look::Plain, true).on_click(
                    cx.listener(|this, _, window, cx| {
                        this.menu_open = false;
                        this.replace(Form::default(), window, cx)
                    }),
                ),
            ))
            .into_any_element()
    }

    /// The advanced launcher: a carousel of workflows, the option groups as tabs, every option.
    pub(super) fn render_advanced(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let launched = self.config.form.clone().universe_match();
        let error = launched.validate().err();
        let width = f32::from(window.viewport_size().width);
        let visible = ((width - 120.) / 230.).floor().max(1.) as usize;
        let mut tabs = div().flex().flex_wrap().items_center().gap(px(6.));
        for (index, name) in GROUPS.iter().enumerate() {
            tabs = tabs.child(
                style::tab(
                    ("group", index),
                    GROUP_ICONS[index],
                    *name,
                    index == self.group,
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.group = index;
                    if index == SAVES_GROUP {
                        this.refresh_saves();
                    }
                    cx.notify();
                })),
            );
        }
        tabs = tabs
            .child(div().flex_1())
            .child(style::small_button("menu", "…", true).on_click(cx.listener(
                |this, _, _, cx| {
                    this.menu_open = !this.menu_open;
                    cx.notify();
                },
            )));
        let mut tabs = div().relative().child(tabs);
        if self.menu_open {
            tabs = tabs.child(gpui::deferred(
                div()
                    .absolute()
                    .top(px(46.))
                    .right_0()
                    .child(self.advanced_menu(cx)),
            ));
        }
        let rows = if self.group == MODELS_GROUP {
            vec![self.models_panel(cx)]
        } else if self.group == SAVES_GROUP {
            vec![self.saves_panel(cx)]
        } else {
            CONTROLS
                .iter()
                .enumerate()
                .filter(|(_, c)| c.group == self.group && !HIDDEN.contains(&c.flag))
                .map(|(i, _)| self.row(i, cx))
                .collect::<Vec<_>>()
        };
        let arguments = launched.arguments();
        let [binary_command, cargo_command] = commands(&self.executable, &arguments);
        let mut footer = style::panel().flex_shrink_0().gap(px(8.));
        for (index, (label, command)) in
            [("Release binary", binary_command), ("Cargo", cargo_command)]
                .into_iter()
                .enumerate()
        {
            let copy_command = command.clone();
            footer = footer.child(
                div()
                    .relative()
                    .flex()
                    .gap(px(12.))
                    .items_center()
                    .child(div().w(px(130.)).child(style::heading(label)))
                    .child(
                        div()
                            .id(("command", index))
                            .flex_1()
                            .min_w_0()
                            .max_h(px(40.))
                            .overflow_y_scroll()
                            .text_size(px(12.))
                            .font_family(style::MONO)
                            .text_color(rgb(style::TEXT))
                            .child(command),
                    )
                    .child(
                        style::button(("copy", index), "Copy", Look::Plain, true).on_click(
                            cx.listener(move |_, _, _, cx| {
                                cx.write_to_clipboard(ClipboardItem::new_string(
                                    copy_command.clone(),
                                ));
                            }),
                        ),
                    ),
            );
        }
        let wrong = error.is_some();
        footer = footer.child(
            div()
                .relative()
                .flex()
                .gap(px(12.))
                .items_center()
                .child(
                    style::button("continue", "Continue", Look::Framed, true)
                        .on_click(cx.listener(|this, _, _, cx| this.continue_latest(cx))),
                )
                .child(div().flex_1().min_w_0().child(style::status(
                    error.unwrap_or_else(|| "Ready · each launch opens a separate process".into()),
                    wrong,
                )))
                .child(
                    style::button("quit", "Close launcher", Look::Plain, true)
                        .on_click(cx.listener(|_, _, _, cx| cx.quit())),
                )
                .child(
                    style::button("launch", "Launch game", Look::Gold, !wrong).when(!wrong, |b| {
                        b.on_click(cx.listener(|this, _, _, cx| this.launch(cx)))
                    }),
                ),
        );
        footer = footer.child(div().relative().child(self.status_lines()));
        let header = div()
            .flex()
            .items_center()
            .gap(px(16.))
            .child(
                style::button("to-simple", "‹ Back", Look::Plain, true)
                    .on_click(cx.listener(|this, _, window, cx| this.go(Screen::Home, window, cx))),
            )
            .child(style::title("Advanced"))
            .child(style::hint(format!(
                "Every option of the game's command line · {}",
                crate::args::VERSION
            )));
        let options = style::panel().flex_1().min_h_0().py(px(8.)).child(
            div()
                .id(("options", self.group))
                .relative()
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .children(rows),
        );
        let root = style::ground().child(style::backdrop()).child(
            div()
                .absolute()
                .size_full()
                .flex()
                .flex_col()
                .gap(px(10.))
                .p(px(16.))
                .child(header)
                .child(self.carousel(visible, cx))
                .child(tabs)
                .child(options)
                .child(footer),
        );
        root.into_any_element()
    }
}

/// The HUD icon of each option group, in `GROUPS` order.
const GROUP_ICONS: [&str; 8] = [
    "flag",
    "menu",
    "centre",
    "pic_tower",
    "clock",
    "diplomacy",
    "cmd_repair",
    "pic_scroll",
];

impl Launcher {
    /// The match report of a saved game (task 117), with Back and Exit.
    pub(super) fn render_report(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let header = div()
            .flex()
            .items_center()
            .gap(px(16.))
            .child(
                style::button("report-back", "‹ Back", Look::Plain, true).on_click(cx.listener(
                    |this, _, window, cx| {
                        let to = this.report_from;
                        this.go(to, window, cx)
                    },
                )),
            )
            .child(style::title("Match report"))
            .child(div().flex_1())
            .child(
                style::button("report-exit", "Exit", Look::Plain, true)
                    .on_click(cx.listener(|_, _, _, cx| cx.quit())),
            );
        let body = match &self.report {
            None => style::hint("Reading the save …").into_any_element(),
            Some(Err(error)) => div()
                .text_color(rgb(style::WARN))
                .child(format!("The report could not be made: {error}"))
                .into_any_element(),
            Some(Ok(report)) => crate::report_view::report_element(report, None, 1180.),
        };
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap(px(12.))
            .p(px(20.))
            .bg(rgb(0x0b1016))
            .text_color(rgb(0xf2f2f2))
            .child(header)
            .child(
                style::panel().flex_1().min_h_0().p(px(14.)).child(
                    div()
                        .id("report-body")
                        .size_full()
                        .overflow_y_scroll()
                        .child(body),
                ),
            )
            .into_any_element()
    }
}
