use rom_rules::{
    ActiveSources, ClaimMode, Condition, Context, DefId, Layer, ModifierOp, Owned, RegionKind,
    Rules, RulesError, build_rules_db,
};
use rusqlite::Connection;

const SCHEMA: &str = include_str!("../../../db/schema.sql");
const SEED: &str = include_str!("../../../db/seed.sql");
const TINY: &str = "
    INSERT INTO def(id,kind,key,name) VALUES
        (10,'entity_type','node.test','Test node'),
        (20,'tech','tech.test','Test technology'),
        (30,'tech','tech.other','Other technology'),
        (40,'resource','res.food','Food'),
        (41,'resource','res.timber','Timber');
    INSERT INTO entity_type(id,class,build_ticks) VALUES (10,'node',123);
    INSERT INTO tech(id,kind,research_ticks) VALUES (20,'upgrade',10),(30,'upgrade',10);
    INSERT INTO resource(id,kind,abbrev) VALUES (40,'base','F'),(41,'base','T');
    INSERT INTO tag(id,key) VALUES (1,'test');
    INSERT INTO def_tag VALUES (10,1);
    INSERT INTO stat(key,scope) VALUES
        ('hp','entity'),('build_time','entity'),('cost.all','entity'),
        ('cost.food','entity'),('cost.timber','entity'),('research_time','player');
";

fn seed_db() -> Connection {
    build_rules_db(SCHEMA, SEED).unwrap()
}

fn seed() -> Rules {
    Rules::load(&seed_db()).unwrap()
}

fn tiny(extra: &str) -> Rules {
    let conn = build_rules_db(SCHEMA, &format!("{TINY}\n{extra}")).unwrap();
    Rules::load(&conn).unwrap()
}

fn sources(rules: &Rules, keys: &[&str]) -> ActiveSources {
    keys.iter().map(|key| rules.id_of(key).unwrap()).collect()
}

fn stat(rules: &Rules, active: &ActiveSources) -> i64 {
    rules.entity_stat("hp", DefId(10), 100, active, &Context::default())
}

#[test]
fn loads_seed() {
    let rules = seed();
    let citizen = rules
        .entity_type(rules.id_of("unit.citizen").unwrap())
        .unwrap();
    assert!(citizen.movement.is_some());
    assert!(citizen.builder.is_some());
    assert_eq!(citizen.harvester.len(), 4);
    let temple = rules.id_of("bld.temple").unwrap();
    assert!(rules.entity_type(temple).unwrap().producer.is_some());
    assert_eq!(rules.key(temple), "bld.temple");
    assert!(rules.definition_count() > 50);
    assert_eq!(rules.constant("sim.ticks_per_second"), Some(20));
    assert_eq!(rules.constant("absent"), None);
    let melee = rules
        .damage_classes
        .values()
        .find(|row| row.key == "melee")
        .unwrap()
        .id;
    let structure = rules
        .armor_classes
        .values()
        .find(|row| row.key == "structure")
        .unwrap()
        .id;
    assert_eq!(rules.damage_pct(melee, structure), 50);
    assert_eq!(rules.damage_pct(999, 999), 100);
    let priest = rules.id_of("unit.priest").unwrap();
    assert!(
        rules.production[&temple]
            .iter()
            .any(|row| row.product_def_id == priest)
    );
    assert!(
        rules.production_by_product[&priest]
            .iter()
            .any(|row| row.producer_type_id == temple)
    );
    assert_eq!(citizen.weapons[0].targets[0].layer, Layer::Ground);
}

#[test]
fn seed_validates() {
    assert_eq!(seed().validate(), vec![]);
}

#[test]
fn abundance_makes_everything_free() {
    let rules = seed();
    let active = sources(&rules, &["tech.age.abundance"]);
    let spearman = rules.id_of("unit.spearman").unwrap();
    assert_eq!(rules.cost_of(spearman, &active).len(), 2);
    assert!(
        rules
            .cost_of(spearman, &active)
            .iter()
            .all(|(_, amount)| *amount == 0)
    );
    assert_eq!(
        rules.build_ticks(spearman, &active),
        rules.build_ticks(spearman, &ActiveSources::new())
    );
    for def in rules.definitions.iter().flatten() {
        assert!(
            rules
                .cost_of(def.id, &active)
                .iter()
                .all(|(_, amount)| *amount == 0),
            "{}",
            def.key
        );
    }
}

#[test]
fn religion_extends_temple_border() {
    let rules = seed();
    let active = sources(&rules, &["tech.religion"]);
    for (key, expected) in [("bld.temple", 4000), ("bld.barracks", 2000)] {
        assert_eq!(
            rules.entity_stat(
                "border_radius",
                rules.id_of(key).unwrap(),
                2000,
                &active,
                &Context::default()
            ),
            expected
        );
    }
}

