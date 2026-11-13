use super::*;
use rom_render::{PickEvent, SurfacePos, flat_surface};
use rom_rules::Rules;
use rom_sim::{MapGrid, Player};
use std::collections::BTreeMap;
use std::sync::Arc;

struct Fx {
    world: World,
    rules: Arc<Rules>,
    ctl: Controller,
}

fn fixture_with(stock: i64) -> Fx {
    let rules = Arc::new(crate::terrain::load_rules().expect("rules"));
    let id = |key: &str| rules.id_of(key).expect("definition");
    let player = |n: u32| Player {
        id: PlayerId(n),
        team: n,
        age: id("age.ancient"),
        faction: id("nation.romans"),
        resources: rules.resources.keys().map(|r| (*r, stock)).collect(),
        techs: BTreeMap::new(),
    };
    let map = MapGrid::new(64, 64, vec![id("ter.plain"); 64 * 64]).expect("map");
    let world =
        World::new(rules.clone(), map, 1, vec![player(0), player(1), player(2)]).expect("world");
    Fx {
        world,
        rules,
        ctl: Controller::new(),
    }
}

fn fixture() -> Fx {
    fixture_with(10_000)
}

fn at(x: i32, y: i32) -> Pos {
    Pos {
        x: x * 1000 + 500,
        y: y * 1000 + 500,
        face: 0,
        ..Pos::default()
    }
}

impl Fx {
    /// First type, in id order, matching a component test.
    fn find(&self, test: impl Fn(&EntityType) -> bool) -> DefId {
        self.rules
            .entity_types
            .values()
            .find(|et| test(et))
            .expect("a type with this component")
            .id
    }
    fn spawn(&mut self, type_id: DefId, owner: u32, x: i32, y: i32) -> EntityId {
        self.world
            .spawn(type_id, PlayerId(owner), at(x, y))
            .expect("spawn")
    }
    fn mover(&self) -> DefId {
        self.find(|et| {
            et.class == EntityClass::Unit && et.builder.is_some() && et.movement.is_some()
        })
    }
    fn armed_other_than_builder(&self) -> DefId {
        self.find(|et| et.movement.is_some() && !et.weapons.is_empty() && et.builder.is_none())
    }
    fn unarmed(&self) -> DefId {
        self.find(|et| {
            et.class == EntityClass::Unit && et.movement.is_some() && et.weapons.is_empty()
        })
    }
    fn producer_with(&self, mode: ProductionMode) -> DefId {
        self.find(|et| {
            et.class == EntityClass::Building
                && et.producer.is_some()
                && self
                    .rules
                    .production
                    .get(&et.id)
                    .is_some_and(|rows| rows.iter().any(|r| r.mode == mode))
        })
    }
    fn click(&mut self, id: EntityId, shift: bool) {
        self.ctl.handle(
            &mut self.world,
            PickEvent::Click {
                world_point: (0.0, 0.0),
                surface: flat_surface((0.0, 0.0)),
                entity: Some(id.0),
                modifiers: Modifiers {
                    shift,
                    ..Modifiers::default()
                },
                double: false,
                on_screen: Vec::new(),
            },
        );
    }
    fn ground_click(&mut self) {
        self.ctl.handle(
            &mut self.world,
            PickEvent::Click {
                world_point: (1.0, 1.0),
                surface: flat_surface((1.0, 1.0)),
                entity: None,
                modifiers: Modifiers::default(),
                double: false,
                on_screen: Vec::new(),
            },
        );
    }
    fn boxed(&mut self, ids: &[EntityId], ctrl: bool) {
        self.ctl.handle(
            &mut self.world,
            PickEvent::BoxSelect {
                entities: ids.iter().map(|i| i.0).collect(),
                modifiers: Modifiers {
                    ctrl,
                    ..Modifiers::default()
                },
            },
        );
    }
    fn select(&mut self, ids: &[EntityId]) {
        self.boxed(ids, true);
        // Buildings are not boxed; click them instead.
        if self.ctl.selection().is_empty() {
            for (n, id) in ids.iter().enumerate() {
                self.click(*id, n > 0);
            }
        }
    }
    fn rclick(&mut self, entity: Option<EntityId>, point: (f32, f32), shift: bool) {
        self.ctl.handle(
            &mut self.world,
            PickEvent::RightClick {
                world_point: point,
                surface: flat_surface(point),
                entity: entity.map(|e| e.0),
                modifiers: Modifiers {
                    shift,
                    ..Modifiers::default()
                },
            },
        );
    }
    fn hud(&mut self, region: HudRegionKind, index: usize, button: PointerButton, shift: bool) {
        self.ctl.handle(
            &mut self.world,
            PickEvent::HudClick {
                region,
                index,
                button,
                modifiers: Modifiers {
                    shift,
                    ..Modifiers::default()
                },
            },
        );
    }
    fn step(&mut self, n: usize) {
        for _ in 0..n {
            self.world.step();
            self.ctl.after_step(&mut self.world);
        }
    }
    fn hud_snapshot(&self) -> HudSnapshot {
        let mut hud = HudSnapshot::default();
        self.ctl.fill(&self.world, &mut hud);
        hud
    }
    fn kinds(&self, id: EntityId) -> Vec<String> {
        self.world
            .entity(id)
            .expect("alive")
            .orders()
            .iter()
            .map(|o| o.kind.clone())
            .collect()
    }
    fn stock(&self) -> BTreeMap<DefId, i64> {
        self.world
            .players()
            .find(|p| p.id == PLAYER)
            .unwrap()
            .resources
            .clone()
    }
}

