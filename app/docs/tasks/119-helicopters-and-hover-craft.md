# Task 119 — Helicopters (modern, information), superseded by hovering craft

Issue #3 (owner, 2026-11-21), part 4. Files: `app/db/seed.sql` (or the generated content in
`app/db/content/` if units are generated there — follow `database.md`), new model files in
`app/db/models/`, `rom-sim` only if aircraft cannot hover today, `rom-ai` only through data,
docs. Implementer: Claude (Sonnet). Starts after task 117 is merged (seed rows).

## Why — the owner, verbatim (issue #3)

"I also want helicopters from the modern age through information age and they will be superseded
by, well, by quadcopters or maybe, well, something else that hovers. And they should be in the
same class actually"

## Today (0.0.113)

Aircraft (movement class `aircraft`): biplane (industrial), fighter and bomber (modern), jet
fighter, strategic bomber, repair drone (information), drone fighter and drone bomber
(automation), interceptor (fusion), orbital fighter and bomber (starfaring). No helicopter.
Built at `bld.airfield`.

## Build

1. **One line of rotorcraft in one class** (`ui_group`/class rows as the other lines; the later
   ones replace the earlier with `replaces_id`, as tanks do):
   - modern: `unit.attack_helicopter` — anti-ground (infantry and light vehicles), short range,
     slow for an aircraft, can hold a point (hover) instead of making passes;
   - information: `unit.gunship_helicopter` — replaces it: missiles that also hurt tanks;
   - automation: `unit.hover_gunship` ("something that hovers": a ducted-fan quadrotor craft)
     — replaces the gunship;
   - fusion or later if the line needs it to stay useful: an upgrade by modifier rather than a
     new unit.
   Built at the airfield (or a helipad if the airfield cannot serve hovering craft; prefer the
   airfield). Costs, hp, speeds and weapons in line with the neighbouring units of each age.
2. **Hover behaviour.** If aircraft in `rom-sim` must keep moving or return to base, give the
   rotorcraft a data flag (component column) to hold position like ground units while still
   flying (hit only by weapons that hit aircraft). If that needs simulation code, keep it small,
   data-driven and deterministic, with a twice-run hash test.
3. **Counter rule** (card 71, `docs/computer-player.md`): the rotorcraft must be answered by
   anti-air of the same age or one upgrade away (flak gun, missile team, …) and by fighters;
   add them to the fight matrix and make it pass (`strengths=255 unanswered=0`).
4. **Models**: one model file per unit in `db/models/` (`app/docs/models-in-meshcraft.md`): a
   helicopter with a main rotor and tail rotor, a heavier gunship with stub wings, a quadrotor
   with four ducted fans; spinning rotors if the kit has a spinner part (look at the other
   aircraft). They go to the owner's model review (model review folder, `HANDOVER.md`).
5. **Computer players** use them through the existing army-composition data.
6. **Docs**: the unit lists in `research/03-timeline.md` / `docs/` where units per age are
   listed.

## Done when

- Gate green; database lint views empty; the new units producible in their ages; fight matrix
  passes; duel 20–0; playthrough: report the new hash and that only the expected unit counts
  changed.
- Screenshot you looked at: `app/docs/img/119-rotorcraft.png` (the three units on a planet,
  gallery or a scripted scene).

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/database.md` (how to add a unit),
`app/docs/models-in-meshcraft.md`, `app/docs/computer-player.md` (counter rule, fight matrix),
the seed rows of `unit.fighter`, `unit.gun_drone`, `unit.drone_fighter`, `bld.airfield`, the
aircraft movement code in `rom-sim`.

Memory rules, logs and worktree as in card 117 (`t119/`, `../rise-of-man-worktrees/119-rotorcraft`,
branch `task/119-rotorcraft`). Do not merge, tag or push.
