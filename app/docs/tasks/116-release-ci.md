# Task 116 — GitHub release matrix, store packages, store listings, a front page

Files: `.github/workflows/`, `packaging/` (new, at the repository root as in electrocraft),
`scripts/release/` (new), `README.md` (repository root), docs. Game crates only where a platform
build needs a fix (see "Game code" below). Implementer: Claude (Opus).

## Why — the owner (2026-11-21), verbatim

"do the ci github release matrix, the same as for ../electrocraft ( but i want the msi artefacts
as well ) … ms store: use the same structure for the msstore store listings as for ../electrocraft
( the languages columns in the csv). snapstore : the name is ascension, github: do a nice front
page with pictures and some explanations of game functional capabilities/game mechanics,
technical info remains at the end. commit, tag, push, trigger build on pushed tag until all
errors clear out and we got a github release. you can force push the same tag if changes are
effected only to the ci related stuf and not te game itself."

Microsoft Store identity (owner):

| Field | Value |
| --- | --- |
| Package/Identity/Name | `K3D0.ascension-rts-platform` |
| Package/Identity/Publisher | `CN=7F0CBC95-8035-4AF2-9B17-C38D58351CE5` |
| PublisherDisplayName | `K3D0` |
| Package Family Name | `K3D0.ascension-rts-platform_tfmtd4yrk952g` |
| Store ID | `9PHHR2KQKVJ2` |

Snap Store name: `ascension`.

## The model: `../electrocraft` (read it; do not copy its names or content)

- `.github/workflows/release.yaml`: jobs `tag` (tag = app version), `validate` (fmt, clippy,
  tests, release-script tests), `linux` (x86_64 and aarch64: AppImage + zsync, portable archive,
  snap built by `scripts/release/snap.py`, installed and started under Xvfb with a screenshot
  colour check), `desktop` (Windows x64 and arm64: portable + MSIX with the Store identity,
  signed when PFX secrets exist; macOS arm64 and x86_64: portable), `publish` (SHA256SUMS, one
  GitHub release, marked latest).
- `scripts/release/{common,appimage,portable,snap,msix}.py` + `test_release.py`;
  `packaging/windows/{package.ps1,store-identity.json,*.ico}`; `packaging/linux/` (desktop file,
  metainfo, AppRun); `packaging/msstore-identity.txt`, `packaging/snapstore`.
- `packaging/microsoft-store-kit/`: `source/` (en-us copy, localized copy, supported languages,
  screenshots list, `build_submission.py`, `render_artwork.py`, tests), `locales/<lang>/…`,
  `artwork/*.svg`, `documentation/` (privacy, certification, submission notes), and the generated
  `listingData-<StoreID>-….csv` with **one column per language**. Ascension uses the same
  structure, the same language set, and its own text and screenshots.

## Build

1. **Workflow** `.github/workflows/release.yaml`, triggered by a pushed version tag (no `v`
   prefix; equal to `version` in `app/Cargo.toml`) and by `workflow_dispatch` on a tag. Same jobs
   and runners as electrocraft. The workspace root is `app/`. Build `rom-app` (binary
   `rise-of-man`) and `rom-cli` (binary `rom`). Native dependencies of GPUI + raylib on Linux.
2. **Artefacts**: Linux AppImage (+ zsync) and portable `.tar.gz` per arch; snaps
   `ascension_<ver>_<arch>.snap` (name `ascension`), installed and started in the job (screenshot
   colour check as electrocraft, with a self-ending run such as `--bench` or `--screenshot`);
   Windows: MSIX (Store identity above) **and an MSI** (WiX or cargo-wix; Program Files install,
   Start-menu shortcut, uninstall entry) for x64 and arm64, plus portable `.zip`; macOS: portable
   archives (arm64, x86_64). Everything the game reads at run time travels in every package
   (assets, the rules database or what builds it, fonts, shaders, scripts): find how the app
   locates them and make each package start from any working directory.
