//! The lobby window (task 88): the seats with their names, races or nations, ready and
//! connection; this seat's choice and Ready; the chat; for the host, the keys to hand out and
//! Start now. When the server starts the match, the match window opens and the lobby closes.

use std::time::{Duration, Instant};

use gpui::prelude::FluentBuilder;
use gpui::{
    AnyWindowHandle, App, AppContext, ClipboardItem, Context, Entity, InteractiveElement,
    IntoElement, ParentElement, Render, SharedString, StatefulInteractiveElement, Styled,
    Subscription, TitlebarOptions, Window, WindowBounds, WindowOptions, div, px, rgb, size,
};
use gpui_component::{
    Root,
    input::{Input, InputEvent, InputState},
};
use rom_net::{SeatKind, SeatState};

use super::session::{LobbyCommand, LobbyState, Phase, Session};
use crate::shell::WindowCtx;
use crate::style::{self, Look};

/// Opens the lobby window for `session`; the match window replaces it when the match starts.
pub fn open(cx: &mut App, ctx: WindowCtx, session: Session) -> Result<(), String> {
    let title = match &ctx.options.name {
        Some(name) => format!("{} · Lobby · {name}", crate::args::window_title()),
        None => format!("{} · Lobby", crate::args::window_title()),
    };
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::centered(size(px(1280.), px(800.)), cx)),
        titlebar: Some(TitlebarOptions {
            title: Some(title.clone().into()),
            ..Default::default()
        }),
        app_id: Some("rise-of-man-lobby".into()),
        ..Default::default()
    };
    cx.open_window(options, move |window, cx| {
        window.set_window_title(&title);
        let view = cx.new(|cx| LobbyView::new(ctx, session, window, cx));
        cx.new(|cx| Root::new(view, window, cx))
    })
    .map(|_| ())
    .map_err(|error| format!("could not open the lobby: {error:#}"))
}

/// The lobby window's view.
pub struct LobbyView {
    ctx: WindowCtx,
    session: Session,
    state: LobbyState,
    chat: Entity<InputState>,
    _subscription: Subscription,
    copied: Option<(usize, Instant)>,
}

