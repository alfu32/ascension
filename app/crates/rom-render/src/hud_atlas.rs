//! Vector artwork atlas: every SVG of `assets/hud/` rasterised with `resvg` at the current HUD
//! scale and packed into one RGBA image with a name → rectangle table.
//!
//! Baking is pure CPU work and does not need a window; the render thread uploads the result
//! as one texture.

use std::collections::BTreeMap;

use resvg::{tiny_skia, usvg};

/// Every SVG source embedded in the binary, by atlas name (`folder/file` without extension).
pub const SOURCES: &[(&str, &str)] = &[
    (
        "chrome/bar_fill",
        include_str!("../../../assets/hud/chrome/bar_fill.svg"),
    ),
    (
        "chrome/bottom_left",
        include_str!("../../../assets/hud/chrome/bottom_left.svg"),
    ),
    (
        "chrome/bottom_right",
        include_str!("../../../assets/hud/chrome/bottom_right.svg"),
    ),
    (
        "chrome/button",
        include_str!("../../../assets/hud/chrome/button.svg"),
    ),
    (
        "chrome/cell",
        include_str!("../../../assets/hud/chrome/cell.svg"),
    ),
    (
        "chrome/cell_wide",
        include_str!("../../../assets/hud/chrome/cell_wide.svg"),
    ),
    (
        "chrome/clock",
        include_str!("../../../assets/hud/chrome/clock.svg"),
    ),
    (
        "chrome/emblem",
        include_str!("../../../assets/hud/chrome/emblem.svg"),
    ),
    (
        "chrome/mark_upgrade",
        include_str!("../../../assets/hud/chrome/mark_upgrade.svg"),
    ),
    (
        "chrome/minimap",
        include_str!("../../../assets/hud/chrome/minimap.svg"),
    ),
    (
        "chrome/research_row",
        include_str!("../../../assets/hud/chrome/research_row.svg"),
    ),
    (
        "chrome/tab_active",
        include_str!("../../../assets/hud/chrome/tab_active.svg"),
    ),
    (
        "chrome/tile",
        include_str!("../../../assets/hud/chrome/tile.svg"),
    ),
    (
        "chrome/topbar",
        include_str!("../../../assets/hud/chrome/topbar.svg"),
    ),
    (
        "icons/age_0",
        include_str!("../../../assets/hud/icons/age_0.svg"),
    ),
    (
        "icons/age_1",
        include_str!("../../../assets/hud/icons/age_1.svg"),
    ),
    (
        "icons/age_2",
        include_str!("../../../assets/hud/icons/age_2.svg"),
    ),
    (
        "icons/age_3",
        include_str!("../../../assets/hud/icons/age_3.svg"),
    ),
    (
        "icons/age_4",
        include_str!("../../../assets/hud/icons/age_4.svg"),
    ),
    (
        "icons/age_5",
        include_str!("../../../assets/hud/icons/age_5.svg"),
    ),
    (
        "icons/age_6",
        include_str!("../../../assets/hud/icons/age_6.svg"),
    ),
    (
        "icons/age_7",
        include_str!("../../../assets/hud/icons/age_7.svg"),
    ),
    (
        "icons/age_8",
        include_str!("../../../assets/hud/icons/age_8.svg"),
    ),
    (
        "icons/age_9",
        include_str!("../../../assets/hud/icons/age_9.svg"),
    ),
    (
        "icons/age_10",
        include_str!("../../../assets/hud/icons/age_10.svg"),
    ),
    (
        "icons/age_11",
        include_str!("../../../assets/hud/icons/age_11.svg"),
    ),
    (
        "icons/age_12",
        include_str!("../../../assets/hud/icons/age_12.svg"),
    ),
    (
        "icons/age_13",
        include_str!("../../../assets/hud/icons/age_13.svg"),
    ),
    (
        "icons/tech_ability",
        include_str!("../../../assets/hud/icons/tech_ability.svg"),
    ),
    (
        "icons/tech_ascension",
        include_str!("../../../assets/hud/icons/tech_ascension.svg"),
    ),
    (
        "icons/tech_evolution",
        include_str!("../../../assets/hud/icons/tech_evolution.svg"),
    ),
    (
        "icons/tech_government",
        include_str!("../../../assets/hud/icons/tech_government.svg"),
    ),
    (
        "icons/tech_project",
        include_str!("../../../assets/hud/icons/tech_project.svg"),
    ),
    (
        "icons/tech_reveal",
        include_str!("../../../assets/hud/icons/tech_reveal.svg"),
    ),
    (
        "icons/tech_rule",
        include_str!("../../../assets/hud/icons/tech_rule.svg"),
    ),
    (
        "icons/tech_unify",
        include_str!("../../../assets/hud/icons/tech_unify.svg"),
    ),
    (
        "icons/tech_upgrade",
        include_str!("../../../assets/hud/icons/tech_upgrade.svg"),
    ),
    (
        "icons/track_civic",
        include_str!("../../../assets/hud/icons/track_civic.svg"),
    ),
    (
        "icons/track_commerce",
        include_str!("../../../assets/hud/icons/track_commerce.svg"),
    ),
    (
        "icons/track_military",
        include_str!("../../../assets/hud/icons/track_military.svg"),
    ),
    (
        "icons/track_science",
        include_str!("../../../assets/hud/icons/track_science.svg"),
    ),
    (
        "icons/arrow_left",
        include_str!("../../../assets/hud/icons/arrow_left.svg"),
    ),
    (
        "icons/arrow_right",
        include_str!("../../../assets/hud/icons/arrow_right.svg"),
    ),
    (
        "icons/badge_done",
        include_str!("../../../assets/hud/icons/badge_done.svg"),
    ),
    (
        "icons/brush_biome",
        include_str!("../../../assets/hud/icons/brush_biome.svg"),
    ),
    (
        "icons/brush_deposit",
        include_str!("../../../assets/hud/icons/brush_deposit.svg"),
    ),
    (
        "icons/brush_elevation",
        include_str!("../../../assets/hud/icons/brush_elevation.svg"),
    ),
    (
        "icons/brush_land",
        include_str!("../../../assets/hud/icons/brush_land.svg"),
    ),
    (
        "icons/brush_water",
        include_str!("../../../assets/hud/icons/brush_water.svg"),
    ),
    (
        "icons/centre",
        include_str!("../../../assets/hud/icons/centre.svg"),
    ),
    (
        "icons/chat",
        include_str!("../../../assets/hud/icons/chat.svg"),
    ),
    (
        "icons/capital",
        include_str!("../../../assets/hud/icons/capital.svg"),
    ),
    (
        "icons/cities",
        include_str!("../../../assets/hud/icons/cities.svg"),
    ),
    (
        "icons/clock",
        include_str!("../../../assets/hud/icons/clock.svg"),
    ),
    (
        "icons/cmd_attack",
        include_str!("../../../assets/hud/icons/cmd_attack.svg"),
    ),
    (
        "icons/cmd_attack_move",
        include_str!("../../../assets/hud/icons/cmd_attack_move.svg"),
    ),
    (
        "icons/cmd_build",
        include_str!("../../../assets/hud/icons/cmd_build.svg"),
    ),
    (
        "icons/cmd_cancel",
        include_str!("../../../assets/hud/icons/cmd_cancel.svg"),
    ),
    (
        "icons/cmd_follow",
        include_str!("../../../assets/hud/icons/cmd_follow.svg"),
    ),
    (
        "icons/cmd_garrison",
        include_str!("../../../assets/hud/icons/cmd_garrison.svg"),
    ),
    (
        "icons/cmd_guard",
        include_str!("../../../assets/hud/icons/cmd_guard.svg"),
    ),
    (
        "icons/cmd_hold",
        include_str!("../../../assets/hud/icons/cmd_hold.svg"),
    ),
    (
        "icons/cmd_move",
        include_str!("../../../assets/hud/icons/cmd_move.svg"),
    ),
    (
        "icons/cmd_patrol",
        include_str!("../../../assets/hud/icons/cmd_patrol.svg"),
    ),
    (
        "icons/cmd_rally",
        include_str!("../../../assets/hud/icons/cmd_rally.svg"),
    ),
    (
        "icons/cmd_repair",
        include_str!("../../../assets/hud/icons/cmd_repair.svg"),
    ),
    (
        "icons/cmd_stop",
        include_str!("../../../assets/hud/icons/cmd_stop.svg"),
    ),
    (
        "icons/cmd_unload",
        include_str!("../../../assets/hud/icons/cmd_unload.svg"),
    ),
    (
        "icons/commerce",
        include_str!("../../../assets/hud/icons/commerce.svg"),
    ),
    (
        "icons/diplomacy",
        include_str!("../../../assets/hud/icons/diplomacy.svg"),
    ),
    (
        "icons/event",
        include_str!("../../../assets/hud/icons/event.svg"),
    ),
    (
        "icons/flag",
        include_str!("../../../assets/hud/icons/flag.svg"),
    ),
    (
        "icons/help",
        include_str!("../../../assets/hud/icons/help.svg"),
    ),
    (
        "icons/life",
        include_str!("../../../assets/hud/icons/life.svg"),
    ),
    (
        "icons/menu",
        include_str!("../../../assets/hud/icons/menu.svg"),
    ),
    (
        "icons/objectives",
        include_str!("../../../assets/hud/icons/objectives.svg"),
    ),
    (
        "icons/pause",
        include_str!("../../../assets/hud/icons/pause.svg"),
    ),
    (
        "icons/pic_house",
        include_str!("../../../assets/hud/icons/pic_house.svg"),
    ),
    (
        "icons/pic_person",
        include_str!("../../../assets/hud/icons/pic_person.svg"),
    ),
    (
        "icons/pic_scroll",
        include_str!("../../../assets/hud/icons/pic_scroll.svg"),
    ),
    (
        "icons/pic_shield",
        include_str!("../../../assets/hud/icons/pic_shield.svg"),
    ),
    (
        "icons/pic_tower",
        include_str!("../../../assets/hud/icons/pic_tower.svg"),
    ),
    (
        "icons/pic_wonder",
        include_str!("../../../assets/hud/icons/pic_wonder.svg"),
    ),
    (
        "icons/population",
        include_str!("../../../assets/hud/icons/population.svg"),
    ),
    (
        "icons/res_energy",
        include_str!("../../../assets/hud/icons/res_energy.svg"),
    ),
    (
        "icons/res_food",
        include_str!("../../../assets/hud/icons/res_food.svg"),
    ),
    (
        "icons/res_knowledge",
        include_str!("../../../assets/hud/icons/res_knowledge.svg"),
    ),
    (
        "icons/res_metal",
        include_str!("../../../assets/hud/icons/res_metal.svg"),
    ),
    (
        "icons/res_oil",
        include_str!("../../../assets/hud/icons/res_oil.svg"),
    ),
    (
        "icons/res_timber",
        include_str!("../../../assets/hud/icons/res_timber.svg"),
    ),
    (
        "icons/res_wealth",
        include_str!("../../../assets/hud/icons/res_wealth.svg"),
    ),
    (
        "icons/research",
        include_str!("../../../assets/hud/icons/research.svg"),
    ),
    (
        "icons/stat_armour",
        include_str!("../../../assets/hud/icons/stat_armour.svg"),
    ),
    (
        "icons/stat_attack",
        include_str!("../../../assets/hud/icons/stat_attack.svg"),
    ),
    (
        "icons/stat_garrison",
        include_str!("../../../assets/hud/icons/stat_garrison.svg"),
    ),
    (
        "icons/stat_hp",
        include_str!("../../../assets/hud/icons/stat_hp.svg"),
    ),
    (
        "icons/stat_range",
        include_str!("../../../assets/hud/icons/stat_range.svg"),
    ),
    (
        "icons/stat_workers",
        include_str!("../../../assets/hud/icons/stat_workers.svg"),
    ),
    (
        "icons/tab_build",
        include_str!("../../../assets/hud/icons/tab_build.svg"),
    ),
    (
        "icons/tab_diplomacy",
        include_str!("../../../assets/hud/icons/tab_diplomacy.svg"),
    ),
    (
        "icons/tab_economy",
        include_str!("../../../assets/hud/icons/tab_economy.svg"),
    ),
    (
        "icons/tab_military",
        include_str!("../../../assets/hud/icons/tab_military.svg"),
    ),
    (
        "icons/tab_order",
        include_str!("../../../assets/hud/icons/tab_order.svg"),
    ),
    (
        "icons/tab_political",
        include_str!("../../../assets/hud/icons/tab_political.svg"),
    ),
    (
        "icons/tab_research",
        include_str!("../../../assets/hud/icons/tab_research.svg"),
    ),
    (
        "icons/tab_train",
        include_str!("../../../assets/hud/icons/tab_train.svg"),
    ),
    (
        "icons/tab_wonders",
        include_str!("../../../assets/hud/icons/tab_wonders.svg"),
    ),
    (
        "icons/zoom_in",
        include_str!("../../../assets/hud/icons/zoom_in.svg"),
    ),
    (
        "icons/zoom_out",
        include_str!("../../../assets/hud/icons/zoom_out.svg"),
    ),
];

