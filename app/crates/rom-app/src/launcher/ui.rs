//! GPUI projection of the launcher form. Child games own their graphics threads.

use std::path::PathBuf;
use std::process::{Child, Command};
use std::time::Duration;

use gpui::{
    AppContext, ClipboardItem, Context, Entity, InteractiveElement, IntoElement, ParentElement,
    Render, SharedString, StatefulInteractiveElement, Styled, Subscription, Window, div, px, rgb,
};
use gpui_component::{
    Disableable, Selectable, Sizable,
    button::{Button, ButtonVariants},
    checkbox::Checkbox,
    input::{Input, InputEvent, InputState},
    select::{SearchableVec, Select, SelectEvent, SelectState},
};

use super::form::{
    CONTROLS, Config, Form, GROUPS, MODELS_GROUP, PRESETS, command_line, commands, preset,
};
use super::models;
use super::{Catalog, Choice};

type ChoiceState = Entity<SelectState<SearchableVec<Choice>>>;

struct Field {
    input: Option<Entity<InputState>>,
    select: Option<ChoiceState>,
}

struct Process {
    child: Child,
    status: String,
    finished: bool,
}

pub(super) struct Launcher {
    config: Config,
    catalog: Catalog,
    path: PathBuf,
    executable: PathBuf,
    fields: Vec<Field>,
    subscriptions: Vec<Subscription>,
    generation: u64,
    group: usize,
    factions_open: bool,
    computers_open: bool,
    look_selects: Vec<ChoiceState>,
    distance: Option<Entity<InputState>>,
    preset_name: Entity<InputState>,
    processes: Vec<Process>,
    message: Option<String>,
    model_folder: Entity<InputState>,
    model_status: String,
    model_unlock: bool,
    model_busy: bool,
}

