//! Application start-up: render thread, GPUI window, clean shutdown.

use std::process::ExitCode;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use gpui::{
    App, AppContext, Application, Bounds, Entity, TitlebarOptions, Window, WindowBounds,
    WindowDecorations, WindowOptions, point, px, size,
};
use gpui_component::Root;
use rom_render::{
    Camera, OrbitSettings, RenderBridge, RenderWorker, ShutdownHandle, SurfacePos, UniverseSettings,
};
use rom_sim::BodyId;

use crate::args::Options;
use crate::bench;
use crate::demo::{DemoDriver, add_stress_units};
use crate::looks;
use crate::match_setup::Match;
use crate::screenshot;
use crate::script::{Script, ScriptContext, ScriptSetup};
use crate::sim_thread::{Extras, Game, SimLink, SimThread, UniverseHost};
use crate::terrain;
use crate::universe_view;
use crate::world_view::WorldView;
use rom_match::{Generated, MatchCore};

/// Longest wait for the render thread to mesh the terrain and bake its objects before the
/// simulation starts anyway.
const TERRAIN_WAIT: Duration = Duration::from_secs(30);

/// Opens the window and runs until it is closed (or the benchmark finishes).
pub fn run(options: &Options, arguments: &[String]) -> ExitCode {
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
    // A network match (task 88) starts in the lobby; its world comes from the server's start.
    let network = options.connect.is_some() || options.host.is_some();
    let started = if options.gallery.is_some() || options.model.is_some() {
        // The review screens show models on plain ground; no match runs.
        crate::gallery::start(options, &bridge).map(|()| None)
    } else if network {
        Ok(None)
    } else {
        start_simulation(options, arguments, &bridge).map(Some)
    };
    let sim = match started {
        Ok(sim) => sim,
        Err(error) => {
            eprintln!("rise-of-man: {error}");
            worker.stop();
            return ExitCode::FAILURE;
        }
    };
    let sim_link = sim.as_ref().map_or_else(SimLink::idle, SimThread::link);
    let sim = Arc::new(Mutex::new(sim));
    let server = Arc::new(Mutex::new(None));
    if options.sphere.is_none()
        && !options.single_planet
        && !options.uses_universe()
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
    bridge.publish_graticule(options.graticule);
    bridge.publish_grass(options.grass);
    bridge.publish_projectiles(options.projectiles);
    bridge.publish_turn(rom_render::TurnSettings {
        middle_pans: options.middle_drag_pans,
        invert_horizontal: options.invert_horizontal_turn,
        invert_vertical: options.invert_vertical_turn,
    });
    bridge.publish_start_markers(options.start_markers);
    let session = network.then(|| {
        crate::net::session::Session::start(
            options,
            crate::net::session::MatchSlots {
                bridge: bridge.clone(),
                sim: Arc::clone(&sim),
                server: Arc::clone(&server),
            },
        )
    });
    let exit_code = Arc::new(AtomicU8::new(0));
    let ctx = WindowCtx {
        options: options.clone(),
        bridge: bridge.clone(),
        shutdown: worker.shutdown_handle(),
        exit_code: Arc::clone(&exit_code),
        display_arguments: crate::saves::display_arguments(arguments),
        sim: Arc::clone(&sim),
    };

    application.run(move |cx: &mut App| {
        gpui_component::init(cx);
        // The in-match menu is GPUI; give it the launcher's dark look.
        gpui_component::Theme::change(gpui_component::ThemeMode::Dark, None, cx);
        {
            let theme = gpui_component::Theme::global_mut(cx);
            theme.colors.background = gpui::rgb(0x0c1118).into();
            theme.colors.foreground = gpui::rgb(0xf2f2f2).into();
            theme.colors.primary = gpui::rgb(0x3fd0ff).into();
            theme.colors.border = gpui::rgb(0x2a3340).into();
            theme.font_family = "DejaVu Sans".into();
        }
        let opened = match session {
            Some(session) => crate::net::lobby::open(cx, ctx.clone(), session),
            None => open_match_window(cx, &ctx, sim_link),
        };
        if let Err(error) = opened {
            eprintln!("rise-of-man: {error}");
            ctx.exit_code.store(1, Ordering::Relaxed);
            cx.spawn(async |cx| {
                let _ = cx.update(|cx| cx.quit());
            })
            .detach();
            return;
        }
        cx.activate(true);
    });
    let sim = sim.lock().ok().and_then(|mut sim| sim.take());
    if let Some(mut sim) = sim {
        sim.stop();
    }
    // The integrated host's server saves and stops with the window (task 88).
    let server = server.lock().ok().and_then(|mut server| server.take());
    if let Some(server) = server {
        server.stop();
    }
    worker.stop();
    ExitCode::from(exit_code.load(Ordering::Relaxed))
}

