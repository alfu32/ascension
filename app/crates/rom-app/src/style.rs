//! The game's look for its GPUI windows (task 112): the launcher, the lobby and the server
//! window draw every panel, button and choice through this module, so none is styled ad hoc.
//!
//! Palette and font are the in-match HUD's (`app/docs/hud.md`, "Style"). Frames are the HUD's
//! vector chrome (`assets/hud/chrome/`) and the launcher's pieces built from the same helpers
//! (`assets/launcher/generate.py`), rasterised with resvg as the HUD atlas does and cut into
//! nine slices: corners keep their shape, edges stretch. Pictures are captures of the game
//! (`assets/launcher/capture.sh`), embedded like the SVGs.

use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

use gpui::{
    App, Div, ElementId, FontWeight, ImageSource, InteractiveElement, IntoElement, ObjectFit,
    ParentElement, RenderImage, SharedString, Stateful, Styled, StyledImage, div, img, px, rgb,
    rgba,
};
use resvg::{tiny_skia, usvg};
use rom_render::image::{Frame, RgbaImage, imageops};

/// Panel fill, darker end.
pub const NAVY0: u32 = 0x0c1118;
/// Panel fill, lighter end.
pub const NAVY1: u32 = 0x141b24;
/// Inner slot fill.
pub const SLOT: u32 = 0x0a0e14;
/// Thin slot line.
pub const LINE: u32 = 0x2a3340;
/// Gold, light.
pub const GOLD: u32 = 0xe8c98a;
/// Gold, mid.
pub const GOLD_M: u32 = 0xb88a45;
/// Accent cyan.
pub const CYAN: u32 = 0x3fd0ff;
/// Body text.
pub const TEXT: u32 = 0xf2f2f2;
/// Help and secondary text (the HUD's muted colour).
pub const MUTED: u32 = 0x97a5b1;
/// Good news: connected, positive rates.
pub const GOOD: u32 = 0x3ddc3d;
/// Warnings and the reasons a button is off.
pub const WARN: u32 = 0xf0b030;
/// Errors and dropped players.
pub const BAD: u32 = 0xeb6e64;
/// Text on the gold plate.
const ON_GOLD: u32 = 0x24170a;

/// The font family every window uses; its bold face is bundled.
pub const FONT: &str = "DejaVu Sans";
/// The monospaced family for keys, commands and logs.
pub const MONO: &str = "DejaVu Sans Mono";

/// Pixels rasterised per drawn pixel, so frames stay sharp on scaled displays.
const OVERSAMPLE: f32 = 2.0;

/// The launcher's own frame pieces (`assets/launcher/generate.py`), by name; the HUD's come
/// from the HUD atlas sources as `chrome/<name>`.
const LAUNCHER_SVG: &[(&str, &str)] = &[
    (
        "panel",
        include_str!("../../../assets/launcher/chrome/panel.svg"),
    ),
    (
        "panel_ornate",
        include_str!("../../../assets/launcher/chrome/panel_ornate.svg"),
    ),
    (
        "button",
        include_str!("../../../assets/launcher/chrome/button.svg"),
    ),
    (
        "button_gold",
        include_str!("../../../assets/launcher/chrome/button_gold.svg"),
    ),
    (
        "frame",
        include_str!("../../../assets/launcher/chrome/frame.svg"),
    ),
    (
        "toggle_on",
        include_str!("../../../assets/launcher/chrome/toggle_on.svg"),
    ),
    (
        "toggle_off",
        include_str!("../../../assets/launcher/chrome/toggle_off.svg"),
    ),
    (
        "glow",
        include_str!("../../../assets/launcher/chrome/glow.svg"),
    ),
];

/// A frame or picture source: the launcher's own, else the HUD's chrome of that name.
fn svg(name: &str) -> Option<&'static str> {
    LAUNCHER_SVG
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, s)| *s)
        .or_else(|| hud_source(&format!("chrome/{name}")))
}

fn hud_source(name: &str) -> Option<&'static str> {
    rom_render::hud_atlas::SOURCES
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, s)| *s)
}

