use crate::{Command, EntityId, MapGrid, Player, PlayerId, Pos, SimError, SimRng, World};
use rom_rules::{DefId, Rules, build_rules_db};
use std::{collections::BTreeMap, sync::Arc};
const PINNED_GEOMETRY: &str = "UPDATE et_territory SET border_radius=24000, economic_radius=20000 WHERE is_city=1; UPDATE game_constant SET value=24000 WHERE key='city.spacing'; UPDATE game_constant SET value=0 WHERE key IN ('city.radius_per_building','city.radius_from_buildings_max');";

const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);

fn rules(sql: &str) -> Arc<Rules> {
    let db = build_rules_db(
        include_str!("../../../../db/schema.sql"),
        include_str!("../../../../db/seed.sql"),
    )
    .unwrap();
    // These tests assert the geometry of the original tuning; pin it here.
    db.execute_batch(PINNED_GEOMETRY).unwrap();
    db.execute_batch(sql).unwrap();
    Arc::new(Rules::load(&db).unwrap())
}

/// A 64×64 world for players 1 and 3 (one team) and 2; `terrain` names each tile.
fn world_on(rules: Arc<Rules>, terrain: impl Fn(i32, i32) -> &'static str) -> World {
    let players = (1..=3)
        .map(|n| Player {
            id: PlayerId(n),
            team: if n == 3 { 1 } else { n },
            faction: rules.id_of("nation.romans").unwrap(),
            age: rules.id_of("age.ancient").unwrap(),
            resources: rules.resources.keys().map(|r| (*r, 100_000)).collect(),
            techs: [(rules.id_of("tech.military.1").unwrap(), 1)].into(),
        })
        .collect();
    let tiles = (0..64 * 64)
        .map(|i| rules.id_of(terrain(i % 64, i / 64)).unwrap())
        .collect();
    World::new(rules, MapGrid::new(64, 64, tiles).unwrap(), 7, players).unwrap()
}

fn world(sql: &str) -> World {
    world_on(rules(sql), |_, _| "ter.plain")
}

fn id(w: &World, key: &str) -> DefId {
    w.rules.id_of(key).unwrap()
}

fn at(x: i32, y: i32) -> Pos {
    Pos::flat(x, y)
}

fn place(w: &mut World, key: &str, owner: PlayerId, pos: Pos) -> EntityId {
    w.place_for_setup(id(w, key), owner, pos).unwrap()
}

fn site(w: &mut World, key: &str, owner: PlayerId, pos: Pos) -> EntityId {
    w.spawn_entity(id(w, key), owner, pos, true).unwrap()
}

fn complete(w: &mut World, site: EntityId) {
    w.entities[site.0 as usize].as_mut().unwrap().build_progress = None;
    w.refresh_territory(site);
}

fn construct(w: &mut World, owner: PlayerId, key: &str, pos: Pos) -> Result<(), SimError> {
    let builder = w
        .spawn(id(w, "unit.citizen"), owner, at(1500, 62_500))
        .unwrap();
    w.submit(Command {
        player: owner,
        executor: builder,
        kind: "construct".into(),
        subject: Some(id(w, key)),
        target_entity: None,
        target_pos: Some(pos),
        queue: false,
    })
    .map(|_| ())
}

fn owner(w: &World, x: i32, y: i32) -> Option<PlayerId> {
    w.territory_owner(x, y)
}

/// The incremental grid and tile counts equal a full recomputation.
fn check(w: &World) {
    let full = w.full_territory();
    assert_eq!(w.surfaces[0].territory.winner, full, "tick {}", w.tick);
    let mut owned: BTreeMap<PlayerId, u64> = BTreeMap::new();
    for winner in full.iter().filter(|w| **w != 0) {
        *owned
            .entry(w.territory.emitters[&EntityId(*winner)].owner)
            .or_default() += 1;
    }
    assert_eq!(w.territory.owned, owned);
}

