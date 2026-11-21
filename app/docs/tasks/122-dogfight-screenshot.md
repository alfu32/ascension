# Task 122 — Replace the outdated dogfight screenshot everywhere (issue #5)

Files only, no crate code: `app/docs/img/orbital-dogfight.png`, `README.md`, the Store kit
(`packaging/microsoft-store-kit/`, `packaging/microsoft-store/04-dogfight.png`), one session
script `app/docs/tasks/122-dogfight-script.txt`. Implementer: Claude (Sonnet).

## Why — the owner, verbatim (issue #5, "Outdated photo")

"Change this photo asset everywhere please."

The photo is `app/docs/img/orbital-dogfight.png` (task 58, 2026-11-16): a few grey fighters over
an early city beside a big plain red block (a building drawn as a placeholder box), an old HUD,
small picture. It is the README's "Aircraft fighting over a city" picture and Store screenshot 4
"Aircraft duel over a city".

## Build

1. Stage a dogfight with today's game (main): fighters of two players in a real fight over a
   built-up city, with today's models — no placeholder boxes in view (if a building in the shot
   still draws as a plain box, frame it out and name it in the report). High or ultra detail,
   daylight, a 1920×1080 window, HUD visible, the aircraft large enough to read. Use a self-ending
   script (`--script`, see the `1xx-*-script.txt` files in `app/docs/tasks/` and card 108's camera
   bench) and keep it as `app/docs/tasks/122-dogfight-script.txt` so the shot can be retaken.
2. Look at the captures yourself and pick the best one. Replace `app/docs/img/orbital-dogfight.png`
   with it (same file name, so every reference keeps working; PNG, 8-bit RGB).
3. Store kit: regenerate with `python3 packaging/microsoft-store-kit/source/build_submission.py`
   (Pillow; use a venv in the scratchpad `t122/` if Pillow is missing — never install system-wide)
   so `packaging/microsoft-store/04-dogfight.png`, the manifest, the validation report and
   `documentation/screenshots.html` follow. Run `test_submission.py`. Only the screenshot-4 files
   and the generated hashes may change; if anything else moves, say so and do not commit it.
4. README: keep the caption true to the new picture (adjust the alt text and the Store caption
   in `source/screenshots.json` only if the picture shows something else).
5. `grep -rn orbital-dogfight` over the repo: every reference listed in the report.

## Done when

- The new picture committed, looked at, and no other picture changed.
- `build_submission.py` and `test_submission.py` pass; the diff touches only the files above.
- Report: the commands, the chosen capture (and why), the references updated.

Read first: `AGENTS.md`, `CLAUDE.md`, `README.md`, `packaging/microsoft-store-kit/README.md`,
`app/docs/tasks/108-camera-bench-script.txt`, any `app/docs/tasks/*-script.txt` with aircraft
(grep `fighter`), and `app/docs/unit-series.md` for the aircraft keys.

Memory rules: `CARGO_BUILD_JOBS=4`; check `free -g`; the app runs self-ending under
`systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0 env -u WAYLAND_DISPLAY …`.
Never `xdotool`, `wmctrl` or `sudo`. Do not stop processes you did not start (the owner may be
playing). Logs only in the scratchpad's `t122/`; never write `scratchpad/gate*.sh`; no cargo
target dir in the scratchpad or /tmp. Work in `../rise-of-man-worktrees/122-dogfight-screenshot`
on branch `task/122-dogfight-screenshot`; commit there (stage by name); do not merge, tag or push.
