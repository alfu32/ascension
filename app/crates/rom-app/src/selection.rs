//! The simulation thread's side of interaction: the selection, control groups, the command
//! palette and build placement. It turns pick events from the render thread into commands for
//! player one, and describes the result as plain data for the HUD snapshot.
//!
//! Behaviour comes from components and production rows, never from definition keys.

use std::collections::BTreeMap;

use rom_render::{
    ButtonState, HudButton, HudCard, HudCityRadius, HudMarker, HudPalette, HudPlacement, HudQueue,
    HudQueueItem, HudRegionKind, HudSelection, HudSnapshot, HudTile, MarkerKind, Modifiers,
    PaletteTab, PickEvent, PointerButton, SurfacePos, slot_of_key,
};
use rom_rules::{DefId, EntityClass, EntityType, ProductionMode, Rules, StanceFamily};
use rom_sim::{BodyId, Command, Entity, EntityId, PlayerId, Pos, SimError, Topology, World};

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
/// Widest distance, in milli-tiles, between a new unit and the producer whose rally it follows.
const RALLY_REACH: i64 = 5000;
/// Combat stance behaviours offered as buttons, in this order.
const STANCE_BEHAVIORS: [&str; 4] = ["aggressive", "defensive", "stand_ground", "hold_fire"];

/// A named palette page and its (slot, product) entries.
type PageRows = (String, Vec<(usize, DefId)>);

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

/// Selection, groups, palette state and placement of player one.
#[derive(Debug, Default)]
pub struct Controller {
    selection: Vec<EntityId>,
    foreign: Option<EntityId>,
    groups: [Vec<EntityId>; 10],
    last_group: Option<(usize, u64)>,
    tab: PaletteTab,
    page: usize,
    data_tab: usize,
    pending_research: BTreeMap<DefId, u64>,
    attack_move: bool,
    placement: Option<Placement>,
    rally: BTreeMap<EntityId, Pos>,
    markers: Vec<Marker>,
    helpers: Vec<Helpers>,
    requeue: Vec<Requeue>,
    message: Option<(String, u64)>,
    focus: Option<Pos>,
    ordered: bool,
    seen_next_id: u32,
    view: ViewCtx,
}

/// Distance in milli-tiles between two positions; positions beyond reach (or on faces that do
/// not touch) rank last.
fn distance(world: &World, a: Pos, b: Pos) -> u64 {
    world.distance(a, b).unwrap_or(u64::MAX)
}

/// Whether an entity stands in the space of a system (on a star's plane).
fn in_space(world: &World, entity: &Entity) -> bool {
    world
        .body(entity.pos.body)
        .is_some_and(|b| b.kind == rom_rules::BodyKind::Star)
}

fn snap(coordinate: i32, size: i64) -> i32 {
    ((i64::from(coordinate) - size * 500 + 500).div_euclid(1000) * 1000 + size * 500) as i32
}

