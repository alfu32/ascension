//! HUD layout: the rectangles of `docs/hud.md` in design space, the uniform scale and the
//! anchoring rules that turn them into screen rectangles. Pure functions, no drawing.

/// Width of the design space in pixels.
pub const DESIGN_WIDTH: f32 = 1672.0;
/// Height of the design space in pixels.
pub const DESIGN_HEIGHT: f32 = 941.0;
/// Design-space height of the top bar; the world view starts below it.
pub const TOP_BAR_HEIGHT: f32 = 130.0;
/// Design-space y where the bottom groups start (top of the selection panel).
pub const BOTTOM_GROUPS_TOP: f32 = 672.0;

/// An axis-aligned rectangle in pixels (design or screen space).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Width.
    pub w: f32,
    /// Height.
    pub h: f32,
}

impl Rect {
    /// A rectangle from its corner and size.
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    /// Right edge.
    pub fn right(&self) -> f32 {
        self.x + self.w
    }

    /// Bottom edge.
    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }

    /// Centre point.
    pub fn centre(&self) -> (f32, f32) {
        (self.x + self.w / 2.0, self.y + self.h / 2.0)
    }

    /// True when the point lies inside the rectangle.
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.right() && y >= self.y && y < self.bottom()
    }

    /// True when `other` lies entirely inside this rectangle (edges may touch).
    pub fn contains_rect(&self, other: &Rect) -> bool {
        const EPS: f32 = 1e-3;
        other.x >= self.x - EPS
            && other.y >= self.y - EPS
            && other.right() <= self.right() + EPS
            && other.bottom() <= self.bottom() + EPS
    }

    /// True when the two rectangles share area (touching edges do not count).
    pub fn overlaps(&self, other: &Rect) -> bool {
        self.x < other.right()
            && other.x < self.right()
            && self.y < other.bottom()
            && other.y < self.bottom()
    }

    /// This rectangle moved by `(dx, dy)`.
    pub fn shifted(&self, dx: f32, dy: f32) -> Rect {
        Rect::new(self.x + dx, self.y + dy, self.w, self.h)
    }

    /// This rectangle with every coordinate multiplied by `s`.
    pub fn scaled(&self, s: f32) -> Rect {
        Rect::new(self.x * s, self.y * s, self.w * s, self.h * s)
    }
}

/// How a group of regions is anchored to the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    /// Top bar: anchored to the top edge, centred horizontally.
    Top,
    /// Bottom left: anchored to the bottom left corner.
    BottomLeft,
    /// Minimap: anchored to the bottom edge, centred horizontally.
    BottomCentre,
    /// Bottom right: anchored to the bottom right corner.
    BottomRight,
}

/// A named rectangle of the HUD.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Region {
    /// Commerce, population and city rows.
    CapsBlock,
    /// The seven resource cells.
    ResourceBlock,
    /// Title emblem.
    Emblem,
    /// Research progress rows.
    ResearchBlock,
    /// Game clock.
    Clock,
    /// The four top-right buttons.
    Buttons,
    /// Chat prompt and latest event.
    MessageLines,
    /// Rollover help line.
    HelpLine,
    /// Palette tabs with their arrows.
    PaletteTabs,
    /// The 5 × 3 command grid.
    CommandGrid,
    /// Production queue panel.
    ProductionQueue,
    /// Frame around the minimap.
    MinimapFrame,
    /// The map itself, inside the frame.
    MinimapView,
    /// Zoom in, zoom out, centre and signal buttons.
    MinimapButtons,
    /// Objectives button.
    Objectives,
    /// Selection panel.
    SelectionPanel,
    /// Portrait inside the selection panel.
    Portrait,
    /// Six stat rows inside the selection panel.
    StatRows,
    /// Six sub-slots inside the selection panel.
    SubSlots,
    /// Multi-selection grid inside the selection panel.
    MultiGrid,
    /// The five data tabs.
    DataTabs,
    /// Content of the active data tab.
    DataView,
}

