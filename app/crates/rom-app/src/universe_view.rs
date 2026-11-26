//! The generated universe as the render thread draws it, and the zoom limit by age.

use rom_mapgen::Universe;
use rom_render::universe::{BodyView, BodyViewKind, ClusterView, SystemView, UniverseScene};
use rom_render::{Level, owner_color};
use rom_rules::{BodyKind, DefId, Rules};

/// Star colours, chosen by system id.
const STAR_COLOURS: [[u8; 3]; 5] = [
    [255, 244, 214],
    [255, 226, 170],
    [215, 228, 255],
    [255, 200, 150],
    [240, 240, 255],
];

/// The farthest camera level allowed by the age's map-scale rank.
pub fn max_level(rules: &Rules, age: DefId) -> Level {
    match rules
        .ages
        .get(&age)
        .and_then(|a| rules.map_scales.get(&a.map_scale))
        .map(|scale| scale.rank)
    {
        Some(1) => Level::System,
        Some(2..) => Level::Galaxy,
        _ => Level::Globe,
    }
}

/// The largest ground half-width, in milli-tiles, the camera of a player in `age` may show on the
/// home planet; 0 for the whole globe (and for an unknown age).
pub fn view_range(rules: &Rules, age: DefId) -> u32 {
    rules
        .ages
        .get(&age)
        .map_or(0, |age| u32::try_from(age.view_range).unwrap_or(0))
}

