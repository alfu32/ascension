# Task 155 — Spacefaring before Starfaring: the own star system first

Owner (chat, 2026-11-25), verbatim: "spaceports should also build space probes earlyer so that
we can explore and colonize the solar system, frigates should also be maybe interesting to have
earlier than starfaring age ... which honestly comes too soon. i'd rename it spacefaring age and
the expansion age i'd make into starfaring age."

Orchestrator's reading (owner to correct before this starts; questions were not answered):
the space ages split by distance. The own star system is explored and settled first; travel to
other stars comes one age later.

Crates: seed and `db/content/generate.py` (ages, names, first ages, research), `rom-sim` only if
the jump gate needs a rule that data cannot express (stop and report first), `rom-ai` only if
its space manager depends on the old ages (by data it should follow), docs, the site's story
text (`site/`), and the music docs' age names (`app/docs/music.md`; track **files and published
titles stay** unless the owner says otherwise). Implementer: Claude (Opus).

Also (owner, issue #160, 2026-11-25), verbatim: "Please rename the gunpowder age to the age of
discovery". `age.gunpowder` is shown as **Age of Discovery** (key unchanged), everywhere a
player reads an age name, as item 1 below.

## What changes

1. **Names.** `age.starfaring` is shown as **Spacefaring Age**; `age.expansion` as
   **Starfaring Age**. Keys stay (saves, server, music keys unchanged). Every player-facing text
   that names these ages follows: the age dialog, help, the launcher, `docs/ages-story.md`, the
   site's chapters, the in-game music titles where they show an age name.
2. **Fusion age: the own system.** The spaceport is buildable from the **Fusion** age (age 9);
   it builds the **space probe** (explore the own system's bodies), the **transport ship**
   (carry colonists to a planet of the own system and found a colony there, as task 54 does)
   and the **space frigate** from Fusion. Their research gates move with them. Satellites stay
   where task 130 put them.
3. **Spacefaring (age 10).** Corvettes, the rest of the fleet, claims on bodies of the own
   system and its neighbours as today, except jumps.
4. **Starfaring (age 11, the old Expansion): jumps.** Travel to another star (charged jumps
   along routes, task 70/111) needs a research of the Starfaring age (say its name, cost and
   place); the survey ship, the jump inhibitor and interstellar trade come with it or after.
   The jump detector (task 118) stays where it is (it warns of foreign ships).
5. **Balance.** Research costs and age-advance times may be adjusted so the Spacefaring age
   arrives no sooner than today (the owner: "comes too soon"); say what changed with numbers.
6. **The computer player** follows by data (it builds what its age unlocks); if a duel or a
   space test of rom-ai depended on the old ages, fix it in rom-ai and say so.

## Done when (every card)

- Gate green from `app/`: fmt, clippy `-D warnings`, workspace tests (≥ the count on main when you
  start, plus this card's tests: a Fusion-age player builds a spaceport, a probe and a transport
  and founds a colony in its own system; a jump is refused before the Starfaring research and
  allowed after; each scenario run twice with equal hashes); fight matrix `unanswered=0`; duel
  20–0; database loads with lint views empty; playthrough and ascension hashes and the age
  ticks of the playthrough reported (before/after) with why they moved.
- Screenshots you have Read, saved as `app/docs/img/155-*.png`, through the game's own paths: the
  age dialog naming the Spacefaring Age, and a Fusion-age spaceport with a probe in the system
  view.
- Report: files changed, choices where the card left one, numbers, what you could not verify.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/database.md`, `app/docs/content.md`,
`app/docs/world-model.md` (space, routes and jumps, colonies), `app/docs/ages-story.md`,
`app/docs/tasks/130-*.md`, `app/db/content/generate.py`, `research/03-timeline.md`.

Memory rules: `CARGO_BUILD_JOBS=4`; check `free -g`; big runs under
`systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0`. Never `xdotool`, `wmctrl` or
`sudo`. Do not stop processes you did not start (the owner may be playing). Logs only in the
scratchpad's `t155/`; never write `scratchpad/gate*.sh`; no cargo target dir in the scratchpad or
/tmp. Work in `../rise-of-man-worktrees/155-space-ages` on branch `task/155-space-ages`; commit
there (stage by name); do not merge, tag or push.
