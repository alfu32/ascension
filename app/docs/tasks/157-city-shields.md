# Task 157 — City and fort shields by research: missile shield, laser shield, energy shield

Owner (chat, 2026-11-25), verbatim: "cities and forts(their upgrades in their line) should have
their own shield gained through research, in the freedom age they can research missile shield,
the upgrade to laser shield and energy shield."

Orchestrator's reading (owner to correct before this starts): three researches, each the
upgrade of the one before, give every own building of the **city line** and the **fort line**
(all their upgrades: fort, star fort, bastion, citadel, …; found by data, the lines' tags or
upgrade chains, never by key) a shield of its own:

| Research | Age | What the shield does |
| --- | --- | --- |
| **Missile Shield** | Freedom (age 7, `age.information`) | intercepts a share of the missiles, rockets and shells (projectile weapons of explosive damage) aimed at the building: the projectile is destroyed in flight with a small flash; nothing else changes |
| **Laser Shield** (upgrade) | Automation (age 8) | point-defence lasers: a larger share, and bullets and orbital bombardment shells too |
| **Energy Shield** (upgrade) | Fusion (age 9) | a shield pool (a share of the building's hit points) that takes every kind of damage first and recharges while not hit, drawing energy (`res.energy`); a dome over the building, flaring where it is hit; interception as the laser shield stays |

**Coverage** (owner, chat, 2026-11-25, with task 158): a city's shield covers its **city area**
(its economic radius, which keeps growing with the city's buildings even when the setup option
of task 158 lets buildings stand outside it): projectiles aimed at anything own inside the area
are intercepted, and the Energy Shield's dome spans the area with one pool for the city. A fort
(and its upgrades) shields itself and a small radius around it (say it).

The owner (chat, 2026-11-25): "make the shields expensive to research". The three researches
are among the dearest of their ages: each costs at least as much as that age's advance itself
(the age research), in the age's expensive resources (metal, knowledge, and energy for the
Energy Shield), and takes long to research; say the numbers beside the age advance's.

Numbers (shares, pool, recharge, energy per period, research costs and times): say them, tuned
so a city still falls to a proper siege of its age (the counter rule, card 71: the fight
matrix keeps `unanswered=0`, and bombardment of task 75 still flattens a city, slower).

Crates: seed and `db/content/generate.py` (the three researches, their effects as data:
modifier rows and, if needed, a shield component: schema change authorised), `rom-sim`
(interception of projectiles in flight, the shield pool before hit points, recharge, energy
upkeep, saved and hashed), `rom-render` (interception flashes, the energy dome and its flare;
nothing per frame that allocates), sounds (interception zap, dome hum and hit;
`app/tools/sounds/README.md`, "Baking only new types"), `rom-ai` (researches them by its usual
scores; nothing special unless a duel shows a need). Implementer: Claude (Opus). Starts after
154 (rom-sim) and 145 (rom-render) merge.

Since this card was written (2026-11-25): task 158 has merged (the setup option `city_area`;
the city area still grows with it off; read `World::footprint_in_city_radius` and the economic
radius code for coverage), and task 155 has merged (age names: Age of Discovery, Spacefaring,
Starfaring; keys unchanged). The owner, 2026-11-25: "please dont struggle to maintain
compatibility too much": a save-format bump may refuse older saves; write no compatibility code.

## Done when (every card)

- Gate green from `app/`: fmt, clippy `-D warnings`, workspace tests (≥ the count on main when you
  start, plus this card's tests: interception by each shield, the pool absorbs and recharges,
  no energy no recharge, every upgrade of the city and fort lines gets it and nothing else
  does; each scenario run twice with equal hashes); fight matrix `unanswered=0`; duel 20–0;
  database loads with lint views empty; playthrough and ascension hashes and rules fingerprint
  reported with why they moved.
- Screenshots you have Read, saved as `app/docs/img/157-*.png`, through the game's own paths: a
  city intercepting rockets, and a fort's energy dome flaring under fire.
- Report: files changed, choices where the card left one, numbers, what you could not verify.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/database.md` (modifiers, techs, damage classes,
projectiles), `app/docs/content.md`, `app/crates/rom-sim/src/combat.rs` (projectiles, task 102),
`app/docs/tasks/75-*.md` (bombardment), `app/docs/computer-player.md` (counter rule).

Memory rules: `CARGO_BUILD_JOBS=4`; check `free -g`; big runs under
`systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0`. Never `xdotool`, `wmctrl` or
`sudo`. Do not stop processes you did not start (the owner may be playing). Logs only in the
scratchpad's `t157/`; never write `scratchpad/gate*.sh`; no cargo target dir in the scratchpad or
/tmp. Work in `../rise-of-man-worktrees/157-city-shields` on branch `task/157-city-shields`;
commit there (stage by name); do not merge, tag or push.
