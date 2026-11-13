//! Application start-up: render thread, GPUI window, clean shutdown.

use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Duration;

use gpui::{
    App, AppContext, Application, Bounds, Entity, TitlebarOptions, Window, WindowBounds,
    WindowDecorations, WindowOptions, point, px, size,
};
use gpui_component::Root;
use rom_render::{
    Camera, OrbitSettings, RenderBridge, RenderWorker, ShutdownHandle, SurfacePos, UniverseSettings,
};

use crate::args::Options;
use crate::bench;
use crate::demo::{DemoDriver, add_stress_units};
use crate::match_setup::{Match, build_match, build_sphere_match};
use crate::screenshot;
use crate::script::{Script, ScriptContext};
use crate::sim_thread::{SimLink, SimThread};
use crate::terrain;
use crate::universe_view;
use crate::world_view::WorldView;

/// Opens the window and runs until it is closed (or the benchmark finishes).
pub fn run(options: &Options) -> ExitCode {
    // GPUI unwraps platform initialisation errors; report them instead of a raw panic.
    let application = match std::panic::catch_unwind(Application::new) {
        Ok(application) => application,
        Err(_) => {
            eprintln!("rise-of-man: could not initialise the windowing system");
            return ExitCode::FAILURE;
        }
    };
    let (mut worker, bridge) = match RenderWorker::start() {
        Ok(started) => started,
        Err(error) => {
            eprintln!("rise-of-man: could not start the render thread: {error}");
            return ExitCode::FAILURE;
        }
    };
    bridge.publish_render_scale(options.scale);
    bridge.publish_hud_options(!options.no_hud, options.hud_scale);
    let mut sim = match start_simulation(options, &bridge) {
        Ok(sim) => Some(sim),
        Err(error) => {
            eprintln!("rise-of-man: {error}");
            worker.stop();
            return ExitCode::FAILURE;
        }
    };
    let sim_link = sim.as_ref().map_or_else(SimLink::idle, SimThread::link);
    if options.sphere.is_none()
        && !options.universe
        && let Some([x, y, distance, yaw, pitch]) = options.camera
    {
        bridge.publish_camera(Camera {
            target: [x, 0.0, y],
            distance,
            yaw: yaw.to_radians(),
            pitch: pitch.to_radians(),
            ..Camera::default()
        });
    }
    bridge.publish_box_count(options.boxes);
    bridge.publish_start_markers(options.start_markers);
    let shutdown = worker.shutdown_handle();
    let exit_code = Arc::new(AtomicU8::new(0));
    let app_exit_code = Arc::clone(&exit_code);
    let options = options.clone();

    application.run(move |cx: &mut App| {
        gpui_component::init(cx);
        let (width, height) = options.size;
        let canvas = size(px(width as f32), px(height as f32));
        let mut window_options = WindowOptions {
            titlebar: Some(TitlebarOptions {
                title: Some("Ascension: Rise of Man".into()),
                ..Default::default()
            }),
            app_id: Some("rise-of-man".into()),
            ..Default::default()
        };
        let fixed_window = options.bench_seconds.is_some() || options.screenshot.is_some();
        let opened = if fixed_window {
            // Benchmark and screenshot windows have no title bar or component frame (which pads
            // client-decorated windows for a shadow), so the whole requested size is canvas even
            // at monitor size.
            window_options.window_bounds = Some(WindowBounds::Windowed(Bounds::new(
                point(px(0.0), px(0.0)),
                canvas,
            )));
            window_options.window_decorations = Some(WindowDecorations::Client);
            let exit_code = Arc::clone(&app_exit_code);
            cx.open_window(window_options, move |window, cx| {
                let view = world_view(
                    &bridge,
                    &shutdown,
                    &sim_link,
                    (options.size, options.stats),
                    window,
                    cx,
                );
                if let Some(seconds) = options.bench_seconds {
                    let bench = bench::Bench {
                        seconds,
                        requested: options.size,
                        scale: options.scale,
                    };
                    bench::start(
                        bench,
                        view.clone(),
                        bridge,
                        sim_link,
                        shutdown,
                        exit_code,
                        cx,
                    );
                } else if let Some(path) = options.screenshot.clone() {
                    screenshot::start(
                        path,
                        Duration::from_secs_f64(options.after_seconds),
                        view.clone(),
                        bridge,
                        shutdown,
                        exit_code,
                        cx,
                    );
                }
                view
            })
            .map(|_| ())
        } else {
            window_options.window_bounds = Some(WindowBounds::centered(canvas, cx));
            cx.open_window(window_options, move |window, cx| {
                let view = world_view(
                    &bridge,
                    &shutdown,
                    &sim_link,
                    (options.size, options.stats),
                    window,
                    cx,
                );
                cx.new(|cx| Root::new(view, window, cx))
            })
            .map(|_| ())
        };
        if let Err(error) = opened {
            eprintln!("rise-of-man: could not open the window: {error:#}");
            app_exit_code.store(1, Ordering::Relaxed);
            cx.spawn(async |cx| {
                let _ = cx.update(|cx| cx.quit());
            })
            .detach();
            return;
        }
        cx.activate(true);
    });
    if let Some(sim) = &mut sim {
        sim.stop();
    }
    worker.stop();
    ExitCode::from(exit_code.load(Ordering::Relaxed))
}

