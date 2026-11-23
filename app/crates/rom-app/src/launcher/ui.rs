//! GPUI projection of the launcher form. Child games own their graphics threads.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::time::{Duration, Instant};

use gpui::{
    AppContext, Context, Entity, FocusHandle, InteractiveElement, IntoElement, ParentElement,
    Render, Styled, Subscription, Window, actions, div,
};
use gpui_component::{
    input::{InputEvent, InputState},
    select::{SearchableVec, SelectEvent, SelectState},
    slider::{SliderEvent, SliderState},
};

use super::form::{CONTROLS, Config, Form, GROUPS, command_line, preset};
use super::models;
use super::presets::{self, Preset};
use super::scripts;
use super::simple::Mode;
use super::{Catalog, Choice};

actions!(rom_launcher, [Back, Confirm]);

/// The key context of the launcher's root, where Escape and Enter are bound.
pub(super) const CONTEXT: &str = "RomLauncher";

/// Which screen the launcher shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Screen {
    /// The four big choices.
    Home,
    /// One simple form.
    Form(Mode),
    /// The launcher with every option.
    Advanced,
    /// The match report of a saved game (task 117).
    Report,
}

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
    saves: Vec<crate::saves::Entry>,
    saves_note: Option<String>,
    /// The match report shown (task 117), or why it could not be made; `None` while it loads.
    report: Option<Result<std::sync::Arc<rom_match::MatchReport>, String>>,
    /// The screen the report goes back to.
    report_from: Screen,
    screen: Screen,
    focus: FocusHandle,
    simple_inputs: BTreeMap<&'static str, Entity<InputState>>,
    /// The nation dropdown of each race with more nations than a row of choices holds.
    nation_selects: BTreeMap<String, ChoiceState>,
    simple_subscriptions: Vec<Subscription>,
    /// A Start press within a second of the last one is ignored (Enter can arrive twice).
    last_start: Option<Instant>,
    /// Extra arguments for the next Start (a test hook: `ROM_LAUNCHER_APPEND`).
    append: Vec<String>,
    /// The presets that come with the game (task 112).
    bundled: Vec<Preset>,
    /// The forms' setup script dropdown and starting age dropdown.
    script_select: Option<ChoiceState>,
    age_select: Option<ChoiceState>,
    /// The name a Play form is saved under as a preset.
    play_preset_name: Entity<InputState>,
    /// The first card the advanced carousel shows.
    carousel: usize,
    /// Whether the advanced launcher's "…" menu is open.
    menu_open: bool,
    /// The launcher's music (task 133) and when it began.
    music: crate::sound::music::LauncherMusic,
    music_began: Instant,
    /// The Sound group's volumes as sliders (task 133), by option name.
    volume_sliders: BTreeMap<&'static str, Entity<SliderState>>,
    /// The form's editors must be made again (a game changed the Sound settings).
    stale_fields: bool,
}

/// Lists this short are drawn as a row of choices; longer ones as a dropdown.
const SEGMENTS: usize = 7;

mod view;

