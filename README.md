# Ascension — Rise of Man

A real-time strategy game spanning early history to an age of galactic empires.
Rust, raylib 6 (offscreen 3D) inside a GPUI shell, procedural meshes and
shader materials.

Status: in development. See the task ledger for what works today.

| Path | Contents |
| --- | --- |
| `research/01-reference-data-model.md` | how the reference game models its rules, and what we change |
| `research/02-rendering-ui-stack.md` | raylib + GPUI integration, shadows, shader materials, baked vegetation, code editor |
| `research/03-timeline.md` | the 14 ages, reaching space, nations → blocs → humanity |
| `research/04-hud-and-screens.md` | in-match HUD, game setup, settings |
| `research/05-computer-opponents.md` | how computer players decide, difficulty levels, test harness |
| `app/db/schema.sql` | game database schema (rules + save state) |
| `app/db/seed.sql` | small placeholder dataset that exercises the schema |
| `app/docs/database.md` | explanation of the schema and how to add content |
| `app/docs/implementation-plan.md` | crates, ground rules, phases and milestones |
| `app/docs/hud.md` | in-match HUD design: style, layout rectangles, how it is produced |
| `app/docs/tasks/STATUS.md` | task ledger: what is running, merged or blocked |

```sh
sqlite3 /tmp/rom.db < app/db/schema.sql && sqlite3 /tmp/rom.db < app/db/seed.sql
```