/// Converts a generated universe. `start_bodies` holds the home planet of each player in player
/// order; a home planet takes the colour of the first player living there. The star clusters of
/// its systems come from `rules` (task 162).
pub fn to_universe_scene(
    universe: &Universe,
    start_bodies: &[u32],
    rules: &Rules,
) -> UniverseScene {
    let colour_of = |body: u32| {
        start_bodies
            .iter()
            .position(|&b| b == body)
            .map(|player| owner_color(player as u8 + 1).map(|c| (c * 255.0).round() as u8))
    };
    UniverseScene {
        systems: universe
            .systems
            .iter()
            .map(|s| {
                let celestial = Some(s.id) == universe.celestial_system();
                SystemView {
                    id: s.id,
                    name: s.name.clone(),
                    x: s.x,
                    y: s.y,
                    z: s.z,
                    constellation: s.constellation.clone(),
                    cluster: s.cluster,
                    // The celestial home glows with its black hole's accretion disc.
                    star_colour: if celestial {
                        rom_render::universe::ACCRETION_RGB[1]
                    } else {
                        STAR_COLOURS[s.id as usize % STAR_COLOURS.len()]
                    },
                    celestial,
                }
            })
            .collect(),
        bodies: universe
            .bodies
            .iter()
            .map(|b| BodyView {
                id: b.id,
                name: b.name.clone(),
                system: b.system_id,
                kind: match b.kind {
                    // The celestial home's black hole stands where a star would (task 76).
                    BodyKind::Star | BodyKind::BlackHole => BodyViewKind::Star,
                    BodyKind::Habitable => BodyViewKind::Habitable,
                    BodyKind::Resource => BodyViewKind::Resource,
                    BodyKind::Celestial => BodyViewKind::Celestial,
                },
                orbit_radius: b.orbit_radius,
                orbit_angle: b.orbit_angle_mdeg,
                n_or_size: b
                    .n
                    .unwrap_or_else(|| 1 + b.node_entity_type.map_or(0, |t| t.0 % 3)),
                home_race_colour: b.home_race.and_then(|_| colour_of(b.id)),
            })
            .collect(),
        routes: universe
            .routes
            .iter()
            .map(|r| (r.system_a, r.system_b))
            .collect(),
        figure: universe.figure.clone(),
        clusters: {
            let mut ids: Vec<u32> = universe.systems.iter().filter_map(|s| s.cluster).collect();
            ids.sort_unstable();
            ids.dedup();
            ids.iter()
                .filter_map(|id| rules.star_clusters.get(id))
                .map(|c| ClusterView {
                    id: c.id,
                    name: c.name.clone(),
                    x: i32::try_from(c.x).unwrap_or(0),
                    y: i32::try_from(c.y).unwrap_or(0),
                })
                .collect()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::Options;
    use crate::terrain::{generate_universe_content, load_rules};

    #[test]
    fn camera_tier_is_found_by_rank_independently_of_view_range_or_names() {
        let mut rules = load_rules().expect("rules");
        let age = rules.ages.values().next().unwrap().id;
        let scale_key = rules.ages[&age].map_scale.clone();
        let mut scale = rules.map_scales.remove(&scale_key).unwrap();
        scale.key = "renamed".into();
        rules.ages.get_mut(&age).unwrap().map_scale = scale.key.clone();
        rules.map_scales.insert(scale.key.clone(), scale);
        for (rank, level) in [(0, Level::Globe), (1, Level::System), (2, Level::Galaxy)] {
            rules.map_scales.get_mut("renamed").unwrap().rank = rank;
            for range in [0, 30_000] {
                rules.ages.get_mut(&age).unwrap().view_range = range;
                assert_eq!(max_level(&rules, age), level);
            }
        }
        assert_eq!(max_level(&rules, DefId(u32::MAX)), Level::Globe);
    }

    #[test]
    fn zoom_cap_follows_the_age() {
        let rules = load_rules().expect("rules");
        let by_ordinal = |n: i64| rules.ages.values().find(|a| a.ordinal == n).unwrap().id;
        let mut last = 0;
        for n in 0..=8 {
            let range = view_range(&rules, by_ordinal(n));
            assert!(range > last, "age {n} must see more than the one before");
            assert_eq!(max_level(&rules, by_ordinal(n)), Level::Globe);
            last = range;
        }
        // Zero view range allows the whole globe; the map-scale tier caps space zoom, and
        // from Fusion (task 138) the home system can be seen.
        assert_eq!(view_range(&rules, by_ordinal(9)), 0);
        assert_eq!(max_level(&rules, by_ordinal(9)), Level::System);
        assert_eq!(max_level(&rules, by_ordinal(10)), Level::System);
        assert_eq!(max_level(&rules, by_ordinal(11)), Level::Galaxy);
        assert_eq!(view_range(&rules, DefId(u32::MAX)), 0);
    }

    #[test]
    fn universe_scene_matches_the_generated_universe() {
        let options = Options {
            universe: true,
            seed: 1,
            ..Options::default()
        };
        let content = generate_universe_content(&options).expect("universe");
        let u = &content.universe;
        let bodies: Vec<u32> = u.starts.iter().map(|s| s.0).collect();
        let scene = to_universe_scene(u, &bodies, &content.rules);
        assert!(scene.is_valid());
        assert_eq!(scene.systems.len(), u.systems.len());
        assert_eq!(scene.bodies.len(), u.bodies.len());
        assert_eq!(scene.routes.len(), u.routes.len());
        for (view, system) in scene.systems.iter().zip(&u.systems) {
            assert_eq!(
                (view.id, &view.name, view.x, view.y),
                (system.id, &system.name, system.x, system.y)
            );
        }
        for (view, body) in scene.bodies.iter().zip(&u.bodies) {
            assert_eq!(
                (view.id, view.system, view.orbit_radius, view.orbit_angle),
                (
                    body.id,
                    body.system_id,
                    body.orbit_radius,
                    body.orbit_angle_mdeg
                )
            );
            assert_eq!(view.home_race_colour.is_some(), body.home_race.is_some());
        }
        // The first player's home planet is a habitable body with a generated surface.
        let home = scene.bodies.iter().find(|b| b.id == bodies[0]).unwrap();
        assert_eq!(home.kind, BodyViewKind::Habitable);
        assert!(u.planets.contains_key(&home.id));
        // The same options give the same scene.
        let again = generate_universe_content(&options).unwrap();
        assert_eq!(
            scene,
            to_universe_scene(&again.universe, &bodies, &again.rules)
        );
    }

    /// Resource bodies are drawn far larger than true (task 69), yet in every generated system
    /// no two drawn bodies touch and every rock stays smaller than every planet.
    #[test]
    fn generated_systems_draw_their_bodies_apart() {
        use rom_render::universe::{BODY_GAP, SMALLEST_PLANET_RADIUS, body_radius, closest_gap};
        for seed in 1..=3 {
            let options = Options {
                universe: true,
                seed,
                players: 4,
                ..Options::default()
            };
            let content = generate_universe_content(&options).expect("universe");
            let scene = to_universe_scene(&content.universe, &[], &content.rules);
            for system in &scene.systems {
                let placed = scene.place_system(system.id);
                let gap = closest_gap(&placed);
                assert!(
                    gap >= BODY_GAP - 1.0,
                    "seed {seed} system {}: {gap}",
                    system.id
                );
            }
            for body in &scene.bodies {
                if body.kind == BodyViewKind::Resource {
                    assert!(body_radius(body) < SMALLEST_PLANET_RADIUS, "{body:?}");
                }
            }
        }
    }
}

#[cfg(test)]
mod match_tests {
    use super::*;
    use crate::args::parse;
    use crate::match_setup::build_universe_match;
    use crate::script::ScriptSetup;
    use crate::terrain::generate_universe_content;
    use rom_render::zoom::{Home, ZoomCamera, ZoomScale};
    use rom_render::{OrbitCamera, planet_radius};
    use rom_rules::{ProductionMode, TechEffectKind};
    use rom_sim::{Command, PlayerId};
    use std::sync::Arc;

    fn content() -> crate::terrain::UniverseContent {
        let options = parse(["--sphere", "64", "--players", "2"].map(str::to_owned)).unwrap();
        assert!(options.uses_universe());
        generate_universe_content(&options).unwrap()
    }

    fn camera(
        content: &crate::terrain::UniverseContent,
        world: &rom_sim::World,
    ) -> (ZoomCamera, OrbitCamera) {
        let home = world.home_body(PlayerId(1)).unwrap();
        let body = world.body(home).unwrap();
        let n = world.surface(home).unwrap().map().width;
        let radius = planet_radius(n);
        let orbit = OrbitCamera::new(radius, radius * 3.0);
        let camera = ZoomCamera::new(
            Arc::new(to_universe_scene(
                &content.universe,
                &[home.0],
                &content.rules,
            )),
            ZoomScale::new(radius, orbit.max_distance),
            Some(Home {
                system: body.system,
                body: home.0,
                dir: orbit.dir,
            }),
            Level::Globe,
        );
        (camera, orbit)
    }

    fn update_limits(world: &rom_sim::World, camera: &mut ZoomCamera, orbit: &mut OrbitCamera) {
        // The same values publish_camera_limits sends, applied through the renderer's public
        // window-free camera API in the same order as its worker.
        let age = world.players().find(|p| p.id == PlayerId(1)).unwrap().age;
        camera.set_cap(max_level(world.rules(), age));
        let range = view_range(world.rules(), age);
        orbit.set_view_range(range);
        camera.set_globe_limit((range > 0).then(|| orbit.distance_cap()));
    }

    #[test]
    fn universe_start_age_sets_every_player_and_camera_limit() {
        let content = content();
        assert!(
            content
                .universe
                .bodies
                .iter()
                .filter(|b| b.home_race.is_some())
                .all(|b| b.n == Some(64))
        );
        for ordinal in [0, 5, 8, 9, 10, 11, 13] {
            let game = build_universe_match(
                &content.rules,
                &content.universe,
                &content.factions,
                1,
                Some(ordinal),
                &ScriptSetup::default(),
            )
            .unwrap();
            for player in game.world.players().filter(|p| p.id != PlayerId(0)) {
                assert_eq!(content.rules.ages[&player.age].ordinal, ordinal);
            }
            let (mut camera, mut orbit) = camera(&content, &game.world);
            update_limits(&game.world, &mut camera, &mut orbit);
            camera.set_l(f32::MAX);
            let expected = max_level(
                &content.rules,
                game.world
                    .players()
                    .find(|p| p.id == PlayerId(1))
                    .unwrap()
                    .age,
            );
            assert_eq!(camera.level(), expected);
            if expected == Level::Globe {
                assert!((camera.l().exp() - orbit.distance_cap()).abs() < 0.01);
            } else {
                assert_eq!(camera.l(), camera.scale().l_cap(expected));
            }
            // Task 162: a level that would show one item is skipped by this universe's zoom.
            let scale = *camera.scale();
            let exists = |level: Level| match level {
                Level::Cluster => scale.beyond_systems(),
                Level::Quadrant => scale.ladder.quadrant.is_some(),
                Level::Galaxy => scale.has_disk(),
                _ => true,
            };
            for level in Level::ALL {
                camera.set_view(level, &mut orbit);
                assert!(camera.level() <= expected);
                if level <= expected && exists(level) {
                    assert_eq!(camera.level(), level);
                }
            }
            // The wheel reaches exactly the same cap, and never skips an unlocked level.
            let mut levels = std::collections::BTreeSet::new();
            camera.set_l(camera.scale().l_min());
            for _ in 0..160 {
                camera.wheel(-1.0);
                camera.sync_orbit(&mut orbit);
                camera.adopt_orbit(&orbit);
                levels.insert(camera.level());
                assert!(camera.level() <= expected);
            }
            assert_eq!(camera.level(), expected);
            assert!(
                Level::ALL
                    .iter()
                    .filter(|level| **level <= expected && exists(**level))
                    .all(|level| levels.contains(level))
            );
        }
        assert!(
            build_universe_match(
                &content.rules,
                &content.universe,
                &content.factions,
                1,
                Some(99),
                &ScriptSetup::default()
            )
            .is_err()
        );
    }

    #[test]
    fn research_widens_the_existing_universe_camera_immediately() {
        let mut content = content();
        let rules = Arc::make_mut(&mut content.rules);
        let target_age = rules
            .ages
            .values()
            .filter(|a| rules.map_scales[&a.map_scale].rank == 1)
            .min_by_key(|a| a.ordinal)
            .unwrap()
            .clone();
        let tech = rules
            .tech_effects
            .iter()
            .find(|(_, effects)| {
                effects.iter().any(|e| {
                    e.effect == TechEffectKind::SetAge && e.target_def_id == Some(target_age.id)
                })
            })
            .unwrap()
            .0
            .to_owned();
        // Isolate completion of the real age effect; the full technology tree is covered by playthrough.
        rules.prerequisites.remove(&tech);
        rules.costs.remove(&tech);
        rules.techs.get_mut(&tech).unwrap().research_ticks = 1;
        let producer = rules
            .production
            .iter()
            .find(|(_, rows)| {
                rows.iter()
                    .any(|r| r.mode == ProductionMode::Research && r.product_def_id == tech)
            })
            .unwrap()
            .0
            .to_owned();
        let run = || {
            let mut game = build_universe_match(
                &content.rules,
                &content.universe,
                &content.factions,
                1,
                Some(target_age.ordinal - 1),
                &ScriptSetup::default(),
            )
            .unwrap();
            let (mut camera, mut orbit) = camera(&content, &game.world);
            update_limits(&game.world, &mut camera, &mut orbit);
            camera.set_l(f32::MAX);
            let before = camera.l();
            assert!(camera.level() <= Level::Globe);
            let mut notices = crate::space_view::Notices::default();
            notices.update(&game.world);
            let mut hints = Vec::new();
            // Some start is near a face edge: take the first offset where the footprint fits.
            let executor = [10000, -10000, 14000, -14000]
                .into_iter()
                .find_map(|dx| {
                    let at = game.world.offset(game.starts[0].city_pos, dx, 0);
                    game.world.place_for_setup(producer, PlayerId(1), at).ok()
                })
                .unwrap();
            game.world
                .submit(Command {
                    player: PlayerId(1),
                    executor,
                    kind: "research".into(),
                    subject: Some(tech),
                    target_entity: None,
                    target_pos: None,
                    queue: false,
                    text: None,
                    facing: None,
                })
                .unwrap();
            for _ in 0..10 {
                game.world.step();
                hints.extend(notices.update(&game.world).into_iter().map(|n| n.text));
                if game
                    .world
                    .players()
                    .find(|p| p.id == PlayerId(1))
                    .unwrap()
                    .age
                    == target_age.id
                {
                    break;
                }
            }
            assert_eq!(
                game.world
                    .players()
                    .find(|p| p.id == PlayerId(1))
                    .unwrap()
                    .age,
                target_age.id
            );
            update_limits(&game.world, &mut camera, &mut orbit);
            assert_eq!(
                camera.l(),
                before,
                "raising a limit must not jump the camera"
            );
            // The first time the reach widens, the event line says how to see it.
            assert_eq!(
                hints,
                vec![crate::space_view::reach_hint(1).unwrap().to_owned()]
            );
            camera.set_l(f32::MAX);
            assert_eq!(camera.level(), Level::System);
            assert!(camera.l() > before);
            game.world.hash()
        };
        assert_eq!(run(), run());
    }
}

/// Task 162: the levels beyond a system over the real catalogue.
#[cfg(test)]
mod cluster_tests {
    use super::*;
    use crate::terrain::load_rules;
    use rom_render::galaxy::{GalaxyLayout, system_view_clearance};
    use rom_render::zoom::{Target, ZoomCamera, ZoomScale};
    use rom_render::{OrbitCamera, planet_radius};
    use std::sync::Arc;

    /// Every catalogue star as a system, joined to the next star of its cluster, as a Galaxy
    /// match takes them (bodies are not needed to place and pick systems).
    fn catalogue_scene(rules: &Rules) -> UniverseScene {
        let systems: Vec<SystemView> = rules
            .stars
            .values()
            .map(|s| SystemView {
                id: s.id,
                name: s.name.clone(),
                x: s.x as i32,
                y: s.y as i32,
                z: s.z as i32,
                constellation: String::new(),
                star_colour: [255, 255, 255],
                celestial: false,
                cluster: Some(s.cluster_id),
            })
            .collect();
        let mut routes = Vec::new();
        for pair in systems.windows(2) {
            if pair[0].cluster != pair[1].cluster || pair[0].id % 5 == 0 {
                routes.push((pair[0].id, pair[1].id));
            }
        }
        UniverseScene {
            systems,
            routes,
            clusters: rules
                .star_clusters
                .values()
                .map(|c| ClusterView {
                    id: c.id,
                    name: c.name.clone(),
                    x: c.x as i32,
                    y: c.y as i32,
                })
                .collect(),
            ..UniverseScene::default()
        }
    }

    fn camera(scene: UniverseScene) -> (ZoomCamera, OrbitCamera) {
        let radius = planet_radius(192);
        let orbit = OrbitCamera::new(radius, radius * 3.0);
        let zoom = ZoomCamera::new(
            Arc::new(scene),
            ZoomScale::new(radius, orbit.max_distance),
            None,
            Level::Galaxy,
        );
        (zoom, orbit)
    }

    #[test]
    fn every_cluster_keeps_the_system_view_clear() {
        let rules = load_rules().expect("rules");
        let layout = GalaxyLayout::new(&catalogue_scene(&rules));
        assert_eq!(layout.groups.len(), rules.star_clusters.len());
        let clearance = system_view_clearance();
        for group in &layout.groups {
            for (a, &i) in group.members.iter().enumerate() {
                for &j in &group.members[a + 1..] {
                    let way = [0, 1, 2].map(|k| layout.cluster[i][k] - layout.cluster[j][k]);
                    let d = (way[0] * way[0] + way[1] * way[1] + way[2] * way[2]).sqrt();
                    assert!(d >= clearance * 0.999, "{}: {d}", group.name);
                }
            }
        }
        // Every level exists for the whole catalogue: four quadrants and many clusters.
        let ladder = layout.ladder;
        assert!(ladder.cluster.is_some() && ladder.quadrant.is_some() && ladder.galaxy.is_some());
        println!(
            "clearance {clearance:.0}, cluster radius {:.0}, ladder {ladder:?}",
            layout.cluster_radius
        );
    }

    /// Picking never hits a neighbour: in the cluster view each system is picked at its own
    /// place, and on the disk (the quadrant and galaxy views) each cluster at its centre and at
    /// every one of its stars.
    #[test]
    fn picking_never_hits_a_neighbour_at_any_level() {
        let rules = load_rules().expect("rules");
        let size = (1280.0, 800.0);
        let (mut zoom, mut orbit) = camera(catalogue_scene(&rules));
        let groups = zoom.layout().groups.len();
        let mut picked = 0;
        // The cluster view of every eighth cluster, flown to as a double click on it would.
        for group in (0..groups).step_by(8) {
            zoom.fly_to_target(Target::Cluster(group), &mut orbit);
            for _ in 0..600 {
                zoom.advance(1.0 / 30.0);
                zoom.sync_orbit(&mut orbit);
                if !zoom.flying() {
                    break;
                }
            }
            assert_eq!(zoom.level(), Level::Cluster, "cluster {group}");
            assert_eq!(zoom.anchored_group(), group);
            let view = zoom.map_view(&orbit).view(size, 0.0);
            let positions = zoom.map_positions();
            for &i in &zoom.layout().groups[group].members {
                let Some((x, y, _)) = view.project(positions[i]) else {
                    continue;
                };
                if x < 0.0 || y < 0.0 || x > size.0 || y > size.1 {
                    continue;
                }
                assert_eq!(zoom.target_at(&view, x, y), Some(Target::System(i)));
                picked += 1;
            }
        }
        assert!(picked > 40, "{picked} systems picked");
        // On the disk, seen from above at the quadrant's and the galaxy's widest zooms.
        for level in [Level::Quadrant, Level::Galaxy] {
            zoom.set_view(level, &mut orbit);
            assert_eq!(zoom.level(), level);
            assert!(zoom.on_disk());
            let view = zoom.map_view(&orbit).view(size, 0.0);
            let positions = zoom.map_positions();
            let mut clusters = 0;
            for (g, group) in zoom.layout().groups.iter().enumerate() {
                let points = std::iter::once(group.centre)
                    .chain(group.members.iter().map(|&i| positions[i]));
                let mut seen = false;
                for point in points {
                    let Some((x, y, _)) = view.project(point) else {
                        continue;
                    };
                    if x < 0.0 || y < 0.0 || x > size.0 || y > size.1 {
                        continue;
                    }
                    seen = true;
                    assert_eq!(
                        zoom.target_at(&view, x, y),
                        Some(Target::Cluster(g)),
                        "{level:?}: {}",
                        group.name
                    );
                }
                clusters += usize::from(seen);
            }
            assert!(clusters >= 20, "{level:?}: {clusters} clusters on screen");
        }
    }

    /// Wheel notches from the whole system to the whole galaxy, for the record.
    #[test]
    fn the_wheel_crosses_the_levels_in_a_few_notches() {
        let rules = load_rules().expect("rules");
        let (mut zoom, _) = camera(catalogue_scene(&rules));
        let s = *zoom.scale();
        let mut count = |from: f32, to: f32| {
            zoom.set_l(from);
            let mut n = 0;
            while zoom.l() < to - 1e-4 && n < 1000 {
                zoom.wheel(-1.0);
                n += 1;
            }
            n
        };
        let cluster = count(s.l_system_map, s.l_cluster);
        let quadrant = count(s.l_cluster, s.l_quadrant);
        let galaxy = count(s.l_quadrant, s.l_end);
        println!("notches: system->cluster {cluster}, ->quadrant {quadrant}, ->galaxy {galaxy}");
        assert!(
            cluster + quadrant + galaxy <= 24,
            "{cluster} {quadrant} {galaxy}"
        );
    }
}
