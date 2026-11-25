//! The in-match menu: resume, save game, load game, quit. It is a GPUI overlay on the world
//! view (the in-match HUD itself is drawn by raylib); opening it holds the simulation.

use gpui::{
    AnyElement, AppContext, Context, Entity, InteractiveElement, IntoElement, MouseButton,
    ParentElement, SharedString, StatefulInteractiveElement, Styled, Subscription, Window, div, px,
    rgb, rgba,
};
use gpui_component::{
    Disableable, Selectable, Sizable,
    button::{Button, ButtonVariants},
    input::{Input, InputEvent, InputState},
    slider::{Slider, SliderEvent, SliderState},
};
use rom_render::InputEvent as WorldInput;

use super::WorldView;
use crate::saves::{self, Entry, Kind};
use crate::sim_thread::{EndNotice, MenuPage};
use std::sync::Arc;

const GOLD: u32 = 0xe8c98a;
const AMBER: u32 = 0xf0b030;
const CYAN: u32 = 0x3fd0ff;
const MUTED: u32 = 0x9fb0c3;
/// The width of the match report in the menu.
const REPORT_WIDTH: f32 = 1280.;

/// Which page of the menu shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Page {
    Main,
    Save,
    Load,
    /// The match has ended (task 117): the modal end dialog.
    Ended,
    /// The match report, from the end dialog.
    Report,
}

/// The open menu.
pub(super) struct Menu {
    page: Page,
    /// Whether the simulation was held before the menu opened.
    was_paused: bool,
    status: Option<String>,
    /// A save was asked for and its outcome is awaited.
    pending: bool,
    name: Option<Entity<InputState>>,
    entries: Vec<Entry>,
    /// The end of the match, once the dialog has shown it.
    end: Option<Arc<EndNotice>>,
    _subscription: Option<Subscription>,
    /// The mixer's sliders (task 133): option name, label and slider.
    mixer: Vec<(&'static str, &'static str, Entity<SliderState>)>,
    _mixer_subscriptions: Vec<Subscription>,
}

impl WorldView {
    /// Whether the menu covers the world (input then goes to it, not to the renderer).
    pub(super) fn menu_open(&self) -> bool {
        self.menu.is_some()
    }

    /// Once per poll: opens the menu when the HUD asked for it and reads finished saves.
    pub(super) fn pump(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(page) = self.sim.take_menu_request() {
            self.open_menu(window, cx);
            match page {
                MenuPage::Main => {}
                MenuPage::Save => self.show_save(window, cx),
                MenuPage::Load => self.show_load(None, cx),
            }
        }
        if let Some(end) = self.sim.take_end() {
            self.show_end(end, window, cx);
        }
        while let Some(outcome) = self.sim.take_save_outcome() {
            let waiting = self.menu.as_ref().is_some_and(|m| m.pending);
            if !waiting {
                continue;
            }
            match outcome {
                Ok(_) => self.close_menu(window, cx),
                Err(error) => {
                    if let Some(menu) = self.menu.as_mut() {
                        menu.pending = false;
                        menu.status = Some(format!("Could not save: {error}"));
                    }
                    cx.notify();
                }
            }
        }
    }

