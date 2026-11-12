# HUD vector sources

Edit `generate.py`, then run `python3 app/assets/hud/generate.py` from the
repository root. Commit the script and its SVG output together.
`python3 app/assets/hud/generate.py --check` verifies that output matches.

Every glyph is independently authored geometry on a transparent 64 × 64
view box. Chrome uses the design-space dimensions in `app/docs/hud.md`.
Reference pictures informed the arrangement and palette; no picture pixels
are included. Captions, the game title, and numbers are live font-atlas text.

The renderer embeds all SVGs through `hud_atlas::SOURCES`. Its CPU tests
verify required icon names, source coverage, rasterisation and packing.
The bundled font and redistribution licence live in `app/assets/fonts/`.

For a preview without a window or GPU, run from `app/`:

```sh
cargo run -p rom-render --example hud_preview -- docs/img/hud-preview.png 1672 941
```
