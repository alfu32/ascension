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
    ButtonState, HudButton, HudCard, HudCityRadius, HudLineSite, HudMarker, HudPalette,
    HudPlacement, HudQueue, HudQueueItem, HudSelection, HudTile, IconMark, MarkerKind, PaletteTab,
    ResearchCell, ResearchOverview, ResearchTrack, SLOT_KEYS,
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
    /// The entity the newest alert is about while its marker lasts (task 118): Backspace flies
    /// the camera there.
    pub alert_about: Option<u32>,
    /// Alerts on the shown surface, marked on the minimap (task 118).
    pub alerts: Vec<crate::HudAlert>,
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
    /// Capitals known at city level on the current surface, by owner (task 131).
    pub capitals: Vec<crate::HudCapital>,
    /// The names of the cities shown on the current surface (task 144).
    pub city_names: Vec<crate::HudCityName>,
    /// The rename field while it has the keyboard (task 144).
    pub rename: Option<crate::HudRename>,
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
    /// The fortification line or minefield being dragged, site by site (task 147b).
    pub line: Vec<HudLineSite>,
    /// A brush is in hand: the left button paints along its drag instead of selecting (task 150).
    pub painting: bool,
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
    /// The chat: the channel tag, the line being typed, the latest lines and the history.
    pub chat: HudChat,
    /// The new-age dialog over everything, while it is open (task 133).
    pub age_dialog: Option<HudAgeDialog>,
    /// The entity the pointer rests on in the world, its side and what a right click on it
    /// does (task 151).
    pub pointer: Option<crate::HudPointer>,
    /// Targets of orders just given by a right click, flashing (task 151).
    pub flashes: Vec<crate::HudFlash>,
    /// Each player's side as the person sees them, indexed by player id: selection rings take
    /// their colour from it, never from the team colour (task 151).
    pub sides: Vec<crate::Side>,
}

/// The dialog that tells a new age's story (task 133): drawn over the world and the HUD until
/// a click or a key closes it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HudAgeDialog {
    /// The line above the name: the age's number and its era.
    pub heading: String,
    /// The age's name.
    pub name: String,
    /// What the age is remembered by.
    pub subtitle: String,
    /// The age's story, one paragraph.
    pub story: String,
    /// How to close it.
    pub hint: String,
}

/// A line of the chat as drawn: who said it, in which channel, and how visible it still is.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HudChatLine {
    /// The sender's name.
    pub name: String,
    /// The sender's player colour.
    pub name_colour: [u8; 4],
    /// The channel tag, such as `[Allies]`.
    pub tag: String,
    /// The channel's colour.
    pub tag_colour: [u8; 4],
    /// The words.
    pub text: String,
    /// Opacity, 255 for a fresh line; a line fades out as it ages.
    pub alpha: u8,
}

/// The open chat history: a window onto the lines, newest at the bottom (task 94).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HudChatHistory {
    /// The visible lines, oldest first.
    pub lines: Vec<HudChatLine>,
    /// Older lines above the window.
    pub above: usize,
    /// Newer lines below the window.
    pub below: usize,
}

/// The chat as the HUD shows it (task 94).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HudChat {
    /// The channel tag beside the message line, such as `[All]`.
    pub channel: String,
    /// The channel's colour.
    pub channel_colour: [u8; 4],
    /// The line being typed, if the chat has the keyboard.
    pub draft: Option<String>,
    /// The caret's place in the draft, in characters.
    pub caret: usize,
    /// The latest lines, drawn over the world at the left, oldest first.
    pub recent: Vec<HudChatLine>,
    /// The history, when it is open.
    pub history: Option<HudChatHistory>,
}

impl Default for HudChat {
    fn default() -> Self {
        Self {
            channel: "[All]".to_owned(),
            channel_colour: CYAN,
            draft: None,
            caret: 0,
            recent: Vec::new(),
            history: None,
        }
    }
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
            alert_about: None,
            alerts: Vec::new(),
            help: String::new(),
            selection: HudSelection::None,
            palette: None,
            queue: None,
            selected: Vec::new(),
            markers: Vec::new(),
            placement: None,
            line: Vec::new(),
            painting: false,
            data_tab: 0,
            overview: ResearchOverview::default(),
            city_radii: Vec::new(),
            capitals: Vec::new(),
            city_names: Vec::new(),
            rename: None,
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
            chat: HudChat::default(),
            age_dialog: None,
            pointer: None,
            flashes: Vec::new(),
            sides: Vec::new(),
        }
    }
}

/// Characters of the draft shown around the caret.
const CHAT_WINDOW: usize = 34;
/// The selection panel's heading, in design pixels of the bottom-right group: a city's name
/// stands there, and its rename field (task 144).
pub(crate) const SELECTION_HEADING: Rect = Rect::new(998.0, 681.0, 320.0, 21.0);
/// Rows of the open chat history.
pub const CHAT_HISTORY_ROWS: usize = 14;

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
    /// The city under the pointer on the minimap (task 144): its name and whose it is.
    pub minimap_city: Option<(String, String)>,
    /// The entity the render thread's own pick finds under the pointer this frame (task 151):
    /// the snapshot's [`HudSnapshot::pointer`] is shown only while it names the same entity.
    pub pointed: Option<u32>,
    /// Where the pointer is, in the same pixels as the layout: the order badge stands beside it.
    pub pointer_at: Option<(f32, f32)>,
}

impl HudOverlay {
    /// The snapshot's pointer entry, while the render thread's pick agrees with it.
    pub fn pointer<'a>(&self, h: &'a HudSnapshot) -> Option<&'a crate::HudPointer> {
        h.pointer
            .as_ref()
            .filter(|p| self.hover.is_none() && self.pointed == Some(p.entity))
    }
}

/// A portrait of the single selection (task 145): a model in its owner's colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Portrait {
    /// The composed model.
    pub model: u16,
    /// The owner whose colour it wears.
    pub owner: u8,
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
    /// The portraits in the atlas (task 145), by model and owner colour, and their slots,
    /// oldest stored first.
    portrait_slots: Vec<(Portrait, usize)>,
    /// The fully opaque parts of each frame piece, in its own pixels (task 108).
    cover: Vec<Vec<AtlasRect>>,
}

