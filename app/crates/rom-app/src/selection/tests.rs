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
    fixture_config(stock, "age.ancient", &[])
}

fn fixture_config(stock: i64, age: &str, researched: &[&str]) -> Fx {
    let rules = Arc::new(crate::terrain::load_rules().expect("rules"));
    let id = |key: &str| rules.id_of(key).expect("definition");
    let player = |n: u32| Player {
        id: PlayerId(n),
        team: n,
        age: id(age),
        faction: id("nation.romans"),
        resources: rules.resources.keys().map(|r| (*r, stock)).collect(),
        techs: researched.iter().map(|key| (id(key), 1)).collect(),
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
            et.class == EntityClass::Unit
                && et.movement.is_some()
                && et.weapons.is_empty()
                && !supports_army(&self.rules, et)
        })
    }
    fn key(&self, key: &str) -> DefId {
        self.rules.id_of(key).expect("definition")
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
fn box_takes_medics_and_supply_carts_with_soldiers_not_workers() {
    let mut fx = fixture();
    let soldier = fx.spawn(fx.key("unit.spearman"), 1, 10, 10);
    let medic = fx.spawn(fx.key("unit.medic"), 1, 11, 10);
    let wagon = fx.spawn(fx.key("unit.supply_wagon"), 1, 12, 10);
    let citizen = fx.spawn(fx.key("unit.citizen"), 1, 13, 10);
    let priest = fx.spawn(fx.key("unit.priest"), 1, 14, 10);
    let all = [soldier, medic, wagon, citizen, priest];
    fx.boxed(&all, false);
    assert_eq!(ids(&fx), vec![soldier.0, medic.0, wagon.0]);
    // Without soldiers in the box nothing is left out; Ctrl keeps everything.
    fx.boxed(&[medic, citizen, priest], false);
    assert_eq!(ids(&fx), vec![medic.0, citizen.0, priest.0]);
    fx.boxed(&all, true);
    assert_eq!(ids(&fx).len(), 5);
    // A double click on a soldier takes the soldiers of its type and the support on screen.
    let archer = fx.spawn(fx.key("unit.slinger"), 1, 15, 10);
    let other = fx.spawn(fx.key("unit.spearman"), 1, 16, 10);
    fx.ctl.handle(
        &mut fx.world,
        PickEvent::Click {
            world_point: (0.0, 0.0),
            surface: flat_surface((0.0, 0.0)),
            entity: Some(soldier.0),
            modifiers: Modifiers::default(),
            double: true,
            on_screen: vec![
                soldier.0, medic.0, wagon.0, citizen.0, priest.0, archer.0, other.0,
            ],
        },
    );
    assert_eq!(ids(&fx), vec![soldier.0, medic.0, wagon.0, other.0]);
    // A double click on a medic takes the medics only.
    fx.ctl.handle(
        &mut fx.world,
        PickEvent::Click {
            world_point: (0.0, 0.0),
            surface: flat_surface((0.0, 0.0)),
            entity: Some(medic.0),
            modifiers: Modifiers::default(),
            double: true,
            on_screen: vec![soldier.0, medic.0, wagon.0],
        },
    );
    assert_eq!(ids(&fx), vec![medic.0]);
}

#[test]
fn support_walks_with_an_attack_move_and_builders_mend_on_right_click() {
    let mut fx = fixture();
    let soldier = fx.spawn(fx.key("unit.spearman"), 1, 10, 10);
    let medic = fx.spawn(fx.key("unit.medic"), 1, 11, 10);
    fx.boxed(&[soldier, medic], false);
    fx.ctl.attack_move = true;
    fx.ground_click();
    fx.step(1);
    assert_eq!(fx.kinds(soldier), ["attack_move"]);
    assert_eq!(fx.kinds(medic), ["move"]);
    assert!(
        !fx.ctl.take_cues().contains(&crate::sound::UiCue::Refused),
        "the medic is not refused"
    );
    // An enemy knocks a farm about; a citizen sent to it mends it.
    let farm = fx.spawn(fx.key("bld.farm"), 1, 30, 30);
    let foe = fx.spawn(fx.key("unit.spearman"), 2, 33, 30);
    fx.world
        .submit(Command {
            player: PlayerId(2),
            executor: foe,
            kind: "attack".into(),
            subject: None,
            target_entity: Some(farm),
            target_pos: None,
            queue: false,
            text: None,
            facing: None,
        })
        .expect("attack");
    for _ in 0..400 {
        let f = fx.world.entity(farm).expect("farm");
        if f.hp < f.max_hp {
            break;
        }
        fx.step(1);
    }
    let f = fx.world.entity(farm).expect("farm");
    assert!(f.hp < f.max_hp);
    let citizen = fx.spawn(fx.key("unit.citizen"), 1, 26, 30);
    fx.click(citizen, false);
    fx.rclick(Some(farm), (30.0, 30.0), false);
    fx.step(1);
    assert_eq!(fx.kinds(citizen)[0], "repair");
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
fn orders_and_refusals_are_told_to_the_sound() {
    let mut fx = fixture();
    let t = fx.armed_other_than_builder();
    let a = fx.spawn(t, 1, 10, 10);
    let foe = fx.spawn(t, 2, 12, 12);
    // Scripts name them `army1` and `foe1` (task 125).
    let pick = crate::script::live_pick(&fx.world, PLAYER, "army1", false, false).unwrap();
    fx.ctl.handle(&mut fx.world, pick);
    assert_eq!(ids(&fx), vec![a.0]);
    let pick = crate::script::live_pick(&fx.world, PLAYER, "foe1", true, false).unwrap();
    fx.ctl.handle(&mut fx.world, pick);
    assert_eq!(
        fx.ctl.take_cues(),
        vec![crate::sound::UiCue::Ordered {
            executor: a,
            kind: "attack".into()
        }]
    );
    assert!(fx.ctl.take_cues().is_empty());
    // Ordering a foreign unit's attack on itself is refused: the error cue.
    fx.ctl.submit(
        &mut OrderSink::Lead(&mut fx.world),
        foe,
        "attack",
        None,
        Some(a),
        None,
        false,
    );
    assert_eq!(fx.ctl.take_cues(), vec![crate::sound::UiCue::Refused]);
}

#[test]
fn right_click_ground_with_a_producer_sets_its_rally_through_an_order() {
    let mut fx = fixture();
    let t = fx.producer_with(ProductionMode::Train);
    let producer = fx.spawn(t, 1, 10, 10);
    fx.click(producer, false);
    fx.rclick(None, (20.5, 12.5), false);
    fx.step(2);
    assert!(fx.world.rally(producer).is_some());
    let flags: Vec<_> = fx
        .hud_snapshot()
        .markers
        .into_iter()
        .filter(|m| m.kind == MarkerKind::Rally)
        .collect();
    assert_eq!(flags.len(), 1);
    assert!(flags[0].from.is_some());
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
            text: None,
            facing: None,
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
    // Ancient producers show the first step of each line; successors are hidden.
    let train_rows = fx.rules.production[&pt]
        .iter()
        .filter(|r| {
            r.mode == ProductionMode::Train
                && fx.rules.entity_types[&r.product_def_id]
                    .upgrades_from_id
                    .is_none()
                && fx.rules.is_available(
                    r.product_def_id,
                    fx.world.players().find(|p| p.id == PLAYER).unwrap().faction,
                )
        })
        .count();
    assert_eq!(palette.buttons.len(), train_rows);
    assert!(!palette.tabs[PaletteTab::Build.index()]);
    // A unit with a weapon gets ORDER with stop, attack-move, patrol, the four stances and
    // Delete (task 142; Attack ground too when it throws projectiles).
    let kind = fx.armed_other_than_builder();
    let armed = fx.spawn(kind, 1, 20, 20);
    fx.click(armed, false);
    let palette = fx.hud_snapshot().palette.expect("palette");
    assert_eq!(palette.tab, PaletteTab::Order);
    let ground = usize::from(fx.world.attacks_ground(kind));
    assert_eq!(palette.buttons.len(), 4 + STANCE_BEHAVIORS.len() + ground);
}

#[test]
fn locked_and_unaffordable_buttons_are_disabled_with_reason() {
    let mut fx = fixture();
    let bt = fx.mover();
    let b = fx.spawn(bt, 1, 10, 10);
    fx.click(b, false);
    let mut seen_locked = false;
    for page in 0..fx.hud_snapshot().palette.unwrap().pages {
        fx.ctl.page = Some(page);
        for button in fx.hud_snapshot().palette.unwrap().buttons {
            if !button.state.enabled() && button.state.reason().starts_with("Needs") {
                seen_locked = true;
            }
            assert_eq!(button.state.enabled(), button.state.reason().is_empty());
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
            seen_locked |= !button.state.enabled() && button.state.reason().starts_with("Needs");
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
    assert!(
        priced
            .iter()
            .all(|b| !b.state.enabled() && !b.state.reason().is_empty())
    );
    assert!(priced.iter().any(
        |b| b.state.reason().starts_with("Not enough") || b.state.reason().starts_with("Needs")
    ));
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
        .find(|b| b.state.enabled())
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
        .find(|b| b.state.enabled())
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
    assert_eq!(placement_check(&fx.world, PLAYER, building, free), Ok(()));
    assert_eq!(
        placement_check(
            &fx.world,
            PLAYER,
            building,
            fx.world.entity(city).unwrap().pos
        ),
        Err(SimError::PlacementOverlap)
    );
    assert_eq!(
        placement_check(&fx.world, PLAYER, building, Pos::flat(100, 100)),
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
        text: None,
        facing: None,
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
        .find(|b| b.state.enabled() && b.name == name)
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
            entity: None,
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
            entity: None,
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
        single_planet: true,
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
        .and_then(|p| p.buttons.iter().find(|b| b.state.enabled()))
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
                entity: None,
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

#[test]
fn right_click_on_a_body_orders_orbit_and_on_a_system_orders_travel() {
    let (content, mut game) = crate::match_setup::universe_game_for_tests();
    let home = game.world.home_body(PLAYER).expect("home");
    let system = game.world.body(home).unwrap().system;
    let rules = game.world.rules().clone();
    let ship_type = rules.id_of("unit.survey_ship").expect("ship");
    let star = game.world.space_of(system).expect("star");
    let mut at = game.world.body_position(star).expect("position");
    at.x += 4000;
    let ship = game.world.spawn(ship_type, PLAYER, at).expect("ship");
    game.world.step();
    let mut ctl = Controller::new();
    ctl.set_view(ViewCtx {
        planet: home,
        system: Some(system),
        in_space: true,
        frame: crate::space_view::SpaceFrame::of(&game.world, system),
    });
    ctl.handle(
        &mut game.world,
        PickEvent::Click {
            world_point: (0.0, 0.0),
            surface: SurfacePos {
                face: 0,
                x: 0,
                y: 0,
            },
            entity: Some(ship.0),
            modifiers: Modifiers::default(),
            double: false,
            on_screen: Vec::new(),
        },
    );
    assert_eq!(ctl.selection(), &[ship]);
    let rock = game
        .world
        .bodies()
        .iter()
        .find(|b| b.system == system && b.kind == rom_rules::BodyKind::Resource)
        .expect("a resource body")
        .id;
    let right = |body, system| PickEvent::SpaceRightClick {
        surface: SurfacePos {
            face: 0,
            x: 0,
            y: 0,
        },
        entity: None,
        body,
        system,
        modifiers: Modifiers::default(),
    };
    ctl.handle(&mut game.world, right(Some(rock.0), None));
    game.world.step();
    let order = game
        .world
        .entity(ship)
        .unwrap()
        .orders()
        .front()
        .unwrap()
        .clone();
    assert_eq!(order.kind, "orbit");
    assert_eq!(order.target_entity, game.world.body_marker(rock));
    let neighbour = content
        .universe
        .routes
        .iter()
        .find_map(|r| {
            (r.system_a == system)
                .then_some(r.system_b)
                .or((r.system_b == system).then_some(r.system_a))
        })
        .expect("a neighbouring system");
    ctl.handle(&mut game.world, right(None, Some(neighbour)));
    game.world.step();
    let order = game
        .world
        .entity(ship)
        .unwrap()
        .orders()
        .front()
        .unwrap()
        .clone();
    assert_eq!(order.kind, "travel");
    assert_eq!(
        order.target_pos.unwrap().body,
        game.world.space_of(neighbour).unwrap()
    );
    // On empty space the ship moves.
    ctl.handle(
        &mut game.world,
        PickEvent::SpaceRightClick {
            surface: SurfacePos {
                face: 0,
                x: 500_000,
                y: 0,
            },
            entity: None,
            body: None,
            system: None,
            modifiers: Modifiers::default(),
        },
    );
    game.world.step();
    assert_eq!(
        game.world
            .entity(ship)
            .unwrap()
            .orders()
            .front()
            .unwrap()
            .kind,
        "move"
    );
}

/// Task 111: with the view in another system, a right click on a point of its plane or on one of
/// its bodies sends the selected ship there along the routes (the simulation decides whether it
/// comes out there or at the edge).
#[test]
fn right_click_in_another_system_orders_travel_to_that_point() {
    let (content, mut game) = crate::match_setup::universe_game_for_tests();
    let home = game.world.home_body(PLAYER).expect("home");
    let system = game.world.body(home).unwrap().system;
    let ship_type = game.world.rules().id_of("unit.survey_ship").unwrap();
    let star = game.world.space_of(system).unwrap();
    let mut at = game.world.body_position(star).unwrap();
    at.x += 4000;
    let ship = game.world.spawn(ship_type, PLAYER, at).unwrap();
    game.world.step();
    let other = content
        .universe
        .routes
        .iter()
        .find_map(|r| {
            (r.system_a == system)
                .then_some(r.system_b)
                .or((r.system_b == system).then_some(r.system_a))
        })
        .unwrap();
    let frame = crate::space_view::SpaceFrame::of(&game.world, other).unwrap();
    let mut ctl = Controller::new();
    ctl.set_view(ViewCtx {
        planet: home,
        system: Some(other),
        in_space: true,
        frame: Some(frame),
    });
    ctl.handle(
        &mut game.world,
        PickEvent::BoxSelect {
            entities: vec![ship.0],
            modifiers: Modifiers::default(),
        },
    );
    assert_eq!(ctl.selection(), &[ship]);
    let front = |world: &World| {
        world
            .entity(ship)
            .unwrap()
            .orders()
            .front()
            .cloned()
            .unwrap()
    };
    let click = SurfacePos {
        face: 0,
        x: 30_000,
        y: -20_000,
    };
    ctl.handle(
        &mut game.world,
        PickEvent::SpaceRightClick {
            surface: click,
            entity: None,
            body: None,
            system: None,
            modifiers: Modifiers::default(),
        },
    );
    game.world.step();
    let order = front(&game.world);
    assert_eq!(order.kind, "travel");
    assert_eq!(order.target_pos, Some(frame.pos_of(30.0, -20.0)));
    // A body of that system: the ship is sent beside it.
    let planet = game
        .world
        .bodies()
        .iter()
        .find(|b| b.system == other && b.kind != rom_rules::BodyKind::Star)
        .unwrap()
        .id;
    ctl.handle(
        &mut game.world,
        PickEvent::SpaceRightClick {
            surface: click,
            entity: None,
            body: Some(planet.0),
            system: None,
            modifiers: Modifiers::default(),
        },
    );
    game.world.step();
    let order = front(&game.world);
    assert_eq!(order.kind, "travel");
    assert_eq!(order.target_pos, game.world.body_position(planet));
}

fn pos_of(surface: SurfacePos) -> Pos {
    Pos {
        x: surface.x as i32,
        y: surface.y as i32,
        face: surface.face,
        ..Pos::default()
    }
}

#[test]
fn button_states_follow_rules_and_stock() {
    let mut fx = fixture();
    let civic = fx.rules.id_of("tech.civic.1").unwrap();
    assert_eq!(
        crate::hud::button_state(&fx.world, PLAYER, civic, false),
        ButtonState::Ready
    );
    let poor = fixture_with(0);
    assert!(
        matches!(crate::hud::button_state(&poor.world, PLAYER, civic, false),
        ButtonState::Unaffordable { missing } if missing.len() == 2 && missing.iter().all(|(_, n)| *n == 200))
    );
    let locked = fx.rules.id_of("tech.civic.2").unwrap();
    assert!(
        matches!(crate::hud::button_state(&fx.world, PLAYER, locked, false),
        ButtonState::Locked { reason } if reason.contains("Classical"))
    );
    let done = fixture_config(0, "age.ancient", &["tech.civic.1"]);
    assert_eq!(
        crate::hud::button_state(&done.world, PLAYER, civic, false),
        ButtonState::Done
    );
    let library = fx.spawn(fx.rules.id_of("bld.library").unwrap(), 1, 20, 20);
    fx.click(library, false);
    fx.ctl.tab = PaletteTab::Research;
    // Civic level 1 is the third cell of the first row.
    fx.hud(HudRegionKind::CommandSlot, 2, PointerButton::Left, false);
    let civic_button = |fx: &Fx| {
        let palette = fx.hud_snapshot().palette.unwrap();
        palette.buttons.into_iter().find(|b| b.slot == 2).unwrap()
    };
    // Submission is already reflected before the public queue exposes it.
    assert_eq!(civic_button(&fx).state, ButtonState::InProgress { pct: 0 });
    fx.step(5);
    assert!(
        matches!(crate::hud::button_state(&fx.world, PLAYER, civic, false),
        ButtonState::InProgress { pct } if pct > 0 && pct < 100)
    );
    assert!(!civic_button(&fx).state.enabled());
}

/// Task 113: in a network match a research ordered shows as started from the click until its
/// order comes back in the server's turn, not only for the tick of the click; one that never
/// comes back (refused) shows as ready again after two seconds.
#[test]
fn a_research_sent_to_the_server_shows_as_started_until_it_returns() {
    let mut fx = fixture();
    fx.ctl.set_network();
    let library = fx.spawn(fx.rules.id_of("bld.library").unwrap(), 1, 20, 20);
    fx.click(library, false);
    fx.ctl.tab = PaletteTab::Research;
    let civic_button = |fx: &Fx| {
        let palette = fx.hud_snapshot().palette.unwrap();
        palette.buttons.into_iter().find(|b| b.slot == 2).unwrap()
    };
    let research = |fx: &mut Fx| {
        let mut outbox = Outbox::default();
        fx.ctl.handle_with(
            &mut OrderSink::Follow {
                world: &fx.world,
                outbox: &mut outbox,
            },
            PickEvent::HudClick {
                region: HudRegionKind::CommandSlot,
                index: 2,
                button: PointerButton::Left,
                modifiers: Modifiers::default(),
            },
        );
        outbox.orders
    };
    let follow = |fx: &mut Fx, n: usize| {
        for _ in 0..n {
            fx.world.step();
            let mut outbox = Outbox::default();
            fx.ctl.after_step_with(&mut OrderSink::Follow {
                world: &fx.world,
                outbox: &mut outbox,
            });
        }
    };
    let sent = research(&mut fx);
    assert_eq!(sent.len(), 1);
    assert_eq!(civic_button(&fx).state, ButtonState::InProgress { pct: 0 });
    // Two turns pass before the server's turn with the order arrives: still started.
    follow(&mut fx, 2);
    assert_eq!(civic_button(&fx).state, ButtonState::InProgress { pct: 0 });
    // The turn applies it: the world shows the research from then on.
    fx.world.submit(sent[0].clone()).unwrap();
    follow(&mut fx, 5);
    assert!(
        matches!(civic_button(&fx).state, ButtonState::InProgress { pct } if pct > 0),
        "{:?}",
        civic_button(&fx).state
    );
    // A research the server refuses is ready again once the wait is over.
    let mut refused = fixture();
    refused.ctl.set_network();
    let library = refused.spawn(refused.rules.id_of("bld.library").unwrap(), 1, 20, 20);
    refused.click(library, false);
    refused.ctl.tab = PaletteTab::Research;
    assert_eq!(research(&mut refused).len(), 1);
    assert_eq!(
        civic_button(&refused).state,
        ButtonState::InProgress { pct: 0 }
    );
    follow(&mut refused, 40);
    assert_eq!(
        civic_button(&refused).state,
        ButtonState::InProgress { pct: 0 }
    );
    follow(&mut refused, 1);
    assert_eq!(civic_button(&refused).state, ButtonState::Ready);
}

#[test]
fn library_shows_the_full_grid_with_done_badges() {
    let mut fx = fixture_config(10_000, "age.ancient", &["tech.civic.1", "tech.military.1"]);
    let library = fx.spawn(fx.rules.id_of("bld.library").unwrap(), 1, 20, 20);
    fx.click(library, false);
    fx.ctl.tab = PaletteTab::Research;
    let (palette, actions) = fx.ctl.palette(&fx.world).unwrap();
    // Three age rows of five cells: the age advance, then Military, Civic, Commerce, Science.
    assert_eq!(actions.len(), 15);
    let at = |row: usize, col: usize| {
        let Some((_, Action::Research(d))) = actions.iter().find(|(s, _)| *s == row * 5 + col)
        else {
            panic!("no cell at row {row}, column {col}");
        };
        *d
    };
    let key = |d: DefId| fx.rules.def(d).key.as_str().to_owned();
    for row in 0..3 {
        let level = row + 1;
        assert_eq!(fx.rules.techs[&at(row, 0)].kind, rom_rules::TechKind::Age);
        assert_eq!(fx.rules.techs[&at(row, 0)].level, level as i64);
        for (col, track) in ["military", "civic", "commerce", "science"]
            .iter()
            .enumerate()
        {
            assert_eq!(key(at(row, col + 1)), format!("tech.{track}.{level}"));
        }
    }
    let state = |slot: usize| {
        &palette
            .buttons
            .iter()
            .find(|b| b.slot == slot)
            .unwrap()
            .state
    };
    // Researched items stay visible with the done badge.
    assert_eq!(state(1), &ButtonState::Done);
    assert_eq!(state(2), &ButtonState::Done);
    assert!(matches!(state(3), ButtonState::Ready));
    // The age advance is locked until its row is done; later rows are locked too.
    assert!(matches!(state(0), ButtonState::Locked { .. }));
    assert!(matches!(state(5), ButtonState::Locked { .. }));
    // With every track level of row one done, the age advance opens.
    let mut fx = fixture_config(
        10_000,
        "age.ancient",
        &[
            "tech.military.1",
            "tech.civic.1",
            "tech.commerce.1",
            "tech.science.1",
        ],
    );
    let library = fx.spawn(fx.rules.id_of("bld.library").unwrap(), 1, 20, 20);
    fx.click(library, false);
    fx.ctl.tab = PaletteTab::Research;
    let (palette, _) = fx.ctl.palette(&fx.world).unwrap();
    assert!(matches!(
        palette.buttons.iter().find(|b| b.slot == 0).unwrap().state,
        ButtonState::Ready
    ));
}

#[test]
fn library_opens_on_the_current_age_page() {
    let page_of = |age: &str| {
        let mut fx = fixture_config(10_000, age, &[]);
        let library = fx.spawn(fx.rules.id_of("bld.library").unwrap(), 1, 20, 20);
        fx.click(library, false);
        fx.ctl.tab = PaletteTab::Research;
        let (palette, actions) = fx.ctl.palette(&fx.world).unwrap();
        let next_age = actions.iter().find_map(|(slot, a)| match a {
            Action::Research(d)
                if *slot % 5 == 0 && fx.rules.techs[d].kind == rom_rules::TechKind::Age =>
            {
                Some(fx.rules.techs[d].level)
            }
            _ => None,
        });
        (palette.page, palette.pages, next_age)
    };
    // Ages 0-2 are working towards levels 1-3 on the first page, ages 3-5 towards 4-6 on the next.
    assert_eq!(page_of("age.ancient").0, 0);
    assert_eq!(page_of("age.medieval").0, 0);
    let gunpowder = page_of("age.gunpowder");
    assert_eq!(gunpowder.0, 1);
    assert_eq!(gunpowder.2, Some(4));
    assert!(gunpowder.1 > 2);
    assert_eq!(page_of("age.information").0, 2);
    // Arrows still turn pages, and a chosen page is kept.
    let mut fx = fixture();
    let library = fx.spawn(fx.rules.id_of("bld.library").unwrap(), 1, 20, 20);
    fx.click(library, false);
    fx.ctl.tab = PaletteTab::Research;
    fx.hud(HudRegionKind::PaletteArrow, 1, PointerButton::Left, false);
    assert_eq!(fx.ctl.palette(&fx.world).unwrap().0.page, 1);
}

#[test]
fn obsolete_steps_are_hidden() {
    let rules = crate::terrain::load_rules().unwrap();
    let keys: Vec<_> = rules
        .techs
        .keys()
        .map(|id| rules.def(*id).key.as_str())
        .collect();
    let mut fx = fixture_config(10_000, "age.galactic", &keys);
    let pt = fx.rules.id_of("bld.barracks").unwrap();
    let producer = fx.spawn(pt, 1, 20, 20);
    fx.click(producer, false);
    fx.ctl.tab = PaletteTab::Train;
    let (_, actions) = fx.ctl.palette(&fx.world).unwrap();
    let player = fx.world.players().find(|p| p.id == PLAYER).unwrap();
    let active = player.active_sources();
    let owned = fx.world.owned_types(PLAYER);
    let train: Vec<_> = actions
        .iter()
        .filter_map(|(_, a)| {
            if let Action::Train(d) = a {
                Some(*d)
            } else {
                None
            }
        })
        .collect();
    assert!(!train.is_empty());
    assert!(
        train
            .iter()
            .all(|d| !fx.rules.is_obsolete(*d, &active, &owned)
                && fx.rules.is_unlocked(*d, &active, &owned))
    );
    assert!(
        fx.rules.production[&pt]
            .iter()
            .any(|r| r.mode == ProductionMode::Train
                && fx.rules.is_obsolete(r.product_def_id, &active, &owned)
                && !train.contains(&r.product_def_id))
    );
    // At the start, locked successors are absent even when the current step is unaffordable.
    let mut early = fixture_with(0);
    let producer = early.spawn(pt, 1, 20, 20);
    early.click(producer, false);
    early.ctl.tab = PaletteTab::Train;
    assert!(
        early
            .hud_snapshot()
            .palette
            .unwrap()
            .buttons
            .iter()
            .all(|b| matches!(
                b.state,
                ButtonState::Unaffordable { .. } | ButtonState::Locked { .. }
            ))
    );
    // A line with no unlocked step still previews its first step in greyscale.
    let future = early.rules.id_of("bld.airfield").unwrap();
    let producer = early.spawn(future, 1, 40, 40);
    early.click(producer, false);
    early.ctl.tab = PaletteTab::Train;
    let (palette, actions) = early.ctl.palette(&early.world).unwrap();
    assert!(!palette.buttons.is_empty());
    assert!(
        palette
            .buttons
            .iter()
            .all(|b| matches!(b.state, ButtonState::Locked { .. }))
    );
    for (_, action) in actions {
        if let Action::Train(def) = action {
            assert!(early.rules.entity_types[&def].upgrades_from_id.is_none());
        }
    }
}

#[test]
fn locked_buildings_are_shown_greyscale() {
    let mut fx = fixture();
    let b = fx.spawn(fx.mover(), 1, 10, 10);
    fx.click(b, false);
    let pages = fx.hud_snapshot().palette.unwrap().pages;
    let mut locked = 0;
    let mut shown = 0;
    for page in 0..pages {
        fx.ctl.page = Some(page);
        for button in fx.hud_snapshot().palette.unwrap().buttons {
            shown += 1;
            if let ButtonState::Locked { reason } = button.state {
                assert!(reason.starts_with("Needs"));
                locked += 1;
            }
        }
    }
    // The renderer consumes Locked as desaturated artwork, not just a dark tint.
    let layout = rom_render::hud_layout::HudLayout::new(1672, 941, None);
    let mut painter = rom_render::hud::HudPainter::bake(1.0).unwrap();
    let snapshot = fx.hud_snapshot();
    assert!(
        painter
            .compose(&layout, &snapshot)
            .iter()
            .any(|q| matches!(q.texture, rom_render::hud::HudTexture::Greyscale))
    );
    assert!(locked > 0);
    assert_eq!(
        shown,
        fx.rules.production[&fx.mover()]
            .iter()
            .filter(|r| r.mode == ProductionMode::Construct)
            .count()
    );
}

#[test]
fn research_overview_marks_done_next_and_locked() {
    use rom_render::ResearchCell;
    let mut fx = fixture_config(10_000, "age.ancient", &["tech.civic.1"]);
    let library = fx.spawn(fx.rules.id_of("bld.library").unwrap(), 1, 20, 20);
    fx.click(library, false);
    fx.ctl.tab = PaletteTab::Research;
    fx.hud(HudRegionKind::CommandSlot, 3, PointerButton::Left, false);
    fx.hud(HudRegionKind::DataTab, 4, PointerButton::Left, false);
    let hud = fx.hud_snapshot();
    assert_eq!(hud.data_tab, 4);
    assert_eq!(hud.overview.tracks.len(), 4);
    assert!(hud.overview.tracks.iter().all(|t| t.levels.len() == 13));
    assert_eq!(
        &hud.overview.tracks[0].levels[..3],
        &[ResearchCell::Done, ResearchCell::Next, ResearchCell::Locked]
    );
    assert_eq!(hud.overview.tracks[1].levels[0], ResearchCell::InProgress);
    assert_eq!(hud.overview.ages.len(), 14);
    assert_eq!(
        &hud.overview.ages[..3],
        &[ResearchCell::Done, ResearchCell::Next, ResearchCell::Locked]
    );
}

#[test]
fn rejection_messages_name_the_rule() {
    let fx = fixture();
    assert_eq!(
        rejection_message(&fx.world, PLAYER, &SimError::CitiesTooClose, None),
        "Too close to another city (20 tiles)"
    );
    assert_eq!(
        rejection_message(&fx.world, PLAYER, &SimError::NotInOwnTerritory, None),
        "Outside your borders"
    );
    let barracks = fx.rules.id_of("bld.barracks").unwrap();
    assert_eq!(
        rejection_message(&fx.world, PLAYER, &SimError::Locked, Some(barracks)),
        "Needs Militia"
    );
    assert_eq!(
        rejection_message(&fx.world, PLAYER, &SimError::NotInCityRadius, None),
        "Outside a city's economic radius"
    );
    let mut fx = fixture_config(10_000, "age.ancient", &["tech.civic.1"]);
    let city_type = fx.rules.id_of("bld.city").unwrap();
    fx.spawn(city_type, 1, 20, 20);
    let builder = fx.spawn(fx.mover(), 1, 10, 10);
    fx.click(builder, false);
    fx.hud(HudRegionKind::CommandSlot, 0, PointerButton::Left, false);
    fx.ctl
        .place(&mut OrderSink::Lead(&mut fx.world), at(30, 20), false);
    assert_eq!(
        fx.hud_snapshot().events.last().unwrap(),
        "Too close to another city (20 tiles)"
    );
    fx.ctl
        .place(&mut OrderSink::Lead(&mut fx.world), at(60, 60), false);
    assert_eq!(
        fx.hud_snapshot().events.last().unwrap(),
        "Outside your borders"
    );
}

#[test]
fn task_30_scripts_reach_the_capture_states_without_a_window() {
    use crate::script::{Script, ScriptAction, ScriptContext, ScriptSetup};
    let options = crate::args::Options::default();
    let content = crate::terrain::generate_content(&options).unwrap();
    for (text, city_script) in [
        (
            include_str!("../../../../docs/tasks/30-palette-script.txt"),
            false,
        ),
        (
            include_str!("../../../../docs/tasks/30-city-script.txt"),
            true,
        ),
    ] {
        let mut game =
            crate::match_setup::build_match(&content.rules, &content.map, &content.factions, 1)
                .unwrap();
        ScriptSetup::scan(text)
            .unwrap()
            .apply_grants(&mut game.world)
            .unwrap();
        let start = &game.starts[0];
        let context = ScriptContext::of(start);
        let mut script = Script::parse(text, &context, &|dx, dy| {
            let pos = game.world.offset(start.city_pos, dx, dy);
            SurfacePos {
                face: pos.face,
                x: i64::from(pos.x),
                y: i64::from(pos.y),
            }
        })
        .unwrap();
        let mut ctl = Controller::new();
        for _ in 0..710 {
            for action in script.due_actions(game.world.tick()) {
                match action {
                    ScriptAction::SelectResearchers => ctl.select_researchers(&game.world),
                    ScriptAction::SelectCraft => ctl.select_own_craft(&game.world),
                    ScriptAction::SelectProducers => ctl.select_producers(&game.world),
                    ScriptAction::SelectShips => ctl.select_own_ships(&game.world),
                    // No scene here boxes near a live name (task 140).
                    ScriptAction::BoxNear { .. } => {}
                    ScriptAction::EntityPick {
                        target,
                        right,
                        shift,
                    } => match crate::script::live_pick(&game.world, PLAYER, &target, right, shift)
                    {
                        Ok(pick) => ctl.handle(&mut game.world, pick),
                        Err(error) => ctl.notify(&game.world, error),
                    },
                    ScriptAction::Speed(_)
                    | ScriptAction::Input(_)
                    | ScriptAction::PointAt { .. }
                    | ScriptAction::Save(_)
                    | ScriptAction::QuickSave
                    | ScriptAction::Menu(_)
                    | ScriptAction::Ascend(_)
                    | ScriptAction::Chat(..)
                    | ScriptAction::ChatFrom(..)
                    | ScriptAction::ChatKeys(_)
                    | ScriptAction::Visit(_)
                    | ScriptAction::Look(_) => {}
                }
            }
            for event in script.due(game.world.tick()) {
                ctl.handle(&mut game.world, event);
            }
            game.world.step();
            ctl.after_step(&mut game.world);
        }
        let mut hud = HudSnapshot::default();
        ctl.fill(&game.world, &mut hud);
        if city_script {
            let scene = crate::terrain::to_scene(&content.map);
            let camera = rom_render::home_camera(&scene);
            let surface = rom_render::Surface::new(Arc::new(scene));
            let layout = rom_render::hud_layout::HudLayout::new(1672, 941, None);
            let view = rom_render::pick::canvas_view(&camera, &layout, true);
            let preview = hud.placement.as_ref().expect("placement preview");
            // Keep the full contour and four tiles of ghost clearance visible in the capture.
            for dx in [-0.5, 0.5] {
                for dy in [-0.5, 0.5] {
                    let x = preview.x as f32 / 1000.0 + dx * f32::from(preview.width);
                    let y = preview.y as f32 / 1000.0 + dy * f32::from(preview.height);
                    for lift in [0.0, 4.0] {
                        let (px, py, _) = view
                            .project([x, surface.height(x, y) + lift, y])
                            .expect("placement in front of camera");
                        assert!(
                            !layout.contains_hud(px, py) && layout.world_area().contains(px, py),
                            "placement is hidden by the HUD at {px}, {py}"
                        );
                    }
                }
            }
            assert_eq!(
                hud.placement.as_ref().map(|p| p.valid),
                Some(Some(true)),
                "{hud:?}"
            );
            // The circle shows the grown radius: the stat plus the building bonus.
            let grown: Vec<i64> = game
                .world
                .entities()
                .filter(|e| e.owner == PLAYER)
                .map(|e| game.world.city_economic_radius(e.id))
                .filter(|&r| r > 16_000)
                .collect();
            assert!(
                hud.city_radii
                    .iter()
                    .any(|c| grown.contains(&c.economic) && c.spacing == 20_000 && c.minimap),
                "{:?} vs {grown:?}",
                hud.city_radii
            );
        } else {
            let palette = hud.palette.as_ref().expect("library selected");
            assert_eq!(palette.tab, PaletteTab::Research);
            assert!(
                palette
                    .buttons
                    .iter()
                    .any(|b| matches!(b.state, ButtonState::InProgress { pct } if pct > 0))
            );
            assert!(
                palette
                    .buttons
                    .iter()
                    .any(|b| matches!(b.state, ButtonState::Unaffordable { .. }))
            );
            assert!(
                palette
                    .buttons
                    .iter()
                    .any(|b| matches!(b.state, ButtonState::Locked { .. }))
            );
            assert!(
                hud.overview
                    .tracks
                    .iter()
                    .any(|t| t.levels.contains(&rom_render::ResearchCell::Done))
            );
            assert_eq!(hud.data_tab, 4);
        }
    }
}

#[test]
fn prerequisite_help_names_only_missing_tracks() {
    let fx = fixture_config(10_000, "age.ancient", &["tech.military.1"]);
    let advance = fx.rules.id_of("tech.age.classical").unwrap();
    let reason = crate::hud::unmet(&fx.world, PLAYER, advance);
    assert!(!reason.contains("Militia"));
    assert!(
        ["Chiefdom", "Barter", "Writing"]
            .iter()
            .any(|n| reason.contains(n))
    );
    let fx = fixture_config(
        10_000,
        "age.ancient",
        &[
            "tech.military.1",
            "tech.civic.1",
            "tech.commerce.1",
            "tech.science.1",
        ],
    );
    assert_eq!(
        crate::hud::button_state(&fx.world, PLAYER, advance, false),
        ButtonState::Ready
    );
}

fn wealth_fixture_config(stock: i64, age: &str, techs: &[&str]) -> Fx {
    let mut fx = fixture_config(stock, age, techs);
    fx.world = fx
        .world
        .with_match_options(rom_sim::MatchOptions {
            victories: Default::default(),
            ..Default::default()
        })
        .unwrap();
    fx
}

fn wealth_fixture() -> Fx {
    wealth_fixture_config(10_000, "age.ancient", &[])
}

fn wealth_units(fx: &mut Fx) -> (EntityId, EntityId, EntityId) {
    let relic_type = fx.find(|et| {
        et.node
            .as_ref()
            .is_some_and(|n| n.claim_mode == rom_rules::ClaimMode::Carry)
    });
    let tag = fx.rules.entity_types[&relic_type]
        .node
        .as_ref()
        .unwrap()
        .claim_tag_id
        .unwrap();
    let priest_type = fx.find(|et| et.class == EntityClass::Unit && fx.rules.has_tag(et.id, tag));
    let trader_type = fx.find(|et| et.trader.as_ref().is_some_and(|t| t.runs_routes));
    let relic = fx.spawn(relic_type, 0, 12, 10);
    let priest = fx.spawn(priest_type, 1, 10, 10);
    let trader = fx.spawn(trader_type, 1, 15, 20);
    (relic, priest, trader)
}

fn wealth_cities(fx: &mut Fx) -> (EntityId, EntityId) {
    let city = fx.find(|et| et.territory.as_ref().is_some_and(|t| t.is_city));
    (fx.spawn(city, 1, 20, 20), fx.spawn(city, 1, 50, 20))
}

#[test]
fn right_click_relic_with_priest_issues_carry() {
    let mut fx = wealth_fixture();
    let (relic, priest, _) = wealth_units(&mut fx);
    fx.click(priest, false);
    fx.ctl.hovered = Some(relic);
    assert_eq!(fx.hud_snapshot().help, "Carry to a temple");
    fx.rclick(Some(relic), (12.5, 10.5), false);
    fx.step(1);
    let order = fx.world.entity(priest).unwrap().orders().front().unwrap();
    assert_eq!(order.kind, "carry");
    assert_eq!(order.target_entity, Some(relic));
    assert_eq!(fx.hud_snapshot().markers[0].kind, MarkerKind::Move);
    // A second carry order joins the existing order when Shift is held.
    let relic_type = fx.world.entity(relic).unwrap().type_id;
    let next = fx.spawn(relic_type, 0, 15, 10);
    fx.rclick(Some(next), (15.5, 10.5), true);
    fx.step(1);
    assert_eq!(fx.world.entity(priest).unwrap().orders().len(), 2);
}

#[test]
fn right_click_city_with_trader_issues_trade() {
    let mut fx = wealth_fixture();
    let (_, _, trader) = wealth_units(&mut fx);
    let (home, target) = wealth_cities(&mut fx);
    fx.click(trader, false);
    assert_eq!(
        crate::hud::wealth_card(&fx.world, fx.world.entity(trader).unwrap()),
        ["No route"]
    );
    fx.ctl.hovered = Some(target);
    let offer = fx.world.trade_offer(PLAYER, trader, target).unwrap();
    // Cities by their names (task 144).
    let there = fx.world.city_name(target).unwrap().to_owned();
    let here = fx.world.city_name(home).unwrap().to_owned();
    assert_eq!(
        fx.hud_snapshot().help,
        format!(
            "Trade route to {there}: +{} per trip",
            offer.per_trip_income
        )
    );
    fx.rclick(Some(target), (50.5, 20.5), false);
    fx.step(1);
    let route = fx.world.trade_routes(PLAYER).next().unwrap();
    assert_eq!((route.home, route.target), (home, target));
    assert!(
        fx.hud_snapshot()
            .help
            .contains(&format!("+{} per trip", route.per_trip_income))
    );
    let hud = fx.hud_snapshot();
    assert_eq!(hud.trade_routes.len(), 1);
    assert!(
        hud.trade_routes[0].to_target,
        "starts at home, heading to partner"
    );
    let HudSelection::Single(card) = hud.selection else {
        panic!("trader card")
    };
    assert_eq!(
        card.wealth[0],
        format!(
            "Route: {here} → {there}, +{} per trip",
            route.per_trip_income
        )
    );
    assert_eq!(card.wealth[1], format!("Next: walk to {there}"));
    // The direction changes on arrival, with no dependency on path detours.
    for _ in 0..1200 {
        fx.step(1);
        if !fx.hud_snapshot().trade_routes[0].to_target {
            break;
        }
    }
    assert!(!fx.hud_snapshot().trade_routes[0].to_target);
    fx.world.despawn(target).unwrap();
    fx.step(1);
    assert_eq!(
        fx.hud_snapshot().events.last().unwrap(),
        "Trade route ended: city lost"
    );
    let at = fx.ctl.message.as_ref().unwrap().1;
    fx.step(2);
    assert_eq!(fx.ctl.message.as_ref().unwrap().1, at);
}

/// Task 72: the Trade route button arms a pick; the help line says what to pick, valid
/// partners are ringed, any other city says why not, and clicking a valid one runs the route.
#[test]
fn trade_route_button_picks_a_partner_city() {
    let mut fx = wealth_fixture();
    let (_, _, trader) = wealth_units(&mut fx);
    let (home, target) = wealth_cities(&mut fx);
    // A city too near the home city for a route.
    let city = fx.find(|et| et.territory.as_ref().is_some_and(|t| t.is_city));
    let near = fx.spawn(city, 1, 28, 20);
    fx.click(trader, false);
    fx.ctl.tab = PaletteTab::Order;
    let palette = fx.hud_snapshot().palette.unwrap();
    let button = palette
        .buttons
        .iter()
        .find(|b| b.name == "Trade route")
        .expect("a Trade route button");
    assert!(!button.armed);
    let slot = button.slot;
    assert!(
        fx.hud_snapshot()
            .help
            .starts_with("Trade route, then another city;")
    );
    fx.hud(HudRegionKind::CommandSlot, slot, PointerButton::Left, false);
    let hud = fx.hud_snapshot();
    assert!(
        hud.palette
            .unwrap()
            .buttons
            .iter()
            .any(|b| b.slot == slot && b.armed)
    );
    assert_eq!(
        hud.help,
        "Pick another city of yours or an ally's for the trade route"
    );
    assert_eq!(hud.trade_targets, vec![target.0]);
    fx.ctl.hovered = Some(near);
    let help = fx.hud_snapshot().help;
    let name = fx.world.city_name(near).unwrap().to_owned();
    assert!(
        help.starts_with(&format!(
            "No trade route to {name}: too close for a trade route"
        )),
        "{help}"
    );
    // A click on the ground keeps the pick open; a click on the city takes the route.
    fx.ground_click();
    assert!(fx.ctl.trade_pick);
    fx.click(target, false);
    assert!(!fx.ctl.trade_pick);
    fx.step(1);
    let route = fx.world.trade_routes(PLAYER).next().unwrap();
    assert_eq!((route.home, route.target), (home, target));
    assert!(fx.hud_snapshot().trade_targets.is_empty());
}

#[test]
fn right_click_relic_with_soldier_moves() {
    let mut fx = wealth_fixture();
    let (relic, _, _) = wealth_units(&mut fx);
    let soldier = fx.spawn(fx.armed_other_than_builder(), 1, 10, 10);
    fx.click(soldier, false);
    fx.rclick(Some(relic), (12.5, 10.5), false);
    fx.step(1);
    let order = fx.world.entity(soldier).unwrap().orders().front().unwrap();
    assert_eq!(order.kind, "move");
    assert_eq!(order.target_pos, Some(at(12, 10)));
}

#[test]
fn wealth_rejections_name_the_rule() {
    let fx = wealth_fixture_config(10_000, "age.classical", &["tech.commerce.1"]);
    for (error, text) in [
        (SimError::CannotCarry, "This unit cannot carry that"),
        (SimError::CarrierFull, "Already carrying"),
        (SimError::NoHomeCity, "No home city on this world"),
        (
            SimError::RouteTooShort,
            "Too close for a trade route (20 tiles)",
        ),
        (SimError::NotATradePartner, "Not a trade partner"),
        (SimError::CaravanLimit, "Trade route limit reached (3)"),
    ] {
        assert_eq!(rejection_message(&fx.world, PLAYER, &error, None), text);
    }
}

#[test]
fn keeper_panel_counts_kept_relics() {
    let mut fx = wealth_fixture();
    let (relic, priest, _) = wealth_units(&mut fx);
    let keeper = fx.find(|et| et.keeper.is_some());
    let temple = fx.spawn(keeper, 1, 16, 10);
    fx.click(priest, false);
    fx.rclick(Some(relic), (12.5, 10.5), false);
    fx.step(400);
    fx.click(temple, false);
    assert_eq!(fx.world.kept_nodes(temple).count(), 1);
    let HudSelection::Single(card) = fx.hud_snapshot().selection else {
        panic!("keeper card")
    };
    assert_eq!(card.wealth, ["Relics 1 / 5", "+20 wealth per 30 s"]);
    let snapshot = crate::sim_thread::snapshot_of(&fx.world);
    let keeper = snapshot.entities.iter().find(|e| e.id == temple.0).unwrap();
    assert_eq!((keeper.kept, keeper.capacity), (1, 5));
    assert!(!snapshot.entities.iter().any(|e| e.id == relic.0));
    assert_eq!(crate::hud::snapshot_of(&fx.world, PLAYER).wealth.relics, 1);
}

#[test]
fn no_keeper_notice_appears_once_and_carried_node_is_hidden() {
    let mut fx = wealth_fixture();
    let (relic, priest, _) = wealth_units(&mut fx);
    fx.click(priest, false);
    fx.rclick(Some(relic), (12.5, 10.5), false);
    fx.step(100);
    assert_eq!(
        fx.hud_snapshot().events.last().unwrap(),
        "No temple with room for the relic"
    );
    let notified = fx.ctl.message.as_ref().unwrap().1;
    let snapshot = crate::sim_thread::snapshot_of(&fx.world);
    assert!(
        snapshot
            .entities
            .iter()
            .find(|e| e.id == priest.0)
            .unwrap()
            .carrying
    );
    assert!(!snapshot.entities.iter().any(|e| e.id == relic.0));
    fx.step(1200);
    assert_eq!(fx.ctl.message.as_ref().unwrap().1, notified);
}

#[test]
fn economy_tab_sums_routes() {
    let mut fx = wealth_fixture_config(10_000, "age.classical", &["tech.commerce.1"]);
    let (_, _, trader) = wealth_units(&mut fx);
    let (_, city) = wealth_cities(&mut fx);
    let trader2 = fx.spawn(fx.world.entity(trader).unwrap().type_id, 1, 15, 23);
    fx.select(&[trader, trader2]);
    fx.rclick(Some(city), (50.5, 20.5), false);
    fx.step(1);
    fx.hud(HudRegionKind::DataTab, 2, PointerButton::Left, false);
    let mut hud = crate::hud::snapshot_of(&fx.world, PLAYER);
    fx.ctl.fill(&fx.world, &mut hud);
    assert_eq!(hud.data_tab, 2);
    assert_eq!(hud.wealth.routes, 2);
    assert_eq!(hud.wealth.limit, 3);
    assert_eq!(hud.wealth.per_trip, 18);
}

#[test]
fn wealth_screenshot_script_reaches_capture_without_a_window() {
    use crate::script::{Script, ScriptAction, ScriptContext, ScriptSetup};
    let text = include_str!("../../../../docs/tasks/32-wealth-script.txt");
    let content = crate::terrain::generate_content(&crate::args::Options::default()).unwrap();
    let mut game =
        crate::match_setup::build_match(&content.rules, &content.map, &content.factions, 1)
            .unwrap();
    ScriptSetup::scan(text)
        .unwrap()
        .apply_grants(&mut game.world)
        .unwrap();
    let context = ScriptContext::of(&game.starts[0]);
    let city_pos = game.starts[0].city_pos;
    let mut script = Script::parse(text, &context, &|dx, dy| {
        let p = game.world.offset(city_pos, dx, dy);
        SurfacePos {
            face: p.face,
            x: i64::from(p.x),
            y: i64::from(p.y),
        }
    })
    .unwrap();
    let mut ctl = Controller::new();
    let mut messages = Vec::new();
    for _ in 0..8005 {
        for action in script.due_actions(game.world.tick()) {
            match action {
                ScriptAction::SelectResearchers => ctl.select_researchers(&game.world),
                ScriptAction::SelectCraft => ctl.select_own_craft(&game.world),
                ScriptAction::SelectProducers => ctl.select_producers(&game.world),
                ScriptAction::SelectShips => ctl.select_own_ships(&game.world),
                // No scene here boxes near a live name (task 140).
                ScriptAction::BoxNear { .. } => {}
                ScriptAction::EntityPick {
                    target,
                    right,
                    shift,
                } => {
                    let pick = crate::script::live_pick(&game.world, PLAYER, &target, right, shift)
                        .unwrap_or_else(|e| {
                            panic!("tick {}: {e}; notices {messages:?}", game.world.tick())
                        });
                    ctl.handle(&mut game.world, pick);
                }
                ScriptAction::Speed(_)
                | ScriptAction::Input(_)
                | ScriptAction::PointAt { .. }
                | ScriptAction::Save(_)
                | ScriptAction::QuickSave
                | ScriptAction::Menu(_)
                | ScriptAction::Ascend(_)
                | ScriptAction::Chat(..)
                | ScriptAction::ChatFrom(..)
                | ScriptAction::ChatKeys(_)
                | ScriptAction::Visit(_)
                | ScriptAction::Look(_) => {}
            }
        }
        for pick in script.due(game.world.tick()) {
            ctl.handle(&mut game.world, pick);
        }
        game.world.step();
        ctl.after_step(&mut game.world);
        if let Some(message) = &ctl.message
            && messages.last() != Some(message)
        {
            messages.push(message.clone());
        }
    }
    assert!(messages.is_empty(), "{messages:?}");
    let mut hud = crate::hud::snapshot_of(&game.world, PLAYER);
    ctl.fill(&game.world, &mut hud);
    println!(
        "wealth-script ticks={} cities={} relics_kept={} routes={} per_trip={}",
        game.world.tick(),
        hud.cities.0,
        hud.wealth.relics,
        hud.wealth.routes,
        hud.wealth.per_trip
    );
    assert_eq!(hud.wealth.routes, 1);
    assert!(hud.wealth.per_period > 0, "the trader's income is shown");
    assert_eq!(hud.cities.0, 2);
    assert_eq!(hud.data_tab, 2);
    assert_eq!(hud.trade_routes.len(), 1);
    assert!(hud.trade_routes[0].length >= 25_000);
    assert!(
        game.world
            .entities()
            .any(|e| e.owner == PLAYER && game.world.kept_nodes(e.id).next().is_some())
    );
    let HudSelection::Single(card) = hud.selection else {
        panic!("merchant selected")
    };
    assert!(card.wealth.iter().any(|s| s.starts_with("Route:")));
}

#[test]
fn allied_trade_and_mixed_selection_keep_move_fallback() {
    let mut fx = wealth_fixture();
    let mut players: Vec<_> = fx.world.players().cloned().collect();
    for player in &mut players {
        if player.id == PlayerId(2) {
            player.team = 1;
        }
    }
    fx.world = World::new(fx.rules.clone(), fx.world.map().clone(), 1, players)
        .unwrap()
        .with_match_options(rom_sim::MatchOptions {
            victories: Default::default(),
            ..Default::default()
        })
        .unwrap();
    let (relic, priest, trader) = wealth_units(&mut fx);
    let (home, target) = wealth_cities(&mut fx);
    fx.world.despawn(target).unwrap();
    let allied = fx.spawn(fx.world.entity(home).unwrap().type_id, 2, 50, 20);
    fx.select(&[priest, trader]);
    fx.rclick(Some(allied), (50.5, 20.5), true);
    fx.step(1);
    assert_eq!(
        fx.world
            .entity(trader)
            .unwrap()
            .orders()
            .front()
            .unwrap()
            .kind,
        "trade"
    );
    assert_eq!(
        fx.world
            .entity(priest)
            .unwrap()
            .orders()
            .front()
            .unwrap()
            .kind,
        "move"
    );
    // Replacing trade with a carry-node click makes the trader move and the carrier carry.
    fx.rclick(Some(relic), (12.5, 10.5), false);
    fx.step(1);
    assert_eq!(
        fx.world
            .entity(trader)
            .unwrap()
            .orders()
            .front()
            .unwrap()
            .kind,
        "move"
    );
    assert_eq!(
        fx.world
            .entity(priest)
            .unwrap()
            .orders()
            .front()
            .unwrap()
            .kind,
        "carry"
    );
    assert!(fx.hud_snapshot().events.is_empty());
}

#[test]
fn undiscovered_deposit_preview_names_the_required_age() {
    let mut fx = fixture();
    let well = fx.rules.id_of("bld.oil_well").unwrap();
    let age = fx.rules.id_of("age.industrial").unwrap();
    assert_eq!(
        placement_check(&fx.world, PLAYER, well, at(24, 24)),
        Err(SimError::ResourceAgeRequired(age))
    );
    fx.ctl.placement = Some(Placement {
        def: well,
        footprint: (2, 2),
        centre: Some(at(24, 24)),
        valid: None,
        dirty: true,
        checked: None,
        checked_age: None,
    });
    fx.ctl.refresh_placement(&fx.world);
    assert_eq!(fx.ctl.placement.as_ref().unwrap().valid, Some(false));
    let mut hud = crate::hud::snapshot_of(&fx.world, PLAYER);
    fx.ctl.fill(&fx.world, &mut hud);
    assert_eq!(
        hud.help,
        format!("Deposit revealed in {}", fx.rules.def(age).name)
    );
}

#[test]
fn orbital_review_script_trains_launches_selects_and_bombards_through_picks() {
    use crate::script::{Script, ScriptAction, ScriptContext, ScriptSetup};
    let text = include_str!("../../../../docs/tasks/43-orbit-script.txt");
    let options = crate::args::Options {
        sphere: Some(64),
        players: 2,
        // Seed 2: the home planet of seed 1 puts these scripts' buildings across a face edge.
        seed: 2,
        ..crate::args::Options::default()
    };
    let content = crate::terrain::generate_universe_content(&options).unwrap();
    let setup = ScriptSetup::scan(text).unwrap();
    let mut game = crate::match_setup::build_universe_match(
        &content.rules,
        &content.universe,
        &content.factions,
        1,
        None,
        &setup,
    )
    .unwrap();
    let home = game.world.home_body(PLAYER).unwrap();
    let start = game.starts[0].clone();
    let context = ScriptContext::of(&start);
    let mut script = Script::parse(text, &context, &|dx, dy| {
        surface_of(game.world.offset(start.city_pos, dx, dy))
    })
    .unwrap();
    let mut ctl = Controller::new();
    ctl.set_view(ViewCtx {
        planet: home,
        ..ViewCtx::default()
    });
    let target = game
        .world
        .entities()
        .find(|e| e.owner == PlayerId(2) && e.pos.body == home)
        .unwrap()
        .id;
    let hp = game.world.entity(target).unwrap().hp;
    let mut saw_beam = false;
    for _ in 0..1400 {
        for action in script.due_actions(game.world.tick()) {
            match action {
                ScriptAction::SelectResearchers => ctl.select_researchers(&game.world),
                ScriptAction::SelectCraft => ctl.select_own_craft(&game.world),
                ScriptAction::SelectProducers => ctl.select_producers(&game.world),
                ScriptAction::SelectShips => ctl.select_own_ships(&game.world),
                // No scene here boxes near a live name (task 140).
                ScriptAction::BoxNear { .. } => {}
                ScriptAction::EntityPick {
                    target,
                    right,
                    shift,
                } => {
                    let pick = crate::script::live_pick(&game.world, PLAYER, &target, right, shift)
                        .unwrap();
                    ctl.handle(&mut game.world, pick);
                }
                ScriptAction::Speed(1) => {
                    let last = game
                        .world
                        .entities()
                        .find_map(|e| game.world.orbit(e.id).and_then(|o| o.last_shot));
                    println!("review capture tick={} shot={last:?}", game.world.tick());
                }
                ScriptAction::Speed(_)
                | ScriptAction::Input(_)
                | ScriptAction::PointAt { .. }
                | ScriptAction::Save(_)
                | ScriptAction::QuickSave
                | ScriptAction::Menu(_)
                | ScriptAction::Ascend(_)
                | ScriptAction::Chat(..)
                | ScriptAction::ChatFrom(..)
                | ScriptAction::ChatKeys(_)
                | ScriptAction::Visit(_)
                | ScriptAction::Look(_) => {}
            }
        }
        for event in script.due(game.world.tick()) {
            ctl.handle(&mut game.world, event);
        }
        game.world.step();
        ctl.after_step(&mut game.world);
        if game.world.tick() >= 1300 {
            let snapshot = crate::sim_thread::snapshot_on(
                &game.world,
                PLAYER,
                home,
                &game.world.visible_entities(PLAYER),
                &crate::looks::ModelBook::default(),
                &mut crate::sim_thread::Motions::default(),
            );
            saw_beam |= snapshot
                .entities
                .iter()
                .any(|e| e.altitude == 8.0 && e.shot.is_some());
        }
    }
    let ship = *ctl
        .selection()
        .first()
        .expect("script selects the orbital ship");
    let orbit = game
        .world
        .orbit(ship)
        .expect("training launched into orbit")
        .clone();
    assert_eq!(orbit.body, home);
    assert!(saw_beam, "render snapshots include actual bombard shots");
    assert!(
        game.world.entity(target).unwrap().hp < hp,
        "ground building took damage"
    );
    assert!(
        game.world
            .entity(ship)
            .unwrap()
            .orders()
            .iter()
            .any(|o| o.kind == "attack" && o.target_entity == Some(target))
    );
    // The attack-hotkey click takes the same ordinary attack path as right-click.
    ctl.attack_move = true;
    let pick = crate::script::live_pick(&game.world, PLAYER, "enemy1", false, false).unwrap();
    ctl.handle(&mut game.world, pick);
    assert!(
        game.world
            .entity(ship)
            .unwrap()
            .orders()
            .iter()
            .any(|o| o.kind == "attack" && o.target_entity == Some(target))
    );
    println!(
        "orbital review tick={} last_shot={:?} target_hp={} hash={:016x}",
        game.world.tick(),
        orbit.last_shot,
        game.world.entity(target).unwrap().hp,
        game.world.hash()
    );
}

/// A university with three scholars working in it.
fn university_with_workers() -> (Fx, EntityId, Vec<EntityId>) {
    let mut fx = fixture();
    let university = fx.rules.id_of("bld.university").unwrap();
    let scholar = fx.rules.id_of("unit.scholar").unwrap();
    let building = fx.spawn(university, 1, 20, 20);
    let workers: Vec<EntityId> = (0..3).map(|k| fx.spawn(scholar, 1, 16 + k, 24)).collect();
    for worker in &workers {
        assert!(fx.ctl.submit(
            &mut OrderSink::Lead(&mut fx.world),
            *worker,
            "harvest",
            None,
            Some(building),
            None,
            false
        ));
    }
    fx.step(400);
    assert_eq!(
        fx.world.occupants(building),
        workers,
        "all three are inside"
    );
    (fx, building, workers)
}

#[test]
fn workers_inside_show_in_the_building_panel_and_can_be_picked() {
    let (mut fx, building, workers) = university_with_workers();
    fx.click(building, false);
    let hud = fx.hud_snapshot();
    let HudSelection::Single(card) = &hud.selection else {
        panic!("one building selected");
    };
    assert_eq!(card.inside.len(), 3);
    assert_eq!(card.workers.as_deref(), Some("3 / 7"));
    // Clicking a portrait selects that worker; it stays inside until it is ordered.
    fx.hud(HudRegionKind::InsideWorker, 1, PointerButton::Left, false);
    assert_eq!(ids(&fx), vec![workers[1].0]);
    assert!(fx.world.entity(workers[1]).unwrap().is_inside());
    // Shift adds a second worker to the first.
    fx.click(building, false);
    fx.hud(HudRegionKind::InsideWorker, 0, PointerButton::Left, false);
    fx.click(building, false);
    fx.hud(HudRegionKind::InsideWorker, 2, PointerButton::Left, true);
    assert_eq!(
        ids(&fx),
        vec![workers[2].0],
        "shift keeps units, not the building"
    );
    // A double click selects every worker of that type inside.
    fx.click(building, false);
    fx.hud(
        HudRegionKind::InsideWorkerAll,
        0,
        PointerButton::Left,
        false,
    );
    assert_eq!(ids(&fx), workers.iter().map(|w| w.0).collect::<Vec<_>>());
    // An order given to one of them takes just that worker out.
    fx.rclick(None, (40.0, 40.0), false);
    fx.step(2);
    assert!(
        fx.world.occupants(building).is_empty(),
        "all three were selected and ordered"
    );
}

#[test]
fn release_button_and_right_click_apply_to_everyone_inside() {
    let (mut fx, building, workers) = university_with_workers();
    fx.click(building, false);
    let hud = fx.hud_snapshot();
    let palette = hud.palette.expect("palette of a building with workers");
    let release = palette
        .buttons
        .iter()
        .find(|b| b.name == "Release all workers")
        .expect("release button");
    assert_eq!(release.slot, 3, "hotkey R");
    // The hotkey works.
    fx.ctl.handle(
        &mut fx.world,
        PickEvent::Hotkey {
            ch: 'r',
            modifiers: Modifiers::default(),
        },
    );
    fx.step(2);
    assert!(fx.world.occupants(building).is_empty());
    for worker in &workers {
        assert!(!fx.world.entity(*worker).unwrap().is_inside());
    }
    // A right click with the building selected sends all its workers to the point.
    let (mut fx, building, workers) = university_with_workers();
    fx.click(building, false);
    fx.rclick(None, (40.0, 40.0), false);
    fx.step(2);
    assert!(fx.world.occupants(building).is_empty());
    assert!(workers.iter().all(|w| {
        fx.world
            .entity(*w)
            .unwrap()
            .orders()
            .iter()
            .any(|o| o.kind == "move")
    }));
}

#[test]
fn placing_a_city_on_another_planet_is_refused_with_a_message() {
    // A builder on another body cannot reach the site; the preview says so.
    let mut fx = fixture();
    let city = fx.rules.id_of("bld.city").unwrap();
    let builder = fx.mover();
    let worker = fx.spawn(builder, 1, 10, 10);
    fx.click(worker, false);
    let site = Pos {
        body: rom_sim::BodyId(7),
        ..at(30, 30)
    };
    assert!(!fx.ctl.builder_can_reach(&fx.world, city, site));
    assert!(fx.ctl.builder_can_reach(&fx.world, city, at(30, 30)));
    assert_eq!(
        rejection_message(&fx.world, PLAYER, &SimError::OtherBody, Some(city)),
        "No selected builder can reach this site (another planet)"
    );
}

/// Runs the task 54 review script without a window: the camera's flight to the planet the
/// transport orbits is stood in for by showing that planet when the script asks for focus.
fn colony_script_run() -> (crate::match_setup::Match, Controller, BodyId, u64) {
    use crate::script::{Script, ScriptAction, ScriptContext, ScriptSetup};
    let text = include_str!("../../../../docs/tasks/54-colony-script.txt");
    let options = crate::args::Options {
        sphere: Some(64),
        players: 2,
        // Seed 2: the home planet of seed 1 puts these scripts' buildings across a face edge.
        seed: 2,
        ..crate::args::Options::default()
    };
    let content = crate::terrain::generate_universe_content(&options).unwrap();
    let setup = ScriptSetup::scan(text).unwrap();
    let game = crate::match_setup::build_universe_match(
        &content.rules,
        &content.universe,
        &content.factions,
        1,
        None,
        &setup,
    )
    .unwrap();
    let home = game.world.home_body(PLAYER).unwrap();
    let system = game.world.body(home).unwrap().system;
    let start = game.starts[0].clone();
    let mut context = ScriptContext::of(&start);
    let at = game.world.body_position(home).unwrap();
    let mut planets: Vec<(u64, u32)> = game
        .world
        .bodies()
        .iter()
        .filter(|b| b.system == system && b.kind == rom_rules::BodyKind::Habitable && b.id != home)
        .map(|b| (at.distance(game.world.body_position(b.id).unwrap()), b.id.0))
        .collect();
    planets.sort_unstable();
    context.planets = planets.iter().map(|(_, id)| *id).collect();
    let colony = BodyId(context.planets[0]);
    let mut script = Script::parse(text, &context, &|dx, dy| {
        surface_of(game.world.offset(start.city_pos, dx, dy))
    })
    .unwrap();
    // The match's leader (task 85) makes the planets ships are bound for.
    let crate::match_setup::Match {
        world,
        starts,
        builder_type,
    } = game;
    let mut core = rom_match::MatchCore::new(
        world,
        crate::computer::ComputerPlayers::empty(),
        Some(content.universe),
    );
    let mut ctl = Controller::new();
    ctl.set_view(ViewCtx {
        planet: home,
        system: Some(system),
        ..ViewCtx::default()
    });
    for _ in 0..4100 {
        let world = core.world_mut();
        for action in script.due_actions(world.tick()) {
            match action {
                ScriptAction::SelectResearchers => ctl.select_researchers(world),
                ScriptAction::SelectCraft => ctl.select_own_craft(world),
                ScriptAction::SelectProducers => ctl.select_producers(world),
                ScriptAction::SelectShips => ctl.select_own_ships(world),
                // No scene here boxes near a live name (task 140).
                ScriptAction::BoxNear { .. } => {}
                ScriptAction::EntityPick {
                    target,
                    right,
                    shift,
                } => {
                    let pick =
                        crate::script::live_pick(world, PLAYER, &target, right, shift).unwrap();
                    ctl.handle(world, pick);
                }
                // The focus key: the camera comes down to the planet the selected ship orbits.
                ScriptAction::Input(_) => {
                    let below = ctl
                        .selection()
                        .iter()
                        .find_map(|id| world.orbit(*id).map(|o| o.body));
                    if let Some(planet) = below {
                        ctl.set_view(ViewCtx {
                            planet,
                            system: Some(system),
                            ..ViewCtx::default()
                        });
                    }
                }
                ScriptAction::Speed(_)
                | ScriptAction::Save(_)
                | ScriptAction::QuickSave
                | ScriptAction::Menu(_)
                | ScriptAction::Ascend(_)
                | ScriptAction::Chat(..)
                | ScriptAction::ChatFrom(..)
                | ScriptAction::ChatKeys(_)
                | ScriptAction::Visit(_)
                | ScriptAction::PointAt { .. }
                | ScriptAction::Look(_) => {}
            }
        }
        for event in script.due(world.tick()) {
            ctl.handle(world, event);
        }
        let outcome = core.lead(Vec::new());
        assert!(outcome.problems.is_empty(), "{:?}", outcome.problems);
        ctl.after_step(core.world_mut());
    }
    let (world, _, _) = core.into_parts();
    let hash = world.hash();
    let game = crate::match_setup::Match {
        world,
        starts,
        builder_type,
    };
    (game, ctl, colony, hash)
}

#[test]
fn colony_review_script_boards_lands_unloads_and_founds_a_city_through_picks() {
    let (game, ctl, colony, hash) = colony_script_run();
    let world = &game.world;
    let builders = &game.starts[0].builders;
    let transport = world
        .entities()
        .find(|e| e.owner == PLAYER && world.is_carrier_type(e.type_id) && world.is_landed(e.id))
        .expect("the transport landed");
    assert_eq!(transport.pos.body, colony);
    assert!(world.passengers(transport.id).is_empty());
    for builder in builders {
        let e = world.entity(*builder).unwrap();
        assert_eq!((e.pos.body, e.container_id), (colony, None));
    }
    let city = world
        .cities(PLAYER)
        .into_iter()
        .find(|c| world.entity(*c).unwrap().pos.body == colony)
        .expect("a city on the colony planet");
    println!(
        "colony review tick={} city={city:?} complete={} hash={hash:016x}",
        world.tick(),
        world.entity(city).unwrap().build_progress.is_none()
    );
    // The system view rings the colony in the player's colour.
    let system = world.body(colony).unwrap().system;
    let snapshot =
        crate::space_view::space_snapshot(world, system, &world.visible_entities(PLAYER));
    assert!(
        snapshot
            .colonies
            .iter()
            .any(|c| c.body == colony.0 && c.owner == 1)
    );
    let _ = ctl;
    assert_eq!(hash, colony_script_run().3);
}

#[test]
fn a_carrier_lists_its_passengers_and_offers_unload_and_lift_off() {
    let (mut game, mut ctl, colony, _) = colony_script_run();
    let transport = game
        .world
        .entities()
        .find(|e| e.owner == PLAYER && game.world.is_landed(e.id))
        .unwrap()
        .id;
    // Two units walk up to the landed transport and board it.
    let walkers: Vec<EntityId> = game.starts[0].builders[..2].to_vec();
    ctl.set_selection(walkers.clone());
    let landed_at = game.world.entity(transport).unwrap().pos;
    ctl.right_click(
        &mut OrderSink::Lead(&mut game.world),
        landed_at,
        Some(transport.0),
        Modifiers::default(),
    );
    for _ in 0..600 {
        game.world.step();
    }
    assert_eq!(game.world.passengers(transport), walkers);
    ctl.set_selection(vec![transport]);
    let mut hud = HudSnapshot::default();
    ctl.fill(&game.world, &mut hud);
    let HudSelection::Single(card) = &hud.selection else {
        panic!("one card");
    };
    assert_eq!(card.inside.len(), 2);
    assert_eq!(card.garrison.as_deref(), Some("2 / 20"));
    let (_, actions) = ctl.palette(&game.world).unwrap();
    assert!(actions.contains(&(4, Action::Unload)));
    assert!(actions.contains(&(5, Action::Lift)));
    // Unload sets them down; Lift off takes the ship back to orbit.
    ctl.press_slot(&mut OrderSink::Lead(&mut game.world), 4, false);
    game.world.step();
    assert!(game.world.passengers(transport).is_empty());
    ctl.press_slot(&mut OrderSink::Lead(&mut game.world), 5, false);
    for _ in 0..40 {
        game.world.step();
    }
    assert_eq!(
        game.world.orbit(transport).map(|o| o.body),
        Some(colony),
        "back in orbit"
    );
}

/// Replays a task 58 review script on a universe of two players with planets of the default
/// size for `ticks` ticks, calling `each` after every step with the planet view's render
/// snapshot.
fn orbital_combat_script(
    text: &str,
    ticks: u64,
    mut each: impl FnMut(&crate::match_setup::Match, &Controller, &rom_render::EntitySnapshot),
) -> (crate::match_setup::Match, Controller) {
    use crate::script::{Script, ScriptAction, ScriptContext, ScriptSetup};
    let options = crate::args::Options {
        players: 2,
        // Seed 2: with the real-sky universe (task 59) the home planet of seed 1 puts the
        // scripts' buildings across a face edge.
        seed: 2,
        ..crate::args::Options::default()
    };
    let content = crate::terrain::generate_universe_content(&options).unwrap();
    let setup = ScriptSetup::scan(text).unwrap();
    let mut game = crate::match_setup::build_universe_match(
        &content.rules,
        &content.universe,
        &content.factions,
        1,
        None,
        &setup,
    )
    .unwrap();
    let home = game.world.home_body(PLAYER).unwrap();
    let start = game.starts[0].clone();
    let context = ScriptContext::of(&start);
    let mut script = Script::parse(text, &context, &|dx, dy| {
        surface_of(game.world.offset(start.city_pos, dx, dy))
    })
    .unwrap();
    // Kept from one snapshot to the next, as the simulation thread keeps it.
    let mut motions = crate::sim_thread::Motions::default();
    let mut ctl = Controller::new();
    ctl.set_view(ViewCtx {
        planet: home,
        ..ViewCtx::default()
    });
    for _ in 0..ticks {
        for action in script.due_actions(game.world.tick()) {
            match action {
                ScriptAction::SelectResearchers => ctl.select_researchers(&game.world),
                ScriptAction::SelectCraft => ctl.select_own_craft(&game.world),
                ScriptAction::SelectProducers => ctl.select_producers(&game.world),
                ScriptAction::SelectShips => ctl.select_own_ships(&game.world),
                // No scene here boxes near a live name (task 140).
                ScriptAction::BoxNear { .. } => {}
                ScriptAction::EntityPick {
                    target,
                    right,
                    shift,
                } => {
                    let pick = crate::script::live_pick(&game.world, PLAYER, &target, right, shift)
                        .unwrap();
                    ctl.handle(&mut game.world, pick);
                }
                ScriptAction::Speed(_)
                | ScriptAction::Input(_)
                | ScriptAction::PointAt { .. }
                | ScriptAction::Save(_)
                | ScriptAction::QuickSave
                | ScriptAction::Menu(_)
                | ScriptAction::Ascend(_)
                | ScriptAction::Chat(..)
                | ScriptAction::ChatFrom(..)
                | ScriptAction::ChatKeys(_)
                | ScriptAction::Visit(_)
                | ScriptAction::Look(_) => {}
            }
        }
        for event in script.due(game.world.tick()) {
            ctl.handle(&mut game.world, event);
        }
        game.world.step();
        ctl.after_step(&mut game.world);
        let snapshot = crate::sim_thread::snapshot_on(
            &game.world,
            PLAYER,
            home,
            &game.world.visible_entities(PLAYER),
            &crate::looks::ModelBook::default(),
            &mut motions,
        );
        each(&game, &ctl, &snapshot);
    }
    (game, ctl)
}

#[test]
fn takeoff_script_climbs_orbital_fighters_through_the_order_palette() {
    let text = include_str!("../../../../docs/tasks/58-takeoff-script.txt");
    let mut halfway = false;
    let (game, ctl) = orbital_combat_script(text, 120, |game, _, snapshot| {
        if game.world.tick() == 50 {
            let craft: Vec<_> = snapshot
                .entities
                .iter()
                .filter(|v| v.spacecraft && !v.building)
                .collect();
            assert_eq!(craft.len(), 3);
            halfway = craft.iter().all(|v| v.altitude > 2.0 && v.altitude < 6.0);
        }
    });
    assert!(halfway, "drawn on their way up at tick 50");
    let craft = ctl.selection().to_vec();
    assert_eq!(craft.len(), 3);
    for id in craft {
        assert!(game.world.orbit(id).is_some(), "in orbit after the climb");
    }
}

#[test]
fn dogfight_script_shows_tracers_between_craft_in_orbit() {
    let text = include_str!("../../../../docs/tasks/58-dogfight-script.txt");
    let mut tracer_ticks = Vec::new();
    let (game, _) = orbital_combat_script(text, 200, |game, _, snapshot| {
        let aloft = |id: u32| {
            snapshot
                .entities
                .iter()
                .any(|v| v.id == id && v.altitude > 0.0)
        };
        if snapshot
            .entities
            .iter()
            .any(|v| v.shot.and_then(|s| s.target).is_some_and(aloft) && v.altitude > 0.0)
        {
            tracer_ticks.push(game.world.tick());
        }
    });
    println!("dogfight tracers at ticks {tracer_ticks:?}");
    assert!(tracer_ticks.contains(&171), "a volley at the review frame");
    // Both sides took hits in orbit.
    for player in [PLAYER, PlayerId(2)] {
        let hit = game.world.stats().losses.get(&player).copied().unwrap_or(0) > 0
            || game
                .world
                .entities()
                .any(|e| e.owner == player && game.world.orbit(e.id).is_some() && e.hp < e.max_hp);
        assert!(hit, "{player:?} was hit");
    }
}

#[test]
fn battery_script_fires_up_at_a_ship_in_orbit() {
    let text = include_str!("../../../../docs/tasks/58-battery-script.txt");
    let mut tracer_ticks = Vec::new();
    // Since task 75 a battery hits a ship of its age for half its hit points: the first volley,
    // at tick 10, brings the corvette down.
    let (game, _) = orbital_combat_script(text, 50, |game, _, snapshot| {
        let battery = |v: &rom_render::EntityView| v.building && v.altitude == 0.0;
        if snapshot
            .entities
            .iter()
            .any(|v| battery(v) && v.shot.is_some())
        {
            tracer_ticks.push(game.world.tick());
        }
    });
    println!("battery tracers at ticks {tracer_ticks:?}");
    assert!(!tracer_ticks.is_empty());
    assert!(
        !game
            .world
            .entities()
            .any(|e| e.owner == PlayerId(2) && game.world.orbit(e.id).is_some()),
        "the corvette still orbits above"
    );
    assert_eq!(game.world.stats().losses.get(&PlayerId(2)), Some(&1));
}

#[test]
fn carrier_script_boards_craft_and_offers_to_launch_them() {
    let text = include_str!("../../../../docs/tasks/58-carrier-script.txt");
    let (mut game, mut ctl) = orbital_combat_script(text, 172, |_, _, _| {});
    let carrier = *ctl.selection().first().expect("the carrier is selected");
    assert_eq!(game.world.passengers(carrier).len(), 3);
    let mut hud = HudSnapshot::default();
    ctl.fill(&game.world, &mut hud);
    let HudSelection::Single(card) = &hud.selection else {
        panic!("one card");
    };
    assert_eq!(card.inside.len(), 3);
    assert_eq!(card.garrison.as_deref(), Some("3 / 10"));
    let (palette, actions) = ctl.palette(&game.world).unwrap();
    assert!(actions.contains(&(4, Action::Unload)));
    assert!(palette.buttons.iter().any(|b| b.name == "Launch craft"));
    // Launch: they come out into the carrier's orbit.
    ctl.press_slot(&mut OrderSink::Lead(&mut game.world), 4, false);
    game.world.step();
    assert!(game.world.passengers(carrier).is_empty());
    let body = game.world.orbit(carrier).unwrap().body;
    let craft: Vec<_> = game
        .world
        .entities()
        .filter(|e| game.world.is_craft_type(e.type_id))
        .map(|e| e.id)
        .collect();
    assert_eq!(craft.len(), 3);
    for id in craft {
        assert_eq!(game.world.orbit(id).map(|o| o.body), Some(body));
    }
}

#[test]
fn bombard_script_strikes_the_town_and_leaves_ruins() {
    let text = include_str!("../../../../docs/tasks/75-impact-script.txt");
    let mut blast_ticks = Vec::new();
    let mut ruins = false;
    let mut flashes = 0;
    let (game, _) = orbital_combat_script(text, 400, |game, ctl, _| {
        let home = game.world.home_body(PLAYER).unwrap();
        let system = game.world.body(home).unwrap().system;
        let sightings = game.world.visible_entities(PLAYER);
        flashes += crate::space_view::space_snapshot(&game.world, system, &sightings)
            .flashes
            .len();
        if game
            .world
            .blasts()
            .last()
            .is_some_and(|b| b.tick + 1 == game.world.tick())
        {
            blast_ticks.push(game.world.tick() - 1);
        }
        let mut hud = HudSnapshot::default();
        ctl.fill(&game.world, &mut hud);
        ruins |= !hud.wrecks.is_empty() && !hud.blasts.is_empty();
    });
    println!("bombard blasts at ticks {blast_ticks:?}");
    assert!(!blast_ticks.is_empty(), "the frigate never fired");
    assert!(ruins, "no ruins were shown");
    assert!(flashes > 0, "no flash on the planet in the system view");
    // The town's city is down and buildings around it fell.
    let foe = PlayerId(2);
    assert!(
        game.world
            .entities()
            .any(|e| e.owner == foe && e.neutralised)
    );
    assert!(game.world.wrecks().iter().any(|w| w.owner == foe));
}

#[test]
fn battery_script_holds_the_frigates_off() {
    let text = include_str!("../../../../docs/tasks/75-battery-script.txt");
    let mut tracer_ticks = Vec::new();
    let (game, _) = orbital_combat_script(text, 600, |game, _, snapshot| {
        let battery = |v: &rom_render::EntityView| v.building && v.altitude == 0.0;
        if snapshot
            .entities
            .iter()
            .any(|v| battery(v) && v.shot.is_some())
        {
            tracer_ticks.push(game.world.tick());
        }
    });
    println!("battery tracers at ticks {tracer_ticks:?}");
    assert!(!tracer_ticks.is_empty());
    // The batteries stand; the frigates that came for the city did not.
    let foe = PlayerId(2);
    let rules = game.world.rules();
    let batteries = game
        .world
        .entities()
        .filter(|e| e.owner == foe && rules.key(e.type_id) == "bld.ground_battery")
        .count();
    let ships = game
        .world
        .entities()
        .filter(|e| e.owner == PLAYER && game.world.orbit(e.id).is_some())
        .count();
    assert_eq!(batteries, 2);
    assert!(ships < 3, "{ships} frigates left");
}

/// Names on every page of the builder's build palette.
fn build_names(fx: &mut Fx) -> Vec<String> {
    let bt = fx.mover();
    let builder = fx.spawn(bt, 1, 10, 10);
    fx.click(builder, false);
    let pages = fx.hud_snapshot().palette.expect("palette").pages;
    let mut names = Vec::new();
    for _ in 0..pages {
        names.extend(
            fx.hud_snapshot()
                .palette
                .unwrap()
                .buttons
                .into_iter()
                .map(|b| b.name),
        );
        fx.hud(HudRegionKind::PaletteArrow, 1, PointerButton::Left, false);
    }
    names
}

/// Task 100: a building whose in-place upgrade is researched leaves the build palette; a
/// worked site (a farm) stays beside its upgrade.
#[test]
fn superseded_buildings_leave_the_build_palette_but_worked_sites_stay() {
    let mut before = fixture_config(10_000, "age.automation", &[]);
    let faction = before
        .world
        .players()
        .find(|p| p.id == PLAYER)
        .unwrap()
        .faction;
    let name_of = |fx: &Fx, key: &str| {
        fx.rules
            .display_name(fx.rules.id_of(key).unwrap(), faction)
            .to_string()
    };
    let (stable, farm) = (name_of(&before, "bld.stable"), name_of(&before, "bld.farm"));
    let names = build_names(&mut before);
    assert!(
        names.contains(&stable) && names.contains(&farm),
        "{names:?}"
    );
    let mut after = fixture_config(
        10_000,
        "age.automation",
        &["tech.up.vehicle_plant", "tech.up.auto_farm"],
    );
    let names = build_names(&mut after);
    assert!(!names.contains(&stable), "{names:?}");
    assert!(names.contains(&farm), "{names:?}");
}

/// Task 107: the market screenshot script trades through the palette: four lots of food sold,
/// one of timber bought, each at the price its button showed.
#[test]
fn market_script_sells_and_buys_through_the_palette() {
    let text = include_str!("../../../../docs/tasks/107-market-sales-script.txt");
    let mut quotes = Vec::new();
    let (game, ctl) = orbital_combat_script(text, 40, |game, ctl, _| {
        let mut hud = rom_render::HudSnapshot::default();
        ctl.fill(&game.world, &mut hud);
        if let Some(palette) = hud.palette {
            quotes.push((game.world.tick(), palette.buttons));
        }
    });
    let world = &game.world;
    let rules = world.rules();
    let (food, timber) = (
        rules.id_of("res.food").unwrap(),
        rules.id_of("res.timber").unwrap(),
    );
    assert!(world.can_trade(PLAYER));
    assert!(world.market_pressure(PLAYER, food) < -300);
    assert!(world.market_pressure(PLAYER, timber) > 0);
    // The selected market shows a buy and a sell button per tradeable resource, enabled.
    let (_, buttons) = quotes.last().expect("a palette");
    let trades: Vec<_> = buttons
        .iter()
        .filter(|b| {
            matches!(
                b.mark,
                rom_render::IconMark::Buy(_) | rom_render::IconMark::Sell(_)
            )
        })
        .collect();
    assert_eq!(trades.len(), 2 * world.tradeable_resources(PLAYER).len());
    assert!(trades.iter().all(|b| b.state.enabled()), "{trades:?}");
    let mut hud = rom_render::HudSnapshot::default();
    ctl.fill(world, &mut hud);
    assert!(
        hud.help.starts_with("Buy 15% over the price"),
        "{}",
        hud.help
    );
}

/// Task 85: a stance button gives a `stance` order like any other, so a follower sees it in the
/// journal; the stance changes at the next step.
#[test]
fn a_stance_button_gives_an_order() {
    let mut fx = fixture();
    let kind = fx.armed_other_than_builder();
    let armed = fx.spawn(kind, 1, 20, 20);
    fx.click(armed, false);
    let mut previous = fx.world.entity(armed).unwrap().stance;
    let palette = fx.hud_snapshot().palette.expect("palette");
    // Stop, attack-move, patrol, the stances, Delete and, for a thrower, Attack ground.
    let ground = usize::from(fx.world.attacks_ground(kind));
    assert_eq!(palette.buttons.len(), 4 + STANCE_BEHAVIORS.len() + ground);
    fx.world.record_accepted(true);
    let mut stances = std::collections::BTreeSet::new();
    for button in palette
        .buttons
        .iter()
        .filter(|b| (10..14).contains(&b.slot))
    {
        fx.hud(
            HudRegionKind::CommandSlot,
            button.slot,
            PointerButton::Left,
            false,
        );
        let orders = fx.world.take_accepted();
        assert_eq!(orders.len(), 1, "{orders:?}");
        assert_eq!(
            (orders[0].kind.as_str(), orders[0].executor),
            ("stance", armed)
        );
        assert_eq!(
            fx.world.entity(armed).unwrap().stance,
            previous,
            "not before the step"
        );
        fx.step(1);
        previous = fx.world.entity(armed).unwrap().stance;
        stances.insert(previous);
    }
    assert_eq!(
        stances.len(),
        STANCE_BEHAVIORS.len(),
        "every button set its stance"
    );
}

/// Task 88: the person's orders go into the leader's world standalone and into the outbox for
/// the server in a network match, where the follower's world stays as the server left it; the
/// person's player is the session's, not player one's.
#[test]
fn the_order_sink_leads_standalone_and_sends_when_following() {
    let mut fx = fixture();
    let mover = fx.mover();
    let unit = fx.spawn(mover, 1, 5, 5);
    fx.select(&[unit]);
    let order = || PickEvent::RightClick {
        world_point: (20.5, 20.5),
        surface: flat_surface((20.5, 20.5)),
        entity: None,
        modifiers: Modifiers::default(),
    };
    // Following: the order waits for the server; the world does not change.
    let before = fx.world.hash();
    let mut outbox = Outbox::default();
    fx.ctl.handle_with(
        &mut OrderSink::Follow {
            world: &fx.world,
            outbox: &mut outbox,
        },
        order(),
    );
    assert_eq!(
        fx.world.hash(),
        before,
        "a follower's world changes only by turns"
    );
    assert_eq!(outbox.orders.len(), 1);
    assert_eq!(outbox.orders[0].executor, unit);
    assert_eq!(outbox.orders[0].player, PLAYER);
    fx.step(1);
    assert!(fx.world.entity(unit).unwrap().orders().is_empty());
    // Standalone: the same order goes into the leader's world at once.
    fx.ctl
        .handle_with(&mut OrderSink::Lead(&mut fx.world), order());
    fx.step(1);
    assert!(!fx.world.entity(unit).unwrap().orders().is_empty());
    // The seat of player two commands player two's units, in its name.
    let theirs = fx.spawn(mover, 2, 8, 8);
    let mut seat_two = Controller::for_player(PlayerId(2));
    let mut outbox = Outbox::default();
    let mut sink = OrderSink::Follow {
        world: &fx.world,
        outbox: &mut outbox,
    };
    seat_two.handle_with(
        &mut sink,
        PickEvent::BoxSelect {
            entities: vec![unit.0, theirs.0],
            modifiers: Modifiers::default(),
        },
    );
    assert_eq!(seat_two.selection(), &[theirs], "only its own units");
    seat_two.handle_with(&mut sink, order());
    assert_eq!(outbox.orders.len(), 1);
    assert_eq!(outbox.orders[0].executor, theirs);
    assert_eq!(outbox.orders[0].player, PlayerId(2));
}

/// Task 114: a universe of two players (seed 1) in which player one, the person, has ascended
/// and, with `alive`, brought the first dormant body to life. Returns the world, player one's
/// home planet and that body.
fn painter_world(alive: bool) -> (World, BodyId, BodyId) {
    let options = crate::args::Options {
        players: 2,
        ..crate::args::Options::default()
    };
    let content = crate::terrain::generate_universe_content(&options).unwrap();
    let setup = crate::script::ScriptSetup::scan("").unwrap();
    let game = crate::match_setup::build_universe_match(
        &content.rules,
        &content.universe,
        &content.factions,
        1,
        None,
        &setup,
    )
    .unwrap();
    let rules = game.world.rules().clone();
    // Player one ascended at tick 0 (as a finished ascension leaves it).
    let mut snapshot = game.world.snapshot();
    for p in &mut snapshot.players {
        if p.player.id == PLAYER {
            p.status = rom_sim::PlayerStatus::Ascended;
            p.ascended = Some(0);
            p.ascensions = 1;
        }
    }
    let mut world = World::restore(rules, snapshot).expect("restore");
    assert!(world.is_ascended(PLAYER));
    let home = world.home_body(PLAYER).unwrap();
    let (moon, _) = world.dormant_bodies().next().expect("a dormant body");
    if alive {
        let marker = world.body_marker(moon).expect("marker");
        world
            .submit(Command {
                player: PLAYER,
                executor: EntityId(0),
                kind: "bring_life".into(),
                subject: None,
                target_entity: Some(marker),
                target_pos: None,
                queue: false,
                text: None,
                facing: None,
            })
            .expect("bring life");
        world.step();
        assert!(world.living(moon).is_some());
    }
    (world, home, moon)
}

fn painter(world: &World, planet: BodyId) -> Controller {
    let _ = world;
    let mut ctl = Controller::new();
    ctl.set_view(ViewCtx {
        planet,
        ..ViewCtx::default()
    });
    ctl
}

fn hud_click(ctl: &mut Controller, world: &mut World, region: HudRegionKind, index: usize) {
    ctl.handle(
        world,
        PickEvent::HudClick {
            region,
            index,
            button: PointerButton::Left,
            modifiers: Modifiers::default(),
        },
    );
}

#[test]
fn every_brush_is_in_the_ascended_palette_by_its_family() {
    let (mut world, _, moon) = painter_world(true);
    let mut ctl = painter(&world, moon);
    hud_click(&mut ctl, &mut world, HudRegionKind::PaletteTab, 0);
    let (palette, _) = ctl.palette(&world).expect("a palette");
    assert_eq!(palette.tab, PaletteTab::Build);
    let families = palette.groups.len();
    assert_eq!(families, 5, "land, water, elevation, biomes, deposits");
    assert!(palette.groups[0].current);
    let rules = world.rules().clone();
    let mut seen = BTreeSet::new();
    for family in 0..families {
        // A click on the family's row shows its brushes, the row framed.
        hud_click(&mut ctl, &mut world, HudRegionKind::PaletteGroup, family);
        let (palette, actions) = ctl.palette(&world).expect("a palette");
        assert!(palette.groups[family].current);
        assert_eq!(palette.groups.iter().filter(|g| g.current).count(), 1);
        assert_eq!(palette.groups[family].count, actions.len());
        for ((slot, action), button) in actions.iter().zip(&palette.buttons) {
            let Action::Brush(brush) = action else {
                panic!("only brushes on the Build tab");
            };
            assert_eq!(rules.brushes[brush].family, {
                let first = actions.iter().find_map(|(_, a)| match a {
                    Action::Brush(b) => Some(*b),
                    _ => None,
                });
                rules.brushes[&first.unwrap()].family
            });
            assert_eq!(button.slot, *slot);
            // Its picture, swatch and size come from the data, its rollover names its size.
            let rom_render::IconMark::Brush { swatch, size } = button.mark else {
                panic!("a brush mark");
            };
            assert!((1..=3).contains(&size));
            assert_eq!(swatch.is_some(), rules.brushes[brush].terrain_id.is_some());
            assert!(button.name.contains("brush,"), "{}", button.name);
            assert!(seen.insert(*brush), "listed once");
        }
    }
    let all: BTreeSet<DefId> = rules.brushes.keys().copied().collect();
    assert_eq!(seen, all, "no brush unreachable");
    // The slot keys still pick brushes: the first family's second brush with W.
    hud_click(&mut ctl, &mut world, HudRegionKind::PaletteGroup, 0);
    let (_, actions) = ctl.palette(&world).unwrap();
    ctl.handle(
        &mut world,
        PickEvent::Hotkey {
            ch: 'w',
            modifiers: Modifiers::default(),
        },
    );
    assert_eq!(ctl.brush.map(Action::Brush), Some(actions[1].1));
    let (palette, _) = ctl.palette(&world).unwrap();
    assert!(palette.groups[0].armed);
    // The hint names the brush in hand; without one, the families.
    let mut hud = HudSnapshot::default();
    ctl.fill(&world, &mut hud);
    assert!(hud.help.contains("drag on the ground"), "{}", hud.help);
    assert!(hud.painting, "a brush in hand: the left drag paints");
    ctl.brush = None;
    ctl.fill(&world, &mut hud);
    assert!(
        hud.help
            .starts_with("Paint land, water, mountains, biomes and deposits"),
        "{}",
        hud.help
    );
    // Sizes tell the brushes of one terrain apart: land 1, 2, 3.
    let sizes: Vec<u8> = actions
        .iter()
        .map(|(_, a)| match a {
            Action::Brush(b) => brush_size(&rules, *b),
            _ => 0,
        })
        .collect();
    assert_eq!(sizes, [1, 2, 3]);
}

#[test]
fn the_palette_paints_nothing_on_a_planet_the_painter_did_not_bring_to_life() {
    let (mut world, home, moon) = painter_world(false);
    for planet in [home, moon] {
        let mut ctl = painter(&world, planet);
        hud_click(&mut ctl, &mut world, HudRegionKind::PaletteTab, 0);
        let (palette, actions) = ctl.palette(&world).expect("the order tab");
        assert!(!palette.tabs[0], "no brushes");
        assert_eq!(palette.tab, PaletteTab::Order);
        assert!(palette.groups.is_empty());
        assert!(
            actions.iter().all(|(_, a)| !matches!(a, Action::Brush(_))),
            "no brush"
        );
        // The closed tab says why.
        assert!(
            palette.tab_notes[0].contains("did not bring"),
            "{}",
            palette.tab_notes[0]
        );
        let mut hud = HudSnapshot::default();
        ctl.fill(&world, &mut hud);
        assert!(!hud.help.starts_with("Paint "), "{}", hud.help);
    }
    // In space nothing is painted either.
    let (world, _, moon) = painter_world(true);
    let mut ctl = painter(&world, moon);
    ctl.set_view(ViewCtx {
        planet: moon,
        in_space: true,
        ..ViewCtx::default()
    });
    assert!(ctl.brush_families(&world).is_empty());
}

/// Task 123: Land arms a pick for aircraft, the click lands them on open ground, and Take off
/// (the Lift slot) sends a landed aircraft up; right-clicking an own airfield lands there.
#[test]
fn aircraft_land_with_the_land_pick_and_take_off() {
    let mut fx = fixture();
    let plane = fx.find(|et| et.movement.as_ref().is_some_and(|m| m.takeoff_ticks > 0));
    let ticks = fx.world.takeoff_ticks(plane).unwrap() as usize;
    let id = fx.spawn(plane, 1, 10, 10);
    fx.select(&[id]);
    fx.hud(HudRegionKind::PaletteTab, 1, PointerButton::Left, false);
    let (_, actions) = fx.ctl.palette(&fx.world).unwrap();
    assert!(actions.contains(&(6, Action::Land)));
    assert!(
        !actions.contains(&(5, Action::Lift)),
        "airborne: nothing to take off from"
    );
    fx.hud(HudRegionKind::CommandSlot, 6, PointerButton::Left, false);
    let land = fx.hud_snapshot().palette.unwrap().buttons;
    assert!(
        land.iter()
            .any(|b| b.slot == 6 && b.armed && b.name == "Land")
    );
    assert!(fx.hud_snapshot().help.contains("airfield"));
    fx.ground_click();
    fx.step(1);
    assert_eq!(fx.kinds(id), ["land"]);
    fx.step(ticks + 200);
    assert!(fx.world.is_aircraft_landed(id));
    let (_, actions) = fx.ctl.palette(&fx.world).unwrap();
    assert!(actions.contains(&(5, Action::Lift)));
    assert!(
        !actions.contains(&(6, Action::Land)),
        "landed: nothing to land"
    );
    let buttons = fx.hud_snapshot().palette.unwrap().buttons;
    assert!(buttons.iter().any(|b| b.slot == 5 && b.name == "Take off"));
    fx.hud(HudRegionKind::CommandSlot, 5, PointerButton::Left, false);
    fx.step(ticks + 1);
    assert!(!fx.world.is_grounded(id));
    // A right-click on an own airfield lands it there.
    let airfield = fx.find(|et| fx.world.is_airfield_type(et.id));
    let field = fx.spawn(airfield, 1, 20, 20);
    let at = fx.world.entity(field).unwrap().pos;
    let point = (at.x as f32 / 1000.0, at.y as f32 / 1000.0);
    fx.rclick(Some(field), point, false);
    fx.step(1);
    assert_eq!(fx.kinds(id), ["land"]);
}

/// Task 131: the person's own capital is marked from the first tick: on its planet's ground
/// and minimap, and in the selection panel, whose help says what losing it means; an enemy
/// capital nobody has found is not marked anywhere.
#[test]
fn the_own_capital_is_marked_from_the_first_tick_and_an_unknown_one_is_not() {
    let (mut ctl, mut game) = sphere_fixture();
    let world = &mut game.world;
    world.step();
    let start = game.starts.iter().find(|s| s.player == PLAYER).unwrap();
    let capital = world.capital(PLAYER).expect("a capital");
    let body = world.entity(capital).unwrap().pos.body;
    ctl.set_view(ViewCtx {
        planet: body,
        ..ViewCtx::default()
    });
    let mut hud = HudSnapshot::default();
    ctl.fill(world, &mut hud);
    assert_eq!(hud.capitals.len(), 1, "{:?}", hud.capitals);
    let own = hud.capitals[0];
    assert_eq!(own.city, capital.0);
    assert_eq!(own.owner, PLAYER.0 as u8);
    assert_eq!((own.face, own.x, own.y), {
        let at = world.entity(capital).unwrap().pos;
        (at.face, at.x, at.y)
    });
    assert!(!own.remembered);
    assert!(own.ring > world.city_economic_radius(capital));
    assert_eq!(start.player, PLAYER);
    // The enemy's capital is unknown: neither its city nor its planet nor its system shows.
    let enemy = game
        .starts
        .iter()
        .find(|s| s.player != PLAYER)
        .map(|s| s.player)
        .unwrap();
    assert!(world.capital(enemy).is_some());
    let space = crate::capital_view::space_capitals(&world.known_capitals(PLAYER));
    assert!(space.iter().all(|c| c.owner == PLAYER.0 as u8), "{space:?}");
    // Selected, its card names it a capital and the help line says what losing it means.
    ctl.handle(
        world,
        PickEvent::Click {
            world_point: (0.0, 0.0),
            surface: flat_surface((0.0, 0.0)),
            entity: Some(capital.0),
            modifiers: Modifiers::default(),
            double: false,
            on_screen: Vec::new(),
        },
    );
    let mut hud = HudSnapshot::default();
    ctl.fill(world, &mut hud);
    let HudSelection::Single(card) = hud.selection else {
        panic!("one city selected");
    };
    assert!(card.capital);
    assert!(hud.help.starts_with("Your capital"), "{}", hud.help);
    assert!(hud.help.contains("lose the game"), "{}", hud.help);
}

fn press_delete(fx: &mut Fx) {
    fx.ctl.handle(
        &mut fx.world,
        PickEvent::Hotkey {
            ch: DELETE_KEY,
            modifiers: Modifiers::default(),
        },
    );
}

/// Task 142: the Delete key removes own units at once; a finished building asks first, on the
/// command card, and a second Delete (or Yes) answers; Escape keeps it; a city stays.
#[test]
fn delete_removes_units_and_asks_before_a_finished_building() {
    let mut fx = fixture();
    let soldier = fx.armed_other_than_builder();
    let unit = fx.spawn(soldier, 1, 10, 10);
    let other = fx.spawn(soldier, 1, 12, 10);
    fx.select(&[unit, other]);
    let hud = fx.hud_snapshot();
    let palette = hud.palette.expect("orders");
    assert!(
        palette
            .buttons
            .iter()
            .any(|b| b.slot == DELETE_SLOT && b.name.starts_with("Delete")),
        "a Delete button in the last slot"
    );
    press_delete(&mut fx);
    fx.step(1);
    assert!(fx.world.entity(unit).is_none() && fx.world.entity(other).is_none());

    let barracks_t = fx.find(|et| {
        et.class == EntityClass::Building
            && et.producer.is_some()
            && et.territory.is_none()
            && et.market.is_none()
            && fx
                .rules
                .production
                .get(&et.id)
                .is_some_and(|rows| rows.iter().any(|r| r.mode == ProductionMode::Train))
    });
    let barracks = fx.spawn(barracks_t, 1, 20, 20);
    fx.click(barracks, false);
    // Its own page opens first, as before.
    assert_eq!(fx.hud_snapshot().palette.unwrap().tab, PaletteTab::Train);
    press_delete(&mut fx);
    fx.step(1);
    assert!(fx.world.entity(barracks).is_some(), "it asks first");
    let hud = fx.hud_snapshot();
    let palette = hud.palette.expect("the question");
    assert_eq!(palette.tab, PaletteTab::Order);
    let names: Vec<(usize, &str)> = palette
        .buttons
        .iter()
        .map(|b| (b.slot, b.name.as_str()))
        .collect();
    assert_eq!(
        names,
        vec![
            (DELETE_SLOT, "Delete? Yes"),
            (DELETE_SLOT - 1, "No, keep it")
        ]
    );
    assert!(hud.help.contains("Delete"), "{}", hud.help);
    // No keeps it.
    fx.hud(
        HudRegionKind::CommandSlot,
        DELETE_SLOT - 1,
        PointerButton::Left,
        false,
    );
    fx.step(1);
    assert!(fx.world.entity(barracks).is_some());
    assert_eq!(fx.hud_snapshot().palette.unwrap().tab, PaletteTab::Train);
    // Escape keeps it too.
    press_delete(&mut fx);
    fx.ctl.handle(&mut fx.world, PickEvent::Escape);
    fx.step(1);
    assert!(fx.world.entity(barracks).is_some());
    // Delete twice: gone.
    press_delete(&mut fx);
    press_delete(&mut fx);
    fx.step(1);
    assert!(fx.world.entity(barracks).is_none());

    // A finished city cannot be deleted.
    let city_t = fx.find(|et| et.territory.as_ref().is_some_and(|t| t.is_city));
    let city = fx.spawn(city_t, 1, 40, 40);
    fx.click(city, false);
    press_delete(&mut fx);
    fx.step(1);
    assert!(fx.world.entity(city).is_some());
    assert!(
        fx.hud_snapshot().events.iter().any(|e| e.contains("city")),
        "it says why"
    );
}

/// Task 142: Attack ground arms a pick for units that throw projectiles; the next click on the
/// ground orders them to fire there, and the target is drawn with a line from each unit.
#[test]
fn attack_ground_arms_a_pick_and_orders_the_point() {
    let mut fx = fixture();
    let world = &fx.world;
    let thrower =
        fx.find(|et| et.movement.is_some() && et.builder.is_none() && world.attacks_ground(et.id));
    let melee = fx.find(|et| {
        et.movement.is_some()
            && et.builder.is_none()
            && !et.weapons.is_empty()
            && et.weapons.iter().all(|w| w.range_max == 0)
    });
    let a = fx.spawn(thrower, 1, 10, 10);
    let b = fx.spawn(melee, 1, 12, 10);
    fx.select(&[a, b]);
    let palette = fx.hud_snapshot().palette.expect("orders");
    assert_eq!(palette.tab, PaletteTab::Order);
    assert!(palette.buttons.iter().any(|btn| btn.slot == GROUND_SLOT));
    assert!(!fx.ctl.aiming(&fx.world));
    fx.hud(
        HudRegionKind::CommandSlot,
        GROUND_SLOT,
        PointerButton::Left,
        false,
    );
    assert!(fx.ctl.aiming(&fx.world));
    fx.ground_click();
    assert!(!fx.ctl.aiming(&fx.world), "a plain click ends the pick");
    assert_eq!(fx.ctl.selection().len(), 2, "the click selects nothing");
    fx.step(1);
    assert_eq!(fx.kinds(a), vec!["attack_ground"]);
    assert!(fx.kinds(b).is_empty(), "a blow cannot be thrown");
    let hud = fx.hud_snapshot();
    assert!(
        hud.markers
            .iter()
            .any(|m| m.kind == MarkerKind::Ground && m.from.is_some()),
        "the target is drawn"
    );
}

/// Task 150: the dabs of a stroke between two points are evenly spaced, none beyond the end.
#[test]
fn a_stroke_between_two_points_is_a_series_of_dabs_without_gaps() {
    let pos = |x: i32, y: i32| Pos {
        x,
        y,
        face: 0,
        body: BodyId(0),
    };
    let dabs = stroke_samples(pos(1000, 1000), pos(8000, 1000), 2000);
    let xs: Vec<i32> = dabs.iter().map(|d| d.x).collect();
    assert_eq!(xs, vec![3000, 5000, 7000]);
    assert!(dabs.iter().all(|d| d.y == 1000));
    // A diagonal: every dab is one spacing from the last.
    let diagonal = stroke_samples(pos(0, 0), pos(6000, 8000), 2500);
    assert_eq!(diagonal.len(), 4);
    assert_eq!((diagonal[0].x, diagonal[0].y), (1500, 2000));
    assert_eq!((diagonal[3].x, diagonal[3].y), (6000, 8000));
    // Shorter than a spacing: nothing yet (the next move continues from the last dab).
    assert!(stroke_samples(pos(0, 0), pos(900, 0), 1000).is_empty());
    // Across a face edge only the end counts.
    let other = Pos {
        face: 1,
        ..pos(0, 0)
    };
    assert_eq!(stroke_samples(pos(5000, 5000), other, 1000), vec![other]);
}

fn press_key(fx: &mut Fx, ch: char) {
    fx.ctl.handle(
        &mut fx.world,
        PickEvent::Hotkey {
            ch,
            modifiers: Modifiers::default(),
        },
    );
}

/// Task 141: militia fall in with the army; citizens are called up from the command card (F)
/// and stood down again (R), only inside the borders.
#[test]
fn militia_go_with_the_army_and_are_called_up_and_stood_down_from_the_card() {
    let mut fx = fixture();
    fx.spawn(fx.key("bld.city"), 1, 20, 20);
    let soldier = fx.spawn(fx.key("unit.spearman"), 1, 20, 26);
    let militia = fx.spawn(fx.key("unit.militia_ancient"), 1, 21, 26);
    let citizen = fx.spawn(fx.key("unit.citizen"), 1, 22, 26);
    fx.boxed(&[soldier, militia, citizen], false);
    assert_eq!(
        ids(&fx),
        vec![soldier.0, militia.0],
        "militia go with the army"
    );

    fx.boxed(&[citizen], false);
    // A citizen's card opens on its Build tab; the orders are on the Order tab.
    fx.hud(HudRegionKind::PaletteTab, 1, PointerButton::Left, false);
    let palette = fx.hud_snapshot().palette.expect("orders");
    assert_eq!(palette.tab, PaletteTab::Order);
    let call = palette
        .buttons
        .iter()
        .find(|b| b.slot == CALL_UP_SLOTS[0])
        .expect("Call up in F");
    assert_eq!(call.name, "Call up as militia: Levies");
    assert!(call.state.enabled());
    press_key(&mut fx, 'f');
    fx.step(1);
    assert_eq!(
        fx.world.entity(citizen).unwrap().type_id,
        fx.key("unit.militia_ancient")
    );
    assert_eq!(ids(&fx), vec![citizen.0], "it stays selected");

    let palette = fx.hud_snapshot().palette.expect("orders");
    let stand = palette
        .buttons
        .iter()
        .find(|b| b.slot == STAND_DOWN_SLOTS[0])
        .expect("Stand down in R");
    assert_eq!(stand.name, "Stand down: Citizens again");
    assert!(palette.buttons.iter().all(|b| b.slot != CALL_UP_SLOTS[0]));
    press_key(&mut fx, 'r');
    fx.step(1);
    assert_eq!(
        fx.world.entity(citizen).unwrap().type_id,
        fx.key("unit.citizen")
    );

    // Outside the borders the button says why, and pressing it orders nothing.
    let far = fx.spawn(fx.key("unit.citizen"), 1, 62, 62);
    fx.boxed(&[far], false);
    fx.hud(HudRegionKind::PaletteTab, 1, PointerButton::Left, false);
    let palette = fx.hud_snapshot().palette.expect("orders");
    let call = palette
        .buttons
        .iter()
        .find(|b| b.slot == CALL_UP_SLOTS[0])
        .expect("Call up in F");
    assert!(call.state.reason().contains("borders"), "{:?}", call.state);
    press_key(&mut fx, 'f');
    fx.step(1);
    assert_eq!(
        fx.world.entity(far).unwrap().type_id,
        fx.key("unit.citizen")
    );
}

/// Task 143: a group's orders name its formation, Square until another is picked on the Order
/// tab's Formation page (or with Alt and a digit); picking one forms the group up at once.
#[test]
fn a_group_walks_in_the_formation_picked_square_until_then() {
    let mut fx = fixture();
    let a = fx.spawn(fx.key("unit.spearman"), 1, 10, 10);
    let b = fx.spawn(fx.key("unit.archer"), 1, 11, 10);
    let c = fx.spawn(fx.key("unit.medic"), 1, 12, 10);
    let subject = |fx: &Fx, id: EntityId| {
        let order = &fx.world.entity(id).expect("alive").orders()[0];
        (order.subject, order.target_pos)
    };
    fx.select(&[a, b, c]);
    let palette = fx.hud_snapshot().palette.expect("orders");
    assert_eq!(
        (palette.tab, palette.pages, palette.page),
        (PaletteTab::Order, 2, 0)
    );
    // The second page: Square, Compact, Circle, Line and Double line on the first row.
    fx.hud(HudRegionKind::PaletteArrow, 1, PointerButton::Left, false);
    let palette = fx.hud_snapshot().palette.expect("formations");
    assert_eq!(palette.page_name, "Formation");
    let names: Vec<&str> = (0..5)
        .map(|slot| {
            palette
                .buttons
                .iter()
                .find(|btn| btn.slot == slot)
                .map_or("", |btn| btn.name.as_str())
        })
        .collect();
    assert_eq!(
        names,
        [
            "Formation: Square (Alt+1)",
            "Formation: Compact (Alt+2)",
            "Formation: Circle (Alt+3)",
            "Formation: Line (Alt+4)",
            "Formation: Double Line (Alt+5)"
        ]
    );
    assert!(palette.buttons.iter().any(|btn| btn.slot == 0 && btn.armed));
    // A right click sends the group in a square, each to its own place.
    fx.rclick(None, (30.0, 10.0), false);
    fx.step(1);
    let square = fx.key("form.square");
    let (sa, pa) = subject(&fx, a);
    let (sb, pb) = subject(&fx, b);
    assert_eq!((sa, sb), (Some(square), Some(square)));
    assert_ne!(pa, pb);
    // Alt+4 picks the line: the walking group re-forms in one rank.
    fx.ctl.handle(
        &mut fx.world,
        PickEvent::Hotkey {
            ch: '4',
            modifiers: Modifiers {
                alt: true,
                ..Modifiers::default()
            },
        },
    );
    fx.step(1);
    let line = fx.key("form.line");
    assert_eq!(subject(&fx, a).0, Some(line));
    assert_eq!(subject(&fx, c).0, Some(line));
    let hud = fx.hud_snapshot().palette.expect("formations");
    assert!(hud.buttons.iter().any(|btn| btn.slot == 3 && btn.armed));
    // The slot key on the Formation page picks too (Q: Square); the group order after names it.
    fx.ctl.handle(
        &mut fx.world,
        PickEvent::Hotkey {
            ch: 'q',
            modifiers: Modifiers::default(),
        },
    );
    fx.rclick(None, (20.0, 20.0), false);
    fx.step(1);
    assert_eq!(subject(&fx, b).0, Some(square));
    // One unit alone names no formation.
    fx.select(&[a]);
    fx.rclick(None, (25.0, 25.0), false);
    fx.step(1);
    assert_eq!(subject(&fx, a).0, None);
}

// ----- task 151: what the pointer rests on ---------------------------------------------------

fn point_at(fx: &mut Fx, entity: Option<EntityId>) {
    fx.ctl.handle(
        &mut fx.world,
        PickEvent::Pointed {
            entity: entity.map(|e| e.0),
        },
    );
}

fn badge_on(fx: &mut Fx, entity: EntityId) -> Option<rom_render::OrderBadge> {
    point_at(fx, Some(entity));
    fx.hud_snapshot().pointer.expect("pointed at").badge
}

#[test]
fn the_side_of_the_hovered_entity_follows_the_relation_not_the_team_colour() {
    use rom_render::Side;
    let rules = Arc::new(crate::terrain::load_rules().expect("rules"));
    let id = |key: &str| rules.id_of(key).expect("definition");
    // Player 3 shares player 1's team.
    let player = |n: u32, team: u32| Player {
        id: PlayerId(n),
        team,
        age: id("age.ancient"),
        faction: id("nation.romans"),
        resources: rules.resources.keys().map(|r| (*r, 1000)).collect(),
        techs: Default::default(),
    };
    let map = MapGrid::new(64, 64, vec![id("ter.plain"); 64 * 64]).expect("map");
    let players = vec![player(0, 0), player(1, 1), player(2, 2), player(3, 1)];
    let world = World::new(rules.clone(), map, 1, players).expect("world");
    let mut fx = Fx {
        world,
        rules,
        ctl: Controller::new(),
    };
    let t = fx.armed_other_than_builder();
    let own = fx.spawn(t, 1, 10, 10);
    let ally = fx.spawn(t, 3, 12, 10);
    let foe = fx.spawn(t, 2, 14, 10);
    let node_t = fx.find(|et| et.node.is_some());
    let nature = fx.spawn(node_t, 0, 16, 10);
    for (entity, side, word) in [
        (own, Side::Own, "Yours · "),
        (ally, Side::Ally, "Ally · "),
        (foe, Side::Enemy, "Enemy · "),
        (nature, Side::Neutral, "Neutral · "),
    ] {
        point_at(&mut fx, Some(entity));
        let pointer = fx.hud_snapshot().pointer.expect("pointed at");
        assert_eq!(pointer.entity, entity.0);
        assert_eq!(pointer.side, side, "{word}");
        assert!(pointer.text.starts_with(word), "{}", pointer.text);
    }
    // The enemy's line names its kind, its people and its health.
    point_at(&mut fx, Some(foe));
    let romans = fx.key("nation.romans");
    let e = fx.world.entity(foe).expect("foe");
    assert_eq!(
        fx.hud_snapshot().pointer.expect("pointed at").text,
        format!(
            "Enemy · {} ({}) · {}/{}",
            fx.rules.display_name(t, romans),
            fx.rules.def(romans).name,
            e.hp,
            e.max_hp
        )
    );
    // Nothing under the pointer: nothing to say.
    point_at(&mut fx, None);
    assert_eq!(fx.hud_snapshot().pointer, None);
    // The selection rings' table: by relation, whatever the team colours.
    assert_eq!(
        fx.hud_snapshot().sides,
        vec![Side::Neutral, Side::Own, Side::Enemy, Side::Ally]
    );
}

#[test]
fn the_right_click_badge_names_the_order_per_target_kind() {
    use rom_render::OrderBadge;
    let mut fx = fixture();
    let t = fx.armed_other_than_builder();
    let soldier = fx.spawn(t, 1, 10, 10);
    let foe = fx.spawn(t, 2, 12, 12);
    let friend = fx.spawn(t, 1, 14, 10);
    // Nothing selected: the side shows, no badge.
    assert_eq!(badge_on(&mut fx, foe), None);
    fx.click(soldier, false);
    assert_eq!(badge_on(&mut fx, foe), Some(OrderBadge::Attack));
    let pointer = fx.hud_snapshot().pointer.expect("pointed at");
    assert_eq!(pointer.hint, "Right-click: attack");
    // A right click on an own soldier is a move there: no badge, no hint.
    assert_eq!(badge_on(&mut fx, friend), None);
    assert_eq!(fx.hud_snapshot().pointer.expect("pointed at").hint, "");
    // A gather site for a worker that gathers its resource.
    let worker_t = fx.find(|et| et.movement.is_some() && !et.harvester.is_empty());
    let resource = fx.rules.entity_types[&worker_t].harvester[0].resource_id;
    let site_t = fx.find(|et| {
        et.class == EntityClass::Building
            && et
                .gather_site
                .as_ref()
                .is_some_and(|g| g.resource_id == resource)
    });
    let worker = fx.spawn(worker_t, 1, 30, 10);
    let site = fx.spawn(site_t, 1, 34, 10);
    fx.click(worker, false);
    assert_eq!(badge_on(&mut fx, site), Some(OrderBadge::Gather));
    // A builder: a construction site is built; an unarmed builder walks to an enemy instead of
    // attacking it, so no badge.
    let bt = fx.mover();
    let building = fx.rules.production[&bt]
        .iter()
        .find(|r| r.mode == ProductionMode::Construct)
        .unwrap()
        .product_def_id;
    let builder = fx.spawn(bt, 1, 40, 40);
    fx.world
        .submit(Command {
            player: PLAYER,
            executor: builder,
            kind: "construct".into(),
            subject: Some(building),
            target_entity: None,
            target_pos: Some(at(46, 46)),
            queue: false,
            text: None,
            facing: None,
        })
        .expect("construct");
    fx.step(1);
    let construction = fx
        .world
        .entities()
        .find(|e| e.build_progress.is_some())
        .expect("site")
        .id;
    fx.click(builder, false);
    assert_eq!(badge_on(&mut fx, construction), Some(OrderBadge::Build));
    if fx.rules.entity_types[&bt].weapons.is_empty() {
        assert_eq!(badge_on(&mut fx, foe), None);
    }
    // A mixed selection shows the first badge any of them would take: the soldier attacks.
    fx.boxed(&[builder, soldier], true);
    assert_eq!(badge_on(&mut fx, foe), Some(OrderBadge::Attack));
    // The badge agrees with the order the right click then gives.
    fx.click(worker, false);
    assert_eq!(badge_on(&mut fx, site), Some(OrderBadge::Gather));
    fx.rclick(Some(site), (34.5, 10.5), false);
    fx.step(1);
    assert_eq!(fx.kinds(worker)[0], "harvest");
}

#[test]
fn the_target_of_a_right_click_order_flashes_twice_in_the_orders_colour() {
    let mut fx = fixture();
    let t = fx.armed_other_than_builder();
    let soldier = fx.spawn(t, 1, 10, 10);
    let foe = fx.spawn(t, 2, 12, 12);
    fx.click(soldier, false);
    fx.rclick(Some(foe), (12.5, 12.5), false);
    let flashes = fx.hud_snapshot().flashes;
    assert_eq!(flashes.len(), 1);
    assert_eq!((flashes[0].entity, flashes[0].hostile), (foe.0, true));
    // Ground orders do not flash; the move marker stays theirs.
    fx.rclick(None, (20.0, 12.0), false);
    assert_eq!(fx.hud_snapshot().flashes.len(), 1);
    // A friendly order flashes green.
    let worker_t = fx.find(|et| et.movement.is_some() && !et.harvester.is_empty());
    let resource = fx.rules.entity_types[&worker_t].harvester[0].resource_id;
    let site_t = fx.find(|et| {
        et.class == EntityClass::Building
            && et
                .gather_site
                .as_ref()
                .is_some_and(|g| g.resource_id == resource)
    });
    let worker = fx.spawn(worker_t, 1, 30, 10);
    let site = fx.spawn(site_t, 1, 34, 10);
    fx.click(worker, false);
    fx.rclick(Some(site), (34.5, 10.5), false);
    let flashes = fx.hud_snapshot().flashes;
    assert!(flashes.iter().any(|f| f.entity == site.0 && !f.hostile));
    // Two flashes, then gone.
    let lit: usize = (0..rom_render::pointer::FLASH_TICKS)
        .map(|_| {
            fx.step(1);
            usize::from(
                fx.hud_snapshot()
                    .flashes
                    .iter()
                    .any(|f| f.entity == site.0 && f.lit()),
            )
        })
        .sum();
    assert!(lit > 0);
    assert!(fx.hud_snapshot().flashes.is_empty());
}

#[test]
fn only_the_render_threads_pointed_pick_names_the_hovered_entity() {
    let mut fx = fixture();
    let t = fx.armed_other_than_builder();
    let foe = fx.spawn(t, 2, 12, 12);
    point_at(&mut fx, Some(foe));
    // A planet hover that found nothing (the pointer is on a ship in space) leaves it.
    fx.ctl.handle(
        &mut fx.world,
        PickEvent::Hover {
            entity: None,
            world_point: (3.0, 3.0),
            surface: flat_surface((3.0, 3.0)),
        },
    );
    assert_eq!(fx.hud_snapshot().pointer.map(|p| p.entity), Some(foe.0));
    point_at(&mut fx, None);
    assert_eq!(fx.hud_snapshot().pointer, None);
}
