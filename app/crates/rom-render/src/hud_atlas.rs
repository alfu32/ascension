//! Vector artwork atlas: every SVG of `assets/hud/` rasterised with `resvg` at the current HUD
//! scale and packed into one RGBA image with a name → rectangle table.
//!
//! Baking is pure CPU work and does not need a window; the render thread uploads the result
//! as one texture.

use std::collections::BTreeMap;

use resvg::{tiny_skia, usvg};

/// Every SVG source embedded in the binary, by atlas name (`folder/file` without extension).
pub const SOURCES: &[(&str, &str)] = &[
    ("chrome/bar_fill", include_str!("../../../assets/hud/chrome/bar_fill.svg")),
    ("chrome/bottom_left", include_str!("../../../assets/hud/chrome/bottom_left.svg")),
    ("chrome/bottom_right", include_str!("../../../assets/hud/chrome/bottom_right.svg")),
    ("chrome/button", include_str!("../../../assets/hud/chrome/button.svg")),
    ("chrome/cell", include_str!("../../../assets/hud/chrome/cell.svg")),
    ("chrome/cell_wide", include_str!("../../../assets/hud/chrome/cell_wide.svg")),
    ("chrome/clock", include_str!("../../../assets/hud/chrome/clock.svg")),
    ("chrome/emblem", include_str!("../../../assets/hud/chrome/emblem.svg")),
    ("chrome/minimap", include_str!("../../../assets/hud/chrome/minimap.svg")),
    ("chrome/research_row", include_str!("../../../assets/hud/chrome/research_row.svg")),
    ("chrome/tab_active", include_str!("../../../assets/hud/chrome/tab_active.svg")),
    ("chrome/tile", include_str!("../../../assets/hud/chrome/tile.svg")),
    ("chrome/topbar", include_str!("../../../assets/hud/chrome/topbar.svg")),
    ("icons/arrow_left", include_str!("../../../assets/hud/icons/arrow_left.svg")),
    ("icons/arrow_right", include_str!("../../../assets/hud/icons/arrow_right.svg")),
    ("icons/centre", include_str!("../../../assets/hud/icons/centre.svg")),
    ("icons/chat", include_str!("../../../assets/hud/icons/chat.svg")),
    ("icons/cities", include_str!("../../../assets/hud/icons/cities.svg")),
    ("icons/clock", include_str!("../../../assets/hud/icons/clock.svg")),
    ("icons/cmd_attack", include_str!("../../../assets/hud/icons/cmd_attack.svg")),
    ("icons/cmd_attack_move", include_str!("../../../assets/hud/icons/cmd_attack_move.svg")),
    ("icons/cmd_build", include_str!("../../../assets/hud/icons/cmd_build.svg")),
    ("icons/cmd_cancel", include_str!("../../../assets/hud/icons/cmd_cancel.svg")),
    ("icons/cmd_follow", include_str!("../../../assets/hud/icons/cmd_follow.svg")),
    ("icons/cmd_garrison", include_str!("../../../assets/hud/icons/cmd_garrison.svg")),
    ("icons/cmd_guard", include_str!("../../../assets/hud/icons/cmd_guard.svg")),
    ("icons/cmd_hold", include_str!("../../../assets/hud/icons/cmd_hold.svg")),
    ("icons/cmd_move", include_str!("../../../assets/hud/icons/cmd_move.svg")),
    ("icons/cmd_patrol", include_str!("../../../assets/hud/icons/cmd_patrol.svg")),
    ("icons/cmd_rally", include_str!("../../../assets/hud/icons/cmd_rally.svg")),
    ("icons/cmd_repair", include_str!("../../../assets/hud/icons/cmd_repair.svg")),
    ("icons/cmd_stop", include_str!("../../../assets/hud/icons/cmd_stop.svg")),
    ("icons/cmd_unload", include_str!("../../../assets/hud/icons/cmd_unload.svg")),
    ("icons/commerce", include_str!("../../../assets/hud/icons/commerce.svg")),
    ("icons/diplomacy", include_str!("../../../assets/hud/icons/diplomacy.svg")),
    ("icons/event", include_str!("../../../assets/hud/icons/event.svg")),
    ("icons/flag", include_str!("../../../assets/hud/icons/flag.svg")),
    ("icons/help", include_str!("../../../assets/hud/icons/help.svg")),
    ("icons/menu", include_str!("../../../assets/hud/icons/menu.svg")),
    ("icons/objectives", include_str!("../../../assets/hud/icons/objectives.svg")),
    ("icons/pause", include_str!("../../../assets/hud/icons/pause.svg")),
    ("icons/population", include_str!("../../../assets/hud/icons/population.svg")),
    ("icons/res_energy", include_str!("../../../assets/hud/icons/res_energy.svg")),
    ("icons/res_food", include_str!("../../../assets/hud/icons/res_food.svg")),
    ("icons/res_knowledge", include_str!("../../../assets/hud/icons/res_knowledge.svg")),
    ("icons/res_metal", include_str!("../../../assets/hud/icons/res_metal.svg")),
    ("icons/res_oil", include_str!("../../../assets/hud/icons/res_oil.svg")),
    ("icons/res_timber", include_str!("../../../assets/hud/icons/res_timber.svg")),
    ("icons/res_wealth", include_str!("../../../assets/hud/icons/res_wealth.svg")),
    ("icons/research", include_str!("../../../assets/hud/icons/research.svg")),
    ("icons/stat_armour", include_str!("../../../assets/hud/icons/stat_armour.svg")),
    ("icons/stat_attack", include_str!("../../../assets/hud/icons/stat_attack.svg")),
    ("icons/stat_garrison", include_str!("../../../assets/hud/icons/stat_garrison.svg")),
    ("icons/stat_hp", include_str!("../../../assets/hud/icons/stat_hp.svg")),
    ("icons/stat_range", include_str!("../../../assets/hud/icons/stat_range.svg")),
    ("icons/stat_workers", include_str!("../../../assets/hud/icons/stat_workers.svg")),
    ("icons/tab_build", include_str!("../../../assets/hud/icons/tab_build.svg")),
    ("icons/tab_diplomacy", include_str!("../../../assets/hud/icons/tab_diplomacy.svg")),
    ("icons/tab_economy", include_str!("../../../assets/hud/icons/tab_economy.svg")),
    ("icons/tab_military", include_str!("../../../assets/hud/icons/tab_military.svg")),
    ("icons/tab_order", include_str!("../../../assets/hud/icons/tab_order.svg")),
    ("icons/tab_political", include_str!("../../../assets/hud/icons/tab_political.svg")),
    ("icons/tab_research", include_str!("../../../assets/hud/icons/tab_research.svg")),
    ("icons/tab_train", include_str!("../../../assets/hud/icons/tab_train.svg")),
    ("icons/tab_wonders", include_str!("../../../assets/hud/icons/tab_wonders.svg")),
    ("icons/zoom_in", include_str!("../../../assets/hud/icons/zoom_in.svg")),
    ("icons/zoom_out", include_str!("../../../assets/hud/icons/zoom_out.svg")),
];

