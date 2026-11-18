//! Plain HUD snapshots and shared composition for GPU drawing and CPU previews.

use std::collections::BTreeMap;

use crate::hud_atlas::{self, AtlasError, AtlasImage, AtlasRect};
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
    HudQueueItem, HudSelection, HudTile, IconMark, MarkerKind, PaletteTab, ResearchCell,
    ResearchOverview, ResearchTrack, SLOT_KEYS,
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
    /// The entity the newest event is about (a ship that departed or arrived): clicking the
    /// event line flies the camera there.
    pub event_about: Option<u32>,
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
    /// Own wealth economy totals.
    pub wealth: crate::HudWealth,
    /// Selected traders' routes on the current surface.
    pub trade_routes: Vec<crate::HudTradeRoute>,
    /// Selected units' patrols on the current surface.
    pub patrols: Vec<crate::HudPatrol>,
    /// Cities the selected trader could run a route to, ringed while a partner is picked.
    pub trade_targets: Vec<u32>,
    /// The building footprint being placed.
    pub placement: Option<HudPlacement>,
    /// Recent blasts of area weapons on the shown surface (task 75).
    pub blasts: Vec<crate::HudBlast>,
    /// Buildings that fell lately on the shown surface (task 75).
    pub wrecks: Vec<crate::HudWreck>,
    /// Stains left where buildings and units were destroyed, on the shown surface where the
    /// player sees now (task 80).
    pub stains: Vec<crate::HudStain>,
    /// Ticks a stain lasts (`world.stain_days × world.day_ticks`).
    pub stain_life_ticks: u64,
    /// Ascensions under way, for the panel at the top of the screen (task 76): one line each,
    /// in the ascending player's colour.
    pub banner: Vec<(String, [u8; 4])>,
    /// The diplomacy tab: one line per player, its standing and any ascension countdown.
    pub diplomacy: Vec<(String, [u8; 4])>,
    /// A panel over the world at the left: the celestial home's residents and chat (task 76).
    pub panel: Option<HudPanel>,
    /// The chat channel's name beside the message line, and the line being typed, if any.
    pub chat: (String, Option<String>),
}

