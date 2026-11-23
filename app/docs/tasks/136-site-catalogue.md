# Task 136 — The site's catalogue in the review page's layout, with the age stories as chapters

Owner (chat, 2026-11-23), on the public site of task 135: "index page is good, im not happy with
the units presentation page, we shouldnt split music from unit pictures. cant you just use
https://claude.ai/artifact/Vp2yJ4jZPJRSAAAKQijr4v [the model review page] as index page …
put the age story as a chapter title in the presentation of that age's units, add a link to
privacy md, thats all." The home page (the README) stays as it is.

Files: `site/` (build, templates, styles, script) and, if the review builder needs a switch,
`app/docs/art/review-page/build.py` (without changing what it produces for the review page:
its output for a review round must stay byte-identical). No crate. Implementer: Claude (Opus).

## What changes

1. **One catalogue page** (`site/dist/catalogue.html`; `units.html`, `sounds.html` and
   `story.html` redirect to it with a plain link page or are dropped and every link updated):
   the **review page's own presentation**: its model cards with the pictures and that model's
   sounds on the same card, its grouping by age and kind, its sound players and shared cues,
   its look. Start from what `app/docs/art/review-page/build.py` writes (run it as task 135
   does and reuse its data, styles and card markup), and take out **every review element**:
   status selects, verdict buttons, notes and directions fields, fingerprints and model ids as
   text, round numbers and "new in this round" markers, the overview text about rounds, the
   database and anything that writes; no `claude.ai` links.
2. **Each age is a chapter**: the chapter title is the age (number counted from one, name,
   era), then its story paragraph (the italic "Story" of that age in `app/docs/ages-story.md`,
   or `app/assets/text/ages.toml`, which holds the same), then a player for that age's main
   theme (`app/assets/music/<age>.theme.ogg`); then that age's models. The main title's player
   opens the page. Models of "every age" (e.g. the citizen) get a short chapter of their own.
3. **Links**: the page links **Privacy** (`privacy.html`) and **Home**; the home page's header
   links **Catalogue** (one link instead of Story / Units & buildings / Sounds), **Privacy** and
   **Download**.
4. Keep task 135's rules: content-hashed files, link and leak checks (no email addresses, no
   review notes, no server paths, no claude.ai), name scan clean, light and dark, phone width,
   no third-party requests, audio `preload="none"`.

## Done when

- `python3 site/build.py --cache /home/alfu64/Development/rise-of-man-authoring/site-cache`
  builds (pictures come from the cache; no cargo build is needed or allowed in this task).
- Screenshots you have Read (headless Chrome as in task 135, served by `python3 -I site/serve.py
  --root site/dist --port <free port>`): the catalogue at desktop and phone width, light and
  dark, showing an age chapter with its story and theme player above its model cards, and a
  card with its pictures and sounds; the home page header.
- The review page's own build is unchanged (rebuild a round with and without your change and
  compare the output, if you touched its builder).
- Not deployed by you: the orchestrator runs `site/deploy.sh` after review.
- Report: files changed, what was removed from the review layout and how, screenshot paths,
  anything not verified.

Read first: `AGENTS.md`, `CLAUDE.md`, `site/README.md`, `site/build.py`,
`app/docs/tasks/135-public-site.md`, `app/docs/art/review-page/` (`build.py`, `page.html`,
`sprites.py`), `app/docs/ages-story.md`, `app/assets/text/ages.toml`.

Rules: no `sudo`, `xdotool` or `wmctrl`; do not stop processes you did not start; logs and the
Python environment only in the scratchpad's `t136/` (task 135's environment with markdown is in
the scratchpad's `t135/venv`); never write `scratchpad/gate*.sh`. Work in
`../rise-of-man-worktrees/136-site-catalogue` on branch `task/136-site-catalogue`; commit there
(stage by name); do not merge, tag or push.