/// Captures of the game (`assets/launcher/capture.sh`): the backdrop, one card per map style
/// (`maps/<style key>`) and the preset tiles (`tiles/<name>`).
const PICTURES: &[(&str, &[u8])] = &[
    (
        "backdrop",
        include_bytes!("../../../assets/launcher/backdrop.jpg"),
    ),
    ("hero", include_bytes!("../../../assets/launcher/hero.jpg")),
    (
        "maps/map.galaxy",
        include_bytes!("../../../assets/launcher/maps/map.galaxy.jpg"),
    ),
    (
        "maps/map.many_worlds",
        include_bytes!("../../../assets/launcher/maps/map.many_worlds.jpg"),
    ),
    (
        "maps/map.one_world",
        include_bytes!("../../../assets/launcher/maps/map.one_world.jpg"),
    ),
    (
        "maps/map.quadrant",
        include_bytes!("../../../assets/launcher/maps/map.quadrant.jpg"),
    ),
    (
        "maps/map.sector",
        include_bytes!("../../../assets/launcher/maps/map.sector.jpg"),
    ),
    (
        "maps/map.solar",
        include_bytes!("../../../assets/launcher/maps/map.solar.jpg"),
    ),
    (
        "tiles/classic",
        include_bytes!("../../../assets/launcher/tiles/classic.jpg"),
    ),
    (
        "tiles/custom",
        include_bytes!("../../../assets/launcher/tiles/custom.jpg"),
    ),
    (
        "tiles/demo",
        include_bytes!("../../../assets/launcher/tiles/demo.jpg"),
    ),
    (
        "tiles/gallery",
        include_bytes!("../../../assets/launcher/tiles/gallery.jpg"),
    ),
    (
        "tiles/model",
        include_bytes!("../../../assets/launcher/tiles/model.jpg"),
    ),
    (
        "tiles/peaceful",
        include_bytes!("../../../assets/launcher/tiles/peaceful.jpg"),
    ),
    (
        "tiles/quick",
        include_bytes!("../../../assets/launcher/tiles/quick.jpg"),
    ),
    (
        "tiles/sandbox",
        include_bytes!("../../../assets/launcher/tiles/sandbox.jpg"),
    ),
    (
        "tiles/space",
        include_bytes!("../../../assets/launcher/tiles/space.jpg"),
    ),
    (
        "tiles/war",
        include_bytes!("../../../assets/launcher/tiles/war.jpg"),
    ),
    (
        "tiles/watch",
        include_bytes!("../../../assets/launcher/tiles/watch.jpg"),
    ),
];

/// Installs the look: the bundled bold face and the widget theme (inputs and dropdowns as HUD
/// slots, cyan focus). Every window calls this once after `gpui_component::init`.
pub fn install(cx: &mut App) {
    let bold: &'static [u8] = include_bytes!("../../../assets/fonts/DejaVuSans-Bold.ttf");
    if let Err(error) = cx
        .text_system()
        .add_fonts(vec![std::borrow::Cow::Borrowed(bold)])
    {
        eprintln!("rise-of-man: bundled font: {error}");
    }
    gpui_component::Theme::change(gpui_component::ThemeMode::Dark, None, cx);
    let theme = gpui_component::Theme::global_mut(cx);
    let colors = &mut theme.colors;
    colors.background = rgb(SLOT).into();
    colors.foreground = rgb(TEXT).into();
    colors.primary = rgb(CYAN).into();
    colors.primary_foreground = rgb(SLOT).into();
    colors.border = rgb(LINE).into();
    colors.input = rgb(0x33414f).into();
    colors.ring = rgb(CYAN).into();
    colors.popover = rgb(NAVY1).into();
    colors.popover_foreground = rgb(TEXT).into();
    colors.accent = rgba(0x3fd0ff33).into();
    colors.accent_foreground = rgb(TEXT).into();
    colors.list_active = rgba(0x3fd0ff33).into();
    colors.list_active_border = rgb(CYAN).into();
    colors.list_hover = rgba(0x3fd0ff1a).into();
    colors.muted_foreground = rgb(MUTED).into();
    colors.selection = rgba(0x3fd0ff55).into();
    colors.caret = rgb(CYAN).into();
    colors.scrollbar_thumb = rgba(0xb88a4599).into();
    colors.scrollbar_thumb_hover = rgb(GOLD_M).into();
    theme.font_family = FONT.into();
    theme.radius = px(2.);
    theme.radius_lg = px(3.);
}