impl Region {
    /// Every region, in drawing order.
    pub const ALL: [Region; 22] = [
        Region::CapsBlock,
        Region::ResourceBlock,
        Region::Emblem,
        Region::ResearchBlock,
        Region::Clock,
        Region::Buttons,
        Region::MessageLines,
        Region::HelpLine,
        Region::PaletteTabs,
        Region::CommandGrid,
        Region::ProductionQueue,
        Region::MinimapFrame,
        Region::MinimapView,
        Region::MinimapButtons,
        Region::Objectives,
        Region::SelectionPanel,
        Region::Portrait,
        Region::StatRows,
        Region::SubSlots,
        Region::MultiGrid,
        Region::DataTabs,
        Region::DataView,
    ];

    /// The group the region is anchored with.
    pub fn group(self) -> Group {
        match self {
            Region::CapsBlock
            | Region::ResourceBlock
            | Region::Emblem
            | Region::ResearchBlock
            | Region::Clock
            | Region::Buttons => Group::Top,
            Region::MessageLines
            | Region::HelpLine
            | Region::PaletteTabs
            | Region::CommandGrid
            | Region::ProductionQueue => Group::BottomLeft,
            Region::MinimapFrame
            | Region::MinimapView
            | Region::MinimapButtons
            | Region::Objectives => Group::BottomCentre,
            Region::SelectionPanel
            | Region::Portrait
            | Region::StatRows
            | Region::SubSlots
            | Region::MultiGrid
            | Region::DataTabs
            | Region::DataView => Group::BottomRight,
        }
    }

    /// The region this one is drawn inside, if any; nested regions may overlap their parent.
    pub fn parent(self) -> Option<Region> {
        match self {
            Region::MinimapView | Region::MinimapButtons | Region::Objectives => {
                Some(Region::MinimapFrame)
            }
            Region::Portrait | Region::StatRows | Region::SubSlots | Region::MultiGrid => {
                Some(Region::SelectionPanel)
            }
            _ => None,
        }
    }

    /// The rectangle in design space (1672 × 941).
    pub fn design(self) -> Rect {
        match self {
            Region::CapsBlock => Rect::new(20.0, 5.0, 205.0, 122.0),
            Region::ResourceBlock => Rect::new(240.0, 8.0, 470.0, 110.0),
            Region::Emblem => Rect::new(710.0, 0.0, 250.0, 135.0),
            Region::ResearchBlock => Rect::new(990.0, 10.0, 325.0, 110.0),
            Region::Clock => Rect::new(1352.0, 38.0, 62.0, 62.0),
            Region::Buttons => Rect::new(1432.0, 42.0, 212.0, 50.0),
            Region::MessageLines => Rect::new(16.0, 578.0, 362.0, 56.0),
            Region::HelpLine => Rect::new(22.0, 644.0, 580.0, 34.0),
            Region::PaletteTabs => Rect::new(22.0, 692.0, 580.0, 38.0),
            Region::CommandGrid => Rect::new(26.0, 742.0, 330.0, 176.0),
            Region::ProductionQueue => Rect::new(370.0, 735.0, 232.0, 185.0),
            Region::MinimapFrame => Rect::new(620.0, 620.0, 345.0, 321.0),
            Region::MinimapView => Rect::new(690.0, 690.0, 240.0, 190.0),
            Region::MinimapButtons => Rect::new(636.0, 702.0, 38.0, 167.0),
            Region::Objectives => Rect::new(705.0, 886.0, 200.0, 44.0),
            Region::SelectionPanel => Rect::new(978.0, 672.0, 360.0, 258.0),
            Region::Portrait => Rect::new(995.0, 702.0, 125.0, 148.0),
            Region::StatRows => Rect::new(1138.0, 708.0, 180.0, 144.0),
            Region::SubSlots => Rect::new(998.0, 860.0, 323.0, 46.0),
            Region::MultiGrid => Rect::new(1000.0, 716.0, 327.0, 190.0),
            Region::DataTabs => Rect::new(1350.0, 695.0, 132.0, 206.0),
            Region::DataView => Rect::new(1485.0, 695.0, 160.0, 205.0),
        }
    }
}