#[test]
fn border_grows_from_a_completed_city_only() {
    let mut w = world("");
    let builder = w
        .spawn(id(&w, "unit.citizen"), P1, at(15_000, 30_500))
        .unwrap();
    w.submit(Command {
        player: P1,
        executor: builder,
        kind: "construct".into(),
        subject: Some(id(&w, "bld.city")),
        target_entity: None,
        target_pos: Some(at(20_500, 30_500)),
        queue: false,
    })
    .unwrap();
    w.step();
    let city = w.cities(P1)[0];
    assert!(w.entity(city).unwrap().build_progress.is_some());
    assert!(w.owned_tile_counts().is_empty());
    assert_eq!(owner(&w, 20, 30), None);
    for _ in 0..2000 {
        if w.entity(city).unwrap().build_progress.is_none() {
            break;
        }
        w.step();
    }
    assert_eq!(w.entity(city).unwrap().build_progress, None);
    assert_eq!(owner(&w, 20, 30), Some(P1));
    // The border radius is 24 tiles: (44, 30) is exactly on it, (45, 30) beyond.
    assert_eq!(owner(&w, 44, 30), Some(P1));
    assert_eq!(owner(&w, 45, 30), None);
    assert!(w.owned_tile_counts()[&P1] > 1500);
    check(&w);
}

#[test]
fn site_does_not_emit_border() {
    let mut w = world("");
    let city = site(&mut w, "bld.city", P1, at(20_500, 30_500));
    site(&mut w, "bld.temple", P1, at(40_000, 30_000));
    assert!(w.owned_tile_counts().is_empty());
    assert!(w.territory.emitters.is_empty());
    complete(&mut w, city);
    assert_eq!(owner(&w, 20, 30), Some(P1));
    assert_eq!(owner(&w, 40, 30), Some(P1));
    check(&w);
}

#[test]
fn contested_tile_goes_to_stronger_emitter_and_ties_to_earlier() {
    // The later completion loses the tie although its id is lower.
    let mut w = world("");
    let late = site(&mut w, "bld.city", P2, at(50_500, 30_500));
    place(&mut w, "bld.city", P1, at(10_500, 30_500));
    for _ in 0..5 {
        w.step();
    }
    complete(&mut w, late);
    assert_eq!(owner(&w, 30, 30), Some(P1)); // 20 tiles from each.
    assert_eq!(owner(&w, 31, 30), Some(P2)); // Stronger: nearer the second city.
    assert_eq!(owner(&w, 29, 30), Some(P1));
    check(&w);
    // Completed on the same tick, the lower id wins.
    for (first, second) in [(P1, P2), (P2, P1)] {
        let mut w = world("");
        place(&mut w, "bld.city", first, at(50_500, 30_500));
        place(&mut w, "bld.city", second, at(10_500, 30_500));
        assert_eq!(owner(&w, 30, 30), Some(first));
        check(&w);
    }
}

#[test]
fn religion_extends_temple_border_on_the_grid() {
    let mut w = world("");
    place(&mut w, "bld.temple", P1, at(30_000, 30_000));
    let before = w.owned_tile_counts()[&P1];
    // (32, 30) has its centre about 2.5 tiles from the temple: outside 2000, inside 4000.
    assert_eq!(owner(&w, 32, 30), None);
    w.complete_research(P1, id(&w, "tech.religion"));
    assert_eq!(owner(&w, 32, 30), Some(P1));
    assert!(w.owned_tile_counts()[&P1] > before);
    assert_eq!(w.territory.emitters.values().next().unwrap().radius, 4000);
    check(&w);
}

#[test]
fn destroying_an_emitter_shrinks_the_border() {
    let mut w = world("");
    let west = place(&mut w, "bld.city", P1, at(10_500, 30_500));
    place(&mut w, "bld.city", P1, at(30_500, 30_500));
    place(&mut w, "bld.city", P2, at(50_500, 30_500));
    assert_eq!(owner(&w, 2, 30), Some(P1));
    assert_eq!(owner(&w, 20, 30), Some(P1));
    let before = w.owned_tile_counts()[&P1];
    w.despawn(west).unwrap();
    assert_eq!(owner(&w, 2, 30), None);
    // Still within the other own city's reach.
    assert_eq!(owner(&w, 20, 30), Some(P1));
    assert!(w.owned_tile_counts()[&P1] < before);
    check(&w);
    let east = w.cities(P2)[0];
    let middle = w.cities(P1)[0];
    w.despawn(middle).unwrap();
    assert_eq!(owner(&w, 30, 30), Some(P2));
    assert!(!w.owned_tile_counts().contains_key(&P1));
    w.despawn(east).unwrap();
    assert!(w.owned_tile_counts().is_empty());
    check(&w);
}

