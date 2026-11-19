//! Owner launcher: rules-backed forms and independent game processes.

mod assets;
mod form;
mod models;
mod ui;

use std::collections::{BTreeMap, BTreeSet};
use std::process::ExitCode;
use std::time::Duration;

use gpui::{App, AppContext, Application, TitlebarOptions, WindowBounds, WindowOptions, px, size};
use gpui_component::{Root, Theme, ThemeMode};
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

struct Catalog {
    lists: BTreeMap<&'static str, Vec<Choice>>,
    factions: Vec<Choice>,
    terrains: Vec<Choice>,
    /// Computer player profiles and difficulties (by rank).
    profiles: Vec<Choice>,
    difficulties: Vec<Choice>,
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
                ("age", age_choices),
                ("gallery", gallery),
                ("model", models),
                ("view", choices(&["surface", "globe", "system", "starmap"])),
                ("sun", choices(&["orbit", "fixed", "night"])),
                ("graticule", choices(&["on", "off"])),
                ("grass", choices(&["on", "off"])),
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
    let failed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let app_failed = failed.clone();
    application.run(move |cx: &mut App| {
        gpui_component::init(cx);
        Theme::change(ThemeMode::Dark, None, cx);
        {
            let theme = Theme::global_mut(cx);
            theme.colors.background = gpui::rgb(0x0c1118).into();
            theme.colors.foreground = gpui::rgb(0xf2f2f2).into();
            theme.colors.primary = gpui::rgb(0x3fd0ff).into();
            theme.colors.border = gpui::rgb(0x2a3340).into();
            theme.font_family = "DejaVu Sans".into();
        }
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(1100.), px(850.)), cx)),
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