/// One frame cut into nine pictures: corners, edges and centre, row by row.
pub struct Slices {
    parts: Vec<Arc<RenderImage>>,
    /// Drawn width of the left, top, right and bottom borders, in pixels.
    edge: [f32; 4],
}

/// Every picture the windows draw, made once per process.
pub struct Chrome {
    frames: BTreeMap<&'static str, Slices>,
    images: BTreeMap<&'static str, Arc<RenderImage>>,
    pictures: BTreeMap<&'static str, Arc<RenderImage>>,
    icons: std::sync::Mutex<BTreeMap<String, Arc<RenderImage>>>,
}

/// The rasterised chrome, made on first use.
pub fn chrome() -> &'static Chrome {
    static CHROME: OnceLock<Chrome> = OnceLock::new();
    CHROME.get_or_init(Chrome::bake)
}

/// Nine-slice cuts per frame: source, display scale (pixels per SVG unit) and the left, top,
/// right and bottom borders in SVG units.
const CUTS: &[(&str, f32, [f32; 4])] = &[
    ("panel", 1.0, [16., 16., 16., 16.]),
    ("panel_ornate", 0.7, [74., 42., 74., 42.]),
    ("button", 1.0, [11., 11., 11., 11.]),
    ("button_gold", 1.0, [11., 11., 11., 11.]),
    ("cell", 1.0, [8., 8., 8., 8.]),
    ("tab_active", 1.0, [8., 15., 15., 8.]),
    ("frame", 1.0, [10., 10., 10., 10.]),
    ("tile", 1.0, [10., 10., 10., 10.]),
];

impl Chrome {
    fn bake() -> Self {
        let source = svg;
        let frames = CUTS
            .iter()
            .filter_map(|&(name, scale, edge)| Some((name, slice(source(name)?, scale, edge))))
            .collect();
        let mut images = BTreeMap::new();
        for (name, scale) in [
            ("emblem", 2.0),
            ("toggle_on", 1.0),
            ("toggle_off", 1.0),
            ("glow", 1.0),
        ] {
            if let Some(svg) = source(name) {
                images.insert(name, to_render(rasterise(svg, scale * OVERSAMPLE)));
            }
        }
        let pictures = PICTURES
            .iter()
            .filter_map(|(name, bytes)| {
                let image = rom_render::image::load_from_memory(bytes)
                    .map_err(|e| eprintln!("rise-of-man: picture {name}: {e}"))
                    .ok()?;
                Some((*name, to_render(image.into_rgba8())))
            })
            .collect();
        Self {
            frames,
            images,
            pictures,
            icons: Default::default(),
        }
    }

    fn frame(&self, name: &str) -> &Slices {
        self.frames
            .get(name)
            .or_else(|| self.frames.values().next())
            .expect("the chrome has frames")
    }

    fn image(&self, name: &str) -> Option<Arc<RenderImage>> {
        self.images.get(name).cloned()
    }

    /// HUD icon `name` (`assets/hud/icons/<name>.svg`), rasterised on first use.
    fn icon(&self, name: &str) -> Option<Arc<RenderImage>> {
        let mut icons = self.icons.lock().ok()?;
        if let Some(icon) = icons.get(name) {
            return Some(icon.clone());
        }
        let source = hud_source(&format!("icons/{name}"))?;
        let icon = to_render(rasterise(source, OVERSAMPLE));
        icons.insert(name.to_owned(), icon.clone());
        Some(icon)
    }

    /// A capture of the game by name (`backdrop`, or a map style's key).
    pub fn picture(&self, name: &str) -> Option<Arc<RenderImage>> {
        self.pictures.get(name).cloned()
    }
}