impl Launcher {
    pub fn new(
        config: Config,
        catalog: Catalog,
        path: PathBuf,
        executable: PathBuf,
        warning: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut this = Self {
            config,
            catalog,
            path,
            executable,
            fields: Vec::new(),
            subscriptions: Vec::new(),
            generation: 0,
            group: 0,
            factions_open: false,
            computers_open: false,
            look_selects: Vec::new(),
            distance: None,
            preset_name: cx
                .new(|cx| InputState::new(window, cx).placeholder("Name a custom preset")),
            processes: Vec::new(),
            message: warning,
            model_folder: cx.new(|cx| {
                InputState::new(window, cx)
                    .default_value(models::default_folder().display().to_string())
            }),
            model_status: "Export writes <model id>.octd and .md for every model; import reads \
                           one edited .octd back into db/models."
                .into(),
            model_unlock: false,
            model_busy: false,
        };
        this.prepare(window, cx);
        cx.spawn(async |this, cx| {
            // Present the initial form after X11 has mapped the window, even without input.
            cx.background_executor()
                .timer(Duration::from_millis(100))
                .await;
            if this.update(cx, |_, cx| cx.notify()).is_err() {
                return;
            }
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(500))
                    .await;
                if this
                    .update(cx, |this, cx| {
                        let mut changed = false;
                        for process in &mut this.processes {
                            if !process.finished {
                                match process.child.try_wait() {
                                    Ok(Some(status)) => {
                                        process.status = status.code().map_or_else(
                                            || "exited by signal".into(),
                                            |code| format!("exited · code {code}"),
                                        );
                                        process.finished = true;
                                        eprintln!(
                                            "launcher: pid {} {}",
                                            process.child.id(),
                                            process.status
                                        );
                                        changed = true;
                                    }
                                    Ok(None) => {}
                                    Err(e) => {
                                        process.status = format!("status unavailable: {e}");
                                        changed = true;
                                    }
                                }
                            }
                        }
                        if changed {
                            cx.notify();
                        }
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
        // Opt-in: open on another group (`ROM_LAUNCHER_GROUP=5` shows Models), for screenshots.
        if let Some(group) = std::env::var("ROM_LAUNCHER_GROUP")
            .ok()
            .and_then(|g| g.parse::<usize>().ok())
            .filter(|g| *g < GROUPS.len())
        {
            this.group = group;
        }
        // Opt-in visual smoke test uses the same form and launch handler as the button.
        // Use an isolated XDG_CONFIG_HOME; no desktop automation or window manipulation.
        if std::env::var_os("ROM_LAUNCHER_SMOKE").is_some() {
            this.config.form = preset(4, this.catalog.first_model());
            this.config
                .form
                .set("screenshot", "target/from-launcher.png");
            this.config.form.set("after", "3");
            this.group = 3;
            this.prepare(window, cx);
            cx.spawn(async |this, cx| {
                cx.background_executor().timer(Duration::from_secs(2)).await;
                let _ = this.update(cx, |this, cx| this.launch(cx));
            })
            .detach();
        }
        this
    }

    fn save(&mut self) {
        if let Err(e) = self.config.save(&self.path) {
            self.message = Some(format!("Could not save {}: {e}", self.path.display()));
        }
    }

    fn change(&mut self, flag: &str, value: String, cx: &mut Context<Self>) {
        self.config.form.set(flag, value);
        self.save();
        cx.notify();
    }

    fn replace(&mut self, form: Form, window: &mut Window, cx: &mut Context<Self>) {
        self.config.form = form;
        self.prepare(window, cx);
        self.save();
        cx.notify();
    }

    fn prepare(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.generation += 1;
        let generation = self.generation;
        self.fields.clear();
        self.subscriptions.clear();
        for (index, control) in CONTROLS.iter().enumerate() {
            let flag = control.flag;
            let input = if control.toggle {
                None
            } else {
                let input = cx.new(|cx| {
                    InputState::new(window, cx)
                        .default_value(self.config.form.get(flag).to_owned())
                        .placeholder("Default / disabled")
                });
                self.subscriptions.push(cx.subscribe_in(
                    &input,
                    window,
                    move |this, input, event: &InputEvent, window, cx| {
                        if this.generation == generation && matches!(event, InputEvent::Change) {
                            let value = input.read(cx).value().to_string();
                            this.change(flag, value.clone(), cx);
                            if let Some(select) = this.fields[index].select.clone() {
                                select.update(cx, |select, cx| {
                                    select.set_selected_value(&value, window, cx)
                                });
                            }
                        }
                    },
                ));
                Some(input)
            };
            let select = if let Some(choices) = self.catalog.lists.get(flag) {
                let mut choices = choices.clone();
                choices.insert(
                    0,
                    Choice {
                        key: String::new(),
                        label: "Default / disabled".into(),
                    },
                );
                let selected = choices
                    .iter()
                    .position(|c| c.key == self.config.form.get(flag))
                    .map(|i| gpui_component::IndexPath::default().row(i));
                let state = cx.new(|cx| {
                    SelectState::new(SearchableVec::new(choices), selected, window, cx)
                        .searchable(true)
                });
                let input = input.clone();
                self.subscriptions.push(cx.subscribe_in(
                    &state,
                    window,
                    move |this, _, event: &SelectEvent<SearchableVec<Choice>>, window, cx| {
                        if this.generation != generation {
                            return;
                        }
                        if let SelectEvent::Confirm(Some(value)) = event {
                            this.change(flag, value.clone(), cx);
                            if let Some(input) = &input {
                                input.update(cx, |input, cx| {
                                    input.set_value(value.clone(), window, cx)
                                });
                            }
                        }
                    },
                ));
                Some(state)
            } else {
                None
            };
            self.fields.push(Field { input, select });
        }
        self.prepare_look(window, cx);
    }

    fn prepare_look(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.look_selects.clear();
        let generation = self.generation;
        let text = self.config.form.get("look-at");
        let (keys, distance) = text.split_once(',').unwrap_or((text, ""));
        let (terrain, near) = keys.split_once('+').unwrap_or((keys, ""));
        let values = [terrain.to_owned(), near.to_owned()];
        let distance = distance.to_owned();
        for (index, value) in values.iter().enumerate() {
            let mut choices = self.catalog.terrains.clone();
            choices.insert(
                0,
                Choice {
                    key: String::new(),
                    label: if index == 0 {
                        "No terrain target"
                    } else {
                        "No nearby terrain"
                    }
                    .into(),
                },
            );
            let selected = choices
                .iter()
                .position(|c| c.key == *value)
                .map(|i| gpui_component::IndexPath::default().row(i));
            let state = cx.new(|cx| {
                SelectState::new(SearchableVec::new(choices), selected, window, cx).searchable(true)
            });
            self.subscriptions.push(cx.subscribe_in(
                &state,
                window,
                move |this, _, event: &SelectEvent<SearchableVec<Choice>>, window, cx| {
                    if this.generation == generation && matches!(event, SelectEvent::Confirm(_)) {
                        this.compose_look(window, cx);
                    }
                },
            ));
            self.look_selects.push(state);
        }
        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(distance)
                .placeholder("Distance (tiles)")
        });
        self.subscriptions.push(cx.subscribe_in(
            &input,
            window,
            move |this, _, event: &InputEvent, window, cx| {
                if this.generation == generation && matches!(event, InputEvent::Change) {
                    this.compose_look(window, cx);
                }
            },
        ));
        self.distance = Some(input);
    }

    fn compose_look(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let values: Vec<String> = self
            .look_selects
            .iter()
            .map(|s| s.read(cx).selected_value().cloned().unwrap_or_default())
            .collect();
        let mut value = values[0].clone();
        if !value.is_empty() {
            if !values[1].is_empty() {
                value.push('+');
                value.push_str(&values[1]);
            }
            let distance = self.distance.as_ref().unwrap().read(cx).value();
            if !distance.is_empty() {
                value.push(',');
                value.push_str(&distance);
            }
        }
        self.change("look-at", value.clone(), cx);
        let index = CONTROLS.iter().position(|c| c.flag == "look-at").unwrap();
        self.fields[index]
            .input
            .clone()
            .unwrap()
            .update(cx, |input, cx| input.set_value(value, window, cx));
    }

    fn launch(&mut self, cx: &mut Context<Self>) {
        if let Err(error) = self.config.form.validate() {
            self.message = Some(error);
            cx.notify();
            return;
        }
        self.message = None;
        self.save();
        let arguments = self.config.form.arguments();
        match Command::new(&self.executable)
            .args(&arguments)
            .env_remove("WAYLAND_DISPLAY")
            .spawn()
        {
            Ok(child) => {
                eprintln!(
                    "launcher: started pid {} {}",
                    child.id(),
                    command_line(&self.executable, &arguments)
                );
                self.processes.push(Process {
                    child,
                    status: "running".into(),
                    finished: false,
                });
            }
            Err(e) => {
                self.message = Some(format!(
                    "Could not launch {}: {e}",
                    self.executable.display()
                ))
            }
        }
        cx.notify();
    }

    /// Runs `work` off the UI thread, showing `pending` until its status line arrives.
    fn model_job(
        &mut self,
        pending: &str,
        work: impl FnOnce() -> Result<String, String> + Send + 'static,
        cx: &mut Context<Self>,
    ) {
        if self.model_busy {
            return;
        }
        self.model_busy = true;
        self.model_status = pending.into();
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = cx.background_executor().spawn(async move { work() }).await;
            let _ = this.update(cx, |this, cx| {
                this.model_busy = false;
                this.model_status = result.unwrap_or_else(|e| e);
                cx.notify();
            });
        })
        .detach();
    }

    /// The Models group: the export folder, the two model tools and their status line.
    fn models_panel(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let folder_row = div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .w(px(145.))
                    .flex_shrink_0()
                    .text_color(rgb(0xe8c98a))
                    .child("Export folder"),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(Input::new(&self.model_folder).small()),
            )
            .child(
                Button::new("model-folder")
                    .small()
                    .label("Choose folder…")
                    .on_click(cx.listener(|_, _, window, cx| {
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
                    })),
            );
        let actions = div()
            .flex()
            .items_center()
            .gap_3()
            .child(
                Button::new("export-models")
                    .primary()
                    .label("Export all models")
                    .disabled(self.model_busy)
                    .on_click(cx.listener(|this, _, _, cx| {
                        let folder = PathBuf::from(this.model_folder.read(cx).value().trim());
                        this.model_job(
                            "Exporting every model…",
                            move || models::export_into(&folder),
                            cx,
                        );
                    })),
            )
            .child(
                Button::new("import-model")
                    .label("Import model…")
                    .disabled(self.model_busy)
                    .on_click(cx.listener(|this, _, _, cx| {
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
                    })),
            )
            .child(
                Checkbox::new("model-unlock")
                    .label("Allow changing approved models (--unlock)")
                    .checked(self.model_unlock)
                    .on_click(cx.listener(|this, checked: &bool, _, cx| {
                        this.model_unlock = *checked;
                        cx.notify();
                    })),
            );
        div()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .child(
                div().text_color(rgb(0xe8c98a)).child(
                    "Edit building and unit models in Meshcraft · docs/models-in-meshcraft.md",
                ),
            )
            .child(folder_row)
            .child(actions)
            .child(
                div()
                    .id("model-status")
                    .text_sm()
                    .max_h(px(160.))
                    .overflow_y_scroll()
                    .text_color(rgb(if self.model_busy { 0xf0b030 } else { 0x3fd0ff }))
                    .child(self.model_status.clone()),
            )
            .into_any_element()
    }

    /// One line per computer slot: a button per profile and per difficulty.
    fn computer_slots(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let form = &self.config.form;
        let mut list = div()
            .id("computer-list")
            .max_h(px(260.))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_2()
            .p_2();
        for slot in 0..form.computer_slots() {
            let (profile, difficulty) = form.computer(slot);
            let mut line = div().flex().flex_wrap().items_center().gap_1().child(
                div()
                    .w(px(70.))
                    .text_color(rgb(0xe8c98a))
                    .child(format!("Player {}", slot + form.first_computer())),
            );
            for (index, choice) in self.catalog.profiles.iter().enumerate() {
                let (key, difficulty) = (choice.key.clone(), difficulty.clone());
                line = line.child(
                    Button::new(("ai-profile", slot * 64 + index))
                        .small()
                        .label(choice.label.clone())
                        .selected(choice.key == profile)
                        .on_click(cx.listener(move |this, _, window, cx| {
                            let mut form = this.config.form.clone();
                            form.set_computer(slot, &key, &difficulty);
                            this.replace(form, window, cx);
                        })),
                );
            }
            line = line.child(div().w(px(12.)));
            for (index, choice) in self.catalog.difficulties.iter().enumerate() {
                let (key, profile) = (choice.key.clone(), profile.clone());
                line = line.child(
                    Button::new(("ai-difficulty", slot * 64 + index))
                        .small()
                        .label(choice.label.clone())
                        .selected(choice.key == difficulty)
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

    fn row(&self, index: usize, cx: &mut Context<Self>) -> gpui::AnyElement {
        let control = &CONTROLS[index];
        let flag = control.flag;
        let field = &self.fields[index];
        let default = if control.default.is_empty() {
            "automatic / disabled"
        } else {
            control.default
        };
        let mut row = div()
            .flex()
            .flex_col()
            .gap_1()
            .p_3()
            .border_b_1()
            .border_color(rgb(0x2a3340));
        let mut editor = div().flex().items_center().gap_2().w_full();
        if control.toggle {
            editor = editor.child(
                Checkbox::new(SharedString::from(flag))
                    .label(format!("--{flag}"))
                    .checked(self.config.form.get(flag) == "true")
                    .on_click(cx.listener(move |this, checked: &bool, _, cx| {
                        this.change(flag, checked.to_string(), cx)
                    })),
            );
        } else {
            editor = editor
                .child(
                    div()
                        .w(px(145.))
                        .flex_shrink_0()
                        .text_color(rgb(0xe8c98a))
                        .child(format!("--{flag}")),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .child(Input::new(field.input.as_ref().unwrap()).small()),
                );
            if let Some(select) = &field.select {
                editor = editor.child(
                    div()
                        .w(px(300.))
                        .child(Select::new(select).small().placeholder("Choose…")),
                );
            }
            if flag == "seed" {
                editor = editor.child(Button::new("random").small().label("Random").on_click(
                    cx.listener(|this, _, window, cx| {
                        let seed = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_nanos() as u64;
                        let mut form = this.config.form.clone();
                        form.set("seed", seed.to_string());
                        this.replace(form, window, cx);
                    }),
                ));
            }
            if flag == "ai" {
                editor = editor.child(
                    Button::new("choose-computers")
                        .small()
                        .label("Choose per slot…")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.computers_open = !this.computers_open;
                            cx.notify();
                        })),
                );
            }
            if flag == "factions" {
                editor = editor.child(
                    Button::new("choose-factions")
                        .small()
                        .label("Choose factions…")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.factions_open = !this.factions_open;
                            cx.notify();
                        })),
                );
            }
        }
        row = row.child(editor).child(
            div()
                .text_xs()
                .text_color(rgb(0xaab7c7))
                .child(format!("{}  Default: {default}", control.help)),
        );
        if flag == "factions" && self.factions_open {
            let mut list = div()
                .id("faction-list")
                .max_h(px(190.))
                .overflow_y_scroll()
                .flex()
                .flex_col()
                .gap_2()
                .p_2();
            for choice in &self.catalog.factions {
                let key = choice.key.clone();
                list = list.child(
                    Checkbox::new(SharedString::from(key.clone()))
                        .label(choice.label.clone())
                        .checked(
                            self.config
                                .form
                                .get("factions")
                                .split(',')
                                .any(|v| v == key),
                        )
                        .on_click(cx.listener(move |this, checked: &bool, window, cx| {
                            let mut keys = this
                                .config
                                .form
                                .get("factions")
                                .split(',')
                                .filter(|s| !s.is_empty() && *s != key)
                                .map(str::to_owned)
                                .collect::<Vec<_>>();
                            if *checked {
                                keys.push(key.clone());
                            }
                            let mut form = this.config.form.clone();
                            form.set("factions", keys.join(","));
                            this.replace(form, window, cx);
                        })),
                );
            }
            row = row.child(list);
        }
        if flag == "ai" && self.computers_open {
            row = row.child(self.computer_slots(cx));
        }
        if flag == "look-at" {
            row = row.child(
                div()
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
                            .child(Input::new(self.distance.as_ref().unwrap()).small()),
                    ),
            );
        }
        row.into_any_element()
    }
}

