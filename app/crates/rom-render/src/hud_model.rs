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

/// Availability and progress of one palette command.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum ButtonState {
    /// Requirements and price are met.
    #[default]
    Ready,
    /// Resource icon names and the amounts still needed.
    Unaffordable { missing: Vec<(String, i64)> },
    /// First unmet requirement in player-facing words.
    Locked { reason: String },
    /// A completed, nonrepeatable technology.
    Done,
    /// A queued or active research order.
    InProgress { pct: u32 },
}

impl ButtonState {
    /// Whether pressing this button can submit an order.
    pub fn enabled(&self) -> bool {
        matches!(self, Self::Ready)
    }

    /// Rollover and event explanation.
    pub fn reason(&self) -> String {
        match self {
            Self::Locked { reason } => reason.clone(),
            Self::Unaffordable { missing } => format!(
                "Not enough {}",
                missing
                    .iter()
                    .map(|(icon, amount)| format!(
                        "{} (short {amount})",
                        icon.trim_start_matches("icons/res_")
                    ))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::Done => "Researched".into(),
            Self::InProgress { pct } => format!("Research queued / in progress ({pct}%)"),
            Self::Ready => String::new(),
        }
    }
}

/// A cell in the research overview.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResearchCell {
    /// Completed research or age.
    Done,
    /// Queued or active research.
    InProgress,
    /// Lowest unresearched level, or the next age.
    Next,
    /// A later level or age.
    Locked,
}

/// A track's label and levels in ascending order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResearchTrack {
    /// Display label supplied by the app.
    pub label: String,
    /// One cell per level.
    pub levels: Vec<ResearchCell>,
}

/// All research tracks and age pips, independent of the selected producer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResearchOverview {
    /// Tracks in metadata order.
    pub tracks: Vec<ResearchTrack>,
    /// Ages in ordinal order.
    pub ages: Vec<ResearchCell>,
}

/// A city radius to draw on the current surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HudCityRadius {
    /// Surface face.
    pub face: u8,
    /// Centre in milli-tiles.
    pub x: i32,
    /// Centre in milli-tiles.
    pub y: i32,
    /// Economic radius in milli-tiles; zero when only spacing is shown.
    pub economic: i64,
    /// Exclusion radius in milli-tiles; zero outside city placement.
    pub spacing: i64,
    /// Show the economic radius on the minimap.
    pub minimap: bool,
}

/// An alert's place on the shown surface (task 118): a pulsing ring on the minimap.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HudAlert {
    /// Surface face.
    pub face: u8,
    /// Centre in milli-tiles.
    pub x: i32,
    /// Centre in milli-tiles.
    pub y: i32,
    /// Game ticks since it was raised.
    pub age_ticks: u32,
    /// Game ticks the marker lasts (`alert.marker_ticks`).
    pub life_ticks: u32,
    /// What it marks: an attack (red) or foreign ships jumping in (amber).
    pub kind: AlertMark,
}

/// What an alert marker stands for (task 118).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlertMark {
    /// A city or army under attack: red.
    Attack,
    /// Foreign ships jumped in: amber.
    Arrival,
}

impl AlertMark {
    /// The marker's colour.
    pub fn colour(self) -> [u8; 4] {
        match self {
            Self::Attack => [255, 70, 50, 255],
            Self::Arrival => [255, 190, 60, 255],
        }
    }
}

/// A blast of an area weapon on the shown surface (task 75): its flash, shockwave, debris and
/// the scorched ground it leaves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HudBlast {
    /// Surface face.
    pub face: u8,
    /// Centre in milli-tiles.
    pub x: i32,
    /// Centre in milli-tiles.
    pub y: i32,
    /// Reach of the blast in milli-tiles.
    pub radius: i32,
    /// Game ticks since it struck.
    pub age_ticks: u32,
    /// Varies the debris, the secondary bursts and the flames.
    pub seed: u32,
}

/// A building that fell lately on the shown surface (task 75): it collapses, then its ruin
/// burns and smokes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HudWreck {
    /// Surface face.
    pub face: u8,
    /// Centre in milli-tiles.
    pub x: i32,
    /// Centre in milli-tiles.
    pub y: i32,
    /// Its type id.
    pub kind: u32,
    /// Footprint in tiles.
    pub footprint: (u8, u8),
    /// Height it stood to, in milli-tiles.
    pub height: i32,
    /// Its owner.
    pub owner: u8,
    /// Game ticks since it fell.
    pub age_ticks: u32,
    /// Varies the rubble and the flames.
    pub seed: u32,
}

/// A stain left on the ground where a building or unit was destroyed (task 80): soot, ash and
/// churned earth, and a building's debris, fading over the stain's life.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HudStain {
    /// The id the destroyed entity had; varies the look.
    pub id: u32,
    /// Surface face.
    pub face: u8,
    /// Centre in milli-tiles.
    pub x: i32,
    /// Centre in milli-tiles.
    pub y: i32,
    /// Radius in milli-tiles.
    pub radius: i32,
    /// Game tick it was destroyed at.
    pub tick: u64,
    /// A building's stain also shows its debris.
    pub building: bool,
    /// Its owner, whose colour some debris wears.
    pub owner: u8,
    /// The composed model it was drawn with, whose colours its debris takes.
    pub model: Option<u16>,
}

