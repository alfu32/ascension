//! Plain HUD snapshots and shared composition for GPU drawing and CPU previews.

use crate::hud_atlas::{self, AtlasError, AtlasImage};
use crate::hud_layout::{Group, HudLayout, Rect, Region};
use crate::hud_text::{Align, GlyphAtlas, TextSize, TextStyle};

/// One resource's stock and income per configured simulation income period.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HudResource {
    /// Atlas icon name.
    pub icon: String,
    /// Display name.
    pub label: String,
    /// Current stock.
    pub amount: i64,
    /// Effective income per period (not per real second).
    pub rate: i64,
    /// True when commerce limits the rate.
    pub capped: bool,
    /// False before the resource's first age.
    pub visible: bool,
}

/// Research currently advancing at one producer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HudResearch {
    /// Technology display name.
    pub label: String,
    /// Completion percentage, 0 through 100.
    pub progress_pct: u32,
}

/// Selection details; additional variants belong to the selection task.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum HudSelection {
    /// No selected entities.
    #[default]
    None,
}

/// Immutable match interface data, published by the simulation thread.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HudSnapshot {
    /// Completed game ticks.
    pub game_ticks: u64,
    /// Fixed game ticks per game second, independent of speed.
    pub ticks_per_second: u32,
    /// Base resources in rules id order.
    pub resources: Vec<HudResource>,
    /// Highest commerce-limited resource income and commerce cap, per period.
    pub commerce: (i64, i64),
    /// Used population and population cap.
    pub population: (i64, i64),
    /// City count and limit.
    pub cities: (i64, i64),
    /// Active research in executor id order.
    pub research: Vec<HudResearch>,
    /// Recent event messages, oldest first.
    pub events: Vec<String>,
    /// Rollover help, or empty for the default.
    pub help: String,
    /// Current selection.
    pub selection: HudSelection,
}

impl Default for HudSnapshot {
    fn default() -> Self {
        Self {
            game_ticks: 0,
            ticks_per_second: 20,
            resources: Vec::new(),
            commerce: (0, 0),
            population: (0, 0),
            cities: (0, 0),
            research: Vec::new(),
            events: Vec::new(),
            help: String::new(),
            selection: HudSelection::None,
        }
    }
}

const WHITE: [u8; 4] = [242, 242, 242, 255];
const MUTED: [u8; 4] = [151, 165, 177, 255];
const GOLD: [u8; 4] = [232, 201, 138, 255];
const CYAN: [u8; 4] = [63, 208, 255, 255];
const GREEN: [u8; 4] = [61, 220, 61, 255];
const AMBER: [u8; 4] = [240, 176, 48, 255];

/// Texture selected by one composed quad.
#[derive(Clone, Copy, Debug)]
pub enum HudTexture {
    /// SVG artwork atlas.
    Artwork,
    /// Font atlas.
    Glyphs,
    /// Single white pixel, tinted by the quad colour.
    Solid,
}

/// One textured rectangle used by both HUD rendering paths.
#[derive(Clone, Copy, Debug)]
pub struct HudQuad {
    /// Texture to sample.
    pub texture: HudTexture,
    /// Source pixels in that texture.
    pub source: Rect,
    /// Destination screen rectangle.
    pub dest: Rect,
    /// Straight RGBA tint.
    pub colour: [u8; 4],
}

/// CPU artwork and font atlases with the common HUD composition code.
pub struct HudPainter {
    /// Baked vector artwork.
    pub artwork: AtlasImage,
    /// Baked glyphs.
    pub glyphs: GlyphAtlas,
    quads: Vec<HudQuad>,
    scale: f32,
}

impl HudPainter {
    /// Bakes the embedded artwork and text at one scale, without a graphics context.
    pub fn bake(scale: f32) -> Result<Self, AtlasError> {
        Ok(Self {
            artwork: hud_atlas::bake(scale)?,
            glyphs: GlyphAtlas::bake(scale),
            quads: Vec::new(),
            scale,
        })
    }

    fn art(&mut self, name: &str, dest: Rect) {
        if let Some(r) = self.artwork.rect(name) {
            self.quads.push(HudQuad {
                texture: HudTexture::Artwork,
                source: Rect::new(r.x as f32, r.y as f32, r.w as f32, r.h as f32),
                dest,
                colour: WHITE,
            });
        }
    }

