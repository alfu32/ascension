# Task 163 — Jumps between clusters from the Age of Abundance; fair home clusters

Owner (chat, 2026-11-26), verbatim, answering the proposal for task 162:
- "The alternative is to tie jumps between clusters to a later age is a good idea"
- "Cluster size: I'd use 3 to 7 systems but mind that it could impact planet choices for 8
  players game ( even though we can resolve that with more habitable planets per star system
  when te number of planets in a cluster is low )"

Runs after task 162 (star clusters in the data, `star_system.cluster_id`, the five zoom levels).
Start from main after 162 has merged.

## What changes

1. **A new map-scale tier.**
   - **The tier.** Add `map_scale` row `galactic` ("Galactic", rank 3). `age.abundance` and
     `age.galactic` move to it. Today `age.expansion` (Starfaring, age 12 counted from 1) is
     `interstellar`, rank 2, and opens jumps.
   - **The rule.** Reaching rank 3 is what allows a jump along a route whose two systems are in
     different clusters. A jump within a cluster still needs rank 2, as now. The rule reads the
     rank, never an age key.
   - **Where it is enforced.** The simulation refuses such an order with a typed error (a new
     `SimError` variant), the same way it refuses a jump before rank 2. Check every way into a
     jump: orders, queued routes, transports and survey ships.
   - **Pathing.** A multi-hop route plan stops at the last system the player may reach.
2. **What a player sees follows the tier.**
   - interplanetary: the own system;
   - interstellar: the cluster view, up to the own cluster and its route ends;
   - galactic: the quadrant and galaxy views.

   Use the existing per-tier camera limits (`map_scale.max_zoom_out` and the zoom cap in
   `rom-render/src/zoom.rs` / `rom-app/src/space_view.rs`). Describe the result in
   `world-model.md`.
3. **Fair home clusters** (`rom-mapgen`). With jumps between clusters closed until age 13, a
   player's cluster is everything it can expand into for most of the space ages. For every
   cluster holding at least one home:
   - **Size.** Homes are placed only in clusters of at least 3 systems. If the region cannot
     supply that, the layout attempt fails and the next seeded attempt runs, as other layout
     checks do now.
   - **Share.** The cluster must hold, per home in it, at least the style's per-player share of
     habitable planets (besides the homes) and resource bodies. When the cluster is short, add
     habitable planets and resource bodies to its systems, smallest system first. These are
     generated before play like every other body, are deterministic, and may exceed the style's
     per-system maximum. This is the owner's "more habitable planets per star system when the
     number of planets in a cluster is low".
   - **Check.** Clusters with homes then differ by at most the existing fairness tolerance
     (`fairness`, `home_shares`); extend those checks to cover the cluster.
   - **Proof.** Show it holds for 2, 4 and 8 players in every universe map style. Run
     `rom` universe checks over many seeds and report the failures, which should be none, and
     the attempts used.
4. **Computer players** (`rom-ai`): the navy, expansion and transport planners consider only
   reachable systems: their own cluster before rank 3, and every cluster after it. They must
   not issue orders that the simulation refuses. After rank 3 they expand and attack across
   clusters. Show it in an `ai-match` on a universe style that runs into the Abundance age.
5. **The HUD and the age dialog.**
   - The Abundance age dialog and its story mention jumps to other clusters.
   - The Starfaring hint says jumps reach the stars of your own cluster.
   - A refused jump shows its reason ("Other clusters open in the Age of Abundance").
   - Data-driven text, no key branches.

## Files

`app/db/schema.sql` (only if the tier needs no new column, otherwise stop and report),
`app/db/seed.sql` (via the generator if the age rows are generated), `rom-rules`, `rom-sim`
(`space.rs`, jump validation), `rom-mapgen` (homes, top-up, fairness), `rom-ai` (`fleet.rs`,
navy, expansion), `rom-app` (view cap and hint text), `rom-render` (the zoom cap, if it lives
there), and docs: `world-model.md`, `database.md` (map scales).

Implementer: Claude (Opus).

## Done when (every card)

- **Gate green** from `app/`:
  - fmt;
  - clippy `-D warnings`;
  - workspace tests: at least the count on main when you start, plus this card's tests;
  - fight matrix `unanswered=0`;
  - duel 20–0;
  - the database loads with empty lint views;
  - the playthrough and ascension hashes, either unchanged or explained (the playthrough
    reaches the galactic age, so they may change).
- **Tests:**
  - a jump between clusters is refused at rank 2 and allowed at rank 3, run twice with equal
    hashes;
  - a jump within a cluster is allowed at rank 2;
  - a route plan stops at the cluster edge;
  - fair home clusters for 2, 4 and 8 players over seeds 1..50 in each universe style;
  - the AI issues no refused cross-cluster jump in an `ai-match` before Abundance.
- **Screenshots** you have Read, saved as `app/docs/img/163-*.png`:
  - the refused jump message;
  - an 8-player universe's galaxy view with the homes marked;
  - the Abundance dialog.
- **Report:**
  - files changed;
  - the top-up numbers (how often, how many bodies);
  - the tier and camera choices;
  - what you could not verify.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/world-model.md` (tiers, routes and jumps,
quadrants, fair homes), `app/docs/database.md` (map_scale, map styles), the task 162 card and
its merged code, `app/crates/rom-sim/src/space.rs`, `app/crates/rom-mapgen/src/universe.rs`,
`app/crates/rom-ai/src/fleet.rs`, `app/crates/rom-app/src/space_view.rs`.

Memory rules: `CARGO_BUILD_JOBS=4`; check `free -g`; big runs under
`systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0`. Never use `xdotool`,
`wmctrl` or `sudo`. Do not stop processes you did not start (the owner may be playing). Logs go
only in the scratchpad's `t163/`. Never write `scratchpad/gate*.sh`. No cargo target dir in the
scratchpad or /tmp. Work in `../rise-of-man-worktrees/163-cluster-jumps` on branch
`task/163-cluster-jumps`. Commit there (stage by name); do not merge, tag or push.
