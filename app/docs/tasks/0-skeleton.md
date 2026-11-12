# Task 0 — Workspace skeleton and rules-database build step

Crate: workspace root `app/` plus `rom-rules` and `rom-cli` (other crates are empty skeletons).

Read first: `AGENTS.md` (all of it), `app/docs/implementation-plan.md` (Ground rules, Phase 0), `app/docs/database.md` (first two sections), `app/db/schema.sql` header and the two `lint_*` views at the end.

## Build

1. Create a Cargo workspace rooted at `app/` (`app/Cargo.toml`, resolver 3, edition 2024, `rust-version = "1.99.1"`, `[workspace.package] version = "0.0.3"`, license `MIT OR Apache-2.0`). Add `app/rust-toolchain.toml` pinning channel `1.99.1` with `clippy` and `rustfmt`.
2. Create these member crates under `app/crates/`: `rom-rules`, `rom-sim`, `rom-mapgen`, `rom-ai`, `rom-save`, `rom-render`, `rom-app`, `rom-cli`.
   - `rom-cli` is a binary crate named `rom`. `rom-app` is a binary crate named `rise-of-man` whose `main` prints the version and exits (no GUI yet).
   - All others are library crates with a one-line crate doc comment and no code beyond what this task needs.
   - Do NOT add `gpui`, `gpui-component` or `raylib` yet. Only add dependencies this task uses.
   - Respect the dependency directions in the `AGENTS.md` crate table.
3. In `rom-rules`, implement:
   - `pub fn build_rules_db(schema_sql: &str, seed_sql: &str) -> Result<rusqlite::Connection, RulesBuildError>`: opens an in-memory SQLite database, enables foreign keys, executes the schema then the seed, then verifies: `PRAGMA foreign_key_check` returns no rows; view `lint_def_without_subtype` returns no rows; view `lint_unproducible` returns no rows. On any violation return a typed error that names the check and lists the offending rows (keys where available).
   - `pub fn write_rules_db(conn: &rusqlite::Connection, path: &Path) -> Result<(), RulesBuildError>`: writes the database to a file (use SQLite's backup API or `VACUUM INTO`), replacing an existing file.
   - `RulesBuildError` as a `thiserror` enum.
   - Use `rusqlite` with the `bundled` feature, exact latest stable version.
4. In `rom-cli`, implement the subcommand `rom build-rules --schema <path> --seed <path> --out <path>` that calls the two functions, prints `rules.db: <N> definitions, <M> tables` on success (N = row count of `def`, M = number of tables) and exits non-zero with the error message on failure. Argument parsing may be hand-written or use `clap` (exact version). `anyhow` is allowed in this binary only.
5. Add `app/.gitignore` ignoring `target/` and `*.db`.
6. Add `.github/workflows/ci.yml` (at the repository root) that, on push and pull request, on `ubuntu-latest`, with working directory `app`, installs the pinned toolchain and runs the three release-gate commands, then runs `cargo run -p rom-cli --locked -- build-rules --schema db/schema.sql --seed db/seed.sql --out target/rules.db`.
7. Commit `app/Cargo.lock`.

## Tests to add (in `rom-rules`)

- `seed_builds_clean`: `build_rules_db` on the real `app/db/schema.sql` + `app/db/seed.sql` (via `include_str!` with a relative path) succeeds and `def` has more than 50 rows.
- `broken_foreign_key_is_reported`: appending a seed statement that inserts a `cost` row with a non-existent `resource_id` yields the foreign-key error variant.
- `missing_subtype_is_reported`: appending `INSERT INTO def(kind,key,name) VALUES ('tech','tech.orphan','Orphan');` yields the missing-subtype variant and the error text contains `tech.orphan`.
- `unproducible_is_reported`: appending a new `entity_type` of class `unit` with no `production` row yields the unproducible variant naming its key.
- `write_and_reopen`: `write_rules_db` to a temp file, reopen it, and the `def` row count matches.

## Rules that matter most here

- Do not modify `app/db/schema.sql`, `app/db/seed.sql`, or anything under `research/` or `app/docs/`. If the schema seems wrong, stop and report it.
- Naming rules in `AGENTS.md` apply to every file you write.
- No `unsafe`.

## Done when (run from `app/`, all must succeed)

```
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo run -p rom-cli --locked -- build-rules --schema db/schema.sql --seed db/seed.sql --out target/rules.db
```

The last command must print a line starting with `rules.db: 100 definitions, 84 tables`.

## Out of scope

Typed rule structs, any simulation code, any graphics code, packaging.

## Finish

Commit your work on the current branch with a descriptive message (do not push). In your final message: the commands you ran with their outcome, files created, and anything you could not verify.