fn ids(fx: &Fx) -> Vec<u32> {
    fx.ctl.selection().iter().map(|e| e.0).collect()
}

#[test]
fn click_and_shift_click_selection() {
    let mut fx = fixture();
    let t = fx.mover();
    let (a, b, c) = (
        fx.spawn(t, 1, 10, 10),
        fx.spawn(t, 1, 12, 10),
        fx.spawn(t, 1, 14, 10),
    );
    let foe = fx.spawn(t, 2, 30, 30);
    fx.click(a, false);
    assert_eq!(ids(&fx), vec![a.0]);
    fx.click(b, true);
    assert_eq!(ids(&fx), vec![a.0, b.0]);
    fx.click(a, true);
    assert_eq!(ids(&fx), vec![b.0]);
    // A double click picks every own entity of the type that is on screen.
    fx.ctl.handle(
        &mut fx.world,
        PickEvent::Click {
            world_point: (0.0, 0.0),
            surface: flat_surface((0.0, 0.0)),
            entity: Some(c.0),
            modifiers: Modifiers::default(),
            double: true,
            on_screen: vec![a.0, b.0, c.0, foe.0],
        },
    );
    assert_eq!(ids(&fx), vec![a.0, b.0, c.0]);
    fx.click(foe, false);
    assert!(fx.ctl.selection().is_empty());
    let hud = fx.hud_snapshot();
    assert!(matches!(hud.selection, HudSelection::Single(ref card) if card.foreign));
    assert!(hud.palette.is_none(), "no orders for foreign entities");
    fx.click(a, false);
    fx.ground_click();
    assert!(fx.ctl.selection().is_empty());
}

#[test]
fn box_prefers_military_unless_ctrl() {
    let mut fx = fixture();
    let (armed_t, plain_t) = (fx.armed_other_than_builder(), fx.unarmed());
    let armed = fx.spawn(armed_t, 1, 10, 10);
    let plain = fx.spawn(plain_t, 1, 12, 10);
    let foe = fx.spawn(armed_t, 2, 14, 10);
    fx.boxed(&[armed, plain, foe], false);
    assert_eq!(ids(&fx), vec![armed.0]);
    fx.boxed(&[armed, plain, foe], true);
    assert_eq!(ids(&fx), vec![armed.0, plain.0]);
    fx.boxed(&[plain], false);
    assert_eq!(ids(&fx), vec![plain.0]);
    // Shift adds to the selection.
    fx.ctl.handle(
        &mut fx.world,
        PickEvent::BoxSelect {
            entities: vec![armed.0],
            modifiers: Modifiers {
                shift: true,
                ..Modifiers::default()
            },
        },
    );
    assert_eq!(ids(&fx), vec![armed.0, plain.0]);
}

