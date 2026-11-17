//! The world view: shows the render thread's frames and forwards input to it.

use std::cell::Cell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

mod menu;

use crate::saves::Kind;
use crate::sim_thread::{SimLink, SimReport};
use gpui::{
    Context, CursorStyle, FocusHandle, ImageSource, InteractiveElement, IntoElement, KeyDownEvent,
    KeyUpEvent, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ObjectFit,
    ParentElement, Pixels, Point, Render, RenderImage, ScrollDelta, ScrollWheelEvent,
    StatefulInteractiveElement, Styled, StyledImage, Timer, Window, canvas, div, img, px, rgb,
    rgba,
};
use rom_render::{
    Frame, InputEvent, Key, Modifiers, PointerButton, RenderBridge, RenderStats, WheelGate,
};

/// How often the mailbox is checked for a new frame. Half a 60 Hz frame: at 16 ms the poll
/// drifted against the render clock and lost up to a fifth of the frames (docs/performance.md).
const POLL_INTERVAL: Duration = Duration::from_millis(8);
/// Step for changing the render scale with Page Up / Page Down, in percent.
const SCALE_STEP: u32 = 10;
/// Pixels of smooth scrolling that count as one wheel step.
const PIXELS_PER_WHEEL_STEP: f32 = 80.0;
/// Lines GPUI reports for one notch of a mouse wheel on Linux (its `SCROLL_LINES`).
const LINES_PER_NOTCH: f32 = 3.0;

/// Full-window canvas showing the latest frame, with a stats overlay on top.
pub struct WorldView {
    bridge: RenderBridge,
    show_stats: bool,
    sim: SimLink,
    focus: FocusHandle,
    pending: Option<Frame>,
    presented: Option<Arc<RenderImage>>,
    presented_frames: u64,
    presented_size: (u32, u32),
    recent_presents: VecDeque<Instant>,
    /// Canvas origin in window coordinates (logical pixels), measured during layout.
    origin: Rc<Cell<Point<Pixels>>>,
    /// Whether real mouse and keyboard input reaches the renderer; off for screenshot and
    /// benchmark runs, where a pointer resting at the window edge would pan the camera and
    /// keys would change the result.
    pointer_input: bool,
    /// The in-match menu while it is open.
    menu: Option<menu::Menu>,
    /// This window's own options (size, scale, HUD) that a loaded game keeps.
    display_arguments: Vec<String>,
    /// One zoom step per wheel gesture, whatever the wheel fires.
    wheel_gate: WheelGate,
    /// When the view was made: the clock of the wheel gate.
    created: Instant,
}

impl WorldView {
    /// Stops forwarding mouse and keyboard input to the renderer and the simulation
    /// (automated screenshot and benchmark runs, which end on their own).
    pub fn ignore_pointer(&mut self) {
        self.pointer_input = false;
    }

    /// Creates the view and starts polling the frame mailbox.
    pub fn new(
        bridge: RenderBridge,
        sim: SimLink,
        show_stats: bool,
        display_arguments: Vec<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus = cx.focus_handle();
        window.focus(&focus);
        cx.on_release_in(window, |view, window, _| {
            if let Some(image) = view.presented.take() {
                retire_image(window, image);
            }
        })
        .detach();
        cx.observe_window_activation(window, |view, window, _| {
            if !window.is_window_active() {
                view.bridge.push_input(InputEvent::ReleaseAll);
            }
        })
        .detach();
        let poll_bridge = bridge.clone();
        cx.spawn_in(window, async move |view, cx| {
            loop {
                Timer::after(POLL_INTERVAL).await;
                let alive = match poll_bridge.latest_frame() {
                    Some(frame) => view
                        .update(cx, |view, cx| {
                            view.pending = Some(frame);
                            cx.notify();
                        })
                        .is_ok(),
                    None => view.upgrade().is_some(),
                };
                if !alive {
                    break;
                }
                let _ = view.update_in(cx, |view, window, cx| view.pump(window, cx));
            }
        })
        .detach();
        Self {
            bridge,
            show_stats,
            sim,
            focus,
            pending: None,
            presented: None,
            presented_frames: 0,
            presented_size: (0, 0),
            recent_presents: VecDeque::new(),
            origin: Rc::new(Cell::new(Point::default())),
            pointer_input: true,
            menu: None,
            display_arguments,
            wheel_gate: WheelGate::default(),
            created: Instant::now(),
        }
    }

    /// Frames shown so far and the pixel size of the newest one.
    pub fn presented(&self) -> (u64, (u32, u32)) {
        (self.presented_frames, self.presented_size)
    }

    fn present_pending(&mut self, window: &mut Window) {
        let Some(frame) = self.pending.take() else {
            return;
        };
        // The mailbox hands the frame over, so the pixels are normally not shared and move without a copy.
        let pixels = Arc::try_unwrap(frame.pixels).unwrap_or_else(|shared| (*shared).clone());
        let image = Arc::new(RenderImage::new(vec![rom_render::image::Frame::new(
            pixels,
        )]));
        if let Some(previous) = self.presented.replace(image) {
            retire_image(window, previous);
        }
        let now = Instant::now();
        self.presented_frames += 1;
        self.presented_size = (frame.width, frame.height);
        self.recent_presents.push_back(now);
        while self
            .recent_presents
            .front()
            .is_some_and(|at| now.duration_since(*at) > Duration::from_secs(1))
        {
            self.recent_presents.pop_front();
        }
    }

