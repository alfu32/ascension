//! The server window (task 88, "Run a server"): a game server without a game, and a GPUI window
//! that shows its address, the keys to hand out (Copy, Add a key), the seats and their states,
//! the match's tick and speed, and the server's log, with Start now, Pause, Resume, Speed, Save
//! and Stop. Closing the window saves and stops the server.

use std::process::ExitCode;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use gpui::{
    App, AppContext, Application, ClipboardItem, Context, Entity, InteractiveElement, IntoElement,
    ParentElement, Render, SharedString, StatefulInteractiveElement, Styled, TitlebarOptions,
    Window, WindowBounds, WindowOptions, div, px, rgb, size,
};
use gpui_component::{
    Disableable, Root, Sizable,
    button::{Button, ButtonVariants},
    input::{Input, InputState},
};
use rom_net::{Operator, Server, ServerHandle, Status};

use super::session::{data_folder, join_address, server_config};
use crate::args::Options;

const GOLD: u32 = 0xd8b26a;
const MUTED: u32 = 0x8d98a8;
const PANEL: u32 = 0x141b24;
const LINE: u32 = 0x2a3340;
const WARN: u32 = 0xeb6e64;

/// Lines of the log the window keeps.
const KEPT_LINES: usize = 300;

/// What the window shows of the server.
#[derive(Default)]
struct Shared {
    handle: Option<ServerHandle>,
    keys: Vec<String>,
    log: Vec<String>,
    error: Option<String>,
    /// Names of the races and nations, by key.
    names: std::collections::BTreeMap<String, String>,
}

/// Runs the server and its window until the window closes (or, in scripted runs, until
/// `ROM_NET_QUIT_AFTER` seconds have passed).
pub fn run(options: &Options) -> ExitCode {
    let application = match std::panic::catch_unwind(Application::new) {
        Ok(application) => application,
        Err(_) => {
            eprintln!("rise-of-man: could not initialise the windowing system");
            return ExitCode::FAILURE;
        }
    };
    let shared = Arc::new(Mutex::new(Shared::default()));
    let exit_code = Arc::new(AtomicU8::new(0));
    start_server(options, &shared);
    let app_shared = Arc::clone(&shared);
    let app_exit = Arc::clone(&exit_code);
    let options = options.clone();
    application.run(move |cx: &mut App| {
        gpui_component::init(cx);
        gpui_component::Theme::change(gpui_component::ThemeMode::Dark, None, cx);
        {
            let theme = gpui_component::Theme::global_mut(cx);
            theme.colors.background = gpui::rgb(0x0c1118).into();
            theme.colors.foreground = gpui::rgb(0xf2f2f2).into();
            theme.colors.primary = gpui::rgb(0x3fd0ff).into();
            theme.colors.border = gpui::rgb(0x2a3340).into();
            theme.font_family = "DejaVu Sans".into();
        }
        let title = format!(
            "{} · Server :{}",
            crate::args::window_title(),
            options.host.unwrap_or(7400)
        );
        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(1000.), px(760.)), cx)),
            titlebar: Some(TitlebarOptions {
                title: Some(title.clone().into()),
                ..Default::default()
            }),
            app_id: Some("rise-of-man-server".into()),
            ..Default::default()
        };
        let opened = cx.open_window(window_options, move |window, cx| {
            window.set_window_title(&title);
            let view = cx.new(|cx| ServerView::new(options, app_shared, window, cx));
            cx.new(|cx| Root::new(view, window, cx))
        });
        if let Err(error) = opened {
            eprintln!("rise-of-man: could not open the server window: {error:#}");
            app_exit.store(1, Ordering::Relaxed);
            cx.spawn(async |cx| {
                let _ = cx.update(|cx| cx.quit());
            })
            .detach();
            return;
        }
        cx.activate(true);
    });
    // Closing the window saves and stops the server.
    let handle = shared.lock().ok().and_then(|mut s| s.handle.take());
    if let Some(handle) = handle {
        handle.stop();
    }
    if shared.lock().is_ok_and(|s| s.error.is_some()) {
        exit_code.store(1, Ordering::Relaxed);
    }
    ExitCode::from(exit_code.load(Ordering::Relaxed))
}

