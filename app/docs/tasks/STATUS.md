# Task ledger

The single source of truth for what is assigned, running and done. Any
session or implementer can resume from this file and the task cards beside
it; nothing needed to continue lives only in a chat or a temporary folder.

| Task | Card | Branch | Implementer | State | Notes |
| --- | --- | --- | --- | --- | --- |
| 0 — workspace skeleton and rules-database build | `0-skeleton.md` | `task/0-skeleton` | Codex (`gpt-6-luna`, started before the model was chosen) | merged in 0.0.3 | Gate re-run by the orchestrator: fmt, clippy, 5 tests, `rules.db: 100 definitions, 84 tables`. Codex could not commit from the worktree; the orchestrator committed. |
| 1 — stack proof: raylib world view inside GPUI, measured | `1-stack-proof.md` | `task/1-stack-proof` | Opus agent | merged in 0.0.9 | Gate re-run by the orchestrator after merging main: fmt, clippy, all tests; short benchmark 59.2 fps at 1280×800. Agent measured 59.8–60.0 fps at 1882×1048 (the largest window this display allows); 4K not measured. Read-back dominates the render thread (≈11 ms of 16.7 at that size on an integrated GPU). Deviations: mailbox polled every 8 ms; `latest_frame()` takes the frame. Follow-up: asynchronous read-back before higher resolutions. |
| 2 — typed rules loader, validation, modifier engine | `2-rules-loader.md` | `task/2-rules-loader` | Codex (`gpt-6.1-sol`, high) | merged in 0.0.6 | First run stopped correctly on a schema conflict (`game_constant.value` was `REAL`, fixed in 0.0.5). Second run complete. Gate re-run by the orchestrator: fmt, clippy, 39 tests, `rules ok: 100 definitions, 4 modifiers`. |
| 3 — simulation core: state, tick loop, order book, straight-line movement | `3-sim-core.md` | `task/3-sim-core` | Codex (`gpt-6.1-sol`, high) | merged in 0.0.7 | Gate re-run by the orchestrator: fmt, clippy, 80 tests; same seed gives the same hash, another seed a different one; 10,000 units × 6,000 ticks in 2.0 s. Follow-ups: entity slots are never compacted; order kind is stored as text per order. |
| 4 — production, construction and research orders | `4-production.md` | `task/4-production` | Codex (`gpt-6.1-sol`, high) | merged in 0.0.8 | Gate re-run by the orchestrator: fmt, clippy, 113 tests, identical hash on repeat. Review change: the ramped price copied rule data on every call; replaced with `Rules::cost_of_ramped`. |
| 5 — gathering, income, commerce cap, idle workers | `5-gathering.md` | `task/5-gathering` | Codex (`gpt-6.1-sol`, high) | merged in 0.0.10 | Gate re-run by the orchestrator: fmt, clippy, all tests, identical hash on repeat. Follow-up: 10,000 units × 6,000 ticks now takes 15.7 s (was 2.0 s) — income and idle checks scan every entity each tick; still ~200× faster than real time, optimise before large matches. |
| 6 — map generation | `6-mapgen.md` | `task/6-mapgen` | Sonnet agent | merged in 0.0.10 | Gate re-run by the orchestrator: fmt, clippy, 13 tests incl. fairness over 100 seeds, identical hash on repeat; both sample maps inspected as images. Review fix: the image export overflowed on the highest land (drew it black); heights are now clamped. Note: the many-worlds style has no feature rows in the seed yet. |
| 7 — path search and local avoidance | `7-pathfinding.md` | `task/7-pathfinding` | Opus agent | merged in 0.0.11 | Gate re-run by the orchestrator: fmt, clippy, all tests; identical hash on repeat with 20 % obstacles; 10,000 units × 6,000 ticks in 5.4 s (22 s before on this machine). Income and idle logic are now incremental. Notes: a searched path starts one tick after the request; 25 milli-tile push limit and 8-repath limit are code constants. |
| 8 — show a generated map in the window | `8-terrain-view.md` | `task/8-terrain-view` | Sonnet agent | merged in 0.0.11 | Gate re-run by the orchestrator: fmt, clippy, all tests; fresh screenshot with another seed inspected; 57.4 fps at 1280×800 in a short run. Frame alpha is forced opaque. Interactive keys (`Home`, `R`) were not exercised by hand. |
| 9 — combat, stances and healing | `9-combat.md` | `task/9-combat` | Codex (`gpt-6.1-sol`, high) | merged in 0.0.14 | Gate re-run by the orchestrator: fmt, clippy, all tests, identical battle hash on repeat. **Known defect:** the symmetric 200-vs-200 battle ends 2 survivors against 106 on every seed, so one side is systematically favoured — next task. Also: the 10,000-unit run rose from 5.4 s to 9.3 s (target acquisition). |
| 10 — run the simulation in the app and draw its entities | `10-live-entities.md` | `task/10-live-entities` | Sonnet agent | merged in 0.0.14 | Gate re-run by the orchestrator: fmt, clippy, all tests; demo screenshot inspected; 59.0 fps with 2,039 entities at 1280×800, simulation holding 20 ticks/s. Not exercised: closing the window by hand, the overlay's new lines. |
| 11 — HUD chrome, live top bar and minimap | `11-hud-chrome.md` | `task/11-hud-chrome` | Sonnet agent, completed by Codex | review | CPU integration and two previews complete; fmt, Clippy, 257 tests and release build pass. Display runs explicitly deferred to orchestrator; FPS unverified. Uncommitted continuation; details in `app/docs/hud-validation.md`. |
| 12 — remove the side bias from combat | `12-battle-symmetry.md` | `task/12-battle-symmetry` | Opus agent | running | worktree `../rise-of-man-worktrees/12-battle-symmetry` |
| (next) races in the loader, map generation and setup | — | — | — | ready to write | `faction_alias` is not loaded yet; map generation puts every player on one home world; the many-worlds style has no feature rows |

States: `ready` → `running` → `review` → `merged`; or `blocked` (say why),
`handed over` (say to whom and from which commit).

## When an implementer stops mid-task

Usage limits, crashes and timeouts are treated the same way.

1. **Keep the work.** The task's branch and worktree stay as they are. Commit
   whatever exists as `wip: <task>` on the task branch and push the branch, so
   it survives the machine and the session.
2. **Record it here.** State `handed over`, the last commit, what the card
   still requires (from the "Done when" list), and why it stopped.
3. **Hand over, do not wait.** Give the same card plus the branch to the next
   implementer in this order: Sonnet agent → Opus agent → Codex (when its
   limit has reset). The new implementer reads the diff first and continues;
   it restarts from scratch only if the partial work fails review.
4. **Review is unchanged.** The release gate and the card's "Done when" list
   decide, whoever wrote the code.
5. **If the orchestrating session itself stops**, a new session reads
   `CLAUDE.md`, then this ledger, and continues with the first task that is
   not `merged`.

Worktrees live in `../rise-of-man-worktrees/<task>` (outside the repository,
on durable storage), never in a temporary folder.