#[test]
fn control_group_store_and_recall() {
    let mut fx = fixture();
    let t = fx.mover();
    let (a, b) = (fx.spawn(t, 1, 10, 10), fx.spawn(t, 1, 30, 20));
    fx.boxed(&[a, b], false);
    let ctrl = Modifiers {
        ctrl: true,
        ..Modifiers::default()
    };
    fx.ctl.handle(
        &mut fx.world,
        PickEvent::Hotkey {
            ch: '3',
            modifiers: ctrl,
        },
    );
    fx.ground_click();
    assert!(fx.ctl.selection().is_empty());
    let plain = PickEvent::Hotkey {
        ch: '3',
        modifiers: Modifiers::default(),
    };
    fx.ctl.handle(&mut fx.world, plain.clone());
    assert_eq!(ids(&fx), vec![a.0, b.0]);
    assert!(fx.ctl.take_focus().is_none());
    fx.ctl.handle(&mut fx.world, plain);
    let (x, y) = fx
        .ctl
        .take_focus()
        .expect("second press centres the camera");
    assert!((x - 20.5).abs() < 0.01 && (y - 15.5).abs() < 0.01);
    // Dead members drop out of the recalled group.
    fx.world.despawn(b).unwrap();
    fx.ground_click();
    fx.ctl.handle(
        &mut fx.world,
        PickEvent::Hotkey {
            ch: '3',
            modifiers: Modifiers::default(),
        },
    );
    assert_eq!(ids(&fx), vec![a.0]);
}

#[test]
fn right_click_enemy_attacks_and_ground_moves() {
    let mut fx = fixture();
    let t = fx.armed_other_than_builder();
    let a = fx.spawn(t, 1, 10, 10);
    // Within the unit's line of sight: hidden enemies cannot be targeted.
    let foe = fx.spawn(t, 2, 12, 12);
    fx.click(a, false);
    fx.rclick(Some(foe), (12.5, 12.5), false);
    fx.step(1);
    assert_eq!(fx.kinds(a)[0], "attack");
    assert_eq!(fx.hud_snapshot().markers[0].kind, MarkerKind::Attack);
    fx.rclick(None, (20.0, 12.0), false);
    fx.step(1);
    assert_eq!(fx.kinds(a), vec!["move"]);
    assert_eq!(
        fx.hud_snapshot().markers.last().unwrap().kind,
        MarkerKind::Move
    );
    assert!(fx.ctl.player_has_ordered());
    // Markers last half a second.
    fx.step(12);
    assert!(fx.hud_snapshot().markers.is_empty());
}

#[test]
fn right_click_gather_site_harvests() {
    let mut fx = fixture();
    let worker_t = fx.find(|et| et.movement.is_some() && !et.harvester.is_empty());
    let resource = fx.rules.entity_types[&worker_t].harvester[0].resource_id;
    let site_t = fx.find(|et| {
        et.class == EntityClass::Building
            && et
                .gather_site
                .as_ref()
                .is_some_and(|g| g.resource_id == resource)
    });
    let worker = fx.spawn(worker_t, 1, 10, 10);
    let site = fx.spawn(site_t, 1, 20, 20);
    let other_t = fx.unarmed();
    let idler = fx.find(|et| et.movement.is_some() && et.harvester.is_empty());
    let _ = other_t;
    let useless = fx.spawn(idler, 1, 12, 10);
    fx.boxed(&[worker, useless], true);
    fx.rclick(Some(site), (20.5, 20.5), false);
    fx.step(1);
    assert_eq!(fx.kinds(worker)[0], "harvest");
    assert_eq!(fx.kinds(useless), vec!["move"]);
}

#[test]
fn right_click_site_helps_build() {
    let mut fx = fixture();
    let bt = fx.mover();
    let building = fx
        .rules
        .production
        .get(&bt)
        .unwrap()
        .iter()
        .find(|r| r.mode == ProductionMode::Construct)
        .unwrap()
        .product_def_id;
    let (builder, helper) = (fx.spawn(bt, 1, 10, 10), fx.spawn(bt, 1, 12, 10));
    fx.world
        .submit(Command {
            player: PLAYER,
            executor: builder,
            kind: "construct".into(),
            subject: Some(building),
            target_entity: None,
            target_pos: Some(at(30, 30)),
            queue: false,
        })
        .expect("construct");
    fx.step(1);
    let site = fx
        .world
        .entities()
        .find(|e| e.build_progress.is_some())
        .expect("site")
        .id;
    fx.click(helper, false);
    fx.rclick(Some(site), (30.0, 30.0), false);
    fx.step(1);
    assert_eq!(fx.kinds(helper)[0], "repair");
}