/// Starts the server on its own thread (resuming a match builds it, which takes seconds).
fn start_server(options: &Options, shared: &Arc<Mutex<Shared>>) {
    let options = options.clone();
    let thread_shared = Arc::clone(shared);
    let started = std::thread::Builder::new()
        .name("rom-net start".into())
        .spawn(move || {
            let shared = thread_shared;
            let log_shared = Arc::clone(&shared);
            let log: rom_net::Log = Arc::new(move |line: &str| {
                println!("server: {line}");
                if let Ok(mut s) = log_shared.lock() {
                    s.log.push(line.to_owned());
                    let over = s.log.len().saturating_sub(KEPT_LINES);
                    s.log.drain(..over);
                }
            });
            if let Ok(rules) = crate::terrain::load_rules()
                && let Ok(mut s) = shared.lock()
            {
                s.names = rules
                    .factions
                    .keys()
                    .map(|id| (rules.key(*id).to_owned(), rules.def(*id).name.clone()))
                    .collect();
            }
            let result = data_folder("server")
                .and_then(|dir| server_config(&options, options.remote_players, dir, log))
                .and_then(|config| Server::start(config).map_err(|e| e.to_string()))
                .and_then(|handle| {
                    let keys = handle
                        .add_keys(options.remote_players, false)
                        .map_err(|e| e.to_string())?;
                    Ok((handle, keys))
                });
            if let Ok(mut s) = shared.lock() {
                match result {
                    Ok((handle, keys)) => {
                        for key in &keys {
                            println!("key: {key}");
                        }
                        if let Some(path) = std::env::var_os("ROM_NET_KEYS") {
                            let _ = std::fs::write(path, keys.join("\n") + "\n");
                        }
                        s.handle = Some(handle);
                        s.keys = keys;
                    }
                    Err(error) => {
                        eprintln!("rise-of-man: the server did not start: {error}");
                        s.error = Some(format!("The server did not start: {error}"));
                    }
                }
            }
        });
    if let Err(error) = started
        && let Ok(mut s) = shared.lock()
    {
        s.error = Some(format!("could not start the server: {error}"));
    }
}

/// The server window's view.
struct ServerView {
    options: Options,
    shared: Arc<Mutex<Shared>>,
    status: Option<Status>,
    speed: Entity<InputState>,
    copied: Option<(usize, Instant)>,
    stopped: bool,
}

impl ServerView {
    fn new(
        options: Options,
        shared: Arc<Mutex<Shared>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let speed = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("percent")
                .default_value(options.speed_pct.to_string())
        });
        let quit_after = std::env::var("ROM_NET_QUIT_AFTER")
            .ok()
            .and_then(|s| s.parse::<f64>().ok())
            .filter(|s| s.is_finite() && *s > 0.0)
            .map(Duration::from_secs_f64);
        let opened = Instant::now();
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(400))
                    .await;
                if this.update(cx, |view, cx| view.poll(cx)).is_err() {
                    break;
                }
                if quit_after.is_some_and(|after| opened.elapsed() >= after) {
                    let _ = cx.update(|cx| cx.quit());
                    break;
                }
            }
        })
        .detach();
        Self {
            options,
            shared,
            status: None,
            speed,
            copied: None,
            stopped: false,
        }
    }

    fn poll(&mut self, cx: &mut Context<Self>) {
        let status = self
            .shared
            .lock()
            .ok()
            .and_then(|s| s.handle.as_ref().map(ServerHandle::status));
        let finished = self
            .shared
            .lock()
            .is_ok_and(|s| s.handle.as_ref().is_some_and(ServerHandle::is_finished));
        if finished && !self.stopped {
            self.stopped = true;
        }
        self.status = status;
        cx.notify();
    }

    fn command(&self, operator: Operator) {
        if let Ok(s) = self.shared.lock()
            && let Some(handle) = &s.handle
        {
            handle.command(operator);
        }
    }

    fn add_key(&self) {
        if let Ok(mut s) = self.shared.lock() {
            let added = s.handle.as_ref().map(|h| h.add_keys(1, false));
            match added {
                Some(Ok(keys)) => {
                    for key in &keys {
                        println!("key: {key}");
                    }
                    s.keys.extend(keys);
                }
                Some(Err(error)) => s.log.push(format!("could not add a key: {error}")),
                None => {}
            }
        }
    }

    fn set_speed(&self, cx: &mut Context<Self>) {
        let text = self.speed.read(cx).value().trim().to_owned();
        match text.parse::<u32>() {
            Ok(pct) if (1..=10_000).contains(&pct) => self.command(Operator::Speed(pct)),
            _ => {
                if let Ok(mut s) = self.shared.lock() {
                    s.log.push(format!(
                        "speed: a whole number from 1 to 10000, not {text:?}"
                    ));
                }
            }
        }
    }
}

