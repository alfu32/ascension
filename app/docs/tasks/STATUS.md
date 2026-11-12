# Task ledger

The single source of truth for what is assigned, running and done. Any
session or implementer can resume from this file and the task cards beside
it; nothing needed to continue lives only in a chat or a temporary folder.

| Task | Card | Branch | Implementer | State | Notes |
| --- | --- | --- | --- | --- | --- |
| 0 — workspace skeleton and rules-database build | `0-skeleton.md` | `task/0-skeleton` | Codex (`gpt-6-luna`, started before the model was chosen) | merged in 0.0.3 | Gate re-run by the orchestrator: fmt, clippy, 5 tests, `rules.db: 100 definitions, 84 tables`. Codex could not commit from the worktree; the orchestrator committed. |
| 1 — stack proof: raylib world view inside GPUI, measured | `1-stack-proof.md` | `task/1-stack-proof` | Opus agent | running | worktree `../rise-of-man-worktrees/1-stack-proof` |
| 2 — typed rules loader, validation, modifier engine | `2-rules-loader.md` | `task/2-rules-loader` | Codex (`gpt-6.1-sol`, high) | running | worktree `../rise-of-man-worktrees/2-rules-loader`; orchestrator commits. First run stopped correctly on a schema conflict (`game_constant.value` was `REAL`); fixed in 0.0.5 and restarted. |

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
