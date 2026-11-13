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

pub use crate::hud_model::{
    ButtonState, HudButton, HudCard, HudCityRadius, HudMarker, HudPalette, HudPlacement, HudQueue,
    HudQueueItem, HudSelection, HudTile, MarkerKind, PaletteTab, ResearchCell, ResearchOverview,
    ResearchTrack, SLOT_KEYS,
};
use crate::pick::HudTarget;

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
    /// Command palette of the selection, absent when nothing can be commanded.
    pub palette: Option<HudPalette>,
    /// Production queue of the single selected producer.
    pub queue: Option<HudQueue>,
    /// Ids of the selected entities, ascending; the world view rings and outlines them.
    pub selected: Vec<u32>,
    /// Order and rally markers on the ground.
    pub markers: Vec<HudMarker>,
    /// Selected data tab (0..4; 4 is research).
    pub data_tab: usize,
    /// Research tracks and age pips.
    pub overview: ResearchOverview,
    /// City circles on the current surface.
    pub city_radii: Vec<HudCityRadius>,
    /// The building footprint being placed.
    pub placement: Option<HudPlacement>,
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
            palette: None,
            queue: None,
            selected: Vec::new(),
            markers: Vec::new(),
            placement: None,
            data_tab: 0,
            overview: ResearchOverview::default(),
            city_radii: Vec::new(),
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
    /// Desaturated artwork atlas.
    Greyscale,
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

/// Per-frame pointer state the HUD draws on top of the snapshot.
#[derive(Clone, Copy, Debug, Default)]
pub struct HudOverlay {
    /// The HUD element under the pointer.
    pub hover: Option<HudTarget>,
    /// The selection rectangle being dragged, in the same pixels as the layout.
    pub drag_box: Option<Rect>,
}

/// CPU artwork and font atlases with the common HUD composition code.
pub struct HudPainter {
    /// Baked vector artwork.
    pub artwork: AtlasImage,
    /// Desaturated artwork, baked once per scale.
    pub greyscale: Vec<u8>,
    /// Baked glyphs.
    pub glyphs: GlyphAtlas,
    quads: Vec<HudQuad>,
    scale: f32,
}

