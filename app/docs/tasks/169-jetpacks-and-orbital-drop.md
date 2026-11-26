# Task 169 — Jetpacks from the Spacefaring age, orbital drop from the Starfaring age

GitHub issue #172.

Owner (chat, 2026-11-25), verbatim: "infantry units can swim from age 9, fly short distances
from age 10 ( research jetpacks) and be deployed from orbit directly from age 11 ( research
orbital drop )."

Ages are counted from 0:
- **jetpacks** from age 10, the Spacefaring age (key `age.starfaring`);
- **orbital drop** from age 11, the Starfaring age (key `age.expansion`).

Swimming is task 168.

## What changes

1. **Jetpacks** (research, from the Spacefaring age). Every unit tagged `infantry` gains a
   **Jump** ability:
   - **The jump:** a short flight to a target point within about 6 tiles. It crosses walls,
     water, cliffs and units, and lands on a free land tile.
   - **Cooldown:** about 20 seconds per unit.
   - **Vulnerability:** in flight the unit is in the air layer, so anti-air can hit it.
   - **Fit:** use the existing flight layer (`entity_flight`, the `flies` movement layer) and an
     ability or order row; no key branches.
   - **The order:** a new order kind if none fits, with a hotkey in the order panel, like the
     other unit commands (task 142).
   - **Walls:** fortifications (task 147) do not stop a jump. Shield fields (task 156) do,
     because the field blocks the air above the line too. Say so in the docs.
     Task 156 (merged in 0.150.0) blocks only walkers at standing field tiles; flyers pass
     (`rom-sim/src/fields.rs`). A jump must therefore check its flight path against the
     standing field stretches and be refused when it would cross one. Aircraft are unchanged.
2. **Orbital drop** (research, from the Starfaring age). Infantry aboard a starship in orbit
   above a planet can be dropped directly onto the surface:
   - **Order:** "Drop" on the carrying ship.
   - **Target:** any visible land point on the planet below, within about 30 tiles of the
     point under the ship.
   - **Who:** units with the `infantry` tag, from every ship whose container accepts them (the
     space transport, the frigate and cruiser with passenger room).
   - **Landing:** the units arrive after a short fall (a few seconds, deterministic), and can be
     shot at while falling by planetary defences and anti-air.
   - **Without the research** the ship must land first, as today (task 72).
3. **Computer players.**
   - **Jetpacks:** they use them to cross walls and gaps when attacking.
   - **Orbital drop:** they use it to land an attack on an enemy planet once they hold orbit
     there. Show both in an `ai-match`.
4. **The counter rule** (card 71) and the fight matrix stay at `unanswered=0`. Jumping
   infantry are answered by anti-air and by area weapons.

## Crates

Seed (the two researches, the ability or order rows), `rom-rules`, `rom-sim` (jump flight,
cooldown, drop from orbit), `rom-render` (the jump arc, the falling capsules: a simple shape,
no new model unless needed), `rom-app` (orders, hotkeys, targeting), `rom-ai`.

## Read first

- `AGENTS.md`, `CLAUDE.md`;
- `docs/world-model.md` (orbit, landing, task 72);
- `docs/database.md` (flight, containers, orders);
- `docs/tasks/142-unit-commands.md`;
- `crates/rom-sim/src/` (flight, orbital transport landing);
- `crates/rom-ai/src/`.

## Done when

- Gate green from `app/`:
  - fmt, clippy `-D warnings`, workspace tests (≥ main plus this card's tests);
  - fight matrix `unanswered=0`;
  - duel 20–0;
  - the database loads with empty lint views.
- Playthrough and ascension hashes unchanged, or explained.
- **Tests:**
  - jump refused before the research and when cooling down;
  - a jump crosses a wall but not a shield field;
  - anti-air hits a unit in flight;
  - drop refused without the research or when not in orbit;
  - the dropped units land inside the range;
  - each scenario run twice with equal hashes.
- **Screenshots** `docs/img/169-*.png`, each opened and checked:
  - infantry mid-jump over a stone wall;
  - an orbital drop seen from the planet.
- Report as AGENTS.md says. Logs only in the scratchpad's `t169/`; never write `gate*.sh`.