/// Rasterises `svg` at `scale` pixels per unit into straight-alpha RGBA. A source that does not
/// parse gives one transparent pixel (the tests check every source parses).
fn rasterise(svg: &str, scale: f32) -> RgbaImage {
    let Ok(tree) = usvg::Tree::from_str(svg, &usvg::Options::default()) else {
        return RgbaImage::new(1, 1);
    };
    let size = tree.size();
    let width = (size.width() * scale).round().max(1.0) as u32;
    let height = (size.height() * scale).round().max(1.0) as u32;
    let Some(mut pixmap) = tiny_skia::Pixmap::new(width, height) else {
        return RgbaImage::new(1, 1);
    };
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(
            width as f32 / size.width(),
            height as f32 / size.height(),
        ),
        &mut pixmap.as_mut(),
    );
    let mut pixels = pixmap.take();
    // tiny-skia premultiplies; images are drawn with straight alpha.
    for pixel in pixels.as_chunks_mut::<4>().0 {
        let alpha = u32::from(pixel[3]);
        if alpha != 0 && alpha != 255 {
            for channel in &mut pixel[..3] {
                *channel = ((u32::from(*channel) * 255 + alpha / 2) / alpha).min(255) as u8;
            }
        }
    }
    RgbaImage::from_raw(width, height, pixels).unwrap_or_else(|| RgbaImage::new(1, 1))
}

/// An RGBA picture as GPUI draws it (BGRA).
fn to_render(mut image: RgbaImage) -> Arc<RenderImage> {
    let bytes: &mut [u8] = &mut image;
    for pixel in bytes.as_chunks_mut::<4>().0 {
        pixel.swap(0, 2);
    }
    Arc::new(RenderImage::new([Frame::new(image)]))
}

/// Cuts the frame `svg`, drawn at `scale` pixels per unit, into nine parts at `edge` units.
fn slice(svg: &str, scale: f32, edge: [f32; 4]) -> Slices {
    let drawn = edge.map(|units| (units * scale).round().max(1.0));
    let image = rasterise(svg, scale * OVERSAMPLE);
    let (width, height) = image.dimensions();
    let cut = drawn.map(|d| (d * OVERSAMPLE) as u32);
    let xs = [
        0,
        cut[0].min(width / 2),
        width.saturating_sub(cut[2]).max(width / 2),
        width,
    ];
    let ys = [
        0,
        cut[1].min(height / 2),
        height.saturating_sub(cut[3]).max(height / 2),
        height,
    ];
    let mut parts = Vec::with_capacity(9);
    for row in 0..3 {
        for column in 0..3 {
            let (x, y) = (xs[column], ys[row]);
            let w = (xs[column + 1] - x).max(1).min(width - x.min(width - 1));
            let h = (ys[row + 1] - y).max(1).min(height - y.min(height - 1));
            let part = imageops::crop_imm(&image, x.min(width - 1), y.min(height - 1), w, h);
            parts.push(to_render(part.to_image()));
        }
    }
    Slices { parts, edge: drawn }
}

/// A picture filling its box.
fn picture(image: Arc<RenderImage>, fit: ObjectFit) -> gpui::Img {
    img(ImageSource::Render(image)).size_full().object_fit(fit)
}

/// The nine slices of frame `name`, laid over the whole of the element they are put in (which
/// must be `relative`). Put it first so the content paints above it. The slices are painted
/// into exact rectangles (an `img` keeps its picture's aspect ratio, which a stretched edge
/// must not).
pub fn frame(name: &str) -> Div {
    let slices = chrome().frame(name);
    let parts = slices.parts.clone();
    let [l, t, r, b] = slices.edge;
    div()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .bottom_0()
        .child(
            gpui::canvas(
                |_, _, _| (),
                move |bounds, _, window, _| {
                    let (width, height) =
                        (f32::from(bounds.size.width), f32::from(bounds.size.height));
                    // A box smaller than its corners shrinks them in proportion.
                    let kx = (width / (l + r)).min(1.0);
                    let ky = (height / (t + b)).min(1.0);
                    let (x, y) = (f32::from(bounds.origin.x), f32::from(bounds.origin.y));
                    let xs = [x, x + l * kx, x + width - r * kx, x + width];
                    let ys = [y, y + t * ky, y + height - b * ky, y + height];
                    for row in 0..3 {
                        for column in 0..3 {
                            let (x0, x1, y0, y1) =
                                (xs[column], xs[column + 1], ys[row], ys[row + 1]);
                            if x1 - x0 < 0.5 || y1 - y0 < 0.5 {
                                continue;
                            }
                            let rect = gpui::Bounds::from_corners(
                                gpui::point(px(x0), px(y0)),
                                gpui::point(px(x1), px(y1)),
                            );
                            let _ = window.paint_image(
                                rect,
                                gpui::Corners::default(),
                                parts[row * 3 + column].clone(),
                                0,
                                false,
                            );
                        }
                    }
                },
            )
            .size_full(),
        )
}

