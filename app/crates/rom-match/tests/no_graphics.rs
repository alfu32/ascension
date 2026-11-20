//! Task 85: the match core is built without any graphics crate, so a server builds it alone.

use std::collections::{BTreeMap, BTreeSet};

/// Every package `rom-match` depends on, directly or not, read from the workspace's lock file.
fn closure() -> BTreeSet<String> {
    let lock = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Cargo.lock"))
        .expect("Cargo.lock");
    let mut deps: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for package in lock.split("[[package]]").skip(1) {
        let name = package
            .lines()
            .find_map(|l| l.strip_prefix("name = "))
            .map(|n| n.trim_matches('"').to_owned())
            .expect("a name");
        let list = package
            .split("dependencies = [")
            .nth(1)
            .and_then(|rest| rest.split(']').next())
            .unwrap_or("");
        let names = list
            .lines()
            .map(|l| l.trim().trim_matches(',').trim_matches('"'))
            .filter(|l| !l.is_empty())
            .map(|l| l.split(' ').next().unwrap_or(l).to_owned())
            .collect();
        deps.entry(name).or_default().extend::<Vec<String>>(names);
    }
    let mut seen = BTreeSet::new();
    let mut open = vec!["rom-match".to_owned()];
    while let Some(name) = open.pop() {
        if seen.insert(name.clone()) {
            open.extend(deps.get(&name).into_iter().flatten().cloned());
        }
    }
    seen
}

#[test]
fn the_match_core_needs_no_graphics() {
    let all = closure();
    assert!(all.contains("rom-sim") && all.contains("rom-ai") && all.contains("rom-save"));
    for graphics in [
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
    ] {
        assert!(!all.contains(graphics), "rom-match pulls in {graphics}");
    }
}