/// Edge of an icon in design pixels when baked; icons are drawn at or below this size.
pub const ICON_DESIGN_SIZE: f32 = 44.0;
/// Largest atlas width the packer will use.
const MAX_ATLAS_WIDTH: u32 = 8192;
/// Transparent pixels kept around every entry so filtering never bleeds neighbours in.
const PADDING: u32 = 2;

/// A rectangle of the atlas image in pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AtlasRect {
    /// Left edge.
    pub x: u32,
    /// Top edge.
    pub y: u32,
    /// Width.
    pub w: u32,
    /// Height.
    pub h: u32,
}

impl AtlasRect {
    fn overlaps(&self, other: &AtlasRect) -> bool {
        self.x < other.x + other.w
            && other.x < self.x + self.w
            && self.y < other.y + other.h
            && other.y < self.y + self.h
    }
}

/// Why the artwork could not be baked.
#[derive(Debug)]
pub enum AtlasError {
    /// An SVG file failed to parse.
    Parse(&'static str, String),
    /// The pixmap for an SVG could not be allocated.
    Pixmap(&'static str),
}

impl std::fmt::Display for AtlasError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AtlasError::Parse(name, error) => write!(f, "svg {name}: {error}"),
            AtlasError::Pixmap(name) => write!(f, "svg {name}: could not allocate its pixmap"),
        }
    }
}