    fn canvas_point(&self, position: Point<Pixels>, window: &Window) -> (f32, f32) {
        let scale = window.scale_factor();
        let origin = self.origin.get();
        (
            f32::from(position.x - origin.x) * scale,
            f32::from(position.y - origin.y) * scale,
        )
    }

    fn button_event(
        &self,
        button: MouseButton,
        position: Point<Pixels>,
        window: &Window,
        modifiers: gpui::Modifiers,
        down: bool,
    ) {
        if !self.pointer_input {
            return;
        }
        let button = match button {
            MouseButton::Left => PointerButton::Left,
            MouseButton::Right => PointerButton::Right,
            MouseButton::Middle => PointerButton::Middle,
            _ => return,
        };
        let (x, y) = self.canvas_point(position, window);
        let mods = Modifiers {
            shift: modifiers.shift,
            ctrl: modifiers.control,
            alt: modifiers.alt,
        };
        self.bridge.push_input(if down {
            InputEvent::ButtonDown { button, x, y, mods }
        } else {
            InputEvent::ButtonUp { button, x, y, mods }
        });
    }

    fn key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if !self.pointer_input {
            return;
        }
        if self.menu_open() {
            if event.keystroke.key == "escape" {
                self.menu_back(window, cx);
            }
            return;
        }
        match event.keystroke.key.as_str() {
            "escape" => {
                self.bridge.push_input(InputEvent::Escape);
                self.open_menu(window, cx);
                return;
            }
            "f5" => {
                self.sim.request_save(Kind::Quick, "");
                return;
            }
            "f9" => {
                self.quick_load(window, cx);
                return;
            }
            _ => {}
        }
        let current = self.bridge.render_scale();
        match event.keystroke.key.as_str() {
            "pageup" => self
                .bridge
                .publish_render_scale(current.saturating_add(SCALE_STEP)),
            "pagedown" => self
                .bridge
                .publish_render_scale(current.saturating_sub(SCALE_STEP)),
            "=" | "+" => self
                .sim
                .set_speed(self.sim.speed().saturating_mul(2).min(1600)),
            "-" => self.sim.set_speed((self.sim.speed() / 2).max(6)),
            "space" if event.keystroke.modifiers.shift => self
                .bridge
                .push_input(InputEvent::KeyDown(Key::FollowSelection)),
            name => {
                if let Some(key) = Key::from_name(name) {
                    self.bridge.push_input(InputEvent::KeyDown(key));
                } else if let (Some(ch), None) = (name.chars().next(), name.chars().nth(1))
                    && ch.is_ascii_alphanumeric()
                {
                    let modifiers = &event.keystroke.modifiers;
                    self.bridge.push_input(InputEvent::Hotkey {
                        ch: ch.to_ascii_lowercase(),
                        mods: Modifiers {
                            shift: modifiers.shift,
                            ctrl: modifiers.control,
                            alt: modifiers.alt,
                        },
                    });
                }
            }
        }
    }
}

