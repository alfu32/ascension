//! Rise of Man desktop shell: a GPUI window that shows the offscreen world view.

mod args;
mod bench;
mod demo;
mod gallery;
mod hud;
mod looks;
mod match_setup;
mod screenshot;
mod script;
mod selection;
mod shell;
mod sim_thread;
mod space_view;
mod terrain;
mod universe_view;
mod world_view;

use std::process::ExitCode;

fn main() -> ExitCode {
    let options = match args::parse(std::env::args().skip(1)) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("rise-of-man: {error}\n{}", args::USAGE);
            return ExitCode::from(2);
        }
    };
    if options.help {
        println!("rise-of-man {}\n{}", env!("CARGO_PKG_VERSION"), args::USAGE);
        return ExitCode::SUCCESS;
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
    shell::run(&options)
}