/// The simple forms' text fields.
const TEXT_FIELDS: [&str; 8] = [
    "seed", "speed", "port", "url", "key", "name", "bind", "autosave",
];

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
            saves: Vec::new(),
            saves_note: None,
            report: None,
            report_from: Screen::Home,
            screen: Screen::Home,
            focus: cx.focus_handle().tab_stop(false),
            simple_inputs: BTreeMap::new(),
            nation_selects: BTreeMap::new(),
            simple_subscriptions: Vec::new(),
            last_start: None,
            append: std::env::var("ROM_LAUNCHER_APPEND")
                .map(|text| text.split_whitespace().map(str::to_owned).collect())
                .unwrap_or_default(),
            bundled: presets::bundled(),
            script_select: None,
            age_select: None,
            play_preset_name: cx.new(|cx| InputState::new(window, cx).placeholder("Preset name")),
            carousel: 0,
            menu_open: false,
            music: crate::sound::music::LauncherMusic::new(),
            music_began: Instant::now(),
            volume_sliders: BTreeMap::new(),
            stale_fields: false,
        };
        // The bundled scripts live beside the settings, where a game can read them.
        if let Err(error) = scripts::install(&this.path) {
            this.message = Some(format!("Could not write the bundled scripts: {error}"));
        }
        if this.find_preset(&this.config.chosen).is_none() {
            this.config.chosen = this
                .bundled
                .get(1)
                .or(this.bundled.first())
                .map(|p| p.name.clone())
                .unwrap_or_default();
        }
        // Test hook: `ROM_LAUNCHER_PRESET=<name>` chooses a preset tile and puts it in the forms.
        if let Ok(name) = std::env::var("ROM_LAUNCHER_PRESET")
            && let Some(preset) = this.find_preset(&name)
        {
            this.config.simple = preset.apply(&this.config.simple, &this.path);
            this.config.chosen = name;
        }
        this.refresh_saves();
        this.prepare(window, cx);
        this.prepare_simple(window, cx);
        window.focus(&this.focus);
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
                        // A game remembers seat tokens for the Join form's Rejoin (task 88), and
                        // the Sound settings changed in its menu (task 133).
                        if let Ok(on_disk) = Config::load(&this.path) {
                            if on_disk.tokens != this.config.tokens {
                                this.config.tokens = on_disk.tokens;
                                changed = true;
                            }
                            let sound = on_disk.form.sound_settings();
                            if changed && sound != this.config.form.sound_settings() {
                                this.config.form.set_sound(&sound);
                                crate::sound::set_settings(this.config.form.sound_settings());
                                this.stale_fields = true;
                            }
                        }
                        // The launcher's music (task 133): held while a game runs or while
                        // the person has turned it off.
                        let running = this.processes.iter().any(|p| !p.finished);
                        let wanted = this.config.launcher_music && !running;
                        let actions = this.music.tick(this.music_began.elapsed(), wanted);
                        if let Some(engine) = crate::sound::engine() {
                            crate::sound::music::perform(&engine, &actions);
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
        // Opt-in, for screenshots and scripted runs: open on a screen (`simple`, `play`, `host`,
        // `join`, `server`, `advanced`); `ROM_LAUNCHER_START=1` then presses the form's Start
        // after two seconds, with `ROM_LAUNCHER_APPEND` added to its arguments.
        if let Ok(name) = std::env::var("ROM_LAUNCHER_SCREEN") {
            this.screen = match name.as_str() {
                "play" => Screen::Form(Mode::Play),
                "host" => Screen::Form(Mode::Host),
                "join" => Screen::Form(Mode::Join),
                "server" => Screen::Form(Mode::Serve),
                "advanced" => Screen::Advanced,
                _ => Screen::Home,
            };
        }
        // `ROM_LAUNCHER_START=1` presses the form's final button, or on the home screen Play.
        if std::env::var_os("ROM_LAUNCHER_START").is_some() {
            let screen = this.screen;
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(Duration::from_secs(2)).await;
                let _ = this.update(cx, |this, cx| match screen {
                    Screen::Form(mode) => this.start(mode, cx),
                    Screen::Home => this.play_chosen(cx),
                    Screen::Advanced | Screen::Report => {}
                });
            })
            .detach();
        }
        // Test hook: `ROM_LAUNCHER_MENU=1` opens the advanced launcher's "…" menu.
        this.menu_open = std::env::var_os("ROM_LAUNCHER_MENU").is_some();
        // Opt-in: open on another group (`ROM_LAUNCHER_GROUP=6` shows Models), for screenshots.
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
            this.config.form = preset(4, this.catalog.first_model(), &this.catalog.planet_style);
            this.config
                .form
                .set("screenshot", "target/from-launcher.png");
            this.config.form.set("after", "3");
            this.screen = Screen::Advanced;
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
        crate::sound::set_settings(self.config.form.sound_settings());
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
        // The Sound group's volumes are sliders (task 133).
        self.volume_sliders.clear();
        for (flag, _) in crate::sound::SLIDERS {
            let value = self
                .config
                .form
                .get(flag)
                .trim()
                .parse::<f32>()
                .unwrap_or(100.)
                .clamp(0., 100.);
            let slider = cx.new(|_| {
                SliderState::new()
                    .min(0.)
                    .max(100.)
                    .step(1.)
                    .default_value(value)
            });
            self.subscriptions.push(cx.subscribe_in(
                &slider,
                window,
                move |this, _, event: &SliderEvent, _, cx| {
                    let SliderEvent::Change(value) = event;
                    let text = (value.end().round().clamp(0., 100.) as u32).to_string();
                    if this.generation == generation && this.config.form.get(flag) != text {
                        this.change(flag, text, cx);
                    }
                },
            ));
            self.volume_sliders.insert(flag, slider);
        }
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
            let list = if flag == "script" {
                Some(self.script_choices(self.config.form.get(flag)))
            } else {
                self.catalog
                    .lists
                    .get(flag)
                    .filter(|c| c.len() > SEGMENTS)
                    .cloned()
            };
            let select = if let Some(mut choices) = list {
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
        let form = self.config.form.clone().universe_match();
        if let Err(error) = form.validate() {
            self.message = Some(error);
            cx.notify();
            return;
        }
        self.message = None;
        self.save();
        let arguments = form.arguments();
        self.spawn_game(arguments, cx);
    }

    /// Starts one game (or server) process with these arguments.
    fn spawn_game(&mut self, arguments: Vec<String>, cx: &mut Context<Self>) {
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

    /// Reads the save folder again.
    fn refresh_saves(&mut self) {
        match crate::saves::folder() {
            Ok(folder) => {
                self.saves = crate::saves::list(&folder, &self.catalog.rules_id);
                self.saves_note = Some(if self.saves.is_empty() {
                    format!("No saved games in {}.", folder.display())
                } else {
                    format!("Saved games in {}", folder.display())
                });
            }
            Err(error) => {
                self.saves.clear();
                self.saves_note = Some(format!("No save folder: {error}"));
            }
        }
    }

    /// Starts a game process that continues the save at `path`, paused.
    fn load_save(&mut self, path: &std::path::Path, cx: &mut Context<Self>) {
        let display = crate::saves::display_arguments(&self.config.form.arguments());
        match crate::saves::relaunch(path, &display) {
            Ok(child) => {
                eprintln!(
                    "launcher: started pid {} --load {}",
                    child.id(),
                    path.display()
                );
                self.message = None;
                self.processes.push(Process {
                    child,
                    status: "running".into(),
                    finished: false,
                });
            }
            Err(error) => self.message = Some(error),
        }
        cx.notify();
    }

    /// Opens the match report of the save at `path` (task 117): the save is read off the UI
    /// thread under the game's rules.
    fn show_report(&mut self, path: &std::path::Path, window: &mut Window, cx: &mut Context<Self>) {
        let path = path.to_owned();
        self.report = None;
        self.report_from = self.screen;
        self.go(Screen::Report, window, cx);
        cx.spawn(async move |this, cx| {
            let report = cx
                .background_executor()
                .spawn(async move { report_of(&path) })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.report = Some(report.map(std::sync::Arc::new));
                cx.notify();
            });
        })
        .detach();
    }

    /// Continues the newest save this game can load.
    fn continue_latest(&mut self, cx: &mut Context<Self>) {
        self.refresh_saves();
        match self
            .saves
            .iter()
            .find(|e| e.refusal.is_none())
            .map(|e| e.path.clone())
        {
            Some(path) => self.load_save(&path, cx),
            None => {
                self.message = Some("There is no saved game to continue.".into());
                cx.notify();
            }
        }
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
}

