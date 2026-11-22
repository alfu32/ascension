//! Writes the sound table: every `.ogg` in `app/assets/sounds/`, sorted by name, embedded with
//! `include_bytes!` and keyed by its file name without the extension.

use std::fmt::Write as _;
use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let dir = manifest.join("../../assets/sounds");
    let dir = dir.canonicalize().expect("app/assets/sounds exists");
    println!("cargo:rerun-if-changed={}", dir.display());

    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .expect("read app/assets/sounds")
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            (path.extension()? == "ogg").then(|| path.file_stem()?.to_str().map(str::to_owned))?
        })
        .collect();
    names.sort();

    let mut out =
        String::from("/// Every embedded sound: (name, Ogg Vorbis bytes), sorted by name.\n");
    out.push_str("pub(crate) static SOUND_FILES: &[(&str, &[u8])] = &[\n");
    for name in &names {
        let path = dir.join(format!("{name}.ogg"));
        println!("cargo:rerun-if-changed={}", path.display());
        writeln!(
            out,
            "    ({name:?}, include_bytes!({:?})),",
            path.display().to_string()
        )
        .unwrap();
    }
    out.push_str("];\n");
    let target = PathBuf::from(std::env::var("OUT_DIR").expect("out dir")).join("sounds.rs");
    std::fs::write(target, out).expect("write the sound table");
}