/// Columns of the command grid.
pub const GRID_COLUMNS: usize = 5;
/// Rows of the command grid.
pub const GRID_ROWS: usize = 3;
/// Size of one command slot in design pixels.
pub const SLOT_SIZE: (f32, f32) = (58.0, 52.0);
/// Distance between command slot origins in design pixels.
pub const SLOT_PITCH: (f32, f32) = (67.0, 59.0);
/// Design-space origin of the first command slot.
pub const SLOT_ORIGIN: (f32, f32) = (28.0, 744.0);

/// Percentage change of the HUD scale beyond which the artwork is re-baked.
pub const REBAKE_THRESHOLD_PERCENT: f32 = 10.0;

/// True when `new` differs from `baked` by more than the re-bake threshold.
pub fn scale_needs_rebake(baked: f32, new: f32) -> bool {
    baked <= 0.0 || ((new - baked) / baked).abs() * 100.0 > REBAKE_THRESHOLD_PERCENT
}

/// Screen layout for one frame size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HudLayout {
    /// Frame width in pixels.
    pub width: f32,
    /// Frame height in pixels.
    pub height: f32,
    /// Uniform scale from design to screen pixels.
    pub scale: f32,
}

impl HudLayout {
    /// Layout for a `width` × `height` frame; `percent` overrides the automatic scale.
    pub fn new(width: u32, height: u32, percent: Option<u32>) -> Self {
        let (width, height) = (width as f32, height as f32);
        let scale = match percent {
            Some(percent) => percent.clamp(25, 400) as f32 / 100.0,
            None => (width / DESIGN_WIDTH).min(height / DESIGN_HEIGHT),
        };
        Self {
            width,
            height,
            scale: scale.max(0.05),
        }
    }

    /// Screen position of the design-space origin of `group`.
    pub fn origin(&self, group: Group) -> (f32, f32) {
        let full_width = DESIGN_WIDTH * self.scale;
        let bottom = self.height - DESIGN_HEIGHT * self.scale;
        match group {
            Group::Top => ((self.width - full_width) / 2.0, 0.0),
            Group::BottomLeft => (0.0, bottom),
            Group::BottomCentre => ((self.width - full_width) / 2.0, bottom),
            Group::BottomRight => (self.width - full_width, bottom),
        }
    }

    /// Maps a design-space rectangle of `group` to the screen.
    pub fn map(&self, group: Group, design: Rect) -> Rect {
        let (ox, oy) = self.origin(group);
        Rect::new(
            ox + design.x * self.scale,
            oy + design.y * self.scale,
            design.w * self.scale,
            design.h * self.scale,
        )
    }

    /// True for an inert HUD surface that consumes pointer input.
    pub fn contains_hud(&self, x: f32, y: f32) -> bool {
        y < TOP_BAR_HEIGHT * self.scale
            || [
                (Group::BottomLeft, Rect::new(0.0, 680.0, 612.0, 261.0)),
                (Group::BottomRight, Rect::new(975.0, 672.0, 697.0, 269.0)),
            ]
            .into_iter()
            .any(|(g, r)| self.map(g, r).contains(x, y))
            || [Region::MessageLines, Region::HelpLine, Region::MinimapFrame]
                .into_iter()
                .any(|r| self.rect(r).contains(x, y))
    }

    /// Screen rectangle of a region.
    pub fn rect(&self, region: Region) -> Rect {
        self.map(region.group(), region.design())
    }

    /// Screen rectangle of the command slot in column `col` and row `row`.
    pub fn command_slot(&self, col: usize, row: usize) -> Rect {
        self.map(
            Group::BottomLeft,
            Rect::new(
                SLOT_ORIGIN.0 + SLOT_PITCH.0 * col as f32,
                SLOT_ORIGIN.1 + SLOT_PITCH.1 * row as f32,
                SLOT_SIZE.0,
                SLOT_SIZE.1,
            ),
        )
    }