impl std::error::Error for AtlasError {}

/// The baked artwork: straight-alpha RGBA pixels and where each SVG landed.
#[derive(Clone, Debug)]
pub struct AtlasImage {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// RGBA8 pixels, straight (not premultiplied) alpha, row-major.
    pub rgba: Vec<u8>,
    /// HUD scale the artwork was rasterised at.
    pub scale: f32,
    rects: BTreeMap<&'static str, AtlasRect>,
}

impl AtlasImage {
    /// Rectangle of the named entry (for example `chrome/topbar` or `icons/res_food`).
    pub fn rect(&self, name: &str) -> Option<AtlasRect> {
        self.rects.get(name).copied()
    }

    /// Every entry with its rectangle, in name order.
    pub fn entries(&self) -> impl Iterator<Item = (&'static str, AtlasRect)> + '_ {
        self.rects.iter().map(|(name, rect)| (*name, *rect))
    }
}

struct Baked {
    name: &'static str,
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

fn rasterise(
    name: &'static str,
    svg: &str,
    scale: f32,
    options: &usvg::Options,
) -> Result<Baked, AtlasError> {
    let tree = usvg::Tree::from_str(svg, options)
        .map_err(|error| AtlasError::Parse(name, error.to_string()))?;
    let size = tree.size();
    let (zoom_x, zoom_y, width, height) = if name.starts_with("icons/") {
        let edge = (ICON_DESIGN_SIZE * scale).ceil().max(8.0);
        (edge / size.width(), edge / size.height(), edge as u32, edge as u32)
    } else {
        let width = (size.width() * scale).ceil().max(1.0);
        let height = (size.height() * scale).ceil().max(1.0);
        (width / size.width(), height / size.height(), width as u32, height as u32)
    };
    let mut pixmap = tiny_skia::Pixmap::new(width, height).ok_or(AtlasError::Pixmap(name))?;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(zoom_x, zoom_y),
        &mut pixmap.as_mut(),
    );
    let mut pixels = pixmap.take();
    for pixel in pixels.chunks_exact_mut(4) {
        let alpha = u32::from(pixel[3]);
        if alpha != 0 && alpha != 255 {
            for channel in &mut pixel[..3] {
                *channel = ((u32::from(*channel) * 255 + alpha / 2) / alpha).min(255) as u8;
            }
        }
    }
    Ok(Baked {
        name,
        width,
        height,
        pixels,
    })
}

/// Rasterises every embedded SVG at `scale` (HUD pixels per design pixel) into one atlas.
pub fn bake(scale: f32) -> Result<AtlasImage, AtlasError> {
    bake_sources(SOURCES, scale)
}

fn bake_sources(
    sources: &[(&'static str, &str)],
    scale: f32,
) -> Result<AtlasImage, AtlasError> {
    let options = usvg::Options::default();
    let mut baked = sources
        .iter()
        .map(|(name, svg)| rasterise(name, svg, scale, &options))
        .collect::<Result<Vec<_>, _>>()?;
    baked.sort_by(|a, b| b.height.cmp(&a.height).then(a.name.cmp(b.name)));

    let widest = baked.iter().map(|b| b.width + PADDING * 2).max().unwrap_or(1);
    let width = widest.next_power_of_two().clamp(1024, MAX_ATLAS_WIDTH).max(widest);
    let mut placements = Vec::with_capacity(baked.len());
    let (mut x, mut y, mut row_height) = (PADDING, PADDING, 0);
    for item in &baked {
        if x + item.width + PADDING > width {
            x = PADDING;
            y += row_height + PADDING;
            row_height = 0;
        }
        placements.push((x, y));
        x += item.width + PADDING;
        row_height = row_height.max(item.height);
    }
    let height = (y + row_height + PADDING).max(1);
    let mut rgba = vec![0_u8; width as usize * height as usize * 4];
    let mut rects = BTreeMap::new();
    for (item, (px, py)) in baked.iter().zip(placements) {
        for row in 0..item.height {
            let from = row as usize * item.width as usize * 4;
            let to = ((py + row) as usize * width as usize + px as usize) * 4;
            let len = item.width as usize * 4;
            rgba[to..to + len].copy_from_slice(&item.pixels[from..from + len]);
        }
        rects.insert(
            item.name,
            AtlasRect {
                x: px,
                y: py,
                w: item.width,
                h: item.height,
            },
        );
    }
    Ok(AtlasImage {
        width,
        height,
        rgba,
        scale,
        rects,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_listed_svg_parses_and_rasterises() {
        let atlas = bake(1.0).expect("bake");
        assert_eq!(atlas.entries().count(), SOURCES.len());
        for (name, rect) in atlas.entries() {
            assert!(rect.w > 0 && rect.h > 0, "{name} is empty");
            let mut opaque = 0;
            for row in 0..rect.h {
                let start = (((rect.y + row) * atlas.width + rect.x) * 4) as usize;
                opaque += atlas.rgba[start..start + rect.w as usize * 4]
                    .chunks_exact(4)
                    .filter(|px| px[3] > 128)
                    .count();
            }
            assert!(opaque > 20, "{name} draws nothing");
        }
        // Pieces the layout relies on exist.
        for name in ["chrome/topbar", "chrome/tile", "chrome/minimap", "icons/res_food"] {
            assert!(atlas.rect(name).is_some(), "{name} missing");
        }
    }

    #[test]
    fn atlas_rectangles_do_not_overlap() {
        for scale in [0.6, 1.0, 1.7] {
            let atlas = bake(scale).expect("bake");
            let rects: Vec<_> = atlas.entries().collect();
            for (name, rect) in &rects {
                assert!(rect.x + rect.w <= atlas.width && rect.y + rect.h <= atlas.height);
                assert!(rect.x >= PADDING && rect.y >= PADDING, "{name}");
            }
            for (i, (a_name, a)) in rects.iter().enumerate() {
                for (b_name, b) in &rects[i + 1..] {
                    assert!(!a.overlaps(b), "{a_name} overlaps {b_name} at scale {scale}");
                }
            }
        }
    }

    #[test]
    fn icon_names_are_unique() {
        let mut names: Vec<&str> = SOURCES.iter().map(|(name, _)| *name).collect();
        let total = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), total, "duplicate atlas names");
        assert!(
            SOURCES
                .iter()
                .filter(|(name, _)| name.starts_with("icons/"))
                .count()
                >= 50
        );
    }

    /// Writes the atlas as a PNG when `HUD_ATLAS_DUMP` names a file; a development aid.
    #[test]
    fn dump_atlas_when_asked() {
        let Ok(path) = std::env::var("HUD_ATLAS_DUMP") else {
            return;
        };
        let scale = std::env::var("HUD_ATLAS_SCALE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1.0);
        let atlas = bake(scale).expect("bake");
        image::RgbaImage::from_raw(atlas.width, atlas.height, atlas.rgba)
            .expect("image")
            .save(path)
            .expect("save");
    }
}