#[test]
fn shift_queues_orders() {
    let mut fx = fixture();
    let t = fx.mover();
    let a = fx.spawn(t, 1, 10, 10);
    fx.click(a, false);
    fx.rclick(None, (20.0, 10.0), false);
    fx.rclick(None, (25.0, 10.0), true);
    fx.rclick(None, (30.0, 10.0), true);
    fx.step(1);
    assert_eq!(fx.kinds(a), vec!["move", "move", "move"]);
    // Without Shift the order replaces the queue.
    fx.rclick(None, (5.0, 5.0), false);
    fx.step(1);
    assert_eq!(fx.kinds(a), vec!["move"]);
}

#[test]
fn palette_lists_production_rows_for_selected_type() {
    let mut fx = fixture();
    let bt = fx.mover();
    let builder = fx.spawn(bt, 1, 10, 10);
    fx.click(builder, false);
    let hud = fx.hud_snapshot();
    let palette = hud.palette.expect("palette");
    assert_eq!(palette.tab, PaletteTab::Build);
    assert!(palette.tabs[PaletteTab::Build.index()]);
    let faction = fx.world.players().find(|p| p.id == PLAYER).unwrap().faction;
    let rows: Vec<_> = fx.rules.production[&bt]
        .iter()
        .filter(|r| r.mode == ProductionMode::Construct && r.ui_page == palette.page_name)
        .collect();
    assert!(!rows.is_empty() && palette.pages >= 1);
    assert_eq!(palette.buttons.len(), rows.len());
    for row in rows {
        let slot = row.ui_row as usize * 5 + row.ui_col as usize;
        let button = palette
            .buttons
            .iter()
            .find(|b| b.slot == slot)
            .expect("slot");
        assert_eq!(
            button.name,
            fx.rules.display_name(row.product_def_id, faction)
        );
    }
    // Pages follow the arrows and wrap.
    if palette.pages > 1 {
        fx.hud(HudRegionKind::PaletteArrow, 1, PointerButton::Left, false);
        let next = fx.hud_snapshot().palette.unwrap();
        assert_eq!(next.page, 1);
        assert_ne!(next.page_name, palette.page_name);
    }
    // A producer building gets TRAIN with its train rows.
    let pt = fx.producer_with(ProductionMode::Train);
    let producer = fx.spawn(pt, 1, 30, 30);
    fx.click(producer, false);
    let palette = fx.hud_snapshot().palette.expect("palette");
    assert_eq!(palette.tab, PaletteTab::Train);
    let train_rows = fx.rules.production[&pt]
        .iter()
        .filter(|r| r.mode == ProductionMode::Train)
        .count();
    assert_eq!(palette.buttons.len(), train_rows);
    assert!(!palette.tabs[PaletteTab::Build.index()]);
    // A unit with a weapon gets ORDER with stop, attack-move and the four stances.
    let armed = fx.spawn(fx.armed_other_than_builder(), 1, 20, 20);
    fx.click(armed, false);
    let palette = fx.hud_snapshot().palette.expect("palette");
    assert_eq!(palette.tab, PaletteTab::Order);
    assert_eq!(palette.buttons.len(), 2 + STANCE_BEHAVIORS.len());
}

#[test]
fn locked_and_unaffordable_buttons_are_disabled_with_reason() {
    let mut fx = fixture();
    let bt = fx.mover();
    let b = fx.spawn(bt, 1, 10, 10);
    fx.click(b, false);
    let mut seen_locked = false;
    for page in 0..fx.hud_snapshot().palette.unwrap().pages {
        fx.ctl.page = page;
        for button in fx.hud_snapshot().palette.unwrap().buttons {
            if !button.enabled && button.reason.starts_with("Requires") {
                seen_locked = true;
            }
            assert_eq!(button.enabled, button.reason.is_empty());
        }
    }
    // Locked rows exist in every rules database that has prerequisites; check the research and
    // train rows of producers too.
    for mode in [ProductionMode::Train, ProductionMode::Research] {
        let pt = fx.producer_with(mode);
        let p = fx.spawn(pt, 1, 40, 40);
        fx.click(p, false);
        fx.ctl.tab = if mode == ProductionMode::Train {
            PaletteTab::Train
        } else {
            PaletteTab::Research
        };
        for button in fx.hud_snapshot().palette.unwrap().buttons {
            seen_locked |= !button.enabled && button.reason.starts_with("Requires");
        }
    }
    assert!(seen_locked, "at least one row is locked at the start");
    // Without stock every priced button is disabled and says what is missing.
    let mut poor = fixture_with(0);
    let b = poor.spawn(poor.mover(), 1, 10, 10);
    poor.click(b, false);
    let palette = poor.hud_snapshot().palette.unwrap();
    let priced: Vec<_> = palette
        .buttons
        .iter()
        .filter(|b| !b.price.is_empty())
        .collect();
    assert!(!priced.is_empty());
    assert!(priced.iter().all(|b| !b.enabled && !b.reason.is_empty()));
    assert!(
        priced
            .iter()
            .any(|b| b.reason.starts_with("Not enough") || b.reason.starts_with("Requires"))
    );
    // Pressing a disabled button does nothing but explain.
    let slot = priced[0].slot;
    poor.hud(HudRegionKind::CommandSlot, slot, PointerButton::Left, false);
    assert!(poor.ctl.placement.is_none());
    assert!(!poor.hud_snapshot().events.is_empty());
}

