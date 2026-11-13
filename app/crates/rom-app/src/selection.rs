//! The simulation thread's side of interaction: the selection, control groups, the command
//! palette and build placement. It turns pick events from the render thread into commands for
//! player one, and describes the result as plain data for the HUD snapshot.
//!
//! Behaviour comes from components and production rows, never from definition keys.

use std::collections::BTreeMap;

use rom_render::{
    HudButton, HudCard, HudMarker, HudPalette, HudPlacement, HudQueue, HudQueueItem, HudRegionKind,
    HudSelection, HudSnapshot, HudTile, MarkerKind, Modifiers, PaletteTab, PickEvent,
    PointerButton, SurfacePos, slot_of_key,
};
use rom_rules::{DefId, EntityClass, EntityType, ProductionMode, Rules, StanceFamily};
use rom_sim::{Command, Entity, EntityId, PlayerId, Pos, SimError, Topology, World};

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
}

/// The simulation position of a picked surface position (milli-tiles on a face).
fn pos_of(surface: SurfacePos) -> Pos {
    let clamp = |v: i64| v.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
    Pos {
        x: clamp(surface.x),
        y: clamp(surface.y),
        face: surface.face,
    }
}

/// Distance in milli-tiles between two positions; positions beyond reach (or on faces that do
/// not touch) rank last.
fn distance(world: &World, a: Pos, b: Pos) -> u64 {
    world.distance(a, b).unwrap_or(u64::MAX)
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
                self.ordered = true;
                true
            }
            Err(error) => {
                self.say(world, error.to_string());
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
                pos_of(surface),
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
            } => self.right_click(world, pos_of(surface), entity, modifiers),
            PickEvent::HudClick {
                region,
                index,
                button,
                modifiers,
            } => self.hud_click(world, region, index, button, modifiers),
            PickEvent::Hover { surface, .. } => self.hover(world, pos_of(surface)),
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
                self.focus = match world.topology() {
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
                if row.mode == mode && rules.is_available(row.product_def_id, player.faction) {
                    by_page.entry(row.ui_page.clone()).or_default().push(row);
                }
            }
            let mut pages = Vec::new();
            for (name, mut rows) in by_page {
                rows.sort_by_key(|r| (r.ui_row, r.ui_col, r.product_def_id));
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
                .push(self.button(world, rules, *slot, *action, &own, &sources, &owned));
        }
        Some((palette, actions))
    }

    #[allow(clippy::too_many_arguments)]
    fn button(
        &self,
        world: &World,
        rules: &Rules,
        slot: usize,
        action: Action,
        own: &[&Entity],
        sources: &rom_rules::ActiveSources,
        owned: &rom_rules::Owned,
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
            enabled: true,
            reason: String::new(),
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
                if !rules.is_unlocked(def, sources, owned) {
                    button.enabled = false;
                    button.reason =
                        format!("Requires {}", unmet(rules, def, sources, owned, faction));
                } else if let Some((resource, amount)) = price.iter().find(|(resource, amount)| {
                    player.resources.get(resource).copied().unwrap_or(0) < *amount
                }) {
                    button.enabled = false;
                    button.reason =
                        format!("Not enough {} (need {amount})", rules.def(*resource).name);
                }
                button.armed = matches!(&self.placement, Some(p) if matches!(action, Action::Construct(d) if d == p.def));
            }
            Action::Stop => {
                button.name = "Stop".into();
                button.icon = "icons/cmd_stop".into();
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
            && !button.enabled
        {
            let reason = button.reason.clone();
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
            face: at.face,
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
            face: at.face,
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
                valid: p.valid,
            })
        });
        if let Some((text, at)) = &self.message
            && tick.saturating_sub(*at) <= MESSAGE_TICKS
        {
            hud.events.push(text.clone());
        }
    }
}

/// Display names of the prerequisites of `def` that are not met yet.
fn unmet(
    rules: &Rules,
    def: DefId,
    sources: &rom_rules::ActiveSources,
    owned: &rom_rules::Owned,
    faction: DefId,
) -> String {
    let names: Vec<&str> = rules
        .prerequisites
        .get(&def)
        .into_iter()
        .flatten()
        .filter(|row| {
            let id = row.requires_def_id;
            let met = if let Some(age) = rules.ages.get(&id) {
                sources.iter().any(|s| {
                    rules
                        .ages
                        .get(s)
                        .is_some_and(|later| later.ordinal >= age.ordinal)
                })
            } else if rules.techs.contains_key(&id) {
                sources.contains(&id)
            } else {
                owned.contains(&id)
            };
            !met
        })
        .map(|row| rules.display_name(row.requires_def_id, faction))
        .collect();
    if names.is_empty() {
        "a prerequisite".to_owned()
    } else {
        names.join(", ")
    }
}

#[cfg(test)]
mod tests;