impl Render for WorldView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.present_pending(window);

        let layout_bridge = self.bridge.clone();
        let layout_origin = Rc::clone(&self.origin);
        let layout_probe = canvas(
            move |bounds, window, _| {
                layout_origin.set(bounds.origin);
                let scale = window.scale_factor();
                let physical = |extent: Pixels| (f32::from(extent) * scale).round().max(0.0) as u32;
                layout_bridge
                    .publish_layout(physical(bounds.size.width), physical(bounds.size.height));
            },
            |_, _, _, _| {},
        )
        .absolute()
        .size_full();

        let picture = match &self.presented {
            Some(image) => img(ImageSource::Render(Arc::clone(image)))
                .absolute()
                .size_full()
                .object_fit(ObjectFit::Fill)
                .into_any_element(),
            None => div()
                .absolute()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(rgb(0x9fb0c3))
                .child("Starting renderer…")
                .into_any_element(),
        };

        let overlay = self.show_stats.then(|| {
            stats_overlay(
                self.bridge.stats(),
                self.bridge.render_scale(),
                self.recent_presents.len(),
                self.sim.report(),
            )
        });
        // A hand over what a double click enters (a planet, a star, a system).
        let cursor = if self.bridge.pointer_hand() {
            CursorStyle::PointingHand
        } else {
            CursorStyle::Arrow
        };
        div()
            .id("world-view")
            .track_focus(&self.focus)
            .cursor(cursor)
            .relative()
            .size_full()
            .overflow_hidden()
            .bg(rgb(0x1b2430))
            .on_mouse_move(cx.listener(|view, event: &MouseMoveEvent, window, _| {
                if !view.pointer_input || view.menu_open() {
                    return;
                }
                let (x, y) = view.canvas_point(event.position, window);
                view.bridge.push_input(InputEvent::PointerMoved { x, y });
            }))
            .on_hover(cx.listener(|view, hovered: &bool, _, _| {
                if !*hovered && view.pointer_input && !view.menu_open() {
                    view.bridge.push_input(InputEvent::PointerLeft);
                }
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|view, event: &MouseDownEvent, window, _| {
                    if view.menu_open() {
                        return;
                    }
                    window.focus(&view.focus);
                    view.button_event(event.button, event.position, window, event.modifiers, true);
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|view, event: &MouseDownEvent, window, _| {
                    if view.menu_open() {
                        return;
                    }
                    view.button_event(event.button, event.position, window, event.modifiers, true);
                }),
            )
            .on_mouse_down(
                MouseButton::Middle,
                cx.listener(|view, event: &MouseDownEvent, window, _| {
                    if view.menu_open() {
                        return;
                    }
                    view.button_event(event.button, event.position, window, event.modifiers, true);
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|view, event: &MouseUpEvent, window, _| {
                    if view.menu_open() {
                        return;
                    }
                    view.button_event(event.button, event.position, window, event.modifiers, false);
                }),
            )
            .on_mouse_up(
                MouseButton::Right,
                cx.listener(|view, event: &MouseUpEvent, window, _| {
                    if view.menu_open() {
                        return;
                    }
                    view.button_event(event.button, event.position, window, event.modifiers, false);
                }),
            )
            .on_mouse_up(
                MouseButton::Middle,
                cx.listener(|view, event: &MouseUpEvent, window, _| {
                    if view.menu_open() {
                        return;
                    }
                    view.button_event(event.button, event.position, window, event.modifiers, false);
                }),
            )
            .on_scroll_wheel(cx.listener(|view, event: &ScrollWheelEvent, _, _| {
                let notches = match event.delta {
                    ScrollDelta::Lines(delta) => delta.y / LINES_PER_NOTCH,
                    ScrollDelta::Pixels(delta) => f32::from(delta.y) / PIXELS_PER_WHEEL_STEP,
                };
                if notches == 0.0 || !view.pointer_input || view.menu_open() {
                    return;
                }
                // A notched wheel moves one step per gesture; smooth scrolling passes as it comes.
                let now_ms = view.created.elapsed().as_millis() as u64;
                let steps = view.wheel_gate.pass(now_ms, notches);
                if steps != 0.0 {
                    view.bridge.push_input(InputEvent::Wheel { steps });
                }
            }))
            .on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
                view.key_down(event, window, cx);
                cx.notify();
            }))
            .on_key_up(cx.listener(|view, event: &KeyUpEvent, _, _| {
                if !view.pointer_input || view.menu_open() {
                    return;
                }
                if let Some(key) = Key::from_name(&event.keystroke.key) {
                    view.bridge.push_input(InputEvent::KeyUp(key));
                }
            }))
            .child(layout_probe)
            .child(picture)
            .children(overlay)
            .children(self.render_menu(cx))
    }
}

/// Small GPUI panel drawn above the frame image.
fn stats_overlay(
    stats: RenderStats,
    scale: u32,
    shown_last_second: usize,
    sim: SimReport,
) -> impl IntoElement {
    div()
        .absolute()
        .top(px(150.0))
        .right(px(10.0))
        .px(px(10.0))
        .py(px(6.0))
        .rounded(px(4.0))
        .bg(rgba(0x0f151ccc))
        .text_color(rgb(0xe6edf5))
        .text_size(px(12.0))
        .flex()
        .flex_col()
        .gap(px(2.0))
        .child(format!(
            "render {}x{} at {scale}%",
            stats.render_width, stats.render_height
        ))
        .child(format!(
            "{} rendered/s, {shown_last_second} shown/s",
            stats.frames_last_second
        ))
        .child(format!(
            "frame {:.1} ms avg, {:.1} ms max",
            stats.frame_ms_avg, stats.frame_ms_max
        ))
        .child(format!(
            "read-back {:.1} ms avg, {:.1} ms max",
            stats.readback_ms_avg, stats.readback_ms_max
        ))
        .child(format!(
            "tick {}, {:.1} ticks/s, {} entities",
            sim.tick, sim.ticks_per_second, sim.entities
        ))
        .children((sim.computers > 0).then(|| {
            format!(
                "{} computer players: think {:.2} ms avg, {:.2} ms max; {} orders, {} refused",
                sim.computers,
                f64::from(sim.think_us_avg) / 1000.0,
                f64::from(sim.think_us_max) / 1000.0,
                sim.computer_orders,
                sim.computer_rejected
            )
        }))
        .child("PgUp/PgDn: render scale, +/-: sim speed")
}

/// Evicts `image` from the GPU atlas two frames from now, once no frame can still reference it.
fn retire_image(window: &Window, image: Arc<RenderImage>) {
    window.on_next_frame(move |window, _| {
        window.on_next_frame(move |window, _| {
            let _ = window.drop_image(image);
        });
    });
}