/// Edge of an icon in design pixels when baked; icons are drawn at or below this size.
pub const ICON_DESIGN_SIZE: f32 = 44.0;
/// Model pictures (task 91) the atlas keeps room for, in a shelf below the artwork.
pub const MODEL_ICON_SLOTS: usize = 160;
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
    #[cfg(test)]
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
    shelf: Shelf,
}

/// The shelf of model pictures below the artwork: square cells in rows.
#[derive(Clone, Copy, Debug)]
struct Shelf {
    /// Top of the first row.
    y: u32,
    /// Edge of one picture.
    edge: u32,
    /// Cells in a row.
    columns: u32,
}

/// Edge in pixels of an icon baked at HUD `scale`: vector icons and model pictures alike.
pub fn icon_edge(scale: f32) -> u32 {
    (ICON_DESIGN_SIZE * scale).ceil().max(8.0) as u32
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

    /// Edge in pixels of a model picture.
    pub fn model_icon_edge(&self) -> u32 {
        self.shelf.edge
    }

    /// The rectangle of model picture slot `slot` (below [`MODEL_ICON_SLOTS`]).
    pub fn model_slot(&self, slot: usize) -> Option<AtlasRect> {
        if slot >= MODEL_ICON_SLOTS {
            return None;
        }
        let (row, column) = (
            slot as u32 / self.shelf.columns,
            slot as u32 % self.shelf.columns,
        );
        let pitch = self.shelf.edge + PADDING;
        Some(AtlasRect {
            x: PADDING + column * pitch,
            y: self.shelf.y + row * pitch,
            w: self.shelf.edge,
            h: self.shelf.edge,
        })
    }

    /// Copies straight RGBA `pixels` (`rect.w × rect.h`) into the atlas at `rect`.
    pub fn write(&mut self, rect: AtlasRect, pixels: &[u8]) {
        write_rect(&mut self.rgba, self.width, rect, pixels);
    }
}