/// A framed sub-panel: the HUD's thin gold bevel round a navy fill, a cyan accent beneath.
pub fn panel() -> Div {
    div()
        .relative()
        .flex()
        .flex_col()
        .gap_2()
        .px(px(20.))
        .py(px(16.))
        .child(frame("panel"))
        .child(accent())
}

/// The main panel of a screen: the HUD top bar's heavy bevel with its corner strokes.
pub fn main_panel() -> Div {
    div()
        .relative()
        .flex()
        .flex_col()
        .px(px(36.))
        .py(px(26.))
        .child(frame("panel_ornate"))
        .child(accent())
}

/// The short cyan glow the HUD puts at the bottom of its panels.
fn accent() -> Div {
    let glow = chrome().image("glow");
    div()
        .absolute()
        .bottom(px(-3.))
        .left_0()
        .right_0()
        .flex()
        .justify_center()
        .children(glow.map(|g| {
            div()
                .w(px(100.))
                .h(px(8.))
                .child(picture(g, ObjectFit::Fill))
        }))
}

/// A heading: bold capitals in gold, as the HUD's panel titles.
pub fn heading(text: impl Into<SharedString>) -> Div {
    let text: SharedString = text.into();
    div()
        .text_size(px(13.))
        .font_weight(FontWeight::BOLD)
        .text_color(rgb(GOLD))
        .child(SharedString::from(text.to_uppercase()))
}

/// A screen title: large bold capitals in gold.
pub fn title(text: impl Into<SharedString>) -> Div {
    let text: SharedString = text.into();
    div()
        .text_size(px(24.))
        .font_weight(FontWeight::BOLD)
        .text_color(rgb(GOLD))
        .child(SharedString::from(text.to_uppercase()))
}

/// Help text in the HUD's muted colour.
pub fn hint(text: impl Into<SharedString>) -> Div {
    div()
        .text_size(px(12.))
        .text_color(rgb(MUTED))
        .child(text.into())
}

/// A row label: bold, light.
pub fn label(text: impl Into<SharedString>) -> Div {
    div()
        .text_size(px(13.))
        .font_weight(FontWeight::BOLD)
        .text_color(rgb(TEXT))
        .child(text.into())
}

/// How a button looks.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Look {
    /// The main action: a gold plate.
    Gold,
    /// Any other action: a gold bevel round a dark slot.
    Framed,
    /// Back, Close and the like: a dark slot with a thin line.
    Plain,
}

/// The cyan outline shown while the pointer is over a control (a member of group `group`).
fn hover_outline(group: &'static str) -> Div {
    div()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .bottom_0()
        .opacity(0.)
        .group_hover(group, |s| s.opacity(0.85))
        .child(frame("tab_active"))
}

/// A button with `text` in bold capitals. Attach its click handler only when `enabled`; a
/// disabled button is drawn faded.
pub fn button(
    id: impl Into<ElementId>,
    text: impl Into<SharedString>,
    look: Look,
    enabled: bool,
) -> Stateful<Div> {
    let text: SharedString = text.into();
    let (art, colour) = match look {
        Look::Gold => ("button_gold", ON_GOLD),
        Look::Framed => ("button", GOLD),
        Look::Plain => ("cell", TEXT),
    };
    let mut button = div()
        .id(id)
        .group("rom-button")
        .relative()
        .flex()
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .h(px(36.))
        .px(px(if look == Look::Gold { 26. } else { 18. }))
        .child(frame(art))
        .child(
            div()
                .relative()
                .text_size(px(13.))
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(colour))
                .whitespace_nowrap()
                .child(SharedString::from(text.to_uppercase())),
        );
    if enabled {
        button = button.cursor_pointer().child(hover_outline("rom-button"));
    } else {
        button = button.opacity(0.4);
    }
    button
}

/// A small square button (a stepper's − and +).
pub fn small_button(id: impl Into<ElementId>, glyph: &'static str, enabled: bool) -> Stateful<Div> {
    let mut button = div()
        .id(id)
        .group("rom-button")
        .relative()
        .flex()
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .size(px(32.))
        .child(frame("button"))
        .child(
            div()
                .relative()
                .text_size(px(18.))
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(GOLD))
                .child(glyph),
        );
    if enabled {
        button = button.cursor_pointer().child(hover_outline("rom-button"));
    } else {
        button = button.opacity(0.35);
    }
    button
}