impl LobbyView {
    fn new(ctx: WindowCtx, session: Session, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let chat =
            cx.new(|cx| InputState::new(window, cx).placeholder("Say something to everyone"));
        let subscription =
            cx.subscribe_in(&chat, window, |view, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    view.send_chat(window, cx);
                }
            });
        let handle = window.window_handle();
        // Test hook for screenshots: the lobby ends itself after some seconds.
        let quit_after = std::env::var("ROM_NET_LOBBY_SECONDS")
            .ok()
            .and_then(|s| s.parse::<f64>().ok())
            .filter(|s| s.is_finite() && *s > 0.0)
            .map(Duration::from_secs_f64);
        let opened = Instant::now();
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(150))
                    .await;
                let Ok(step) = this.update(cx, |view, cx| view.poll(cx)) else {
                    break;
                };
                match step {
                    Step::Wait => {}
                    Step::Play => {
                        let _ = cx.update(|cx| Self::play(&this, handle, cx));
                        break;
                    }
                    Step::Quit(code) => {
                        let _ = this.update(cx, |view, _| {
                            view.ctx
                                .exit_code
                                .store(code, std::sync::atomic::Ordering::Relaxed);
                        });
                        let _ = cx.update(|cx| cx.quit());
                        break;
                    }
                }
                if quit_after.is_some_and(|after| opened.elapsed() >= after) {
                    let _ = cx.update(|cx| cx.quit());
                    break;
                }
            }
        })
        .detach();
        window.on_window_should_close(cx, |_, _| true);
        Self {
            ctx,
            session,
            state: LobbyState::default(),
            chat,
            _subscription: subscription,
            copied: None,
        }
    }

    /// Reads the session; says whether to play or end.
    fn poll(&mut self, cx: &mut Context<Self>) -> Step {
        let state = self.session.state();
        let changed = state.phase != self.state.phase
            || state.status != self.state.status
            || state.error != self.state.error
            || state.lobby != self.state.lobby
            || state.chat.len() != self.state.chat.len()
            || state.keys.len() != self.state.keys.len()
            || state.log.len() != self.state.log.len();
        self.state = state;
        if changed || self.copied.is_some() {
            cx.notify();
        }
        match self.state.phase {
            Phase::Playing => Step::Play,
            // A self-ending run (benchmark or screenshot) ends when it cannot join.
            Phase::Failed
                if self.ctx.options.bench_seconds.is_some()
                    || self.ctx.options.screenshot.is_some() =>
            {
                eprintln!(
                    "rise-of-man: {}",
                    self.state.error.as_deref().unwrap_or("network play failed")
                );
                Step::Quit(1)
            }
            _ => Step::Wait,
        }
    }

    /// The match started: its window opens and the lobby closes.
    fn play(this: &gpui::WeakEntity<Self>, lobby: AnyWindowHandle, cx: &mut App) {
        let Some(view) = this.upgrade() else { return };
        let ctx = view.read(cx).ctx.clone();
        let link = ctx
            .sim
            .lock()
            .ok()
            .and_then(|sim| sim.as_ref().map(crate::sim_thread::SimThread::link));
        let Some(link) = link else { return };
        if let Err(error) = crate::shell::open_match_window(cx, &ctx, link) {
            eprintln!("rise-of-man: {error}");
            ctx.exit_code.store(1, std::sync::atomic::Ordering::Relaxed);
            cx.quit();
            return;
        }
        let _ = lobby.update(cx, |_, window, _| window.remove_window());
    }

    fn send_chat(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.chat.read(cx).value().trim().to_owned();
        if text.is_empty() {
            return;
        }
        self.session.send(LobbyCommand::Chat(text));
        self.chat
            .update(cx, |input, cx| input.set_value("", window, cx));
    }

    fn mine(&self) -> Option<&rom_net::SeatInfo> {
        let seat = self.state.seat?;
        self.state
            .lobby
            .as_ref()?
            .seats
            .iter()
            .find(|s| s.seat == seat)
    }

    fn seats(&self) -> gpui::AnyElement {
        let mut table = div().flex().flex_col().gap_1();
        let header = |text: &str, width: f32| {
            div()
                .w(px(width))
                .text_size(px(11.))
                .font_weight(gpui::FontWeight::BOLD)
                .text_color(rgb(style::MUTED))
                .child(SharedString::from(text.to_uppercase()))
        };
        table = table.child(
            div()
                .flex()
                .gap_2()
                .child(header("Seat", 50.))
                .child(header("Name", 270.))
                .child(header("Race or nation", 170.))
                .child(header("Ready", 80.))
                .child(header("Connection", 180.)),
        );
        let Some(lobby) = &self.state.lobby else {
            return table.into_any_element();
        };
        for seat in &lobby.seats {
            let ours = Some(seat.seat) == self.state.seat;
            let cell = |text: String, width: f32| {
                div()
                    .w(px(width))
                    .text_size(px(14.))
                    .child(SharedString::from(text))
            };
            let mut name = seat.name.clone();
            if seat.host {
                name.push_str(" (host)");
            }
            if ours {
                name.push_str(" · you");
            }
            let (connection, colour) = match (seat.kind, seat.state) {
                (SeatKind::Computer, _) => ("computer".to_owned(), style::MUTED),
                // In the lobby a taken seat stays `open` until the match starts.
                (_, SeatState::Open) if seat.connected => ("connected".to_owned(), style::GOOD),
                (_, SeatState::Open) => ("open: waiting for someone".to_owned(), style::MUTED),
                (_, SeatState::Connected) => ("connected".to_owned(), style::GOOD),
                (_, SeatState::Waiting) => ("dropped: waiting".to_owned(), style::BAD),
                (_, SeatState::Computer) => ("a computer holds it".to_owned(), style::MUTED),
                (_, SeatState::Idle) => ("dropped".to_owned(), style::BAD),
            };
            let (ready, ready_colour) = match seat.kind {
                SeatKind::Computer => ("—".to_owned(), style::MUTED),
                SeatKind::Human if seat.ready => ("ready".to_owned(), style::GOOD),
                SeatKind::Human => ("not yet".to_owned(), style::WARN),
            };
            table = table.child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .py(px(6.))
                    .border_t_1()
                    .border_color(rgb(style::LINE))
                    .text_color(rgb(if ours { style::GOLD } else { style::TEXT }))
                    .child(cell(format!("{}", seat.seat + 1), 50.))
                    .child(cell(name, 270.))
                    .child(cell(self.state.faction_name(&seat.faction), 170.))
                    .child(cell(ready, 80.).text_color(rgb(ready_colour)))
                    .child(cell(connection, 180.).text_color(rgb(colour))),
            );
        }
        table.into_any_element()
    }

    fn own_choice(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(mine) = self.mine().cloned() else {
            return div().into_any_element();
        };
        let in_lobby = self
            .state
            .lobby
            .as_ref()
            .is_some_and(|l| l.phase == "lobby" || l.phase == "countdown");
        let mut factions = style::choices();
        for (index, (key, name)) in self.state.playable.iter().enumerate() {
            let key = key.clone();
            factions = factions.child(
                style::choice(("faction", index), name.clone(), mine.faction == key).when(
                    in_lobby,
                    |c| {
                        c.on_click(cx.listener(move |view, _, _, _| {
                            view.session.send(LobbyCommand::Choose(key.clone()));
                        }))
                    },
                ),
            );
        }
        let ready = mine.ready;
        let look = if ready { Look::Plain } else { Look::Gold };
        div()
            .flex()
            .flex_col()
            .gap(px(12.))
            .child(style::heading("Your seat"))
            .child(style::hint("Your race or nation:"))
            .child(factions)
            .child(
                div().flex().gap_2().items_center().child(
                    style::button(
                        "ready",
                        if ready { "Not ready" } else { "Ready" },
                        look,
                        in_lobby,
                    )
                    .when(in_lobby, |b| {
                        b.on_click(cx.listener(move |view, _, _, _| {
                            view.session.send(LobbyCommand::Ready(!ready));
                        }))
                    }),
                ),
            )
            .into_any_element()
    }

    fn host_panel(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        if !self.state.host {
            return div()
                .flex()
                .flex_col()
                .gap(px(10.))
                .child(style::heading("The start"))
                .child(style::hint(
                    "The match starts five seconds after every person is ready, or when the host starts it.",
                ))
                .into_any_element();
        }
        let mut keys = div().flex().flex_col().gap(px(6.));
        for (index, key) in self.state.keys.iter().enumerate() {
            let copied = self
                .copied
                .is_some_and(|(i, at)| i == index && at.elapsed() < Duration::from_secs(2));
            let text = key.clone();
            keys = keys.child(
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
        let in_lobby = self
            .state
            .lobby
            .as_ref()
            .is_some_and(|l| l.phase == "lobby" || l.phase == "countdown");
        div()
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(style::heading("Keys to hand out"))
            .child(style::hint("Each key admits one person once. Others join:"))
            .child(
                div()
                    .font_family(style::MONO)
                    .text_size(px(13.))
                    .text_color(rgb(style::CYAN))
                    .child(SharedString::from(
                        self.state
                            .address
                            .clone()
                            .unwrap_or_else(|| "this computer's address".into()),
                    )),
            )
            .child(keys)
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        style::button("add-key", "Add a key", Look::Plain, true).on_click(
                            cx.listener(|view, _, _, _| {
                                view.session.send(LobbyCommand::AddKey);
                            }),
                        ),
                    )
                    .child(
                        style::button("start-now", "Start now", Look::Gold, in_lobby).when(
                            in_lobby,
                            |b| {
                                b.on_click(cx.listener(|view, _, _, _| {
                                    view.session.send(LobbyCommand::Start);
                                }))
                            },
                        ),
                    ),
            )
            .child(style::hint("Start now plays empty seats with computers."))
            .into_any_element()
    }

    fn chat_panel(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let mut lines = div().flex().flex_col().gap_1().text_size(px(13.));
        let shown = self.state.chat.len().saturating_sub(8);
        for line in &self.state.chat[shown..] {
            lines = lines.child(SharedString::from(line.clone()));
        }
        div()
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(style::heading("Chat"))
            .child(lines)
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(div().flex_1().child(Input::new(&self.chat)))
                    .child(
                        style::button("chat-send", "Send", Look::Plain, true).on_click(
                            cx.listener(|view, _, window, cx| view.send_chat(window, cx)),
                        ),
                    ),
            )
            .into_any_element()
    }
}