#[test]
fn train_button_submits_and_queue_shows_progress() {
    let mut fx = fixture();
    let pt = fx.producer_with(ProductionMode::Train);
    let producer = fx.spawn(pt, 1, 30, 30);
    fx.click(producer, false);
    let palette = fx.hud_snapshot().palette.unwrap();
    let button = palette
        .buttons
        .iter()
        .find(|b| b.enabled)
        .expect("a trainable unit");
    let (slot, name) = (button.slot, button.name.clone());
    let before = fx.stock();
    fx.hud(HudRegionKind::CommandSlot, slot, PointerButton::Left, false);
    fx.step(3);
    assert!(
        fx.stock().iter().any(|(r, v)| *v < before[r]),
        "the price was paid"
    );
    let queue = fx.hud_snapshot().queue.expect("queue");
    assert_eq!(queue.items.len(), 1);
    assert_eq!(queue.items[0].name, name);
    assert!(queue.items[0].progress_pct > 0 && queue.items[0].progress_pct < 100);
    // Shift trains five and the queue shows three plus the rest.
    fx.hud(HudRegionKind::CommandSlot, slot, PointerButton::Left, true);
    fx.step(1);
    let queue = fx.hud_snapshot().queue.unwrap();
    let limit = fx.rules.entity_types[&pt]
        .producer
        .as_ref()
        .unwrap()
        .queue_limit as usize;
    assert_eq!(queue.items.len(), 3.min(limit));
    assert_eq!(queue.more, (1 + SHIFT_BATCH).min(limit).saturating_sub(3));
}

#[test]
fn cancel_from_queue_refunds() {
    let mut fx = fixture();
    let pt = fx.producer_with(ProductionMode::Train);
    let producer = fx.spawn(pt, 1, 30, 30);
    fx.click(producer, false);
    let slot = fx
        .hud_snapshot()
        .palette
        .unwrap()
        .buttons
        .iter()
        .find(|b| b.enabled)
        .unwrap()
        .slot;
    let before = fx.stock();
    fx.hud(HudRegionKind::CommandSlot, slot, PointerButton::Left, false);
    fx.step(2);
    let paid = fx.stock();
    assert!(paid.iter().any(|(r, v)| *v < before[r]));
    fx.hud(HudRegionKind::QueueItem, 0, PointerButton::Right, false);
    fx.step(2);
    let pct = fx.rules.constant("economy.cancel_refund_pct").unwrap();
    for (resource, now) in fx.stock() {
        let spent = before[&resource] - paid[&resource];
        assert_eq!(now, paid[&resource] + spent * pct / 100, "{resource:?}");
    }
    assert!(fx.hud_snapshot().queue.unwrap().items.is_empty());
    // With two entries only the chosen one goes; the other is ordered again.
    fx.hud(HudRegionKind::CommandSlot, slot, PointerButton::Left, false);
    fx.hud(HudRegionKind::CommandSlot, slot, PointerButton::Left, false);
    fx.step(2);
    assert_eq!(fx.hud_snapshot().queue.unwrap().items.len(), 2);
    fx.hud(HudRegionKind::QueueItem, 1, PointerButton::Right, false);
    fx.step(3);
    assert_eq!(fx.hud_snapshot().queue.unwrap().items.len(), 1);
}

