//! Plain interface data the simulation thread publishes for the selection, the command palette,
//! the production queue and the world overlays. No behaviour: the render thread only draws it.

/// The four palette tabs, in display order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PaletteTab {
    /// Buildings a selected builder can construct.
    #[default]
    Build,
    /// Stop, attack-move and stance buttons.
    Order,
    /// Units a selected producer can train.
    Train,
    /// Technologies a selected producer can research.
    Research,
}

impl PaletteTab {
    /// Every tab in display order.
    pub const ALL: [PaletteTab; 4] = [Self::Build, Self::Order, Self::Train, Self::Research];

    /// Position in [`Self::ALL`].
    pub fn index(self) -> usize {
        match self {
            Self::Build => 0,
            Self::Order => 1,
            Self::Train => 2,
            Self::Research => 3,
        }
    }

    /// The tab at `index`, if any.
    pub fn from_index(index: usize) -> Option<Self> {
        Self::ALL.get(index).copied()
    }
}

/// Keyboard letters of the 15 palette slots, row by row.
pub const SLOT_KEYS: [char; 15] = [
    'Q', 'W', 'E', 'R', 'T', 'A', 'S', 'D', 'F', 'G', 'Z', 'X', 'C', 'V', 'B',
];

/// Palette slot of a keyboard letter (case-insensitive).
pub fn slot_of_key(ch: char) -> Option<usize> {
    SLOT_KEYS
        .iter()
        .position(|key| key.eq_ignore_ascii_case(&ch))
}

/// One command button.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HudButton {
    /// Slot in the 5 × 3 grid, 0 to 14, row by row.
    pub slot: usize,
    /// Atlas icon name.
    pub icon: String,
    /// Display name shown in the help line.
    pub name: String,
    /// Price line, empty when free.
    pub price: String,
    /// False when locked, unaffordable or otherwise unusable.
    pub enabled: bool,
    /// Why the button is disabled, empty when enabled.
    pub reason: String,
    /// True while the button's mode is active (placement, stance).
    pub armed: bool,
}

/// The command palette of the current selection.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HudPalette {
    /// The shown tab.
    pub tab: PaletteTab,
    /// Which tabs have anything to show.
    pub tabs: [bool; 4],
    /// Shown page of the tab, from 0.
    pub page: usize,
    /// Number of pages of the tab.
    pub pages: usize,
    /// Name of the shown page; empty when the tab has one unnamed page.
    pub page_name: String,
    /// Buttons of the shown page.
    pub buttons: Vec<HudButton>,
}

/// One entry of the production queue.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HudQueueItem {
    /// Atlas icon name.
    pub icon: String,
    /// Display name.
    pub name: String,
    /// Completion percentage, 0 to 100.
    pub progress_pct: u32,
}

/// The queue of the single selected producer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HudQueue {
    /// The first entries, at most three.
    pub items: Vec<HudQueueItem>,
    /// Entries beyond the shown ones.
    pub more: usize,
}

/// Details of one selected entity.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HudCard {
    /// Atlas icon name of the placeholder pictogram.
    pub icon: String,
    /// Display name.
    pub name: String,
    /// True for an entity of another owner: shown, but without orders.
    pub foreign: bool,
    /// Current and maximum hit points.
    pub hp: (i64, i64),
    /// Attack value text, when the type has a weapon.
    pub attack: Option<String>,
    /// Armour value, when above zero.
    pub armour: Option<String>,
    /// Range text, when the type has a weapon.
    pub range: Option<String>,
    /// Garrison text, when the type can hold units.
    pub garrison: Option<String>,
    /// Workers text, when the type is a gather site.
    pub workers: Option<String>,
    /// Construction percentage while unfinished.
    pub progress_pct: Option<u32>,
}

/// One tile of the multi-selection grid.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HudTile {
    /// Atlas icon name.
    pub icon: String,
    /// Display name.
    pub name: String,
    /// Number of selected entities of the type.
    pub count: usize,
    /// Combined health in thousandths.
    pub hp_permille: u32,
}

/// Selection details.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum HudSelection {
    /// No selected entities.
    #[default]
    None,
    /// One own or foreign entity.
    Single(HudCard),
    /// Several own entities: up to twelve type tiles and the total.
    Multiple {
        /// Selected entities.
        total: usize,
        /// One tile per selected type.
        tiles: Vec<HudTile>,
    },
}

/// What a world marker means.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkerKind {
    /// Move or attack-move target (green).
    Move,
    /// Attack target (red).
    Attack,
    /// Rally point of a selected producer (cyan).
    Rally,
}

/// A short-lived marker on the ground.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HudMarker {
    /// East position in milli-tiles.
    pub x: i32,
    /// South position in milli-tiles.
    pub y: i32,
    /// Meaning.
    pub kind: MarkerKind,
    /// Ticks since the marker appeared; rally markers stay at zero.
    pub age_ticks: u32,
}

/// The building footprint being placed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HudPlacement {
    /// Footprint width in tiles.
    pub width: u8,
    /// Footprint height in tiles.
    pub height: u8,
    /// Footprint centre, east, in milli-tiles.
    pub x: i32,
    /// Footprint centre, south, in milli-tiles.
    pub y: i32,
    /// Whether the position is currently valid; `None` before the first answer.
    pub valid: Option<bool>,
}
