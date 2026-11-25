//! Owner launcher: rules-backed forms and independent game processes.

mod assets;
mod form;
mod models;
mod presets;
mod scripts;
mod simple;
mod ui;

use std::collections::{BTreeMap, BTreeSet};
use std::process::ExitCode;
use std::time::Duration;

use gpui::{
    App, AppContext, Application, KeyBinding, TitlebarOptions, WindowBounds, WindowOptions, px,
    size,
};
use gpui_component::Root;
use rom_rules::Rules;

use form::{Config, config_path};

/// Empty invocation selects the launcher; explicit launch mode may carry a shutdown timer.
pub(crate) fn requested(arguments: &[String]) -> bool {
    arguments.is_empty() || arguments.first().is_some_and(|a| a == "--launcher")
}

#[derive(Clone)]
struct Choice {
    key: String,
    label: String,
}

impl gpui_component::select::SelectItem for Choice {
    type Value = String;
    fn title(&self) -> gpui::SharedString {
        self.label.clone().into()
    }
    fn value(&self) -> &String {
        &self.key
    }
}

/// The map styles as the launcher lists them: by `def.sort` (Planet, Solar, Cluster, Sector,
/// Quadrant, Galaxy in the seed, task 109), then id.
fn styles_in_order(rules: &Rules) -> Vec<rom_rules::DefId> {
    let mut ids: Vec<_> = rules.map_styles.keys().copied().collect();
    ids.sort_by_key(|id| (rules.def(*id).sort, *id));
    ids
}

/// A race the person may start as, with its nations (race cards, then the nation).
#[derive(Clone)]
struct Race {
    key: String,
    name: String,
    nations: Vec<Choice>,
}

/// One map style as a picture card: its key, name and a line on what it holds.
#[derive(Clone)]
struct StyleCard {
    key: String,
    name: String,
    line: String,
}

/// The playable factions grouped by the race above them (the faction itself when none is).
fn races(rules: &Rules, playable: &[rom_rules::DefId]) -> Vec<Race> {
    let race_of = |id: rom_rules::DefId| {
        let mut at = id;
        let mut seen = BTreeSet::new();
        while seen.insert(at) {
            match rules.factions.get(&at) {
                Some(f) if f.kind == rom_rules::FactionKind::Race => return at,
                Some(f) => match f.parent_id {
                    Some(parent) => at = parent,
                    None => break,
                },
                None => break,
            }
        }
        id
    };
    let mut groups: BTreeMap<rom_rules::DefId, Vec<Choice>> = BTreeMap::new();
    for &id in playable {
        groups.entry(race_of(id)).or_default().push(Choice {
            key: rules.key(id).into(),
            label: rules.def(id).name.clone(),
        });
    }
    groups
        .into_iter()
        .map(|(race, nations)| Race {
            key: rules.key(race).into(),
            name: rules.def(race).name.clone(),
            nations,
        })
        .collect()
}

/// What a map style holds, in one line, from its rules row and the star catalogue.
fn style_line(rules: &Rules, style: &rom_rules::MapStyle) -> String {
    use rom_rules::StarRegion;
    match style.star_region {
        StarRegion::Single if style.shared_home => {
            "One planet: every player on it, its moons around it".into()
        }
        StarRegion::Single => format!(
            "One star system: every home planet, {}–{} resource bodies",
            style.resource_bodies_per_system_min, style.resource_bodies_per_system_max
        ),
        StarRegion::Nearest => format!(
            "{}–{} star systems, routes between them",
            style.systems_min, style.systems_max
        ),
        StarRegion::Wedge => {
            let (low, high) = rom_mapgen::wedge_size(style, rules.stars.len());
            format!(
                "1/{} to 1/{} of the galaxy: {low}–{high} star systems",
                style.region_min_den, style.region_max_den
            )
        }
        StarRegion::All => format!("The whole galaxy: all {} star systems", rules.stars.len()),
    }
}

struct Catalog {
    lists: BTreeMap<&'static str, Vec<Choice>>,
    factions: Vec<Choice>,
    terrains: Vec<Choice>,
    /// Computer player profiles and difficulties (by rank).
    profiles: Vec<Choice>,
    difficulties: Vec<Choice>,
    /// The playable races and nations, by race.
    races: Vec<Race>,
    /// The map styles as picture cards, in the launcher's order.
    styles: Vec<StyleCard>,
    /// The style of one planet every player shares (the advanced Planet workflows).
    planet_style: String,
    /// The start placement the rules use when none is chosen.
    placement_default: String,
    /// The rules' default mix of factions for eight players, by key.
    mix: Vec<String>,
    /// The rules this launcher was started with; a save must match them to load.
    rules_id: crate::saves::RulesId,
}

