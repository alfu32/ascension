//! The server window (task 88, "Run a server"): a game server without a game, and a GPUI window
//! that shows its address, the keys to hand out (Copy, Add a key), the seats and their states,
//! the match's tick and speed, and the server's log, with Start now, Pause, Resume, Speed, Save
//! and Stop. Closing the window saves and stops the server.

use std::process::ExitCode;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use gpui::prelude::FluentBuilder;
use gpui::{
    App, AppContext, Application, ClipboardItem, Context, Entity, InteractiveElement, IntoElement,
    ParentElement, Render, SharedString, StatefulInteractiveElement, Styled, TitlebarOptions,
    Window, WindowBounds, WindowOptions, div, px, rgb, size,
};
use gpui_component::{
    Root, Sizable,
    input::{Input, InputState},
};
use rom_net::{Operator, Server, ServerHandle, Status};

use super::session::{data_folder, join_address, server_config};
use crate::args::Options;
use crate::style::{self, Look};

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
        crate::style::install(cx);
        let title = format!(
            "{} · Server :{}",
            crate::args::window_title(),
            options.host.unwrap_or(7400)
        );
        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(1280.), px(800.)), cx)),
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
        let mut page =
            div()
                .flex()
                .flex_col()
                .gap(px(16.))
                .p(px(24.))
                .child(
                    div()
                        .flex()
                        .items_end()
                        .gap(px(16.))
                        .child(style::title("Game server"))
                        .child(style::hint(SharedString::from(crate::args::window_title()))),
                )
                .child(div().text_size(px(15.)).text_color(rgb(style::TEXT)).child(
                    SharedString::from(format!(
                        "Listening on {}:{port} · players join {address}",
                        self.options.bind
                    )),
                ))
                .child(style::status(summary, false));
        if let Some(error) = error {
            page = page.child(
                style::panel().child(
                    div()
                        .relative()
                        .text_color(rgb(style::BAD))
                        .child(SharedString::from(error)),
                ),
            );
        }
        // Keys.
        let mut key_rows = div().flex().flex_col().gap(px(6.));
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
                            .flex_1()
                            .min_w_0()
                            .font_family(style::MONO)
                            .text_size(px(13.))
                            .child(SharedString::from(key.clone())),
                    )
                    .child(
                        style::button(
                            ("copy-key", index),
                            if copied { "Copied" } else { "Copy" },
                            Look::Plain,
                            true,
                        )
                        .on_click(cx.listener(move |view, _, _, cx| {
                            cx.write_to_clipboard(ClipboardItem::new_string(text.clone()));
                            view.copied = Some((index, Instant::now()));
                            cx.notify();
                        })),
                    ),
            );
        }
        let keys_panel = style::panel().flex_1().child(
            div()
                .relative()
                .flex()
                .flex_col()
                .gap(px(10.))
                .child(style::heading("Keys to hand out"))
                .child(style::hint("Each key admits one person once."))
                .child(key_rows)
                .child(div().flex().child(
                    style::button("add-key", "Add a key", Look::Plain, true).on_click(cx.listener(
                        |view, _, _, cx| {
                            view.add_key();
                            cx.notify();
                        },
                    )),
                )),
        );
        // Controls.
        let can_start = !(playing || self.stopped);
        let can_save = playing;
        let can_stop = !self.stopped;
        let controls = style::panel().w(px(460.)).child(
            div()
                .relative()
                .flex()
                .flex_col()
                .gap(px(12.))
                .child(style::heading("Match"))
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap(px(8.))
                        .child(
                            style::button("start", "Start now", Look::Gold, can_start).when(
                                can_start,
                                |b| {
                                    b.on_click(
                                        cx.listener(|view, _, _, _| view.command(Operator::Start)),
                                    )
                                },
                            ),
                        )
                        .child(style::button("pause", "Pause", Look::Framed, running).when(
                            running,
                            |b| {
                                b.on_click(
                                    cx.listener(|view, _, _, _| view.command(Operator::Pause)),
                                )
                            },
                        ))
                        .child(
                            style::button("resume", "Resume", Look::Framed, paused).when(
                                paused,
                                |b| {
                                    b.on_click(
                                        cx.listener(|view, _, _, _| view.command(Operator::Resume)),
                                    )
                                },
                            ),
                        )
                        .child(style::button("save", "Save", Look::Framed, can_save).when(
                            can_save,
                            |b| {
                                b.on_click(
                                    cx.listener(|view, _, _, _| view.command(Operator::Save(None))),
                                )
                            },
                        ))
                        .child(style::button("stop", "Stop", Look::Plain, can_stop).when(
                            can_stop,
                            |b| {
                                b.on_click(cx.listener(|view, _, _, cx| {
                                    view.command(Operator::Stop);
                                    view.stopped = true;
                                    cx.notify();
                                }))
                            },
                        )),
                )
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .items_center()
                        .child(style::label("Speed"))
                        .child(div().w(px(90.)).child(Input::new(&self.speed).small()))
                        .child(style::hint("%"))
                        .child(
                            style::button("speed", "Set", Look::Plain, true)
                                .on_click(cx.listener(|view, _, _, cx| view.set_speed(cx))),
                        ),
                ),
        );
        // Seats.
        let header = |text: &str, width: f32| {
            div()
                .w(px(width))
                .text_size(px(11.))
                .font_weight(gpui::FontWeight::BOLD)
                .text_color(rgb(style::MUTED))
                .child(SharedString::from(text.to_uppercase()))
        };
        let mut seats = div().flex().flex_col().gap_1().text_size(px(14.)).child(
            div()
                .flex()
                .gap_2()
                .child(header("Seat", 50.))
                .child(header("Name", 240.))
                .child(header("Race or nation", 200.))
                .child(header("State", 120.))
                .child(header("Ready", 80.)),
        );
        if let Some(status) = &status {
            for seat in &status.seats {
                let state = format!("{:?}", seat.state).to_lowercase();
                let colour = match state.as_str() {
                    "connected" => style::GOOD,
                    "waiting" | "idle" => style::BAD,
                    _ => style::MUTED,
                };
                seats = seats.child(
                    div()
                        .flex()
                        .gap_2()
                        .py(px(6.))
                        .border_t_1()
                        .border_color(rgb(style::LINE))
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
                        .child(
                            div()
                                .w(px(120.))
                                .text_color(rgb(colour))
                                .child(SharedString::from(state)),
                        )
                        .child(
                            div()
                                .w(px(80.))
                                .text_color(rgb(style::GOOD))
                                .child(if seat.ready { "ready" } else { "" }),
                        ),
                );
            }
        }
        let mut log_lines = div()
            .flex()
            .flex_col()
            .text_size(px(12.))
            .font_family(style::MONO)
            .text_color(rgb(style::TEXT));
        let shown = log.len().saturating_sub(14);
        for line in &log[shown..] {
            log_lines = log_lines.child(SharedString::from(line.clone()));
        }
        page = page
            .child(div().flex().gap(px(16.)).child(keys_panel).child(controls))
            .child(
                style::panel().child(
                    div()
                        .relative()
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .child(style::heading("Seats"))
                        .child(seats),
                ),
            )
            .child(
                style::panel().child(
                    div()
                        .relative()
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .child(style::heading("Log"))
                        .child(log_lines),
                ),
            );
        style::ground().child(style::backdrop()).child(
            div()
                .id("server")
                .absolute()
                .size_full()
                .overflow_y_scroll()
                .child(page),
        )
    }
}