/// What opening the match window needs.
#[derive(Clone)]
pub(crate) struct WindowCtx {
    /// The launch options.
    pub options: Options,
    /// The render thread's bridge.
    pub bridge: RenderBridge,
    /// Stops the render thread.
    pub shutdown: ShutdownHandle,
    /// The process's exit code.
    pub exit_code: Arc<AtomicU8>,
    /// The launch arguments as a save keeps them.
    pub display_arguments: Vec<String>,
    /// The simulation thread, once a match runs (a network match starts after the lobby).
    pub sim: Arc<Mutex<Option<SimThread>>>,
}

/// Opens the match window over `sim`: a fixed canvas for the benchmark and screenshots, which
/// then run and end the program; otherwise a normal window.
pub(crate) fn open_match_window(
    cx: &mut App,
    ctx: &WindowCtx,
    sim_link: SimLink,
) -> Result<(), String> {
    let options = &ctx.options;
    let (width, height) = options.size;
    let canvas = size(px(width as f32), px(height as f32));
    let mut window_options = WindowOptions {
        titlebar: Some(TitlebarOptions {
            title: Some(crate::args::window_title().into()),
            ..Default::default()
        }),
        app_id: Some("rise-of-man".into()),
        ..Default::default()
    };
    window_options.window_decorations = Some(decorations(options));
    let fixed_window = options.bench_seconds.is_some() || options.screenshot.is_some();
    let (bridge, shutdown, display_arguments) = (
        ctx.bridge.clone(),
        ctx.shutdown.clone(),
        ctx.display_arguments.clone(),
    );
    let opened = if fixed_window {
        // Benchmark and screenshot windows have no component frame (which pads
        // client-decorated windows for a shadow), so the whole requested size is canvas even
        // at monitor size; a desktop title bar sits outside that canvas.
        window_options.window_bounds = Some(WindowBounds::Windowed(Bounds::new(
            point(px(0.0), px(0.0)),
            canvas,
        )));
        let exit_code = Arc::clone(&ctx.exit_code);
        let options = options.clone();
        cx.open_window(window_options, move |window, cx| {
            let view = world_view(
                &bridge,
                &shutdown,
                &sim_link,
                (options.size, options.stats),
                &display_arguments,
                window,
                cx,
            );
            view.update(cx, |view, _| view.ignore_pointer());
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
        let stats = options.stats;
        let size = options.size;
        window_options.window_bounds = Some(WindowBounds::centered(canvas, cx));
        cx.open_window(window_options, move |window, cx| {
            let view = world_view(
                &bridge,
                &shutdown,
                &sim_link,
                (size, stats),
                &display_arguments,
                window,
                cx,
            );
            cx.new(|cx| Root::new(view, window, cx))
        })
        .map(|_| ())
    };
    opened.map_err(|error| format!("could not open the window: {error:#}"))
}

/// The desktop's own title bar and borders (server-side decorations) unless `--no-chrome`
/// asks for a bare window.
fn decorations(options: &Options) -> WindowDecorations {
    if options.no_chrome {
        WindowDecorations::Client
    } else {
        WindowDecorations::Server
    }
}

/// Sets up the freshly opened window and creates the world view for it.
fn world_view(
    bridge: &RenderBridge,
    shutdown: &ShutdownHandle,
    sim: &SimLink,
    ((width, height), stats): ((u32, u32), bool),
    display_arguments: &[String],
    window: &mut Window,
    cx: &mut App,
) -> Entity<WorldView> {
    // GPUI's X11 window creation writes the title as Latin-1; setting it again writes the
    // UTF-8 `_NET_WM_NAME`, so the dash shows correctly in the title bar.
    window.set_window_title(&crate::args::window_title());
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
    let display_arguments = display_arguments.to_vec();
    cx.new(|cx| WorldView::new(bridge, sim, stats, display_arguments, window, cx))
}

/// Publishes the match's world to the render thread as the person playing `me` sees it first:
/// the universe and their home planet (or the planet or map), its looks and hidden terrain.
/// Returns the drawing half of a universe match.
pub(crate) fn show_match(
    options: &Options,
    built: &rom_match::Built,
    me: rom_sim::PlayerId,
    bridge: &RenderBridge,
) -> Result<Option<UniverseHost>, String> {
    let rules = Arc::clone(built.game.world.rules());
    let mut host = None;
    let own = built
        .game
        .starts
        .iter()
        .find(|s| s.player == me)
        .ok_or("the match has no start for this player")?;
    match &built.generated {
        Generated::Universe(universe) => {
            let game = &built.game;
            let bodies: Vec<u32> = universe.starts.iter().map(|s| s.0).collect();
            let home_body = own.city_pos.body.0;
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
            let sun = terrain::sun_settings(&rules, options);
            let mut universe_host = UniverseHost::new(BodyId(home_body), home_system);
            universe_host.sun = sun;
            for start in &game.starts {
                let at = start.city_pos;
                universe_host.remember_home_start(
                    at.body,
                    SurfacePos {
                        face: at.face,
                        x: i64::from(at.x),
                        y: i64::from(at.y),
                    },
                );
            }
            host = Some(universe_host);
            crate::sim_thread::publish_camera_limits(&game.world, me, bridge);
            bridge.publish_zoom_all(options.zoom_all);
            bridge.publish_universe(
                Arc::new(universe_view::to_universe_scene(universe, &bodies)),
                UniverseSettings {
                    home: Some((home_system, home_body)),
                    view: options.view,
                },
            );
            bridge.publish_ship_scales(Arc::new(looks::ship_scales(&rules)));
            bridge.publish_looks(Arc::new(looks::scene_looks(
                &rules,
                options.seed,
                planet.base,
            )));
            let mut scene = terrain::to_planet_scene(planet);
            terrain::apply_terrain_colours(&mut scene.feature_colors, &rules);
            scene.sun = sun;
            scene.starts.clear();
            scene.nodes.clear();
            bridge.publish_hidden_terrain(Arc::new(terrain::hidden_terrain(&game.world, me)));
            bridge.publish_planet(
                Arc::new(scene),
                OrbitSettings {
                    max_distance: None,
                    view: match (&options.look_at, planet.starts.first().copied()) {
                        (Some(look), Some(home)) => Some(
                            terrain::look_at(&rules, planet, home, look)
                                .ok_or_else(|| format!("--look-at: no tile matches {look:?}"))?,
                        ),
                        _ => options.orbit,
                    },
                    home: Some(SurfacePos {
                        face: own.city_pos.face,
                        x: i64::from(own.city_pos.x),
                        y: i64::from(own.city_pos.y),
                    }),
                    body: None,
                },
            );
        }
        Generated::Sphere(planet) => {
            let game = &built.game;
            // Starts and nodes are drawn as entities (and only where the player may see them),
            // not as scene markers; the camera starts at the first player's city.
            let mut scene = terrain::to_planet_scene(planet);
            terrain::apply_terrain_colours(&mut scene.feature_colors, &rules);
            scene.sun = terrain::sun_settings(&rules, options);
            scene.starts.clear();
            scene.nodes.clear();
            let home = Some(SurfacePos {
                face: own.city_pos.face,
                x: i64::from(own.city_pos.x),
                y: i64::from(own.city_pos.y),
            });
            let view = match (&options.look_at, home) {
                (Some(look), Some(home)) => Some(
                    terrain::look_at(&rules, planet, home, look)
                        .ok_or_else(|| format!("--look-at: no tile matches {look:?}"))?,
                ),
                _ => options.orbit,
            };
            bridge.publish_looks(Arc::new(looks::scene_looks(
                &rules,
                options.seed,
                planet.base,
            )));
            bridge.publish_hidden_terrain(Arc::new(terrain::hidden_terrain(&game.world, me)));
            bridge.publish_planet(
                Arc::new(scene),
                OrbitSettings {
                    max_distance: None,
                    view,
                    home,
                    body: None,
                },
            );
        }
        Generated::Flat(map) => {
            let mut scene = terrain::to_scene(map);
            terrain::apply_terrain_colours(&mut scene.feature_colors, &rules);
            bridge.publish_looks(Arc::new(looks::scene_looks(&rules, options.seed, map.base)));
            bridge.publish_hidden_terrain(Arc::new(terrain::hidden_terrain(&built.game.world, me)));
            bridge.publish_terrain(Arc::new(scene));
        }
    }
    Ok(host)
}

/// Generates the map or planet, publishes its terrain, builds the match and starts the
/// simulation thread. The match is built from the options' `MatchSettings` by the match core
/// (`rom_match`), as a server builds it.
fn start_simulation(
    options: &Options,
    arguments: &[String],
    bridge: &RenderBridge,
) -> Result<SimThread, String> {
    let setup = match &options.script {
        Some(path) => {
            let text = std::fs::read_to_string(path)
                .map_err(|error| format!("reading {}: {error}", path.display()))?;
            ScriptSetup::scan(&text).map_err(|e| format!("{}: {e}", path.display()))?
        }
        None => ScriptSetup::default(),
    };
    let built = options.match_settings(setup).build()?;
    let rules = Arc::clone(built.game.world.rules());
    let host = show_match(options, &built, crate::selection::PLAYER, bridge)?;
    let rom_match::Built {
        mut game,
        mut computers,
        generated,
        ..
    } = built;
    let universe = match generated {
        Generated::Universe(universe) => Some(universe),
        _ => None,
    };
    // A save replaces the freshly made world and its computer players; the planet's looks above
    // came from the same seed.
    let mut loaded = None;
    let mut speed_pct = options.speed_pct;
    if let Some(path) = &options.load {
        let (world, info, brains) = crate::computer::load_match(path, &rules)
            .map_err(|error| format!("loading {}: {error}", path.display()))?;
        game.world = world;
        computers = brains;
        speed_pct = info.speed_pct.clamp(1, 10_000);
        let home = game.starts.first().map(|s| s.city_pos.body);
        if let Some(at) = capital(&game.world, home) {
            match game.world.surface(at.body).map(|s| s.topology()) {
                Some(rom_sim::Topology::Cube(_)) => bridge.focus_surface(SurfacePos {
                    face: at.face,
                    x: i64::from(at.x),
                    y: i64::from(at.y),
                }),
                _ => bridge.focus_camera(at.x as f32 / 1000.0, at.y as f32 / 1000.0),
            }
        }
        loaded = Some(info);
    }
    let wanderers = add_stress_units(&mut game, options.stress_units, options.seed);
    let driver = options.demo.then(|| {
        let mut driver = DemoDriver::new(&game, options.seed, wanderers);
        // The demo only moves what no brain plays.
        for player in computers.players() {
            driver.release(player);
        }
        driver
    });
    let script = match &options.script {
        Some(path) => Some(read_script(path, &game, crate::selection::PLAYER)?),
        None => None,
    };
    // Feature objects are baked when the render thread takes the terrain: the match starts
    // only after that, so nothing is generated while it runs.
    let waiting = std::time::Instant::now();
    while !bridge.terrain_ready() && waiting.elapsed() < TERRAIN_WAIT {
        std::thread::sleep(Duration::from_millis(5));
    }
    // A one-shot zoom request must reach the prepared camera, not the empty startup view.
    if options.zoom_out {
        bridge.publish_zoom_out();
    }
    let saving = match crate::saves::folder() {
        Ok(folder) => {
            let style = options
                .style
                .clone()
                .or_else(|| {
                    rules
                        .map_styles
                        .keys()
                        .next()
                        .map(|id| rules.key(*id).to_owned())
                })
                .unwrap_or_default();
            let mut setup = crate::saves::SaveSetup::new(
                &game.world,
                options.seed,
                &style,
                arguments,
                options.autosave_minutes,
                folder,
            );
            if let Some(info) = loaded {
                // The save's own settings (seed, map, launch arguments, seats) carry on.
                setup.info = info;
            }
            Some(setup)
        }
        Err(error) => {
            eprintln!("rise-of-man: saving is off: {error}");
            None
        }
    };
    SimThread::spawn(
        Game::Lead(Box::new(MatchCore::new(game.world, computers, universe))),
        driver,
        script,
        speed_pct,
        bridge.clone(),
        host,
        Extras {
            saving,
            paused: options.load.is_some(),
            journals: None,
        },
    )
    .map_err(|error| format!("could not start the simulation thread: {error}"))
}

/// Where the player's first city stands, preferring one on `home`: where a loaded game's
/// camera starts.
fn capital(world: &rom_sim::World, home: Option<BodyId>) -> Option<rom_sim::Pos> {
    let cities: Vec<rom_sim::Pos> = world
        .entities()
        .filter(|e| {
            e.owner == crate::selection::PLAYER
                && world.rules().entity_types[&e.type_id]
                    .territory
                    .as_ref()
                    .is_some_and(|t| t.is_city)
        })
        .map(|e| e.pos)
        .collect();
    cities
        .iter()
        .find(|p| Some(p.body) == home)
        .or(cities.first())
        .copied()
}

/// Parses a `--script` file against the start of `me`, the person's player.
pub(crate) fn read_script(
    path: &std::path::Path,
    game: &Match,
    me: rom_sim::PlayerId,
) -> Result<Script, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("reading {}: {error}", path.display()))?;
    let start = game
        .starts
        .iter()
        .find(|s| s.player == me)
        .ok_or("the match has no start for this player")?;
    let offset = |dx: i64, dy: i64| {
        let at = game.world.offset(start.city_pos, dx, dy);
        SurfacePos {
            face: at.face,
            x: i64::from(at.x),
            y: i64::from(at.y),
        }
    };
    let mut context = ScriptContext::of(start);
    let world = &game.world;
    if let Some(home) = world.body(start.city_pos.body)
        && let Some(at) = world.body_position(home.id)
    {
        let mut rocks: Vec<(u64, u32)> = world
            .bodies()
            .iter()
            .filter(|b| b.system == home.system && b.kind == rom_rules::BodyKind::Resource)
            .filter_map(|b| Some((at.distance(world.body_position(b.id)?), b.id.0)))
            .collect();
        rocks.sort_unstable();
        context.rocks = rocks.into_iter().map(|(_, id)| id).collect();
        let mut planets: Vec<(u64, u32)> = world
            .bodies()
            .iter()
            .filter(|b| {
                b.system == home.system
                    && b.kind == rom_rules::BodyKind::Habitable
                    && b.id != home.id
            })
            .filter_map(|b| Some((at.distance(world.body_position(b.id)?), b.id.0)))
            .collect();
        planets.sort_unstable();
        context.planets = planets.into_iter().map(|(_, id)| id).collect();
        let mut neighbours: Vec<(i32, u32)> = world
            .routes()
            .iter()
            .filter_map(|r| match home.system {
                s if s == r.system_a => Some((r.length, r.system_b)),
                s if s == r.system_b => Some((r.length, r.system_a)),
                _ => None,
            })
            .collect();
        neighbours.sort_unstable();
        context.systems = neighbours.into_iter().map(|(_, id)| id).collect();
    }
    Script::parse(&text, &context, &offset).map_err(|error| format!("{}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_window_has_the_desktop_title_bar_unless_no_chrome() {
        let mut options = Options::default();
        assert_eq!(decorations(&options), WindowDecorations::Server);
        options.no_chrome = true;
        assert_eq!(decorations(&options), WindowDecorations::Client);
    }
}