/// What the poll found.
enum Step {
    Wait,
    Play,
    Quit(u8),
}

impl Render for LobbyView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = &self.state;
        let countdown = state
            .lobby
            .as_ref()
            .and_then(|l| l.countdown)
            .map(|s| format!("The match starts in {s} s"));
        let status = countdown.unwrap_or_else(|| state.status.clone());
        let mut page = div()
            .flex()
            .flex_col()
            .gap(px(16.))
            .p(px(24.))
            .child(
                div()
                    .flex()
                    .items_end()
                    .gap(px(16.))
                    .child(style::title("Lobby"))
                    .child(style::hint(SharedString::from(format!(
                        "{} · {}",
                        crate::args::window_title(),
                        if state.url.is_empty() {
                            "…".to_owned()
                        } else {
                            state.url.clone()
                        }
                    )))),
            )
            .child(style::status(status, false));
        if let Some(error) = state.error.clone() {
            page = page.child(
                style::panel()
                    .child(
                        div()
                            .relative()
                            .text_color(rgb(style::BAD))
                            .child(SharedString::from(error)),
                    )
                    .child(
                        div().relative().child(
                            style::button("close", "Close", Look::Plain, true)
                                .on_click(cx.listener(|_, _, window, _| window.remove_window())),
                        ),
                    ),
            );
        } else {
            let own = self.own_choice(cx);
            let host = self.host_panel(cx);
            let chat = self.chat_panel(cx);
            page = page
                .child(style::panel().child(div().relative().child(self.seats())))
                .child(
                    div()
                        .flex()
                        .gap(px(16.))
                        .child(style::panel().flex_1().child(div().relative().child(own)))
                        .child(
                            style::panel()
                                .w(px(440.))
                                .child(div().relative().child(host)),
                        ),
                )
                .child(style::panel().child(div().relative().child(chat)));
        }
        style::ground().child(style::backdrop()).child(
            div()
                .id("lobby")
                .absolute()
                .size_full()
                .overflow_y_scroll()
                .child(page),
        )
    }
}