#[test]
fn incremental_equals_full_recompute() {
    let mut w = world("");
    let mut rng = SimRng::new(11);
    let kinds = ["bld.city", "bld.temple", "bld.spaceport", "bld.farm"];
    let religion = id(&w, "tech.religion");
    for _ in 0..200 {
        let live: Vec<EntityId> = w.entities().map(|e| e.id).collect();
        let sites: Vec<EntityId> = w
            .entities()
            .filter(|e| e.build_progress.is_some())
            .map(|e| e.id)
            .collect();
        let pos = at(rng.range(0, 64_000) as i32, rng.range(0, 64_000) as i32);
        let kind = kinds[rng.range(0, kinds.len() as u32) as usize];
        let player = PlayerId(rng.range(1, 4));
        match rng.range(0, 10) {
            0..=2 => {
                w.spawn(id(&w, kind), player, pos).unwrap();
            }
            3..=4 => {
                site(&mut w, kind, player, pos);
            }
            5..=6 if !sites.is_empty() => {
                complete(&mut w, sites[rng.range(0, sites.len() as u32) as usize]);
            }
            7..=8 if !live.is_empty() => {
                w.despawn(live[rng.range(0, live.len() as u32) as usize])
                    .unwrap();
            }
            9 => w.complete_research(player, religion),
            _ => w.step(),
        }
        check(&w);
    }
    assert!(w.territory.emitters.len() > 3);
}

#[test]
fn space_tiles_are_never_owned() {
    let mut w = world_on(
        rules(""),
        |x, _| {
            if x >= 40 { "ter.space" } else { "ter.plain" }
        },
    );
    place(&mut w, "bld.city", P1, at(30_500, 30_500));
    assert_eq!(owner(&w, 39, 30), Some(P1));
    for x in 40..55 {
        assert_eq!(owner(&w, x, 30), None);
    }
    check(&w);
}

#[test]
fn farm_outside_city_radius_rejected() {
    let mut w = world("");
    place(&mut w, "bld.city", P1, at(20_500, 30_500));
    assert_eq!(
        construct(&mut w, P1, "bld.farm", at(50_000, 30_000)),
        Err(SimError::NotInCityRadius)
    );
    construct(&mut w, P1, "bld.farm", at(30_000, 30_000)).unwrap();
    w.step();
    let farm = w
        .entities()
        .find(|e| e.type_id == id(&w, "bld.farm"))
        .unwrap();
    assert_eq!(farm.city_id, Some(w.cities(P1)[0]));
}

#[test]
fn barracks_outside_territory_rejected() {
    let mut w = world("");
    place(&mut w, "bld.city", P1, at(20_500, 30_500));
    assert_eq!(
        construct(&mut w, P1, "bld.barracks", at(46_000, 30_000)),
        Err(SimError::NotInOwnTerritory)
    );
    construct(&mut w, P1, "bld.barracks", at(40_000, 30_000)).unwrap();
}

#[test]
fn second_library_in_same_city_rejected() {
    let mut w = world("UPDATE game_constant SET value=3 WHERE key='player.city_limit.base';");
    place(&mut w, "bld.city", P1, at(20_500, 20_500));
    place(&mut w, "bld.city", P1, at(20_500, 50_500));
    construct(&mut w, P1, "bld.library", at(26_000, 20_000)).unwrap();
    // Still queued, not yet a site: it counts.
    assert_eq!(
        construct(&mut w, P1, "bld.library", at(14_000, 20_000)),
        Err(SimError::LimitReached)
    );
    w.step();
    assert_eq!(
        construct(&mut w, P1, "bld.library", at(14_000, 20_000)),
        Err(SimError::LimitReached)
    );
    // The other city has its own library allowance.
    construct(&mut w, P1, "bld.library", at(26_000, 54_000)).unwrap();
}