impl Render for Launcher {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let validation = self.config.form.validate();
        let error = validation.as_ref().err().cloned();
        let mut presets = div().flex().gap_2().flex_wrap();
        for (index, name) in PRESETS.iter().enumerate() {
            presets = presets.child(
                Button::new(("preset", index))
                    .small()
                    .label(*name)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        let form = preset(index, this.catalog.first_model());
                        this.group = match index {
                            3 | 4 => 3,
                            5 => 4,
                            _ => 0,
                        };
                        this.replace(form, window, cx);
                    })),
            );
        }
        let mut custom = div()
            .flex()
            .gap_2()
            .items_center()
            .child(
                div()
                    .w(px(230.))
                    .child(Input::new(&self.preset_name).small()),
            )
            .child(
                Button::new("save-preset")
                    .small()
                    .label("Save preset")
                    .on_click(cx.listener(|this, _, _, cx| {
                        let name = this.preset_name.read(cx).value().trim().to_owned();
                        if name.is_empty() {
                            this.message = Some("Enter a preset name first.".into());
                        } else if this.config.presets.len() >= 12
                            && !this.config.presets.contains_key(&name)
                        {
                            this.message =
                                Some("Twelve custom presets saved; remove one first.".into());
                        } else {
                            this.config.presets.insert(name, this.config.form.clone());
                            this.message = None;
                            this.save();
                        }
                        cx.notify();
                    })),
            )
            .child(
                Button::new("reset")
                    .small()
                    .label("Reset to defaults")
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.replace(Form::default(), window, cx);
                    })),
            );
        for (index, name) in self.config.presets.keys().enumerate() {
            let name = name.clone();
            let remove_name = name.clone();
            custom =
                custom
                    .child(
                        Button::new(("custom", index))
                            .small()
                            .label(name.clone())
                            .on_click(cx.listener(move |this, _, window, cx| {
                                let form = this.config.presets[&name].clone();
                                this.replace(form, window, cx);
                            })),
                    )
                    .child(Button::new(("remove", index)).small().label("×").on_click(
                        cx.listener(move |this, _, _, cx| {
                            this.config.presets.remove(&remove_name);
                            this.save();
                            cx.notify();
                        }),
                    ));
        }
        let mut tabs = div().flex().gap_2().pb_2();
        for (index, name) in GROUPS.iter().enumerate() {
            tabs = tabs.child(
                Button::new(("group", index))
                    .label(*name)
                    .selected(index == self.group)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.group = index;
                        cx.notify();
                    })),
            );
        }
        let rows = if self.group == MODELS_GROUP {
            vec![self.models_panel(cx)]
        } else {
            CONTROLS
                .iter()
                .enumerate()
                .filter(|(_, c)| c.group == self.group)
                .map(|(i, _)| self.row(i, cx))
                .collect::<Vec<_>>()
        };
        let arguments = self.config.form.arguments();
        let [binary_command, cargo_command] = commands(&self.executable, &arguments);
        let mut footer = div()
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .bg(rgb(0x141b24))
            .border_t_1()
            .border_color(rgb(0xb88a45));
        for (index, (label, command)) in
            [("RELEASE BINARY", binary_command), ("CARGO", cargo_command)]
                .into_iter()
                .enumerate()
        {
            let copy_command = command.clone();
            footer = footer.child(
                div()
                    .flex()
                    .gap_3()
                    .items_center()
                    .child(
                        div()
                            .w(px(125.))
                            .text_xs()
                            .text_color(rgb(0xe8c98a))
                            .child(label),
                    )
                    .child(
                        div()
                            .id(("command", index))
                            .flex_1()
                            .min_w_0()
                            .max_h(px(70.))
                            .overflow_y_scroll()
                            .text_xs()
                            .font_family("DejaVu Sans Mono")
                            .child(command),
                    )
                    .child(Button::new(("copy", index)).small().label("Copy").on_click(
                        cx.listener(move |_, _, _, cx| {
                            cx.write_to_clipboard(ClipboardItem::new_string(copy_command.clone()));
                        }),
                    )),
            );
        }
        footer = footer.child(
            div()
                .flex()
                .gap_3()
                .items_center()
                .child(
                    Button::new("launch")
                        .primary()
                        .label("Launch game")
                        .disabled(error.is_some())
                        .on_click(cx.listener(|this, _, _, cx| this.launch(cx))),
                )
                .child(
                    div()
                        .flex_1()
                        .text_sm()
                        .text_color(rgb(if error.is_some() { 0xf0b030 } else { 0x3fd0ff }))
                        .child(error.unwrap_or_else(|| {
                            "Ready · each launch opens a separate process".into()
                        })),
                )
                .child(
                    Button::new("quit")
                        .label("Close launcher")
                        .on_click(cx.listener(|_, _, _, cx| cx.quit())),
                ),
        );
        if let Some(message) = &self.message {
            footer = footer.child(
                div()
                    .text_sm()
                    .text_color(rgb(0xf0b030))
                    .child(message.clone()),
            );
        }
        if !self.processes.is_empty() {
            footer = footer.child(
                div()
                    .id("processes")
                    .max_h(px(85.))
                    .overflow_y_scroll()
                    .children(self.processes.iter().map(|p| {
                        div()
                            .text_xs()
                            .child(format!("PID {} · {}", p.child.id(), p.status))
                    })),
            );
        }
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(0x0c1118))
            .text_color(rgb(0xf2f2f2))
            .text_sm()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .p_4()
                    .flex_shrink_0()
                    .child(
                        div()
                            .text_xl()
                            .text_color(rgb(0xe8c98a))
                            .child("RISE OF MAN  /  LAUNCHER"),
                    )
                    .child(presets)
                    .child(custom.flex_wrap())
                    .child(tabs),
            )
            .child(
                div()
                    .id(("options", self.group))
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .bg(rgb(0x0a0e14))
                    .children(rows),
            )
            .child(footer.flex_shrink_0())
    }
}