/// Narrowest run of opaque pixels kept in a frame piece's cover.
const COVER_MIN_RUN: u32 = 8;

/// The frame pieces every HUD picture starts with, by atlas name, and where `l` puts them: the
/// bar fill across the width, then the panels.
fn chrome_pieces(l: &HudLayout) -> [(&'static str, Rect); 6] {
    use Group::{BottomCentre as C, BottomLeft as L, BottomRight as R, Top as T};
    [
        (
            "chrome/bar_fill",
            Rect::new(0.0, 0.0, l.width, 135.0 * l.scale),
        ),
        (
            "chrome/topbar",
            l.map(T, Rect::new(0.0, 0.0, 1672.0, 135.0)),
        ),
        (
            "chrome/bottom_left",
            l.map(L, Rect::new(0.0, 570.0, 612.0, 371.0)),
        ),
        ("chrome/minimap", l.map(C, Region::MinimapFrame.design())),
        (
            "chrome/bottom_right",
            l.map(R, Rect::new(975.0, 662.0, 697.0, 279.0)),
        ),
        ("chrome/emblem", l.map(T, Region::Emblem.design())),
    ]
}

/// The parts of `piece` in `image` whose pixels are all fully opaque, as rectangles relative to
/// the piece: runs of opaque pixels in each row, joined downwards while they keep their span.
fn opaque_parts(image: &AtlasImage, piece: AtlasRect) -> Vec<AtlasRect> {
    let alpha = |x: u32, y: u32| {
        image.rgba[((piece.y + y) as usize * image.width as usize + (piece.x + x) as usize) * 4 + 3]
    };
    let mut done = Vec::new();
    let mut open: Vec<AtlasRect> = Vec::new();
    for y in 0..piece.h {
        let mut runs = Vec::new();
        let mut x = 0;
        while x < piece.w {
            if alpha(x, y) == u8::MAX {
                let start = x;
                while x < piece.w && alpha(x, y) == u8::MAX {
                    x += 1;
                }
                if x - start >= COVER_MIN_RUN {
                    runs.push((start, x - start));
                }
            } else {
                x += 1;
            }
        }
        let mut next = Vec::with_capacity(runs.len());
        for mut rect in open.drain(..) {
            if runs.contains(&(rect.x, rect.w)) {
                rect.h += 1;
                next.push(rect);
            } else {
                done.push(rect);
            }
        }
        for (x, w) in runs {
            if !next.iter().any(|r| r.x == x && r.w == w) {
                next.push(AtlasRect { x, y, w, h: 1 });
            }
        }
        open = next;
    }
    done.extend(open);
    done.retain(|rect| rect.h >= 2);
    done
}

impl HudPainter {
    /// Bakes the embedded artwork and text at one scale, without a graphics context.
    pub fn bake(scale: f32) -> Result<Self, AtlasError> {
        let artwork = hud_atlas::bake(scale)?;
        let greyscale = hud_atlas::greyscale(&artwork.rgba);
        let cover = chrome_pieces(&HudLayout::new(1, 1, None))
            .iter()
            .map(|(name, _)| {
                artwork
                    .rect(name)
                    .map_or_else(Vec::new, |piece| opaque_parts(&artwork, piece))
            })
            .collect();
        Ok(Self {
            artwork,
            greyscale,
            glyphs: GlyphAtlas::bake(scale),
            quads: Vec::new(),
            scale,
            model_slots: BTreeMap::new(),
            portrait_slots: Vec::new(),
            cover,
        })
    }

    /// Screen rectangles the HUD of layout `l` covers with fully opaque frame pixels: nothing
    /// drawn under them before the HUD shows. Each keeps a margin of a source pixel and a half
    /// from its edges, where filtering blends in the neighbouring pixels.
    pub fn opaque_cover(&self, l: &HudLayout) -> Vec<Rect> {
        let mut out = Vec::new();
        for ((name, dest), parts) in chrome_pieces(l).iter().zip(&self.cover) {
            let Some(piece) = self.artwork.rect(name) else {
                continue;
            };
            let sx = dest.w / piece.w.max(1) as f32;
            let sy = dest.h / piece.h.max(1) as f32;
            let (mx, my) = (sx.max(1.0) * 1.5, sy.max(1.0) * 1.5);
            for part in parts {
                let rect = Rect::new(
                    dest.x + part.x as f32 * sx + mx,
                    dest.y + part.y as f32 * sy + my,
                    part.w as f32 * sx - 2.0 * mx,
                    part.h as f32 * sy - 2.0 * my,
                );
                if rect.w >= 4.0 && rect.h >= 4.0 {
                    out.push(rect);
                }
            }
        }
        out
    }

    /// The models the palette, the queue and the selection's tiles of `h` show, in order, each
    /// once.
    pub fn shown_models(h: &HudSnapshot) -> Vec<u16> {
        let buttons = h
            .palette
            .iter()
            .flat_map(|p| p.buttons.iter().map(|b| b.model));
        let rows = h.queue.iter().flat_map(|q| q.items.iter().map(|i| i.model));
        let tiles: Vec<Option<u16>> = match &h.selection {
            HudSelection::None => Vec::new(),
            HudSelection::Single(card) => card.inside.iter().take(12).map(|t| t.model).collect(),
            HudSelection::Multiple { tiles, .. } => {
                tiles.iter().take(12).map(|t| t.model).collect()
            }
        };
        let mut models: Vec<u16> = Vec::new();
        for model in tiles.into_iter().chain(buttons).chain(rows).flatten() {
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

    /// The portrait the single selection of `h` shows (task 145), if its type has a model.
    pub fn shown_portrait(h: &HudSnapshot) -> Option<Portrait> {
        match &h.selection {
            HudSelection::Single(card) => card.model.map(|model| Portrait {
                model,
                owner: card.owner,
            }),
            _ => None,
        }
    }

    /// The portrait `h` shows when it has not been rendered yet.
    pub fn missing_portrait(&self, h: &HudSnapshot) -> Option<Portrait> {
        Self::shown_portrait(h).filter(|p| self.portrait(*p).is_none())
    }

    /// Where `portrait` is in the atlas, once it has been stored.
    pub fn portrait(&self, portrait: Portrait) -> Option<AtlasRect> {
        let (_, slot) = self.portrait_slots.iter().find(|(p, _)| *p == portrait)?;
        self.artwork.portrait_slot(*slot)
    }

    /// Stores `portrait` (straight RGBA, [`AtlasImage::portrait_edge`] a side) in the atlas and
    /// its greyscale copy: in a free slot, or else in the oldest one `h` does not show. Returns
    /// the rectangle written, for the textures.
    pub fn store_portrait(
        &mut self,
        portrait: Portrait,
        pixels: &[u8],
        h: &HudSnapshot,
    ) -> Option<AtlasRect> {
        let slot = match self.portrait_slots.iter().position(|(p, _)| *p == portrait) {
            Some(at) => self.portrait_slots.remove(at).1,
            None => {
                let free = (0..hud_atlas::PORTRAIT_SLOTS)
                    .find(|s| self.portrait_slots.iter().all(|(_, used)| used != s));
                match free {
                    Some(slot) => slot,
                    None => {
                        let shown = Self::shown_portrait(h);
                        let at = self
                            .portrait_slots
                            .iter()
                            .position(|(p, _)| Some(*p) != shown)?;
                        self.portrait_slots.remove(at).1
                    }
                }
            }
        };
        let rect = self.artwork.portrait_slot(slot)?;
        if pixels.len() != rect.w as usize * rect.h as usize * 4 {
            return None;
        }
        self.portrait_slots.push((portrait, slot));
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

    /// A brush's swatch along the bottom of `tile` and its size as one to three rising bars at
    /// the right end (task 114).
    fn brush_mark(&mut self, swatch: Option<[u8; 3]>, size: u8, tile: Rect) {
        let s = self.scale;
        let band = Rect::new(
            tile.x + 6.0 * s,
            tile.bottom() - 15.0 * s,
            tile.w - 12.0 * s,
            11.0 * s,
        );
        if let Some([r, g, b]) = swatch {
            self.solid(
                Rect::new(band.x - s, band.y - s, band.w + 2.0 * s, band.h + 2.0 * s),
                [20, 13, 5, 235],
            );
            self.solid(band, [r, g, b, 255]);
        }
        let (width, gap) = (3.5 * s, 1.5 * s);
        for i in 0..3u8 {
            let height = (4.0 + 3.5 * f32::from(i)) * s;
            let x = band.right() - (3.0 - f32::from(i)) * (width + gap) - s;
            let bar = Rect::new(x, band.bottom() - height, width, height);
            self.solid(
                Rect::new(bar.x - s, bar.y - s, bar.w + 2.0 * s, bar.h + 2.0 * s),
                [20, 13, 5, 235],
            );
            self.solid(
                bar,
                if i < size.clamp(1, 3) {
                    WHITE
                } else {
                    [70, 76, 86, 255]
                },
            );
        }
    }

    /// The palette's groups listed in the production queue panel: icon, name and size of each,
    /// the shown one framed in cyan, a cyan square before the count of the one holding the armed
    /// button.
    fn group_block(&mut self, l: &HudLayout, h: &HudSnapshot, overlay: &HudOverlay) {
        use crate::pick::HudRegionKind as K;
        let s = l.scale;
        let Some(palette) = h.palette.as_ref() else {
            return;
        };
        for (i, group) in palette
            .groups
            .iter()
            .take(crate::hud_layout::PALETTE_GROUPS)
            .enumerate()
        {
            let r = l.palette_group(i);
            self.solid(
                r,
                if group.current {
                    [18, 52, 70, 235]
                } else {
                    [8, 12, 19, 210]
                },
            );
            self.art(
                &group.icon,
                Rect::new(r.x + 6.0 * s, r.y + 2.0 * s, 23.0 * s, 23.0 * s),
            );
            let y = 768.0 + i as f32 * 30.0;
            self.caption(
                l,
                Group::BottomLeft,
                &group.name.to_uppercase(),
                Rect::new(416.0, y + 5.0, 130.0, 17.0),
                if group.current { WHITE } else { MUTED },
            );
            self.text(
                &group.count.to_string(),
                l.map(Group::BottomLeft, Rect::new(548.0, y + 5.0, 40.0, 17.0)),
                TextSize::Small,
                Align::Right,
                if group.current { CYAN } else { MUTED },
            );
            if group.armed {
                self.solid(
                    Rect::new(r.right() - 52.0 * s, r.y + 10.0 * s, 7.0 * s, 7.0 * s),
                    CYAN,
                );
            }
            if group.current {
                self.frame(r, 1.5 * s, CYAN);
            } else if overlay.hover
                == Some(HudTarget {
                    region: K::PaletteGroup,
                    index: i,
                })
            {
                self.frame(r, 1.5 * s, GOLD);
            }
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

    /// Whether `text` fits the width of `area` without being shortened.
    fn fits(&self, text: &str, area: Rect, size: TextSize) -> bool {
        let ratio = self.scale / self.glyphs.scale;
        self.glyphs.measure(text, size) as f32 <= area.w / ratio
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

    /// The order a right click gives, in a small tile beside the pointer, framed in the side's
    /// colour of the entity under it (task 151). Nothing when the pointer is on nothing the
    /// selection can act on.
    fn order_badge(&mut self, l: &HudLayout, h: &HudSnapshot, overlay: &HudOverlay) {
        let (Some(pointer), Some((x, y))) = (overlay.pointer(h), overlay.pointer_at) else {
            return;
        };
        let Some(badge) = pointer.badge else {
            return;
        };
        let s = l.scale;
        let size = 30.0 * s;
        // Above and right of the pointer: clear of the arrow, which hangs below and right of
        // its tip, and of the tooltip that space draws under it.
        let tile = Rect::new(x + 12.0 * s, y - 10.0 * s - size, size, size);
        self.solid(tile, [12, 17, 24, 225]);
        self.frame(tile, (2.0 * s).max(1.0), pointer.side.colour());
        let inset = 4.0 * s;
        self.art(
            badge.icon(),
            Rect::new(
                tile.x + inset,
                tile.y + inset,
                size - 2.0 * inset,
                size - 2.0 * inset,
            ),
        );
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
                h.palette
                    .as_ref()
                    .map(|p| p.tab_notes[index.min(3)].clone())
                    .unwrap_or_default(),
            ),
            Some(HudTarget {
                region: K::PaletteGroup,
                index,
            }) => h
                .palette
                .as_ref()
                .and_then(|p| p.groups.get(index))
                .map_or_else(Default::default, |g| (g.name.clone(), g.note.clone())),
            Some(HudTarget {
                region: K::CityName,
                ..
            }) => match &h.selection {
                HudSelection::Single(card) => (
                    card.title.clone().unwrap_or_default(),
                    "Click to rename the city".to_owned(),
                ),
                _ => Default::default(),
            },
            // The city under the pointer on the minimap (task 144).
            Some(HudTarget {
                region: K::Minimap, ..
            }) => overlay.minimap_city.clone().unwrap_or_default(),
            _ => Default::default(),
        };
        let (first, second) = if h.rename.is_some() {
            (
                "Rename the city".to_owned(),
                "Enter keeps the name, Escape cancels".to_owned(),
            )
        } else if !first.is_empty() {
            (first, second)
        } else if let Some(pointer) = overlay.pointer(h) {
            // The entity under the pointer (task 151): whose it is, what it is, its health;
            // then what a right click on it does, else the simulation's own help.
            let second = if pointer.hint.is_empty() {
                h.help.split('\n').next().unwrap_or("").to_owned()
            } else {
                pointer.hint.clone()
            };
            (pointer.text.clone(), second)
        } else if !h.help.is_empty() {
            // A hint of two lines is split at its line break.
            h.help.split_once('\n').map_or_else(
                || (h.help.clone(), String::new()),
                |(a, b)| (a.to_owned(), b.to_owned()),
            )
        } else if let Some(help) = &overlay.world_help {
            help.clone()
        } else {
            (
                "Rollover an item for help and details...".to_owned(),
                String::new(),
            )
        };
        // A palette with groups lists them beside the grid instead of naming its page.
        let paged = h
            .palette
            .as_ref()
            .filter(|p| p.pages > 1 && p.groups.is_empty());
        let right = if paged.is_some() { 110.0 } else { 0.0 };
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
        if let Some(palette) = paged {
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
            } else if matches!(b.mark, IconMark::Brush { .. }) {
                // A brush leaves the bottom of the tile to its swatch and size.
                let icon = 29.0 * s;
                Rect::new(r.x + (r.w - icon) / 2.0, r.y + 5.0 * s, icon, icon)
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
            if let IconMark::Brush { swatch, size } = b.mark {
                self.brush_mark(swatch, size, r);
            }
            // A market button names its trade in its top right corner and the price of the
            // moment along its bottom.
            if let IconMark::Buy(price) | IconMark::Sell(price) = b.mark {
                let word = if matches!(b.mark, IconMark::Buy(_)) {
                    "Buy"
                } else {
                    "Sell"
                };
                let colour = if b.state.enabled() { GOLD } else { MUTED };
                self.text(
                    word,
                    Rect::new(r.right() - 34.0 * s, r.y + 3.0 * s, 28.0 * s, 12.0 * s),
                    TextSize::Small,
                    Align::Right,
                    colour,
                );
                let band = Rect::new(
                    r.x + 3.0 * s,
                    r.bottom() - 16.0 * s,
                    r.w - 6.0 * s,
                    13.0 * s,
                );
                self.solid(band, [6, 9, 14, 170]);
                self.text(
                    &price.to_string(),
                    band,
                    TextSize::Small,
                    Align::Centre,
                    colour,
                );
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
                let heading = l.map(R, SELECTION_HEADING);
                match (&h.rename, &card.title) {
                    // The rename field (task 144): the draft with its caret on a dark field.
                    (Some(field), _) if card.renamable => {
                        self.solid(heading, [8, 12, 20, 230]);
                        let chars: Vec<char> = field.draft.chars().collect();
                        let caret = field.caret.min(chars.len());
                        let mut shown: String = chars[..caret].iter().collect();
                        shown.push('|');
                        shown.extend(&chars[caret..]);
                        self.text(&shown, heading, TextSize::Large, Align::Centre, CYAN);
                    }
                    // A city is headed by its name (task 144).
                    (_, Some(title)) => self.text(
                        title,
                        heading,
                        TextSize::Large,
                        Align::Centre,
                        if card.capital { GOLD } else { WHITE },
                    ),
                    _ => self.text("SELECTION", heading, TextSize::Large, Align::Centre, WHITE),
                }
                let tile = l.map(R, Rect::new(1008.0, 708.0, 104.0, 94.0));
                self.art("chrome/tile", tile);
                // Its own model's portrait (task 145); until that is rendered, the smaller
                // picture of the model, else the pictogram.
                let centred = |edge: f32| {
                    Rect::new(
                        tile.x + (tile.w - edge) / 2.0,
                        tile.y + (tile.h - edge) / 2.0,
                        edge,
                        edge,
                    )
                };
                match Self::shown_portrait(h).and_then(|p| self.portrait(p)) {
                    Some(r) => self.quads.push(HudQuad {
                        texture: HudTexture::Artwork,
                        source: Rect::new(r.x as f32, r.y as f32, r.w as f32, r.h as f32),
                        dest: centred(hud_atlas::PORTRAIT_DESIGN_SIZE * s),
                        colour: WHITE,
                    }),
                    None => self.picture(&card.icon, card.model, centred(64.0 * s)),
                }
                let name_area = l.map(R, Rect::new(995.0, 808.0, 125.0, 18.0));
                // A capital says so in its title (task 131), or under it when the title has no
                // room for it.
                let titled = format!("{} \u{2014} Capital", card.name);
                let title_fits = card.capital && self.fits(&titled, name_area, TextSize::Small);
                self.text(
                    if title_fits { &titled } else { &card.name },
                    name_area,
                    TextSize::Small,
                    Align::Centre,
                    if card.capital { GOLD } else { WHITE },
                );
                let note = match (card.capital && !title_fits, card.foreign) {
                    (true, true) => "Foreign capital".to_owned(),
                    (true, false) => "Capital".to_owned(),
                    (false, true) => "Foreign".to_owned(),
                    (false, false) => card
                        .progress_pct
                        .map_or_else(String::new, |pct| format!("Building {pct}%")),
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
                    let icon = 24.0 * s;
                    self.picture(
                        &worker.icon,
                        worker.model,
                        Rect::new(r.x + (r.w - icon) / 2.0, r.y + 2.0 * s, icon, icon),
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
                    // The type's own model (task 145), the pictogram until it is rendered.
                    let icon = 44.0 * s;
                    self.picture(
                        &tile.icon,
                        tile.model,
                        Rect::new(r.x + 6.0 * s, r.y + 3.0 * s, icon, icon),
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

    /// The tag, the draft with its caret or the hint, on the message line.
    fn chat_line(&mut self, l: &HudLayout, chat: &HudChat) {
        use Group::BottomLeft as L;
        self.caption(
            l,
            L,
            &chat.channel,
            Rect::new(50.0, 584.0, 92.0, 17.0),
            chat.channel_colour,
        );
        let field = Rect::new(146.0, 584.0, 217.0, 17.0);
        match &chat.draft {
            Some(text) => {
                // Only the part around the caret is shown, so the caret never runs off the field.
                let chars: Vec<char> = text.chars().collect();
                let caret = chat.caret.min(chars.len());
                let start = caret.saturating_sub(CHAT_WINDOW - 6);
                let end = (start + CHAT_WINDOW).min(chars.len());
                let mut shown: String = chars[start..caret].iter().collect();
                shown.push('|');
                shown.extend(&chars[caret..end]);
                self.caption(l, L, &shown, field, WHITE);
            }
            None => self.caption(l, L, "Enter to talk, F7 for history", field, MUTED),
        }
    }

    /// One row of the chat: tag, name, words, in columns.
    fn chat_row(&mut self, l: &HudLayout, line: &HudChatLine, y: f32) {
        use Group::BottomLeft as L;
        let fade = |c: [u8; 4]| {
            [
                c[0],
                c[1],
                c[2],
                (u16::from(c[3]) * u16::from(line.alpha) / 255) as u8,
            ]
        };
        self.caption(
            l,
            L,
            &line.tag,
            Rect::new(28.0, y, 76.0, 17.0),
            fade(line.tag_colour),
        );
        self.caption(
            l,
            L,
            &line.name,
            Rect::new(106.0, y, 124.0, 17.0),
            fade(line.name_colour),
        );
        self.caption(
            l,
            L,
            &line.text,
            Rect::new(234.0, y, 290.0, 17.0),
            fade(WHITE),
        );
    }

    /// The latest lines over the world at the left, their bottom just above the message line.
    fn chat_recent(&mut self, l: &HudLayout, lines: &[HudChatLine]) {
        use Group::BottomLeft as L;
        for (i, line) in lines.iter().enumerate() {
            let y = 572.0 - 21.0 * (lines.len() - i) as f32;
            let back = (u16::from(line.alpha) * 150 / 255) as u8;
            self.solid(
                l.map(L, Rect::new(16.0, y - 2.0, 520.0, 21.0)),
                [8, 11, 18, back],
            );
            self.chat_row(l, line, y);
        }
    }

    /// The history: a framed panel of rows with the number of older and newer lines.
    fn chat_history(&mut self, l: &HudLayout, history: &HudChatHistory) {
        use Group::BottomLeft as L;
        let height = (CHAT_HISTORY_ROWS as f32 + 1.0) * 21.0 + 16.0;
        let top = 572.0 - height;
        let area = l.map(L, Rect::new(16.0, top, 520.0, height));
        self.solid(area, [8, 11, 18, 225]);
        self.frame(area, l.scale.max(1.0), [232, 201, 138, 160]);
        let title = format!(
            "CHAT HISTORY  Up/Down scroll, Esc closes  ({} older, {} newer)",
            history.above, history.below
        );
        self.caption(l, L, &title, Rect::new(28.0, top + 8.0, 496.0, 17.0), GOLD);
        for (i, line) in history.lines.iter().take(CHAT_HISTORY_ROWS).enumerate() {
            self.chat_row(l, line, top + 29.0 + i as f32 * 21.0);
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
        for (name, dest) in chrome_pieces(l) {
            self.art(name, dest);
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
        self.chat_line(l, &h.chat);
        if let Some(panel) = &h.panel {
            self.side_panel(l, panel);
        } else if let Some(history) = &h.chat.history {
            self.chat_history(l, history);
        } else {
            self.chat_recent(l, &h.chat.recent);
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
        // The ascended painter's brush families take the queue's place (task 114).
        let grouped = h.palette.as_ref().is_some_and(|p| !p.groups.is_empty());
        self.art(
            if grouped {
                "icons/life"
            } else {
                "icons/stat_workers"
            },
            l.map(L, Rect::new(384.0, 740.0, 23.0, 23.0)),
        );
        self.caption(
            l,
            L,
            if grouped {
                "BRUSH FAMILIES"
            } else {
                "PRODUCTION QUEUE"
            },
            Rect::new(412.0, 741.0, 179.0, 19.0),
            WHITE,
        );
        if grouped {
            self.group_block(l, h, overlay);
        } else {
            self.queue_block(l, h, overlay);
        }
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
                format!("Trade: +{} per period", h.wealth.per_period),
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
        self.order_badge(l, h, overlay);
        if let Some(dialog) = &h.age_dialog {
            self.age_dialog(l, dialog);
        }
        &self.quads
    }

    /// `text` broken into lines no wider than `width` screen pixels at `size` × `factor`.
    fn wrap(&self, text: &str, width: f32, size: TextSize, factor: f32) -> Vec<String> {
        let ratio = self.scale / self.glyphs.scale * factor;
        let mut lines = Vec::new();
        let mut line = String::new();
        for word in text.split_whitespace() {
            let candidate = if line.is_empty() {
                word.to_owned()
            } else {
                format!("{line} {word}")
            };
            if !line.is_empty() && self.glyphs.measure(&candidate, size) as f32 * ratio > width {
                lines.push(std::mem::replace(&mut line, word.to_owned()));
            } else {
                line = candidate;
            }
        }
        if !line.is_empty() {
            lines.push(line);
        }
        lines
    }

    /// The new-age dialog (task 133): the world dimmed, a framed panel in the middle with the
    /// age's number and era, its name, subtitle and story, and how to close it.
    fn age_dialog(&mut self, l: &HudLayout, dialog: &HudAgeDialog) {
        let s = l.scale.max(0.1);
        let width = (820.0 * s).min(l.width - 32.0);
        let margin = 56.0 * s;
        const STORY_FACTOR: f32 = 1.12;
        let story = self.wrap(
            &dialog.story,
            width - 2.0 * margin,
            TextSize::Large,
            STORY_FACTOR,
        );
        let line = 27.0 * s;
        let height =
            (40.0 + 26.0 + 52.0 + 30.0 + 34.0) * s + story.len() as f32 * line + (34.0 + 30.0) * s;
        let x = ((l.width - width) / 2.0).max(0.0);
        let y = ((l.height - height) / 2.0 - 40.0 * s).max(8.0);
        self.solid(Rect::new(0.0, 0.0, l.width, l.height), [2, 4, 8, 120]);
        let panel = Rect::new(x, y, width, height);
        self.solid(panel, [12, 17, 24, 242]);
        self.frame(panel, (3.0 * s).max(1.0), [184, 138, 69, 255]);
        let inset = 7.0 * s;
        self.frame(
            Rect::new(
                x + inset,
                y + inset,
                width - 2.0 * inset,
                height - 2.0 * inset,
            ),
            (1.0 * s).max(1.0),
            [232, 201, 138, 150],
        );
        let mut at = y + 40.0 * s;
        let row = |at: f32, h: f32| Rect::new(x + margin, at, width - 2.0 * margin, h * s);
        self.text(
            &dialog.heading,
            row(at, 26.0),
            TextSize::Small,
            Align::Centre,
            CYAN,
        );
        at += 26.0 * s;
        self.text_scaled(
            &dialog.name,
            row(at, 52.0),
            (TextSize::Large, 2.0),
            Align::Centre,
            GOLD,
        );
        at += 52.0 * s;
        self.text_scaled(
            &dialog.subtitle,
            row(at, 30.0),
            (TextSize::Large, 1.1),
            Align::Centre,
            MUTED,
        );
        at += 30.0 * s;
        // A thin cyan rule, the HUD's accent, between the title and the story.
        let rule = 180.0 * s;
        self.solid(
            Rect::new(
                x + (width - rule) / 2.0,
                at + 14.0 * s,
                rule,
                (1.5 * s).max(1.0),
            ),
            [63, 208, 255, 200],
        );
        at += 34.0 * s;
        for text in &story {
            self.text_scaled(
                text,
                Rect::new(x + margin, at, width - 2.0 * margin, line),
                (TextSize::Large, STORY_FACTOR),
                Align::Centre,
                WHITE,
            );
            at += line;
        }
        at += 34.0 * s;
        self.text(
            &dialog.hint,
            row(at - 10.0 * s, 22.0),
            TextSize::Small,
            Align::Centre,
            MUTED,
        );
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

#[cfg(test)]
mod cover_tests {
    use super::*;

    /// The artwork's alpha under screen point (`x`, `y`) of `piece` drawn at `dest`.
    fn alpha_under(p: &HudPainter, piece: AtlasRect, dest: Rect, x: f32, y: f32) -> u8 {
        let sx = ((x - dest.x) / dest.w * piece.w as f32) as u32;
        let sy = ((y - dest.y) / dest.h * piece.h as f32) as u32;
        let (sx, sy) = (sx.min(piece.w - 1), sy.min(piece.h - 1));
        p.artwork.rgba
            [((piece.y + sy) as usize * p.artwork.width as usize + (piece.x + sx) as usize) * 4 + 3]
    }

    #[test]
    fn the_cover_lies_on_opaque_frame_pixels_and_hides_much_of_the_screen() {
        for (width, height) in [(1280, 800), (1882, 1043)] {
            let l = HudLayout::new(width, height, None);
            let p = HudPainter::bake(l.scale).expect("artwork bakes");
            let cover = p.opaque_cover(&l);
            assert!(!cover.is_empty());
            let mut area = 0.0;
            for r in &cover {
                area += r.w * r.h;
                // Each corner (a pixel and a half inside) and the centre lie on a frame piece
                // whose pixel there and its neighbours are fully opaque.
                for (x, y) in [
                    (r.x, r.y),
                    (r.right(), r.y),
                    (r.x, r.bottom()),
                    (r.right(), r.bottom()),
                    (r.x + r.w / 2.0, r.y + r.h / 2.0),
                ] {
                    let opaque = chrome_pieces(&l).iter().any(|(name, dest)| {
                        let piece = p.artwork.rect(name).expect("frame piece");
                        dest.contains(x, y)
                            && [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)]
                                .iter()
                                .all(|(dx, dy)| {
                                    alpha_under(&p, piece, *dest, x + dx, y + dy) == u8::MAX
                                })
                    });
                    assert!(opaque, "{width}x{height}: {r:?} at ({x}, {y})");
                }
            }
            // The top bar and the bottom panels: about a third of the screen.
            let share = area / (width * height) as f32;
            assert!(share > 0.2, "{width}x{height}: cover {share}");
        }
    }
}

#[cfg(test)]
mod capital_card_tests {
    use super::*;

    fn card(name: &str, capital: bool) -> HudCard {
        HudCard {
            icon: "icons/pic_house".to_owned(),
            model: None,
            owner: 1,
            name: name.to_owned(),
            foreign: false,
            capital,
            title: None,
            renamable: false,
            hp: (100, 100),
            attack: None,
            armour: None,
            range: None,
            garrison: None,
            workers: None,
            wealth: Vec::new(),
            progress_pct: None,
            inside: Vec::new(),
        }
    }

    /// Glyph quads drawn in `colour` for a selected `card`.
    fn glyphs_in(p: &mut HudPainter, l: &HudLayout, card: HudCard, colour: [u8; 4]) -> usize {
        let h = HudSnapshot {
            selection: HudSelection::Single(Box::new(card)),
            ..HudSnapshot::default()
        };
        p.compose(l, &h)
            .iter()
            .filter(|q| matches!(q.texture, HudTexture::Glyphs) && q.colour == colour)
            .count()
    }

    /// Task 131: a selected capital's title reads "<name> — Capital" in gold; a name too long
    /// for that keeps its title and says "Capital" under it.
    #[test]
    fn a_capital_says_so_in_its_title_or_under_it() {
        let l = HudLayout::new(1672, 941, None);
        let mut p = HudPainter::bake(l.scale).expect("artwork bakes");
        let plain = glyphs_in(&mut p, &l, card("City", false), GOLD);
        let titled = glyphs_in(&mut p, &l, card("City", true), GOLD);
        // "City — Capital" in gold (shadows aside): its letters and the dash.
        assert!(titled >= plain + 12, "{plain} -> {titled}");
        let cyan = glyphs_in(&mut p, &l, card("City", false), CYAN);
        assert_eq!(
            glyphs_in(&mut p, &l, card("City", true), CYAN),
            cyan,
            "no note"
        );
        let long = "Metropolitan Administrative Centre";
        let noted = glyphs_in(&mut p, &l, card(long, true), CYAN);
        assert!(
            noted >= cyan + "Capital".len(),
            "the note under the title: {noted}"
        );
    }

    /// Task 144: a city is headed by its name instead of "SELECTION"; while its rename field is
    /// open the heading is the draft with a caret, and a click on it is the city's name.
    #[test]
    fn a_city_is_headed_by_its_name_and_renamed_there() {
        let l = HudLayout::new(1672, 941, None);
        let mut p = HudPainter::bake(l.scale).expect("artwork bakes");
        let mut city = card("City", false);
        let plain = glyphs_in(&mut p, &l, city.clone(), WHITE);
        city.title = Some("Roma".into());
        city.renamable = true;
        let titled = glyphs_in(&mut p, &l, city.clone(), WHITE);
        // "SELECTION" gives way to "Roma" (shadows drawn in black).
        assert_eq!(plain - titled, "SELECTION".len() - "Roma".len());
        let field = |draft: &str| HudSnapshot {
            selection: HudSelection::Single(Box::new(city.clone())),
            rename: Some(crate::HudRename {
                draft: draft.into(),
                caret: 4,
            }),
            ..HudSnapshot::default()
        };
        let cyan = |p: &mut HudPainter, h: &HudSnapshot| {
            p.compose(&l, h)
                .iter()
                .filter(|q| matches!(q.texture, HudTexture::Glyphs) && q.colour == CYAN)
                .count()
        };
        let open = cyan(&mut p, &field("Nova Roma"));
        let closed = glyphs_in(&mut p, &l, city.clone(), CYAN);
        assert!(open >= closed + "NovaRoma|".len(), "{closed} -> {open}");
        // The heading is the name's click target for an own city only.
        let heading = l.map(Group::BottomRight, SELECTION_HEADING);
        let (x, y) = (heading.x + heading.w / 2.0, heading.y + heading.h / 2.0);
        let target = |card: HudCard| {
            let h = HudSnapshot {
                selection: HudSelection::Single(Box::new(card)),
                ..HudSnapshot::default()
            };
            crate::pick::hud_target(&l, &h, (64, 64), x, y).map(|t| t.region)
        };
        assert_eq!(
            target(city.clone()),
            Some(crate::pick::HudRegionKind::CityName)
        );
        city.renamable = false;
        assert_eq!(target(city), Some(crate::pick::HudRegionKind::Panel));
    }

    /// Task 151: the help line names the entity under the pointer, and a badge beside the
    /// pointer shows the right click's order, framed in the side's colour, only while the
    /// render thread's pick agrees with the snapshot.
    #[test]
    fn the_hovered_entity_is_named_and_badged_only_while_the_pick_agrees() {
        let l = HudLayout::new(1672, 941, None);
        let mut p = HudPainter::bake(l.scale).expect("artwork bakes");
        let h = HudSnapshot {
            pointer: Some(crate::HudPointer {
                entity: 9,
                side: crate::Side::Enemy,
                badge: Some(crate::OrderBadge::Attack),
                text: "Enemy · Spearman (Romans) · 80/100".into(),
                hint: "Right-click: attack".into(),
            }),
            ..HudSnapshot::default()
        };
        let count = |p: &mut HudPainter, overlay: &HudOverlay| {
            let quads = p.compose_with(&l, &h, overlay);
            let white = quads
                .iter()
                .filter(|q| matches!(q.texture, HudTexture::Glyphs) && q.colour == WHITE)
                .count();
            let framed = quads
                .iter()
                .filter(|q| {
                    matches!(q.texture, HudTexture::Solid)
                        && q.colour == crate::Side::Enemy.colour()
                })
                .count();
            (white, framed)
        };
        let elsewhere = HudOverlay {
            pointed: Some(8),
            pointer_at: Some((600.0, 400.0)),
            ..HudOverlay::default()
        };
        let agreed = HudOverlay {
            pointed: Some(9),
            ..elsewhere.clone()
        };
        let (white_off, framed_off) = count(&mut p, &elsewhere);
        let (white_on, framed_on) = count(&mut p, &agreed);
        assert_eq!(
            framed_off, 0,
            "no badge while the pick names another entity"
        );
        assert_eq!(
            framed_on, 4,
            "the badge's frame: four sides in the enemy's red"
        );
        // The line replaces the default "Rollover an item..." text with the entity's.
        assert_ne!(white_on, white_off);
        // A HUD element under the pointer wins the help line and hides the badge.
        let over_hud = HudOverlay {
            hover: Some(HudTarget {
                region: crate::pick::HudRegionKind::Panel,
                index: 0,
            }),
            ..agreed
        };
        assert_eq!(count(&mut p, &over_hud).1, 0);
    }

    /// Task 133: the new-age dialog dims the screen and stands over everything, its story
    /// wrapped inside the panel.
    #[test]
    fn the_age_dialog_is_drawn_last_with_its_story_wrapped() {
        let l = HudLayout::new(1280, 800, None);
        let mut p = HudPainter::bake(l.scale).expect("artwork bakes");
        let plain = p.compose(&l, &HudSnapshot::default()).len();
        let dialog = HudAgeDialog {
            heading: "AGE 4 OF 13 · HISTORY".into(),
            name: "Enlightenment Age".into(),
            subtitle: "Reason against darkness".into(),
            story: "Your scholars have turned their eyes from heaven to the world, and the world \
                    has answered. Reason is the new faith; the printing presses never stop. \
                    Mind the crowd at the palace gates: reason can be turned against kings."
                .into(),
            hint: "Click or press any key to continue".into(),
        };
        let shown = HudSnapshot {
            age_dialog: Some(dialog.clone()),
            ..HudSnapshot::default()
        };
        let quads = p.compose(&l, &shown).to_vec();
        assert!(quads.len() > plain + 100);
        // The dimming covers the whole screen, after everything the HUD drew before it.
        let dim = &quads[plain];
        assert_eq!((dim.dest.w, dim.dest.h), (1280.0, 800.0));
        // The story takes several lines, each inside the panel.
        let lines = p.wrap(&dialog.story, 600.0, TextSize::Large, 1.0);
        assert!(lines.len() >= 3, "{lines:?}");
        assert_eq!(
            lines.join(" "),
            dialog
                .story
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        );
        let panel = quads[plain + 1].dest;
        for q in &quads[plain + 1..] {
            assert!(
                q.dest.x >= panel.x - 0.5 && q.dest.right() <= panel.right() + 0.5,
                "{:?} outside {panel:?}",
                q.dest
            );
        }
    }
}

#[cfg(test)]
mod portrait_tests {
    use super::*;

    fn tile(model: Option<u16>, count: usize) -> HudTile {
        HudTile {
            icon: "icons/pic_shield".to_owned(),
            model,
            name: "Spearmen".to_owned(),
            count,
            hp_permille: 1000,
        }
    }

    fn single(model: Option<u16>, owner: u8) -> HudSnapshot {
        HudSnapshot {
            selection: HudSelection::Single(Box::new(HudCard {
                icon: "icons/pic_shield".to_owned(),
                model,
                owner,
                name: "Spearman".to_owned(),
                hp: (10, 10),
                ..HudCard::default()
            })),
            ..HudSnapshot::default()
        }
    }

    /// Artwork quads whose source lies in `rect`.
    fn quads_from(p: &mut HudPainter, l: &HudLayout, h: &HudSnapshot, rect: AtlasRect) -> usize {
        p.compose(l, h)
            .iter()
            .filter(|q| {
                matches!(q.texture, HudTexture::Artwork)
                    && q.source.x as u32 == rect.x
                    && q.source.y as u32 == rect.y
            })
            .count()
    }

    /// The multi-selection's tiles ask for their types' pictures and show them once stored, the
    /// count badge and health bar still drawn; a type without a model keeps its pictogram.
    #[test]
    fn tiles_show_their_models_pictures() {
        let l = HudLayout::new(1280, 800, None);
        let mut p = HudPainter::bake(l.scale).expect("artwork bakes");
        let h = HudSnapshot {
            selection: HudSelection::Multiple {
                total: 5,
                tiles: vec![tile(Some(7), 3), tile(None, 2)],
            },
            ..HudSnapshot::default()
        };
        assert_eq!(HudPainter::shown_models(&h), vec![7]);
        assert_eq!(p.missing_models(&h), vec![7]);
        let edge = p.artwork.model_icon_edge() as usize;
        let rect = p
            .store_model_icon(7, &vec![9; edge * edge * 4], &h)
            .expect("stored");
        assert!(p.missing_models(&h).is_empty());
        assert_eq!(quads_from(&mut p, &l, &h, rect), 1);
        let texts = p
            .compose(&l, &h)
            .iter()
            .filter(|q| matches!(q.texture, HudTexture::Glyphs))
            .count();
        assert!(texts > 0, "the counts are still written");
    }

    /// The single selection's portrait is keyed by model and owner, drawn larger than a tile's
    /// picture once stored, and the oldest portrait not shown makes room when the shelf is full.
    #[test]
    fn the_single_selection_shows_its_portrait() {
        let l = HudLayout::new(1280, 800, None);
        let mut p = HudPainter::bake(l.scale).expect("artwork bakes");
        let h = single(Some(4), 2);
        let wanted = Portrait { model: 4, owner: 2 };
        assert_eq!(p.missing_portrait(&h), Some(wanted));
        assert_eq!(p.missing_portrait(&single(None, 2)), None);
        let edge = p.artwork.portrait_edge() as usize;
        let pixels = vec![5; edge * edge * 4];
        let rect = p.store_portrait(wanted, &pixels, &h).expect("stored");
        assert_eq!(p.missing_portrait(&h), None);
        // Another owner's same model is another portrait.
        assert!(p.missing_portrait(&single(Some(4), 3)).is_some());
        let drawn: Vec<HudQuad> = p
            .compose(&l, &h)
            .iter()
            .filter(|q| q.source.x as u32 == rect.x && q.source.y as u32 == rect.y)
            .cloned()
            .collect();
        assert_eq!(drawn.len(), 1);
        assert!(drawn[0].dest.w > p.artwork.model_icon_edge() as f32);
        // Fill the shelf while model 4 is shown: it stays, the oldest other one goes.
        for model in 10..10 + hud_atlas::PORTRAIT_SLOTS as u16 {
            let other = Portrait { model, owner: 1 };
            assert!(p.store_portrait(other, &pixels, &h).is_some());
        }
        assert!(p.portrait(wanted).is_some(), "the shown portrait is kept");
        assert!(
            p.portrait(Portrait {
                model: 10,
                owner: 1
            })
            .is_none()
        );
        assert!(
            p.portrait(Portrait {
                model: 11,
                owner: 1
            })
            .is_some()
        );
    }

    /// Until the portrait is rendered the tile shows the model's smaller picture if it has one.
    #[test]
    fn the_portrait_falls_back_to_the_small_picture() {
        let l = HudLayout::new(1280, 800, None);
        let mut p = HudPainter::bake(l.scale).expect("artwork bakes");
        let h = single(Some(4), 1);
        let edge = p.artwork.model_icon_edge() as usize;
        let rect = p
            .store_model_icon(4, &vec![3; edge * edge * 4], &h)
            .expect("stored");
        assert_eq!(quads_from(&mut p, &l, &h, rect), 1);
    }
}