impl HudPainter {
    /// Bakes the embedded artwork and text at one scale, without a graphics context.
    pub fn bake(scale: f32) -> Result<Self, AtlasError> {
        let artwork = hud_atlas::bake(scale)?;
        let greyscale = hud_atlas::greyscale(&artwork.rgba);
        Ok(Self {
            artwork,
            greyscale,
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

    fn solid(&mut self, dest: Rect, colour: [u8; 4]) {
        self.quads.push(HudQuad {
            texture: HudTexture::Solid,
            source: Rect::new(0.0, 0.0, 1.0, 1.0),
            dest,
            colour,
        });
    }

    fn frame(&mut self, r: Rect, t: f32, colour: [u8; 4]) {
        self.solid(Rect::new(r.x, r.y, r.w, t), colour);
        self.solid(Rect::new(r.x, r.bottom() - t, r.w, t), colour);
        self.solid(Rect::new(r.x, r.y + t, t, r.h - 2.0 * t), colour);
        self.solid(Rect::new(r.right() - t, r.y + t, t, r.h - 2.0 * t), colour);
    }

    fn bar(&mut self, r: Rect, fraction: f32, colour: [u8; 4]) {
        self.solid(r, [8, 11, 16, 255]);
        self.solid(
            Rect::new(r.x, r.y, r.w * fraction.clamp(0.0, 1.0), r.h),
            colour,
        );
    }

    fn health_colour(fraction: f32) -> [u8; 4] {
        if fraction > 0.5 {
            GREEN
        } else if fraction > 0.25 {
            AMBER
        } else {
            [230, 60, 45, 255]
        }
    }

    /// Two-line rollover help: the hovered item's name, then its price and blocking reason.
    fn help_block(&mut self, l: &HudLayout, h: &HudSnapshot, overlay: &HudOverlay) {
        use crate::pick::HudRegionKind as K;
        let (first, second) = match overlay.hover {
            Some(HudTarget {
                region: K::CommandSlot,
                index,
            }) => h
                .palette
                .as_ref()
                .and_then(|p| p.buttons.iter().find(|b| b.slot == index))
                .map_or_else(Default::default, |b| {
                    let mut line = b.price.clone();
                    if !b.state.reason().is_empty() {
                        if !line.is_empty() {
                            line.push_str("   ");
                        }
                        line.push_str(&b.state.reason());
                    }
                    (b.name.clone(), line)
                }),
            Some(HudTarget {
                region: K::QueueItem,
                index,
            }) => h
                .queue
                .as_ref()
                .and_then(|q| q.items.get(index))
                .map_or_else(Default::default, |item| {
                    (item.name.clone(), "Right-click to cancel".to_owned())
                }),
            Some(HudTarget {
                region: K::SelectionTile,
                index,
            }) => match &h.selection {
                HudSelection::Multiple { tiles, .. } => {
                    tiles.get(index).map_or_else(Default::default, |t| {
                        (t.name.clone(), "Click to select only these".to_owned())
                    })
                }
                _ => Default::default(),
            },
            Some(HudTarget {
                region: K::PaletteTab,
                index,
            }) => (
                ["Build", "Orders", "Train", "Research"][index.min(3)].to_owned(),
                String::new(),
            ),
            _ => Default::default(),
        };
        let (first, second) = if first.is_empty() {
            let text = if h.help.is_empty() {
                "Rollover an item for help and details..."
            } else {
                &h.help
            };
            (text.to_owned(), String::new())
        } else {
            (first, second)
        };
        let right = if h.palette.as_ref().is_some_and(|p| p.pages > 1) {
            110.0
        } else {
            0.0
        };
        if second.is_empty() {
            self.caption(
                l,
                Group::BottomLeft,
                &first,
                Rect::new(68.0, 651.0, 520.0 - right, 18.0),
                WHITE,
            );
        } else {
            self.caption(
                l,
                Group::BottomLeft,
                &first,
                Rect::new(68.0, 645.0, 520.0 - right, 16.0),
                WHITE,
            );
            self.caption(
                l,
                Group::BottomLeft,
                &second,
                Rect::new(68.0, 662.0, 520.0, 16.0),
                GOLD,
            );
        }
        if let Some(palette) = h.palette.as_ref().filter(|p| p.pages > 1) {
            let label = format!(
                "{} {}/{}",
                palette.page_name.to_uppercase(),
                palette.page + 1,
                palette.pages
            );
            self.text(
                &label,
                l.map(Group::BottomLeft, Rect::new(480.0, 645.0, 118.0, 16.0)),
                TextSize::Small,
                Align::Right,
                CYAN,
            );
        }
    }

    /// Tabs, page arrows and the command grid with its buttons.
    fn palette_block(&mut self, l: &HudLayout, h: &HudSnapshot, overlay: &HudOverlay) {
        use crate::pick::HudRegionKind as K;
        let s = l.scale;
        let palette = h.palette.as_ref();
        let active = palette.map_or(PaletteTab::Build, |p| p.tab);
        self.art("chrome/tab_active", l.palette_tab(active.index()));
        for (i, (icon, label)) in [
            ("build", "BUILD"),
            ("order", "ORDER"),
            ("train", "TRAIN"),
            ("research", "RESEARCH"),
        ]
        .into_iter()
        .enumerate()
        {
            let r = l.palette_tab(i);
            let enabled = palette.is_none_or(|p| p.tabs[i]);
            self.art(
                &format!("icons/tab_{icon}"),
                Rect::new(r.x + 9.0 * s, r.y + 8.0 * s, 23.0 * s, 23.0 * s),
            );
            self.text(
                label,
                Rect::new(r.x + 38.0 * s, r.y + 10.0 * s, 83.0 * s, 17.0 * s),
                TextSize::Small,
                Align::Left,
                if enabled { WHITE } else { MUTED },
            );
            if !enabled {
                self.solid(
                    Rect::new(r.x + 4.0 * s, r.y + 4.0 * s, r.w - 8.0 * s, r.h - 8.0 * s),
                    [6, 9, 14, 140],
                );
            }
            if overlay.hover
                == Some(HudTarget {
                    region: K::PaletteTab,
                    index: i,
                })
            {
                self.frame(r, 1.5 * s, GOLD);
            }
        }
        for (i, name) in ["arrow_left", "arrow_right"].into_iter().enumerate() {
            let x = if i == 0 { 26.0 } else { 578.0 };
            let r = l.map(Group::BottomLeft, Rect::new(x, 700.0, 20.0, 23.0));
            self.art(&format!("icons/{name}"), r);
            if palette.is_none_or(|p| p.pages < 2) {
                self.solid(r, [6, 9, 14, 150]);
            }
        }
        for row in 0..3 {
            for col in 0..5 {
                self.art("chrome/tile", l.command_slot(col, row));
            }
        }
        let Some(palette) = palette else {
            return;
        };
        for b in &palette.buttons {
            let r = l.command_slot(b.slot % 5, b.slot / 5);
            let icon = 34.0 * s;
            let start = self.quads.len();
            if matches!(b.state, ButtonState::Locked { .. }) {
                self.art("chrome/tile", r);
            }
            self.art(
                &b.icon,
                Rect::new(
                    r.x + (r.w - icon) / 2.0,
                    r.y + (r.h - icon) / 2.0 + 2.0 * s,
                    icon,
                    icon,
                ),
            );
            if matches!(b.state, ButtonState::Locked { .. }) {
                for quad in &mut self.quads[start..] {
                    quad.texture = HudTexture::Greyscale;
                    quad.colour = [195, 195, 195, 255];
                }
            }
            self.text(
                &SLOT_KEYS[b.slot.min(14)].to_string(),
                Rect::new(r.x + 5.0 * s, r.y + 3.0 * s, 22.0 * s, 12.0 * s),
                TextSize::Small,
                Align::Left,
                if b.state.enabled() { WHITE } else { MUTED },
            );
            let inner = Rect::new(r.x + 3.0 * s, r.y + 3.0 * s, r.w - 6.0 * s, r.h - 6.0 * s);
            match b.state {
                ButtonState::Unaffordable { .. } => self.solid(inner, [6, 9, 14, 165]),
                ButtonState::Done => self.art(
                    "icons/badge_done",
                    Rect::new(r.right() - 22.0 * s, r.y + 2.0 * s, 20.0 * s, 20.0 * s),
                ),
                ButtonState::InProgress { pct } => self.bar(
                    Rect::new(inner.x, inner.bottom() - 4.0 * s, inner.w, 3.0 * s),
                    pct.min(100) as f32 / 100.0,
                    CYAN,
                ),
                _ => {}
            }
            if b.armed {
                self.frame(r, 2.0 * s, CYAN);
            } else if overlay.hover
                == Some(HudTarget {
                    region: K::CommandSlot,
                    index: b.slot,
                })
            {
                self.frame(r, 1.5 * s, if b.state.enabled() { GOLD } else { MUTED });
            }
        }
    }

    /// Rows of the production queue: icon, name and progress bar.
    fn queue_block(&mut self, l: &HudLayout, h: &HudSnapshot, overlay: &HudOverlay) {
        use crate::pick::HudRegionKind as K;
        let s = l.scale;
        let Some(queue) = &h.queue else {
            return;
        };
        for (i, item) in queue.items.iter().take(3).enumerate() {
            let slot = l.queue_slot(i);
            self.art("chrome/tile", slot);
            let icon = slot.h - 6.0 * s;
            self.art(
                &item.icon,
                Rect::new(slot.x + (slot.w - icon) / 2.0, slot.y + 3.0 * s, icon, icon),
            );
            let y = 772.0 + i as f32 * 49.0;
            self.caption(
                l,
                Group::BottomLeft,
                &item.name,
                Rect::new(434.0, y, 160.0, 17.0),
                WHITE,
            );
            let bar = l.map(Group::BottomLeft, Rect::new(434.0, y + 24.0, 160.0, 8.0));
            self.bar(bar, item.progress_pct.min(100) as f32 / 100.0, CYAN);
            if overlay.hover
                == Some(HudTarget {
                    region: K::QueueItem,
                    index: i,
                })
            {
                self.frame(l.queue_row(i), 1.5 * s, GOLD);
            }
        }
        if queue.more > 0 {
            self.text(
                &format!("+{}", queue.more),
                l.map(Group::BottomLeft, Rect::new(540.0, 741.0, 52.0, 19.0)),
                TextSize::Small,
                Align::Right,
                GOLD,
            );
        }
    }

    /// Title, portrait, stat rows or multi-selection grid.
    fn selection_block(&mut self, l: &HudLayout, h: &HudSnapshot, overlay: &HudOverlay) {
        use crate::pick::HudRegionKind as K;
        use Group::BottomRight as R;
        let s = l.scale;
        match &h.selection {
            HudSelection::None => {
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
            }
            HudSelection::Single(card) => {
                self.text(
                    "SELECTION",
                    l.map(R, Rect::new(998.0, 681.0, 320.0, 21.0)),
                    TextSize::Large,
                    Align::Centre,
                    WHITE,
                );
                let tile = l.map(R, Rect::new(1008.0, 708.0, 104.0, 94.0));
                self.art("chrome/tile", tile);
                let icon = 64.0 * s;
                self.art(
                    &card.icon,
                    Rect::new(
                        tile.x + (tile.w - icon) / 2.0,
                        tile.y + (tile.h - icon) / 2.0,
                        icon,
                        icon,
                    ),
                );
                self.text(
                    &card.name,
                    l.map(R, Rect::new(995.0, 808.0, 125.0, 18.0)),
                    TextSize::Small,
                    Align::Centre,
                    WHITE,
                );
                let note = if card.foreign {
                    "Foreign".to_owned()
                } else if let Some(pct) = card.progress_pct {
                    format!("Building {pct}%")
                } else {
                    String::new()
                };
                if !note.is_empty() {
                    self.text(
                        &note,
                        l.map(R, Rect::new(995.0, 828.0, 125.0, 16.0)),
                        TextSize::Small,
                        Align::Centre,
                        if card.foreign { AMBER } else { CYAN },
                    );
                }
                let fraction = card.hp.0 as f32 / card.hp.1.max(1) as f32;
                self.bar(
                    l.map(R, Rect::new(1008.0, 850.0, 104.0, 8.0)),
                    fraction,
                    Self::health_colour(fraction),
                );
                let rows = [
                    (
                        "stat_hp",
                        "HP",
                        Some(format!("{} / {}", card.hp.0, card.hp.1)),
                    ),
                    ("stat_attack", "ATTACK", card.attack.clone()),
                    ("stat_armour", "ARMOUR", card.armour.clone()),
                    ("stat_range", "RANGE", card.range.clone()),
                    ("stat_garrison", "GARRISON", card.garrison.clone()),
                    ("stat_workers", "WORKERS", card.workers.clone()),
                ];
                let mut row = 0.0;
                for (icon, label, value) in rows {
                    let Some(value) = value else {
                        continue;
                    };
                    let y = 708.0 + row * 24.0;
                    self.art(
                        &format!("icons/{icon}"),
                        l.map(R, Rect::new(1138.0, y, 20.0, 20.0)),
                    );
                    self.caption(l, R, label, Rect::new(1164.0, y + 2.0, 72.0, 16.0), GOLD);
                    self.text(
                        &value,
                        l.map(R, Rect::new(1236.0, y + 2.0, 82.0, 16.0)),
                        TextSize::Small,
                        Align::Right,
                        WHITE,
                    );
                    row += 1.0;
                }
            }
            HudSelection::Multiple { total, tiles } => {
                self.text(
                    "MULTIPLE SELECTION",
                    l.map(R, Rect::new(1006.0, 681.0, 220.0, 21.0)),
                    TextSize::Large,
                    Align::Left,
                    WHITE,
                );
                self.text(
                    &format!("{total} selected"),
                    l.map(R, Rect::new(1226.0, 684.0, 92.0, 17.0)),
                    TextSize::Small,
                    Align::Right,
                    GOLD,
                );
                for (i, tile) in tiles.iter().take(12).enumerate() {
                    let r = l.multi_tile(i);
                    self.art("chrome/tile", r);
                    let icon = 36.0 * s;
                    self.art(
                        &tile.icon,
                        Rect::new(r.x + (r.w - icon) / 2.0, r.y + 5.0 * s, icon, icon),
                    );
                    if tile.count > 1 {
                        self.text(
                            &format!("x{}", tile.count),
                            Rect::new(r.right() - 34.0 * s, r.y + 4.0 * s, 28.0 * s, 14.0 * s),
                            TextSize::Small,
                            Align::Right,
                            WHITE,
                        );
                    }
                    let fraction = tile.hp_permille as f32 / 1000.0;
                    self.bar(
                        Rect::new(
                            r.x + 6.0 * s,
                            r.bottom() - 11.0 * s,
                            r.w - 12.0 * s,
                            5.0 * s,
                        ),
                        fraction,
                        Self::health_colour(fraction),
                    );
                    if overlay.hover
                        == Some(HudTarget {
                            region: K::SelectionTile,
                            index: i,
                        })
                    {
                        self.frame(r, 1.5 * s, GOLD);
                    }
                }
            }
        }
    }

    fn research_overview(&mut self, l: &HudLayout, overview: &ResearchOverview) {
        use Group::BottomRight as R;
        let cell = |this: &mut Self, rect: Rect, state: ResearchCell| {
            let colour = match state {
                ResearchCell::Done => GOLD,
                ResearchCell::InProgress => CYAN,
                ResearchCell::Next => WHITE,
                ResearchCell::Locked => [72, 80, 90, 255],
            };
            if state == ResearchCell::Next {
                this.frame(rect, l.scale.max(1.0), colour);
            } else {
                this.solid(rect, colour);
            }
        };
        for (row, track) in overview.tracks.iter().enumerate() {
            let y = 702.0 + row as f32 * 35.0;
            self.caption(l, R, &track.label, Rect::new(1492.0, y, 150.0, 16.0), WHITE);
            for (col, state) in track.levels.iter().enumerate() {
                cell(
                    self,
                    l.map(R, Rect::new(1493.0 + col as f32 * 11.0, y + 19.0, 8.0, 8.0)),
                    *state,
                );
            }
        }
        self.caption(l, R, "AGES", Rect::new(1492.0, 849.0, 150.0, 16.0), WHITE);
        for (col, state) in overview.ages.iter().enumerate() {
            cell(
                self,
                l.map(R, Rect::new(1493.0 + col as f32 * 10.0, 872.0, 7.0, 7.0)),
                *state,
            );
        }
    }

    /// Composes the complete chrome and live text, in drawing order.
    pub fn compose(&mut self, l: &HudLayout, h: &HudSnapshot) -> &[HudQuad] {
        self.compose_with(l, h, &HudOverlay::default())
    }

    /// Like [`Self::compose`], with the pointer hover and drag box of the current frame.
    pub fn compose_with(
        &mut self,
        l: &HudLayout,
        h: &HudSnapshot,
        overlay: &HudOverlay,
    ) -> &[HudQuad] {
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
        self.help_block(l, h, overlay);
        self.palette_block(l, h, overlay);
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
        self.queue_block(l, h, overlay);
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
        self.selection_block(l, h, overlay);
        self.art(
            "chrome/tab_active",
            l.map(
                R,
                Rect::new(1350.0, 695.0 + h.data_tab.min(4) as f32 * 42.0, 132.0, 38.0),
            ),
        );
        for (i, (icon, label)) in [
            ("political", "POLITICAL"),
            ("military", "MILITARY"),
            ("economy", "ECONOMY"),
            ("diplomacy", "DIPLOMACY"),
            ("research", "RESEARCH"),
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
        if h.data_tab == 4 {
            self.research_overview(l, &h.overview);
        }
        if let Some(drag) = overlay.drag_box {
            self.solid(drag, [63, 208, 255, 36]);
            self.frame(drag, (1.5 * l.scale).max(1.0), CYAN);
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