fn heading(text: &str) -> gpui::Div {
    div()
        .text_color(rgb(GOLD))
        .text_sm()
        .child(SharedString::from(text.to_owned()))
}

fn panel() -> gpui::Div {
    div()
        .p_4()
        .bg(rgb(PANEL))
        .border_1()
        .border_color(rgb(LINE))
        .rounded(px(6.))
        .flex()
        .flex_col()
        .gap_2()
}

impl Render for ServerView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (keys, log, error, names) = self
            .shared
            .lock()
            .map(|s| {
                (
                    s.keys.clone(),
                    s.log.clone(),
                    s.error.clone(),
                    s.names.clone(),
                )
            })
            .unwrap_or_default();
        let port = self.options.host.unwrap_or(7400);
        let address = join_address(&self.options.bind, port);
        let status = self.status.clone();
        let running = status.as_ref().is_some_and(|s| s.running);
        let paused = status.as_ref().is_some_and(|s| s.paused_by.is_some());
        let playing = status.as_ref().is_some_and(|s| s.phase == "playing");
        let summary = match &status {
            Some(s) => {
                let mut line = format!(
                    "{} · tick {} · speed {} % · {} connection(s) · keys {} unused, {} used",
                    s.phase, s.tick, s.speed, s.connections, s.keys.unused, s.keys.used
                );
                if let Some(by) = &s.paused_by {
                    line.push_str(&format!(" · paused by {by}"));
                }
                if !s.waiting_for.is_empty() {
                    line.push_str(&format!(" · waiting for {}", s.waiting_for.join(", ")));
                }
                if s.hash_checks > 0 {
                    line.push_str(&format!(
                        " · {} hash checks, {} out of step",
                        s.hash_checks, s.desyncs
                    ));
                }
                if let Some(save) = &s.last_save {
                    line.push_str(&format!(" · last save {save}"));
                }
                line
            }
            None if error.is_some() => "not running".to_owned(),
            None => "starting …".to_owned(),
        };
        let mut page = div()
            .id("server")
            .size_full()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_4()
            .p_6()
            .bg(rgb(0x0c1118))
            .text_color(rgb(0xf2f2f2))
            .text_size(px(14.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(div().text_xl().text_color(rgb(GOLD)).child("GAME SERVER"))
                    .child(SharedString::from(format!(
                        "Listening on {}:{port} · players join {address}",
                        self.options.bind
                    )))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(MUTED))
                            .child(SharedString::from(summary)),
                    ),
            );
        if let Some(error) = error {
            page = page.child(
                panel()
                    .border_color(rgb(WARN))
                    .child(div().text_color(rgb(WARN)).child(SharedString::from(error))),
            );
        }
        // Keys.
        let mut key_rows = div().flex().flex_col().gap_1();
        for (index, key) in keys.iter().enumerate() {
            let copied = self
                .copied
                .is_some_and(|(i, at)| i == index && at.elapsed() < Duration::from_secs(2));
            let text = key.clone();
            key_rows = key_rows.child(
                div()
                    .flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .font_family("DejaVu Sans Mono")
                            .text_sm()
                            .child(SharedString::from(key.clone())),
                    )
                    .child(
                        Button::new(("copy-key", index))
                            .small()
                            .label(if copied { "Copied" } else { "Copy" })
                            .on_click(cx.listener(move |view, _, _, cx| {
                                cx.write_to_clipboard(ClipboardItem::new_string(text.clone()));
                                view.copied = Some((index, Instant::now()));
                                cx.notify();
                            })),
                    ),
            );
        }
        let keys_panel = panel()
            .flex_1()
            .child(heading("KEYS TO HAND OUT"))
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(MUTED))
                    .child("Each key admits one person once."),
            )
            .child(key_rows)
            .child(
                Button::new("add-key")
                    .small()
                    .label("Add a key")
                    .on_click(cx.listener(|view, _, _, cx| {
                        view.add_key();
                        cx.notify();
                    })),
            );
        // Controls.
        let controls = panel()
            .w(px(420.))
            .child(heading("MATCH"))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(
                        Button::new("start")
                            .primary()
                            .label("Start now")
                            .disabled(playing || self.stopped)
                            .on_click(cx.listener(|view, _, _, _| view.command(Operator::Start))),
                    )
                    .child(
                        Button::new("pause")
                            .label("Pause")
                            .disabled(!running)
                            .on_click(cx.listener(|view, _, _, _| view.command(Operator::Pause))),
                    )
                    .child(
                        Button::new("resume")
                            .label("Resume")
                            .disabled(!paused)
                            .on_click(cx.listener(|view, _, _, _| view.command(Operator::Resume))),
                    )
                    .child(
                        Button::new("save")
                            .label("Save")
                            .disabled(!playing)
                            .on_click(
                                cx.listener(|view, _, _, _| view.command(Operator::Save(None))),
                            ),
                    )
                    .child(
                        Button::new("stop")
                            .label("Stop")
                            .disabled(self.stopped)
                            .on_click(cx.listener(|view, _, _, cx| {
                                view.command(Operator::Stop);
                                view.stopped = true;
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .items_center()
                    .child("Speed")
                    .child(div().w(px(90.)).child(Input::new(&self.speed).small()))
                    .child("%")
                    .child(
                        Button::new("speed")
                            .small()
                            .label("Set")
                            .on_click(cx.listener(|view, _, _, cx| view.set_speed(cx))),
                    ),
            );
        // Seats.
        let mut seats = div().flex().flex_col().gap_1().text_sm();
        if let Some(status) = &status {
            for seat in &status.seats {
                seats = seats.child(
                    div()
                        .flex()
                        .gap_2()
                        .py_1()
                        .border_t_1()
                        .border_color(rgb(LINE))
                        .child(
                            div()
                                .w(px(50.))
                                .child(SharedString::from(format!("{}", seat.seat + 1))),
                        )
                        .child(
                            div()
                                .w(px(240.))
                                .child(SharedString::from(seat.name.clone())),
                        )
                        .child(
                            div().w(px(200.)).child(SharedString::from(
                                names
                                    .get(&seat.faction)
                                    .cloned()
                                    .unwrap_or_else(|| seat.faction.clone()),
                            )),
                        )
                        .child(div().w(px(120.)).child(SharedString::from(
                            format!("{:?}", seat.state).to_lowercase(),
                        )))
                        .child(
                            div()
                                .w(px(80.))
                                .child(if seat.ready { "ready" } else { "" }),
                        ),
                );
            }
        }
        let mut log_lines = div()
            .flex()
            .flex_col()
            .text_xs()
            .font_family("DejaVu Sans Mono");
        let shown = log.len().saturating_sub(14);
        for line in &log[shown..] {
            log_lines = log_lines.child(SharedString::from(line.clone()));
        }
        page.child(div().flex().gap_4().child(keys_panel).child(controls))
            .child(panel().child(heading("SEATS")).child(seats))
            .child(panel().child(heading("LOG")).child(log_lines))
    }
}