#[test]
fn city_limit_enforced() {
    let mut w = world("");
    place(&mut w, "bld.city", P1, at(20_500, 30_500));
    assert_eq!(
        construct(&mut w, P1, "bld.city", at(50_500, 30_500)),
        Err(SimError::CityLimit)
    );
    let mut w = world(
        "UPDATE game_constant SET value=2 WHERE key='player.city_limit.base';
         UPDATE et_territory SET border_radius=90000 WHERE is_city=1;",
    );
    place(&mut w, "bld.city", P1, at(20_500, 30_500));
    construct(&mut w, P1, "bld.city", at(50_500, 30_500)).unwrap();
    // Queued and site cities both count toward the limit.
    assert_eq!(
        construct(&mut w, P1, "bld.city", at(50_500, 58_500)),
        Err(SimError::CityLimit)
    );
    w.step();
    assert_eq!(
        construct(&mut w, P1, "bld.city", at(50_500, 58_500)),
        Err(SimError::CityLimit)
    );
}

#[test]
fn cities_too_close_rejected() {
    let mut w = world(
        "UPDATE game_constant SET value=5 WHERE key='player.city_limit.base';
         UPDATE et_territory SET border_radius=90000 WHERE is_city=1;",
    );
    place(&mut w, "bld.city", P1, at(20_500, 20_500));
    place(&mut w, "bld.city", P2, at(50_500, 50_500));
    assert_eq!(
        construct(&mut w, P1, "bld.city", at(40_500, 20_500)),
        Err(SimError::CitiesTooClose)
    );
    // Spacing holds against every owner's cities.
    assert_eq!(
        construct(&mut w, P1, "bld.city", at(50_500, 30_500)),
        Err(SimError::CitiesTooClose)
    );
    // Exactly the spacing apart (24 tiles) is allowed.
    construct(&mut w, P1, "bld.city", at(20_500, 44_500)).unwrap();
}

#[test]
fn first_city_may_be_placed_without_territory() {
    let mut w = world("");
    assert!(w.owned_tile_counts().is_empty());
    construct(&mut w, P1, "bld.city", at(40_500, 30_500)).unwrap();
    // A second one is no longer the first: the limit applies.
    assert_eq!(
        construct(&mut w, P1, "bld.city", at(10_500, 10_500)),
        Err(SimError::CityLimit)
    );
    // The exemption does not lift the spacing rule.
    assert_eq!(
        construct(&mut w, P2, "bld.city", at(40_500, 40_500)),
        Err(SimError::CitiesTooClose)
    );
    construct(&mut w, P2, "bld.city", at(10_500, 10_500)).unwrap();
}

/// One placement rule: the extra rows, what to set up, where placement fails, where it passes.
struct Case {
    sql: &'static str,
    subject: &'static str,
    prepare: fn(&mut World),
    fail: Pos,
    error: SimError,
    pass: Pos,
}