/// A small mark drawn over a palette picture (task 91).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum IconMark {
    /// Nothing.
    #[default]
    None,
    /// An upgrade of a type: an upward chevron.
    Upgrade,
    /// A level of a research track, drawn as that many pips.
    Level(u8),
    /// A market purchase (task 107): what one lot costs now, written along the bottom.
    Buy(u32),
    /// A market sale (task 107): what one lot pays now, written along the bottom.
    Sell(u32),
    /// A painting brush (task 114): a swatch of what it lays along the bottom (`None` when it
    /// keeps the ground) and its size, 1 to 3 rising bars at the right.
    Brush {
        /// Colour of the terrain it lays.
        swatch: Option<[u8; 3]>,
        /// Size among the brushes that paint the same: 1 small, 2 wide, 3 huge.
        size: u8,
    },
}

/// One command button.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HudButton {
    /// Slot in the 5 × 3 grid, 0 to 14, row by row.
    pub slot: usize,
    /// Atlas icon name; stands in for the model picture until it is rendered.
    pub icon: String,
    /// The composed model (an index of [`crate::SceneLooks::models`]) whose rendered picture
    /// the button shows.
    pub model: Option<u16>,
    /// The mark drawn over the picture.
    pub mark: IconMark,
    /// Display name shown in the help line.
    pub name: String,
    /// Price line, empty when free.
    pub price: String,
    /// Availability or progress, computed by the app.
    pub state: ButtonState,
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
    /// Groups of the tab's pages, listed beside the grid where the production queue stands (the
    /// ascended painter's brush families, task 114); empty for every other palette.
    pub groups: Vec<HudPaletteGroup>,
    /// Second rollover line of each tab: what it holds, or why it is closed.
    pub tab_notes: [String; 4],
}

/// One group of a palette's pages (task 114): a brush family.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HudPaletteGroup {
    /// Atlas icon name.
    pub icon: String,
    /// Display name.
    pub name: String,
    /// Buttons in the group.
    pub count: usize,
    /// True for the group shown in the grid.
    pub current: bool,
    /// True when the armed button belongs to the group.
    pub armed: bool,
    /// Second rollover line.
    pub note: String,
}

/// One entry of the production queue.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HudQueueItem {
    /// Atlas icon name; stands in for the model picture until it is rendered.
    pub icon: String,
    /// The composed model whose rendered picture the row shows.
    pub model: Option<u16>,
    /// The mark drawn over the picture.
    pub mark: IconMark,
    /// Display name.
    pub name: String,
    /// Completion percentage, 0 to 100.
    pub progress_pct: u32,
    /// Why a finished entry has not left the queue (no water beside a dock, population cap),
    /// in player-facing words; None while it is merely in progress.
    pub blocked: Option<String>,
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
    /// Keeper or trader details, one short line per row.
    pub wealth: Vec<String>,
    /// Construction percentage while unfinished.
    pub progress_pct: Option<u32>,
    /// The workers inside a gather building, one portrait each (at most twelve).
    pub inside: Vec<HudTile>,
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
    Single(Box<HudCard>),
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
    /// Face of a sphere planet (0 on a flat map).
    pub face: u8,
    /// East position in milli-tiles.
    pub x: i32,
    /// South position in milli-tiles.
    pub y: i32,
    /// Meaning.
    pub kind: MarkerKind,
    /// Ticks since the marker appeared; rally markers stay at zero.
    pub age_ticks: u32,
    /// A rally marker's producer, `(face, x, y)`: a line is drawn from there to the flag.
    pub from: Option<(u8, i32, i32)>,
}

/// The building footprint being placed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HudPlacement {
    /// Face of a sphere planet (0 on a flat map).
    pub face: u8,
    /// Footprint width in tiles.
    pub width: u8,
    /// Footprint height in tiles.
    pub height: u8,
    /// Footprint centre, east, in milli-tiles.
    pub x: i32,
    /// Footprint centre, south, in milli-tiles.
    pub y: i32,
    /// Render type, used to match the procedural building's height.
    pub kind: u16,
    /// Owner colour of the future building.
    pub owner: u8,
    /// Whether the position is currently valid; `None` before the first answer.
    pub valid: Option<bool>,
}

/// Wealth totals for the economy data tab.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HudWealth {
    /// Relics contained by own keepers.
    pub relics: usize,
    /// Accepted routes, including reserved slots.
    pub routes: usize,
    /// Effective player route limit.
    pub limit: i64,
    /// Total income per arrival over the routes.
    pub per_trip: i64,
}

/// A selected unit's patrol on the currently viewed surface: the great circle through its two
/// ends, sampled from `from` (t = 0) through `to` (t = 1000) up to t = `span_milli`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HudPatrol {
    /// Where the patrol began.
    pub from: crate::SurfacePos,
    /// The point patrolled to.
    pub to: crate::SurfacePos,
    /// How far along the circle to draw, in thousandths: 1000 between the ends; a full turn
    /// of it in orbit.
    pub span_milli: u32,
    /// Height above the ground in milli-tiles: the orbit's for a ship in orbit (the world view
    /// draws it at the height the patrolling ship is drawn at, which grows with its size).
    pub lift_milli: u32,
    /// The patrolling entity.
    pub entity: u32,
    /// For one leg of a multi-point route: the owner whose colour it is drawn in, with a
    /// marker at `from`; `None` for the plain back-and-forth patrol.
    pub route_owner: Option<u8>,
}

/// A selected trader's route on the currently viewed surface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HudTradeRoute {
    /// Home city centre.
    pub home: crate::SurfacePos,
    /// Partner city centre.
    pub target: crate::SurfacePos,
    /// Current destination, reconstructed from arrivals at city footprints.
    pub to_target: bool,
    /// Route length in milli-tiles, for ground sampling density.
    pub length: u64,
}
