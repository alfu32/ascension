# Rendering and UI stack

Findings from the sibling project `../meshcraft` (same author), which already
ships the exact combination wanted here: a GPUI shell with a raylib 6 canvas,
shader materials, shader-generated vegetation, shadow mapping and a GPUI code
editor. File paths below are relative to `../meshcraft`.

## 1. Pinned versions that are known to work together

| Crate | Version | Notes |
| --- | --- | --- |
| `gpui` | `=0.2.2` | `default-features = false, features = ["font-kit", "x11"]` |
| `gpui-component` | `=0.5.1` | **vendored and patched** in `vendor/gpui-component` |
| `raylib` | `=6.0.0` | |
| `image` | `=0.25.10` | frame hand-off type |
| `tree-sitter` | `=0.25.10` | + `tree-sitter-glsl =0.2.0` for the editor |
| `ropey` | `=2.0.0-beta.1` | |
| `rfd` | `=0.17.2` | `xdg-portal` file dialogs |
| toolchain | Rust 1.99.1, edition 2024 | `rust-toolchain.toml` |

Patches to carry over (root `Cargo.toml` `[patch.crates-io]`):
- `xattr` — upstream 0.2.3 does not compile on Linux (`ENOATTR`); local copy maps it to `ENODATA`.
- `gpui-component` — highlighter read the wrong rope chunk; unfocused inputs
  kept scheduling frames and blink timers. See `vendor/gpui-component/MESHCRAFT-PATCHES.md`.

Linux needs the `libxkbcommon-x11` development package. Run with
`env -u WAYLAND_DISPLAY cargo run` to force the validated X11 path.
Packaging scripts exist for Linux (AppImage/Snap), Windows (MSIX/WiX) and
macOS under `packaging/`.

## 2. How raylib is shown inside GPUI

Source: `apps/desktop/src/gpui_app.rs` (`ViewportBridge`, `RaylibWorker`,
`render_loop`, `copy_target_to_gpui_image`, `RaylibCanvas`).

```
GPUI thread                               raylib thread ("…-raylib-offscreen")
───────────                               ───────────────────────────────────
publish layout / camera / scene  ──────►  hidden raylib window (.hidden())
   (Arc<Mutex<…>>, Arc<RwLock<…>>)        render into RenderTexture2D
request_redraw (Condvar + revision)       post-process / resolve
                                          texture.load_image()  → RGBA bytes
latest_frame()  ◄──────────────────────   swap R/B, flip rows → image::Frame
img(ImageSource::Render(frame))           store in Arc<Mutex<Option<ViewFrame>>>
```

Details that matter:
- raylib owns its own hidden window and GL context on a dedicated thread; all
  GPU objects are created, used and destroyed on that thread.
- The hand-off is a **latest-frame mailbox**, not a queue: stale frames are
  dropped, never buffered.
- GPUI wants premultiplied **BGRA**, top-down. raylib gives RGBA, bottom-up.
- A GPUI task polls the mailbox every 16 ms and calls `cx.notify()` only when
  the frame sequence number changed.
- `RenderImage` only owns CPU pixels; the GPU atlas entry must be evicted with
  `window.drop_image(old)`. Do it **two frames later** (`on_next_frame` twice)
  or the renderer may still reference the slot.
- Mouse and keyboard arrive in GPUI (`on_mouse_move`, `on_scroll_wheel`,
  `on_key_down`) and are translated into camera/tool messages for the bridge.
- An earlier attempt embedded the raylib window as a native child surface. It
  was abandoned: menus and pop-overs appeared *under* the 3D view.
- Measured in that project: 59–60 frames/s delivered at 1280×800, read-back
  plus conversion usually under 12 ms.

### What differs for a game

| Meshcraft (editor) | Rise of Man (game) |
| --- | --- |
| redraws only when something changes | renders continuously |
| scene = whole document snapshot | scene = interpolated snapshot of the simulation |
| HUD is GPUI | in-match HUD is drawn by raylib into the same texture |

Risk: CPU read-back cost grows with pixel count (1080p ≈ 2×, 4K ≈ 8× the
measured case). Measure early. Mitigations in order: render the 3D view at a
reduced internal scale; asynchronous read-back through pixel-buffer objects;
platform shared-texture interop only if both fall short.