fn icon_of(et: &EntityType) -> &'static str {
    match (et.class, et.ui_group.as_str()) {
        (EntityClass::Building, "wonder") => "icons/pic_wonder",
        (EntityClass::Building, "military") => "icons/pic_tower",
        (EntityClass::Building, _) => "icons/pic_house",
        (EntityClass::Unit, "military" | "foot" | "mounted") => "icons/pic_shield",
        _ => "icons/pic_person",
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
            .filter(|e| e.owner == PLAYER && in_space(world, e))
            .filter(|e| world.rules().entity_types[&e.type_id].movement.is_some())
            .map(|e| e.id)
            .collect();
        self.set_selection(ships);
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
    }

    fn mine<'w>(&self, world: &'w World, id: EntityId) -> Option<&'w Entity> {
        world.entity(id).filter(|e| e.owner == PLAYER)
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
        self.page = 0;
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
            PickEvent::Hover { surface, .. } => {
                let at = self.pos_of(surface);
                self.hover(world, at);
            }
            PickEvent::Hotkey { ch, modifiers } => self.hotkey(world, ch, modifiers),
            PickEvent::Escape => {
                self.placement = None;
                self.attack_move = false;
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
        if self.attack_move {
            let target = at;
            let ids: Vec<EntityId> = self.own_selected(world).iter().map(|e| e.id).collect();
            for id in ids {
                self.submit(
                    world,
                    id,
                    "attack_move",
                    None,
                    None,
                    Some(target),
                    mods.shift,
                );
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
        let Some(clicked) = world.entity(id) else {
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
            Enemy(EntityId),
            Site(EntityId),
            Gather(EntityId, DefId),
        }
        let kind = entity
            .map(EntityId)
            .and_then(|id| world.entity(id))
            .map_or(Kind::Ground, |e| {
                let et = &rules.entity_types[&e.type_id];
                if Self::is_enemy(world, e) {
                    Kind::Enemy(e.id)
                } else if e.owner == PLAYER && e.build_progress.is_some() {
                    Kind::Site(e.id)
                } else if let (PLAYER, Some(site)) = (e.owner, et.gather_site.as_ref()) {
                    Kind::Gather(e.id, site.resource_id)
                } else {
                    Kind::Ground
                }
            });
        let selected: Vec<EntityId> = self.own_selected(world).iter().map(|e| e.id).collect();
        let mut issued = false;
        for id in selected {
            let Some(e) = world.entity(id) else {
                continue;
            };
            let et = &rules.entity_types[&e.type_id];
            let can_move = et.movement.is_some();
            let armed = !et.weapons.is_empty();
            let queue = mods.shift;
            match &kind {
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
                    let kind = if self.attack_move && armed {
                        "attack_move"
                    } else {
                        "move"
                    };
                    issued |= self.submit(world, id, kind, None, None, Some(target), queue);
                }
                _ => {
                    if et.producer.as_ref().is_some_and(|p| p.has_rally) {
                        self.rally.insert(id, target);
                        self.ordered = true;
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
        }
    }

    /// Own ships selected, in space and able to move.
    fn selected_ships(&self, world: &World) -> Vec<EntityId> {
        let rules = world.rules();
        self.own_selected(world)
            .iter()
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

    /// A right click in space: attack a ship, orbit a body, travel to a system or move.
    fn space_right_click(
        &mut self,
        world: &mut World,
        surface: SurfacePos,
        entity: Option<u32>,
        body: Option<u32>,
        system: Option<u32>,
        mods: Modifiers,
    ) {
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
            if let (Some(foe), true) = (foe, armed) {
                self.submit(world, id, "attack", None, Some(foe), None, mods.shift);
            } else if let Some(body) = body {
                self.order_orbit(world, id, BodyId(body), mods.shift);
            } else if let Some(system) = system {
                self.order_travel(world, id, system, mods.shift);
            } else if let Some(at) = point {
                self.submit(world, id, "move", None, None, Some(at), mods.shift);
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
                    .and_then(|e| world.surface(e.pos.body))
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
                        .map(|e| e.pos),
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
                    self.page = 0;
                }
            }
            HudRegionKind::PaletteArrow if button == PointerButton::Left => {
                if let Some((palette, _)) = self.palette(world)
                    && palette.pages > 1
                {
                    self.page = if index == 0 {
                        (palette.page + palette.pages - 1) % palette.pages
                    } else {
                        (palette.page + 1) % palette.pages
                    };
                }
            }
            HudRegionKind::CommandSlot if button == PointerButton::Left => {
                self.press_slot(world, index, mods.shift);
            }
            HudRegionKind::QueueItem if button == PointerButton::Right => {
                self.cancel_queued(world, index);
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
                if mode == ProductionMode::Research && !crate::hud::next_research(world, def) {
                    continue;
                }
                let page = if mode == ProductionMode::Research {
                    String::new()
                } else {
                    row.ui_page.clone()
                };
                by_page.entry(page).or_default().push(row);
            }
            let mut pages = Vec::new();
            for (name, mut rows) in by_page {
                rows.sort_by_key(|r| (r.ui_row, r.ui_col, r.product_def_id));
                if mode == ProductionMode::Research {
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
        let tabs = [
            !build_pages.is_empty(),
            can_move,
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
        let production = |pages: &[PageRows],
                          make: fn(DefId) -> Action|
         -> (usize, usize, String, Vec<(usize, Action)>) {
            let count = pages.len().max(1);
            let page = self.page.min(count - 1);
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
                let (pages, make): (&[PageRows], fn(DefId) -> Action) = match tab {
                    PaletteTab::Build => (&build_pages, Action::Construct),
                    PaletteTab::Train => (&train_pages, Action::Train),
                    _ => (&research_pages, Action::Research),
                };
                let (count, page, name, entries) = production(pages, make);
                palette.pages = count;
                palette.page = page;
                palette.page_name = name;
                actions = entries;
            }
            PaletteTab::Order => {
                actions.push((0, Action::Stop));
                if own.iter().any(|e| {
                    in_space(world, e) && rules.entity_types[&e.type_id].movement.is_some()
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
            name: String::new(),
            price: String::new(),
            state: ButtonState::Ready,
            armed: false,
        };
        match action {
            Action::Construct(def) | Action::Train(def) | Action::Research(def) => {
                button.name = rules.display_name(def, faction).to_owned();
                button.icon = rules
                    .entity_type(def)
                    .map_or("icons/pic_scroll", icon_of)
                    .to_owned();
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
            Action::Construct(def) => {
                let et = &rules.entity_types[&def];
                self.placement = Some(Placement {
                    def,
                    footprint: (et.footprint_w, et.footprint_h),
                    centre: None,
                    valid: None,
                    dirty: false,
                    checked: None,
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
            Action::Orbit => self.orbit_nearest(world),
            Action::Travel => self.travel_on(world),
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
                for id in ids {
                    if let Err(error) = world.set_stance(PLAYER, id, stance) {
                        self.say(world, error.to_string());
                    }
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

    fn refresh_placement(&mut self, world: &World) {
        let Some(p) = &mut self.placement else {
            return;
        };
        let due = p
            .checked
            .is_none_or(|at| world.tick() >= at + PLACEMENT_CHECK_TICKS);
        if let (true, true, Some(centre)) = (p.dirty, due, p.centre) {
            p.valid = Some(placement_check(world, p.def, centre).is_ok());
            p.dirty = false;
            p.checked = Some(world.tick());
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
        let mut builders: Vec<(u64, EntityId)> = self
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
            .map(|e| (distance(world, e.pos, centre), e.id))
            .collect();
        builders.sort();
        let Some(&(_, first)) = builders.first() else {
            self.say(world, "No selected builder can build this");
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
        let alive = |id: &EntityId| world.entity(*id).is_some();
        self.selection.retain(|id| alive(id));
        for group in &mut self.groups {
            group.retain(|id| world.entity(*id).is_some());
        }
        if self.foreign.is_some_and(|id| world.entity(id).is_none()) {
            self.foreign = None;
        }
        self.rally.retain(|id, _| world.entity(*id).is_some());
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
        self.follow_rally(world);
        self.refresh_placement(world);
    }

    /// Sends units that just left a producer with a rally point to that point.
    fn follow_rally(&mut self, world: &mut World) {
        let first_new = self.seen_next_id;
        self.seen_next_id = world.next_entity_id().0;
        if self.rally.is_empty() || first_new == 0 {
            return;
        }
        let rules = world.rules().clone();
        let fresh: Vec<(EntityId, Pos)> = world
            .entities()
            .filter(|e| e.id.0 >= first_new && e.owner == PLAYER)
            .filter(|e| rules.entity_types[&e.type_id].movement.is_some())
            .map(|e| (e.id, e.pos))
            .collect();
        for (id, at) in fresh {
            let nearest = self
                .rally
                .iter()
                .filter_map(|(producer, rally)| {
                    let p = world.entity(*producer)?;
                    let d = distance(world, p.pos, at);
                    (d <= RALLY_REACH as u64).then_some((d, *rally))
                })
                .min_by_key(|(d, _)| *d);
            if let Some((_, rally)) = nearest {
                self.submit(world, id, "move", None, None, Some(rally), false);
            }
        }
    }

    // ----- snapshot ------------------------------------------------------------------------

    /// Adds the selection, palette, queue and overlay data to a HUD snapshot.
    pub fn fill(&self, world: &World, hud: &mut HudSnapshot) {
        let rules = world.rules();
        hud.data_tab = self.data_tab;
        let pending: Vec<_> = self
            .pending_research
            .iter()
            .filter(|(_, tick)| **tick == world.tick())
            .map(|(id, _)| *id)
            .collect();
        hud.overview = crate::hud::research_overview(world, &pending);
        let faction = world
            .players()
            .find(|p| p.id == PLAYER)
            .map_or(DefId(0), |p| p.faction);
        let own = self.own_selected(world);
        let mut selected: Vec<u32> = own.iter().map(|e| e.id.0).collect();
        selected.extend(
            self.foreign
                .filter(|id| world.entity(*id).is_some())
                .map(|id| id.0),
        );
        selected.sort_unstable();
        hud.selected = selected;
        let card = |e: &Entity, foreign: bool| {
            let et = &rules.entity_types[&e.type_id];
            let attack = et.weapons.iter().map(|w| w.damage).max();
            let range = et.weapons.iter().map(|w| w.range_max).max();
            let workers = et.gather_site.as_ref().map(|_| {
                let busy = world
                    .entities()
                    .filter(|w| w.work_site == Some(e.id))
                    .count();
                format!("{busy} / {}", e.gather_slots)
            });
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
                garrison: et.container.as_ref().map(|c| format!("max {}", c.capacity)),
                workers,
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
                .and_then(|id| world.entity(id))
                .map_or(HudSelection::None, |e| HudSelection::Single(card(e, true))),
            [one] => HudSelection::Single(card(one, false)),
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
                        Some(HudQueueItem {
                            icon: rules
                                .entity_type(def)
                                .map_or("icons/pic_scroll", icon_of)
                                .to_owned(),
                            name: rules.display_name(def, faction).to_owned(),
                            progress_pct: (o.progress_ticks * 100)
                                .checked_div(total)
                                .map_or(0, |p| p.min(100) as u32),
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
        let tick = world.tick();
        hud.markers = self
            .markers
            .iter()
            .map(|m| HudMarker {
                face: m.pos.face,
                x: m.pos.x,
                y: m.pos.y,
                kind: m.kind,
                age_ticks: tick.saturating_sub(m.tick).min(MARKER_TICKS) as u32,
            })
            .collect();
        for (producer, at) in &self.rally {
            if self.selection.contains(producer) {
                hud.markers.push(HudMarker {
                    face: at.face,
                    x: at.x,
                    y: at.y,
                    kind: MarkerKind::Rally,
                    age_ticks: 0,
                });
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
        }
    }
}

/// Translate rejected placement and research orders into the rule the player can fix.
fn rejection_message(world: &World, error: &SimError, subject: Option<DefId>) -> String {
    match error {
        SimError::CitiesTooClose => format!(
            "Too close to another city ({} tiles)",
            world.rules().constant("city.spacing").unwrap_or(0) / 1000
        ),
        SimError::NotInOwnTerritory => "Outside your borders".into(),
        SimError::NotInCityRadius => "Outside a city's economic radius".into(),
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