3. **Microsoft Store kit** `packaging/microsoft-store-kit/` modelled on electrocraft: Ascension's
   title, short and long description, features, keywords, hardware notes, release notes, in every
   language electrocraft has (translate faithfully; no names of other games, studios or fiction);
   screenshots from `app/docs/img/` and the launcher captures (Store sizes); artwork rendered from
   the game's own emblem/chrome; privacy (the game collects nothing; network play talks only to
   the server the player enters); the generated `listingData-9PHHR2KQKVJ2-….csv` with the
   language columns. Note in the kit's README that the owner should compare it with a fresh
   Partner Center export, since the field rows come from that template.
4. **Front page** `README.md` at the repository root: title and emblem, pictures from
   `app/docs/img/` (planet, cities, armies, the star map, space fleets, painting a world, the
   launcher), what the game is and how it plays (ages 0–13, nations → blocs → humanity, round
   planets and star systems, routes and charged jumps, inhibitors, markets, computer opponents,
   ascension and bringing worlds to life, network play with a server), how to install (the
   release artefacts), then the technical part (Rust, raylib + GPUI, SQLite rules, determinism,
   crates, building from source, running a server: link `app/docs/network.md`). Keep the current
   README's technical facts; nothing private (no emails, VPS addresses or keys).
5. **Iterate to a release.** Commit, push, tag, watch the run (`gh run watch`, `gh run view
   --log-failed`), fix, repeat until the release exists with every artefact.

## Rules for pushing (owner's authorization for this task)

- Push to `main`; stage files by name; commit messages end with
  `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- **CI-only changes** (workflow, `packaging/`, `scripts/release/`, `README.md`, docs, store kit):
  move the current version tag to the new commit and force-push **that tag only**
  (`git tag -f X.Y.Z && git push -f origin X.Y.Z`). Never force-push `main`; never delete other
  tags; never rewrite history.
- **Game code** (anything under `app/crates`, `app/db`, `app/assets`, `app/Cargo.*` other than
  what packaging strictly needs): run the full gate from `app/` first (fmt, clippy, tests,
  `python3 db/content/generate.py --check`, database load with empty lint views, release build,
  `rom build-rules` + `check-rules` + `rom playthrough --db target/rules.db --seed 1` with
  hash=2b4a0812188cd904 and ascension hash=a252236eec91a838), then bump the patch version in
  `app/Cargo.toml` (`cargo metadata --offline` to refresh the lock), commit "Release X.Y.Z: …",
  tag the new version normally and push; the release then happens at that tag. Platform fixes
  must not change game behaviour (hashes) on Linux.
- A failed run's GitHub release must not be left half-made: the publish job creates the release
  only when every job passed; if a release for the tag already exists from an earlier attempt,
  delete that release (not the tag) before re-running.
- Never put secrets in the repository; the MSIX is signed only when the owner adds PFX secrets
  (name them as electrocraft does, `ASCENSION_PFX_B64`, `ASCENSION_PFX_PASSWORD`).

## Done when

- A GitHub release at the version tag with: AppImages (+zsync), snaps, Linux/Windows/macOS
  portable archives, MSIX and MSI for x64 and arm64, SHA256SUMS; all jobs green.
- The Store kit's generated CSV has every language column and passes its own tests.
- `README.md` renders on GitHub with the pictures.
- Report: the run URL, the release URL, the artefact list with sizes, every game-code change (with
  gate output), what is not verified (e.g. the MSI and MSIX were not installed on a real
  Windows machine; macOS archives not started), and what the owner must do (signing secrets,
  Partner Center import, snap name registration and upload).

Memory rules: `CARGO_BUILD_JOBS=4`; check `free -g`; local runs of the app self-ending under
`systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0 env -u WAYLAND_DISPLAY …`.
Never `xdotool`, `wmctrl` or `sudo`. Do not stop processes you did not start. Logs only in
`scratchpad/t116/`; never write `scratchpad/gate*.sh`; no cargo target dir or source copy in the
scratchpad. Never write the names of other games or studios, or the word for star-lane routes
other than "route".
