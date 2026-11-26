# Task 162 — Five zoom levels: planet, system, cluster, quadrant, galaxy

Owner (chat, 2026-11-26), verbatim:
- "the measure is given by the distance that it is given to a star system in star system view
  .... maybe it suffices to render those coordinates bigger in such a way that we cant even
  glimpse at 2 solar systems with inner plannets in solar view on the same screen or at least
  that they are clearly separated... we need to eliminate click confusion, maybe it would be a
  healthy approach to just ignore one of the spatial coordinates and effectively represent the
  galaxy as a disk .... ( even though we would keep the 3d rotation and positioning, would
  contribute at spatial discrimination between star systems ). maybe we should have this galaxy
  disk view as a new super view, while spatial clusters remain as they are now but with a far
  fewer number of systems. ( a few in theory )"
- "so basically we will have an extra zoom leve : galaxy. in brief : zooms are in order of
  magnitude of the coverage: planet / system / cluster -> we see indifidual stars / quadrant ->
  we see individual clusters / galaxy -> we see individual clusters"

Runs after task 161 (catalogue curation: duplicates merged and close stars moved apart in 3D),
task 164 (both regenerate `seed.sql`) and task 159 (it changes `schema.sql` and the save format)
have merged. Start from main after both.

## The levels

The camera zooms continuously through five levels. Each frame is nested in the next by a
similarity transform, as now (`rom-render/src/zoom.rs`, "Rendering notes" in
`docs/world-model.md`):

| Level | Shows | Positions |
| --- | --- | --- |
| Planet | the surface and the globe | as now |
| System | one system's star, planets and bodies | as now |
| Cluster | the few systems of one cluster, as individual stars | true 3D (the curated catalogue `x, y, z`), the current star map's look |
| Quadrant | the clusters of one galactic quadrant, each as a small labelled group of stars | on the galactic disk |
| Galaxy | the whole disk: every cluster of the match, the quadrants marked | on the galactic disk |

- **Cluster view.** This is today's 3D star map, limited to one cluster's systems. Its scale is
  chosen per cluster, so its systems are always clearly apart: in the system view, at any zoom
  before the hand-over, no other system's star or planets are on screen. Measure the system view
  (widest camera distance, field of view) and derive the scale from the cluster's smallest
  spacing. The 3D rotation and elevation stay. Systems of other clusters are not drawn here,
  except as the far ends of routes that leave the cluster: draw these as a line toward the
  target cluster's direction with its name, clickable to fly there.
- **The disk (quadrant and galaxy views).**
  - **Placement.** A cluster sits at its centroid's galactic longitude and compressed distance
    from the Sun, with its galactic latitude dropped, in the orbit plane (frame `y = 0`). The
    camera keeps its full 3D rotation. A cluster is drawn as its stars, small, around that point
    (their true offsets, flattened onto the disk), with its name.
  - **Routes.** Routes between clusters are drawn as lines between clusters.
  - **The galaxy view** marks the four quadrants (I–IV, galactic longitude 0–90–180–270,
    `glon_mdeg`, task 89) with faint sector lines and labels.
- **Skipped levels.** A level that would show a single item is skipped by the zoom: a match
  whose systems are all in one cluster goes from the cluster view to the end of the zoom, and a
  match inside one quadrant has no quadrant level. A one-system (solar) match keeps planet and
  system only.
- **Clicks and flights.**
  - Clicking a cluster in the quadrant or galaxy view flies into its cluster view.
  - Clicking a system in the cluster view flies into it.
  - Home (the existing key) returns to the home planet.
  - Picking never hits a neighbour: test it without a window.
- **Ships.** Ships inside a cluster are drawn in the cluster view as now. A ship travelling a
  route between two clusters is drawn on that route's line in the quadrant and galaxy views. In
  the cluster view it shows leaving toward the route's exit. Inhibitors, jump charge and every
  other rule are unchanged.
- **The minimap** (`hud_minimap.rs`): in map mode it shows the current level's content (the
  cluster's systems, or the clusters).
- **Labels.** Cluster names show at the quadrant and galaxy levels; system names at the cluster
  level, as now.

## Clusters in the data

- **Schema.**
  - New table `star_cluster(id, name, x, y)`: disk position in map units, from the centroid's
    galactic longitude and compressed distance.
  - New column `star.cluster_id NOT NULL REFERENCES star_cluster(id)`.
  - `star_system` gets `cluster_id`, the cluster of its star (NULL for the celestial home and
    for systems without a catalogue star).

  Lint views stay empty. Write it up in `database.md` ("Star catalogue") and the schema comments.
