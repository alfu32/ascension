# Task 117 — The match ends when no human is left; an end dialog and a match report

Crates: `rom-sim` (end rule, per-player statistics, history), `rom-save` + `app/db/schema.sql`
(save tables for them; schema change authorised here), `rom-app` (end dialog, report screen,
network end), `rom-render` only if the dialog is drawn in the raylib HUD, `rom-net` (a finished
phase, the report in the state folder and in `/status`), `rom-cli` (`rom report <save>`), docs.
Implementer: Claude (Opus).

## Why — the owner (2026-11-21), verbatim

"The last player standing can carry on, ascend or leave..... i was thinkng about human players,
when threse only computers left there is no point in continuing. when the game is ended there
should be an in-game dialog modal saying that the match has ended and propose to exit or to view
game statistics and and of the match report with rankings of the players accros several aspects.
after which the game ends, in a network game the game should be finished .... maybe the server
should exit or reset ... idk"

What happened before (server match "fusion-1h-1c", 0.0.112): the computer held the owner's
capital from tick 173 269; the owner was defeated at 175 669 and the computer's conquest victory
was recorded, but nothing on screen said so (only the word "defeated" in the diplomacy tab); the
match went on to tick 227 151.

## The decision this changes

CLAUDE.md said "matches have no end condition" (task 76; `docs/ascension.md`, "The match
without an end"). New rule (owner, 2026-11-21):

- **The match ends when no human is left in it**: the setup had at least one human seat and now
  every human player is defeated. An ascended human is still in the match (it may descend); a
  human seat played by a computer after a disconnect is still a human seat. Matches that never
  had a human (benchmarks, `ai-match`, duels, fight matrix, playthrough) never end this way, so
  their results and hashes stay as they are.
- **When only humans are left** (every computer player defeated or ascended and one human team
  remains, `last_standing`), the match also reaches its end and the report is made, but the
  remaining humans may choose **Keep playing** (free play: the world goes on, the report is
  frozen at the end tick). This keeps the ascension route open to the winner.
- The first recorded victory stays as today; the report shows it.

The end is decided in the simulation (deterministic, integers, id order) at the tick it happens,
recorded in the state (`end_tick`, `end_reason`), saved and hashed, so every network peer agrees.

## Build

1. **End rule** in `rom-sim` (`victory.rs` / `ascension.rs`): `World::match_end()` →
   `Option<MatchEnd { tick, reason }>` with reasons `NoHumanLeft` and `LastStanding { team }`.
   "Human" comes from the player's controller in the setup, not from a name. After the end,
   orders from defeated players stay refused as today; nothing else changes in the tick.
2. **Per-player statistics** (integers, updated where the event happens, in id order), kept in
   the world, hashed and saved: resources gathered (per resource), resources spent, units
   trained, buildings completed, cities founded, cities captured and lost, kills and losses
   (already there), techs researched, the tick each age was reached, peak land tiles, peak army
   size, ascensions. Add only counters you can update at one clear point.
3. **History**: every `stats.history_period_ticks` (new `constant` row, 1200 = one game minute)
   one sample per player: score, land tiles, army size, city count, resource stock total, age.
   Score is a simple documented sum of weighted counters with weights as `constant` rows
   (`score.*`), not code literals.
4. **Save**: new save tables for the counters and the history (`player_stat`, `player_history`
   or columns where that is cleaner), the match end columns; the save format number goes up;
   older saves still load (counters start at zero, history empty). Lint views empty, seed loads.
5. **The end dialog** in the match window, modal, shown to every human when the end tick is
   reached: title "The match has ended", one line on why ("You were defeated by …", "No human
   player is left", "Your team is the last standing"), the winner if any, and buttons
   **Match report**, **Exit** (back to the launcher) and, only for a remaining human,
   **Keep playing**. The world behind it stops (single player: the sim stops ticking unless Keep
   playing). Same look as the in-match menu (`MenuPage` in `sim_thread.rs`, the menu's drawing).
6. **The match report**: a GPUI screen (other screens are GPUI) opened from the dialog and from
   the launcher's saved games, with
   - a ranking table, one row per player (name, race, human or computer, final standing: won,
     defeated at time, ascended, playing), columns per aspect: Overall score, Military (kills,
     losses), Economy (gathered, spent), Expansion (cities, peak land), Technology (techs, age
     reached and when), Ascension; each column ranked 1st, 2nd …, the leader marked;
   - a line chart per aspect over game time from the history (score, land, army, resources);
   - match facts: map style, seed, length in game time, end reason, first victory.
   Buttons: Back (to the dialog, or the launcher) and Exit.
7. **Network play**: the end is reached on the server's leader at the same tick as on every
   follower. Then the server
   - writes `report.json` (the same data as the report screen) beside `server.db` in its state
     folder (the management page archives the folder with the match);
   - goes to a `Finished` phase: `/status` shows `"phase": "finished"`, the end tick, reason and
     the ranking; it keeps serving `/status` and lets players reconnect to see the report;
   - stops ticking when no human is left; when humans are left it keeps ticking while any of
     them chose Keep playing and is connected;
   - **does not exit and does not reset itself**: the systemd service would restart it with
     `--resume` anyway, and an automatic reset would throw away the report before people read
     it. A new match is started from the management page (it archives the finished one).
   Clients get the end through the turns like any other state; the dialog is the same.
8. **`rom report <save-or-server.db>`** prints the ranking table and the facts as text (for tests
   and for the server owner).
9. **Docs**: `docs/ascension.md` "The match without an end" rewritten as "How a match ends";
   `docs/network.md` the finished phase and `report.json`; `docs/database.md` the new tables.

## Tests

- The end rule: a two-player match (human vs computer) where the computer takes the capital ends
  at defeat + 0 ticks with `NoHumanLeft`; human vs human with a computer: one human defeated →
  not ended; both → ended; all computers defeated → `LastStanding`; an ascended human keeps the
  match going; AI-only match never ends. Each scenario run twice, same hashes.
- Counters: gathering, training, building, capture, research each move the right counter.
- History: sampled exactly every period; saved and loaded unchanged (round trip).
- Old saves (one from `app/docs/` fixtures or a 0.0.113 save made in the test) load.
- Network: an in-process server match reaches the end on leader and follower at the same tick;
  `/status` reports `finished`; `report.json` is written.

## Done when

- Gate green from `app/`. The playthrough is AI-only, so its world is unchanged, but its hash
  covers the new counters: give the new `hash=` and `ascension hash=`, and show that
  `final_age`, `buildings`, `units`, `techs`, the age ticks and the ascension resources are
  identical to `2b4a0812188cd904` / `a252236eec91a838` (the run's other numbers). Fight matrix
  `strengths=255 unanswered=0`; duel 1..20 unchanged (20–0, mean tick 54 449).
- A self-ending app run (script: a human with no army and a computer on easiest that takes the
  capital, or a scripted defeat) that shows the end dialog, then the report screen; screenshots
  you looked at: `app/docs/img/117-end-dialog.png`, `app/docs/img/117-report.png`.
- Report: files changed, gate output, the new hashes with the comparison, what you could not
  verify.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/ascension.md`, `app/docs/network.md`,
`app/docs/database.md`, `app/db/schema.sql` (player, match, victory tables),
`rom-sim/src/victory.rs`, `ascension.rs` (`last_standing`), `capture.rs`, `world.rs` (stats),
`hash.rs`, `rom-save/src/{read,write}.rs`, `rom-app/src/sim_thread.rs` (`MenuPage`, the run
loop), `rom-app/src/ascension_view.rs`, `rom-app/src/launcher/` (saved games list),
`rom-net/src/server/hub.rs` (`Phase`, status), `rom-match/src/core.rs`.

Memory rules: `CARGO_BUILD_JOBS=4`; check `free -g`; app runs self-ending under
`systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0 env -u WAYLAND_DISPLAY …`.
Never `xdotool`, `wmctrl` or `sudo`. Do not stop processes you did not start (the owner may be
playing). Logs only in the scratchpad's `t117/`; never write `scratchpad/gate*.sh`; no cargo
target dir in the scratchpad or /tmp. No names of other games or studios; say "route" for
star-lane routes. Work in `../rise-of-man-worktrees/117-match-end` on branch
`task/117-match-end`; do not merge, tag or push.