#[test]
fn religion_pct_on_city_hp() {
    let rules = seed();
    let active = sources(&rules, &["tech.religion"]);
    assert_eq!(
        rules.entity_stat(
            "hp",
            rules.id_of("bld.city").unwrap(),
            1200,
            &active,
            &Context::default()
        ),
        1500
    );
}

#[test]
fn relics_are_wealth_nodes_instead_of_rare_research_bonuses() {
    let rules = seed();
    assert!(rules.id_of("rare.relics").is_none());
    let node = rules.entity_types[&rules.id_of("node.relics").unwrap()]
        .node
        .as_ref()
        .unwrap();
    assert_eq!(node.resource_id, rules.id_of("res.wealth").unwrap());
    assert_eq!(node.income, 20);
}

#[test]
fn op_order_set_add_pct_mul() {
    let rules = tiny(
        "INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES
        (20,'hp','mul',133),(20,'hp','pct',17),(20,'hp','add',9),(20,'hp','set',101);",
    );
    // (101 + 9) * 117 / 100 = 128; 128 * 133 / 100 = 170.
    assert_eq!(stat(&rules, &ActiveSources::from([DefId(20)])), 170);
}

#[test]
fn largest_set_wins_and_negative_rounding_is_toward_zero() {
    let rules = tiny(
        "INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES
        (20,'hp','set',-11),(20,'hp','set',-9),(20,'hp','pct',50),
        (20,'hp','mul',150),(20,'hp','mul',50);",
    );
    // -9 * 150 / 100 = -13; -13 * 150 / 100 = -19; -19 * 50 / 100 = -9.
    assert_eq!(stat(&rules, &ActiveSources::from([DefId(20)])), -9);
}

#[test]
fn stack_group_keeps_strongest() {
    let rules = tiny(
        "INSERT INTO modifier(source_def_id,stat_key,op,value,stack_group) VALUES
        (20,'hp','pct',10,'same'),(30,'hp','pct',25,'same');",
    );
    assert_eq!(
        stat(&rules, &ActiveSources::from([DefId(20), DefId(30)])),
        125
    );
}

#[test]
fn stack_groups_compare_absolute_values_and_keep_ops_separate() {
    let rules = tiny(
        "INSERT INTO modifier(source_def_id,stat_key,op,value,stack_group) VALUES
        (20,'hp','pct',10,'same'),(30,'hp','pct',-25,'same'),
        (20,'hp','add',20,'same'),(30,'hp','add',-5,'same');",
    );
    assert_eq!(
        stat(&rules, &ActiveSources::from([DefId(20), DefId(30)])),
        90
    );
}

#[test]
fn stack_group_ties_use_lowest_modifier_id() {
    let rules = tiny(
        "INSERT INTO modifier(id,source_def_id,stat_key,op,value,stack_group) VALUES
        (2,20,'hp','pct',25,'same'),(1,30,'hp','pct',-25,'same');",
    );
    assert_eq!(
        stat(&rules, &ActiveSources::from([DefId(20), DefId(30)])),
        75
    );
}

#[test]
fn condition_requires_context_flag() {
    for (condition, context) in [
        (
            "in_own_territory",
            Context {
                in_own_territory: true,
                ..Context::default()
            },
        ),
        (
            "in_enemy_territory",
            Context {
                in_enemy_territory: true,
                ..Context::default()
            },
        ),
        (
            "garrisoned",
            Context {
                garrisoned: true,
                ..Context::default()
            },
        ),
        (
            "near_supply",
            Context {
                near_supply: true,
                ..Context::default()
            },
        ),
        (
            "entrenched",
            Context {
                entrenched: true,
                ..Context::default()
            },
        ),
        (
            "on_high_ground",
            Context {
                on_high_ground: true,
                ..Context::default()
            },
        ),
    ] {
        let rules = tiny(&format!(
            "INSERT INTO modifier(source_def_id,stat_key,op,value,condition) VALUES (20,'hp','add',10,'{condition}');"
        ));
        let active = ActiveSources::from([DefId(20)]);
        assert_eq!(stat(&rules, &active), 100, "{condition}");
        assert_eq!(
            rules.entity_stat("hp", DefId(10), 100, &active, &context),
            110,
            "{condition}"
        );
    }
}