/// Copies RGBA `pixels` (`rect.w × rect.h`) into `image`, `width` pixels wide, at `rect`.
pub fn write_rect(image: &mut [u8], width: u32, rect: AtlasRect, pixels: &[u8]) {
    let len = rect.w as usize * 4;
    assert_eq!(pixels.len(), len * rect.h as usize, "picture size");
    for row in 0..rect.h as usize {
        let to = ((rect.y as usize + row) * width as usize + rect.x as usize) * 4;
        image[to..to + len].copy_from_slice(&pixels[row * len..(row + 1) * len]);
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
        let edge = icon_edge(scale) as f32;
        (
            edge / size.width(),
            edge / size.height(),
            edge as u32,
            edge as u32,
        )
    } else {
        let width = (size.width() * scale).ceil().max(1.0);
        let height = (size.height() * scale).ceil().max(1.0);
        (
            width / size.width(),
            height / size.height(),
            width as u32,
            height as u32,
        )
    };
    let mut pixmap = tiny_skia::Pixmap::new(width, height).ok_or(AtlasError::Pixmap(name))?;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(zoom_x, zoom_y),
        &mut pixmap.as_mut(),
    );
    let mut pixels = pixmap.take();
    for pixel in pixels.as_chunks_mut::<4>().0 {
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

/// Converts straight RGBA pixels to greyscale with Rec. 709 luminance, preserving alpha.
pub fn greyscale(pixels: &[u8]) -> Vec<u8> {
    pixels
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|p| {
            let y =
                ((2126 * u32::from(p[0]) + 7152 * u32::from(p[1]) + 722 * u32::from(p[2]) + 5000)
                    / 10000) as u8;
            [y, y, y, p[3]]
        })
        .collect()
}