    /// Screen rectangle of palette tab `index` (0 to 3).
    pub fn palette_tab(&self, index: usize) -> Rect {
        self.map(
            Group::BottomLeft,
            Rect::new(54.0 + index as f32 * 130.0, 692.0, 126.0, 38.0),
        )
    }

    /// Screen rectangle of the palette page arrow: 0 for previous, 1 for next.
    pub fn palette_arrow(&self, index: usize) -> Rect {
        let x = if index == 0 { 22.0 } else { 570.0 };
        self.map(Group::BottomLeft, Rect::new(x, 692.0, 30.0, 38.0))
    }

    /// Screen rectangle of the icon slot of production queue row `index` (0 to 2).
    pub fn queue_slot(&self, index: usize) -> Rect {
        self.map(
            Group::BottomLeft,
            Rect::new(386.0, 772.0 + index as f32 * 49.0, 40.0, 38.0),
        )
    }

    /// Screen rectangle of the whole production queue row `index`.
    pub fn queue_row(&self, index: usize) -> Rect {
        self.map(
            Group::BottomLeft,
            Rect::new(380.0, 768.0 + index as f32 * 49.0, 218.0, 46.0),
        )
    }

    /// Screen rectangle of multi-selection tile `index` (0 to 11, row by row).
    pub fn multi_tile(&self, index: usize) -> Rect {
        self.map(
            Group::BottomRight,
            Rect::new(
                1000.0 + (index % 4) as f32 * 83.0,
                716.0 + (index / 4) as f32 * 65.0,
                78.0,
                60.0,
            ),
        )
    }

    /// Screen rectangle of minimap button `index` (zoom in, zoom out, centre, signal).
    pub fn minimap_button(&self, index: usize) -> Rect {
        self.map(
            Group::BottomCentre,
            Rect::new(636.0, 702.0 + index as f32 * 43.0, 38.0, 38.0),
        )
    }