#[test]
fn placement_check_reports_valid_and_invalid() {
    let mut fx = fixture();
    let bt = fx.mover();
    let is_city = |id: &DefId| {
        fx.rules.entity_types[id]
            .territory
            .as_ref()
            .is_some_and(|t| t.is_city)
    };
    let constructs: Vec<DefId> = fx
        .rules
        .production
        .get(&bt)
        .unwrap()
        .iter()
        .filter(|r| r.mode == ProductionMode::Construct)
        .map(|r| r.product_def_id)
        .collect();
    let city_type = *constructs.iter().find(|id| is_city(id)).unwrap();
    // A building that is not a city, so the player's city limit does not apply.
    let building = *constructs.iter().find(|id| !is_city(id)).unwrap();
    let et = fx.rules.entity_types[&building].clone();
    let city = fx
        .world
        .place_for_setup(city_type, PLAYER, at(20, 20))
        .expect("town");
    // Inside the city's economic radius and territory, clear of the city's footprint.
    let free = Pos {
        x: snap(30_000, et.footprint_w),
        y: snap(20_000, et.footprint_h),
        face: 0,
        ..Pos::default()
    };
    assert_eq!(placement_check(&fx.world, building, free), Ok(()));
    assert_eq!(
        placement_check(&fx.world, building, fx.world.entity(city).unwrap().pos),
        Err(SimError::PlacementOverlap)
    );
    assert_eq!(
        placement_check(&fx.world, building, Pos::flat(100, 100)),
        Err(SimError::OutsideMap)
    );
    // The submit result agrees on a valid and an invalid spot.
    let builder = fx.spawn(bt, 1, 10, 10);
    let command = |pos| Command {
        player: PLAYER,
        executor: builder,
        kind: "construct".into(),
        subject: Some(building),
        target_entity: None,
        target_pos: Some(pos),
        queue: false,
    };
    assert!(fx.world.submit(command(free)).is_ok());
    assert_eq!(
        fx.world
            .submit(command(fx.world.entity(city).unwrap().pos))
            .unwrap_err(),
        SimError::PlacementOverlap
    );
    // The controller publishes the answer for the hovered tile and places on click.
    fx.click(builder, false);
    let faction = fx.world.players().find(|p| p.id == PLAYER).unwrap().faction;
    let name = fx.rules.display_name(building, faction).to_string();
    let slot = fx
        .hud_snapshot()
        .palette
        .unwrap()
        .buttons
        .iter()
        .find(|b| b.enabled && b.name == name)
        .unwrap()
        .slot;
    fx.hud(HudRegionKind::CommandSlot, slot, PointerButton::Left, false);
    assert!(
        fx.hud_snapshot()
            .palette
            .unwrap()
            .buttons
            .iter()
            .any(|b| b.armed)
    );
    fx.ctl.handle(
        &mut fx.world,
        PickEvent::Hover {
            // Inside the city's radius and territory, clear of the city and the queued site.
            world_point: (20.5, 30.5),
            surface: flat_surface((20.5, 30.5)),
        },
    );
    let preview = fx.hud_snapshot().placement.expect("preview");
    assert_eq!(preview.valid, Some(true));
    fx.step(PLACEMENT_CHECK_TICKS as usize);
    fx.ctl.handle(
        &mut fx.world,
        PickEvent::Hover {
            world_point: (20.5, 20.5),
            surface: flat_surface((20.5, 20.5)),
        },
    );
    assert_eq!(fx.hud_snapshot().placement.unwrap().valid, Some(false));
    fx.ctl.handle(&mut fx.world, PickEvent::Escape);
    assert!(fx.hud_snapshot().placement.is_none());
}

#[test]
fn dead_entities_leave_selection() {
    let mut fx = fixture();
    let t = fx.mover();
    let (a, b) = (fx.spawn(t, 1, 10, 10), fx.spawn(t, 1, 12, 10));
    fx.boxed(&[a, b], false);
    assert_eq!(ids(&fx), vec![a.0, b.0]);
    fx.world.despawn(a).unwrap();
    fx.step(1);
    assert_eq!(ids(&fx), vec![b.0]);
    assert_eq!(fx.hud_snapshot().selected, vec![b.0]);
    fx.select(&[]);
}