impl Catalog {
    fn new(rules: &Rules) -> Self {
        let definitions = |ids: Vec<rom_rules::DefId>| {
            ids.into_iter()
                .map(|id| Choice {
                    key: rules.key(id).into(),
                    label: format!("{} · {}", rules.def(id).name, rules.key(id)),
                })
                .collect::<Vec<_>>()
        };
        let choices = |names: &[&str]| {
            names
                .iter()
                .map(|s| Choice {
                    key: (*s).into(),
                    label: (*s).into(),
                })
                .collect::<Vec<_>>()
        };
        let mut ages: Vec<_> = rules.ages.values().collect();
        ages.sort_by_key(|age| age.ordinal);
        let age_choices = ages
            .iter()
            .map(|age| Choice {
                key: age.ordinal.to_string(),
                label: rules.def(age.id).name.clone(),
            })
            .collect();
        let mut models = Vec::new();
        let mut entities = BTreeSet::new();
        for (&entity, visuals) in &rules.entity_visuals {
            for visual in visuals {
                if rules
                    .meshes
                    .get(&visual.mesh_id)
                    .is_some_and(|mesh| mesh.generator == crate::looks::COMPOSED)
                {
                    let id = rom_models::visual_model_id(rules, entity, visual);
                    models.push(Choice {
                        key: id.clone(),
                        label: id,
                    });
                    entities.insert(entity);
                }
            }
        }
        let mut gallery = choices(&["all", "units"]);
        gallery.extend(ages.iter().map(|age| {
            Choice {
                key: rules
                    .key(age.id)
                    .strip_prefix("age.")
                    .unwrap_or(rules.key(age.id))
                    .into(),
                label: rules.def(age.id).name.clone(),
            }
        }));
        gallery.extend(definitions(entities.into_iter().collect()));
        gallery.extend(models.clone());
        Self {
            rules_id: crate::saves::RulesId::of(rules),
            lists: BTreeMap::from([
                ("style", definitions(styles_in_order(rules))),
                (
                    "start-placement",
                    rules
                        .match_option_choices
                        .get(rom_mapgen::StartPlacement::OPTION)
                        .map(|rows| {
                            let mut rows: Vec<_> = rows.iter().collect();
                            rows.sort_by_key(|row| row.sort);
                            rows.into_iter()
                                .map(|row| Choice {
                                    key: row.value.clone(),
                                    label: row.name.clone(),
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                ),
                ("city-area", choices(&["on", "off"])),
                ("gather-rate", choices(&["100", "50", "25", "10", "5", "2"])),
                (
                    "research-time",
                    choices(&["100", "200", "400", "1000", "2000"]),
                ),
                ("age", age_choices),
                ("gallery", gallery),
                ("model", models),
                ("view", choices(&["surface", "globe", "system", "starmap"])),
                ("sun", choices(&["orbit", "fixed", "night"])),
                ("graticule", choices(&["on", "off"])),
                ("grass", choices(&["on", "off"])),
                ("detail", choices(&["min", "medium", "max", "ultra"])),
                ("projectiles", choices(&["on", "off"])),
                ("middle-drag", choices(&["turn", "pan"])),
                (
                    "on-disconnect",
                    simple::drop_choices()
                        .into_iter()
                        .map(|(key, label, _)| Choice {
                            key: key.into(),
                            label: label.into(),
                        })
                        .collect(),
                ),
                (
                    "gallery-view",
                    choices(&["game", "front", "side", "top", "close"]),
                ),
                ("pose", choices(&["idle", "move", "attack", "work"])),
                (
                    "size",
                    choices(&["1280x800", "1920x1080", "2560x1440", "3840x2160"]),
                ),
            ]),
            races: races(
                rules,
                &rom_match::start_age(rules)
                    .map(|age| rules.playable_start_factions(age))
                    .unwrap_or_default(),
            ),
            styles: styles_in_order(rules)
                .into_iter()
                .map(|id| StyleCard {
                    key: rules.key(id).into(),
                    name: rules.def(id).name.clone(),
                    line: style_line(rules, &rules.map_styles[&id]),
                })
                .collect(),
            planet_style: styles_in_order(rules)
                .into_iter()
                .find(|id| {
                    let style = &rules.map_styles[id];
                    style.star_region == rom_rules::StarRegion::Single && style.shared_home
                })
                .map(|id| rules.key(id).to_owned())
                .unwrap_or_default(),
            placement_default: rules
                .match_options
                .get(rom_mapgen::StartPlacement::OPTION)
                .map(|o| o.default_value.clone())
                .unwrap_or_default(),
            mix: rom_match::default_factions(rules, 8)
                .into_iter()
                .map(|id| rules.key(id).into())
                .collect(),
            factions: definitions(rules.factions.keys().copied().collect()),
            terrains: definitions(rules.terrains.keys().copied().collect()),
            profiles: rules
                .ai_profiles
                .values()
                .map(|p| Choice {
                    key: p.key.clone(),
                    label: p.name.clone(),
                })
                .collect(),
            difficulties: {
                let mut list: Vec<_> = rules.ai_difficulties.values().collect();
                list.sort_by_key(|d| d.rank);
                list.into_iter()
                    .map(|d| Choice {
                        key: d.key.clone(),
                        label: d.name.clone(),
                    })
                    .collect()
            },
        }
    }

    fn first_model(&self) -> Option<&str> {
        self.lists.get("model")?.first().map(|c| c.key.as_str())
    }
}

/// Remembers the seat token a server gave for `url`, so the Join form offers Rejoin (task 88).
pub(crate) fn remember_token(url: &str, token: &str) -> Result<(), String> {
    let path = config_path()?;
    let mut config = Config::load(&path)?;
    config
        .tokens
        .insert(url.trim().to_owned(), token.to_owned());
    config.save_tokens(&path)
}

/// Keeps the Sound sliders changed in a match's menu (task 133) with the launcher's settings, so
/// the next game and the launcher take them.
pub(crate) fn remember_sound(settings: &crate::sound::Settings) -> Result<(), String> {
    let path = config_path()?;
    let mut config = Config::load(&path)?;
    config.form.set_sound(settings);
    config.save(&path)
}

/// Opens the launcher without starting raylib or a simulation.
pub(crate) fn run(arguments: &[String]) -> ExitCode {
    // Launcher-only --after is a timer for visual inspection; it is never forwarded to a child.
    let timer = if arguments.is_empty() || arguments == ["--launcher"] {
        None
    } else if arguments.len() == 3 && arguments[0] == "--launcher" && arguments[1] == "--after" {
        match arguments[2]
            .parse::<f64>()
            .ok()
            .filter(|s| s.is_finite() && *s > 0.0 && *s <= 86400.0)
        {
            Some(seconds) => Some(Duration::from_secs_f64(seconds)),
            None => {
                eprintln!("launcher: --after expects 0 < seconds <= 86400");
                return ExitCode::from(2);
            }
        }
    } else {
        eprintln!("usage: rise-of-man --launcher [--after <seconds>]");
        return ExitCode::from(2);
    };
    let prepared = (|| {
        let catalog = Catalog::new(&crate::terrain::load_rules()?);
        let path = config_path()?;
        let (config, warning) = match Config::load(&path) {
            Ok(config) => (config, None),
            Err(error) => (
                Config::default(),
                Some(format!("Could not load saved form: {error}")),
            ),
        };
        let executable = std::env::current_exe().map_err(|e| e.to_string())?;
        Ok::<_, String>((catalog, path, config, warning, executable))
    })();
    let (catalog, path, config, warning, executable) = match prepared {
        Ok(prepared) => prepared,
        Err(e) => {
            eprintln!("launcher: {e}");
            return ExitCode::FAILURE;
        }
    };
    let application = match std::panic::catch_unwind(Application::new) {
        Ok(application) => application.with_assets(assets::Assets),
        Err(_) => {
            eprintln!("launcher: could not initialise windowing");
            return ExitCode::FAILURE;
        }
    };
    // The launcher's controls click (task 125), as loud as the Sound settings say.
    crate::sound::install(
        config.form.sound_settings(),
        config.form.get("mute") == "true",
    );
    let failed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let app_failed = failed.clone();
    application.run(move |cx: &mut App| {
        gpui_component::init(cx);
        cx.bind_keys([
            KeyBinding::new("escape", ui::Back, Some(ui::CONTEXT)),
            KeyBinding::new("enter", ui::Confirm, Some(ui::CONTEXT)),
        ]);
        crate::style::install(cx);
        // 1280 × 800 unless a scripted capture asks for another size (`ROM_LAUNCHER_SIZE`).
        let (width, height) = std::env::var("ROM_LAUNCHER_SIZE")
            .ok()
            .and_then(|s| {
                let (w, h) = s.split_once('x')?;
                Some((w.parse::<f32>().ok()?, h.parse::<f32>().ok()?))
            })
            .filter(|(w, h)| (640.0..=7680.0).contains(w) && (480.0..=4320.0).contains(h))
            .unwrap_or((1280., 800.));
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(width), px(height)), cx)),
            window_min_size: Some(size(px(960.), px(640.))),
            titlebar: Some(TitlebarOptions {
                title: Some(format!("Rise of Man · Launcher {}", crate::args::VERSION).into()),
                ..Default::default()
            }),
            app_id: Some("rise-of-man-launcher".into()),
            ..Default::default()
        };
        let opened = cx.open_window(options, move |window, cx| {
            window.set_window_title(&format!("Rise of Man · Launcher {}", crate::args::VERSION));
            let view = cx.new(|cx| {
                ui::Launcher::new(config, catalog, path, executable, warning, window, cx)
            });
            cx.new(|cx| Root::new(view, window, cx))
        });
        if let Err(e) = opened {
            eprintln!("launcher: could not open window: {e}");
            app_failed.store(true, std::sync::atomic::Ordering::Relaxed);
            cx.spawn(async |cx| {
                let _ = cx.update(|cx| cx.quit());
            })
            .detach();
        }
        if let Some(timer) = timer {
            cx.spawn(async move |cx| {
                cx.background_executor().timer(timer).await;
                let _ = cx.update(|cx| cx.quit());
            })
            .detach();
        }
        cx.activate(true);
    });
    crate::sound::shutdown();
    if failed.load(std::sync::atomic::Ordering::Relaxed) {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invocation_routing_preserves_existing_arguments() {
        assert!(requested(&[]));
        assert!(requested(&["--launcher".into()]));
        for flag in [
            "--help", "--stats", "--demo", "--bench", "--sphere", "--run", "",
        ] {
            assert!(!requested(&[flag.into()]));
        }
        // A game option's value must never be interpreted as a routing flag.
        assert!(!requested(&["--screenshot".into(), "--launcher".into()]));
    }

    #[test]
    fn catalog_uses_rules_and_composed_model_ids() {
        let rules = crate::terrain::load_rules().unwrap();
        let catalog = Catalog::new(&rules);
        assert_eq!(catalog.factions.len(), rules.factions.len());
        assert_eq!(catalog.terrains.len(), rules.terrains.len());
        assert_eq!(catalog.lists["age"].len(), rules.ages.len());
        assert_eq!(catalog.mix.len(), 8);
        assert!(catalog.mix.iter().all(|key| {
            catalog
                .races
                .iter()
                .any(|r| r.nations.iter().any(|n| &n.key == key))
        }));
        assert_eq!(catalog.profiles.len(), rules.ai_profiles.len());
        assert_eq!(catalog.difficulties.len(), rules.ai_difficulties.len());
        assert!(
            catalog
                .profiles
                .iter()
                .any(|c| c.key == crate::computer::DEFAULT_SPEC.0)
        );
        // Task 109: the map styles in the owner's order, by def.sort.
        assert_eq!(
            catalog.lists["style"]
                .iter()
                .map(|c| c.label.as_str())
                .collect::<Vec<_>>(),
            [
                "Planet · map.one_world",
                "Solar · map.solar",
                "Cluster · map.many_worlds",
                "Sector · map.sector",
                "Quadrant · map.quadrant",
                "Galaxy · map.galaxy",
            ]
        );
        // Task 112: races group every playable nation; each map style has a line from its data.
        let playable = rom_match::start_age(&rules)
            .map(|age| rules.playable_start_factions(age))
            .unwrap_or_default();
        assert!(!playable.is_empty());
        assert_eq!(
            catalog.races.iter().map(|r| r.nations.len()).sum::<usize>(),
            playable.len()
        );
        assert!(catalog.races.len() > 1 && catalog.races.len() < playable.len());
        assert_eq!(catalog.styles.len(), rules.map_styles.len());
        assert!(catalog.styles.iter().any(|s| s.key == catalog.planet_style));
        assert!(catalog.styles.iter().all(|s| !s.line.is_empty()));
        assert!(
            catalog.lists["start-placement"]
                .iter()
                .any(|c| c.key == catalog.placement_default)
        );
        let book = crate::looks::ModelBook::new(&rules);
        assert_eq!(
            catalog.lists["model"]
                .iter()
                .map(|c| &c.key)
                .collect::<Vec<_>>(),
            book.entries.iter().map(|m| &m.id).collect::<Vec<_>>()
        );
    }
}
