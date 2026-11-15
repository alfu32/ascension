//! Embeds the model files (`db/models/**/*.sql`, `assets/models/**/*.mesh`) so the game carries
//! them as it carries the seed.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

fn files(root: &Path, extension: &str, out: &mut Vec<(String, PathBuf)>) {
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == extension) {
                let relative = path
                    .strip_prefix(root)
                    .expect("inside the root")
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join("/");
                out.push((relative, path));
            }
        }
    }
    out.sort();
}

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let app = manifest.join("../..");
    let mut code = String::new();
    for (name, dir, extension) in [
        ("SQL", app.join("db/models"), "sql"),
        ("MESHES", app.join("assets/models"), "mesh"),
    ] {
        println!("cargo:rerun-if-changed={}", dir.display());
        let mut found = Vec::new();
        files(&dir, extension, &mut found);
        let _ = writeln!(code, "/// Embedded `{extension}` files by path.");
        let _ = writeln!(code, "pub static {name}: &[(&str, &str)] = &[");
        for (relative, path) in found {
            println!("cargo:rerun-if-changed={}", path.display());
            let path = path.canonicalize().expect("model file path");
            let _ = writeln!(
                code,
                "    ({relative:?}, include_str!({:?})),",
                path.display().to_string()
            );
        }
        code.push_str("];\n");
    }
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("out dir")).join("embedded.rs");
    std::fs::write(out, code).expect("write embedded model list");
}
