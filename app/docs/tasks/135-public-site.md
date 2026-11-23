# Task 135 — The public site on ascension-euw-1: home, privacy, the story, the units and sounds catalogues

Owner (chat, 2026-11-23), verbatim: "make a new static site on ascension-euw-1 that will include:
main page with the content of README.md with link privacy.md and links to the story, units
catalog and sounds catalog (as they are presented in the validation app but without the
validation elements). track the static site as a module(folder) in this project".

A new folder `site/` at the repository root (beside `app/`, `packaging/`, `research/`): the
generator, its templates and styles, the deploy script and the web server snippet. No crate is
changed. Implementer: Claude (Opus).

## What the site holds

- **Home** (`index.html`): the content of the repository's `README.md`, rendered (its images
  copied beside it; links to files in the repository become links to GitHub, links to releases
  stay as they are), with a site header linking **Story**, **Units & buildings**, **Sounds**,
  **Privacy**, and **Download** (the latest release on GitHub).
- **Privacy** (`privacy.html`): `packaging/microsoft-store-kit/documentation/PRIVACY.md`
  rendered (the same text the Store listing uses; this page becomes the Store's privacy URL).
- **Story** (`story.html`): from `app/docs/ages-story.md`, the public part only: the theme, and
  for each of the 14 ages its name, era, the story paragraph and the reasoning (forces, outcome,
  perception, art form, how the theme sounds), with that age's **main theme** as an audio player
  (`app/assets/music/<age>.theme.ogg`) and the main title at the top. Leave out the internal
  production sections (status of the sketches, the owner's rules for the pieces, "What comes
  next").
- **Units & buildings** (`units.html`): every model of the game, as the model review page shows
  it (`app/docs/art/review-page/`: the model sheet picture, the name, the age, the kind; grouped
  by age and kind), **without** the review elements: no status, no verdict buttons, no notes,
  no directions, no fingerprints or model ids beyond a readable name, no round numbers, no
  "approved/draft" labels, no database. Each entry links to its sounds on the sounds page.
- **Sounds** (`sounds.html`): every unit's and building's sounds, as the review page's sound
  cards present them (select, move, attack, fire, fight, moving, active, build_done, train_done,
  research_done …, from `app/tools/sounds/assignment.json` and the sound sprites the review page
  builds with `app/docs/art/review-page/sprites.py`), plus the shared cues (interface, ambience,
  activity), again without any review elements.

A clean, fast, static site in the game's look (navy and brass, the game's emblem
`packaging/ascension.png`), both light and dark, readable on a phone; no trackers, no cookies,
no third-party requests (fonts and scripts served from the site itself, or none); no analytics.
Audio with `preload="none"`.

## Build

1. `site/build.py` (Python 3 standard library plus at most one pinned Markdown package, set up
   in a throwaway environment described in `site/README.md`): reads the repository and writes
   the site into `site/dist/` (ignored by git: add `site/dist/` to `.gitignore`). It reuses the
   review page's data (`--gallery-manifest`, the model sheet screenshots, `sprites.py`) rather
   than re-deriving it: factor what both need into a function the review builder and the site
   builder share only if that is a small change; otherwise call the review page's scripts.
2. **Pictures**: the model sheets come from the game (`rise-of-man --model <id> --screenshot`),
   as the review page makes them, converted to web-sized JPEG/WebP. They are generated into
   `site/dist/` (or a cache folder outside git), never committed. The release binary must be
   built from this branch (main at 0.0.125 or later): one `cargo build --release --locked -p
   rom-app` under the memory rules below.
3. `site/deploy.sh`: builds if needed and copies `site/dist/` to the server with `rsync` over
   ssh (`ssh -i ~/oraci/ed25519-oraci-1 ubuntu@89.168.88.241`) into `/var/www/ascension-site/`
   (a folder the owner creates once with sudo, owned by `ubuntu`, readable by nginx). Atomic:
   upload to a new folder and switch a symlink, keeping the previous one for rollback.
4. `site/nginx/ascension.conf`: the full replacement for the server's
   `/etc/nginx/sites-available/ascension.conf` (its current text is in
   `~/ascension-nginx/ascension.conf` on the server: read it there, do not change the server's
   nginx yourself): the static site at `/` from `/var/www/ascension-site/current`, and the game
   server's own paths still proxied to `127.0.0.1:7400` exactly as now (find every path
   `rom serve` answers: at least `/status` and the WebSocket `/ws`; read
   `app/crates/rom-net/src/server/` to be sure, and keep the WebSocket upgrade headers and
   timeouts). Long cache headers for pictures and sounds, short for HTML; gzip for HTML/CSS.
   `site/README.md` gives the owner's one-time sudo steps (create the folder, install the
   config, `nginx -t`, reload) and the everyday `site/deploy.sh`.
5. Check the game clients still connect after the change: the launcher's network join uses
   `https://ascension-euw-1.duckdns.org/` (find where in `rom-app`/`rom-net` and confirm which
   paths it calls; if a client fetches `/` itself, keep that path for the game server, e.g. by
   moving the site under a prefix, and say so).

## Done when

- `python3 site/build.py` builds the site from a clean checkout plus the release binary;
  `site/dist/` holds the five pages, the pictures, the sounds and the music players' files.
- You looked at each page (render them with a headless browser or the game's own screenshot
  tools are not suitable: use `chromium --headless --screenshot` or `wkhtmltoimage` if
  installed, or `firefox --screenshot`; if none works, say so) at desktop and phone width, and
  Read the screenshots.
- Links checked (every internal link and file exists; no link points to the private review
  page or claude.ai).
- Nothing private is published: no email addresses, no server paths, no review notes, no
  owner's verdicts, no keys. The name scan (no reference game or studio names) passes on
  `site/` and on the generated pages.
- **Not deployed by you**: do not run `site/deploy.sh` and do not change the server. The
  orchestrator deploys after review, and the owner installs the nginx config.
- Report: the files, the pages with sizes, the total size of `dist/`, the nginx paths kept for
  the game server, the owner's sudo steps, what you could not verify.

Read first: `AGENTS.md`, `CLAUDE.md`, `README.md`, `app/docs/ages-story.md`,
`app/docs/art/review-page/` (`build.py`, `sprites.py`, `page.html`), `app/docs/sound.md`,
`app/tools/sounds/assignment.json`, `app/docs/music.md`,
`packaging/microsoft-store-kit/documentation/PRIVACY.md`, `app/crates/rom-net/src/server/`.

Memory rules: `CARGO_BUILD_JOBS=4`; check `free -g`; the release build under
`systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0`. Never `xdotool`, `wmctrl`
or `sudo`. Do not stop processes you did not start (the owner may be playing). Logs and the
Python environment only in the scratchpad's `t135/`; never write `scratchpad/gate*.sh`; no cargo
target dir in the scratchpad or /tmp. Work in `../rise-of-man-worktrees/135-public-site` on
branch `task/135-public-site`; commit there (stage by name); do not merge, tag or push.