/// One option of a segmented choice: a dark slot; the chosen one glows cyan.
pub fn choice(
    id: impl Into<ElementId>,
    text: impl Into<SharedString>,
    chosen: bool,
) -> Stateful<Div> {
    let text: SharedString = text.into();
    let mut item = div()
        .id(id)
        .group("rom-choice")
        .relative()
        .flex()
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .h(px(32.))
        .px(px(11.))
        .cursor_pointer()
        .child(frame("cell"))
        .child(
            div()
                .relative()
                .text_size(px(12.))
                .font_weight(FontWeight::BOLD)
                .whitespace_nowrap()
                .text_color(rgb(if chosen { TEXT } else { MUTED }))
                .child(SharedString::from(text.to_uppercase())),
        );
    item = if chosen {
        item.child(frame("tab_active"))
    } else {
        item.child(hover_outline("rom-choice"))
    };
    item
}

/// A row of choices side by side, wrapping when the window is narrow.
pub fn choices() -> Div {
    div().flex().flex_wrap().gap(px(6.))
}

/// A yes/no switch followed by `text`.
pub fn toggle(id: impl Into<ElementId>, on: bool, text: impl Into<SharedString>) -> Stateful<Div> {
    let art = chrome().image(if on { "toggle_on" } else { "toggle_off" });
    div()
        .id(id)
        .group("rom-toggle")
        .flex()
        .items_center()
        .gap_2()
        .cursor_pointer()
        .child(
            div()
                .relative()
                .w(px(48.))
                .h(px(24.))
                .flex_shrink_0()
                .children(art.map(|a| picture(a, ObjectFit::Fill))),
        )
        .child(
            div()
                .text_size(px(13.))
                .text_color(rgb(if on { TEXT } else { MUTED }))
                .group_hover("rom-toggle", |s| s.text_color(rgb(TEXT)))
                .child(text.into()),
        )
}

/// A number between two small buttons; the caller attaches the buttons' handlers.
pub fn stepper(minus: Stateful<Div>, value: impl Into<SharedString>, plus: Stateful<Div>) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(4.))
        .child(minus)
        .child(
            div()
                .relative()
                .w(px(56.))
                .h(px(32.))
                .flex()
                .items_center()
                .justify_center()
                .child(frame("cell"))
                .child(
                    div()
                        .relative()
                        .text_size(px(16.))
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(TEXT))
                        .child(value.into()),
                ),
        )
        .child(plus)
}

/// A picture card: a capture in a gold frame, a name and one line; the chosen one glows cyan.
/// Without a picture, the name stands on a dark slot.
pub fn card(
    id: impl Into<ElementId>,
    picture_name: Option<&str>,
    badge: Option<&str>,
    name: impl Into<SharedString>,
    line: impl Into<SharedString>,
    chosen: bool,
    picture_height: f32,
) -> Stateful<Div> {
    let name: SharedString = name.into();
    let art = picture_name.and_then(|n| chrome().picture(n));
    let mut shot = div()
        .relative()
        .w_full()
        .h(px(picture_height))
        .overflow_hidden()
        .bg(rgb(SLOT));
    if let Some(art) = art {
        shot = shot.child(
            div()
                .absolute()
                .size_full()
                .child(picture(art, ObjectFit::Cover)),
        );
    }
    shot = shot.child(frame("frame"));
    if let Some(badge) = badge.filter(|b| has_icon(b)) {
        shot = shot.child(
            div()
                .absolute()
                .top(px(6.))
                .left(px(6.))
                .size(px(38.))
                .flex()
                .items_center()
                .justify_center()
                .child(frame("tile"))
                .child(div().relative().child(icon(badge, 26.))),
        );
    }
    shot = if chosen {
        shot.child(frame("tab_active"))
    } else {
        shot.child(hover_outline("rom-card"))
    };
    div()
        .id(id)
        .group("rom-card")
        .flex()
        .flex_col()
        .gap(px(4.))
        .cursor_pointer()
        .child(shot)
        .child(
            div()
                .text_size(px(13.))
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(if chosen { CYAN } else { GOLD }))
                .child(SharedString::from(name.to_uppercase())),
        )
        .child(
            div()
                .text_size(px(11.))
                .line_height(px(14.))
                .text_color(rgb(if chosen { TEXT } else { MUTED }))
                .child(line.into()),
        )
}