#[test]
fn simulation_dependent_effects_are_exposed_but_not_applied() {
    let rules = tiny("INSERT INTO modifier(source_def_id,stat_key,op,value,aura_radius,duration_ticks,applies_to) VALUES
        (20,'hp','add',10,0,NULL,'owner'),(20,'hp','add',20,NULL,0,'owner'),
        (20,'hp','add',30,NULL,NULL,'allies'),(20,'hp','add',40,NULL,NULL,'enemies'),
        (20,'hp','add',50,NULL,NULL,'everyone');");
    assert_eq!(stat(&rules, &ActiveSources::from([DefId(20)])), 100);
    assert_eq!(rules.modifiers_from(DefId(20)).len(), 5);
    assert_eq!(rules.modifiers_from(DefId(999)).len(), 0);
    assert_eq!(rules.modifiers_from(DefId(20))[0].aura_radius, Some(0));
    assert_eq!(rules.modifiers_from(DefId(20))[1].duration_ticks, Some(0));
}

#[test]
fn targeting_accepts_exact_def_or_tag_or_no_target() {
    let rules = tiny(
        "INSERT INTO modifier(source_def_id,stat_key,op,value,target_def_id,target_tag_id) VALUES
        (20,'hp','add',1,10,NULL),(20,'hp','add',2,NULL,1),
        (20,'hp','add',4,NULL,NULL),(20,'hp','add',8,20,NULL),
        (20,'hp','add',16,20,1);",
    );
    assert_eq!(stat(&rules, &ActiveSources::from([DefId(20)])), 123);
    assert_eq!(stat(&rules, &ActiveSources::new()), 100);
}

#[test]
fn mul_steps_follow_modifier_ids_across_sources() {
    let rules = tiny(
        "INSERT INTO modifier(id,source_def_id,stat_key,op,value) VALUES
        (1,30,'hp','mul',133),(2,20,'hp','mul',150);",
    );
    assert_eq!(
        rules.entity_stat(
            "hp",
            DefId(10),
            1,
            &ActiveSources::from([DefId(20), DefId(30)]),
            &Context::default()
        ),
        1
    );
}

#[test]
fn costs_apply_all_then_resource_stat_and_clamp_to_zero() {
    let rules = tiny(
        "INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES (10,40,100,999),(10,41,5,999);
        INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES
        (20,'cost.all','add',10),(20,'cost.food','pct',50),(20,'cost.timber','add',-100),
        (20,'build_time','pct',-50);",
    );
    let active = ActiveSources::from([DefId(20)]);
    assert_eq!(
        rules.cost_of(DefId(10), &active),
        vec![(DefId(40), 165), (DefId(41), 0)]
    );
    assert_eq!(rules.build_ticks(DefId(10), &active), 61);
}

#[test]
fn extreme_integer_values_do_not_wrap() {
    let rules = tiny(
        "INSERT INTO modifier(source_def_id,stat_key,op,value,stack_group) VALUES
        (20,'hp','pct',-9223372036854775808,'extreme'),
        (30,'hp','pct',9223372036854775807,'extreme');",
    );
    assert_eq!(
        stat(&rules, &ActiveSources::from([DefId(20), DefId(30)])),
        i64::MIN + 100
    );
}

#[test]
fn priest_needs_classical_age() {
    let rules = seed();
    let priest = rules.id_of("unit.priest").unwrap();
    assert!(!rules.is_unlocked(priest, &ActiveSources::new(), &Owned::new()));
    assert!(rules.is_unlocked(priest, &sources(&rules, &["age.classical"]), &Owned::new()));
}

#[test]
fn age_requirement_met_by_later_age() {
    let rules = seed();
    let temple = rules.id_of("bld.temple").unwrap();
    for (age, expected) in [
        ("age.ancient", false),
        ("age.classical", true),
        ("age.medieval", true),
    ] {
        assert_eq!(
            rules.is_unlocked(
                temple,
                &sources(&rules, &[age, "tech.civic.1"]),
                &Owned::new()
            ),
            expected
        );
    }
}

#[test]
fn prerequisites_are_and_of_or_groups_and_owned_types_are_distinct_from_sources() {
    let rules = tiny("INSERT INTO prerequisite VALUES (30,20,0),(30,10,0),(30,40,1);");
    // A resource is not a supported prerequisite, even if active or owned.
    assert!(!rules.is_unlocked(
        DefId(30),
        &ActiveSources::from([DefId(20), DefId(40)]),
        &Owned::from([DefId(40)])
    ));
    let rules = tiny(
        "INSERT INTO def(id,kind,key,name) VALUES (11,'entity_type','node.other','Other node');
        INSERT INTO entity_type(id,class) VALUES (11,'node');
        INSERT INTO prerequisite VALUES (30,20,0),(30,10,0),(30,11,1);",
    );
    assert!(!rules.is_unlocked(
        DefId(30),
        &ActiveSources::from([DefId(20), DefId(10)]),
        &Owned::new()
    ));
    assert!(rules.is_unlocked(
        DefId(30),
        &ActiveSources::new(),
        &Owned::from([DefId(10), DefId(11)])
    ));
    assert!(rules.is_unlocked(DefId(20), &ActiveSources::new(), &Owned::new()));
}

#[test]
fn availability_inherits_from_parent_faction() {
    let rules = tiny("INSERT INTO def(id,kind,key,name) VALUES
        (50,'faction','race.test','Test race'),(51,'faction','bloc.test','Test bloc'),
        (52,'faction','nation.test','Test nation'),(53,'faction','nation.other','Other nation');
        INSERT INTO faction(id,kind,parent_id) VALUES (50,'race',NULL),(51,'bloc',50),(52,'nation',51),(53,'nation',NULL);
        INSERT INTO availability VALUES (10,50);");
    assert!(rules.is_available(DefId(10), DefId(52)));
    assert!(rules.is_available(DefId(10), DefId(50)));
    assert!(!rules.is_available(DefId(10), DefId(53)));
    assert!(rules.is_available(DefId(20), DefId(53)));
}

fn assert_problem(extra: &str, invariant: &str, keys: &[&str]) {
    let conn = seed_db();
    conn.execute_batch(extra).unwrap();
    let problems = Rules::load(&conn).unwrap().validate();
    assert!(
        problems
            .iter()
            .any(|problem| problem.message.contains(invariant)
                && keys.iter().all(|key| problem.message.contains(key))),
        "{problems:#?}"
    );
}

#[test]
fn prerequisite_cycle_is_reported() {
    assert_problem(
        "INSERT INTO prerequisite SELECT a.id,b.id,0 FROM def a,def b
        WHERE (a.key='tech.religion' AND b.key='unit.priest');
        INSERT INTO prerequisite SELECT a.id,b.id,1 FROM def a,def b WHERE a.key='unit.priest' AND b.key='tech.religion';",
        "prerequisite cycle",
        &["tech.religion", "unit.priest"],
    );
}

#[test]
fn prerequisite_cycles_of_any_length_and_upgrade_loops_are_reported() {
    assert_problem(
        "INSERT INTO prerequisite SELECT a.id,b.id,0 FROM def a,def b WHERE
        (a.key='tech.religion' AND b.key='bld.temple') OR
        (a.key='age.classical' AND b.key='unit.priest');
        INSERT INTO prerequisite SELECT a.id,b.id,1 FROM def a,def b WHERE a.key='unit.priest' AND b.key='tech.religion';",
        "prerequisite cycle",
        &[
            "tech.religion",
            "unit.priest",
            "bld.temple",
            "age.classical",
        ],
    );
    assert_problem(
        "UPDATE entity_type SET upgrades_from_id=id WHERE id=(SELECT id FROM def WHERE key='unit.citizen');",
        "upgrade chain cycle",
        &["unit.citizen"],
    );
    assert_problem("UPDATE entity_type SET upgrades_from_id=(SELECT id FROM def WHERE key='unit.priest') WHERE id=(SELECT id FROM def WHERE key='unit.citizen');
        UPDATE entity_type SET upgrades_from_id=(SELECT id FROM def WHERE key='unit.citizen') WHERE id=(SELECT id FROM def WHERE key='unit.priest');", "upgrade chain cycle", &["unit.citizen", "unit.priest"]);
}

#[test]
fn production_mode_mismatch_is_reported() {
    assert_problem(
        "UPDATE production SET mode='research' WHERE product_def_id=(SELECT id FROM def WHERE key='unit.priest');",
        "production",
        &["unit.priest", "bld.temple"],
    );
    assert_problem(
        "UPDATE production SET mode='train' WHERE product_def_id=(SELECT id FROM def WHERE key='tech.religion');",
        "production",
        &["tech.religion"],
    );
}

#[test]
fn missing_production_components_are_reported() {
    assert_problem(
        "DELETE FROM et_producer WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.temple');",
        "production",
        &["bld.temple", "unit.priest"],
    );
    assert_problem(
        "DELETE FROM et_builder;",
        "production",
        &["unit.citizen", "bld.temple"],
    );
}

#[test]
fn weapon_without_target_is_reported() {
    assert_problem(
        "DELETE FROM et_weapon_target WHERE weapon_id IN (SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.spearman'));",
        "no target layers",
        &["unit.spearman"],
    );
}

#[test]
fn targeted_player_stat_is_reported() {
    assert_problem(
        "INSERT INTO modifier(source_def_id,stat_key,op,value,target_def_id) SELECT id,'research_time','add',1,id FROM def WHERE key='tech.religion';",
        "player stat",
        &["research_time", "tech.religion"],
    );
    assert_problem(
        "INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) SELECT id,'research_time','add',1,(SELECT id FROM tag WHERE key='city') FROM def WHERE key='tech.religion';",
        "player stat",
        &["research_time", "tech.religion", "city"],
    );
}

#[test]
fn wrong_tech_effect_targets_are_reported() {
    for effect in ["set_age", "unlock", "change_faction", "upgrade_type"] {
        for target in ["NULL", "(SELECT id FROM def WHERE key='res.food')"] {
            assert_problem(
                &format!(
                    "INSERT INTO tech_effect(tech_id,effect,target_def_id) SELECT id,'{effect}',{target} FROM def WHERE key='tech.religion';"
                ),
                "tech_effect",
                &["tech.religion"],
            );
        }
    }
}

#[test]
fn non_tech_faction_trigger_is_reported() {
    assert_problem(
        "UPDATE faction_transition SET trigger_tech_id=(SELECT id FROM def WHERE key='unit.citizen');",
        "faction_transition",
        &["nation.romans", "bloc.europe", "unit.citizen"],
    );
}

#[test]
fn unknown_enum_value_is_a_load_error() {
    let conn = seed_db();
    conn.execute_batch("PRAGMA ignore_check_constraints=ON; UPDATE entity_type SET class='unknown' WHERE id=(SELECT id FROM def WHERE key='unit.citizen');").unwrap();
    let citizen: i64 = conn
        .query_row("SELECT id FROM def WHERE key='unit.citizen'", [], |row| {
            row.get(0)
        })
        .unwrap();
    let error = Rules::load(&conn).unwrap_err();
    match error {
        RulesError::InvalidValue {
            table,
            row,
            column,
            value,
            ..
        } => {
            assert_eq!(table, "entity_type");
            assert_eq!(row, format!("id={citizen}"));
            assert_eq!(column, "class");
            assert_eq!(value, "unknown");
        }
        other => panic!("unexpected error: {other}"),
    }
}

#[test]
fn sparse_ids_and_optional_values_are_preserved() {
    let rules = tiny(
        "INSERT INTO et_supply VALUES (10,3000,NULL); INSERT INTO et_node(entity_type_id,resource_id) VALUES (10,40);",
    );
    assert!(rules.definitions[0].is_none());
    assert_eq!(rules.definitions.len(), 42);
    assert_eq!(rules.def(DefId(10)).id, DefId(10));
    assert_eq!(
        rules.entity_types[&DefId(10)]
            .supply
            .as_ref()
            .unwrap()
            .stock,
        None
    );
    assert_eq!(
        rules.entity_types[&DefId(10)].node.as_ref().unwrap().amount,
        None
    );
    assert_eq!(rules.id_of("absent"), None);
}

#[test]
fn invalid_booleans_and_out_of_range_ids_are_rejected() {
    let conn = seed_db();
    conn.execute_batch("UPDATE et_movement SET packs=2;")
        .unwrap();
    assert!(matches!(
        Rules::load(&conn),
        Err(RulesError::InvalidValue { .. })
    ));
    let conn = seed_db();
    conn.execute_batch(
        "INSERT INTO def(id,kind,key,name) VALUES (-1,'tech','tech.negative','Negative');",
    )
    .unwrap();
    assert!(Rules::load(&conn).is_err());
}

#[test]
fn player_stat_ignores_conditions_and_entity_targets() {
    let rules = tiny(
        "INSERT INTO modifier(source_def_id,stat_key,op,value,condition,target_def_id) VALUES
        (20,'research_time','add',10,'',NULL),
        (20,'research_time','add',20,'garrisoned',NULL),
        (20,'research_time','add',40,'',10);",
    );
    assert_eq!(
        rules.player_stat("research_time", 100, &ActiveSources::from([DefId(20)])),
        110
    );
    assert_eq!(
        rules.modifiers_from(DefId(20))[1].condition,
        Condition::Garrisoned
    );
    assert_eq!(rules.modifiers_from(DefId(20))[0].op, ModifierOp::Add);
}

#[test]
fn display_name_prefers_own_alias_then_ancestor_then_default() {
    let rules = seed();
    let id = |key: &str| rules.id_of(key).unwrap();
    let (city, citizen) = (id("bld.city"), id("unit.citizen"));
    // An Avian flight calls the city "Roost" through its race.
    assert_eq!(rules.display_name(city, id("nation.highwind")), "Roost");
    assert_eq!(rules.display_name(city, id("race.avian")), "Roost");
    assert_eq!(
        rules.display_name(citizen, id("nation.ashscale")),
        "Hatchling"
    );
    // No alias anywhere up the chain: the definition's own name.
    assert_eq!(rules.display_name(city, id("nation.romans")), "City");
    // A faction's own alias wins over its race's.
    let rules = tiny("INSERT INTO def(id,kind,key,name) VALUES
        (50,'faction','race.test','Test race'),(52,'faction','nation.test','Test nation');
        INSERT INTO faction(id,kind,parent_id) VALUES (50,'race',NULL),(52,'nation',50);
        INSERT INTO faction_alias(def_id,faction_id,name) VALUES (10,50,'Race name'),(10,52,'Own name');");
    assert_eq!(rules.display_name(DefId(10), DefId(52)), "Own name");
    assert_eq!(rules.display_name(DefId(10), DefId(50)), "Race name");
    assert_eq!(rules.display_name(DefId(10), DefId(0)), "Test node");
}

#[test]
fn race_of_walks_to_the_race() {
    let rules = seed();
    let id = |key: &str| rules.id_of(key).unwrap();
    assert_eq!(rules.race_of(id("nation.romans")), id("race.human"));
    assert_eq!(rules.race_of(id("bloc.europe")), id("race.human"));
    assert_eq!(rules.race_of(id("nation.mirefang")), id("race.reptilian"));
    assert_eq!(rules.race_of(id("race.avian")), id("race.avian"));
}

#[test]
fn playable_start_factions_at_first_age_are_nations_and_groups_only() {
    let rules = seed();
    let first = rules
        .ages
        .values()
        .min_by_key(|age| (age.ordinal, age.id))
        .unwrap()
        .id;
    let start = rules.playable_start_factions(first);
    assert!(start.windows(2).all(|w| w[0] < w[1]));
    assert!(!start.is_empty());
    for f in &start {
        assert_eq!(rules.factions[f].kind, rom_rules::FactionKind::Nation);
    }
    let races: std::collections::BTreeSet<_> = start.iter().map(|f| rules.race_of(*f)).collect();
    assert_eq!(races.len(), 3);
    assert!(start.contains(&rules.id_of("nation.romans").unwrap()));
    assert!(!start.contains(&rules.id_of("bloc.europe").unwrap()));
    assert!(!start.contains(&rules.id_of("race.human").unwrap()));
}

#[test]
fn alias_on_a_faction_is_reported() {
    assert_problem(
        "INSERT INTO faction_alias(def_id,faction_id,name)
        SELECT a.id,b.id,'X' FROM def a,def b WHERE a.key='nation.romans' AND b.key='race.avian';",
        "faction_alias",
        &["nation.romans"],
    );
}

#[test]
fn universe_knobs_load_and_validate() {
    let rules = seed();
    assert_eq!(rules.validate(), vec![]);
    assert_eq!(rules.body_kinds.len(), 3);
    let one = &rules.map_styles[&rules.id_of("map.one_world").unwrap()];
    assert_eq!(
        (one.neutral_systems_min, one.neutral_systems_max),
        (0, 0),
        "one world has no neutral systems"
    );
    assert_eq!(
        (one.habitable_per_system_min, one.habitable_per_system_max),
        (1, 1)
    );
    let many = &rules.map_styles[&rules.id_of("map.many_worlds").unwrap()];
    assert_eq!((many.neutral_systems_min, many.neutral_systems_max), (2, 4));
    assert_eq!(
        (many.habitable_per_system_min, many.habitable_per_system_max),
        (1, 3)
    );
    assert_eq!(
        (
            many.resource_bodies_per_system_min,
            many.resource_bodies_per_system_max
        ),
        (3, 8)
    );
    assert_eq!(
        (many.planet_n_min, many.planet_n_median, many.planet_n_max),
        (128, 192, 256)
    );
    assert!(many.route_extra_pct > 0 && many.star_map_radius > 0);
    for style in [one, many] {
        for (key, pct) in [("ter.forest", 24), ("ter.mountain", 7)] {
            let row = rules.map_style_features[&style.id]
                .iter()
                .find(|row| row.feature_def_id == rules.id_of(key).unwrap())
                .unwrap();
            assert_eq!(row.coverage_pct, pct);
        }
        let bodies: Vec<_> = rules.map_style_features[&style.id]
            .iter()
            .filter(|row| row.region_kind == RegionKind::ResourceBody)
            .collect();
        assert!(bodies.len() >= 3);
        for row in bodies {
            let node = rules.entity_types[&row.feature_def_id]
                .node
                .as_ref()
                .unwrap();
            assert_eq!(node.claim_mode, ClaimMode::Presence);
            let tag = node.claim_tag_id.unwrap();
            assert_eq!(rules.tags[&tag].key, "starship");
        }
    }
}

#[test]
fn bad_knob_range_is_reported() {
    assert_problem(
        "UPDATE map_style SET neutral_systems_min=5, neutral_systems_max=2
        WHERE id=(SELECT id FROM def WHERE key='map.many_worlds');",
        "neutral_systems_min 5",
        &["map.many_worlds"],
    );
    assert_problem(
        "UPDATE map_style SET resource_bodies_per_system_min=9
        WHERE id=(SELECT id FROM def WHERE key='map.one_world');",
        "resource_bodies_per_system",
        &["map.one_world"],
    );
    assert_problem(
        "UPDATE map_style SET planet_n_median=100, planet_n_min=150
        WHERE id=(SELECT id FROM def WHERE key='map.many_worlds');",
        "not in order",
        &["map.many_worlds"],
    );
    assert_problem(
        "UPDATE map_style SET planet_n_max=600
        WHERE id=(SELECT id FROM def WHERE key='map.many_worlds');",
        "planet_n_max 600 is outside 64..=512",
        &["map.many_worlds"],
    );
    assert_problem(
        "UPDATE map_style SET planet_n_min=32
        WHERE id=(SELECT id FROM def WHERE key='map.one_world');",
        "planet_n_min 32 is outside",
        &["map.one_world"],
    );
}

#[test]
fn coverage_pct_on_an_entity_feature_is_reported() {
    assert_problem(
        "UPDATE map_style_feature SET coverage_pct=24
        WHERE feature_def_id=(SELECT id FROM def WHERE key='node.relics');",
        "has coverage_pct set but is not a planet terrain feature",
        &["map.one_world", "node.relics"],
    );
}

#[test]
fn coverage_pct_on_a_space_feature_is_reported() {
    assert_problem(
        "UPDATE map_style_feature SET coverage_pct=24
        WHERE feature_def_id=(SELECT id FROM def WHERE key='ter.nebula');",
        "has coverage_pct set but is not a planet terrain feature",
        &["map.one_world", "ter.nebula"],
    );
}

#[test]
fn resource_body_feature_must_target_a_presence_node() {
    assert_problem(
        "INSERT INTO map_style_feature(map_style_id,feature_def_id,region_kind,pattern)
        SELECT s.id, n.id, 'resource_body', 'scattered' FROM def s, def n
        WHERE s.key='map.many_worlds' AND n.key='node.relics';",
        "is not a node with claim_mode presence",
        &["map.many_worlds", "node.relics"],
    );
    assert_problem(
        "INSERT INTO map_style_feature(map_style_id,feature_def_id,region_kind,pattern)
        SELECT s.id, n.id, 'resource_body', 'scattered' FROM def s, def n
        WHERE s.key='map.one_world' AND n.key='ter.forest';",
        "is not a node with claim_mode presence",
        &["map.one_world", "ter.forest"],
    );
}

#[test]
fn polar_and_start_latitude_knobs_load_and_validate() {
    let r = seed();
    let ice = r.id_of("ter.ice").unwrap();
    assert!(r.validate().is_empty());
    for style in r.map_styles.values() {
        assert_eq!(style.polar_cap_pct, 20);
        assert_eq!(style.start_latitude_max_mdeg, 60_000);
        assert_eq!(style.polar_terrain_id, Some(ice));
    }
    let t = &r.terrains[&ice];
    assert_eq!(t.layer, Layer::Ground);
    assert!(!t.buildable && !t.blocks_los);
    assert_eq!(
        r.materials[&t.material_id.unwrap()].params,
        "{\"base_colour\":\"#e8eef2\"}"
    );
    let plain = r.id_of("ter.plain").unwrap();
    for class in r.movement_classes.values() {
        let rows = &r.movement_class_terrain;
        let crosses_plain = rows
            .get(&class.id)
            .into_iter()
            .flatten()
            .any(|row| row.terrain_id == plain);
        let speed = rows
            .get(&class.id)
            .into_iter()
            .flatten()
            .find(|row| row.terrain_id == ice)
            .map(|row| row.speed_pct);
        assert_eq!(speed, crosses_plain.then_some(50), "{}", class.key);
    }
    let db = seed_db();
    for (cap, latitude) in [(0, 0), (50, 90_000), (13, 42_000)] {
        db.execute(
            "UPDATE map_style SET polar_cap_pct=?1, start_latitude_max_mdeg=?2",
            [cap, latitude],
        )
        .unwrap();
        let r = Rules::load(&db).unwrap();
        assert!(r.validate().is_empty());
        for style in r.map_styles.values() {
            assert_eq!(
                (style.polar_cap_pct, style.start_latitude_max_mdeg),
                (cap, latitude)
            );
        }
    }
    db.execute_batch("UPDATE map_style SET polar_cap_pct=0, polar_terrain_id=NULL;")
        .unwrap();
    assert!(Rules::load(&db).unwrap().validate().is_empty());
    for value in [-1, 90_001] {
        assert_problem(
            &format!("UPDATE map_style SET start_latitude_max_mdeg={value};"),
            "start_latitude_max_mdeg",
            &["map.one_world"],
        );
    }
    assert_problem(
        "UPDATE map_style SET polar_terrain_id=NULL;",
        "polar_terrain_id is required",
        &["map.one_world"],
    );
    for key in ["ter.plain", "ter.forest", "ter.deep"] {
        assert_problem(
            &format!(
                "UPDATE map_style SET polar_terrain_id=(SELECT id FROM def WHERE key='{key}');"
            ),
            "polar_terrain_id must reference",
            &["map.one_world"],
        );
    }
}

#[test]
fn bad_polar_cap_pct_is_reported() {
    for value in [-1, 51] {
        assert_problem(
            &format!("UPDATE map_style SET polar_cap_pct={value};"),
            &format!("polar_cap_pct {value} is outside 0..=50"),
            &["map.one_world"],
        );
    }
}

#[test]
fn generated_seed_validates_with_no_problems() {
    assert_eq!(seed().validate(), vec![]);
}

#[test]
fn every_age_has_an_advance_tech() {
    use rom_rules::{TechEffectKind, TechKind};
    let r = seed();
    for age in r.ages.values().filter(|a| a.ordinal > 0) {
        let advances: Vec<_> = r
            .tech_effects
            .values()
            .flatten()
            .filter(|e| e.effect == TechEffectKind::SetAge && e.target_def_id == Some(age.id))
            .collect();
        assert_eq!(advances.len(), 1, "{}", r.key(age.id));
        assert_eq!(r.techs[&advances[0].tech_id].kind, TechKind::Age);
    }
}

#[test]
fn age_advance_needs_all_four_tracks() {
    let r = seed();
    for age in r.ages.values().filter(|a| a.ordinal > 0) {
        let advance = r
            .techs
            .values()
            .find(|t| t.kind == rom_rules::TechKind::Age && t.level == age.ordinal)
            .unwrap();
        let previous = r
            .ages
            .values()
            .find(|a| a.ordinal == age.ordinal - 1)
            .unwrap()
            .id;
        let tracks: Vec<_> = r
            .techs
            .values()
            .filter(|t| t.kind == rom_rules::TechKind::Track && t.level == age.ordinal)
            .collect();
        assert_eq!(tracks.len(), 4);
        let all: ActiveSources = std::iter::once(previous)
            .chain(tracks.iter().map(|t| t.id))
            .collect();
        assert!(r.is_unlocked(advance.id, &all, &Owned::new()));
        for missing in &tracks {
            let without: ActiveSources =
                all.iter().copied().filter(|id| *id != missing.id).collect();
            assert!(!r.is_unlocked(advance.id, &without, &Owned::new()));
        }
    }
}

#[test]
fn obsolete_when_successor_unlocked() {
    let r = seed();
    let old = r.id_of("unit.spearman").unwrap();
    let mut active = sources(&r, &["age.classical"]);
    assert!(!r.is_obsolete(old, &active, &Owned::new()));
    active.insert(r.id_of("tech.up.hoplite").unwrap());
    assert!(r.is_obsolete(old, &active, &Owned::new()));
}

#[test]
fn unreachable_item_is_reported() {
    assert_problem("INSERT INTO def(kind,key,name) VALUES ('tech','tech.unreachable','Unreachable');
        INSERT INTO tech(id,kind,research_ticks) VALUES(last_insert_rowid(),'upgrade',1);
        INSERT INTO production SELECT p.id,t.id,'research','',0,0 FROM def p,def t WHERE p.key='bld.library' AND t.key='tech.unreachable';
        INSERT INTO prerequisite SELECT t.id,r.id,0 FROM def t,def r WHERE t.key='tech.unreachable' AND r.key='res.food';",
        "unreachable item", &["tech.unreachable"]);
}

#[test]
fn every_line_has_one_first_step_and_continuous_ages() {
    let r = seed();
    let mut lines: std::collections::BTreeMap<&str, Vec<_>> = Default::default();
    for unit in r
        .entity_types
        .values()
        .filter(|e| e.class == rom_rules::EntityClass::Unit)
    {
        lines.entry(&unit.ui_group).or_default().push(unit);
    }
    assert_eq!(lines.len(), 20);
    for (line, steps) in lines {
        let roots: Vec<_> = steps
            .iter()
            .filter(|u| u.upgrades_from_id.is_none())
            .collect();
        assert_eq!(roots.len(), 1, "{line}");
        let mut current = roots[0].id;
        let mut visited = std::collections::BTreeSet::from([current]);
        loop {
            let next: Vec<_> = steps
                .iter()
                .filter(|u| u.upgrades_from_id == Some(current))
                .collect();
            assert!(next.len() <= 1, "branched {line}");
            let Some(next) = next.first() else {
                break;
            };
            let old_age = r.ages[&r.entity_types[&current].first_age_id.unwrap()].ordinal;
            let new_age = r.ages[&next.first_age_id.unwrap()].ordinal;
            assert!(new_age >= old_age, "{line}");
            assert!(visited.insert(next.id));
            current = next.id;
        }
        assert_eq!(visited.len(), steps.len(), "disconnected {line}");
    }
}

#[test]
fn loads_keeper_and_carry_mode() {
    let r = seed();
    let temple = &r.entity_types[&r.id_of("bld.temple").unwrap()];
    assert_eq!(temple.keeper.as_ref().unwrap().capacity, 5);
    let node = r.entity_types[&r.id_of("node.relics").unwrap()]
        .node
        .as_ref()
        .unwrap();
    assert_eq!(node.claim_mode, ClaimMode::Carry);
    assert!(node.claim_tag_id.is_some());
}

#[test]
fn rejects_carry_without_tag_and_nonpositive_keeper() {
    let conn = seed_db();
    conn.execute_batch("UPDATE et_node SET claim_tag_id=NULL WHERE claim_mode='carry'; UPDATE et_keeper SET capacity=0;").unwrap();
    let r = Rules::load(&conn).unwrap();
    let problems = r.validate();
    assert!(
        problems
            .iter()
            .any(|p| p.to_string().contains("requires claim_tag_id"))
    );
    assert!(
        problems
            .iter()
            .any(|p| p.to_string().contains("capacity must be positive"))
    );
}