    /// Screen rectangle of the vertical band between the top bar and the bottom groups,
    /// full width: the part of the frame the world is meant to be read in.
    pub fn world_area(&self) -> Rect {
        let top = TOP_BAR_HEIGHT * self.scale;
        let bottom = self.height - (DESIGN_HEIGHT - BOTTOM_GROUPS_TOP) * self.scale;
        Rect::new(0.0, top, self.width, (bottom - top).max(1.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn regions_of(group: Group) -> Vec<Region> {
        Region::ALL
            .into_iter()
            .filter(|region| region.group() == group)
            .collect()
    }

    #[test]
    fn regions_scale_uniformly_and_anchor_to_edges() {
        // 16:9 at the design size: every rectangle equals its design rectangle.
        let design = HudLayout::new(1672, 941, None);
        assert_eq!(design.scale, 1.0);
        for region in Region::ALL {
            assert_eq!(design.rect(region), region.design());
        }
        // Wider than the design: the scale follows the height, side groups stick to the sides.
        let wide = HudLayout::new(2400, 941, None);
        assert_eq!(wide.scale, 1.0);
        let caps = wide.rect(Region::CapsBlock);
        let buttons = wide.rect(Region::Buttons);
        assert!((caps.x - (364.0 + 20.0)).abs() < 1e-3);
        assert!((buttons.y - 42.0).abs() < 1e-3);
        let sel = wide.rect(Region::SelectionPanel);
        assert!((sel.right() - (2400.0 - (1672.0 - 1338.0))).abs() < 1e-3);
        assert!((wide.rect(Region::PaletteTabs).x - 22.0).abs() < 1e-3);
        let minimap = wide.rect(Region::MinimapFrame);
        assert!((minimap.centre().0 - (2400.0 / 2.0 - 1672.0 / 2.0 + 620.0 + 172.5)).abs() < 1e-3);
        // Taller than the design: the scale follows the width, bottom groups stick to the bottom.
        let tall = HudLayout::new(836, 941, None);
        assert!((tall.scale - 0.5).abs() < 1e-6);
        let tabs = tall.rect(Region::PaletteTabs);
        assert!((tabs.w - 290.0).abs() < 1e-3 && (tabs.h - 19.0).abs() < 1e-3);
        assert!((tabs.bottom() - (941.0 - (941.0 - 730.0) * 0.5)).abs() < 1e-3);
        assert!(tall.rect(Region::Clock).y.abs() < 40.0);
        // Uniform: width and height of a region scale by the same factor.
        let clock = tall.rect(Region::Clock);
        assert!((clock.w - clock.h).abs() < 1e-3);
    }

    #[test]
    fn no_two_regions_of_a_group_overlap() {
        for group in [
            Group::Top,
            Group::BottomLeft,
            Group::BottomCentre,
            Group::BottomRight,
        ] {
            let regions = regions_of(group);
            for (i, a) in regions.iter().enumerate() {
                for b in &regions[i + 1..] {
                    if a.parent() == Some(*b) || b.parent() == Some(*a) {
                        continue;
                    }
                    // The multi-selection grid replaces the portrait, stats and sub-slots.
                    if (*a == Region::MultiGrid || *b == Region::MultiGrid)
                        && a.parent().is_some()
                        && a.parent() == b.parent()
                    {
                        continue;
                    }
                    assert!(!a.design().overlaps(&b.design()), "{a:?} overlaps {b:?}");
                }
            }
        }
    }

    #[test]
    fn minimap_view_lies_inside_its_frame() {
        for (width, height) in [(1672, 941), (1280, 720), (1280, 800), (2560, 1080)] {
            let layout = HudLayout::new(width, height, None);
            let frame = layout.rect(Region::MinimapFrame);
            for child in [
                Region::MinimapView,
                Region::MinimapButtons,
                Region::Objectives,
            ] {
                assert!(
                    frame.contains_rect(&layout.rect(child)),
                    "{child:?} at {width}x{height}"
                );
            }
        }
        for child in [
            Region::Portrait,
            Region::StatRows,
            Region::SubSlots,
            Region::MultiGrid,
        ] {
            assert!(
                Region::SelectionPanel
                    .design()
                    .contains_rect(&child.design()),
                "{child:?}"
            );
        }
    }

    #[test]
    fn command_grid_has_15_slots_with_the_specified_pitch() {
        let layout = HudLayout::new(1672, 941, None);
        let mut slots = Vec::new();
        for row in 0..GRID_ROWS {
            for col in 0..GRID_COLUMNS {
                slots.push(layout.command_slot(col, row));
            }
        }
        assert_eq!(slots.len(), 15);
        assert_eq!((slots[0].x, slots[0].y), (28.0, 744.0));
        assert_eq!((slots[0].w, slots[0].h), (58.0, 52.0));
        assert_eq!(slots[1].x - slots[0].x, 67.0);
        assert_eq!(slots[GRID_COLUMNS].y - slots[0].y, 59.0);
        let grid = layout.rect(Region::CommandGrid);
        assert!(slots.iter().all(|slot| grid.contains_rect(slot)));
        let scaled = HudLayout::new(836, 470, None);
        let pitch = scaled.command_slot(1, 0).x - scaled.command_slot(0, 0).x;
        assert!((pitch - 67.0 * scaled.scale).abs() < 1e-3);
    }

    #[test]
    fn rebake_only_beyond_ten_percent() {
        assert!(!scale_needs_rebake(1.0, 1.09));
        assert!(scale_needs_rebake(1.0, 1.2));
        assert!(scale_needs_rebake(1.0, 0.85));
        assert!(scale_needs_rebake(0.0, 1.0));
    }

    #[test]
    fn world_area_sits_between_bar_and_groups() {
        let layout = HudLayout::new(1672, 941, None);
        let area = layout.world_area();
        assert_eq!((area.y, area.bottom()), (130.0, 672.0));
    }
}