Proposed thread layout: **sim thread** (fixed 20 ticks/s, owns game state) →
publishes a render snapshot → **raylib thread** (interpolates, draws world +
HUD, reads back) → mailbox → **GPUI thread** (shows the image, forwards input,
owns all non-match screens).

## 3. Sun light and shadow map

Source: `apps/desktop/src/gpui_lighting.rs`, `assets/shaders/scene.fs`,
`shadow.fs`.

- One directional light; an orthographic light camera is fitted to the scene
  bounds (centre + radius).
- Shadow target: ordinary 2048² RGBA render texture. Depth is **packed into
  RGB** (24 bit) in `shadow.fs`, so no backend-specific depth-texture
  allocation is needed.
- Scene shader: reprojects with `lightVP`, 3×3 PCF, slope-scaled depth bias
  plus a normal offset, `shadowStrength` blend.
- Normals come from screen-space derivatives of world position, so shared
  indexed meshes need no normal attribute.
- The shadow pass is skipped unless light, geometry or animation changed.

For an RTS the light camera should be fitted to the visible ground area (the
camera frustum's footprint), not to the whole map.

## 4. Shader materials

Source: `assets/shaders/procedural.fs` / `.vs`, `docs/procedural-materials.md`,
`docs/standard-shaders.md`.

- One host template with `/*PARAMETERS*/` and `/*FRAGMENT*/` insertion points.
  A program supplies `proceduralColor`, `proceduralShadingNormal`,
  `proceduralLighting`; the host adds transforms, sun, shadows, optional
  GGX PBR, base texture, and a depth-only path for the shadow pass.
- A program is a JSON definition (parameters with defaults) + vertex + fragment
  snippet. One uniform is generated per parameter.
- Built-ins to reuse: grass, foliage, water, brick, wood, rough texture,
  tiling, rough tiling, polished metal, glass, Perlin noise.
- Patterns are sampled in object space, so they move with the object.
- Three value layers: program defaults → material values → per-assignment
  overrides. Maps directly onto our `material.program` + `material.params`.

## 5. Shader-defined objects (trees, vegetation)

Source: `assets/shaders/procedural_tree.glsl`, `procedural_branching.glsl`,
`apps/desktop/src/gpui_object_bake.rs`, `docs/shader-objects.md`,
`docs/branching-shaders.md`.

- Geometry is produced in the **vertex shader** from `gl_InstanceID` + a seed:
  branch transforms, tapered tubes, leaf cards, wind sway. The CPU supplies
  only counts and bounds.
- One branching engine covers broadleaf, conifer, weeping, shrub, coral and
  roots forms through parameters.
- **Bake to Mesh** reads the vertex shader output back into ordinary triangles.

For the game: do not evaluate thousands of shader trees per frame. Bake each
tree/rock/building variant **once before the match** (after map generation)
into a static mesh, then draw instances. This is what `mesh.baked = 1` means
in the schema. Keep per-frame shader work for things that must animate
(water, grass wind, shields, engine glow).

## 6. Code editor component

Source: `apps/desktop/src/gpui_shader_editor.rs`, `gpui_shader_language.rs`,
`gpui_script_language.rs`.

- `InputState::new(window, cx).code_editor("<language>")` from
  `gpui-component` gives a multi-line editor with tree-sitter highlighting.
- Languages are registered once through `LanguageRegistry::singleton()` with a
  `LanguageConfig` (grammar + highlight query). The GLSL query needs
  `#lua-match?` replaced by `#match?`.
- Hover help is a `HoverProvider` implementation.
- The shader editor previews through the same `ViewportBridge`/`RaylibCanvas`
  pair, re-evaluating shortly after typing stops, with compiler errors shown
  below the panes.

Uses here: in-app material/shader editor, scenario scripts, data inspection.

## 7. Division of UI work

| Surface | Technology | Why |
| --- | --- | --- |
| World view | raylib offscreen | 3D, shaders, shadows |
| In-match HUD | raylib, same texture | no hand-off lag; icons and bars drawn as textured quads; numbers from a small baked glyph atlas |
| Main menu, game setup, settings, hotkey editor, profiles, save/load, lobby, editors | GPUI + gpui-component | real text input, fonts, lists, focus, IME |
| Chat line during a match | GPUI overlay above the image | needs real text entry |

GPUI elements stack correctly over the frame image (that is the point of the
image-based hand-off), so a GPUI chat box, tooltip or pause menu can sit on
top of the running match.
