# Task 113 — Orders take effect quickly in a network match

Crates: `rom-app` (`src/net/play.rs`, the follower branch of `src/sim_thread.rs`), `rom-net`
(only the server's tick wake-up in `src/server/hub.rs` if step 3 needs it), docs
(`app/docs/network.md`, "Timing"). Implementer: Claude (Opus).

## Why — the owner (2026-11-20), verbatim

"with the latest additions ( networking ) the game feels sluggish from the beggining ... is it the
synchronisation routine .... the way it is designed it seems that its gonna be as slow as the
slowest cluient that is still connected .... anyway this session is run entirely on this machine
(client and server )"

## Today (0.0.108)

- The server's leader steps on its own clock and does not wait for clients
  (`ServerConfig::wait_for_clients` is false; strict lockstep is used only by tests). A slow
  client falls behind, catches up, or reloads a snapshot; it does not slow the others.
- In a network match every person's game, the host's included, is a follower: a click goes to
  the server, waits for the server's next tick (0–50 ms at 100 %), comes back in a `Turn`, and is
  applied when the follower steps that turn.
- The follower (`NetPlay::due`) keeps `KEEP = 2` turns in hand (100 ms) and catches up only when
  it holds more than `paced + KEEP + 2` spare turns, so it can sit up to about 7 turns (350 ms)
  behind the server indefinitely: after a start, a snapshot or any hitch (a planet surface made,
  an autosave) the backlog settles anywhere in that range and stays there.
- When `FixedStep::advance` says a tick is due and no spare turn is held, the due tick is
  consumed and lost: the turn that arrives a moment later waits for the follower's next clock
  tick (up to 50 ms more).
- So an order takes roughly 150–400 ms to take effect on one machine, where a single-player
  match (no server) takes it at the next tick. That is the sluggishness, not the network.

## Build

1. **Measure first.** A headless test or bench (in `rom-app/src/net/tests.rs` or a `rom-net`
   test, the server on 127.0.0.1 in-process, no window) that reports, at 100 % speed over at least
   600 ticks: the turns the follower holds (min, mean, max) and the delay from an order submitted
   on the follower to the tick it is applied on the follower (ms, mean and max). Record the
   before numbers in the report.
2. **A small, adaptive buffer.** The turns kept in hand follow the measured jitter of turn
   arrival (e.g. the spread of arrival intervals over the last few seconds), from 0 on a quiet
   loopback or LAN up to a cap of 3 turns on a jittery link. More turns held than that: apply one
   extra turn per tick (or a few) until back to the target, rather than tolerating a backlog;
   far behind keeps today's fast catch-up and snapshot rules. No due tick is lost: when a tick is
   due and no turn is held, the follower applies the next turn as soon as it arrives (within the
   catch-up limit), so arrival, not the follower's own clock phase, sets when it steps.
3. **The server applies an order at its next tick, not later.** Check that an input arriving
   just after a tick is led at the next tick (the hub wakes on the event and on the tick time). If
   it is already so, change nothing in `rom-net` and say so.
4. **Immediate acknowledgement.** The person sees that a click was taken at once (the existing
   order marker or the selection's feedback shown when the order leaves, not when it returns), so
   the round trip does not feel like a missed click. Only presentation; the order still goes
   through the server.
5. **The HUD line.** The network line shows the order delay next to the latency
   ("Online · 3 ms · orders 60 ms"), from the same measurement, so the owner can see it.
6. **Docs.** `network.md` "Timing": the new buffer rule and the expected delay on loopback, LAN
   and a 50 ms internet link; say plainly that the server does not wait for the slowest client,
   and what a slow client experiences.

Determinism is untouched: the follower still applies the leader's turns in order and steps;
only when it applies them changes. Followers must keep producing the leader's hashes.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/network.md` (all, especially "Who does what",
"Timing", "As built"), cards 85, 86 and 88, `rom-app/src/net/play.rs`, `rom-app/src/net/tests.rs`,
the `run` loop and `FixedStep` in `rom-app/src/sim_thread.rs`, `rom-net/src/server/hub.rs`
(`run`, `next_wake`, `step`, `lead`), `rom-net/src/transport.rs` (`POLL`, nodelay),
`rom-match/src/core.rs`.

## Done when

- Gate green from `app/`; playthrough hashes unchanged; fight matrix `strengths=255
  unanswered=0`.
- The measurement from step 1, before and after, in the report. On loopback at 100 % speed the
  mean order delay is at most about 1.5 ticks plus the server's wait (≈ 100 ms) and the held turns
  stay at the target; with injected jitter (a test-only delay on turn delivery) the follower stays
  smooth and does not drift backwards.
- Tests: the follower stays in step (hash checks pass) with the new pacing; held turns return to
  the target after a burst; no due tick is dropped when a turn arrives late; the adaptive target
  rises with jitter and falls back when it calms.
- One self-ending run of the app hosting a match on this machine (server in-process, a scripted
  order), with the HUD line visible in a screenshot you looked at (`app/docs/img/113-net-line.png`).

Memory rules: `CARGO_BUILD_JOBS=4`; check `free -g` and `df -h /tmp`; run the app one instance at a
time in self-ending modes under `systemd-run --user --scope -q -p MemoryMax=6G -p
MemorySwapMax=0 env -u WAYLAND_DISPLAY …`. Never `xdotool`, `wmctrl` or `sudo`; send no input to
windows. Do not stop processes you did not start (the owner may be playing). Wait loops must not
use a `pgrep -f` pattern that matches the loop itself. Logs only in `scratchpad/t113/`; never write
`scratchpad/gate*.sh`; never put a cargo target dir or a source copy in the scratchpad.

Commit on the branch (stage by name); do not push or merge. Final message: commands with real
output, the before/after numbers, files changed, deviations, anything not verified.