/// A panel of lines drawn over the world at the left, above the chat line (task 76).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HudPanel {
    /// Its title.
    pub title: String,
    /// Its lines, oldest first, each in its colour.
    pub lines: Vec<(String, [u8; 4])>,
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
            event_about: None,
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
            trade_targets: Vec::new(),
            wealth: Default::default(),
            trade_routes: Vec::new(),
            patrols: Vec::new(),
            blasts: Vec::new(),
            wrecks: Vec::new(),
            stains: Vec::new(),
            stain_life_ticks: 0,
            banner: Vec::new(),
            diplomacy: Vec::new(),
            panel: None,
            chat: ("[Allies]".to_owned(), None),
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
#[derive(Clone, Debug, Default)]
pub struct HudOverlay {
    /// The HUD element under the pointer.
    pub hover: Option<HudTarget>,
    /// The selection rectangle being dragged, in the same pixels as the layout.
    pub drag_box: Option<Rect>,
    /// Help about what the pointer rests on in the world, or about the level shown: shown when
    /// no HUD element is hovered and the simulation has no help of its own.
    pub world_help: Option<(String, String)>,
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
    /// The atlas slot holding each model's picture (task 91).
    model_slots: BTreeMap<u16, usize>,
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
            model_slots: BTreeMap::new(),
        })
    }

    /// The models the palette and the queue of `h` show, in order, each once.
    pub fn shown_models(h: &HudSnapshot) -> Vec<u16> {
        let buttons = h
            .palette
            .iter()
            .flat_map(|p| p.buttons.iter().map(|b| b.model));
        let rows = h.queue.iter().flat_map(|q| q.items.iter().map(|i| i.model));
        let mut models: Vec<u16> = Vec::new();
        for model in buttons.chain(rows).flatten() {
            if !models.contains(&model) {
                models.push(model);
            }
        }
        models
    }

    /// The models `h` shows that have no picture yet.
    pub fn missing_models(&self, h: &HudSnapshot) -> Vec<u16> {
        Self::shown_models(h)
            .into_iter()
            .filter(|m| !self.model_slots.contains_key(m))
            .collect()
    }

    /// Where the picture of `model` is in the atlas, once it has been stored.
    pub fn model_icon(&self, model: u16) -> Option<AtlasRect> {
        self.artwork.model_slot(*self.model_slots.get(&model)?)
    }

    /// Stores the picture of `model` (straight RGBA, [`AtlasImage::model_icon_edge`] a side) in
    /// the atlas and its greyscale copy: in a free slot, or else in the slot of a model `h` does
    /// not show. Returns the rectangle written, for the textures.
    pub fn store_model_icon(
        &mut self,
        model: u16,
        pixels: &[u8],
        h: &HudSnapshot,
    ) -> Option<AtlasRect> {
        let slot = match self.model_slots.get(&model) {
            Some(slot) => *slot,
            None => {
                let used: std::collections::BTreeSet<usize> =
                    self.model_slots.values().copied().collect();
                let free = (0..hud_atlas::MODEL_ICON_SLOTS).find(|s| !used.contains(s));
                let slot = match free {
                    Some(slot) => slot,
                    None => {
                        let shown = Self::shown_models(h);
                        let (old, slot) = self
                            .model_slots
                            .iter()
                            .find(|(m, _)| !shown.contains(m))
                            .map(|(m, s)| (*m, *s))?;
                        self.model_slots.remove(&old);
                        slot
                    }
                };
                self.model_slots.insert(model, slot);
                slot
            }
        };
        let rect = self.artwork.model_slot(slot)?;
        if pixels.len() != rect.w as usize * rect.h as usize * 4 {
            self.model_slots.remove(&model);
            return None;
        }
        self.artwork.write(rect, pixels);
        hud_atlas::write_rect(
            &mut self.greyscale,
            self.artwork.width,
            rect,
            &hud_atlas::greyscale(pixels),
        );
        Some(rect)
    }

    /// Whether the picture of `model` is in the atlas.
    fn has_model_icon(&self, model: Option<u16>) -> bool {
        model.is_some_and(|m| self.model_slots.contains_key(&m))
    }

    /// The picture of a button or queue row: the model's, once rendered, else the artwork.
    fn picture(&mut self, icon: &str, model: Option<u16>, dest: Rect) {
        match model.and_then(|m| self.model_icon(m)) {
            Some(r) => self.quads.push(HudQuad {
                texture: HudTexture::Artwork,
                source: Rect::new(r.x as f32, r.y as f32, r.w as f32, r.h as f32),
                dest,
                colour: WHITE,
            }),
            None => self.art(icon, dest),
        }
    }

    /// A track level's pips in the bottom-left corner of `tile`: rows of five, bottom up.
    fn level_pips(&mut self, level: u8, tile: Rect, locked: bool) {
        let s = self.scale;
        let (side, gap) = (5.5 * s, 1.8 * s);
        let colour = if locked { MUTED } else { GOLD };
        for i in 0..usize::from(level.min(15)) {
            let (row, column) = ((i / 5) as f32, (i % 5) as f32);
            let x = tile.x + 5.0 * s + column * (side + gap);
            let y = tile.bottom() - 6.0 * s - side - row * (side + gap);
            self.solid(
                Rect::new(x - s, y - s, side + 2.0 * s, side + 2.0 * s),
                [20, 13, 5, 235],
            );
            self.solid(Rect::new(x, y, side, side), colour);
        }
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

    /// Scale factor (at most 1) at which `text` fits `width` screen pixels whole.
    fn fit_factor(&self, text: &str, width: f32, size: TextSize) -> f32 {
        let natural = self.glyphs.measure(text, size) as f32 * self.scale / self.glyphs.scale;
        if natural <= width {
            1.0
        } else {
            // A pixel of slack keeps the fitted text clear of the truncation check.
            ((width - 1.0) / natural).max(0.1)
        }
    }

    /// Draws a right-aligned number whole, shrinking it instead of cutting digits.
    fn number_fitted(&mut self, text: &str, area: Rect, colour: [u8; 4]) {
        let factor = self.fit_factor(text, area.w, TextSize::Small);
        self.text_scaled(text, area, (TextSize::Small, factor), Align::Right, colour);
    }

    /// Draws a stock and its rate as one right-aligned line; both shrink together to fit whole.
    fn amount_and_rate(&mut self, amount: &str, rate: &str, area: Rect, rate_colour: [u8; 4]) {
        let size = TextSize::Small;
        let line = format!("{amount} {rate}");
        let factor = self.fit_factor(&line, area.w, size);
        let ratio = self.scale / self.glyphs.scale * factor;
        let rate_w = self.glyphs.measure(rate, size) as f32 * ratio + 0.5;
        let rate_area = Rect::new(area.right() - rate_w, area.y, rate_w, area.h);
        let amount_area = Rect::new(area.x, area.y, area.w - rate_w, area.h);
        self.text_scaled(rate, rate_area, (size, factor), Align::Right, rate_colour);
        let gap = self.glyphs.measure(" ", size) as f32 * ratio;
        let amount_area = Rect::new(amount_area.x, area.y, amount_area.w - gap, area.h);
        self.text_scaled(amount, amount_area, (size, factor), Align::Right, WHITE);
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
                    (
                        item.name.clone(),
                        match &item.blocked {
                            Some(reason) => format!("{reason}. Right-click to cancel"),
                            None => "Right-click to cancel".to_owned(),
                        },
                    )
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
                region: K::InsideWorker,
                index,
            }) => match &h.selection {
                HudSelection::Single(card) => {
                    card.inside.get(index).map_or_else(Default::default, |t| {
                        (
                            format!("{} (inside)", t.name),
                            "Click to select; double-click for every one of its kind".to_owned(),
                        )
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
        let (first, second) = if !first.is_empty() {
            (first, second)
        } else if !h.help.is_empty() {
            (h.help.clone(), String::new())
        } else if let Some(help) = &overlay.world_help {
            help.clone()
        } else {
            (
                "Rollover an item for help and details...".to_owned(),
                String::new(),
            )
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
            let start = self.quads.len();
            let locked = matches!(b.state, ButtonState::Locked { .. });
            if locked {
                self.art("chrome/tile", r);
            }
            // A track level leaves the bottom-left corner to its pips.
            let dest = if matches!(b.mark, IconMark::Level(_)) {
                let icon = 29.0 * s;
                Rect::new(r.right() - icon - 5.0 * s, r.y + 4.0 * s, icon, icon)
            } else {
                // A model's picture fills more of the tile than a vector glyph.
                let icon = if self.has_model_icon(b.model) {
                    40.0
                } else {
                    34.0
                } * s;
                Rect::new(
                    r.x + (r.w - icon) / 2.0,
                    r.y + (r.h - icon) / 2.0 + 2.0 * s,
                    icon,
                    icon,
                )
            };
            self.picture(&b.icon, b.model, dest);
            if b.mark == IconMark::Upgrade {
                self.art(
                    "chrome/mark_upgrade",
                    Rect::new(
                        r.right() - 21.0 * s,
                        r.bottom() - 22.0 * s,
                        18.0 * s,
                        18.0 * s,
                    ),
                );
            }
            if locked {
                for quad in &mut self.quads[start..] {
                    quad.texture = HudTexture::Greyscale;
                    quad.colour = [195, 195, 195, 255];
                }
            }
            if let IconMark::Level(level) = b.mark {
                self.level_pips(level, r, locked);
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
            self.picture(
                &item.icon,
                item.model,
                Rect::new(slot.x + (slot.w - icon) / 2.0, slot.y + 3.0 * s, icon, icon),
            );
            if item.mark == IconMark::Upgrade {
                self.art(
                    "chrome/mark_upgrade",
                    Rect::new(
                        slot.right() - 13.0 * s,
                        slot.bottom() - 14.0 * s,
                        12.0 * s,
                        12.0 * s,
                    ),
                );
            }
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
            if let Some(reason) = &item.blocked {
                self.caption(
                    l,
                    Group::BottomLeft,
                    reason,
                    Rect::new(434.0, y + 34.0, 170.0, 14.0),
                    AMBER,
                );
            }
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
                for (i, line) in card.wealth.iter().enumerate() {
                    self.caption(
                        l,
                        R,
                        line,
                        Rect::new(998.0, 864.0 + i as f32 * 19.0, 322.0, 17.0),
                        GOLD,
                    );
                }
                if !card.inside.is_empty() {
                    self.caption(
                        l,
                        R,
                        "WORKERS INSIDE",
                        Rect::new(1122.0, 848.0, 190.0, 16.0),
                        GOLD,
                    );
                }
                for (i, worker) in card.inside.iter().take(12).enumerate() {
                    let r = l.inside_tile(i);
                    self.art("chrome/tile", r);
                    let icon = 20.0 * s;
                    self.art(
                        &worker.icon,
                        Rect::new(r.x + (r.w - icon) / 2.0, r.y + 3.0 * s, icon, icon),
                    );
                    let fraction = worker.hp_permille as f32 / 1000.0;
                    self.bar(
                        Rect::new(r.x + 4.0 * s, r.bottom() - 8.0 * s, r.w - 8.0 * s, 4.0 * s),
                        fraction,
                        Self::health_colour(fraction),
                    );
                    if overlay.hover
                        == Some(HudTarget {
                            region: K::InsideWorker,
                            index: i,
                        })
                    {
                        self.frame(r, 1.5 * s, GOLD);
                    }
                }
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

    /// A dark panel of lines over the world at the left, its bottom just above the chat line.
    fn side_panel(&mut self, l: &HudLayout, panel: &HudPanel) {
        use Group::BottomLeft as L;
        let rows = panel.lines.len().min(14) as f32 + 1.0;
        let height = rows * 21.0 + 16.0;
        let top = 572.0 - height;
        self.solid(
            l.map(L, Rect::new(16.0, top, 380.0, height)),
            [8, 11, 18, 205],
        );
        self.frame(
            l.map(L, Rect::new(16.0, top, 380.0, height)),
            l.scale.max(1.0),
            [232, 201, 138, 160],
        );
        self.caption(
            l,
            L,
            &panel.title,
            Rect::new(28.0, top + 8.0, 356.0, 17.0),
            GOLD,
        );
        let skip = panel.lines.len().saturating_sub(14);
        for (i, (line, colour)) in panel.lines.iter().skip(skip).enumerate() {
            self.caption(
                l,
                L,
                line,
                Rect::new(28.0, top + 29.0 + i as f32 * 21.0, 356.0, 17.0),
                *colour,
            );
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
            self.number_fitted(
                &format!("{} / {}", number(value), number(cap)),
                l.map(T, Rect::new(94.0, y + 17.0, 119.0, 17.0)),
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
            let rate_colour = if res.capped {
                if flashing { AMBER } else { GOLD }
            } else if res.rate < 0 {
                AMBER
            } else {
                GREEN
            };
            let (amount, rate) = (number(res.amount), signed_number(res.rate));
            if wide {
                self.amount_and_rate(
                    &amount,
                    &rate,
                    l.map(T, Rect::new(x + 32.0, y + 23.0, w - 36.0, 15.0)),
                    rate_colour,
                );
            } else {
                // Narrow cells stack the stock over the rate so both stay readable.
                self.number_fitted(
                    &amount,
                    l.map(T, Rect::new(x + 32.0, y + 15.0, w - 36.0, 14.0)),
                    WHITE,
                );
                self.number_fitted(
                    &rate,
                    l.map(T, Rect::new(x + 32.0, y + 28.0, w - 36.0, 14.0)),
                    rate_colour,
                );
            }
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
        let (channel, draft) = &h.chat;
        self.caption(l, L, channel, Rect::new(50.0, 584.0, 92.0, 17.0), CYAN);
        match draft {
            Some(text) => self.caption(
                l,
                L,
                &format!("{text}_"),
                Rect::new(146.0, 584.0, 217.0, 17.0),
                WHITE,
            ),
            None => self.caption(
                l,
                L,
                "Message...",
                Rect::new(146.0, 584.0, 217.0, 17.0),
                MUTED,
            ),
        }
        if let Some(panel) = &h.panel {
            self.side_panel(l, panel);
        }
        // Ascensions under way, under the title, for everyone to see.
        let shown = h.banner.len().min(3);
        if shown > 0 {
            self.solid(
                l.map(
                    T,
                    Rect::new(586.0, 142.0, 500.0, 10.0 + 24.0 * shown as f32),
                ),
                [8, 11, 18, 200],
            );
        }
        for (i, (line, colour)) in h.banner.iter().take(shown).enumerate() {
            self.text(
                line,
                l.map(T, Rect::new(596.0, 147.0 + i as f32 * 24.0, 480.0, 19.0)),
                TextSize::Small,
                Align::Centre,
                *colour,
            );
        }
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
        if h.data_tab == 2 {
            for (i, line) in [
                "Wealth".to_owned(),
                format!("Relics kept: {}", h.wealth.relics),
                format!("Routes: {} / {}", h.wealth.routes, h.wealth.limit),
                format!("+{} per trip", h.wealth.per_trip),
            ]
            .iter()
            .enumerate()
            {
                self.caption(
                    l,
                    R,
                    line,
                    Rect::new(1492.0, 709.0 + i as f32 * 26.0, 148.0, 19.0),
                    GOLD,
                );
            }
        }
        if h.data_tab == 3 {
            for (i, (line, colour)) in h.diplomacy.iter().take(9).enumerate() {
                self.caption(
                    l,
                    R,
                    line,
                    Rect::new(1492.0, 700.0 + i as f32 * 21.0, 150.0, 17.0),
                    *colour,
                );
            }
        }
        if h.data_tab == 4 {
            self.research_overview(l, &h.overview);
        }
        // The version, small, in the bottom right corner nothing else uses.
        self.text(
            version_label(),
            l.map(R, Rect::new(1470.0, 905.0, 150.0, 14.0)),
            TextSize::Small,
            Align::Right,
            MUTED,
        );
        if let Some(drag) = overlay.drag_box {
            self.solid(drag, [63, 208, 255, 36]);
            self.frame(drag, (1.5 * l.scale).max(1.0), CYAN);
        }
        &self.quads
    }
}

/// The text of the HUD's version label: the workspace version.
pub fn version_label() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// A rate with its sign and thousands separators: `+1,234`, `-56`, `+0`.
fn signed_number(value: i64) -> String {
    if value < 0 {
        number(value)
    } else {
        format!("+{}", number(value))
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

#[cfg(test)]
mod version_tests {
    #[test]
    fn the_version_label_is_the_package_version() {
        assert_eq!(super::version_label(), env!("CARGO_PKG_VERSION"));
        assert!(!super::version_label().is_empty());
    }
}