/// Rasterises every embedded SVG at `scale` (HUD pixels per design pixel) into one atlas.
pub fn bake(scale: f32) -> Result<AtlasImage, AtlasError> {
    bake_sources(SOURCES, scale)
}

fn bake_sources(sources: &[(&'static str, &str)], scale: f32) -> Result<AtlasImage, AtlasError> {
    let options = usvg::Options::default();
    let mut baked = sources
        .iter()
        .map(|(name, svg)| rasterise(name, svg, scale, &options))
        .collect::<Result<Vec<_>, _>>()?;
    baked.sort_by(|a, b| b.height.cmp(&a.height).then(a.name.cmp(b.name)));

    let widest = baked
        .iter()
        .map(|b| b.width + PADDING * 2)
        .max()
        .unwrap_or(1);
    let width = widest
        .next_power_of_two()
        .clamp(1024, MAX_ATLAS_WIDTH)
        .max(widest);
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
    // The model shelf: room for every picture slot, in rows as wide as the atlas.
    let edge = icon_edge(scale);
    let columns = ((width - PADDING) / (edge + PADDING)).max(1);
    let rows = (MODEL_ICON_SLOTS as u32).div_ceil(columns);
    let shelf = Shelf {
        y: y + row_height + PADDING,
        edge,
        columns,
    };
    let height = shelf.y + rows * (edge + PADDING);
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
        shelf,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greyscale_keeps_luminance() {
        let pixels = [
            255, 0, 0, 255, 0, 255, 0, 173, 0, 0, 255, 0, 128, 128, 128, 99,
        ];
        let grey = greyscale(&pixels);
        assert_eq!(
            grey,
            [
                54, 54, 54, 255, 182, 182, 182, 173, 18, 18, 18, 0, 128, 128, 128, 99
            ]
        );
        for (src, dst) in pixels
            .as_chunks::<4>()
            .0
            .iter()
            .zip(grey.as_chunks::<4>().0.iter())
        {
            let luma = 0.2126 * f32::from(src[0])
                + 0.7152 * f32::from(src[1])
                + 0.0722 * f32::from(src[2]);
            assert!((f32::from(dst[0]) - luma).abs() <= 0.5);
            assert_eq!(dst[3], src[3]);
        }
    }

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
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .filter(|px| px[3] > 128)
                    .count();
            }
            assert!(opaque > 20, "{name} draws nothing");
        }
        // Pieces the layout relies on exist.
        for name in [
            "chrome/mark_upgrade",
            "chrome/topbar",
            "chrome/tile",
            "chrome/minimap",
            "icons/res_food",
        ] {
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
                    assert!(
                        !a.overlaps(b),
                        "{a_name} overlaps {b_name} at scale {scale}"
                    );
                }
            }
        }
    }

    /// Every model picture slot lies inside the atlas, clear of the artwork and of the other
    /// slots, at the smallest and largest HUD scales; the atlas stays within 8192 pixels.
    #[test]
    fn model_pictures_fit_the_atlas() {
        for scale in [0.25, 2.5] {
            let atlas = bake(scale).expect("bake");
            assert!(
                atlas.width <= MAX_ATLAS_WIDTH && atlas.height <= 8192,
                "{scale}"
            );
            let art: Vec<_> = atlas.entries().map(|(_, r)| r).collect();
            let slots: Vec<_> = (0..MODEL_ICON_SLOTS)
                .map(|i| atlas.model_slot(i).expect("slot"))
                .collect();
            assert!(atlas.model_slot(MODEL_ICON_SLOTS).is_none());
            for (i, slot) in slots.iter().enumerate() {
                assert_eq!((slot.w, slot.h), (icon_edge(scale), icon_edge(scale)));
                assert!(slot.x + slot.w <= atlas.width && slot.y + slot.h <= atlas.height);
                assert!(art.iter().all(|a| !a.overlaps(slot)), "slot {i} at {scale}");
                assert!(slots[i + 1..].iter().all(|b| !b.overlaps(slot)), "slot {i}");
            }
            // Writing a picture lands where the slot says.
            let mut atlas = atlas;
            let slot = slots[MODEL_ICON_SLOTS - 1];
            let picture = vec![7_u8; (slot.w * slot.h * 4) as usize];
            atlas.write(slot, &picture);
            let at = ((slot.y * atlas.width + slot.x) * 4) as usize;
            assert_eq!(&atlas.rgba[at..at + 4], &[7, 7, 7, 7]);
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

    #[test]
    fn all_authored_files_and_required_glyphs_are_embedded() {
        use std::collections::BTreeSet;
        let listed: BTreeSet<_> = SOURCES.iter().map(|(name, _)| name.to_string()).collect();
        for folder in ["chrome", "icons"] {
            for file in std::fs::read_dir(format!(
                "{}/../../assets/hud/{folder}",
                env!("CARGO_MANIFEST_DIR")
            ))
            .unwrap()
            {
                let path = file.unwrap().path();
                if path.extension().is_some_and(|e| e == "svg") {
                    let name = format!("{folder}/{}", path.file_stem().unwrap().to_str().unwrap());
                    assert!(listed.contains(&name), "unembedded SVG: {name}");
                }
            }
        }
        for name in [
            "res_food",
            "res_timber",
            "res_metal",
            "res_wealth",
            "res_knowledge",
            "res_oil",
            "res_energy",
            "commerce",
            "population",
            "capital",
            "cities",
            "research",
            "clock",
            "diplomacy",
            "chat",
            "menu",
            "pause",
            "help",
            "objectives",
            "zoom_in",
            "zoom_out",
            "centre",
            "flag",
            "stat_hp",
            "stat_attack",
            "stat_armour",
            "stat_range",
            "stat_garrison",
            "stat_workers",
            "tab_political",
            "tab_military",
            "tab_economy",
            "tab_diplomacy",
            "tab_wonders",
            "tab_build",
            "tab_order",
            "tab_train",
            "tab_research",
            "arrow_left",
            "arrow_right",
            "cmd_move",
            "cmd_attack",
            "cmd_attack_move",
            "cmd_stop",
            "cmd_hold",
            "cmd_patrol",
            "cmd_guard",
            "cmd_follow",
            "cmd_rally",
            "cmd_garrison",
            "cmd_unload",
            "cmd_repair",
            "cmd_build",
            "cmd_cancel",
            "age_0",
            "age_1",
            "age_2",
            "age_3",
            "age_4",
            "age_5",
            "age_6",
            "age_7",
            "age_8",
            "age_9",
            "age_10",
            "age_11",
            "age_12",
            "age_13",
            "track_military",
            "track_civic",
            "track_commerce",
            "track_science",
            "tech_upgrade",
            "tech_project",
            "tech_government",
            "tech_evolution",
            "tech_ascension",
            "tech_reveal",
            "tech_rule",
            "tech_unify",
            "tech_ability",
        ] {
            assert!(
                listed.contains(&format!("icons/{name}")),
                "missing glyph: {name}"
            );
        }
    }
}
