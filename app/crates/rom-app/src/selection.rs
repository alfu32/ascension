//! The simulation thread's side of interaction: the selection, control groups, the command
//! palette and build placement. It turns pick events from the render thread into commands for
//! player one, and describes the result as plain data for the HUD snapshot.
//!
//! Behaviour comes from components and production rows, never from definition keys.

use std::collections::{BTreeMap, BTreeSet};

use rom_render::{
    ButtonState, HudButton, HudCard, HudCityRadius, HudMarker, HudPalette, HudPlacement, HudQueue,
    HudQueueItem, HudRegionKind, HudSelection, HudSnapshot, HudTile, MarkerKind, Modifiers,
    PaletteTab, PickEvent, PointerButton, SurfacePos, slot_of_key,
};
use rom_rules::{DefId, EntityClass, EntityType, ProductionMode, Rules, StanceFamily};
use rom_sim::{BodyId, Command, Entity, EntityId, PlayerId, Pos, SimError, Topology, World};

use crate::palette_icons::generic_icon as icon_of;
use crate::space_view::SpaceFrame;

/// The player this interface commands.
pub const PLAYER: PlayerId = PlayerId(1);
/// Ticks an order marker stays visible: half a second.
const MARKER_TICKS: u64 = 10;
/// Ticks within which a second press of a group digit centres the camera.
const GROUP_DOUBLE_TICKS: u64 = 10;
/// Fewest ticks between two placement checks.
const PLACEMENT_CHECK_TICKS: u64 = 3;
/// Ticks a message stays in the event line: five seconds.
const MESSAGE_TICKS: u64 = 100;
/// Units trained by a Shift click.
const SHIFT_BATCH: usize = 5;
/// Combat stance behaviours offered as buttons, in this order.
const STANCE_BEHAVIORS: [&str; 4] = ["aggressive", "defensive", "stand_ground", "hold_fire"];

/// A named palette page and its (slot, product) entries.
type PageRows = (String, Vec<(usize, DefId)>);

/// Whether a technology is a cell of the research grid: a track level or an age advance.
fn is_grid_tech(rules: &Rules, def: DefId) -> bool {
    rules.techs.get(&def).is_some_and(|t| {
        matches!(
            t.kind,
            rom_rules::TechKind::Track | rom_rules::TechKind::Age
        )
    })
}

/// The research page holding the next age advance of a player in `age`: the page a Library
/// opens on. The last page when every age advance is done or none is on a page.
fn library_default_page(rules: &Rules, age: DefId, pages: &[PageRows]) -> usize {
    let Some(current) = rules.ages.get(&age).map(|a| a.ordinal) else {
        return 0;
    };
    let advances_to_next = |def: DefId| {
        rules
            .techs
            .get(&def)
            .is_some_and(|t| t.kind == rom_rules::TechKind::Age)
            && rules.tech_effects.get(&def).into_iter().flatten().any(|e| {
                e.effect == rom_rules::TechEffectKind::SetAge
                    && e.target_def_id
                        .and_then(|id| rules.ages.get(&id))
                        .is_some_and(|a| a.ordinal == current + 1)
            })
    };
    let grid_pages = pages
        .iter()
        .filter(|(_, entries)| entries.iter().any(|(_, d)| is_grid_tech(rules, *d)))
        .count();
    pages
        .iter()
        .position(|(_, entries)| entries.iter().any(|(_, d)| advances_to_next(*d)))
        .unwrap_or(grid_pages.saturating_sub(1))
}

/// What a palette button does when pressed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    Construct(DefId),
    Train(DefId),
    Research(DefId),
    Stop,
    AttackMove,
    Stance(DefId),
    Orbit,
    Travel,
    /// Every worker inside the selected gather buildings leaves.
    Release,
    /// The selected carriers set their passengers down around themselves once landed; a carrier
    /// in orbit launches its craft.
    Unload,
    /// The selected landed ships return to orbit.
    Lift,
    /// The selected ships in orbit come straight down onto the ground below them.
    Land,
    /// The next point clicked is patrolled: walked to and from, or in orbit circled.
    Patrol,
    /// The next city clicked becomes the selected traders' route partner.
    TradeRoute,
    /// The governors of the selected automated colonies pause, or resume when all are paused.
    Governor,
    /// The selected market buys one lot of a resource (task 107).
    Buy(DefId),
    /// The selected market sells one lot of a resource (task 107).
    Sell(DefId),
    /// An ascended player picks open land of a habitable planet to descend onto (task 76).
    Descend,
    /// An ascended player picks a moon or asteroid to bring to life (task 77).
    BringLife,
    /// The brush an ascended player paints the planet it brought to life with (task 77).
    Brush(DefId),
}

/// What the camera shows, as far as turning picks into orders needs it.
#[derive(Clone, Copy, Debug, Default)]
pub struct ViewCtx {
    /// The body of the planet shown (body 0 in a world of one surface).
    pub planet: BodyId,
    /// The system the camera is anchored to.
    pub system: Option<u32>,
    /// Whether the camera is at the system or star-map level, where picks are in space.
    pub in_space: bool,
    /// The plane of that system.
    pub frame: Option<SpaceFrame>,
}

#[derive(Clone, Debug)]
struct Placement {
    def: DefId,
    footprint: (i64, i64),
    centre: Option<Pos>,
    valid: Option<bool>,
    dirty: bool,
    checked: Option<u64>,
    checked_age: Option<DefId>,
}

#[derive(Clone, Copy, Debug)]
struct Marker {
    pos: Pos,
    kind: MarkerKind,
    tick: u64,
}

/// Other builders that join a construction once its site exists.
#[derive(Clone, Debug)]
struct Helpers {
    pos: Pos,
    def: DefId,
    builders: Vec<EntityId>,
    tries: u8,
}

/// Commands held back until the tick after a `stop` has been applied.
#[derive(Clone, Debug)]
struct Requeue {
    after_tick: u64,
    commands: Vec<Command>,
}

/// Presentation-only memory: the simulation keeps route direction and terminal orders private.
#[derive(Debug)]
struct RouteView {
    home: EntityId,
    target: EntityId,
    order: Option<rom_sim::OrderId>,
    to_target: bool,
}

/// Selection, groups, palette state and placement of player one.
#[derive(Debug, Default)]
pub struct Controller {
    selection: Vec<EntityId>,
    hovered: Option<EntityId>,
    blocked_notices: BTreeSet<rom_sim::OrderId>,
    routes: BTreeMap<EntityId, RouteView>,
    foreign: Option<EntityId>,
    groups: [Vec<EntityId>; 10],
    last_group: Option<(usize, u64)>,
    tab: PaletteTab,
    /// The palette page the player turned to; `None` shows the default page of the tab.
    page: Option<usize>,
    data_tab: usize,
    pending_research: BTreeMap<DefId, u64>,
    attack_move: bool,
    /// The next right-click (or click) on the ground orders a patrol there.
    patrol: bool,
    /// The next city clicked becomes the selected traders' route partner (task 72).
    trade_pick: bool,
    /// The next click on a planet's ground is where the ascended player descends (task 76).
    descend_pick: bool,
    /// The next right click on a moon or asteroid brings it to life (task 77).
    life_pick: bool,
    /// Clicks on the ground of the planet shown paint with this brush (task 77).
    brush: Option<DefId>,
    placement: Option<Placement>,
    markers: Vec<Marker>,
    helpers: Vec<Helpers>,
    requeue: Vec<Requeue>,
    message: Option<(String, u64)>,
    /// The entity the event line's message is about: clicking the line flies there.
    message_about: Option<EntityId>,
    focus: Option<Pos>,
    ordered: bool,
    view: ViewCtx,
    /// The composed models, for the palette's pictures (task 91); generic pictograms without.
    models: Option<std::sync::Arc<crate::looks::ModelBook>>,
}

/// Distance in milli-tiles between two positions; positions beyond reach (or on faces that do
/// not touch) rank last.
fn distance(world: &World, a: Pos, b: Pos) -> u64 {
    world.distance(a, b).unwrap_or(u64::MAX)
}

/// Whether an entity stands in the space of a system (on a star's plane).
/// An entity on a surface, in a system's space or between systems: a ship in transit stays
/// selectable for its whole journey.
fn present(world: &World, id: EntityId) -> Option<&Entity> {
    world.entity(id).or_else(|| {
        world
            .transits()
            .find(|t| t.entity.id == id)
            .map(|t| &t.entity)
    })
}

/// Whether `entity` is an orbital craft in a planet's air that can climb to its orbit.
fn climbs(world: &World, entity: &Entity) -> bool {
    world.is_craft_type(entity.type_id)
        && world.orbit(entity.id).is_none()
        && entity.container_id.is_none()
        && !in_space(world, entity)
}

/// Whether units of this type run trade routes (caravans, space freighters).
fn runs_routes(world: &World, type_id: DefId) -> bool {
    world.rules().entity_types[&type_id]
        .trader
        .as_ref()
        .is_some_and(|t| t.runs_routes)
}

/// Whether the entity is a finished city of a player (not a neutral one).
fn is_city(world: &World, entity: &Entity) -> bool {
    entity.owner != PlayerId(0)
        && entity.build_progress.is_none()
        && world.rules().entity_types[&entity.type_id]
            .territory
            .as_ref()
            .is_some_and(|t| t.is_city)
}

/// Whether the entity orbits a planet and can come down onto it (a carrier, a freighter or an
/// orbital craft; never a satellite).
fn lands(world: &World, entity: &Entity) -> bool {
    world.orbit(entity.id).is_some()
        && !world.is_orbit_bound(entity.type_id)
        && (world.can_land_type(entity.type_id) || world.is_craft_type(entity.type_id))
}

/// Height of a ship in orbit above the ground, in tiles, as the world view draws it.
const ORBIT_LIFT_MILLI: u32 = 8000;

fn in_space(world: &World, entity: &Entity) -> bool {
    world
        .body(entity.pos.body)
        .is_some_and(|b| b.kind == rom_rules::BodyKind::Star)
}

/// Reconstructs the arrival check from public city footprints; route direction is private.
fn reached_city(world: &World, unit: Pos, city: &Entity) -> bool {
    // A freighter between planets (or in their orbit) is on another body than its city.
    if unit.body != city.pos.body {
        return false;
    }
    let et = &world.rules().entity_types[&city.type_id];
    let pos = |p: Pos| SurfacePos {
        face: p.face,
        x: i64::from(p.x),
        y: i64::from(p.y),
    };
    let delta = match world.surface(city.pos.body).map(|s| s.topology()) {
        Some(Topology::Cube(surface)) => surface.unfold(pos(city.pos), pos(unit)),
        Some(Topology::Flat { .. }) => Some((
            i64::from(unit.x) - i64::from(city.pos.x),
            i64::from(unit.y) - i64::from(city.pos.y),
        )),
        None => None,
    };
    let Some((x, y)) = delta else { return false };
    let dx = (x.abs() - et.footprint_w * 500).max(0);
    let dy = (y.abs() - et.footprint_h * 500).max(0);
    let reach = world.rules().constant("build.reach").unwrap_or(0);
    i128::from(dx) * i128::from(dx) + i128::from(dy) * i128::from(dy)
        <= i128::from(reach) * i128::from(reach)
}

fn snap(coordinate: i32, size: i64) -> i32 {
    ((i64::from(coordinate) - size * 500 + 500).div_euclid(1000) * 1000 + size * 500) as i32
}

/// A blocked production order's reason in player-facing words.
fn blocked_reason(reason: &str) -> String {
    match reason {
        "pop_cap" => "Population limit reached",
        "no_spawn_tile" => "No free ground or water beside it",
        "space_age" => "Needs the space age",
        "launch_site" | "no_spawn_space" => "No room to launch",
        other => other,
    }
    .to_owned()
}

fn rules_class(world: &World, entity: &Entity) -> EntityClass {
    world.rules().entity_types[&entity.type_id].class
}

/// The icon of a brush (task 77): a deposit shows the resource it yields, other brushes their
/// family's picture.
fn brush_icon(rules: &Rules, brush: DefId) -> String {
    let Some(row) = rules.brushes.get(&brush) else {
        return "icons/brush_land".into();
    };
    let resource = row
        .node_type_id
        .and_then(|n| {
            rules
                .entity_types
                .get(&n)?
                .node
                .as_ref()
                .map(|n| n.resource_id)
        })
        .or_else(|| {
            let terrain = row.terrain_id?;
            rules
                .entity_types
                .values()
                .filter_map(|et| et.gather_site.as_ref())
                .find(|g| g.needs_node_terrain_id == Some(terrain))
                .map(|g| g.resource_id)
        });
    match (row.family, resource) {
        (rom_rules::BrushFamily::Deposit | rom_rules::BrushFamily::Biome, Some(resource)) => {
            let key = rules.key(resource);
            format!("icons/res_{}", key.split_once('.').map_or(key, |(_, k)| k))
        }
        (rom_rules::BrushFamily::Deposit, None) => "icons/res_oil".into(),
        (rom_rules::BrushFamily::Land, _) => "icons/brush_land".into(),
        (rom_rules::BrushFamily::Water, _) => "icons/brush_water".into(),
        (rom_rules::BrushFamily::Elevation, _) => "icons/brush_elevation".into(),
        (rom_rules::BrushFamily::Biome, None) => "icons/brush_biome".into(),
    }
}

fn stance_icon(behavior: &str) -> &'static str {
    match behavior {
        "aggressive" => "icons/cmd_attack",
        "defensive" => "icons/cmd_guard",
        "stand_ground" => "icons/cmd_hold",
        _ => "icons/cmd_cancel",
    }
}

