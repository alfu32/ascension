# Task 112 — The launcher looks like it belongs to the game

Crates: `rom-app` (launcher, and the lobby and server windows task 88 adds), assets under
`app/assets/launcher/` (new), docs. Implementer: Claude (Opus). Starts after task 88 is released
(both work in `rom-app/src/launcher/`).

## Why — the owner (2026-11-20), verbatim

"rework the launcher, some things like map types , multiple chioices can be rendered better
( maybe even take screen caps of the game ). anyway cleanup the launcher and make it look nice,
like it belongs to the game"

## Today (0.0.106, task 87)

GPUI screens: a home screen with four bright-blue buttons (Play, Host a game, Join a game, Run a
server), short forms of labelled dropdowns, and the old full launcher behind "Advanced …" (seven
groups of plain controls). Nothing of the game's look: no frame art, no imagery, generic colours.
Screenshots: `app/docs/img/launcher-{simple,play,host,join,server,advanced}.png`.

## Build

1. **The game's look.** Take the palette, fonts and frame art from the in-match HUD
   (`app/docs/hud.md` "Style"; the vector chrome in `app/assets/hud/chrome/*.svg`; the fonts in
   `app/assets/fonts/`): dark navy panels, gold trim, teal accents, the ASCENSION / RISE OF MAN
   emblem. Rasterise the SVG chrome the way the game already does (no new image library if one
   is already in the tree) and use it for panels, buttons and the title. One shared style module
   for every launcher screen, so nothing is styled ad hoc.
2. **Pictures of the game.**
   - **Map styles as picture cards**: Planet, Solar, Cluster, Sector, Quadrant, Galaxy, each a card
     with a capture from the game (Planet: the planet orb with its orbit; Solar: the system view;
     Cluster, Sector, Quadrant, Galaxy: the star map), its name and one line on what it holds
     ("10–15 star systems", "1/8 to 1/6 of the galaxy" …, from the data, not hard-coded counts).
   - A backdrop for the home screen (a capture of a planet or the star map, dimmed under the
     panels).
   - Captures are made by a script with the game's own screenshot mode (self-ending runs), kept
     as small images under `app/assets/launcher/` (state the sizes; keep the set under 2 MB), and
     the script is committed so they can be remade when the look of the game changes.
3. **Choices rendered as choices.** A dropdown only where the list is long (nations). Short lists
   become visible choices: difficulty, start placement, disconnect policy and similar as
   segmented buttons; counts (computer opponents, remote players) as a stepper; races as cards
   with their emblem or portrait if the game has one; yes/no settings as toggles. The current
   value is always visible without opening anything.
4. **Clean up.**
   - Home screen: the emblem, the four main choices as large framed cards (an icon or picture and
     one line each), Continue last game, Advanced …, Quit. Nothing else.
   - Forms: one column of labelled rows with consistent spacing, help text in the HUD's muted
     colour, the main button gold, Back plain; the disabled reasons stay readable.
   - Advanced …: the same style, its groups as tabs or a side list, every option kept. No option
     may be lost: the existing tests that map forms to `Options` and to the command line keep
     passing.
   - The lobby and the server status window (task 88) get the same style.
5. **Size and scaling.** Readable at 1280×800 and at 1920×1080; the window may be resized without
   overlap or clipping.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/hud.md` (Style, Window chrome),
`app/docs/launcher.md`, `research/04-hud-and-screens.md` (setup screen), card 87 and card 88,
`rom-app/src/launcher/` (all), the HUD rasterising code in `rom-render` (search `chrome`), the
game's screenshot mode (`rom-app/src/screenshot.rs`) and the screenshot scripts under
`app/docs/tasks/*script*.txt`.

## Done when

- Gate green from `app/`; playthrough hashes unchanged; every existing launcher test passes (add
  tests for new controls: stepper bounds, segmented choice → `Options`).
- Screenshots you looked at, before and after side by side in the report:
  `app/docs/img/112-home.png`, `112-play.png` (map style cards visible), `112-host.png`,
  `112-join.png`, `112-server.png`, `112-advanced.png`, `112-lobby.png`, each at 1280×800, and
  `112-home-1920.png`.
- The capture script and the images in `app/assets/launcher/`, with their total size.
- Starting Play from the new launcher opens a match (one scripted, self-ending run).

Memory rules: `CARGO_BUILD_JOBS=4`; check `free -g` and `df -h /tmp`; run the app one instance at a
time in self-ending modes under `systemd-run --user --scope -q -p MemoryMax=6G -p
MemorySwapMax=0 env -u WAYLAND_DISPLAY …`; captures of big maps (Sector and up) one at a time.
Never `xdotool`, `wmctrl` or `sudo`, and send no input to windows; `import -window <id>` on your own
window is allowed. Do not stop processes you did not start. Wait loops must not use a `pgrep -f`
pattern that matches the loop itself. Logs only in `scratchpad/t112/`; never write
`scratchpad/gate*.sh`; never put a cargo target dir or a source copy in the scratchpad.

Commit on the branch (stage by name); do not push or merge. Final message: commands with real
output, files changed, the screenshots (before/after) you looked at, the asset sizes,
deviations, anything not verified.