/// A HUD icon (gold glyph) at `size` pixels.
pub fn icon(name: &str, size: f32) -> Div {
    div()
        .w(px(size))
        .h(px(size))
        .flex_shrink_0()
        .children(chrome().icon(name).map(|i| picture(i, ObjectFit::Contain)))
}

/// Whether the HUD has icon `name`.
pub fn has_icon(name: &str) -> bool {
    hud_source(&format!("icons/{name}")).is_some()
}

/// The capture behind the home screen, dimmed so the panels read over it.
pub fn backdrop() -> Div {
    let art = chrome().picture("backdrop");
    div()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .bottom_0()
        .bg(rgb(NAVY0))
        .children(art.map(|a| {
            div()
                .absolute()
                .size_full()
                .child(picture(a, ObjectFit::Cover))
        }))
        .child(div().absolute().size_full().bg(gpui::linear_gradient(
            180.,
            gpui::linear_color_stop(rgba(0x0c1118a6), 0.),
            gpui::linear_color_stop(rgba(0x0c1118e6), 1.),
        )))
}

/// The home screen's hero band: a wide capture in a gold frame, darkened towards its left
/// where the emblem stands.
pub fn hero(height: f32) -> Div {
    let art = chrome().picture("hero");
    div()
        .relative()
        .w_full()
        .h(px(height))
        .overflow_hidden()
        .bg(rgb(SLOT))
        .children(art.map(|a| {
            div()
                .absolute()
                .size_full()
                .child(picture(a, ObjectFit::Cover))
        }))
        .child(div().absolute().size_full().bg(gpui::linear_gradient(
            90.,
            gpui::linear_color_stop(rgba(0x0c1118e0), 0.),
            gpui::linear_color_stop(rgba(0x0c111800), 0.75),
        )))
        .child(frame("frame"))
}

/// The title emblem (globe, laurels, plaque) with ASCENSION and RISE OF MAN on it, `width`
/// pixels wide; the HUD's own artwork and text placement.
pub fn emblem(width: f32) -> Div {
    let k = width / 250.0;
    let art = chrome().image("emblem");
    div()
        .relative()
        .w(px(width))
        .h(px(135.0 * k))
        .flex_shrink_0()
        .children(art.map(|a| {
            div()
                .absolute()
                .size_full()
                .child(picture(a, ObjectFit::Fill))
        }))
        .child(
            div()
                .absolute()
                .left(px(34.0 * k))
                .top(px(44.0 * k))
                .w(px(182.0 * k))
                .h(px(30.0 * k))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(24.0 * k))
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(GOLD))
                .child("ASCENSION"),
        )
        .child(
            div()
                .absolute()
                .left(px(38.0 * k))
                .top(px(76.0 * k))
                .w(px(174.0 * k))
                .h(px(20.0 * k))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(10.5 * k))
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(TEXT))
                .child("RISE OF MAN"),
        )
}

/// A labelled field of a form: the label (and its help, muted) above the control.
pub fn field(
    text: impl Into<SharedString>,
    help: Option<SharedString>,
    control: impl IntoElement,
) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(6.))
        .child(
            div()
                .flex()
                .items_end()
                .gap(px(10.))
                .child(label(text).flex_shrink_0())
                .children(help.map(|h| hint(h).flex_1().min_w_0())),
        )
        .child(control)
}

/// A text card for a choice without a picture (a race): a name and a line; the chosen one
/// glows cyan.
pub fn option_card(
    id: impl Into<ElementId>,
    name: impl Into<SharedString>,
    line: impl Into<SharedString>,
    chosen: bool,
) -> Stateful<Div> {
    let name: SharedString = name.into();
    let mut card = div()
        .id(id)
        .group("rom-choice")
        .relative()
        .flex()
        .flex_col()
        .justify_center()
        .gap(px(2.))
        .min_h(px(52.))
        .px(px(12.))
        .py(px(8.))
        .cursor_pointer()
        .child(frame("cell"))
        .child(
            div()
                .relative()
                .text_size(px(13.))
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(if chosen { CYAN } else { GOLD }))
                .child(SharedString::from(name.to_uppercase())),
        )
        .child(
            div()
                .relative()
                .text_size(px(11.))
                .text_color(rgb(if chosen { TEXT } else { MUTED }))
                .child(line.into()),
        );
    card = if chosen {
        card.child(frame("tab_active"))
    } else {
        card.child(hover_outline("rom-choice"))
    };
    card
}

