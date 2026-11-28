# Task 177 — Buildings turned 45° drawn true; the debug build's crash; the drop pod locked

Owner (chat, 2026-11-28): "do buildings turned 45 degrees, fix the debug build crash", and
"everything is reviewed and approved" (review rounds 21–23).

GitHub issue #180.

## 1. Buildings turned 45° are drawn stretched

- **Since task 165** (0.151.0), R / Shift+R turns any building 45°.
- **The stretch:** task 147's drawing rule stretches every building on a diagonal by √2 along
  its x. That is right for wall segments, gates and shield-line pieces: each covers a tile
  corner to corner and must join its neighbours along the line. It is wrong for an ordinary
  building: a 4×4 barracks turned 45° is drawn as a long diamond (`docs/img/165-turn.png`).
- **Fix:** only line pieces are stretched on a diagonal. Decide this from data (the entity has
  an `et_fortification` or `et_field_emitter` row, or a flag on the entity view filled from
  that), never by key. Every other building is drawn at its true size, turned 45°, centred on
  its footprint. The footprint stays as `turned_footprint` makes it (the bounding square);
  the model fits inside it.
- **The ghost matches:** the placement ghost uses the same rule, so what you place is what you
  get. This covers task 170's landing ghost too.
- **Check the corners:** a turned building's corners must not reach outside its footprint, or
  the placement check is wrong. If the true diagonal of a building is larger than the bounding
  square allows, say so and propose the fix (likely nothing, since the square of side max(w, h)
  holds a rotated w×h only if w and h are small enough: work it out and report).
- **Tests:** a barracks at 45° is drawn unstretched; a wall segment at 45° is still stretched;
  the ghost matches the drawing.
- **Screenshots:** retake `docs/img/165-turn.png` showing the turned barracks ghost unstretched,
  and add `docs/img/177-turned-buildings.png` with three buildings built at 0°, 45° and 90°
  beside a diagonal wall.

## 2. The debug build crashes before its first frame

- **What task 168 found:** the debug build of the game panics in
  `crates/rom-render/src/hud_gpu.rs:837` ("attempt to subtract with overflow") before its first
  frame. Release builds wrap silently, so they run, but the arithmetic is wrong there.
- **Fix:**
  - reproduce it (`cargo run -p rom-app` without `--release`, a short `--bench` or a
    `--script`, as other tasks run the game);
  - find why the value goes below zero;
  - fix the cause, not just the symptom (`saturating_sub` only if the value really may go
    below zero and clamping is the right meaning).
  - Then run the debug build through a short scripted match with a few views (surface, system
    view, cluster view) and fix any further debug-only panics it hits, or list them.
- **Test:** a unit test of the corrected computation at the inputs that overflowed.

## 3. Lock the drop pod

The owner approved the drop pod (round 23). Its models are kit models
(`kit.drop_pod`, `_hot`, `_20`, `_40`, `_60`, `_80`, `_open` in `crates/rom-kit/src/pods.rs`),
which `docs/art/approved.toml` does not cover.
- **Lock them:** add a fingerprint lock for approved kit models, either by extending
  `approved.toml` and its test to kit models, or with a test in `rom-kit` that compares each
  pod model's baked fingerprint with a recorded value.
- **Also lock** `kit.lift_engine` (task 170).
- **Document it:** say where the lock lives in `docs/models-in-meshcraft.md` or
  `docs/art-direction.md`.

## Crates

`rom-render` (diagonal drawing, `hud_gpu.rs`), `rom-app` (the ghost, if it lives there),
`rom-kit` or `rom-models` (the lock). No simulation change, so the hashes are unchanged.

## Done when

- **Gate green** from `app/`: fmt, clippy `-D warnings`, workspace tests (≥ main plus this
  card's tests).
- **Hashes:** playthrough and ascension unchanged; fight matrix and duel unchanged.
- **Debug run:** the debug build runs a short scripted match with no panic. Report the command.
- **Screenshots** opened with Read.
- **Locks:** every model in `approved.toml` keeps its fingerprint.
- Report as AGENTS.md says. Logs only in the scratchpad's `t177/`; never write `gate*.sh`.