#[test]
fn each_placement_rule_kind_is_enforced() {
    let wide = "UPDATE et_territory SET border_radius=90000 WHERE is_city=1;";
    let rule = |sql: &'static str| sql;
    let cases = [
        Case {
            sql: "",
            subject: "bld.barracks",
            prepare: |_| {},
            fail: at(56_000, 30_000),
            error: SimError::NotInOwnTerritory,
            pass: at(36_000, 30_000),
        },
        Case {
            sql: "",
            subject: "bld.library",
            prepare: |_| {},
            fail: at(54_000, 30_000),
            error: SimError::NotInCityRadius,
            pass: at(36_000, 30_000),
        },
        Case {
            sql: rule(
                "INSERT INTO placement_rule(entity_type_id,rule,ref_def_id)
                 SELECT b.id,'on_terrain',t.id FROM def b,def t WHERE b.key='bld.barracks' AND t.key='ter.plain';",
            ),
            subject: "bld.barracks",
            prepare: |_| {},
            fail: at(9000, 9000),
            error: SimError::NotOnTerrain,
            pass: at(20_000, 20_000),
        },
        Case {
            sql: rule(
                "INSERT INTO placement_rule(entity_type_id,rule,ref_def_id)
                 SELECT b.id,'adjacent_terrain',t.id FROM def b,def t WHERE b.key='bld.barracks' AND t.key='ter.forest';",
            ),
            subject: "bld.barracks",
            prepare: |_| {},
            fail: at(20_000, 20_000),
            error: SimError::NotAdjacentTerrain,
            pass: at(48_000, 20_000),
        },
        Case {
            sql: rule(
                "INSERT INTO placement_rule(entity_type_id,rule,ref_def_id,distance)
                 SELECT b.id,'near_type',t.id,3000 FROM def b,def t WHERE b.key='bld.barracks' AND t.key='unit.spearman';",
            ),
            subject: "bld.barracks",
            prepare: |w| {
                // Another player's spearman does not count.
                w.spawn(id(w, "unit.spearman"), P2, at(20_000, 22_000))
                    .unwrap();
                w.spawn(id(w, "unit.spearman"), P1, at(20_000, 42_500))
                    .unwrap();
            },
            fail: at(20_000, 20_000),
            error: SimError::NotNearType,
            pass: at(20_000, 40_000),
        },
        Case {
            sql: rule(
                "INSERT INTO placement_rule(entity_type_id,rule,ref_def_id,distance)
                 SELECT b.id,'min_distance_type',t.id,10000 FROM def b,def t WHERE b.key='bld.barracks' AND t.key='bld.farm';",
            ),
            subject: "bld.barracks",
            prepare: |w| {
                w.spawn(id(w, "bld.farm"), P2, at(20_000, 28_000)).unwrap();
            },
            fail: at(20_000, 20_000),
            error: SimError::TooCloseToType,
            pass: at(20_000, 10_000),
        },
        Case {
            sql: rule(
                "INSERT INTO placement_rule(entity_type_id,rule,distance)
                 SELECT id,'min_distance_enemy_city',20000 FROM def WHERE key='bld.barracks';",
            ),
            subject: "bld.barracks",
            prepare: |w| {
                place(w, "bld.city", P2, at(6500, 54_500));
                // An ally's city does not count.
                place(w, "bld.city", PlayerId(3), at(50_500, 30_500));
            },
            fail: at(20_000, 40_000),
            error: SimError::TooCloseToEnemyCity,
            pass: at(38_000, 30_000),
        },
        Case {
            sql: rule(
                "INSERT INTO placement_rule(entity_type_id,rule,ref_def_id)
                 SELECT b.id,'on_node',t.id FROM def b,def t WHERE b.key='bld.barracks' AND t.key='node.relics';",
            ),
            subject: "bld.barracks",
            prepare: |w| {
                w.spawn(id(w, "node.relics"), P1, at(20_500, 40_500))
                    .unwrap();
            },
            fail: at(20_000, 20_000),
            error: SimError::NotOnNode,
            pass: at(20_000, 40_000),
        },
        Case {
            sql: rule(
                "INSERT INTO placement_rule(entity_type_id,rule,layer)
                 SELECT id,'on_layer','fluid' FROM def WHERE key='bld.barracks';",
            ),
            subject: "bld.barracks",
            prepare: |_| {},
            fail: at(20_000, 20_000),
            error: SimError::NotOnLayer,
            pass: at(9000, 9000),
        },
    ];
    for case in cases {
        let sql = if case.sql.is_empty() {
            String::new()
        } else {
            format!(
                "{wide}DELETE FROM placement_rule WHERE rule='on_terrain' AND entity_type_id=(SELECT id FROM def WHERE key='bld.barracks');{}",
                case.sql
            )
        };
        let mut w = world_on(rules(&sql), |x, y| {
            if (5..=12).contains(&x) && (5..=12).contains(&y) {
                "ter.coast"
            } else if x == 50 {
                "ter.forest"
            } else {
                "ter.plain"
            }
        });
        place(&mut w, "bld.city", P1, at(30_500, 30_500));
        (case.prepare)(&mut w);
        let hash = w.hash();
        assert_eq!(
            construct(&mut w, P1, case.subject, case.fail).map_err(|e| (e, hash)),
            Err((case.error, hash)),
        );
        construct(&mut w, P1, case.subject, case.pass)
            .unwrap_or_else(|e| panic!("{} at {:?}: {e}", case.subject, case.pass));
    }
}