/// An entry of a side list (the advanced launcher's groups), as the HUD's category tabs.
pub fn tab(
    id: impl Into<ElementId>,
    glyph: &str,
    text: impl Into<SharedString>,
    active: bool,
) -> Stateful<Div> {
    let text: SharedString = text.into();
    let mut item = div()
        .id(id)
        .group("rom-choice")
        .relative()
        .flex()
        .flex_shrink_0()
        .items_center()
        .gap(px(8.))
        .h(px(38.))
        .pl(px(10.))
        .pr(px(14.))
        .cursor_pointer()
        .child(frame("cell"))
        .child(div().relative().child(icon(glyph, 22.)))
        .child(
            div()
                .relative()
                .text_size(px(12.))
                .font_weight(FontWeight::BOLD)
                .whitespace_nowrap()
                .text_color(rgb(if active { TEXT } else { GOLD }))
                .child(SharedString::from(text.to_uppercase())),
        );
    item = if active {
        item.child(frame("tab_active"))
    } else {
        item.child(hover_outline("rom-choice"))
    };
    item
}

/// The home screen's one obvious button: a large gold plate.
pub fn play_button(
    id: impl Into<ElementId>,
    text: impl Into<SharedString>,
    enabled: bool,
) -> Stateful<Div> {
    let text: SharedString = text.into();
    let mut button = div()
        .id(id)
        .group("rom-button")
        .relative()
        .flex()
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .h(px(64.))
        .w(px(260.))
        .child(frame("button_gold"))
        .child(
            div()
                .relative()
                .text_size(px(28.))
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(ON_GOLD))
                .child(SharedString::from(text.to_uppercase())),
        );
    if enabled {
        button = button.cursor_pointer().child(hover_outline("rom-button"));
    } else {
        button = button.opacity(0.4);
    }
    button
}

/// A status line: amber when something is wrong, cyan when ready.
pub fn status(text: impl Into<SharedString>, wrong: bool) -> Div {
    div()
        .text_size(px(13.))
        .text_color(rgb(if wrong { WARN } else { CYAN }))
        .child(text.into())
}

/// The screen background without a picture: the HUD's navy.
pub fn ground() -> Div {
    div()
        .size_full()
        .relative()
        .bg(rgb(NAVY0))
        .text_color(rgb(TEXT))
        .font_family(FONT)
        .text_size(px(14.))
}

/// The version, small and muted, as the HUD draws it.
pub fn version() -> Div {
    hint(SharedString::from(crate::args::VERSION))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_source_parses_and_slices() {
        for (name, source) in LAUNCHER_SVG {
            assert!(
                usvg::Tree::from_str(source, &usvg::Options::default()).is_ok(),
                "{name}"
            );
        }
        for (name, _, _) in CUTS {
            assert!(svg(name).is_some(), "{name}");
        }
        let chrome = Chrome::bake();
        for (name, _, _) in CUTS {
            let slices = &chrome.frames[name];
            assert_eq!(slices.parts.len(), 9, "{name}");
            // Corners are drawn at their own size: no stretching.
            let corner = slices.parts[0].size(0);
            assert_eq!(corner.width.0 as f32, slices.edge[0] * OVERSAMPLE, "{name}");
            assert_eq!(
                corner.height.0 as f32,
                slices.edge[1] * OVERSAMPLE,
                "{name}"
            );
        }
        assert!(chrome.image("emblem").is_some());
        assert!(chrome.icon("tab_military").is_some() && chrome.icon("no_such").is_none());
        for (name, _) in PICTURES {
            assert!(chrome.picture(name).is_some(), "{name}");
        }
    }

    #[test]
    fn colours_are_bgra() {
        let mut image = RgbaImage::new(1, 1);
        image.put_pixel(0, 0, rom_render::image::Rgba([1, 2, 3, 4]));
        let render = to_render(image);
        assert_eq!(render.as_bytes(0), Some(&[3, 2, 1, 4][..]));
    }
}