- **The generator** (`db/content/stars.py`) groups the curated stars deterministically:
  - **Grouping.** Agglomerative grouping by 3D map distance, nearest pairs first, ties by
    Hipparcos number. A cluster holds 3 to 7 stars. A star with no neighbour within a bound
    you choose may stand alone (cluster of 1); report how many. Report the cluster count and the
    distribution of sizes.
  - **Names.** A cluster is named after its brightest named star with "group", or after a
    traditional cluster name where it is one: "Pleiades", "Hyades". The names must be real
    astronomy or plain words.
  - **Disk overlap.** Clusters must not overlap on the disk at the galaxy view's scale. If two
    do, move the later one along its longitude ray, as task 161 moves stars, and report it.
- **Map generation** (`rom-mapgen`):
  - **Gameplay unchanged.** Routes, route lengths, travel times, region choice and fair homes
    keep using the true 3D positions.
  - **Cluster on each system.** It sets `cluster_id` on each system.
  - **Hashes.** Universe hashes change only through the new field. Say so.
- **Save, load and network.** The universe's systems carry `cluster_id` through save, load and
  the network snapshot. Bump the save format. No compatibility code (owner, 2026-11-25).

## Out of scope

Gameplay tied to clusters is task 163, which runs after this one: jumps between clusters only
from the Age of Abundance, home clusters with a fair share of planets, and the view tiers. This
task is presentation and data only. Galaxy-to-galaxy travel stays parked.

## Files

`app/db/schema.sql`, `app/db/content/stars.py`, `app/db/seed.sql` (regenerated), `rom-rules`
(load clusters), `rom-mapgen`, `rom-save`, `rom-render` (`universe.rs`, `zoom.rs`, `space.rs`,
`hud_minimap.rs`, the star-map drawing), `rom-app` (input, labels, wherever the snapshot passes
the universe), and docs: `world-model.md` (levels, camera), `database.md`.

Implementer: Claude (Opus).

## Done when (every card)

- **Gate green** from `app/`:
  - fmt;
  - clippy `-D warnings`;
  - workspace tests: at least the count on main when you start, plus this card's tests;
  - fight matrix `unanswered=0`;
  - duel 20–0;
  - the database loads with empty lint views;
  - the playthrough and ascension hashes, either unchanged or explained.
- **Tests:**
  - clustering is deterministic, each cluster has 1–7 stars, and every star has a cluster;
  - clusters do not overlap on the disk;
  - the cluster view's system-view rule holds for every cluster;
  - picking never hits a neighbour at each level;
  - level skipping on a one-cluster match;
  - save and load round-trip `cluster_id`.
- **Screenshots** you have Read, saved as `app/docs/img/162-*.png`, through the game's own paths
  (`--view` with new values `cluster`, `quadrant`, `galaxy`, plus the existing bench or script
  paths):
  - a cluster view of 3–7 systems;
  - a quadrant view;
  - the galaxy view of an all-catalogue match;
  - a wheel strip from the system view to the galaxy;
  - the Alpha Centauri system view with no neighbour on screen.
- **Report:**
  - files changed;
  - the cluster numbers;
  - the scales and how you measured the system view;
  - what you could not verify.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/world-model.md` (the real sky, the camera,
rendering notes, quadrants), `app/docs/database.md` "Star catalogue",
`app/db/content/stars.py`, `app/crates/rom-mapgen/src/universe.rs`,
`app/crates/rom-render/src/universe.rs`, `app/crates/rom-render/src/zoom.rs`,
`app/crates/rom-render/src/space.rs`, `app/crates/rom-render/src/hud_minimap.rs`.

Memory rules: `CARGO_BUILD_JOBS=4`; check `free -g`; big runs under
`systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0`. Never use `xdotool`,
`wmctrl` or `sudo`. Do not stop processes you did not start (the owner may be playing). Logs go
only in the scratchpad's `t162/`. Never write `scratchpad/gate*.sh`. No cargo target dir in the
scratchpad or /tmp. Work in `../rise-of-man-worktrees/162-galaxy-zoom-levels` on branch
`task/162-galaxy-zoom-levels`. Commit there (stage by name); do not merge, tag or push.