    /// Opens the menu and holds the simulation until it closes.
    pub(super) fn open_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.menu.is_some() {
            return;
        }
        self.bridge.push_input(WorldInput::ReleaseAll);
        let (mixer, subscriptions) = self.mixer_sliders(window, cx);
        self.menu = Some(Menu {
            page: Page::Main,
            was_paused: self.sim.paused(),
            status: None,
            pending: false,
            name: None,
            entries: Vec::new(),
            end: None,
            _subscription: None,
            mixer,
            _mixer_subscriptions: subscriptions,
        });
        // A network match is everyone's: the menu does not hold it (the pause button does).
        if self.sim.network().is_none() {
            self.sim.set_paused(true);
        }
        window.focus(&self.focus);
        cx.notify();
    }

    /// The end of the match (task 117): the end dialog, over everything, until the person
    /// chooses; or straight on the report when asked to (`end-page report` in a script).
    fn show_end(&mut self, end: Arc<EndNotice>, window: &mut Window, cx: &mut Context<Self>) {
        self.open_menu(window, cx);
        if let Some(menu) = self.menu.as_mut() {
            menu.page = if end.open_report {
                Page::Report
            } else {
                Page::Ended
            };
            menu.status = None;
            menu.pending = false;
            menu.end = Some(end);
        }
        window.focus(&self.focus);
        cx.notify();
    }

    /// Keep playing after the end: the dialog closes and the world goes on.
    fn keep_playing(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.sim.keep_playing();
        if let Some(menu) = self.menu.as_mut() {
            menu.was_paused = false;
        }
        self.close_menu(window, cx);
    }

    /// Back to the launcher: this game process ends.
    fn exit_match(&mut self, cx: &mut Context<Self>) {
        self.sim.request_stop();
        cx.quit();
    }

    /// Closes the menu and releases the simulation unless it was held before.
    pub(super) fn close_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(menu) = self.menu.take()
            && self.sim.network().is_none()
        {
            self.sim.set_paused(menu.was_paused);
        }
        window.focus(&self.focus);
        cx.notify();
    }

    /// Escape: back to the main page, or close the menu from it. The end dialog stays until
    /// the person chooses; the report goes back to it.
    pub(super) fn menu_back(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.menu.as_mut() {
            Some(menu) if menu.page == Page::Ended => {}
            Some(menu) if menu.page == Page::Report => {
                menu.page = Page::Ended;
                window.focus(&self.focus);
                cx.notify();
            }
            Some(menu) if menu.page != Page::Main => {
                menu.page = Page::Main;
                menu.status = None;
                menu.pending = false;
                window.focus(&self.focus);
                cx.notify();
            }
            Some(_) => self.close_menu(window, cx),
            None => {}
        }
    }

    fn show_save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.sim.save_ui().is_none() {
            self.menu_status("This match cannot be saved.", cx);
            return;
        }
        let suggestion = self.sim.suggested_name();
        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Name this save")
                .default_value(suggestion)
        });
        let subscription =
            cx.subscribe_in(&input, window, |view, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    view.submit_save(window, cx);
                }
            });
        input.update(cx, |input, cx| input.focus(window, cx));
        if let Some(menu) = self.menu.as_mut() {
            menu.page = Page::Save;
            menu.status = None;
            menu.name = Some(input);
            menu._subscription = Some(subscription);
        }
        cx.notify();
    }

    fn submit_save(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let Some(menu) = self.menu.as_mut() else {
            return;
        };
        if menu.pending {
            return;
        }
        let name = menu
            .name
            .as_ref()
            .map_or_else(String::new, |n| n.read(cx).value().trim().to_owned());
        menu.pending = true;
        menu.status = Some("Saving…".into());
        self.sim.request_save(Kind::Manual, name);
        cx.notify();
    }

    fn show_load(&mut self, status: Option<String>, cx: &mut Context<Self>) {
        let entries = self
            .sim
            .save_ui()
            .map(|ui| saves::list(&ui.folder, &ui.rules))
            .unwrap_or_default();
        if let Some(menu) = self.menu.as_mut() {
            menu.page = Page::Load;
            menu.status = status.or_else(|| {
                entries
                    .is_empty()
                    .then(|| "There are no saved games yet.".into())
            });
            menu.entries = entries;
        }
        cx.notify();
    }

    fn menu_status(&mut self, text: &str, cx: &mut Context<Self>) {
        if let Some(menu) = self.menu.as_mut() {
            menu.status = Some(text.into());
        }
        cx.notify();
    }

    /// Continues the save at `path` in a fresh game process, then closes this one.
    fn load_game(&mut self, path: &std::path::Path, cx: &mut Context<Self>) {
        match saves::relaunch(path, &self.display_arguments) {
            Ok(_) => {
                self.sim.request_stop();
                cx.quit();
            }
            Err(error) => self.menu_status(&error, cx),
        }
    }

    /// F9: the newest quick save, or the load page when there is none.
    pub(super) fn quick_load(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.sim.network().is_some() {
            return;
        }
        self.open_menu(window, cx);
        let newest = self.sim.save_ui().and_then(|ui| {
            saves::list(&ui.folder, &ui.rules)
                .into_iter()
                .find(|e| e.kind == Kind::Quick && e.refusal.is_none())
        });
        match newest {
            Some(entry) => self.load_game(&entry.path, cx),
            None => self.show_load(Some("There is no quick save to load.".into()), cx),
        }
    }

    /// The overlay, when the menu is open.
    /// The rendering detail levels (task 108): a choice applied at once, during the match.
    fn detail_row(&self, cx: &mut Context<Self>) -> AnyElement {
        let current = self.bridge.detail();
        let mut row = div().flex().flex_row().items_center().gap_2().child(
            div()
                .text_color(rgb(MUTED))
                .child("Rendering detail (grass reach)"),
        );
        for level in rom_render::Detail::ALL {
            row = row.child(
                Button::new(SharedString::from(format!("menu-detail-{}", level.name())))
                    .label(level.name())
                    .small()
                    .selected(level == current)
                    .on_click(cx.listener(move |view, _, _, cx| {
                        view.bridge.publish_detail(level);
                        cx.notify();
                    })),
            );
        }
        row.into_any_element()
    }

    /// The mixer's sliders (task 133), one per category, at the settings in use.
    #[allow(clippy::type_complexity)]
    fn mixer_sliders(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> (
        Vec<(&'static str, &'static str, Entity<SliderState>)>,
        Vec<Subscription>,
    ) {
        let current = crate::sound::settings();
        let mut sliders = Vec::new();
        let mut subscriptions = Vec::new();
        for (flag, label) in crate::sound::SLIDERS {
            let value = current.get(flag).unwrap_or(100) as f32;
            let slider = cx.new(|_| {
                SliderState::new()
                    .min(0.)
                    .max(100.)
                    .step(1.)
                    .default_value(value)
            });
            subscriptions.push(cx.subscribe_in(
                &slider,
                window,
                move |_, _, event: &SliderEvent, _, cx| {
                    let SliderEvent::Change(value) = event;
                    let percent = value.end().round().clamp(0., 100.) as u32;
                    let mut settings = crate::sound::settings();
                    if settings.get(flag) != Some(percent) {
                        settings.set(flag, percent);
                        // Applied at once, and kept with the launcher's settings for the next
                        // game.
                        crate::sound::set_settings(settings);
                        if let Err(error) = crate::launcher::remember_sound(&settings) {
                            eprintln!("rise-of-man: could not keep the Sound settings: {error}");
                        }
                        cx.notify();
                    }
                },
            ));
            sliders.push((flag, label, slider));
        }
        (sliders, subscriptions)
    }

    /// The mixer (task 133): Master, Music, Units, Buildings, Environment and Announcements,
    /// each 0–100 %, applied at once and kept for the next game.
    fn sound_rows(&self, menu: &Menu) -> AnyElement {
        let current = crate::sound::settings();
        let mut rows = div()
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_color(rgb(MUTED)).child("Sound"));
        for (flag, label, slider) in &menu.mixer {
            let percent = current.get(flag).unwrap_or(100);
            rows = rows.child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_3()
                    .child(div().w(px(120.)).child(*label))
                    .child(div().flex_1().child(Slider::new(slider)))
                    .child(
                        div()
                            .w(px(48.))
                            .text_color(rgb(if percent == 0 { MUTED } else { GOLD }))
                            .child(if percent == 0 {
                                "off".to_owned()
                            } else {
                                format!("{percent} %")
                            }),
                    ),
            );
        }
        rows.into_any_element()
    }

    pub(super) fn render_menu(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let menu = self.menu.as_ref()?;
        let heading = |text: &str| {
            div()
                .text_xl()
                .text_color(rgb(GOLD))
                .child(SharedString::from(text.to_owned()))
        };
        let report = menu.page == Page::Report;
        let mut panel = div()
            .id("game-menu")
            .w(px(if report { REPORT_WIDTH + 40. } else { 560. }))
            .max_h(px(if report { 900. } else { 760. }))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_3()
            .p_5()
            .bg(rgb(0x141b24))
            .border_1()
            .border_color(rgb(0xb88a45))
            .rounded(px(6.))
            .text_color(rgb(0xf2f2f2))
            .text_size(px(14.));
        match menu.page {
            Page::Ended => {
                let end = menu.end.as_ref().expect("the end dialog has its end");
                let me = Some(end.me);
                panel = panel.child(heading("The match has ended"));
                for line in crate::report_view::end_lines(&end.report, me) {
                    panel = panel.child(div().text_color(rgb(0xf2f2f2)).child(line));
                }
                panel = panel
                    .child(div().text_xs().text_color(rgb(MUTED)).child(format!(
                        "Game time {}",
                        end.report.clock(end.report.length())
                    )))
                    .child(
                        Button::new("end-report")
                            .primary()
                            .label("Match report")
                            .on_click(cx.listener(|view, _, window, cx| {
                                if let Some(menu) = view.menu.as_mut() {
                                    menu.page = Page::Report;
                                }
                                window.focus(&view.focus);
                                cx.notify();
                            })),
                    );
                if end.can_keep_playing {
                    panel =
                        panel
                            .child(
                                Button::new("end-keep-playing")
                                    .label("Keep playing")
                                    .on_click(cx.listener(|view, _, window, cx| {
                                        view.keep_playing(window, cx)
                                    })),
                            )
                            .child(div().text_xs().text_color(rgb(MUTED)).child(
                                "The world goes on; the report stays as it was at the end. The \
                             Ascension is still open to you.",
                            ));
                }
                panel = panel.child(
                    Button::new("end-exit")
                        .label("Exit")
                        .on_click(cx.listener(|view, _, _, cx| view.exit_match(cx))),
                );
            }
            Page::Report => {
                let end = menu.end.as_ref().expect("the report has its end");
                panel = panel
                    .child(heading("MATCH REPORT"))
                    .child(crate::report_view::report_element(
                        &end.report,
                        Some(end.me),
                        REPORT_WIDTH,
                    ))
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(Button::new("report-back").label("Back").on_click(
                                cx.listener(|view, _, window, cx| view.menu_back(window, cx)),
                            ))
                            .child(
                                Button::new("report-exit")
                                    .label("Exit")
                                    .on_click(cx.listener(|view, _, _, cx| view.exit_match(cx))),
                            ),
                    );
            }
            Page::Main if self.sim.network().is_some() => {
                // A network match is saved by its server (task 88): the host asks it to; the
                // others are told how it saves; nothing loads into it.
                let host = self.sim.network().is_some_and(|n| n.host);
                panel = panel
                    .child(heading("GAME MENU"))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(MUTED))
                            .child(format!(
                                "Ascension — Rise of Man {} · network match",
                                crate::args::VERSION
                            )),
                    )
                    .child(
                        Button::new("menu-resume")
                            .primary()
                            .label("Resume")
                            .on_click(
                                cx.listener(|view, _, window, cx| view.close_menu(window, cx)),
                            ),
                    )
                    .child(self.detail_row(cx))
                    .child(self.sound_rows(menu))
                    .child(
                        Button::new("menu-save")
                            .label("Save game")
                            .disabled(!host)
                            .on_click(cx.listener(|view, _, window, cx| {
                                view.sim.request_save(Kind::Manual, "");
                                view.close_menu(window, cx);
                            })),
                    )
                    .child(div().text_xs().text_color(rgb(MUTED)).child(if host {
                        "Save asks the server to save the match now; it also saves every few minutes."
                    } else {
                        "The server saves the match every few minutes; the host can ask it to save now."
                    }))
                    .child(
                        Button::new("menu-load")
                            .label("Load game")
                            .disabled(true),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(MUTED))
                            .child("Loading is not available in a network match."),
                    )
                    .child(
                        Button::new("menu-quit")
                            .label("Leave the match")
                            .on_click(cx.listener(|view, _, _, cx| {
                                view.sim.request_stop();
                                cx.quit();
                            })),
                    );
            }
            Page::Main => {
                let saving = self.sim.save_ui().is_some();
                panel = panel
                    .child(heading("GAME MENU"))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(MUTED))
                            .child(format!("Ascension — Rise of Man {}", crate::args::VERSION)),
                    )
                    .child(
                        Button::new("menu-resume")
                            .primary()
                            .label("Resume")
                            .on_click(
                                cx.listener(|view, _, window, cx| view.close_menu(window, cx)),
                            ),
                    )
                    .child(self.detail_row(cx))
                    .child(self.sound_rows(menu))
                    .child(
                        Button::new("menu-save")
                            .label("Save game")
                            .disabled(!saving)
                            .on_click(
                                cx.listener(|view, _, window, cx| view.show_save(window, cx)),
                            ),
                    )
                    .child(
                        Button::new("menu-load")
                            .label("Load game")
                            .disabled(!saving)
                            .on_click(cx.listener(|view, _, _, cx| view.show_load(None, cx))),
                    )
                    .child(
                        Button::new("menu-quit")
                            .label("Quit game")
                            .on_click(cx.listener(|view, _, _, cx| {
                                view.sim.request_stop();
                                cx.quit();
                            })),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(MUTED))
                            .child("F5 quick save · F9 quick load · F8 rankings · Esc closes"),
                    );
            }
            Page::Save => {
                let folder = self
                    .sim
                    .save_ui()
                    .map_or_else(String::new, |ui| ui.folder.display().to_string());
                panel = panel
                    .child(heading("SAVE GAME"))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(div().text_color(rgb(GOLD)).child("Name"))
                            .children(menu.name.as_ref().map(Input::new)),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(MUTED))
                            .child(format!("Saved in {folder}")),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                Button::new("save-confirm")
                                    .primary()
                                    .label("Save")
                                    .disabled(menu.pending)
                                    .on_click(cx.listener(|view, _, window, cx| {
                                        view.submit_save(window, cx)
                                    })),
                            )
                            .child(Button::new("save-back").label("Back").on_click(
                                cx.listener(|view, _, window, cx| view.menu_back(window, cx)),
                            )),
                    );
            }
            Page::Load => {
                let mut list = div()
                    .id("load-list")
                    .flex()
                    .flex_col()
                    .gap_2()
                    .max_h(px(340.))
                    .overflow_y_scroll();
                for (index, entry) in menu.entries.iter().enumerate() {
                    let path = entry.path.clone();
                    let refused = entry.refusal.is_some();
                    list = list.child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .p_2()
                            .bg(rgb(0x0a0e14))
                            .rounded(px(4.))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .text_color(rgb(if refused { MUTED } else { CYAN }))
                                            .child(format!(
                                                "{} ({})",
                                                entry.name,
                                                entry.kind.label()
                                            )),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(rgb(MUTED))
                                            .child(entry.summary()),
                                    )
                                    .children(entry.refusal.as_ref().map(|why| {
                                        div().text_xs().text_color(rgb(AMBER)).child(why.clone())
                                    })),
                            )
                            .child(
                                Button::new(("load-entry", index))
                                    .small()
                                    .label("Load")
                                    .disabled(refused)
                                    .on_click(
                                        cx.listener(move |view, _, _, cx| {
                                            view.load_game(&path, cx)
                                        }),
                                    ),
                            ),
                    );
                }
                panel = panel.child(heading("LOAD GAME")).child(list).child(
                    div().text_xs().text_color(rgb(MUTED)).child(
                        "Loading starts the saved game in a new window, paused, and closes this one.",
                    ),
                ).child(
                    Button::new("load-back")
                        .label("Back")
                        .on_click(cx.listener(|view, _, window, cx| view.menu_back(window, cx))),
                );
            }
        }
        if let Some(status) = &menu.status {
            panel = panel.child(div().text_sm().text_color(rgb(AMBER)).child(status.clone()));
        }
        Some(
            div()
                .id("menu-backdrop")
                .absolute()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .bg(rgba(0x000000a8))
                .occlude()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
                .child(panel)
                .into_any_element(),
        )
    }
}