#[test]
fn hash_covers_borders_cities_and_fog() {
    let mut w = world("");
    let city = place(&mut w, "bld.city", P1, at(20_500, 30_500));
    let farm = place(&mut w, "bld.farm", P1, at(26_000, 30_000));
    place(&mut w, "bld.farm", P2, at(30_000, 30_000));
    w.spawn(id(&w, "unit.spearman"), P2, at(31_000, 33_000))
        .unwrap();
    w.step();
    assert_eq!(w.city_of(farm), Some(city));
    assert!(w.remembered_buildings(P1).next().is_some());
    fn changes(w: &mut World, change: impl FnOnce(&mut World)) {
        let before = w.hash();
        change(w);
        assert_ne!(w.hash(), before);
    }
    changes(&mut w, |w| w.surfaces[0].territory.winner[0] = city.0);
    changes(&mut w, |w| {
        w.territory.emitters.get_mut(&city).unwrap().completed += 1
    });
    changes(&mut w, |w| *w.territory.owned.get_mut(&P1).unwrap() += 1);
    changes(&mut w, |w| {
        w.territory.cities.insert(EntityId(99));
    });
    changes(&mut w, |w| {
        w.entities[farm.0 as usize].as_mut().unwrap().city_id = None
    });
    changes(&mut w, |w| {
        w.surfaces[0].vision.players.get_mut(&P1).unwrap().count[0] += 1
    });
    changes(&mut w, |w| {
        w.surfaces[0].vision.players.get_mut(&P2).unwrap().explored[0] ^= 1
    });
    changes(&mut w, |w| {
        let memory = &mut w.surfaces[0].vision.players.get_mut(&P1).unwrap().memory;
        memory.values_mut().next().unwrap().pos.x += 1;
    });
    changes(&mut w, |w| w.vision.viewers[1].as_mut().unwrap().los += 1);
    changes(&mut w, |w| w.surfaces[0].vision.touched.push(3));
    changes(&mut w, |w| {
        w.surfaces[0]
            .vision
            .buildings
            .entry(5)
            .or_default()
            .insert(EntityId(1));
    });
}

/// A city's economic radius recounted from the entities alone.
fn recount(w: &World, city: EntityId) -> i64 {
    let c = w.entity(city).unwrap();
    let base = w.base_economic_radius(city);
    let per = w.rules.constant("city.radius_per_building").unwrap();
    let max = w.rules.constant("city.radius_from_buildings_max").unwrap();
    let count = w
        .entities()
        .filter(|e| {
            e.owner == c.owner
                && w.rules.entity_types[&e.type_id].class == rom_rules::EntityClass::Building
                && !w.is_city_type(e.type_id)
                && e.build_progress.is_none()
                && !e.neutralised
                && w.geo(c.pos.body).within(c.pos, e.pos, base)
        })
        .count() as i64;
    base + max.min(per * count)
}

#[test]
fn civic_levels_compound_border_and_city_radius() {
    let mut w = world_on(
        {
            // The original geometry, without the building bonus.
            let db = build_rules_db(
                include_str!("../../../../db/schema.sql"),
                include_str!("../../../../db/seed.sql"),
            )
            .unwrap();
            db.execute_batch(
                "UPDATE game_constant SET value=0 WHERE key='city.radius_per_building';",
            )
            .unwrap();
            Arc::new(Rules::load(&db).unwrap())
        },
        |_, _| "ter.plain",
    );
    let city = place(&mut w, "bld.city", P1, at(20_500, 30_500));
    assert_eq!(w.city_economic_radius(city), 16_000);
    assert_eq!(w.territory.emitters[&city].radius, 40_000);
    let (mut border, mut economic) = (40_000_i64, 16_000_i64);
    for level in 1..=3 {
        w.complete_research(P1, id(&w, &format!("tech.civic.{level}")));
        border = border * 106 / 100;
        economic = economic * 106 / 100;
        assert_eq!(w.territory.emitters[&city].radius, border, "level {level}");
        assert_eq!(w.city_economic_radius(city), economic, "level {level}");
    }
    // Compounding beats the old additive 5 % per level.
    assert!(border > 40_000 + 3 * 2_000);
    assert!(economic > 16_000 + 3 * 800);
    check(&w);
}