/// A 64-tile planet with two players set up as in a real sphere match.
fn sphere_fixture() -> (Controller, crate::match_setup::Match) {
    let options = crate::args::Options {
        players: 2,
        sphere: Some(64),
        ..crate::args::Options::default()
    };
    let content = crate::terrain::generate_sphere_content(&options, 64).expect("planet");
    let game = crate::match_setup::build_sphere_match(
        &content.rules,
        &content.planet,
        &content.factions,
        1,
    )
    .expect("match");
    (Controller::new(), game)
}

fn surface_of(pos: Pos) -> SurfacePos {
    SurfacePos {
        face: pos.face,
        x: i64::from(pos.x),
        y: i64::from(pos.y),
    }
}

#[test]
fn orders_on_the_sphere_use_the_picked_face() {
    let (mut ctl, mut game) = sphere_fixture();
    let world = &mut game.world;
    let start = game.starts.iter().find(|s| s.player == PLAYER).unwrap();
    let builder = start.builders[0];
    ctl.handle(
        world,
        PickEvent::BoxSelect {
            entities: vec![builder.0],
            modifiers: Modifiers::default(),
        },
    );
    assert_eq!(ctl.selection(), &[builder]);
    let other = SurfacePos {
        face: (start.city_pos.face + 1) % 6,
        x: 20_500,
        y: 33_500,
    };
    ctl.handle(
        world,
        PickEvent::RightClick {
            world_point: (20.5, 33.5),
            surface: other,
            entity: None,
            modifiers: Modifiers::default(),
        },
    );
    world.step();
    let order = world
        .entity(builder)
        .unwrap()
        .orders()
        .back()
        .expect("a move order");
    assert_eq!(order.kind, "move");
    assert_eq!(order.target_pos, Some(pos_of(other)));
    assert_eq!(order.target_pos.unwrap().face, other.face);
}

#[test]
fn placement_on_the_sphere_matches_check_placement() {
    let (mut ctl, mut game) = sphere_fixture();
    let world = &mut game.world;
    let rules = world.rules().clone();
    let start = game.starts.iter().find(|s| s.player == PLAYER).unwrap();
    let builder = start.builders[0];
    ctl.handle(
        world,
        PickEvent::BoxSelect {
            entities: vec![builder.0],
            modifiers: Modifiers::default(),
        },
    );
    // The first enabled construct button that is not a city.
    let hud = {
        let mut hud = HudSnapshot::default();
        ctl.fill(world, &mut hud);
        hud
    };
    let slot = hud
        .palette
        .as_ref()
        .and_then(|p| p.buttons.iter().find(|b| b.enabled))
        .expect("an enabled button")
        .slot;
    ctl.handle(
        world,
        PickEvent::HudClick {
            region: HudRegionKind::CommandSlot,
            index: slot,
            button: PointerButton::Left,
            modifiers: Modifiers::default(),
        },
    );
    let def = ctl.placement.as_ref().expect("placement mode").def;
    let footprint = {
        let et = &rules.entity_types[&def];
        (et.footprint_w, et.footprint_h)
    };
    let city = start.city_pos;
    let spots = [
        // Beside the city on its face, on the city itself, and on a far face.
        world.offset(city, 5_000, 0),
        city,
        Pos {
            x: 10_500,
            y: 10_500,
            face: (city.face + 3) % 6,
            ..Pos::default()
        },
        // Right at a face edge, where the footprint may cross it.
        Pos {
            x: 63_900,
            y: 30_500,
            face: city.face,
            ..Pos::default()
        },
    ];
    for (n, spot) in spots.into_iter().enumerate() {
        ctl.placement.as_mut().unwrap().checked = None;
        ctl.handle(
            world,
            PickEvent::Hover {
                world_point: (spot.x as f32 / 1000.0, spot.y as f32 / 1000.0),
                surface: surface_of(spot),
            },
        );
        let mut hud = HudSnapshot::default();
        ctl.fill(world, &mut hud);
        let preview = hud.placement.expect("preview");
        let snapped = Pos {
            x: snap(spot.x, footprint.0),
            y: snap(spot.y, footprint.1),
            face: spot.face,
            ..Pos::default()
        };
        assert_eq!(preview.face, spot.face, "spot {n}");
        assert_eq!((preview.x, preview.y), (snapped.x, snapped.y), "spot {n}");
        assert_eq!(
            preview.valid,
            Some(world.check_placement(PLAYER, def, snapped).is_ok()),
            "spot {n}"
        );
    }
}
