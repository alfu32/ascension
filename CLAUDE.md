# CLAUDE.md

Guidance for Claude Code in this repository.

@AGENTS.md

`AGENTS.md` holds the project rules shared with every implementer. The rest
of this file is about how the orchestrating session runs the work.

## Status

Implementation follows `app/docs/implementation-plan.md`, phase by phase.
The current state of every task is in `app/docs/tasks/STATUS.md`.

## Decisions already made (do not reopen without the owner)

- One map holds planetary terrains, open space and resource planets from the
  first tick. Each player's view widens when that player reaches the space age.
- Resource planets are claimed by keeping a starship beside them; planetary
  terrains host buildings and ground battles.
- Identity ladder: historical nation (ages 0–7) → continental bloc (8–9) →
  united humanity (10+).
- The game ends at age 13. The intergalactic stage and non-human races are parked.
- Simulation fixed at 20 ticks per game second; game speed is a user setting.
- SQLite is the authoring and save format; the tick loop never runs SQL.
- In-match HUD is drawn by raylib; all other screens are GPUI.

## Orchestration

The main session plans, writes task cards, reviews and merges. It delegates
implementation.

| Work | Who |
| --- | --- |
| Task cards, schema changes, review, release gate, merge | main session |
| Well-specified implementation tasks | Sonnet agent |
| Hard tasks: raylib-in-GPUI bridge, pathfinding and avoidance, computer-player military manager, shadow and material shaders | Opus agent |
| Mechanical work: seed data entry, repetitive structs, doc updates | Haiku agent |
| Extra implementer on well-specified tasks; second reviewer on merged work | Codex CLI |

Rules:

1. Every delegated task gets a card in the template from the implementation
   plan. An agent starts with no context: name the exact docs, tables and
   files it must read.
2. One git worktree per task, under `../rise-of-man-worktrees/<task>`. Parallel
   tasks must not share a crate. Task cards live in `app/docs/tasks/`; the
   ledger `app/docs/tasks/STATUS.md` is updated at every state change and
   describes what to do when an implementer hits a usage limit.
3. Review the diff and the gate output, not the agent's summary. A task is
   done only when the release gate passes in its worktree.
4. A task that fails review twice on one model is escalated, not retried a
   third time.
5. Never relax a hard rule to make a task pass. If the schema is missing
   something, change the schema deliberately (lint views empty, seed loads)
   and note it in `app/docs/database.md`.

### Codex CLI

Installed as `codex` (non-interactive: `codex exec`). It reads `AGENTS.md`.

```sh
# implement a task in a worktree; prompt is the task card
codex exec -m gpt-6.1-sol -c model_reasoning_effort=high \
  -C <worktree> -s workspace-write -c sandbox_workspace_write.network_access=true \
  --add-dir <repo>/.git -o <result-file> - < <task-card>
# second opinion on a branch
codex exec review
```

- Model: `gpt-6.1-sol` at high reasoning effort (owner's choice), passed per run.
- Network access lets `cargo` fetch crates.
- Codex cannot commit from a worktree (the shared `.git` stays read-only in its
  sandbox even with `--add-dir`). Tell it not to commit; the orchestrator
  commits after review.
- Pass sandbox and approval options per run; do not edit `~/.codex/config.toml`.
- It bills the owner's OpenAI account and sends prompts and file contents
  there. Use it for the purposes in the table above, not for anything
  containing secrets.
- Only its final message and the diff are visible, so review its output
  more strictly than a Claude agent's.

## Reference material on this machine

- Sibling project `../meshcraft` is the working example for the raylib 6 +
  GPUI bridge (`apps/desktop/src/gpui_app.rs`), sun and shadow map
  (`gpui_lighting.rs`, `assets/shaders/`), procedural materials and
  shader-defined vegetation, and the GPUI code editor. Port from it rather
  than re-deriving.
- The reference games are installed locally; their paths are in the session
  memory, not in this repository.

## Before publishing a version

Release gate green, database loads with empty lint views, name scan clean:

```sh
grep -rniE "<reference game names>" . --include='*.md' --include='*.sql' --include='*.rs' --include='*.toml'
```

Bump `version`, commit, tag without a `v` prefix, push branch and tag.

Standing instruction from the owner: **always commit, tag and push** finished,
verified work to `main` without asking. This covers ordinary forward commits
only — never force pushes, history rewrites or tag deletion.
