//! Rise of Man desktop shell: a GPUI window that shows the offscreen world view.

mod args;
mod bench;
mod demo;
mod hud;
mod match_setup;
mod screenshot;
mod shell;
mod sim_thread;
mod terrain;
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
    shell::run(&options)
}