#[test]
fn buildings_widen_the_city_radius_up_to_the_cap() {
    let mut w = world(
        "UPDATE game_constant SET value=3000 WHERE key='city.radius_per_building'; UPDATE game_constant SET value=8000 WHERE key='city.radius_from_buildings_max';",
    );
    let city = place(&mut w, "bld.city", P1, at(20_500, 30_500));
    let border = w.territory.emitters[&city].radius;
    assert_eq!(w.city_economic_radius(city), 20_000);
    place(&mut w, "bld.farm", P1, at(24_000, 30_500));
    assert_eq!(w.city_economic_radius(city), 23_000);
    place(&mut w, "bld.granary", P1, at(24_000, 34_500));
    assert_eq!(w.city_economic_radius(city), 26_000);
    // Outside the radius as it was before the bonus (20000 from the city): not counted.
    place(&mut w, "bld.barracks", P1, at(42_000, 30_500));
    assert_eq!(w.city_economic_radius(city), 26_000);
    // A site does not count; another player's building does not count.
    site(&mut w, "bld.farm", P1, at(28_000, 30_500));
    place(&mut w, "bld.farm", P2, at(24_000, 26_500));
    assert_eq!(w.city_economic_radius(city), 26_000);
    place(&mut w, "bld.farm", P1, at(28_000, 34_500));
    assert_eq!(w.city_economic_radius(city), 28_000);
    place(&mut w, "bld.farm", P1, at(28_000, 26_500));
    assert_eq!(w.city_economic_radius(city), 28_000, "capped at +8000");
    assert_eq!(w.city_economic_radius(city), recount(&w, city));
    // The national border is untouched by buildings (barracks emits none).
    assert_eq!(w.territory.emitters[&city].radius, border);
    check(&w);
}

#[test]
fn destroyed_building_shrinks_the_city_radius() {
    let mut w = world(
        "UPDATE game_constant SET value=1000 WHERE key='city.radius_per_building'; UPDATE game_constant SET value=8000 WHERE key='city.radius_from_buildings_max';",
    );
    let city = place(&mut w, "bld.city", P1, at(20_500, 30_500));
    let a = place(&mut w, "bld.farm", P1, at(24_000, 30_500));
    let b = place(&mut w, "bld.granary", P1, at(24_000, 34_500));
    assert_eq!(w.city_economic_radius(city), 22_000);
    w.despawn(a).unwrap();
    assert_eq!(w.city_economic_radius(city), 21_000);
    assert_eq!(w.city_economic_radius(city), recount(&w, city));
    w.despawn(b).unwrap();
    assert_eq!(w.city_economic_radius(city), 20_000);
    assert_eq!(w.city_economic_radius(city), recount(&w, city));
}

#[test]
fn military_and_spaceport_placeable_outside_city_radius() {
    let mut w = world("UPDATE et_territory SET economic_radius=12000 WHERE is_city=1;");
    let city = place(&mut w, "bld.city", P1, at(20_500, 30_500));
    let outside = at(36_000, 30_000);
    assert!(!w.geo(outside.body).within(
        w.entity(city).unwrap().pos,
        outside,
        w.city_economic_radius(city)
    ));
    assert_eq!(w.territory_owner(36, 30), Some(P1));
    assert_eq!(
        construct(&mut w, P1, "bld.farm", outside),
        Err(SimError::NotInCityRadius)
    );
    construct(&mut w, P1, "bld.barracks", outside).unwrap();
    let mut w = world("UPDATE et_territory SET economic_radius=12000 WHERE is_city=1;");
    place(&mut w, "bld.city", P1, at(20_500, 30_500));
    // The spaceport is gated by age, not by the city radius.
    let result = w.check_placement(P1, id(&w, "bld.spaceport"), outside);
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn territory_scenario_is_deterministic() {
    fn run() -> u64 {
        let mut w = world("UPDATE et_territory SET economic_radius=12000 WHERE is_city=1;");
        place(&mut w, "bld.city", P1, at(20_500, 30_500));
        place(&mut w, "bld.farm", P1, at(24_000, 30_500));
        place(&mut w, "bld.granary", P1, at(24_000, 34_500));
        w.complete_research(P1, id(&w, "tech.civic.1"));
        construct(&mut w, P1, "bld.barracks", at(36_000, 30_000)).unwrap();
        for _ in 0..200 {
            w.step();
        }
        w.hash()
    }
    assert_eq!(run(), run());
}