    fn text(&mut self, text: &str, area: Rect, size: TextSize, align: Align, colour: [u8; 4]) {
        self.text_scaled(text, area, (size, 1.0), align, colour);
    }

    fn text_scaled(
        &mut self,
        text: &str,
        area: Rect,
        style: (TextSize, f32),
        align: Align,
        colour: [u8; 4],
    ) {
        let (size, factor) = style;
        // Fit long labels and help to their own field; numbers retain their right anchor.
        let ratio = self.scale / self.glyphs.scale * factor;
        let mut label = text.to_owned();
        let max_width = area.w / ratio;
        if self.glyphs.measure(&label, size) as f32 > max_width {
            while !label.is_empty()
                && self.glyphs.measure(&(label.clone() + "..."), size) as f32 > max_width
            {
                label.pop();
            }
            label.push_str("...");
        }
        let x = match align {
            Align::Left => area.x,
            Align::Centre => area.centre().0,
            Align::Right => area.right(),
        };
        let mut glyphs = Vec::new();
        self.glyphs.draw(
            &label,
            TextStyle {
                size,
                align,
                colour,
            },
            0,
            0,
            &mut glyphs,
        );
        let baseline = area.y
            + (area.h - self.glyphs.line_height(size) as f32 * ratio) / 2.0
            + self.glyphs.ascent(size) as f32 * ratio;
        self.quads.extend(glyphs.into_iter().map(|q| HudQuad {
            texture: HudTexture::Glyphs,
            source: Rect::new(
                q.src.0 as f32,
                q.src.1 as f32,
                q.src.2 as f32,
                q.src.3 as f32,
            ),
            dest: Rect::new(
                x + q.dst.0 as f32 * ratio,
                baseline + q.dst.1 as f32 * ratio,
                q.src.2 as f32 * ratio,
                q.src.3 as f32 * ratio,
            ),
            colour: q.colour,
        }));
    }

    fn caption(&mut self, l: &HudLayout, group: Group, text: &str, rect: Rect, colour: [u8; 4]) {
        self.text(
            text,
            l.map(group, rect),
            TextSize::Small,
            Align::Left,
            colour,
        );
    }