/// Whether player one may place a building of type `def` centred near `centre`. Delegates to
/// the simulation's own check, so the preview always agrees with `construct` at submit.
pub fn placement_check(world: &World, def: DefId, centre: Pos) -> Result<(), SimError> {
    world.check_placement(PLAYER, def, centre).map(|_| ())
}

impl Controller {
    /// An empty controller.
    pub fn new() -> Self {
        Self::default()
    }

    /// Gives the controller the composed models, so palette buttons show their pictures.
    pub fn set_models(&mut self, models: std::sync::Arc<crate::looks::ModelBook>) {
        self.models = Some(models);
    }

    /// Tells the controller what the camera shows, so picks become positions on the right body.
    pub fn set_view(&mut self, view: ViewCtx) {
        self.view = view;
    }

    /// The simulation position of a picked surface position: milli-tiles on a face of the planet
    /// shown, or, in space, orbit-radius units times 1000 from the star of the system shown.
    fn pos_of(&self, surface: SurfacePos) -> Pos {
        if self.view.in_space
            && let Some(frame) = self.view.frame
        {
            return frame.pos_of(surface.x as f32 / 1000.0, surface.y as f32 / 1000.0);
        }
        let clamp = |v: i64| v.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
        Pos {
            x: clamp(surface.x),
            y: clamp(surface.y),
            face: surface.face,
            body: self.view.planet,
        }
    }

    /// Puts a line in the event line.
    pub fn notify(&mut self, world: &World, text: impl Into<String>) {
        self.say(world, text);
    }

    /// Puts a line about an entity in the event line; clicking the line flies the camera to it.
    pub fn notify_about(&mut self, world: &World, text: impl Into<String>, about: EntityId) {
        self.say(world, text);
        self.message_about = Some(about);
    }

    /// Player one's finished buildings that train units and are not cities.
    pub fn select_producers(&mut self, world: &World) {
        let rules = world.rules();
        let ids: Vec<EntityId> = world
            .entities()
            .filter(|e| e.owner == PLAYER && e.build_progress.is_none())
            .filter(|e| {
                let et = &rules.entity_types[&e.type_id];
                et.class == EntityClass::Building
                    && et.territory.as_ref().is_none_or(|t| !t.is_city)
                    && rules
                        .production
                        .get(&e.type_id)
                        .into_iter()
                        .flatten()
                        .any(|r| r.mode == ProductionMode::Train)
            })
            .map(|e| e.id)
            .collect();
        self.set_selection(ids);
    }

    /// Finished producers offering track research, in entity id order.
    pub fn select_researchers(&mut self, world: &World) {
        let rules = world.rules();
        self.set_selection(
            world
                .entities()
                .filter(|e| e.owner == PLAYER && e.build_progress.is_none())
                .filter(|e| {
                    rules
                        .production
                        .get(&e.type_id)
                        .into_iter()
                        .flatten()
                        .any(|r| {
                            r.mode == ProductionMode::Research
                                && rules
                                    .techs
                                    .get(&r.product_def_id)
                                    .is_some_and(|t| t.kind == rom_rules::TechKind::Track)
                        })
                })
                .map(|e| e.id)
                .collect(),
        );
    }

    /// Player one's ships (movers standing in a system's space), ascending.
    pub fn select_own_ships(&mut self, world: &World) {
        let ships: Vec<EntityId> = world
            .entities()
            .filter(|e| e.owner == PLAYER && e.container_id.is_none() && in_space(world, e))
            .filter(|e| world.rules().entity_types[&e.type_id].movement.is_some())
            .map(|e| e.id)
            .collect();
        self.set_selection(ships);
    }

    /// Selects every orbital craft of player one that is not aboard a carrier.
    pub fn select_own_craft(&mut self, world: &World) {
        let craft: Vec<EntityId> = world
            .entities()
            .filter(|e| e.owner == PLAYER && e.container_id.is_none())
            .filter(|e| world.is_craft_type(e.type_id))
            .map(|e| e.id)
            .collect();
        self.set_selection(craft);
    }

    /// True once the player has issued any order; the demo then leaves player one alone.
    pub fn player_has_ordered(&self) -> bool {
        self.ordered
    }

    /// A camera focus request made since the last call.
    pub fn take_focus_pos(&mut self) -> Option<Pos> {
        self.focus.take()
    }

    /// The focus request in tiles of its face.
    #[cfg(test)]
    pub fn take_focus(&mut self) -> Option<(f32, f32)> {
        self.focus
            .take()
            .map(|p| (p.x as f32 / 1000.0, p.y as f32 / 1000.0))
    }

    /// Selected entity ids, ascending.
    #[cfg(test)]
    pub fn selection(&self) -> &[EntityId] {
        &self.selection
    }

    // ----- helpers -------------------------------------------------------------------------

    fn say(&mut self, world: &World, text: impl Into<String>) {
        self.message = Some((text.into(), world.tick()));
        self.message_about = None;
    }