/// Sets up the freshly opened window and creates the world view for it.
fn world_view(
    bridge: &RenderBridge,
    shutdown: &ShutdownHandle,
    sim: &SimLink,
    ((width, height), stats): ((u32, u32), bool),
    window: &mut Window,
    cx: &mut App,
) -> Entity<WorldView> {
    let close_shutdown = shutdown.clone();
    let close_sim = sim.clone();
    // GPUI's X11 client holds its state borrowed while asking this, so calling `cx.quit()`
    // here panics; the event loop ends by itself once the last window is gone.
    window.on_window_should_close(cx, move |_, _| {
        close_sim.request_stop();
        close_shutdown.stop_and_wait();
        true
    });
    // The requested size is in physical pixels; GPUI sizes windows in logical ones.
    let scale_factor = window.scale_factor();
    if scale_factor != 1.0 {
        window.resize(size(
            px(width as f32 / scale_factor),
            px(height as f32 / scale_factor),
        ));
    }
    let (bridge, sim) = (bridge.clone(), sim.clone());
    cx.new(|cx| WorldView::new(bridge, sim, stats, window, cx))
}

/// Generates the map or planet, publishes its terrain, builds the match and starts the
/// simulation thread.
fn start_simulation(options: &Options, bridge: &RenderBridge) -> Result<SimThread, String> {
    let mut game = if options.universe {
        let content = terrain::generate_universe_content(options)?;
        let universe = &content.universe;
        let bodies: Vec<u32> = universe.starts.iter().map(|s| s.0).collect();
        let home_body = *bodies.first().ok_or("the universe has no start")?;
        let home_system = universe
            .bodies
            .iter()
            .find(|b| b.id == home_body)
            .map(|b| b.system_id)
            .ok_or("the home planet is not in the universe")?;
        let planet = universe
            .planets
            .get(&home_body)
            .ok_or("the home planet was not generated")?;
        // The simulation has one surface: the home planet, with the players living there.
        let local: Vec<_> = bodies
            .iter()
            .zip(&content.factions)
            .filter(|(body, _)| **body == home_body)
            .map(|(_, faction)| *faction)
            .collect();
        let game = build_sphere_match(&content.rules, planet, &local, options.seed)
            .map_err(|error| format!("setting up the match: {error}"))?;
        bridge.publish_zoom_all(options.zoom_all);
        bridge.publish_universe(
            Arc::new(universe_view::to_universe_scene(universe, &bodies)),
            UniverseSettings {
                home: Some((home_system, home_body)),
                view: options.view,
            },
        );
        let mut scene = terrain::to_planet_scene(planet);
        scene.starts.clear();
        scene.nodes.clear();
        bridge.publish_planet(
            Arc::new(scene),
            OrbitSettings {
                max_distance: None,
                view: options.orbit,
                home: game.starts.first().map(|s| SurfacePos {
                    face: s.city_pos.face,
                    x: i64::from(s.city_pos.x),
                    y: i64::from(s.city_pos.y),
                }),
            },
        );
        game
    } else {
        match options.sphere {
            Some(n) => {
                let content = terrain::generate_sphere_content(options, n)?;
                let game = build_sphere_match(
                    &content.rules,
                    &content.planet,
                    &content.factions,
                    options.seed,
                )
                .map_err(|error| format!("setting up the match: {error}"))?;
                // Starts and nodes are drawn as entities (and only where the player may see them),
                // not as scene markers; the camera starts at the first player's city.
                let mut scene = terrain::to_planet_scene(&content.planet);
                scene.starts.clear();
                scene.nodes.clear();
                bridge.publish_planet(
                    Arc::new(scene),
                    OrbitSettings {
                        max_distance: None,
                        view: options.orbit,
                        home: game.starts.first().map(|s| SurfacePos {
                            face: s.city_pos.face,
                            x: i64::from(s.city_pos.x),
                            y: i64::from(s.city_pos.y),
                        }),
                    },
                );
                game
            }
            None => {
                let content = terrain::generate_content(options)?;
                bridge.publish_terrain(Arc::new(terrain::to_scene(&content.map)));
                build_match(
                    &content.rules,
                    &content.map,
                    &content.factions,
                    options.seed,
                )
                .map_err(|error| format!("setting up the match: {error}"))?
            }
        }
    };
    let wanderers = add_stress_units(&mut game, options.stress_units, options.seed);
    let driver = options
        .demo
        .then(|| DemoDriver::new(&game, options.seed, wanderers));
    let script = match &options.script {
        Some(path) => Some(read_script(path, &game)?),
        None => None,
    };
    SimThread::spawn(
        game.world,
        driver,
        script,
        options.speed_pct,
        bridge.clone(),
    )
    .map_err(|error| format!("could not start the simulation thread: {error}"))
}

/// Parses a `--script` file against player one's start.
fn read_script(path: &std::path::Path, game: &Match) -> Result<Script, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("reading {}: {error}", path.display()))?;
    let start = game
        .starts
        .iter()
        .find(|s| s.player == crate::selection::PLAYER)
        .ok_or("the match has no start for player one")?;
    let offset = |dx: i64, dy: i64| {
        let at = game.world.offset(start.city_pos, dx, dy);
        SurfacePos {
            face: at.face,
            x: i64::from(at.x),
            y: i64::from(at.y),
        }
    };
    Script::parse(&text, &ScriptContext::of(start), &offset)
        .map_err(|error| format!("{}: {error}", path.display()))
}
