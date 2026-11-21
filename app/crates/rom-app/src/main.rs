//! Rise of Man desktop shell: a GPUI window that shows the offscreen world view.

// Release builds on Windows open no console window beside the game.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod args;
mod ascension_view;
mod bench;
mod chat;
mod computer;
mod demo;
mod gallery;
mod hud;
mod launcher;
mod looks;
mod match_setup;
mod net;
mod palette_icons;
mod saves;
mod screenshot;
mod script;
mod selection;
mod shell;
mod sim_thread;
mod space_view;
mod style;
mod terrain;
mod universe_view;
mod world_view;

use std::process::ExitCode;

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if launcher::requested(&arguments) {
        return launcher::run(&arguments);
    }
    let parsed = args::parse(arguments.clone()).and_then(|options| match options.load.clone() {
        // A save decides the map; its launch arguments come last so they win.
        Some(path) => {
            let arguments = saves::load_arguments(&arguments, &path)?;
            args::parse(arguments.clone()).map(|options| (options, arguments))
        }
        None => Ok((options, arguments)),
    });
    let (options, arguments) = match parsed {
        Ok(parsed) => parsed,
        Err(error) => {
            eprintln!("rise-of-man: {error}\n{}", args::USAGE);
            return ExitCode::from(2);
        }
    };
    if options.version {
        println!("rise-of-man {}", args::VERSION);
        return ExitCode::SUCCESS;
    }
    if options.help {
        println!("rise-of-man {}\n{}", args::VERSION, args::USAGE);
        println!(
            "\nNo arguments or --launcher opens the launcher. --run starts a default game. --version prints the version."
        );
        println!(
            "--load <file> continues a saved game, paused; --autosave <minutes> sets the autosave interval (default {}, 0 is off).",
            args::DEFAULT_AUTOSAVE_MINUTES
        );
        return ExitCode::SUCCESS;
    }
    // Run a server: a status window and no game (task 88).
    if options.serve {
        return net::server_window::run(&options);
    }
    if let Some(path) = &options.gallery_manifest {
        return match gallery::write_manifest(path) {
            Ok(count) => {
                println!("gallery-manifest {} models={count}", path.display());
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("rise-of-man: {error}");
                ExitCode::FAILURE
            }
        };
    }
    shell::run(&options, &arguments)
}
