# Task 90 — The window's title bar by default, and the version on screen

Crates: `rom-app` (window options, arguments, launcher), `rom-render` (the version label in the
HUD). Implementer: Claude (Sonnet).

## Why — the owner (2026-11-18)

"I want to see the game/app version on the game window. I want to see the OS window chrome
(title bar having the default behaviour and control spots) as default, with the option to launch
chrome-less (as it is right now)."

## Build

1. **Title bar by default.** The game window opens with the desktop's own decorations: title
   bar, move by dragging, minimise, maximise, close, resize at the edges, double-click to
   maximise — whatever the window manager gives. GPUI's window options decide this
   (`titlebar`, `window_decorations`: server-side decorations on X11); find what GPUI 0.2.2
   offers and use the platform's own, not drawn imitations.
   - Closing with the title bar's close button goes through the game's own exit path (the same
     as Quit: the simulation stops cleanly, an autosave if one is due).
   - **Chrome-less** stays available: `--no-chrome` on the command line, and a "Window: with
     title bar / without" choice in the launcher (remembered). Without chrome, today's
     behaviour is unchanged.
   - Check what the in-match HUD does today that relied on having no title bar (its own
     move/close controls, the full-screen size); keep them working in both modes.
2. **Version shown.**
   - The title reads `Ascension — Rise of Man 0.0.NN` (the workspace version, `CARGO_PKG_VERSION`).
   - The HUD shows the version small and unobtrusive in a corner that nothing else uses (also
     in chrome-less mode), and the in-match menu shows it under its title. The launcher shows
     it too.
   - `rise-of-man --version` prints it and exits.
3. Screenshots, scripts and the bench are unaffected: `--screenshot`, `--size` and `--bench`
   give the same image size as before in both modes (the title bar is outside the drawn area).

Read first: `AGENTS.md`, `CLAUDE.md`, `research/02-rendering-ui-stack.md` (GPUI window,
X11), `rom-app/src/shell.rs`, `main.rs`, `args.rs`, `launcher/`, the GPUI 0.2.2 window
options in the cargo registry source.

## Done when

- Gate green from `app/`; playthrough hash unchanged.
- Tests: `--no-chrome` and the launcher choice set the window option; `--version` output;
  the HUD label text equals the package version.
- Screenshots you looked at, in `app/docs/img/`, scripts in `app/docs/tasks/90-*-script.txt`:
  `window-chrome.png` (the whole window with its title bar: a desktop capture of that window
  only, e.g. `import -window <id>` from ImageMagick if present, or the X11 root cropped to it —
  no window-manipulation tools), `window-no-chrome.png`, and the HUD corner with the version.
- By hand in the report: the close button ends the game cleanly (the process exits 0 and an
  autosave appears when due). If that cannot be done without a window-manipulation tool, say so;
  the owner will check it.

Run the app only with `env -u WAYLAND_DISPLAY` in self-ending modes. Never use `xdotool`,
`wmctrl` or `sudo` (they crashed the desktop's decoration helper before); do not stop processes
you did not start.

Commit on the branch (stage by name); do not push or merge. Final message: commands with real
output, files changed, what GPUI offered, deviations, anything not verified.