impl Launcher {
    fn go(&mut self, screen: Screen, window: &mut Window, cx: &mut Context<Self>) {
        self.screen = screen;
        self.message = None;
        window.focus(&self.focus);
        cx.notify();
    }

    fn simple_change(&mut self, key: &str, value: String, cx: &mut Context<Self>) {
        self.config.simple.set(key, value);
        self.save();
        cx.notify();
    }

    /// Moves a count of the simple forms by one (a stepper's − or +).
    fn simple_step(&mut self, key: &str, delta: i64, cx: &mut Context<Self>) {
        self.config.simple.step(key, delta);
        self.save();
        cx.notify();
    }

    /// Chooses a nation (or, with a blank key, the default mix) and shows it in the dropdown of
    /// its race.
    fn choose_nation(&mut self, key: String, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(race) = self
            .catalog
            .races
            .iter()
            .find(|r| r.nations.iter().any(|n| n.key == key))
            && let Some(select) = self.nation_selects.get(&race.key).cloned()
        {
            select.update(cx, |select, cx| select.set_selected_value(&key, window, cx));
        }
        self.simple_change("race", key, cx);
    }

    /// Creates the simple forms' text fields and the nation dropdowns of the races with many
    /// nations, from the remembered values.
    fn prepare_simple(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.simple_inputs.clear();
        self.nation_selects.clear();
        self.simple_subscriptions.clear();
        for key in TEXT_FIELDS {
            let input = cx.new(|cx| {
                let input = InputState::new(window, cx)
                    .default_value(self.config.simple.get(key).to_owned());
                if key == "seed" {
                    input.placeholder("Random")
                } else {
                    input
                }
            });
            self.simple_subscriptions.push(cx.subscribe_in(
                &input,
                window,
                move |this, input, event: &InputEvent, _, cx| {
                    if matches!(event, InputEvent::Change) {
                        let value = input.read(cx).value().to_string();
                        this.simple_change(key, value, cx);
                    }
                },
            ));
            self.simple_inputs.insert(key, input);
        }
        let chosen = self.config.simple.get("race").to_owned();
        for race in &self.catalog.races {
            if race.nations.len() <= SEGMENTS {
                continue;
            }
            let selected = race
                .nations
                .iter()
                .position(|c| c.key == chosen)
                .unwrap_or(0);
            let choices = race.nations.clone();
            let state = cx.new(|cx| {
                SelectState::new(
                    SearchableVec::new(choices),
                    Some(gpui_component::IndexPath::default().row(selected)),
                    window,
                    cx,
                )
            });
            self.simple_subscriptions.push(cx.subscribe_in(
                &state,
                window,
                move |this, _, event: &SelectEvent<SearchableVec<Choice>>, _, cx| {
                    if let SelectEvent::Confirm(Some(value)) = event {
                        this.simple_change("race", value.clone(), cx);
                    }
                },
            ));
            self.nation_selects.insert(race.key.clone(), state);
        }
        self.prepare_setup(window, cx);
    }