    /// Composes the complete chrome and live text, in drawing order.
    pub fn compose(&mut self, l: &HudLayout, h: &HudSnapshot) -> &[HudQuad] {
        use Group::{BottomCentre as C, BottomLeft as L, BottomRight as R, Top as T};
        self.quads.clear();
        self.scale = l.scale;
        self.art(
            "chrome/bar_fill",
            Rect::new(0.0, 0.0, l.width, 135.0 * l.scale),
        );
        for (name, group, rect) in [
            ("topbar", T, Rect::new(0.0, 0.0, 1672.0, 135.0)),
            ("bottom_left", L, Rect::new(0.0, 570.0, 612.0, 371.0)),
            ("minimap", C, Region::MinimapFrame.design()),
            ("bottom_right", R, Rect::new(975.0, 662.0, 697.0, 279.0)),
            ("emblem", T, Region::Emblem.design()),
        ] {
            self.art(&format!("chrome/{name}"), l.map(group, rect));
        }
        self.text_scaled(
            "ASCENSION",
            l.map(T, Rect::new(744.0, 46.0, 182.0, 27.0)),
            (TextSize::Large, 1.5),
            Align::Centre,
            GOLD,
        );
        self.text(
            "RISE OF MAN",
            l.map(T, Rect::new(748.0, 78.0, 174.0, 20.0)),
            TextSize::Small,
            Align::Centre,
            WHITE,
        );
        let flashing = (h.game_ticks / u64::from(h.ticks_per_second.max(1))).is_multiple_of(2);
        for (i, (icon, label, (value, cap))) in [
            ("commerce", "COMMERCE", h.commerce),
            ("population", "POPULATION", h.population),
            ("cities", "CITIES", h.cities),
        ]
        .into_iter()
        .enumerate()
        {
            let y = 8.0 + i as f32 * 40.0;
            self.art(
                &format!("icons/{icon}"),
                l.map(T, Rect::new(59.0, y + 5.0, 29.0, 29.0)),
            );
            self.caption(l, T, label, Rect::new(100.0, y, 115.0, 15.0), WHITE);
            self.text(
                &format!("{} / {}", number(value), number(cap)),
                l.map(T, Rect::new(94.0, y + 17.0, 119.0, 17.0)),
                TextSize::Small,
                Align::Right,
                if value >= cap && cap > 0 && flashing {
                    AMBER
                } else {
                    WHITE
                },
            );
        }
        // Keep resource positions stable when age-gated cells are hidden.
        for (i, res) in h
            .resources
            .iter()
            .take(7)
            .enumerate()
            .filter(|(_, r)| r.visible)
        {
            let (x, y, w) = if i < 4 {
                (246.0 + i as f32 * 115.0, 16.0, 111.0)
            } else {
                (246.0 + (i - 4) as f32 * 153.0, 68.0, 150.0)
            };
            let wide = i >= 4;
            self.art(
                if wide {
                    "chrome/cell_wide"
                } else {
                    "chrome/cell"
                },
                l.map(T, Rect::new(x, y, w, 44.0)),
            );
            self.art(&res.icon, l.map(T, Rect::new(x + 4.0, y + 8.0, 27.0, 27.0)));
            self.caption(
                l,
                T,
                &res.label.to_uppercase(),
                Rect::new(x + 34.0, y + 2.0, w - 38.0, 14.0),
                WHITE,
            );
            self.text(
                &number(res.amount),
                l.map(T, Rect::new(x + 30.0, y + 23.0, w - 67.0, 15.0)),
                TextSize::Small,
                Align::Right,
                WHITE,
            );
            self.text(
                &format!("{:+}", res.rate),
                l.map(T, Rect::new(x + w - 37.0, y + 23.0, 33.0, 15.0)),
                TextSize::Small,
                Align::Right,
                if res.capped {
                    if flashing { AMBER } else { GOLD }
                } else if res.rate < 0 {
                    AMBER
                } else {
                    GREEN
                },
            );
        }
        for (i, research) in h.research.iter().take(3).enumerate() {
            let y = 16.0 + i as f32 * 35.0;
            self.art(
                "chrome/research_row",
                l.map(T, Rect::new(998.0, y, 266.0, 31.0)),
            );
            self.art(
                "icons/research",
                l.map(T, Rect::new(1004.0, y + 2.0, 25.0, 25.0)),
            );
            self.caption(
                l,
                T,
                &research.label.to_uppercase(),
                Rect::new(1044.0, y - 1.0, 216.0, 14.0),
                WHITE,
            );
            self.text(
                &format!("{}%", research.progress_pct.min(100)),
                l.map(T, Rect::new(1263.0, y, 39.0, 15.0)),
                TextSize::Small,
                Align::Right,
                WHITE,
            );
            self.quads.push(HudQuad {
                texture: HudTexture::Solid,
                source: Rect::new(0.0, 0.0, 1.0, 1.0),
                dest: l.map(
                    T,
                    Rect::new(
                        1045.0,
                        y + 19.0,
                        216.0 * research.progress_pct.min(100) as f32 / 100.0,
                        5.0,
                    ),
                ),
                colour: CYAN,
            });
        }
        self.art("chrome/clock", l.rect(Region::Clock));
        self.art("icons/clock", l.map(T, Rect::new(1369.0, 44.0, 28.0, 28.0)));
        let seconds = h.game_ticks / u64::from(h.ticks_per_second.max(1));
        let time = if seconds >= 3600 {
            format!(
                "{}:{:02}:{:02}",
                seconds / 3600,
                seconds / 60 % 60,
                seconds % 60
            )
        } else {
            format!("{:02}:{:02}", seconds / 60, seconds % 60)
        };
        self.text(
            &time,
            l.map(T, Rect::new(1357.0, 77.0, 52.0, 17.0)),
            TextSize::Small,
            Align::Centre,
            WHITE,
        );
        for (i, icon) in ["diplomacy", "chat", "menu", "pause"]
            .into_iter()
            .enumerate()
        {
            let x = 1432.0 + i as f32 * 54.0;
            self.art("chrome/button", l.map(T, Rect::new(x, 42.0, 50.0, 50.0)));
            self.art(
                &format!("icons/{icon}"),
                l.map(T, Rect::new(x + 9.0, 51.0, 32.0, 32.0)),
            );
        }
        self.art("icons/chat", l.map(L, Rect::new(24.0, 585.0, 18.0, 18.0)));
        self.caption(l, L, "[Allies]", Rect::new(50.0, 584.0, 58.0, 17.0), CYAN);
        self.caption(
            l,
            L,
            "Message...",
            Rect::new(112.0, 584.0, 251.0, 17.0),
            MUTED,
        );
        self.art("icons/event", l.map(L, Rect::new(24.0, 612.0, 18.0, 18.0)));
        if let Some(event) = h.events.last() {
            self.caption(l, L, event, Rect::new(50.0, 612.0, 312.0, 17.0), GOLD);
        }
        self.art("icons/help", l.map(L, Rect::new(32.0, 650.0, 23.0, 23.0)));
        self.caption(
            l,
            L,
            if h.help.is_empty() {
                "Rollover an item for help and details..."
            } else {
                &h.help
            },
            Rect::new(68.0, 651.0, 520.0, 18.0),
            WHITE,
        );
        self.art(
            "chrome/tab_active",
            l.map(L, Rect::new(54.0, 692.0, 126.0, 38.0)),
        );
        for (i, (icon, label)) in [
            ("build", "BUILD"),
            ("order", "ORDER"),
            ("train", "TRAIN"),
            ("research", "RESEARCH"),
        ]
        .into_iter()
        .enumerate()
        {
            let x = 54.0 + i as f32 * 130.0;
            self.art(
                &format!("icons/tab_{icon}"),
                l.map(L, Rect::new(x + 9.0, 700.0, 23.0, 23.0)),
            );
            self.caption(l, L, label, Rect::new(x + 38.0, 702.0, 83.0, 17.0), WHITE);
        }
        for (x, name) in [(26.0, "arrow_left"), (578.0, "arrow_right")] {
            self.art(
                &format!("icons/{name}"),
                l.map(L, Rect::new(x, 700.0, 20.0, 23.0)),
            );
        }
        for row in 0..3 {
            for col in 0..5 {
                self.art("chrome/tile", l.command_slot(col, row));
            }
        }
        self.art(
            "icons/stat_workers",
            l.map(L, Rect::new(384.0, 740.0, 23.0, 23.0)),
        );
        self.caption(
            l,
            L,
            "PRODUCTION QUEUE",
            Rect::new(412.0, 741.0, 179.0, 19.0),
            WHITE,
        );
        for (i, icon) in ["zoom_in", "zoom_out", "centre", "flag"]
            .into_iter()
            .enumerate()
        {
            let y = 702.0 + i as f32 * 43.0;
            self.art("chrome/button", l.map(C, Rect::new(636.0, y, 38.0, 38.0)));
            self.art(
                &format!("icons/{icon}"),
                l.map(C, Rect::new(642.0, y + 6.0, 26.0, 26.0)),
            );
        }
        self.art(
            "icons/objectives",
            l.map(C, Rect::new(742.0, 896.0, 24.0, 24.0)),
        );
        self.caption(
            l,
            C,
            "OBJECTIVES",
            Rect::new(776.0, 898.0, 117.0, 20.0),
            WHITE,
        );
        self.text(
            "SELECTION",
            l.map(R, Rect::new(998.0, 681.0, 320.0, 21.0)),
            TextSize::Large,
            Align::Centre,
            WHITE,
        );
        self.caption(
            l,
            R,
            "No selection",
            Rect::new(1142.0, 751.0, 171.0, 20.0),
            MUTED,
        );
        self.art(
            "chrome/tab_active",
            l.map(R, Rect::new(1350.0, 695.0, 132.0, 38.0)),
        );
        for (i, (icon, label)) in [
            ("political", "POLITICAL"),
            ("military", "MILITARY"),
            ("economy", "ECONOMY"),
            ("diplomacy", "DIPLOMACY"),
            ("wonders", "WONDERS"),
        ]
        .into_iter()
        .enumerate()
        {
            let y = 695.0 + i as f32 * 42.0;
            self.art(
                &format!("icons/tab_{icon}"),
                l.map(R, Rect::new(1359.0, y + 7.0, 24.0, 24.0)),
            );
            self.caption(l, R, label, Rect::new(1392.0, y + 10.0, 85.0, 16.0), GOLD);
        }
        &self.quads
    }
}

fn number(value: i64) -> String {
    let digits = value.unsigned_abs().to_string();
    let mut result = String::new();
    if value < 0 {
        result.push('-');
    }
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            result.push(',');
        }
        result.push(ch);
    }
    result
}
