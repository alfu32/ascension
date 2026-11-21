# Task 118 — Alerts: a city under attack, a ship jumping into your system; inhibitors later

Issue #3 (owner, 2026-11-21), parts 1–3. Crates: `rom-sim` (alert events), `rom-app` (notices,
minimap marker, camera jump), `rom-render` (marker drawing), `app/db/seed.sql` + one model file
for the new building, docs. Implementer: Claude (Opus). Starts after task 117 is merged (both
touch `rom-sim` and `rom-app`).

## Why — the owner, verbatim (issue #3)

"We need another type of building which is a jump detector or orbital detectors that will send you
a notification when somebody jumps into your system. And that should be available around the
fusion age, while the jump inhibitors should be available only later in expansion age because it
doesn't make sense to have them before you can even travel in hyperspace. You don't, in theory,
you don't even know that that exists

Or it could very well be an upgrade to the survey ship. And the upgrade you, you put the survey
ship somewhere in your solar system. And it could have an action radius by which it detects
foreign ships landing in your system

Also, when one of your cities gets attacked, I don't know, since the second or third age, no, the
second, no, even the first age, you will be notified that you are under attack. And it will say
which city is under attack. And maybe even put a signal on a marker on the mini-map"

## Today (0.0.113)

- `bld.jump_inhibitor` and the first starships (`unit.survey_ship`, corvette …) both arrive in
  the starfaring age. Nothing tells a player that a foreign ship arrived in its system unless it
  sees it. `space_view::Notices` words space events for player one (held orders, arrival
  strikes, reach); there is no alert when a city or unit is attacked.

## Build

1. **Under attack, from the first age.** When an enemy damages a player's building or unit, the
   player gets an alert naming the place: "Rome is under attack" (the city whose territory holds
   the victim; "Your army is under attack near Rome" for units in the field; the planet's name
   off the home world). One alert per city (or per area for field units) per
   `alert.attack_repeat_ticks` (constant row, 600 = 30 s) so a battle is one line, not a stream.
   A short sound cue if the HUD has a sound path (otherwise none). The alert appears in the
   notice area with the existing notice look; clicking it (or a key, `Space` if free — check
   the key map) moves the camera there. A **pulsing marker on the minimap** at the place for
   `alert.marker_ticks` (constant, 200 = 10 s), and on the star map at the system when the
   attack is off the planet being viewed. Allies are not alerted about each other.
2. **Jump detector building** `bld.jump_detector` ("Deep Space Tracking Station"), available in
   the **fusion age**, built on a planet's surface like the inhibitor (copy its placement
   rules; one per planet is enough — use the existing limit columns). While it stands, any
   non-allied ship arriving by jump in its star system raises an alert: "Foreign ships entered
   the Sol system: 3 ships of the Carthaginians" (owner and count as the detector sees them;
   the ships themselves are revealed on the star map for `alert.marker_ticks` at their arrival
   point), with the marker on the star map and the system view. Several detectors do not
   duplicate alerts. Behaviour comes from a new component row (e.g. `et_detector` with
   `scope = 'system'`), not from the key. Model file: one building in `db/models/` in the style
   of the inhibitor (`app/docs/models-in-meshcraft.md`); it goes to the owner's model review
   like every new model (`HANDOVER.md` in the model review folder).
3. **Survey ship sensor upgrade** (the owner's alternative; both are built unless the owner says
   otherwise): a research at the spaceport in the starfaring age, "Survey Sensor Array", after
   which a survey ship given a **Watch** stance (or simply standing still in a system) detects
   foreign jump arrivals within `et_detector.radius` tiles of it (data; start with 40 tiles) and
   raises the same alert. The same component, granted by the tech as a modifier/ability row.
4. **Inhibitors move to the expansion age**: `bld.jump_inhibitor` `first_age` → `age.expansion`
   (and anything that requires it: techs, AI build lists, docs). Check `lint_unproducible` and
   the computer player's space build order; the duel and fight matrix must still pass.
5. Computer players: they may ignore alerts (they already see the state), but they build the
   detector in the fusion age with a low priority (one per home system), through their normal
   build data, not code keyed on names.
6. **Docs**: `world-model.md` (inhibitors in the expansion age; detectors), `hud.md` (alerts and
   the minimap marker), `database.md` (the new component table).

## Determinism

The alert events are computed by the simulation from state (damage dealt, arrivals) and are
deterministic, but they are presentation: they may live in the snapshot only and need not be
hashed. Detection itself does not change the game (no fog change except the brief reveal of
arriving ships, which is a vision rule and must be deterministic and saved if it is state).

## Tests

- An attack on a city raises one alert for its owner, none for the attacker, and no second one
  inside the repeat window; a later attack after the window raises another.
- A foreign ship jumping into a system with a detector raises the alert; an allied one does not;
  without a detector, nothing; a survey ship with the upgrade detects within its radius only.
- The inhibitor is not buildable before the expansion age; the detector is from fusion.
- Two runs of a scenario give the same hashes.

## Done when

- Gate green from `app/`; playthrough, fight matrix and duel: hashes may change because the
  inhibitor's age and the AI build list changed — report the new hashes, and show that the
  playthrough's ages, buildings, units and techs move only by the inhibitor/detector, the fight
  matrix stays `strengths=255 unanswered=0`, and the duel stays 20–0.
- Screenshots you looked at: `app/docs/img/118-under-attack.png` (alert + minimap marker),
  `app/docs/img/118-jump-detected.png` (alert + star map marker), from self-ending scripted
  runs.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/world-model.md` (space, jumps, inhibitors),
`app/docs/hud.md`, `app/docs/database.md`, `app/db/schema.sql` (`et_inhibitor`, entity types),
seed rows for `bld.jump_inhibitor`, `rom-sim` (inhibitor and arrival code, combat damage),
`rom-app/src/space_view.rs` (`Notices`), the minimap drawing in `rom-render`, cards 59, 70, 104,
111.

Memory rules, logs and worktree as in card 117 (`t118/`, `../rise-of-man-worktrees/118-alerts`,
branch `task/118-alerts`). Do not merge, tag or push.
