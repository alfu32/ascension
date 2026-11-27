# Task 171 — The server controller: every match option of the launcher, and a page that looks like the game

Owner (chat, 2026-11-27), verbatim:
- "you need to adapt the controller on the server, many settings that are in the launcher are
  in the service launcher nicht."
- "maybe take the opportunity annd make the page nicer since we already have assets ...."

GitHub issue #174.

## What the controller is

The **Ascension server** page of the owner's ArchiSource instance starts, stops and sets up the
public game server (`rom serve` as the systemd user service `ascension.service` on the VPS).
It has two parts, both versioned in the git repository `~/ascension-admin` on the VPS:

- **The service, `app.py`** (Python, standard library only, `ascension-admin.service` on
  127.0.0.1:18081, behind nginx at `https://archisource-app.duckdns.org/ascension/admin/api/`).
  ArchiSource authenticates every request (permission `ascension.server.manage`). The service:
  - reads its choices from the game's rules database (`choices()`);
  - checks options (`checked_options()`);
  - turns them into `rom serve` flags (`match_args()`);
  - stores presets and archives matches.
- **The page, `server.jsx`**, a React page that ArchiSource runs in the browser. It is copied
  to `~/archisource/pages/ascension/server.jsx`.

The orchestrator has fetched both files to
`/tmp/claude-1000/-home-alfu64-Development-rise-of-man/27c2b360-87db-40cf-bec9-ba7d69bfbe7f/scratchpad/admin2/`.
Work on a copy in `scratchpad/t171/`. **You do not deploy**: the orchestrator reviews and
deploys over ssh.

## What changes

1. **Every match option `rom serve` takes** is in the service and the page, checked as strictly
   as the current ones. Run `~/ascension-bin/rom serve --x` on the VPS through the orchestrator,
   or read `crates/rom-cli` and `crates/rom-match` in the game repo, for the list.

   Missing today:
   - **`--home-size`** (planet face size: what range the launcher allows);
   - **`--city-area on|off`** (task 158);
   - **`--gather-rate`** (100, 50, 25, 10, 5, 2; task 159);
   - **research pace** (task 166 is renaming it). Today's binary takes `--research-time` (100,
     200, 400, 1000, 2000). After task 166 merges it takes `--research-speed` (1000, 500, 200,
     100, 50, 25, 20, 10, 5; lower is slower).

     The page shows **Research speed** in both cases. The service reads the binary's usage line
     once (`rom serve --x` prints it) to decide which flag to pass, converting speed to time as
     10000 / speed where needed. Speeds with no time equivalent in the old binary are refused
     with a clear message.
   - **`--factions`** (one faction per seat, from the rules' faction list, blank for the
     default mix);
   - **`--takeover profile:difficulty`** (who takes over a dropped human);
   - anything else the usage line lists that a host would set.

   Ranges and choice lists come from the rules database or the usage line, as `choices()`
   already does for styles, ages and profiles, never from guesses. Presets and the stored
   options keep working: an old preset without the new keys gets the defaults.
2. **The page looks like the game.** Use the game's own art and colours:
   - the launcher chrome in `app/assets/launcher/chrome/` (`panel.svg`, `panel_ornate.svg`,
     `button.svg`, `button_gold.svg`, `frame.svg`, `toggle_on/off.svg`);
   - the HUD chrome and icons in `app/assets/hud/`;
   - the map-style pictures in `app/assets/launcher/maps/` and `tiles/`;
   - the public site's style (`site/static/site.css`) for the palette and type.

   Keep it readable and fast:
   - **Art delivery:** inline the SVGs the page needs. A page file can hold them, or you can
     propose another way the page gets them, such as a `/static` path on the public site
     `https://ascension-euw-1.duckdns.org/` (`site/`). Never put secrets or server paths in the
     page.
   - **Layout:** a clear match-setup form grouped as the launcher's Play/Host form groups it
     (map, players, pace, rules, server behaviour); the server state, the log and archived
     matches.
   - **Width:** works on a phone width too.
   - **Themes:** dark by default like the game; a light variant is optional.
   - **Accessibility:** every control keeps a label; colours keep enough contrast.
3. **The summary line** (`summary()` in the page) shows the new options when they are not the
   defaults, the way the lobby does.
4. **Documentation:** a short `README.md` in the controller folder describes:
   - the options;
   - how the flag for research is chosen;
   - how to deploy (copy `app.py`, restart `ascension-admin.service`; copy `server.jsx` to
     `~/archisource/pages/ascension/`).

## Done when

- `python3 -m py_compile app.py` passes, and a small `test_app.py` (standard library
  `unittest`) passes. It covers:
  - `checked_options` accepts every new option at its limits and refuses values outside them;
  - `match_args` builds the right flags for both research flag forms;
  - old presets without the new keys load.
- The page is checked in a browser with a local harness: an HTML file that loads React from a
  CDN, mocks the API, and renders `server.jsx` with its data. Screenshots are taken with a
  headless browser if one is installed (chromium or firefox), else described.
  - Shots: desktop and phone widths, the setup form open, and a running match. Save them in
    `scratchpad/t171/shots/` and open each with Read.
  - If the local ArchiSource checkout (`~/Development/ArchiSource`) can run the page more
    faithfully without much effort, say so, but do not start long builds.
- Report:
  - the files changed, with a diff against `scratchpad/admin2/`;
  - the options added, and their ranges and where each comes from;
  - how the art reaches the page;
  - anything you could not verify.

Rules: no `sudo`, no xdotool or wmctrl, no secrets in files, no names of reference games. Logs
and work only in `scratchpad/t171/`. Do not ssh to the VPS yourself; ask the orchestrator for
anything you need from it, in your report.