    /// The forms' starting age and setup script dropdowns, from the remembered values.
    fn prepare_setup(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mut ages = vec![Choice {
            key: String::new(),
            label: "The first age".into(),
        }];
        ages.extend(self.catalog.lists["age"].iter().cloned());
        let mut scripts_list = vec![Choice {
            key: String::new(),
            label: "None".into(),
        }];
        scripts_list.extend(self.script_choices(self.config.simple.get("script")));
        for (key, list) in [("age", ages), ("script", scripts_list)] {
            let selected = list
                .iter()
                .position(|c| c.key == self.config.simple.get(key))
                .unwrap_or(0);
            let state = cx.new(|cx| {
                SelectState::new(
                    SearchableVec::new(list),
                    Some(gpui_component::IndexPath::default().row(selected)),
                    window,
                    cx,
                )
            });
            self.simple_subscriptions.push(cx.subscribe_in(
                &state,
                window,
                move |this, _, event: &SelectEvent<SearchableVec<Choice>>, _, cx| {
                    if let SelectEvent::Confirm(Some(value)) = event {
                        this.simple_change(key, value.clone(), cx);
                    }
                },
            ));
            if key == "age" {
                self.age_select = Some(state);
            } else {
                self.script_select = Some(state);
            }
        }
    }

    /// A seed for a blank seed field; the command line it makes shows it.
    fn random_seed() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64
            % 1_000_000_000
    }

    /// What the form of `mode` would start, or why it cannot.
    fn simple_check(&self, mode: Mode) -> Result<Vec<String>, String> {
        let seed = Self::random_seed();
        let form = self.config.simple.form(mode, &self.catalog.mix, seed)?;
        form.validate()?;
        Ok(form.arguments())
    }

    /// The final button of a form: starts the game or server it describes.
    fn start(&mut self, mode: Mode, cx: &mut Context<Self>) {
        if self
            .last_start
            .is_some_and(|at| at.elapsed() < Duration::from_secs(1))
        {
            return;
        }
        match self.simple_check(mode) {
            Ok(mut arguments) => {
                if arguments == ["--run"] {
                    arguments.clear();
                }
                // The Sound settings go with every match (task 125).
                if mode != Mode::Serve {
                    arguments.extend(self.config.form.sound_arguments());
                }
                arguments.extend(self.append.iter().cloned());
                if arguments.is_empty() {
                    arguments.push("--run".into());
                }
                self.last_start = Some(Instant::now());
                self.message = None;
                self.spawn_game(arguments, cx);
            }
            Err(error) => {
                self.message = Some(error);
                cx.notify();
            }
        }
    }

    /// Joins with the seat token remembered for the typed address.
    fn rejoin(&mut self, cx: &mut Context<Self>) {
        let url = self.config.simple.get("url").trim().to_owned();
        let Some(token) = self.config.tokens.get(&url).cloned() else {
            return;
        };
        match self
            .config
            .simple
            .rejoin_form(&token)
            .and_then(|form| form.validate().map(|_| form.arguments()))
        {
            Ok(mut arguments) => {
                arguments.extend(self.config.form.sound_arguments());
                self.spawn_game(arguments, cx)
            }
            Err(error) => {
                self.message = Some(error);
                cx.notify();
            }
        }
    }

    /// The setup scripts to choose from: every bundled one by title, and `chosen` when it is a
    /// file of the person's.
    fn script_choices(&self, chosen: &str) -> Vec<Choice> {
        let mut list: Vec<Choice> = scripts::BUNDLED
            .iter()
            .map(|(name, text)| Choice {
                key: scripts::path(&self.path, name)
                    .to_string_lossy()
                    .into_owned(),
                label: scripts::about(text, name).title,
            })
            .collect();
        if !chosen.is_empty() && !list.iter().any(|c| c.key == chosen) {
            let label = scripts::describe(&self.path, chosen)
                .map_or_else(|_| chosen.to_owned(), |about| about.title);
            list.push(Choice {
                key: chosen.to_owned(),
                label,
            });
        }
        list
    }

    /// Every preset tile: the bundled ones, then the person's own.
    fn all_presets(&self) -> impl Iterator<Item = &Preset> {
        self.bundled.iter().chain(&self.config.play_presets)
    }

    fn find_preset(&self, name: &str) -> Option<Preset> {
        self.all_presets().find(|p| p.name == name).cloned()
    }

    /// Chooses a preset tile; Play starts it.
    fn choose_preset(&mut self, name: String, cx: &mut Context<Self>) {
        self.config.chosen = name;
        self.message = None;
        self.save();
        cx.notify();
    }

    /// The home screen's Play: starts the chosen preset with the person's own race.
    fn play_chosen(&mut self, cx: &mut Context<Self>) {
        let Some(preset) = self.find_preset(&self.config.chosen) else {
            self.message = Some("Choose a match first.".into());
            cx.notify();
            return;
        };
        self.config.simple = preset.apply(&self.config.simple, &self.path);
        self.save();
        self.start(Mode::Play, cx);
    }

    /// Opens the Play form with the chosen preset in it, to change before starting.
    fn customise(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(preset) = self.find_preset(&self.config.chosen) {
            self.config.simple = preset.apply(&self.config.simple, &self.path);
            self.save();
            self.prepare_simple(window, cx);
        }
        self.go(Screen::Form(Mode::Play), window, cx);
    }

    /// Saves the Play form's match as the person's own preset tile.
    fn save_play_preset(&mut self, cx: &mut Context<Self>) {
        let name = self.play_preset_name.read(cx).value().trim().to_owned();
        if name.is_empty() {
            self.message = Some("Give the preset a name first.".into());
        } else if self.bundled.iter().any(|p| p.name == name) {
            self.message = Some(format!(
                "\"{name}\" is a preset of the game; pick another name."
            ));
        } else {
            let style = self
                .catalog
                .styles
                .iter()
                .find(|s| s.key == self.config.simple.get("style"))
                .map_or("", |s| s.name.as_str());
            let line = match self.config.simple.get("opponents") {
                "0" => format!("{style}, no opponents."),
                "1" => format!("{style}, one opponent."),
                n => format!("{style}, {n} opponents."),
            };
            let preset = Preset::of(&self.config.simple, &name, &line);
            self.config.play_presets.retain(|p| p.name != name);
            self.config.play_presets.push(preset);
            self.config.chosen = name.clone();
            self.message = Some(format!("Saved \"{name}\" with the presets."));
            self.save();
        }
        cx.notify();
    }

    /// Removes one of the person's own presets.
    fn remove_play_preset(&mut self, name: &str, cx: &mut Context<Self>) {
        self.config.play_presets.retain(|p| p.name != name);
        if self.config.chosen == name {
            self.config.chosen = self
                .bundled
                .first()
                .map(|p| p.name.clone())
                .unwrap_or_default();
        }
        self.save();
        cx.notify();
    }

    /// Writes every preset (the game's and the person's) to a file the person picks.
    fn export_presets(&mut self, cx: &mut Context<Self>) {
        let folder = self.path.parent().map(PathBuf::from).unwrap_or_default();
        let chosen = cx.prompt_for_new_path(&folder, Some("presets.json"));
        let text = presets::to_json(&self.all_presets().cloned().collect::<Vec<_>>());
        cx.spawn(async move |this, cx| {
            let picked = chosen.await;
            let _ = this.update(cx, |this, cx| {
                this.message = match picked {
                    Ok(Ok(Some(path))) => Some(match std::fs::write(&path, &text) {
                        Ok(()) => format!("Presets written to {}", path.display()),
                        Err(e) => format!("Could not write {}: {e}", path.display()),
                    }),
                    Ok(Ok(None)) | Err(_) => None,
                    Ok(Err(e)) => Some(format!("No file dialog here ({e}).")),
                };
                cx.notify();
            });
        })
        .detach();
    }

    /// Adds the presets of a file the person picks as tiles (names already used are skipped).
    fn import_presets(&mut self, cx: &mut Context<Self>) {
        let chosen = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Import presets".into()),
        });
        cx.spawn(async move |this, cx| {
            let picked = chosen.await;
            let _ = this.update(cx, |this, cx| {
                if let Ok(Ok(Some(paths))) = picked
                    && let Some(path) = paths.first()
                {
                    let read = std::fs::read_to_string(path)
                        .map_err(|e| e.to_string())
                        .and_then(|text| presets::parse(&text));
                    this.message = Some(match read {
                        Ok(list) => {
                            let mut added = 0;
                            for preset in list {
                                if this.find_preset(&preset.name).is_none() {
                                    this.config.play_presets.push(preset);
                                    added += 1;
                                }
                            }
                            this.save();
                            format!("Imported {added} presets from {}", path.display())
                        }
                        Err(e) => format!("{}: {e}", path.display()),
                    });
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Lets the person pick a script file for the forms' setup.
    fn pick_script(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let chosen = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Use this script".into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            let picked = chosen.await;
            let _ = this.update_in(cx, |this, window, cx| {
                match picked {
                    Ok(Ok(Some(paths))) => {
                        if let Some(path) = paths.first() {
                            this.config
                                .simple
                                .set("script", path.to_string_lossy().into_owned());
                            this.save();
                            this.prepare_setup(window, cx);
                        }
                    }
                    Ok(Err(e)) => this.message = Some(format!("No file dialog here ({e}).")),
                    _ => {}
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Opens the advanced launcher with the options this form stands for.
    fn open_in_advanced(&mut self, mode: Mode, window: &mut Window, cx: &mut Context<Self>) {
        let seed = Self::random_seed();
        match self.config.simple.form(mode, &self.catalog.mix, seed) {
            Ok(form) => {
                self.group = 0;
                self.replace(form, window, cx);
                self.go(Screen::Advanced, window, cx);
            }
            Err(error) => {
                self.message = Some(error);
                cx.notify();
            }
        }
    }
}

impl Render for Launcher {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if std::mem::take(&mut self.stale_fields) {
            self.prepare(window, cx);
        }
        let body = match self.screen {
            Screen::Home => self.render_home(cx),
            Screen::Form(mode) => self.render_form(mode, window, cx),
            Screen::Advanced => self.render_advanced(window, cx),
            Screen::Report => self.render_report(cx),
        };
        div()
            .id("launcher-root")
            .key_context(CONTEXT)
            .track_focus(&self.focus)
            .on_action(cx.listener(|this, _: &Back, window, cx| {
                if matches!(this.screen, Screen::Form(_)) {
                    this.go(Screen::Home, window, cx);
                } else if this.screen == Screen::Report {
                    this.go(this.report_from, window, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &Confirm, _, cx| {
                if let Screen::Form(mode) = this.screen
                    && this.simple_check(mode).is_ok()
                {
                    this.start(mode, cx);
                }
            }))
            .size_full()
            .child(body)
    }
}

/// The match report of a save, named from its seats (task 117).
fn report_of(path: &std::path::Path) -> Result<rom_match::MatchReport, String> {
    let rules = std::sync::Arc::new(rom_match::load_rules()?);
    let (world, info, _) = rom_match::load_match(path, &rules)?;
    Ok(rom_match::MatchReport::of(&world, &info))
}
