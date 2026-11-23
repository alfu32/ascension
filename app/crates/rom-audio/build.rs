//! Writes the sound table: every `.ogg` in `app/assets/sounds/`, sorted by name, embedded with
//! `include_bytes!` and keyed by its file name without the extension; and the music table: every
//! `.ogg` in `app/assets/music/` the same way, with the cue sheet `cues.json` (task 133).

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// The `.ogg` files of `dir` by stem, sorted.
fn stems(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            (path.extension()? == "ogg").then(|| path.file_stem()?.to_str().map(str::to_owned))?
        })
        .collect();
    names.sort();
    names
}

/// A table `name: &[(&str, &[u8])]` of the `.ogg` files of `dir`.
fn table(out: &mut String, name: &str, doc: &str, dir: &Path) {
    println!("cargo:rerun-if-changed={}", dir.display());
    writeln!(out, "/// {doc}: (name, Ogg Vorbis bytes), sorted by name.").unwrap();
    writeln!(out, "pub(crate) static {name}: &[(&str, &[u8])] = &[").unwrap();
    for stem in stems(dir) {
        let path = dir.join(format!("{stem}.ogg"));
        println!("cargo:rerun-if-changed={}", path.display());
        writeln!(
            out,
            "    ({stem:?}, include_bytes!({:?})),",
            path.display().to_string()
        )
        .unwrap();
    }
    out.push_str("];\n");
}

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("out dir"));
    let assets = manifest.join("../../assets");

    let sounds = assets
        .join("sounds")
        .canonicalize()
        .expect("app/assets/sounds exists");
    let mut out = String::new();
    table(&mut out, "SOUND_FILES", "Every embedded sound", &sounds);
    std::fs::write(out_dir.join("sounds.rs"), out).expect("write the sound table");

    let music = assets
        .join("music")
        .canonicalize()
        .expect("app/assets/music exists");
    let mut out = String::new();
    table(
        &mut out,
        "MUSIC_FILES",
        "Every embedded piece of music",
        &music,
    );
    let cues = music.join("cues.json");
    println!("cargo:rerun-if-changed={}", cues.display());
    writeln!(
        out,
        "/// The cue sheet of the music (`cues.json`).\npub(crate) static MUSIC_CUES: &str = include_str!({:?});",
        cues.display().to_string()
    )
    .unwrap();
    std::fs::write(out_dir.join("music.rs"), out).expect("write the music table");
}
