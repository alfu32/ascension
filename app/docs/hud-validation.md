# HUD task 11 validation

Implementation is ready for display-enabled review. Changes are intentionally
uncommitted, as requested for this sandbox handoff.

## Commands and actual results

Run from `app/` unless otherwise stated:

- `cargo fmt --all --check`: exit 0, no output.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`:
  `Finished dev profile [unoptimized + debuginfo] target(s) in 2.47s`;
  no Clippy findings.
- `cargo test --workspace --locked`: **257 passed, 0 failed**, across
  21 unit/integration/doc-test result groups. Includes all requested HUD
  tests, a source-coverage test, and option parsing.
- `cargo build --release -p rom-app --locked`:
  `Finished release profile [optimized] target(s) in 19.35s`.
- `cargo run -p rom-render --example hud_preview -- docs/img/hud-preview.png 1672 941`:
  `hud-preview docs/img/hud-preview.png 1672x941`.
- `cargo run -p rom-render --example hud_preview -- /tmp/hud-preview-720.png 1280 720`:
  `hud-preview /tmp/hud-preview-720.png 1280x720`.
- From the repository root, `python3 app/assets/hud/generate.py --check`:
  exit 0, no differences. All requested glyphs exist and are embedded.
- `cmp app/assets/fonts/DejaVuSans-Bold.ttf /usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf`:
  exit 0; the supplied font matches the system source.
- `git diff --check`: exit 0.

Cargo also emits an existing dependency future-compatibility notice about
`proc-macro-error2 2.0.1`. No warning suppression or dependency change was made.
The first Clippy attempt found four style findings in the new/partial HUD
code; they were fixed before the clean run. All executed tests passed.

## Visual review

Opened the two final previews and `01.HUD.png` as images. The previews retain
the top strip, three bottom groups, central minimap, 5 × 3 command slots,
production tracks and five data tabs. Navy fills, layered chamfered gold
frames, laurel emblems and cyan active edges follow the reference arrangement.
The first preview exposed oversized/truncated captions and thin frames;
subsequent passes corrected text sizing, strengthened bevels, fixed SVG glow
filter bounds, and placed the map beneath the frame's cut corners.

The final artwork is flatter, cleaner and less ornate than the painted
reference. Glyphs are simple gold symbols, the title follows the specified
ASCENSION / RISE OF MAN hierarchy, selection stats are absent in the empty
state, and the preview map/data are illustrative. Both sizes were inspected;
1280 × 720 preserves the arrangement with smaller labels.

## Outstanding display checks and API limits

No application window was opened. The three display-dependent commands were
skipped at the owner's instruction. Therefore there is **no HUD-on bench
line or verified FPS**, and `hud-current.png` was not created. The committed
preview candidate `img/hud-preview.png` is a CPU composition, not a game
screenshot. GPU upload appearance, the live world projection, interactive
camera behaviour and the 55 FPS target need the orchestrator's run.

The public simulation API has no event stream, so snapshots leave `events`
empty; drawing the last supplied event is implemented. Active research uses
public front orders and captured durations; pending submissions appear after
the next tick. Commerce cap and city limit use the same rules stats/constants
as required; no simulation or database changes were made. Income is shown
per configured game income period, and commerce is the largest capped-resource
rate against the per-resource cap.

## Files in this continuation

Created:

- `app/assets/hud/README.md`
- `app/crates/rom-app/src/hud.rs`
- `app/crates/rom-render/src/hud.rs`
- `app/crates/rom-render/src/hud_gpu.rs`
- `app/crates/rom-render/src/hud_preview.rs`
- `app/crates/rom-render/examples/hud_preview.rs`
- `app/docs/img/hud-preview.png`
- `app/docs/hud-validation.md`

Modified:

- `app/assets/hud/generate.py`
- `app/assets/hud/chrome/bar_fill.svg`
- `app/assets/hud/chrome/bottom_left.svg`
- `app/assets/hud/chrome/bottom_right.svg`
- `app/assets/hud/chrome/button.svg`
- `app/assets/hud/chrome/clock.svg`
- `app/assets/hud/chrome/emblem.svg`
- `app/assets/hud/chrome/minimap.svg`
- `app/assets/hud/chrome/topbar.svg`
- `app/crates/rom-app/src/args.rs`
- `app/crates/rom-app/src/main.rs`
- `app/crates/rom-app/src/shell.rs`
- `app/crates/rom-app/src/sim_thread.rs`
- `app/crates/rom-app/src/world_view.rs`
- `app/crates/rom-render/src/hud_atlas.rs`
- `app/crates/rom-render/src/hud_layout.rs`
- `app/crates/rom-render/src/hud_minimap.rs`
- `app/crates/rom-render/src/hud_text.rs`
- `app/crates/rom-render/src/lib.rs`
- `app/crates/rom-render/src/scene.rs`
- `app/crates/rom-render/src/worker.rs`
- `app/docs/hud.md`
- `app/docs/performance.md`
- `app/docs/tasks/STATUS.md`

The earlier WIP's remaining SVGs, all icon files, font/licence and exact
`resvg`/`fontdue` dependencies were retained. The 720p preview is a temporary
review artifact at `/tmp/hud-preview-720.png`.
