//! Task 86: the server binary builds without window libraries and, unless asked, without TLS
//! (on the VPS nginx terminates TLS).

use std::collections::BTreeSet;
use std::process::Command;

/// The packages `cargo tree` lists for `rom-cli` with these extra arguments.
fn tree(extra: &[&str]) -> BTreeSet<String> {
    let output = Command::new(env!("CARGO"))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args([
            "tree",
            "-p",
            "rom-cli",
            "-e",
            "normal",
            "--prefix",
            "none",
            "--locked",
            "--offline",
        ])
        .args(extra)
        .output()
        .expect("cargo tree runs");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.split(' ').next())
        .map(str::to_owned)
        .collect()
}

#[test]
fn the_server_needs_no_graphics_and_no_tls() {
    let plain = tree(&[]);
    assert!(plain.contains("rom-net") && plain.contains("tungstenite"));
    for name in [
        "rom-render",
        "rom-app",
        "gpui",
        "gpui-component",
        "raylib",
        "raylib-sys",
        "resvg",
        "fontdue",
        "image",
        "x11",
        "wayland-client",
        "rustls",
        "ring",
        "rustls-native-certs",
    ] {
        assert!(!plain.contains(name), "rom-cli pulls in {name}");
    }
    let tls = tree(&["--features", "tls"]);
    assert!(tls.contains("rustls") && tls.contains("rustls-native-certs"));
    assert!(!tls.contains("raylib") && !tls.contains("gpui"));
}
