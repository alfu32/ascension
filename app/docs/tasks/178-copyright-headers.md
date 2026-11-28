# Task 178 — A copyright notice at the top of every source file

Owner, GitHub issue #182 ("Copyright", 2026-11-28), verbatim: "Please take care that each and
every source file in the project bears in the header the copyright notice"

## The notice

One line, the same everywhere, in each language's comment syntax:

    Copyright (c) 2026 alfu32. All rights reserved. See LICENSE in the repository root.

- Rust, JavaScript, JSX, CSS (`/* … */`), shaders: `// Copyright …` as the first line, above any
  `//!` module doc.
- Python, shell, TOML, YAML: `# Copyright …`, after a `#!` line where there is one, and after a
  Python encoding line.
- SQL: `-- Copyright …`.
- HTML and the page templates: `<!-- Copyright … -->`, after a `<!doctype>` line.
- SVG: `<!-- Copyright … -->` after the XML declaration, if any.

## Which files

- **Every file authored for this project:** Rust (`.rs`, `build.rs` included), Python (`.py`),
  shell (`.sh`), SQL (`app/db/schema.sql`, `app/db/seed.sql`, `app/db/models/**/*.sql`),
  shaders, the site's HTML, CSS and JS, the `Cargo.toml` files, the GitHub workflow YAML, and
  the HUD and launcher SVG art under `app/assets/`.
- **Not:**
  - third-party or vendored files: `app/vendor/`, `LICENSE-*`, the DejaVu fonts;
  - generated build output: `target/`, `Cargo.lock`, `dist`;
  - JSON (no comments);
  - binary assets (images, sounds, fonts);
  - Markdown documents, which are not source.
- **Generated sources:** `app/db/seed.sql` and the model files come from generators
  (`app/db/content/generate.py`, the model export/import in `rom-models`). The generators write
  the header themselves, so a regeneration keeps it; `generate.py --check` and the model
  round-trip tests must still pass.

## A check that keeps it so

Add `scripts/release/check_headers.py`. It lists the files that should carry the notice (the
rules above) and fails when one does not, naming the files.
- **Where it runs:** the release workflow's `validate` job, beside the release-script tests.
- **Fixing:** it has a `--fix` mode that adds the line, used once now.
- **Rule for agents:** AGENTS.md gets a one-line rule: every new source file starts with the
  notice.

## Outside this repository

The server controller (`~/ascension-admin` on the VPS: `app.py`, `server.jsx`, tests) gets the
same line. The orchestrator does that over ssh.

## Done when

- `python3 -I scripts/release/check_headers.py` passes and lists how many files it checked by
  type.
- **Gate green:** fmt (the header must not upset rustfmt), clippy, workspace tests, `generate.py
  --check`, the database loads with empty lint views.
- **No behaviour change:** playthrough and ascension hashes unchanged.
- **Locked models:** every model in `app/docs/art/approved.toml` keeps its fingerprint. A comment
  line in a model `.sql` file must not change the baked model. If it does, stop and report.