    fn mine<'w>(&self, world: &'w World, id: EntityId) -> Option<&'w Entity> {
        present(world, id).filter(|e| e.owner == PLAYER)
    }

    fn own_selected<'w>(&self, world: &'w World) -> Vec<&'w Entity> {
        self.selection
            .iter()
            .filter_map(|id| self.mine(world, *id))
            .collect()
    }

    fn is_enemy(world: &World, entity: &Entity) -> bool {
        let team_of = |id: PlayerId| world.players().find(|p| p.id == id).map(|p| p.team);
        entity.owner != PLAYER
            && entity.owner != PlayerId(0)
            && team_of(entity.owner) != team_of(PLAYER)
    }

    fn set_selection(&mut self, mut ids: Vec<EntityId>) {
        ids.sort();
        ids.dedup();
        self.selection = ids;
        self.foreign = None;
        self.page = None;
    }

    #[allow(clippy::too_many_arguments)]
    fn submit(
        &mut self,
        world: &mut World,
        executor: EntityId,
        kind: &str,
        subject: Option<DefId>,
        target_entity: Option<EntityId>,
        target_pos: Option<Pos>,
        queue: bool,
    ) -> bool {
        let command = Command {
            player: PLAYER,
            executor,
            kind: kind.into(),
            subject,
            target_entity,
            target_pos,
            queue,
        };
        match world.submit(command) {
            Ok(_) => {
                if kind == "research"
                    && let Some(def) = subject
                {
                    self.pending_research.insert(def, world.tick());
                }
                self.ordered = true;
                true
            }
            Err(error) => {
                self.say(world, rejection_message(world, &error, subject));
                false
            }
        }
    }

    // ----- events --------------------------------------------------------------------------

    /// Applies one pick event.
    pub fn handle(&mut self, world: &mut World, event: PickEvent) {
        match event {
            PickEvent::Click {
                surface,
                entity,
                modifiers,
                double,
                on_screen,
                ..
            } => self.click(
                world,
                self.pos_of(surface),
                entity,
                modifiers,
                double,
                &on_screen,
            ),
            PickEvent::BoxSelect {
                entities,
                modifiers,
            } => self.box_select(world, &entities, modifiers),
            PickEvent::RightClick {
                surface,
                entity,
                modifiers,
                ..
            } => {
                let target = self.pos_of(surface);
                self.right_click(world, target, entity, modifiers);
            }
            PickEvent::SpaceRightClick {
                surface,
                entity,
                body,
                system,
                modifiers,
            } => self.space_right_click(world, surface, entity, body, system, modifiers),
            PickEvent::HudClick {
                region,
                index,
                button,
                modifiers,
            } => self.hud_click(world, region, index, button, modifiers),
            PickEvent::Hover {
                surface, entity, ..
            } => {
                self.hovered = entity.map(EntityId);
                let at = self.pos_of(surface);
                self.hover(world, at);
            }
            PickEvent::HoverExit => self.hovered = None,
            PickEvent::Hotkey { ch, modifiers } => self.hotkey(world, ch, modifiers),
            PickEvent::Escape => {
                self.placement = None;
                self.descend_pick = false;
                self.life_pick = false;
                self.brush = None;
                self.attack_move = false;
                self.patrol = false;
                self.trade_pick = false;
            }
        }
    }

    fn click(
        &mut self,
        world: &mut World,
        at: Pos,
        entity: Option<u32>,
        mods: Modifiers,
        double: bool,
        on_screen: &[u32],
    ) {
        if self.placement.is_some() {
            self.place(world, at, mods.shift);
            return;
        }
        if self.descend_pick && !self.view.in_space {
            self.descend(world, at);
            return;
        }
        if let Some(brush) = self.brush.filter(|_| !self.view.in_space) {
            self.paint(world, brush, at);
            return;
        }
        // The pick ends once no trader is selected any more.
        self.trade_pick &= !self.selected_traders(world).is_empty();
        if self.trade_pick {
            self.pick_trade_partner(world, entity.map(EntityId), mods.shift);
            return;
        }
        if self.patrol {
            let ids: Vec<EntityId> = self.own_selected(world).iter().map(|e| e.id).collect();
            for id in ids {
                self.submit(world, id, "patrol", None, None, Some(at), mods.shift);
            }
            self.marker(world, at, MarkerKind::Move);
            self.patrol = mods.shift;
            return;
        }
        if self.attack_move {
            let target = at;
            let ids: Vec<EntityId> = self.own_selected(world).iter().map(|e| e.id).collect();
            for id in ids {
                let orbital_target = entity.filter(|_| world.orbit(id).is_some()).map(EntityId);
                let (kind, point) = if orbital_target.is_some() {
                    ("attack", None)
                } else {
                    ("attack_move", Some(target))
                };
                self.submit(world, id, kind, None, orbital_target, point, mods.shift);
            }
            self.marker(world, target, MarkerKind::Move);
            self.attack_move = mods.shift;
            return;
        }
        let Some(id) = entity.map(EntityId) else {
            if !mods.shift {
                self.set_selection(Vec::new());
            }
            return;
        };
        let Some(clicked) = present(world, id) else {
            return;
        };
        if clicked.owner != PLAYER {
            self.set_selection(Vec::new());
            self.foreign = Some(id);
            return;
        }
        let type_id = clicked.type_id;
        if double {
            let same: Vec<EntityId> = on_screen
                .iter()
                .map(|n| EntityId(*n))
                .filter(|n| self.mine(world, *n).is_some_and(|e| e.type_id == type_id))
                .collect();
            let mut next = if mods.shift {
                self.selection.clone()
            } else {
                Vec::new()
            };
            next.extend(same);
            next.push(id);
            self.set_selection(next);
            return;
        }
        let mut next = self.selection.clone();
        if mods.shift {
            if let Some(at) = next.iter().position(|n| *n == id) {
                next.remove(at);
            } else {
                next.push(id);
            }
        } else {
            next = vec![id];
        }
        self.set_selection(next);
    }

    /// Descends where the player clicked (task 76), or says why it cannot.
    fn descend(&mut self, world: &mut World, at: Pos) {
        let command = Command {
            player: PLAYER,
            executor: EntityId(0),
            kind: "descend".into(),
            subject: None,
            target_entity: None,
            target_pos: Some(at),
            queue: false,
        };
        match world.submit(command) {
            Ok(_) => {
                self.descend_pick = false;
                self.ordered = true;
                self.marker(world, at, MarkerKind::Move);
                self.say(world, "Descending: a city and its citizens take shape");
            }
            Err(error) => {
                let text = rejection_message(world, &error, None);
                self.say(world, text);
            }
        }
    }

    /// Brings the moon or asteroid `body` to life (task 77), or says why it cannot.
    fn bring_life(&mut self, world: &mut World, body: BodyId) {
        let Some(marker) = world.body_marker(body) else {
            self.say(world, "Pick a moon or asteroid");
            return;
        };
        let command = Command {
            player: PLAYER,
            executor: EntityId(0),
            kind: "bring_life".into(),
            subject: None,
            target_entity: Some(marker),
            target_pos: None,
            queue: false,
        };
        match world.submit(command) {
            Ok(_) => {
                self.life_pick = false;
                self.ordered = true;
                let name = world
                    .body(body)
                    .map_or_else(String::new, |b| b.name.clone());
                self.say(world, format!("Bringing {name} to life"));
            }
            Err(error) => {
                let text = rejection_message(world, &error, None);
                self.say(world, text);
            }
        }
    }

    /// One stroke of `brush` at `at` on the planet shown (task 77), or why it cannot be.
    fn paint(&mut self, world: &mut World, brush: DefId, at: Pos) {
        let command = Command {
            player: PLAYER,
            executor: EntityId(0),
            kind: "paint".into(),
            subject: Some(brush),
            target_entity: None,
            target_pos: Some(at),
            queue: false,
        };
        match world.submit(command) {
            Ok(_) => self.ordered = true,
            Err(error) => {
                self.brush = None;
                let text = rejection_message(world, &error, Some(brush));
                self.say(world, text);
            }
        }
    }

    /// The brushes the person may paint the planet shown with, by family in rules order, as
    /// pages of the palette: none unless it brought that planet to life and may paint it.
    fn brush_pages(&self, world: &World) -> Vec<PageRows> {
        if self.view.in_space || world.paint_check(PLAYER, self.view.planet).is_err() {
            return Vec::new();
        }
        let rules = world.rules();
        let mut by_family: BTreeMap<rom_rules::BrushFamily, Vec<DefId>> = BTreeMap::new();
        for brush in rules.brushes.values() {
            by_family.entry(brush.family).or_default().push(brush.id);
        }
        by_family
            .into_iter()
            .flat_map(|(family, mut ids)| {
                ids.sort_by_key(|id| (rules.def(*id).sort, *id));
                let name = match family {
                    rom_rules::BrushFamily::Land => "Land",
                    rom_rules::BrushFamily::Water => "Water",
                    rom_rules::BrushFamily::Elevation => "Elevation",
                    rom_rules::BrushFamily::Biome => "Biomes",
                    rom_rules::BrushFamily::Deposit => "Deposits",
                };
                ids.chunks(15)
                    .map(|page| {
                        (
                            name.to_owned(),
                            page.iter().enumerate().map(|(s, id)| (s, *id)).collect(),
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    fn box_select(&mut self, world: &World, entities: &[u32], mods: Modifiers) {
        let rules = world.rules();
        let units: Vec<&Entity> = entities
            .iter()
            .filter_map(|n| self.mine(world, EntityId(*n)))
            .filter(|e| rules.entity_types[&e.type_id].class == EntityClass::Unit)
            .collect();
        let armed = units
            .iter()
            .any(|e| !rules.entity_types[&e.type_id].weapons.is_empty());
        let mut next: Vec<EntityId> = units
            .iter()
            .filter(|e| mods.ctrl || !armed || !rules.entity_types[&e.type_id].weapons.is_empty())
            .map(|e| e.id)
            .collect();
        if mods.shift {
            next.extend(self.selection.iter().copied());
        }
        if next.is_empty() && mods.shift {
            return;
        }
        self.set_selection(next);
    }

    /// Blasts of area weapons and fallen buildings the player sees on the planet shown, for
    /// the bombardment effects (task 75), and the stains of what was destroyed there (task 80).
    fn bombardment(&self, world: &World, hud: &mut rom_render::HudSnapshot) {
        let body = self.view.planet;
        let tick = world.tick();
        let sees = |pos: Pos| {
            world.is_visible_on_body(
                PLAYER,
                pos.body,
                pos.face,
                pos.x.div_euclid(1000),
                pos.y.div_euclid(1000),
            )
        };
        let seed = |t: u64, pos: Pos| {
            (t as u32)
                .wrapping_mul(0x9E37_79B1)
                .wrapping_add((pos.x as u32).wrapping_mul(31))
                .wrapping_add(pos.y as u32)
        };
        hud.blasts = world
            .blasts()
            .iter()
            .filter(|b| b.pos.body == body && sees(b.pos))
            .map(|b| rom_render::HudBlast {
                face: b.pos.face,
                x: b.pos.x,
                y: b.pos.y,
                radius: b.radius.clamp(0, i64::from(i32::MAX)) as i32,
                age_ticks: tick.saturating_sub(b.tick).min(u64::from(u32::MAX)) as u32,
                seed: seed(b.tick, b.pos),
            })
            .collect();
        let rules = world.rules();
        hud.wrecks = world
            .wrecks()
            .iter()
            .filter(|w| w.pos.body == body && sees(w.pos))
            .filter_map(|w| {
                let et = rules.entity_type(w.type_id)?;
                let (fw, fh) = (et.footprint_w.clamp(1, 255), et.footprint_h.clamp(1, 255));
                // The plain box height the views give a building of this footprint and kind; the
                // simulation thread puts the height of its model in its place.
                let height = 1000 + (w.type_id.0 % 4) as i64 * 400 + 150 * fw.max(fh);
                Some(rom_render::HudWreck {
                    face: w.pos.face,
                    x: w.pos.x,
                    y: w.pos.y,
                    kind: w.type_id.0,
                    footprint: (fw as u8, fh as u8),
                    height: height as i32,
                    owner: u8::try_from(w.owner.0).unwrap_or(u8::MAX),
                    age_ticks: tick.saturating_sub(w.tick).min(u64::from(u32::MAX)) as u32,
                    seed: seed(w.tick, w.pos),
                })
            })
            .collect();
        // The stains where things were destroyed, where the player sees now (task 80).
        hud.stain_life_ticks = world.stain_life();
        hud.stains = world
            .stains(body)
            .filter(|s| sees(s.pos))
            .map(|s| rom_render::HudStain {
                id: s.id.0,
                face: s.pos.face,
                x: s.pos.x,
                y: s.pos.y,
                radius: i32::try_from(s.radius).unwrap_or(i32::MAX),
                tick: s.tick,
                building: rules
                    .entity_type(s.type_id)
                    .is_some_and(|et| et.class == rom_rules::EntityClass::Building),
                owner: u8::try_from(s.owner.0).unwrap_or(u8::MAX),
                // The simulation thread puts the model it was drawn with in.
                model: None,
            })
            .collect();
    }

    /// The patrols of the selected units on the planet shown: between their ends on the ground,
    /// all the way round the great circle through them in orbit.
    fn patrols(&self, world: &World) -> Vec<rom_render::HudPatrol> {
        let surface = |p: Pos| SurfacePos {
            face: p.face,
            x: i64::from(p.x),
            y: i64::from(p.y),
        };
        self.own_selected(world)
            .iter()
            .filter_map(|e| {
                let order = e.orders().front().filter(|o| o.kind == "patrol")?;
                let (from, to) = (order.home?, order.target_pos?);
                if from.body != self.view.planet || to.body != self.view.planet {
                    return None;
                }
                let orbit = world.orbit(e.id).is_some();
                let ends = world.sphere_point(from).zip(world.sphere_point(to));
                let span = match ends.filter(|_| orbit) {
                    Some((a, b)) => {
                        let (a, b) = (a.map(f64::from), b.map(f64::from));
                        let length = |v: [f64; 3]| v.iter().map(|c| c * c).sum::<f64>().sqrt();
                        let dot: f64 =
                            (0..3).map(|i| a[i] * b[i]).sum::<f64>() / (length(a) * length(b));
                        let angle = dot.clamp(-1.0, 1.0).acos();
                        if angle < 1e-4 {
                            return None;
                        }
                        (std::f64::consts::TAU / angle * 1000.0) as u32
                    }
                    None => 1000,
                };
                Some(rom_render::HudPatrol {
                    from: surface(from),
                    to: surface(to),
                    span_milli: span,
                    lift_milli: if orbit { ORBIT_LIFT_MILLI } else { 0 },
                    entity: e.id.0,
                })
            })
            .collect()
    }

    fn marker(&mut self, world: &World, pos: Pos, kind: MarkerKind) {
        if self.view.in_space
            || world
                .body(pos.body)
                .is_some_and(|b| b.kind == rom_rules::BodyKind::Star)
        {
            return;
        }
        self.markers.push(Marker {
            pos,
            kind,
            tick: world.tick(),
        });
    }

    fn right_click(
        &mut self,
        world: &mut World,
        target: Pos,
        entity: Option<u32>,
        mods: Modifiers,
    ) {
        if self.placement.take().is_some() {
            return;
        }
        let rules = world.rules().clone();
        enum Kind {
            Ground,
            Board(EntityId),
            Enemy(EntityId),
            Site(EntityId),
            Gather(EntityId, DefId),
            Carry(EntityId, u32),
            Trade(EntityId),
        }
        let kind = entity
            .map(EntityId)
            .and_then(|id| world.entity(id))
            .map_or(Kind::Ground, |e| {
                let et = &rules.entity_types[&e.type_id];
                if let Some(node) = et
                    .node
                    .as_ref()
                    .filter(|n| n.claim_mode == rom_rules::ClaimMode::Carry)
                    && e.container_id.is_none()
                    && let Some(tag) = node.claim_tag_id
                {
                    Kind::Carry(e.id, tag)
                } else if Self::is_enemy(world, e) {
                    Kind::Enemy(e.id)
                } else if e.owner == PLAYER && e.build_progress.is_some() {
                    Kind::Site(e.id)
                } else if e.owner == PLAYER
                    && world.is_carrier_type(e.type_id)
                    && (world.is_landed(e.id) || world.orbit(e.id).is_some())
                {
                    Kind::Board(e.id)
                } else if e.owner != PlayerId(0)
                    && e.build_progress.is_none()
                    && et.territory.as_ref().is_some_and(|t| t.is_city)
                {
                    Kind::Trade(e.id)
                } else if let (PLAYER, Some(site)) = (e.owner, et.gather_site.as_ref()) {
                    Kind::Gather(e.id, site.resource_id)
                } else {
                    Kind::Ground
                }
            });
        let mut selected: Vec<EntityId> = self.own_selected(world).iter().map(|e| e.id).collect();
        // An order given to a gather building applies to every worker inside it.
        let inside: Vec<EntityId> = selected
            .iter()
            .flat_map(|id| world.occupants(*id))
            .filter(|id| !selected.contains(id))
            .collect();
        selected.extend(inside);
        let mut issued = false;
        for id in selected {
            let Some(e) = world.entity(id) else {
                continue;
            };
            let et = &rules.entity_types[&e.type_id];
            let can_move = et.movement.is_some();
            let armed = !et.weapons.is_empty();
            let queue = mods.shift;
            let orbiting_here = world.orbit(id).is_some_and(|o| o.body == target.body);
            match &kind {
                Kind::Board(ship)
                    if *ship != id
                        && et.class == EntityClass::Unit
                        && !world.is_spacecraft_type(et.id) =>
                {
                    issued |= self.submit(world, id, "enter", None, Some(*ship), None, queue);
                }
                // A ship above this planet moves within its orbit to above the clicked point; it
                // comes down only with the Land button.
                Kind::Ground if orbiting_here => {
                    let kind = if self.patrol {
                        "patrol"
                    } else if self.attack_move && armed {
                        "attack_move"
                    } else {
                        "move"
                    };
                    issued |= self.submit(world, id, kind, None, None, Some(target), queue);
                }
                // A landed carrier sends its passengers to the clicked point.
                Kind::Ground if world.is_landed(id) => {
                    if !world.passengers(id).is_empty() {
                        issued |= self.submit(world, id, "unload", None, None, Some(target), queue);
                    }
                }
                Kind::Carry(node, tag)
                    if et.class == EntityClass::Unit && rules.has_tag(et.id, *tag) =>
                {
                    issued |= self.submit(world, id, "carry", None, Some(*node), None, queue);
                }
                Kind::Trade(city) if et.trader.as_ref().is_some_and(|t| t.runs_routes) => {
                    issued |= self.submit(world, id, "trade", None, Some(*city), None, queue);
                }
                Kind::Enemy(foe) if armed && can_move => {
                    issued |= self.submit(world, id, "attack", None, Some(*foe), None, queue);
                }
                Kind::Site(site) if et.builder.is_some() => {
                    issued |= self.submit(world, id, "repair", None, Some(*site), None, queue);
                }
                Kind::Gather(site, resource)
                    if et.harvester.iter().any(|h| h.resource_id == *resource) =>
                {
                    issued |= self.submit(world, id, "harvest", None, Some(*site), None, queue);
                }
                _ if can_move => {
                    let kind = if self.patrol {
                        "patrol"
                    } else if self.attack_move && armed {
                        "attack_move"
                    } else {
                        "move"
                    };
                    issued |= self.submit(world, id, kind, None, None, Some(target), queue);
                }
                _ => {
                    if et.producer.as_ref().is_some_and(|p| p.has_rally) {
                        // A gather site becomes the rally target: new workers harvest there.
                        let site = match &kind {
                            Kind::Gather(site, _) if *site != id => Some(*site),
                            _ => None,
                        };
                        let at = site.is_none().then_some(target);
                        issued |= self.submit(world, id, "set_rally", None, site, at, false);
                    }
                }
            }
        }
        if issued {
            let kind = if matches!(kind, Kind::Enemy(_)) {
                MarkerKind::Attack
            } else {
                MarkerKind::Move
            };
            let at = match entity.map(EntityId).and_then(|id| world.entity(id)) {
                Some(e) => e.pos,
                None => target,
            };
            self.marker(world, at, kind);
            self.attack_move = false;
            self.patrol = false;
            self.trade_pick = false;
        }
    }

    /// The own selected units that run trade routes (caravans, space freighters).
    fn selected_traders(&self, world: &World) -> Vec<EntityId> {
        self.own_selected(world)
            .iter()
            .filter(|e| runs_routes(world, e.type_id))
            .map(|e| e.id)
            .collect()
    }

    /// The click while picking a trade partner: a city takes the selected traders' route;
    /// anything else says what to pick and keeps the pick open.
    fn pick_trade_partner(&mut self, world: &mut World, entity: Option<EntityId>, shift: bool) {
        let city = entity
            .and_then(|id| world.entity(id))
            .filter(|e| is_city(world, e))
            .map(|e| (e.id, e.pos));
        let Some((city, at)) = city else {
            let hint = self.trade_hint(world);
            self.say(world, hint);
            return;
        };
        let mut issued = false;
        for id in self.selected_traders(world) {
            issued |= self.submit(world, id, "trade", None, Some(city), None, shift);
        }
        if issued {
            self.marker(world, at, MarkerKind::Move);
        }
        self.trade_pick = false;
    }

    /// What to pick for a trade route: for a space freighter a city on another planet of its
    /// system (or of another system once interstellar trade is researched), for a caravan another
    /// city of its world.
    fn trade_hint(&self, world: &World) -> String {
        let freighter = self
            .selected_traders(world)
            .into_iter()
            .filter_map(|id| world.entity(id))
            .any(|e| world.is_spacecraft_type(e.type_id));
        if !freighter {
            return "Pick another city of yours or an ally's for the trade route".into();
        }
        if world.rule_enabled(PLAYER, rom_sim::RULE_INTERSTELLAR_TRADE) {
            "Pick a city on another planet of this system, or in another star system".into()
        } else {
            "Pick a city on another planet of this system".into()
        }
    }

    /// Why `trader` cannot run a route to `city`, in words for the player.
    fn trade_refusal(world: &World, trader: &Entity, city: &Entity, error: &SimError) -> String {
        let space = world.is_spacecraft_type(trader.type_id);
        let here = world.orbit(trader.id).map_or(trader.pos.body, |o| o.body);
        match error {
            SimError::NotATradePartner if space && city.pos.body == here => {
                "the same planet as the freighter".into()
            }
            SimError::NoHomeCity if space => "no city of yours where the freighter is".into(),
            _ => {
                let mut words = rejection_message(world, error, None);
                if let Some(first) = words.get_mut(..1) {
                    first.make_ascii_lowercase();
                }
                words
            }
        }
    }

    /// The cities on the planet shown that the selected trader could run a route to, while a
    /// trade partner is being picked.
    fn trade_targets(&self, world: &World) -> Vec<u32> {
        let Some(trader) = self.selected_traders(world).first().copied() else {
            return Vec::new();
        };
        if !self.trade_pick || self.view.in_space {
            return Vec::new();
        }
        world
            .entities()
            .filter(|e| e.pos.body == self.view.planet && is_city(world, e))
            .filter(|e| world.trade_offer(PLAYER, trader, e.id).is_ok())
            .map(|e| e.id.0)
            .collect()
    }

    /// Own ships selected, in space (not between systems) and able to move.
    fn selected_ships(&self, world: &World) -> Vec<EntityId> {
        let rules = world.rules();
        self.own_selected(world)
            .iter()
            .filter(|e| world.entity(e.id).is_some())
            .filter(|e| in_space(world, e) && rules.entity_types[&e.type_id].movement.is_some())
            .map(|e| e.id)
            .collect()
    }

    fn order_orbit(
        &mut self,
        world: &mut World,
        ship: EntityId,
        body: BodyId,
        queue: bool,
    ) -> bool {
        let Some(marker) = world.body_marker(body) else {
            self.say(world, "That body cannot be orbited");
            return false;
        };
        self.submit(world, ship, "orbit", None, Some(marker), None, queue)
    }

    fn order_travel(
        &mut self,
        world: &mut World,
        ship: EntityId,
        system: u32,
        queue: bool,
    ) -> bool {
        let Some(at) = world
            .space_of(system)
            .and_then(|star| world.body_position(star))
        else {
            self.say(world, "That system cannot be reached");
            return false;
        };
        self.submit(world, ship, "travel", None, None, Some(at), queue)
    }

    /// A right click in space: attack a ship, orbit a body, travel to a system or move. A body or
    /// a point of another system's plane sends the ship there along the routes (task 111): it
    /// arrives at that point if the player has explored the system, at its edge if not.
    fn space_right_click(
        &mut self,
        world: &mut World,
        surface: SurfacePos,
        entity: Option<u32>,
        body: Option<u32>,
        system: Option<u32>,
        mods: Modifiers,
    ) {
        if self.life_pick
            && let Some(body) = body
        {
            self.bring_life(world, BodyId(body));
            return;
        }
        let ships = self.selected_ships(world);
        let foe = entity
            .map(EntityId)
            .and_then(|id| world.entity(id))
            .filter(|e| Self::is_enemy(world, e))
            .map(|e| e.id);
        let point = self
            .view
            .frame
            .map(|f| f.pos_of(surface.x as f32 / 1000.0, surface.y as f32 / 1000.0));
        for id in ships {
            let armed = world
                .entity(id)
                .is_some_and(|e| !world.rules().entity_types[&e.type_id].weapons.is_empty());
            let here = world
                .entity(id)
                .and_then(|e| world.body(e.pos.body))
                .map(|b| b.system);
            let elsewhere = |world: &World, body: BodyId| {
                world.body(body).is_some_and(|b| Some(b.system) != here)
            };
            if let (Some(foe), true) = (foe, armed) {
                self.submit(world, id, "attack", None, Some(foe), None, mods.shift);
            } else if let Some(body) = body.map(BodyId) {
                if elsewhere(world, body)
                    && let Some(at) = world.body_position(body)
                {
                    self.submit(world, id, "travel", None, None, Some(at), mods.shift);
                } else {
                    self.order_orbit(world, id, body, mods.shift);
                }
            } else if let Some(system) = system {
                self.order_travel(world, id, system, mods.shift);
            } else if let Some(at) = point {
                let kind = if elsewhere(world, at.body) {
                    "travel"
                } else {
                    "move"
                };
                self.submit(world, id, kind, None, None, Some(at), mods.shift);
            }
        }
    }

    /// The Orbit button: every selected ship holds the nearest body of its system.
    fn orbit_nearest(&mut self, world: &mut World) {
        for id in self.selected_ships(world) {
            let Some(ship) = world.entity(id) else {
                continue;
            };
            let system = world.body(ship.pos.body).map(|b| b.system);
            let near = world
                .bodies()
                .iter()
                .filter(|b| Some(b.system) == system && b.kind != rom_rules::BodyKind::Star)
                .filter_map(|b| {
                    let at = world.body_position(b.id)?;
                    Some((world.distance(ship.pos, at)?, b.id))
                })
                .min();
            if let Some((_, body)) = near {
                self.order_orbit(world, id, body, false);
            }
        }
    }

    /// The Unload button: a landed carrier sets everyone down around itself; a launching carrier
    /// in orbit sends its craft out; anything else in orbit must land first.
    fn unload_selected(&mut self, world: &mut World) {
        let carriers: Vec<EntityId> = self
            .own_selected(world)
            .iter()
            .filter(|e| !world.passengers(e.id).is_empty())
            .map(|e| e.id)
            .collect();
        for id in carriers {
            let launches = world.orbit(id).is_some() && world.launches_craft(id);
            if world.is_landed(id) || launches {
                self.submit(world, id, "unload", None, Some(id), None, false);
            } else {
                self.say(
                    world,
                    "Land the ship first: troops leave it only on the ground",
                );
            }
        }
    }

    /// The Travel button: every selected ship goes to the system the camera is anchored to, or,
    /// when that is its own, to the nearest system along a route.
    fn travel_on(&mut self, world: &mut World) {
        for id in self.selected_ships(world) {
            let Some(ship) = world.entity(id) else {
                continue;
            };
            let Some(here) = world.body(ship.pos.body).map(|b| b.system) else {
                continue;
            };
            let target = self.view.system.filter(|s| *s != here).or_else(|| {
                world
                    .routes()
                    .iter()
                    .filter_map(|r| {
                        let other = if r.system_a == here {
                            r.system_b
                        } else if r.system_b == here {
                            r.system_a
                        } else {
                            return None;
                        };
                        Some((r.length, other))
                    })
                    .min()
                    .map(|(_, other)| other)
            });
            match target {
                Some(system) => {
                    self.order_travel(world, id, system, false);
                }
                None => self.say(world, "No system to travel to"),
            }
        }
    }

    fn hotkey(&mut self, world: &mut World, ch: char, mods: Modifiers) {
        if let Some(group) = ch.to_digit(10).map(|d| d as usize) {
            if mods.ctrl {
                let own: Vec<EntityId> = self.own_selected(world).iter().map(|e| e.id).collect();
                self.groups[group] = own;
                return;
            }
            let alive: Vec<EntityId> = self.groups[group]
                .iter()
                .copied()
                .filter(|id| self.mine(world, *id).is_some())
                .collect();
            if alive.is_empty() {
                return;
            }
            let twice = self.last_group.is_some_and(|(g, at)| {
                g == group && world.tick().saturating_sub(at) <= GROUP_DOUBLE_TICKS
            });
            self.last_group = Some((group, world.tick()));
            if twice {
                let topology = alive
                    .first()
                    .and_then(|id| world.entity(*id))
                    .and_then(|e| {
                        world.surface(world.orbit(e.id).map_or(e.pos.body, |orbit| orbit.body))
                    })
                    .map_or_else(|| world.topology(), |s| s.topology());
                self.focus = match topology {
                    Topology::Flat { .. } => {
                        let (mut x, mut y) = (0.0, 0.0);
                        for id in &alive {
                            if let Some(e) = world.entity(*id) {
                                x += e.pos.x as f32 / 1000.0;
                                y += e.pos.y as f32 / 1000.0;
                            }
                        }
                        let count = alive.len() as f32;
                        Some(Pos::flat(
                            (x / count * 1000.0).round() as i32,
                            (y / count * 1000.0).round() as i32,
                        ))
                    }
                    // Face coordinates do not average across faces: centre on the first member.
                    Topology::Cube(_) => alive
                        .first()
                        .and_then(|id| world.entity(*id))
                        .map(|e| world.orbit(e.id).map_or(e.pos, |orbit| orbit.pos)),
                };
            }
            self.set_selection(alive);
            return;
        }
        if let Some(slot) = slot_of_key(ch) {
            let has_button = self
                .palette(world)
                .is_some_and(|(_, buttons)| buttons.iter().any(|(s, _)| *s == slot));
            if has_button {
                self.press_slot(world, slot, mods.shift);
                return;
            }
        }
        if ch.eq_ignore_ascii_case(&'a') && self.can_attack_move(world) {
            self.attack_move = !self.attack_move;
        }
    }

    fn can_attack_move(&self, world: &World) -> bool {
        let rules = world.rules();
        self.own_selected(world).iter().any(|e| {
            let et = &rules.entity_types[&e.type_id];
            et.movement.is_some() && !et.weapons.is_empty()
        })
    }

    fn hud_click(
        &mut self,
        world: &mut World,
        region: HudRegionKind,
        index: usize,
        button: PointerButton,
        mods: Modifiers,
    ) {
        match region {
            HudRegionKind::DataTab if button == PointerButton::Left => {
                self.data_tab = index.min(4);
            }
            HudRegionKind::PaletteTab if button == PointerButton::Left => {
                if let (Some(tab), Some((palette, _))) =
                    (PaletteTab::from_index(index), self.palette(world))
                    && palette.tabs[index]
                {
                    self.tab = tab;
                    self.page = None;
                }
            }
            HudRegionKind::PaletteArrow if button == PointerButton::Left => {
                if let Some((palette, _)) = self.palette(world)
                    && palette.pages > 1
                {
                    self.page = Some(if index == 0 {
                        (palette.page + palette.pages - 1) % palette.pages
                    } else {
                        (palette.page + 1) % palette.pages
                    });
                }
            }
            HudRegionKind::CommandSlot if button == PointerButton::Left => {
                self.press_slot(world, index, mods.shift);
            }
            HudRegionKind::QueueItem if button == PointerButton::Right => {
                self.cancel_queued(world, index);
            }
            HudRegionKind::InsideWorker | HudRegionKind::InsideWorkerAll
                if button == PointerButton::Left =>
            {
                let inside = self.inside_workers(world);
                let Some(&worker) = inside.get(index) else {
                    return;
                };
                let ids: Vec<EntityId> = if region == HudRegionKind::InsideWorkerAll {
                    // Every worker of that type inside the building.
                    let kind = world.entity(worker).map(|w| w.type_id);
                    inside
                        .iter()
                        .copied()
                        .filter(|id| world.entity(*id).map(|w| w.type_id) == kind)
                        .collect()
                } else {
                    vec![worker]
                };
                let mut next: Vec<EntityId> = if mods.shift {
                    // Shift adds to the workers and units already chosen, not to the building.
                    self.own_selected(world)
                        .iter()
                        .filter(|e| rules_class(world, e) == EntityClass::Unit)
                        .map(|e| e.id)
                        .collect()
                } else {
                    Vec::new()
                };
                next.extend(ids);
                self.set_selection(next);
            }
            HudRegionKind::SelectionTile if button == PointerButton::Left => {
                let types = self.tile_types(world);
                if let Some(type_id) = types.get(index) {
                    let keep: Vec<EntityId> = self
                        .own_selected(world)
                        .iter()
                        .filter(|e| e.type_id == *type_id)
                        .map(|e| e.id)
                        .collect();
                    self.set_selection(keep);
                }
            }
            _ => {}
        }
    }

    /// The workers inside the single selected own building, in strip order (at most twelve).
    fn inside_workers(&self, world: &World) -> Vec<EntityId> {
        match self.own_selected(world).as_slice() {
            [one] if one.owner == PLAYER => {
                let mut ids = world.occupants(one.id);
                if ids.is_empty() {
                    ids = world.passengers(one.id);
                }
                ids.truncate(12);
                ids
            }
            _ => Vec::new(),
        }
    }

    /// Types of the multi-selection tiles, in tile order.
    fn tile_types(&self, world: &World) -> Vec<DefId> {
        let mut types: Vec<DefId> = self.own_selected(world).iter().map(|e| e.type_id).collect();
        types.sort();
        types.dedup();
        types.truncate(12);
        types
    }

    // ----- palette -------------------------------------------------------------------------

    /// The palette of the current selection, with the action behind each slot.
    fn palette(&self, world: &World) -> Option<(HudPalette, Vec<(usize, Action)>)> {
        // An ascended player commands nothing material: it may descend (task 76), bring moons
        // and asteroids to life and paint the planets it brought to life (task 77).
        if world.is_ascended(PLAYER) {
            let brushes = self.brush_pages(world);
            let tabs = [!brushes.is_empty(), true, false, false];
            let tab = if tabs[self.tab.index()] {
                self.tab
            } else {
                PaletteTab::Order
            };
            let mut palette = HudPalette {
                tab,
                tabs,
                pages: 1,
                ..HudPalette::default()
            };
            let actions: Vec<(usize, Action)> = if tab == PaletteTab::Build {
                let count = brushes.len().max(1);
                let page = self.page.unwrap_or(0).min(count - 1);
                let (name, entries) = brushes.get(page).cloned().unwrap_or_default();
                palette.pages = count;
                palette.page = page;
                palette.page_name = name;
                entries
                    .into_iter()
                    .map(|(s, d)| (s, Action::Brush(d)))
                    .collect()
            } else {
                vec![(0, Action::Descend), (1, Action::BringLife)]
            };
            palette.buttons = actions
                .iter()
                .map(|(slot, action)| self.button(world, world.rules(), *slot, *action, &[]))
                .collect();
            return Some((palette, actions));
        }
        let own = self.own_selected(world);
        if own.is_empty() {
            return None;
        }
        let rules = world.rules();
        let player = world.players().find(|p| p.id == PLAYER)?;
        let sources = player.active_sources();
        let owned = world.owned_types(PLAYER);
        let first_with = |pick: &dyn Fn(&Entity, &EntityType) -> bool| {
            own.iter()
                .find(|e| pick(e, &rules.entity_types[&e.type_id]))
                .map(|e| e.type_id)
        };
        let builder = first_with(&|_, et| et.builder.is_some());
        let producer = first_with(&|e, et| et.producer.is_some() && e.build_progress.is_none());
        let rows = |type_id: Option<DefId>, mode: ProductionMode| -> Vec<PageRows> {
            let Some(type_id) = type_id else {
                return Vec::new();
            };
            // A producer whose research includes track or age rows shows them as a grid.
            let grid = mode == ProductionMode::Research
                && rules
                    .production
                    .get(&type_id)
                    .into_iter()
                    .flatten()
                    .any(|r| r.mode == mode && is_grid_tech(rules, r.product_def_id));
            let mut by_page: BTreeMap<String, Vec<&rom_rules::Production>> = BTreeMap::new();
            for row in rules.production.get(&type_id).into_iter().flatten() {
                if row.mode != mode {
                    continue;
                }
                let def = row.product_def_id;
                if mode == ProductionMode::Train
                    && (!rules.is_available(def, player.faction)
                        || rules.is_obsolete(def, &sources, &owned)
                        || (!rules.is_unlocked(def, &sources, &owned)
                            && rules.entity_types[&def].upgrades_from_id.is_some()))
                {
                    continue;
                }
                if mode == ProductionMode::Construct && rules.is_superseded_building(def, &sources)
                {
                    continue;
                }
                if mode == ProductionMode::Research
                    && !(grid && is_grid_tech(rules, def))
                    && !crate::hud::next_research(world, def)
                {
                    continue;
                }
                let page = if mode == ProductionMode::Research && !grid {
                    String::new()
                } else {
                    row.ui_page.clone()
                };
                by_page.entry(page).or_default().push(row);
            }
            let mut pages = Vec::new();
            for (name, mut rows) in by_page {
                rows.sort_by_key(|r| (r.ui_row, r.ui_col, r.product_def_id));
                if mode == ProductionMode::Research && !grid {
                    rows.sort_by_key(|r| {
                        let tech = &rules.techs[&r.product_def_id];
                        let priority = match tech.kind {
                            rom_rules::TechKind::Track => 0,
                            rom_rules::TechKind::Age => 1,
                            _ => 2,
                        };
                        (
                            priority,
                            &tech.track,
                            tech.level,
                            r.ui_row,
                            r.ui_col,
                            r.product_def_id,
                        )
                    });
                    for (n, chunk) in rows.chunks(15).enumerate() {
                        pages.push((
                            if n == 0 {
                                String::new()
                            } else {
                                format!("Research {}", n + 1)
                            },
                            chunk
                                .iter()
                                .enumerate()
                                .map(|(slot, row)| (slot, row.product_def_id))
                                .collect(),
                        ));
                    }
                    continue;
                }
                let mut slots: Vec<Vec<Option<DefId>>> = Vec::new();
                let mut spill: Vec<DefId> = Vec::new();
                let mut first: Vec<Option<DefId>> = vec![None; 15];
                for row in &rows {
                    let wanted = (0..5).contains(&row.ui_col) && (0..3).contains(&row.ui_row);
                    let slot = row.ui_row as usize * 5 + row.ui_col as usize;
                    if wanted && first[slot].is_none() {
                        first[slot] = Some(row.product_def_id);
                    } else {
                        spill.push(row.product_def_id);
                    }
                }
                slots.push(first);
                for def in spill {
                    let free = slots
                        .iter_mut()
                        .find_map(|page| page.iter_mut().find(|s| s.is_none()));
                    match free {
                        Some(slot) => *slot = Some(def),
                        None => {
                            let mut page = vec![None; 15];
                            page[0] = Some(def);
                            slots.push(page);
                        }
                    }
                }
                for (n, page) in slots.into_iter().enumerate() {
                    let label = if n == 0 {
                        name.clone()
                    } else {
                        format!("{name} {}", n + 1)
                    };
                    pages.push((
                        label,
                        page.into_iter()
                            .enumerate()
                            .filter_map(|(slot, def)| def.map(|d| (slot, d)))
                            .collect(),
                    ));
                }
            }
            if grid {
                // Grid pages come first, from the lowest level to the highest.
                pages.sort_by_key(|(name, entries): &PageRows| {
                    let level = entries
                        .iter()
                        .filter(|(_, d)| is_grid_tech(rules, *d))
                        .map(|(_, d)| rules.techs[d].level)
                        .min();
                    (level.is_none(), level, name.clone())
                });
            }
            pages
        };
        let build_pages = rows(builder, ProductionMode::Construct);
        let train_pages = rows(producer, ProductionMode::Train);
        let research_pages = rows(producer, ProductionMode::Research);
        let armed_mover = own.iter().any(|e| {
            let et = &rules.entity_types[&e.type_id];
            et.movement.is_some() && !et.weapons.is_empty()
        });
        let can_move = own
            .iter()
            .any(|e| rules.entity_types[&e.type_id].movement.is_some());
        let has_workers = own
            .iter()
            .any(|e| e.owner == PLAYER && !world.occupants(e.id).is_empty());
        let has_colony = own
            .iter()
            .any(|e| e.owner == PLAYER && world.is_colony_type(e.type_id));
        // Markets alone: their orders are the trades (task 107).
        let markets = !own.is_empty()
            && own
                .iter()
                .all(|e| e.owner == PLAYER && world.market_currency(e.id).is_some());
        let tabs = [
            !build_pages.is_empty(),
            can_move || has_workers || has_colony || markets,
            !train_pages.is_empty(),
            !research_pages.is_empty(),
        ];
        let tab = if tabs[self.tab.index()] {
            self.tab
        } else {
            PaletteTab::ALL
                .into_iter()
                .find(|t| tabs[t.index()])
                .unwrap_or_default()
        };
        let mut palette = HudPalette {
            tab,
            tabs,
            pages: 1,
            ..HudPalette::default()
        };
        let mut actions: Vec<(usize, Action)> = Vec::new();
        let research_default = library_default_page(rules, player.age, &research_pages);
        let production = |pages: &[PageRows],
                          make: fn(DefId) -> Action,
                          default: usize|
         -> (usize, usize, String, Vec<(usize, Action)>) {
            let count = pages.len().max(1);
            let page = self.page.unwrap_or(default).min(count - 1);
            let (name, entries) = pages.get(page).cloned().unwrap_or_default();
            (
                count,
                page,
                name,
                entries.into_iter().map(|(s, d)| (s, make(d))).collect(),
            )
        };
        match tab {
            PaletteTab::Build | PaletteTab::Train | PaletteTab::Research => {
                let (pages, make, default): (&[PageRows], fn(DefId) -> Action, usize) = match tab {
                    PaletteTab::Build => (&build_pages, Action::Construct, 0),
                    PaletteTab::Train => (&train_pages, Action::Train, 0),
                    _ => (&research_pages, Action::Research, research_default),
                };
                let (count, page, name, entries) = production(pages, make, default);
                palette.pages = count;
                palette.page = page;
                palette.page_name = name;
                actions = entries;
            }
            PaletteTab::Order => {
                actions.push((0, Action::Stop));
                if has_workers {
                    actions.push((3, Action::Release));
                }
                if has_colony {
                    actions.push((7, Action::Governor));
                }
                if markets {
                    let resources = world.tradeable_resources(PLAYER);
                    for (n, resource) in resources.into_iter().take(5).enumerate() {
                        actions.push((5 + n, Action::Buy(resource)));
                        actions.push((10 + n, Action::Sell(resource)));
                    }
                }
                if own.iter().any(|e| !world.passengers(e.id).is_empty()) {
                    actions.push((4, Action::Unload));
                }
                if own
                    .iter()
                    .any(|e| world.is_landed(e.id) || climbs(world, e))
                {
                    actions.push((5, Action::Lift));
                }
                if own.iter().any(|e| lands(world, e)) {
                    actions.push((6, Action::Land));
                }
                if armed_mover || own.iter().any(|e| world.orbit(e.id).is_some()) {
                    actions.push((7, Action::Patrol));
                }
                if own
                    .iter()
                    .any(|e| e.owner == PLAYER && runs_routes(world, e.type_id))
                {
                    actions.push((8, Action::TradeRoute));
                }
                if own.iter().any(|e| {
                    in_space(world, e)
                        && rules.entity_types[&e.type_id].movement.is_some()
                        && !world.is_craft_type(e.type_id)
                        && !world.is_orbit_bound(e.type_id)
                }) {
                    actions.push((2, Action::Orbit));
                    actions.push((3, Action::Travel));
                }
                if armed_mover {
                    actions.push((1, Action::AttackMove));
                    let stances = rules.stances.values().filter(|s| {
                        s.family == StanceFamily::Combat
                            && STANCE_BEHAVIORS.contains(&s.behavior.as_str())
                    });
                    let mut ordered: Vec<_> = stances.collect();
                    ordered.sort_by_key(|s| {
                        (
                            STANCE_BEHAVIORS
                                .iter()
                                .position(|b| *b == s.behavior)
                                .unwrap_or(usize::MAX),
                            s.id,
                        )
                    });
                    for (n, stance) in ordered.into_iter().enumerate() {
                        actions.push((10 + n, Action::Stance(stance.id)));
                    }
                }
            }
        }
        for (slot, action) in &actions {
            palette
                .buttons
                .push(self.button(world, rules, *slot, *action, &own));
        }
        Some((palette, actions))
    }

    fn button(
        &self,
        world: &World,
        rules: &Rules,
        slot: usize,
        action: Action,
        own: &[&Entity],
    ) -> HudButton {
        let player = world
            .players()
            .find(|p| p.id == PLAYER)
            .expect("the player exists");
        let faction = player.faction;
        let mut button = HudButton {
            slot,
            icon: String::new(),
            model: None,
            mark: rom_render::IconMark::None,
            name: String::new(),
            price: String::new(),
            state: ButtonState::Ready,
            armed: false,
        };
        match action {
            Action::Construct(def) | Action::Train(def) | Action::Research(def) => {
                button.name = rules.display_name(def, faction).to_owned();
                let icon = crate::palette_icons::product_icon(
                    rules,
                    self.models.as_deref(),
                    faction,
                    player.age,
                    def,
                );
                (button.icon, button.model, button.mark) = (icon.art, icon.model, icon.mark);
                let price = world.price(PLAYER, def);
                button.price = price
                    .iter()
                    .filter(|(_, amount)| *amount > 0)
                    .map(|(resource, amount)| format!("{amount} {}", rules.def(*resource).name))
                    .collect::<Vec<_>>()
                    .join(", ");
                button.state = crate::hud::button_state(
                    world,
                    def,
                    self.pending_research
                        .get(&def)
                        .is_some_and(|tick| *tick == world.tick()),
                );
                button.armed = matches!(&self.placement, Some(p) if matches!(action, Action::Construct(d) if d == p.def));
            }
            Action::Stop => {
                button.name = "Stop".into();
                button.icon = "icons/cmd_stop".into();
            }
            Action::Descend => {
                button.name = "Descend: a city on open land".into();
                button.icon = "icons/cmd_rally".into();
                button.armed = self.descend_pick;
            }
            Action::BringLife => {
                button.name = "Bring life to a moon or asteroid".into();
                button.icon = "icons/life".into();
                button.armed = self.life_pick;
            }
            Action::Brush(def) => {
                button.name = rules.def(def).name.clone();
                button.icon = brush_icon(rules, def);
                button.armed = self.brush == Some(def);
            }
            Action::Release => {
                button.name = "Release all workers".into();
                button.icon = "icons/cmd_cancel".into();
            }
            Action::Buy(resource) | Action::Sell(resource) => {
                trade_button(
                    world,
                    &mut button,
                    resource,
                    matches!(action, Action::Buy(_)),
                    own,
                );
            }
            Action::Governor => {
                let running = own
                    .iter()
                    .any(|e| world.is_colony_type(e.type_id) && !world.governor_paused(e.id));
                (button.name, button.icon) = if running {
                    ("Pause governor".into(), "icons/cmd_hold".into())
                } else {
                    ("Resume governor".into(), "icons/cmd_guard".into())
                };
            }
            Action::Unload => {
                // A carrier in orbit launches its craft; anything else sets passengers down.
                let launching = own
                    .iter()
                    .any(|e| world.orbit(e.id).is_some() && world.launches_craft(e.id));
                button.name = if launching { "Launch craft" } else { "Unload" }.into();
                button.icon = "icons/cmd_cancel".into();
            }
            Action::Lift => {
                button.name = if own.iter().any(|e| climbs(world, e)) {
                    "Climb to orbit"
                } else {
                    "Lift off"
                }
                .into();
                button.icon = "icons/cmd_guard".into();
            }
            Action::Land => {
                button.name = "Land".into();
                button.icon = "icons/cmd_hold".into();
            }
            Action::Patrol => {
                button.name = "Patrol".into();
                button.icon = "icons/cmd_attack_move".into();
                button.armed = self.patrol;
            }
            Action::TradeRoute => {
                button.name = "Trade route".into();
                button.icon = "icons/cmd_rally".into();
                button.armed = self.trade_pick;
            }
            Action::Orbit => {
                button.name = "Orbit".into();
                button.icon = "icons/cmd_guard".into();
            }
            Action::Travel => {
                button.name = "Travel".into();
                button.icon = "icons/cmd_attack_move".into();
            }
            Action::AttackMove => {
                button.name = "Attack-move".into();
                button.icon = "icons/cmd_attack_move".into();
                button.armed = self.attack_move;
            }
            Action::Stance(id) => {
                button.name = rules.def(id).name.clone();
                button.icon = stance_icon(&rules.stances[&id].behavior).into();
                let armed: Vec<_> = own
                    .iter()
                    .filter(|e| {
                        let et = &rules.entity_types[&e.type_id];
                        et.movement.is_some() && !et.weapons.is_empty()
                    })
                    .collect();
                button.armed = !armed.is_empty() && armed.iter().all(|e| e.stance == id);
            }
        }
        button
    }

    fn press_slot(&mut self, world: &mut World, slot: usize, shift: bool) {
        let Some((palette, actions)) = self.palette(world) else {
            return;
        };
        let Some((_, action)) = actions.into_iter().find(|(s, _)| *s == slot) else {
            return;
        };
        if let Some(button) = palette.buttons.iter().find(|b| b.slot == slot)
            && !button.state.enabled()
        {
            let reason = button.state.reason();
            self.say(world, reason);
            return;
        }
        let rules = world.rules().clone();
        match action {
            Action::BringLife => {
                self.life_pick = !self.life_pick;
                if self.life_pick {
                    self.say(
                        world,
                        "Right-click a moon or asteroid in a star system to bring it to life",
                    );
                }
            }
            Action::Brush(def) => {
                self.brush = if self.brush == Some(def) {
                    None
                } else {
                    Some(def)
                };
                if self.brush.is_some() {
                    self.say(
                        world,
                        format!(
                            "{}: click the ground to paint, Escape to stop",
                            rules.def(def).name
                        ),
                    );
                }
            }
            Action::Descend => {
                self.descend_pick = !self.descend_pick;
                if self.descend_pick {
                    self.say(
                        world,
                        "Click open land of a habitable planet to descend there with a city",
                    );
                }
            }
            Action::Construct(def) => {
                let et = &rules.entity_types[&def];
                self.placement = Some(Placement {
                    def,
                    footprint: (et.footprint_w, et.footprint_h),
                    centre: None,
                    valid: None,
                    dirty: false,
                    checked: None,
                    checked_age: None,
                });
                self.attack_move = false;
            }
            Action::Train(def) | Action::Research(def) => {
                let (kind, count) = if matches!(action, Action::Train(_)) {
                    ("train", if shift { SHIFT_BATCH } else { 1 })
                } else {
                    ("research", 1)
                };
                let Some(type_id) = self.primary_producer(world) else {
                    return;
                };
                let mut executors: Vec<(usize, EntityId)> = self
                    .own_selected(world)
                    .iter()
                    .filter(|e| e.type_id == type_id && e.build_progress.is_none())
                    .map(|e| (e.orders().len(), e.id))
                    .collect();
                for _ in 0..count {
                    executors.sort();
                    let Some((queued, id)) = executors.first().copied() else {
                        break;
                    };
                    if !self.submit(world, id, kind, Some(def), None, None, true) {
                        break;
                    }
                    executors[0] = (queued + 1, id);
                }
            }
            Action::Stop => {
                let ids: Vec<EntityId> = self.own_selected(world).iter().map(|e| e.id).collect();
                for id in ids {
                    self.submit(world, id, "stop", None, None, None, false);
                }
                self.attack_move = false;
            }
            Action::AttackMove => self.attack_move = !self.attack_move,
            Action::Patrol => {
                self.patrol = !self.patrol;
                self.attack_move = false;
                self.trade_pick = false;
            }
            Action::TradeRoute => {
                self.trade_pick = !self.trade_pick;
                self.patrol = false;
                self.attack_move = false;
            }
            Action::Land => {
                let ids: Vec<EntityId> = self
                    .own_selected(world)
                    .iter()
                    .filter(|e| lands(world, e))
                    .map(|e| e.id)
                    .collect();
                for id in ids {
                    let Some(below) = world.orbit(id).map(|o| o.pos) else {
                        continue;
                    };
                    // Straight down onto the point below, or a clear refusal.
                    let problem = (!world.is_craft_type(world.entity(id).expect("own").type_id))
                        .then(|| world.landing_problem(id, below))
                        .flatten();
                    match problem {
                        Some(reason) => self.say(world, format!("Cannot land: {reason}")),
                        None => {
                            self.submit(world, id, "land", None, None, Some(below), false);
                        }
                    }
                }
            }
            Action::Governor => {
                let colonies: Vec<(EntityId, bool)> = self
                    .own_selected(world)
                    .iter()
                    .filter(|e| world.is_colony_type(e.type_id))
                    .map(|e| (e.id, world.governor_paused(e.id)))
                    .collect();
                // Any running governor pauses them all; when all are paused they resume.
                let kind = if colonies.iter().any(|(_, paused)| !paused) {
                    "pause_governor"
                } else {
                    "resume_governor"
                };
                for (colony, _) in colonies {
                    self.submit(world, colony, kind, None, None, None, false);
                }
            }
            Action::Buy(resource) | Action::Sell(resource) => {
                let kind = if matches!(action, Action::Buy(_)) {
                    "buy"
                } else {
                    "sell"
                };
                let market = self
                    .own_selected(world)
                    .iter()
                    .find(|e| e.build_progress.is_none() && world.market_currency(e.id).is_some())
                    .map(|e| e.id);
                let Some(market) = market else {
                    self.say(world, "The market is not finished yet");
                    return;
                };
                for _ in 0..if shift { SHIFT_BATCH } else { 1 } {
                    if !self.submit(world, market, kind, Some(resource), None, None, false) {
                        break;
                    }
                }
            }
            Action::Release => {
                let sites: Vec<EntityId> = self
                    .own_selected(world)
                    .iter()
                    .filter(|e| !world.occupants(e.id).is_empty())
                    .map(|e| e.id)
                    .collect();
                for site in sites {
                    let workers = world.occupants(site);
                    if !self.submit(world, site, "release", None, None, None, false) {
                        continue;
                    }
                    // With a rally point they walk to it; without one they stand beside the door.
                    let rally = world.rally(site).and_then(|r| match r {
                        rom_sim::Rally::Point(pos) => Some(pos),
                        rom_sim::Rally::Entity(id) => world.entity(id).map(|e| e.pos),
                    });
                    if let Some(rally) = rally {
                        for worker in workers {
                            self.submit(world, worker, "move", None, None, Some(rally), false);
                        }
                    }
                }
            }
            Action::Orbit => self.orbit_nearest(world),
            Action::Travel => self.travel_on(world),
            Action::Unload => self.unload_selected(world),
            Action::Lift => {
                let ids: Vec<EntityId> = self
                    .own_selected(world)
                    .iter()
                    .filter(|e| world.is_landed(e.id) || climbs(world, e))
                    .map(|e| e.id)
                    .collect();
                for id in ids {
                    self.submit(world, id, "lift", None, None, None, false);
                }
            }
            Action::Stance(stance) => {
                let ids: Vec<EntityId> = self
                    .own_selected(world)
                    .iter()
                    .filter(|e| {
                        let et = &rules.entity_types[&e.type_id];
                        et.movement.is_some() && !et.weapons.is_empty()
                    })
                    .map(|e| e.id)
                    .collect();
                // An order like any other (task 85): a follower sees it in the journal.
                for id in ids {
                    self.submit(world, id, "stance", Some(stance), None, None, false);
                }
            }
        }
    }

    fn primary_producer(&self, world: &World) -> Option<DefId> {
        let rules = world.rules();
        self.own_selected(world)
            .iter()
            .find(|e| {
                e.build_progress.is_none() && rules.entity_types[&e.type_id].producer.is_some()
            })
            .map(|e| e.type_id)
    }

    /// Cancels queue row `index` of the single selected producer. The simulation cancels a
    /// whole queue at once (refunding each order), so the other entries are ordered again on
    /// the next tick.
    fn cancel_queued(&mut self, world: &mut World, index: usize) {
        let [id] = self.selection.as_slice() else {
            return;
        };
        let id = *id;
        let Some(entity) = self.mine(world, id) else {
            return;
        };
        let queued: Vec<&rom_sim::Order> = entity
            .orders()
            .iter()
            .filter(|o| matches!(o.kind.as_str(), "train" | "research"))
            .collect();
        if index >= queued.len() {
            return;
        }
        let keep: Vec<Command> = queued
            .iter()
            .enumerate()
            .filter(|(n, _)| *n != index)
            .map(|(_, o)| Command {
                player: PLAYER,
                executor: id,
                kind: o.kind.clone(),
                subject: o.subject,
                target_entity: None,
                target_pos: None,
                queue: true,
            })
            .collect();
        if self.submit(world, id, "stop", None, None, None, false) && !keep.is_empty() {
            self.requeue.push(Requeue {
                after_tick: world.tick(),
                commands: keep,
            });
        }
    }

    // ----- placement -----------------------------------------------------------------------

    fn hover(&mut self, world: &World, at: Pos) {
        let Some(p) = &mut self.placement else {
            return;
        };
        let centre = Pos {
            x: snap(at.x, p.footprint.0),
            y: snap(at.y, p.footprint.1),
            ..at
        };
        if p.centre != Some(centre) {
            p.centre = Some(centre);
            p.dirty = true;
        }
        self.refresh_placement(world);
    }

    /// Whether a selected builder of `def` stands on the body of `centre`, so it can walk there.
    fn builder_can_reach(&self, world: &World, def: DefId, centre: Pos) -> bool {
        let rules = world.rules();
        self.own_selected(world).iter().any(|e| {
            e.build_progress.is_none()
                && e.pos.body == centre.body
                && rules
                    .production
                    .get(&e.type_id)
                    .into_iter()
                    .flatten()
                    .any(|row| row.mode == ProductionMode::Construct && row.product_def_id == def)
        })
    }

    fn refresh_placement(&mut self, world: &World) {
        let reachable = self
            .placement
            .as_ref()
            .and_then(|p| p.centre.map(|c| self.builder_can_reach(world, p.def, c)));
        let Some(p) = &mut self.placement else {
            return;
        };
        let age = world.players().find(|p| p.id == PLAYER).map(|p| p.age);
        let age_changed = p.checked_age != age;
        let due = age_changed
            || p.checked
                .is_none_or(|at| world.tick() >= at + PLACEMENT_CHECK_TICKS);
        if let (true, true, Some(centre)) = (p.dirty || age_changed, due, p.centre) {
            let result = placement_check(world, p.def, centre).and_then(|()| {
                // A site no selected builder can walk to (another planet) is refused too.
                if reachable == Some(false) {
                    Err(SimError::OtherBody)
                } else {
                    Ok(())
                }
            });
            p.valid = Some(result.is_ok());
            p.dirty = false;
            p.checked = Some(world.tick());
            p.checked_age = age;
        }
    }

    fn place(&mut self, world: &mut World, at: Pos, shift: bool) {
        let Some(p) = self.placement.clone() else {
            return;
        };
        let centre = Pos {
            x: snap(at.x, p.footprint.0),
            y: snap(at.y, p.footprint.1),
            ..at
        };
        let rules = world.rules().clone();
        let mut builders: Vec<(bool, u64, EntityId)> = self
            .own_selected(world)
            .iter()
            .filter(|e| e.build_progress.is_none())
            .filter(|e| {
                rules
                    .production
                    .get(&e.type_id)
                    .into_iter()
                    .flatten()
                    .any(|row| row.mode == ProductionMode::Construct && row.product_def_id == p.def)
            })
            .map(|e| {
                (
                    e.pos.body == centre.body,
                    distance(world, e.pos, centre),
                    e.id,
                )
            })
            .collect();
        if builders.is_empty() {
            self.say(world, "No selected builder can build this");
            return;
        }
        // Builders on another body cannot walk to the site.
        builders.retain(|(same_body, _, _)| *same_body);
        let mut builders: Vec<(u64, EntityId)> =
            builders.into_iter().map(|(_, d, id)| (d, id)).collect();
        builders.sort();
        let Some(&(_, first)) = builders.first() else {
            self.say(world, rejection_message(world, &SimError::OtherBody, None));
            if let Some(active) = &mut self.placement {
                active.valid = Some(false);
            }
            return;
        };
        if !self.submit(
            world,
            first,
            "construct",
            Some(p.def),
            None,
            Some(centre),
            shift,
        ) {
            if let Some(active) = &mut self.placement {
                active.valid = Some(false);
            }
            return;
        }
        if builders.len() > 1 {
            self.helpers.push(Helpers {
                pos: centre,
                def: p.def,
                builders: builders.iter().skip(1).map(|(_, id)| *id).collect(),
                tries: 0,
            });
        }
        self.marker(world, centre, MarkerKind::Move);
        if shift {
            if let Some(active) = &mut self.placement {
                active.dirty = true;
                active.checked = None;
            }
        } else {
            self.placement = None;
        }
    }

    // ----- ticking -------------------------------------------------------------------------

    /// Housekeeping after each simulation step: drop dead entities, send queued helpers and
    /// re-queued orders, follow rally points, refresh the placement check.
    pub fn after_step(&mut self, world: &mut World) {
        self.update_wealth(world);
        let alive = |id: &EntityId| present(world, *id).is_some();
        self.selection.retain(|id| alive(id));
        for group in &mut self.groups {
            group.retain(|id| alive(id));
        }
        if self.foreign.is_some_and(|id| present(world, id).is_none()) {
            self.foreign = None;
        }
        let tick = world.tick();
        self.pending_research.retain(|_, at| *at == tick);
        self.markers
            .retain(|m| tick.saturating_sub(m.tick) <= MARKER_TICKS);
        let due: Vec<Requeue> = {
            let (due, later) = std::mem::take(&mut self.requeue)
                .into_iter()
                .partition(|r| tick > r.after_tick);
            self.requeue = later;
            due
        };
        for requeue in due {
            for command in requeue.commands {
                if world.submit(command).is_err() {
                    self.message = Some(("The order could not be restored".into(), tick));
                }
            }
        }
        let helpers = std::mem::take(&mut self.helpers);
        for mut job in helpers {
            let site = world
                .entities()
                .find(|e| {
                    e.owner == PLAYER
                        && e.type_id == job.def
                        && e.pos == job.pos
                        && e.build_progress.is_some()
                })
                .map(|e| e.id);
            match site {
                Some(site) => {
                    for builder in job.builders {
                        if world.entity(builder).is_some() {
                            self.submit(world, builder, "repair", None, Some(site), None, false);
                        }
                    }
                }
                None => {
                    job.tries += 1;
                    if job.tries < 6 {
                        self.helpers.push(job);
                    }
                }
            }
        }
        self.refresh_placement(world);
    }

    fn update_wealth(&mut self, world: &World) {
        let live: BTreeSet<_> = world
            .entities()
            .filter(|e| e.owner == PLAYER)
            .flat_map(|e| e.orders())
            .map(|o| o.id)
            .collect();
        self.blocked_notices.retain(|id| live.contains(id));
        for order in world
            .entities()
            .filter(|e| e.owner == PLAYER)
            .flat_map(|e| e.orders())
        {
            if matches!(&order.status, rom_sim::OrderStatus::Blocked(reason) if reason == "no keeper")
                && self.blocked_notices.insert(order.id)
            {
                self.say(world, "No temple with room for the relic");
            }
        }
        let mut next = BTreeMap::new();
        for route in world.trade_routes(PLAYER) {
            let Some((unit, home)) = world.entity(route.trader).zip(world.entity(route.home))
            else {
                continue;
            };
            let order = unit
                .orders()
                .iter()
                .find(|o| o.kind == "trade")
                .map(|o| o.id);
            let previous = self
                .routes
                .get(&route.trader)
                .filter(|r| r.home == route.home && r.target == route.target && r.order == order);
            let mut to_target = previous.is_some_and(|r| r.to_target);
            if unit
                .orders()
                .front()
                .is_some_and(|o| Some(o.id) == order && o.kind == "trade")
            {
                let destination = if to_target {
                    world.entity(route.target)
                } else {
                    Some(home)
                };
                if destination.is_some_and(|city| reached_city(world, unit.pos, city)) {
                    to_target = !to_target;
                }
            }
            next.insert(
                route.trader,
                RouteView {
                    home: route.home,
                    target: route.target,
                    order,
                    to_target,
                },
            );
        }
        for (trader, route) in &self.routes {
            if next.get(trader).is_none_or(|r| r.order != route.order)
                && world.entity(*trader).is_some_and(|e| e.owner == PLAYER)
                && [route.home, route.target].iter().any(|id| {
                    world.entity(*id).is_none_or(|e| {
                        Self::is_enemy(world, e)
                            || e.neutralised
                            || e.assimilation_until.is_some()
                            || e.build_progress.is_some()
                            || e.hp <= 0
                    })
                })
            {
                self.message = Some(("Trade route ended: city lost".into(), world.tick()));
            }
        }
        self.routes = next;
    }

    fn wealth_help(&self, world: &World) -> Option<String> {
        let rules = world.rules();
        let target = self.hovered.and_then(|id| world.entity(id))?;
        let et = &rules.entity_types[&target.type_id];
        let selected = self.own_selected(world);
        if let Some(node) = et
            .node
            .as_ref()
            .filter(|n| n.claim_mode == rom_rules::ClaimMode::Carry)
            && target.container_id.is_none()
            && selected.iter().any(|e| {
                node.claim_tag_id
                    .is_some_and(|tag| rules.has_tag(e.type_id, tag))
            })
        {
            let keeper = rules.entity_types.values().find(|t| t.keeper.is_some())?;
            let faction = world.players().find(|p| p.id == PLAYER)?.faction;
            return Some(format!(
                "Carry to a {}",
                rules.display_name(keeper.id, faction).to_lowercase()
            ));
        }
        let trader = selected
            .iter()
            .find(|e| runs_routes(world, e.type_id))
            .copied();
        if let Some(trader) = trader.filter(|_| is_city(world, target)) {
            let income: i64 = world
                .trade_routes(PLAYER)
                .filter(|r| {
                    selected.iter().any(|e| e.id == r.trader)
                        && (r.home == target.id || r.target == target.id)
                })
                .map(|r| r.per_trip_income)
                .sum();
            if income > 0 {
                return Some(format!("Trade route: +{income} per trip"));
            }
            return Some(match world.trade_offer(PLAYER, trader.id, target.id) {
                Ok(route) => format!(
                    "Trade route to {}: +{} per trip",
                    crate::hud::city_label(world, target.id),
                    route.per_trip_income
                ),
                Err(error) => format!(
                    "No trade route to {}: {}",
                    crate::hud::city_label(world, target.id),
                    Self::trade_refusal(world, trader, target, &error)
                ),
            });
        }
        None
    }

    // ----- snapshot ------------------------------------------------------------------------

    /// Adds the selection, palette, queue and overlay data to a HUD snapshot.
    pub fn fill(&self, world: &World, hud: &mut HudSnapshot) {
        let rules = world.rules();
        hud.data_tab = self.data_tab;
        hud.help = self.wealth_help(world).unwrap_or_default();
        if hud.help.is_empty() && self.trade_pick && !self.selected_traders(world).is_empty() {
            hud.help = self.trade_hint(world);
        }
        hud.trade_targets = self.trade_targets(world);
        let pending: Vec<_> = self
            .pending_research
            .iter()
            .filter(|(_, tick)| **tick == world.tick())
            .map(|(id, _)| *id)
            .collect();
        hud.overview = crate::hud::research_overview(world, &pending);
        let (faction, age) = world
            .players()
            .find(|p| p.id == PLAYER)
            .map_or((DefId(0), DefId(0)), |p| (p.faction, p.age));
        let own = self.own_selected(world);
        if hud.help.is_empty()
            && !own.is_empty()
            && own.iter().all(|e| runs_routes(world, e.type_id))
        {
            hud.help = if own.iter().any(|e| world.is_spacecraft_type(e.type_id)) {
                "Trade route, then a city on another planet; paid on each arrival"
            } else {
                "Trade route, then another city; paid on each arrival"
            }
            .into();
        }
        if hud.help.is_empty()
            && !self.view.in_space
            && own.iter().any(|e| {
                world
                    .orbit(e.id)
                    .is_some_and(|o| o.body == self.view.planet)
            })
        {
            hud.help = if own.iter().any(|e| lands(world, e)) {
                "Right-click to move within the orbit; Land comes straight down; troops board on the ground"
            } else {
                "Right-click to move within the orbit; Patrol circles the planet"
            }
            .into();
        }
        // A jump inhibitor says what it does to enemy ships (task 104), from its component.
        if hud.help.is_empty()
            && let Some(inhibitor) = own
                .iter()
                .find_map(|e| rules.entity_types[&e.type_id].inhibitor.as_ref())
        {
            hud.help = inhibitor_help(inhibitor);
        }
        // A market says how its prices work (task 107).
        if hud.help.is_empty()
            && !own.is_empty()
            && own.iter().all(|e| world.market_currency(e.id).is_some())
        {
            hud.help = crate::hud::market_help(world, PLAYER);
        }
        if hud.help.is_empty() && own.iter().any(|e| world.jump_charge(e.id).is_some()) {
            hud.help = "Charging a jump: it cannot move or fire until it leaves; \
                        any new order cancels the charge"
                .into();
        }
        if hud.help.is_empty()
            && own.iter().any(|e| {
                rules.entity_types[&e.type_id]
                    .producer
                    .as_ref()
                    .is_some_and(|p| p.has_rally)
            })
        {
            hud.help = "Right-click the ground or a resource site to set the rally point".into();
        }
        let mut selected: Vec<u32> = own.iter().map(|e| e.id.0).collect();
        selected.extend(
            self.foreign
                .filter(|id| present(world, *id).is_some())
                .map(|id| id.0),
        );
        selected.sort_unstable();
        hud.selected = selected;
        let card = |e: &Entity, foreign: bool| {
            let et = &rules.entity_types[&e.type_id];
            let attack = et.weapons.iter().map(|w| w.damage).max();
            let range = et.weapons.iter().map(|w| w.range_max).max();
            let workers = et
                .gather_site
                .as_ref()
                .map(|_| format!("{} / {}", world.occupants(e.id).len(), e.gather_slots));
            HudCard {
                icon: icon_of(et).to_owned(),
                name: rules.display_name(e.type_id, faction).to_owned(),
                foreign,
                hp: (e.hp, e.max_hp),
                attack: attack.map(|a| a.to_string()),
                armour: (et.armor > 0).then(|| et.armor.to_string()),
                range: range.map(|r| {
                    if r == 0 {
                        "Melee".to_owned()
                    } else {
                        format!("{}", r / 1000)
                    }
                }),
                garrison: et.container.as_ref().map(|c| {
                    if world.is_carrier_type(e.type_id) {
                        format!(
                            "{} / {}",
                            world.carried_load(e.id),
                            world.carry_capacity(e.id)
                        )
                    } else {
                        format!("max {}", c.capacity)
                    }
                }),
                workers,
                inside: if foreign {
                    Vec::new()
                } else {
                    let mut inside = world.occupants(e.id);
                    if inside.is_empty() {
                        inside = world.passengers(e.id);
                    }
                    inside
                        .into_iter()
                        .take(12)
                        .filter_map(|id| world.entity(id))
                        .map(|w| {
                            let wet = &rules.entity_types[&w.type_id];
                            HudTile {
                                icon: icon_of(wet).to_owned(),
                                name: rules.display_name(w.type_id, faction).to_owned(),
                                count: 1,
                                hp_permille: (w.hp.max(0) * 1000 / w.max_hp.max(1)).min(1000)
                                    as u32,
                            }
                        })
                        .collect()
                },
                wealth: {
                    let mut lines = crate::hud::wealth_card(world, e);
                    // A caravan's next leg: the city it walks to (a freighter's comes from its
                    // legs, in the card itself).
                    if let Some(view) = self
                        .routes
                        .get(&e.id)
                        .filter(|_| !foreign && !world.is_spacecraft_type(e.type_id))
                    {
                        let city = if view.to_target {
                            view.target
                        } else {
                            view.home
                        };
                        lines.push(format!(
                            "Next: walk to {}",
                            crate::hud::city_label(world, city)
                        ));
                    }
                    lines.extend(crate::space_view::holding_line(world, e));
                    if !foreign {
                        lines.extend(crate::hud::colony_lines(world, e));
                        lines.extend(crate::hud::market_lines(world, e));
                    }
                    if !foreign {
                        lines.extend(crate::space_view::charge_line(world, e));
                    }
                    lines
                },
                progress_pct: e.build_progress.map(|done| {
                    let sources = world
                        .players()
                        .find(|p| p.id == e.owner)
                        .map(|p| p.active_sources())
                        .unwrap_or_default();
                    let total = rules.build_ticks(e.type_id, &sources).max(1) as u64;
                    (u64::from(done) * 100 / total).min(100) as u32
                }),
            }
        };
        hud.selection = match own.as_slice() {
            [] => self
                .foreign
                .and_then(|id| present(world, id))
                .map_or(HudSelection::None, |e| {
                    HudSelection::Single(Box::new(card(e, true)))
                }),
            [one] => HudSelection::Single(Box::new(card(one, false))),
            many => {
                let mut tiles = Vec::new();
                for type_id in self.tile_types(world) {
                    let of_type: Vec<&&Entity> =
                        many.iter().filter(|e| e.type_id == type_id).collect();
                    let (hp, max) = of_type
                        .iter()
                        .fold((0, 0), |(h, m), e| (h + e.hp, m + e.max_hp));
                    let et = &rules.entity_types[&type_id];
                    tiles.push(HudTile {
                        icon: icon_of(et).to_owned(),
                        name: rules.display_name(type_id, faction).to_owned(),
                        count: of_type.len(),
                        hp_permille: (hp.max(0) * 1000 / max.max(1)).min(1000) as u32,
                    });
                }
                HudSelection::Multiple {
                    total: many.len(),
                    tiles,
                }
            }
        };
        hud.palette = self.palette(world).map(|(palette, _)| palette);
        hud.queue = match own.as_slice() {
            [one] if rules.entity_types[&one.type_id].producer.is_some() => {
                let rows: Vec<HudQueueItem> = one
                    .orders()
                    .iter()
                    .filter(|o| matches!(o.kind.as_str(), "train" | "research"))
                    .filter_map(|o| {
                        let def = o.subject?;
                        let total = o.total_ticks.unwrap_or(0);
                        let icon = crate::palette_icons::product_icon(
                            rules,
                            self.models.as_deref(),
                            faction,
                            age,
                            def,
                        );
                        Some(HudQueueItem {
                            icon: icon.art,
                            model: icon.model,
                            mark: icon.mark,
                            name: rules.display_name(def, faction).to_owned(),
                            progress_pct: (o.progress_ticks * 100)
                                .checked_div(total)
                                .map_or(0, |p| p.min(100) as u32),
                            blocked: match &o.status {
                                rom_sim::OrderStatus::Blocked(reason) => {
                                    Some(blocked_reason(reason))
                                }
                                _ => None,
                            },
                        })
                    })
                    .collect();
                Some(HudQueue {
                    more: rows.len().saturating_sub(3),
                    items: rows.into_iter().take(3).collect(),
                })
            }
            _ => None,
        };
        hud.trade_routes = world
            .trade_routes(PLAYER)
            .filter(|r| self.selection.contains(&r.trader))
            .filter_map(|r| {
                let (home, target) = world.entity(r.home).zip(world.entity(r.target))?;
                if home.pos.body != self.view.planet || target.pos.body != self.view.planet {
                    return None;
                }
                let pos = |p: Pos| SurfacePos {
                    face: p.face,
                    x: i64::from(p.x),
                    y: i64::from(p.y),
                };
                Some(rom_render::HudTradeRoute {
                    home: pos(home.pos),
                    target: pos(target.pos),
                    to_target: self.routes.get(&r.trader).is_some_and(|v| v.to_target),
                    length: r.length,
                })
            })
            .collect();
        hud.patrols = self.patrols(world);
        let tick = world.tick();
        self.bombardment(world, hud);
        hud.markers = self
            .markers
            .iter()
            .map(|m| HudMarker {
                face: m.pos.face,
                x: m.pos.x,
                y: m.pos.y,
                kind: m.kind,
                age_ticks: tick.saturating_sub(m.tick).min(MARKER_TICKS) as u32,
                from: None,
            })
            .collect();
        for (producer, rally) in world.rallies() {
            if !self.selection.contains(&producer) {
                continue;
            }
            let (Some(home), Some(at)) = (
                world.entity(producer).map(|e| e.pos),
                match rally {
                    rom_sim::Rally::Point(pos) => Some(pos),
                    rom_sim::Rally::Entity(id) => world.entity(id).map(|e| e.pos),
                },
            ) else {
                continue;
            };
            hud.markers.push(HudMarker {
                face: at.face,
                x: at.x,
                y: at.y,
                kind: MarkerKind::Rally,
                age_ticks: 0,
                from: Some((home.face, home.x, home.y)),
            });
        }
        if let Some(age) = self
            .placement
            .as_ref()
            .and_then(|p| world.placement_discovery_age(PLAYER, p.def))
        {
            hud.help = rejection_message(world, &SimError::ResourceAgeRequired(age), None);
        } else if let Some(p) = self.placement.as_ref().filter(|p| p.valid == Some(false))
            && let Some(centre) = p.centre
        {
            // Say why the red footprint is red.
            let reason = placement_check(world, p.def, centre).err().or_else(|| {
                (!self.builder_can_reach(world, p.def, centre)).then_some(SimError::OtherBody)
            });
            if let Some(reason) = reason {
                hud.help = rejection_message(world, &reason, Some(p.def));
            }
        }
        hud.placement = self.placement.as_ref().and_then(|p| {
            p.centre.map(|c| HudPlacement {
                face: c.face,
                width: p.footprint.0.clamp(1, 255) as u8,
                height: p.footprint.1.clamp(1, 255) as u8,
                x: c.x,
                y: c.y,
                kind: u16::try_from(p.def.0).unwrap_or(u16::MAX),
                owner: PLAYER.0 as u8,
                valid: p.valid,
            })
        });
        let city_selected = own.iter().any(|e| {
            rules.entity_types[&e.type_id]
                .territory
                .as_ref()
                .is_some_and(|t| t.is_city)
        });
        let placing_city = self.placement.as_ref().is_some_and(|p| {
            rules.entity_types[&p.def]
                .territory
                .as_ref()
                .is_some_and(|t| t.is_city)
        });
        let needs_radius = self.placement.as_ref().is_some_and(|p| {
            rules.entity_types[&p.def]
                .placement_rules
                .iter()
                .any(|r| r.rule == rom_rules::PlacementKind::InCityRadius)
        });
        hud.city_radii = world
            .entities()
            .filter(|e| e.pos.body == self.view.planet)
            .filter_map(|e| {
                let territory = rules.entity_types[&e.type_id].territory.as_ref()?;
                if !territory.is_city {
                    return None;
                }
                let economic = if (city_selected || needs_radius)
                    && e.owner == PLAYER
                    && e.build_progress.is_none()
                    && !e.neutralised
                {
                    world.city_economic_radius(e.id)
                } else {
                    0
                };
                let spacing = if placing_city {
                    rules.constant("city.spacing").unwrap_or(0)
                } else {
                    0
                };
                (economic > 0 || spacing > 0).then_some(HudCityRadius {
                    face: e.pos.face,
                    x: e.pos.x,
                    y: e.pos.y,
                    economic,
                    spacing,
                    minimap: city_selected && economic > 0,
                })
            })
            .collect();
        if let Some((text, at)) = &self.message
            && tick.saturating_sub(*at) <= MESSAGE_TICKS
        {
            hud.events.push(text.clone());
            hud.event_about = self.message_about.map(|id| id.0);
        }
    }
}

/// What a jump inhibitor does, from its `et_inhibitor` row (task 104).
/// A market button (task 107): the resource's picture, the trade and its price now, and why
/// it cannot be pressed (no research, not enough to pay or to sell).
fn trade_button(
    world: &World,
    button: &mut HudButton,
    resource: DefId,
    buying: bool,
    own: &[&Entity],
) {
    let rules = world.rules();
    button.icon = crate::hud::resource_icon(rules, resource);
    let name = &rules.def(resource).name;
    let currency = own.iter().find_map(|e| world.market_currency(e.id));
    let (Some(currency), Some(quote)) = (currency, world.market_quote(PLAYER, resource)) else {
        button.name = name.clone();
        return;
    };
    let money = &rules.def(currency).name;
    let (verb, price) = if buying {
        ("Buy", quote.buy)
    } else {
        ("Sell", quote.sell)
    };
    button.name = format!("{verb} {} {name}", quote.lot);
    button.price = if buying {
        format!("Costs {price} {money}")
    } else {
        format!("Pays {price} {money}")
    };
    let shown = u32::try_from(price.max(0)).unwrap_or(u32::MAX);
    button.mark = if buying {
        rom_render::IconMark::Buy(shown)
    } else {
        rom_render::IconMark::Sell(shown)
    };
    let (paid, need) = if buying {
        (currency, quote.buy)
    } else {
        (resource, quote.lot)
    };
    let have = world
        .players()
        .find(|p| p.id == PLAYER)
        .and_then(|p| p.resources.get(&paid).copied())
        .unwrap_or(0);
    button.state = if !world.can_trade(PLAYER) {
        ButtonState::Locked {
            reason: format!(
                "Needs {}",
                crate::hud::rule_tech_name(world, rom_sim::RULE_MARKET_TRADING)
            ),
        }
    } else if own.iter().all(|e| e.build_progress.is_some()) {
        ButtonState::Locked {
            reason: "The market is not finished yet".into(),
        }
    } else if have < need {
        ButtonState::Unaffordable {
            missing: vec![(crate::hud::resource_icon(rules, paid), need - have)],
        }
    } else {
        ButtonState::Ready
    };
}

fn inhibitor_help(inhibitor: &rom_rules::Inhibitor) -> String {
    let mut help = format!(
        "Enemy ships in this star system may leave only the way they came, charging {} % longer",
        inhibitor.charge_pct.max(0)
    );
    if inhibitor.arrival_damage > 0 {
        help.push_str(&format!(
            "; each that jumps in is struck for {} hit points (craft aboard a carrier are \
             sheltered)",
            inhibitor.arrival_damage
        ));
    }
    help
}

/// Translate rejected placement and research orders into the rule the player can fix.
fn rejection_message(world: &World, error: &SimError, subject: Option<DefId>) -> String {
    match error {
        SimError::InvalidCarryTarget => "That relic is no longer available".into(),
        SimError::CannotCarry => "This unit cannot carry that".into(),
        SimError::CarrierFull => "Already carrying".into(),
        SimError::NoHomeCity => "No home city on this world".into(),
        SimError::RouteTooShort => format!(
            "Too close for a trade route ({} tiles)",
            world.rules().constant("trade.min_route").unwrap_or(0) / 1000
        ),
        SimError::NotATradePartner => "Not a trade partner".into(),
        SimError::CaravanLimit => format!(
            "Trade route limit reached ({})",
            crate::hud::caravan_limit(world, PLAYER)
        ),
        SimError::CitiesTooClose => format!(
            "Too close to another city ({} tiles)",
            world.rules().constant("city.spacing").unwrap_or(0) / 1000
        ),
        SimError::ResourceAgeRequired(age) => {
            format!("Deposit revealed in {}", world.rules().def(*age).name)
        }
        SimError::NotInOwnTerritory => "Outside your borders".into(),
        SimError::CannotDescend => "Choose open land of a habitable planet".into(),
        SimError::NoDormantSurface => {
            "Only a moon or asteroid not yet brought to life can be brought to life".into()
        }
        SimError::NotYourLiving => "Only a planet you brought to life can be painted".into(),
        SimError::Settled => "A city stands on this planet: it can no longer be painted".into(),
        SimError::OrdersClosed if world.is_ascended(PLAYER) => {
            "Ascended: you command nothing on the ground until you descend".into()
        }
        SimError::InForeignTerritory => "Another nation's territory".into(),
        SimError::OtherBody => "No selected builder can reach this site (another planet)".into(),
        SimError::NotInCityRadius => "Outside a city's economic radius".into(),
        SimError::CannotBoard => "Cannot board that ship".into(),
        SimError::ShipNotLanded => {
            "The ship must land first: troops board and leave it only on the ground".into()
        }
        SimError::BoundToOrbit => "A satellite moves only within its orbit".into(),
        SimError::HeldByInhibitor => {
            "An enemy jump inhibitor holds this system: leave the way you came, or destroy it"
                .into()
        }
        SimError::NeedsMarketTrading => format!(
            "Trading at the market needs {}",
            crate::hud::rule_tech_name(world, rom_sim::RULE_MARKET_TRADING)
        ),
        SimError::NotTradeable => "That resource is not traded at the market".into(),
        SimError::InsufficientResources
            if subject.is_some_and(|r| world.rules().resources.contains_key(&r)) =>
        {
            "Not enough to trade".into()
        }
        SimError::NeedsInterstellarTrade => format!(
            "Trade with another star system needs {} (spaceport research)",
            crate::hud::rule_tech_name(world, rom_sim::RULE_INTERSTELLAR_TRADE)
        ),
        SimError::NoRoomAboard => "No room aboard".into(),
        SimError::Aboard => "Aboard a ship in space: land it first".into(),
        SimError::CannotLand => "Cannot land there: open ground only, outside enemy cities".into(),
        SimError::NotLanded => "The ship has not landed".into(),
        SimError::CannotClimb => "Orbital craft climb from a planet's air to its orbit".into(),
        SimError::CraftInOrbit => {
            "Orbital craft hold their orbit: attack, board a carrier or come down".into()
        }
        SimError::CannotUnload => "Nobody to set down here".into(),
        SimError::CityLimit => "City limit reached".into(),
        SimError::Locked => subject.map_or("Needs a prerequisite".into(), |id| {
            format!("Needs {}", crate::hud::unmet(world, id))
        }),
        SimError::Unavailable => "Unavailable to your faction".into(),
        SimError::AlreadyResearched => "Already researched".into(),
        SimError::ResearchQueued => "Research already queued".into(),
        SimError::ExclusiveResearch => "Another research choice excludes this technology".into(),
        SimError::PlacementOverlap => "Another building occupies this footprint".into(),
        SimError::UnbuildableTerrain => "Cannot build on this terrain".into(),
        SimError::OutsideMap | SimError::CrossesFaceEdge => {
            "Footprint extends beyond the buildable surface".into()
        }
        SimError::NotOnTerrain
        | SimError::NotAdjacentTerrain
        | SimError::NotNearType
        | SimError::TooCloseToType
        | SimError::TooCloseToEnemyCity
        | SimError::NotOnNode
        | SimError::NotOnLayer => {
            let rules = world.rules();
            let kind = match error {
                SimError::NotOnTerrain => rom_rules::PlacementKind::OnTerrain,
                SimError::NotAdjacentTerrain => rom_rules::PlacementKind::AdjacentTerrain,
                SimError::NotNearType => rom_rules::PlacementKind::NearType,
                SimError::TooCloseToType => rom_rules::PlacementKind::MinDistanceType,
                SimError::TooCloseToEnemyCity => rom_rules::PlacementKind::MinDistanceEnemyCity,
                SimError::NotOnNode => rom_rules::PlacementKind::OnNode,
                _ => rom_rules::PlacementKind::OnLayer,
            };
            let rule = subject
                .and_then(|id| rules.entity_types.get(&id))
                .and_then(|t| t.placement_rules.iter().find(|r| r.rule == kind));
            let target = rule
                .and_then(|r| r.ref_def_id)
                .map(|id| rules.def(id).name.as_str())
                .unwrap_or("the required terrain or layer");
            match kind {
                rom_rules::PlacementKind::AdjacentTerrain => format!("Must touch {target}"),
                rom_rules::PlacementKind::NearType => format!("Must be near {target}"),
                rom_rules::PlacementKind::MinDistanceType => format!("Too close to {target}"),
                rom_rules::PlacementKind::MinDistanceEnemyCity => {
                    "Too close to an enemy city".into()
                }
                _ => format!("Must be on {target}"),
            }
        }
        _ => error.to_string(),
    }
}

#[cfg(test)]
mod tests;
