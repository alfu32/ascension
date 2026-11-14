//! GPU resources owned exclusively by the render thread.
use crate::hud::{HudOverlay, HudPainter, HudSnapshot, HudTexture};
use crate::hud_layout::{HudLayout, Rect, Region, scale_needs_rebake};
use crate::hud_minimap::{self, MapMark};
use crate::hud_text::{Align, TextSize, TextStyle};
use crate::scene::SpaceLabel;
use crate::vision::shade_pixels;
use crate::zoom::{Level, ZoomCamera};
use crate::{Camera, EntityView, Globe, OrbitCamera, PlanetScene, TerrainScene};
use raylib::prelude::*;
use rom_geo::SurfacePos;
use std::sync::Arc;

pub(crate) struct HudGpu {
    painter: HudPainter,
    artwork: Texture2D,
    greyscale: Texture2D,
    glyphs: Texture2D,
}

pub(crate) fn upload(
    _rl: &mut RaylibHandle,
    thread: &RaylibThread,
    w: u32,
    h: u32,
    pixels: &[u8],
) -> Result<Texture2D, String> {
    assert_eq!(pixels.len(), w as usize * h as usize * 4);
    let image = raylib::ffi::Image {
        data: pixels.as_ptr().cast_mut().cast(),
        width: w as i32,
        height: h as i32,
        mipmaps: 1,
        format: PixelFormat::PIXELFORMAT_UNCOMPRESSED_R8G8B8A8 as i32,
    };
    // SAFETY: an active render-thread context copies the live RGBA slice; the borrowed Image is never freed by raylib.
    let texture = unsafe { Texture2D::from_raw(raylib::ffi::LoadTextureFromImage(image)) };
    if texture.id == 0 {
        return Err("could not upload HUD texture".into());
    }
    texture.set_texture_filter(thread, TextureFilter::TEXTURE_FILTER_BILINEAR);
    Ok(texture)
}

pub(crate) fn rectangle(r: Rect) -> Rectangle {
    Rectangle::new(r.x, r.y, r.w, r.h)
}

impl HudGpu {
    pub(crate) fn load(
        rl: &mut RaylibHandle,
        thread: &RaylibThread,
        scale: f32,
    ) -> Result<Self, String> {
        let painter = HudPainter::bake(scale).map_err(|e| e.to_string())?;
        let a = &painter.artwork;
        let artwork = upload(rl, thread, a.width, a.height, &a.rgba)?;
        let greyscale = upload(rl, thread, a.width, a.height, &painter.greyscale)?;
        let g = &painter.glyphs;
        let glyphs = upload(rl, thread, g.width, g.height, &g.rgba)?;
        Ok(Self {
            painter,
            artwork,
            greyscale,
            glyphs,
        })
    }
    /// Draws the names of the star map in the HUD font, dropping those that would overlap.
    pub(crate) fn draw_labels(
        &mut self,
        d: &mut impl RaylibDraw,
        layout: &HudLayout,
        labels: &[SpaceLabel],
        alpha: f32,
    ) {
        if labels.is_empty() || alpha <= 0.01 {
            return;
        }
        let world = layout.world_area();
        let labels: Vec<SpaceLabel> = labels
            .iter()
            .filter(|l| l.y > world.y && l.y < world.bottom() && l.x > 0.0)
            .cloned()
            .collect();
        let labels = &labels[..];
        let atlas = &self.painter.glyphs;
        let ratio = layout.scale / atlas.scale;
        let height = atlas.line_height(TextSize::Small) as f32 * ratio;
        let boxes: Vec<_> = labels
            .iter()
            .map(|l| {
                let width = atlas.measure(&l.text, TextSize::Small) as f32 * ratio;
                (l.x, l.y - height, width, height, l.priority)
            })
            .collect();
        let mut quads = Vec::new();
        for index in hud_minimap::declutter(&boxes) {
            let label = &labels[index];
            let shade = [0, 0, 0, (200.0 * alpha) as u8];
            let colour = [
                label.colour[0],
                label.colour[1],
                label.colour[2],
                (f32::from(label.colour[3]) * alpha) as u8,
            ];
            for (dx, dy, tint) in [(1, 1, shade), (0, 0, colour)] {
                let style = TextStyle {
                    size: TextSize::Small,
                    align: Align::Left,
                    colour: tint,
                };
                atlas.draw(
                    &label.text,
                    style,
                    label.x.round() as i32 + dx,
                    label.y.round() as i32 + dy,
                    &mut quads,
                );
            }
        }
        for q in quads {
            let (x, y, w, h) = q.src;
            d.draw_texture_pro(
                &self.glyphs,
                Rectangle::new(x as f32, y as f32, w as f32, h as f32),
                Rectangle::new(
                    q.dst.0 as f32,
                    q.dst.1 as f32,
                    w as f32 * ratio,
                    h as f32 * ratio,
                ),
                Vector2::zero(),
                0.0,
                Color::new(q.colour[0], q.colour[1], q.colour[2], q.colour[3]),
            );
        }
    }
    pub(crate) fn needs_rebake(&self, scale: f32) -> bool {
        scale_needs_rebake(self.painter.artwork.scale, scale)
    }
    pub(crate) fn draw(
        &mut self,
        d: &mut impl RaylibDraw,
        layout: &HudLayout,
        snapshot: &HudSnapshot,
        overlay: &HudOverlay,
    ) {
        for q in self.painter.compose_with(layout, snapshot, overlay) {
            let colour = Color::new(q.colour[0], q.colour[1], q.colour[2], q.colour[3]);
            let texture = match q.texture {
                HudTexture::Artwork => &self.artwork,
                HudTexture::Greyscale => &self.greyscale,
                HudTexture::Glyphs => &self.glyphs,
                HudTexture::Solid => {
                    d.draw_rectangle_rec(rectangle(q.dest), colour);
                    continue;
                }
            };
            d.draw_texture_pro(
                texture,
                rectangle(q.source),
                rectangle(q.dest),
                Vector2::zero(),
                0.0,
                colour,
            );
        }
    }
}

/// What the minimap draws over its terrain besides entities and the view outline.
pub(crate) struct MinimapOverlays<'a> {
    /// Own cities' economic radii, drawn while a city is selected.
    pub cities: &'a [crate::hud::HudCityRadius],
    /// Whether latitude and longitude lines are on.
    pub graticule: bool,
}

pub(crate) struct MinimapGpu {
    texture: Texture2D,
    size: (u32, u32),
    /// Unshaded pixels, RGBA.
    base: Vec<u8>,
    /// The tile each pixel shows, for fog shading.
    tiles: Vec<u32>,
    /// Scratch for shaded pixels.
    shaded: Vec<u8>,
    /// The planet of a sphere minimap, which is equirectangular.
    globe: Option<Arc<Globe>>,
    /// Start directions on a sphere planet, shown until entities arrive.
    starts: Vec<[f32; 3]>,
    /// The direction each pixel of a sphere minimap faces, for the night side.
    dirs: Vec<[f32; 3]>,
    /// The light direction the night side was last drawn for.
    light: Option<[f32; 3]>,
    /// Whether `shaded` holds the fog-shaded picture (otherwise `base` is the picture).
    fogged: bool,
    /// Scratch for the picture with the night side.
    lit: Vec<u8>,
}
impl MinimapGpu {
    pub(crate) fn load(
        rl: &mut RaylibHandle,
        thread: &RaylibThread,
        terrain: &TerrainScene,
    ) -> Result<Self, String> {
        let (w, h, rgba) = hud_minimap::terrain_pixels(terrain);
        Ok(Self {
            texture: upload(rl, thread, w, h, &rgba)?,
            size: (terrain.width, terrain.height),
            tiles: hud_minimap::terrain_pixel_tiles(terrain),
            base: rgba,
            shaded: Vec::new(),
            globe: None,
            starts: Vec::new(),
            dirs: Vec::new(),
            light: None,
            fogged: false,
            lit: Vec::new(),
        })
    }
    /// The equirectangular minimap of a sphere planet.
    pub(crate) fn load_planet(
        rl: &mut RaylibHandle,
        thread: &RaylibThread,
        planet: &PlanetScene,
        globe: &Arc<Globe>,
    ) -> Result<Self, String> {
        let (w, h, rgba) = hud_minimap::planet_pixels(planet);
        Ok(Self {
            texture: upload(rl, thread, w, h, &rgba)?,
            size: (w, h),
            tiles: hud_minimap::planet_pixel_tiles(planet.n),
            base: rgba,
            shaded: Vec::new(),
            globe: Some(Arc::clone(globe)),
            starts: planet.starts.iter().map(|&s| globe.dir_of(s)).collect(),
            dirs: hud_minimap::planet_pixel_dirs(),
            light: None,
            fogged: false,
            lit: Vec::new(),
        })
    }
    /// Darkens the picture by the tile shades (see [`crate::vision::VisionMap::tile_shades`]).
    pub(crate) fn apply_shades(&mut self, shades: &[[f32; 3]]) {
        if self.tiles.iter().any(|&t| t as usize >= shades.len()) {
            return;
        }
        shade_pixels(&self.base, &self.tiles, shades, &mut self.shaded);
        self.fogged = true;
        self.refresh();
    }
    /// Darkens the side of a sphere minimap that faces away from the light `toward` (a unit
    /// direction from the planet centre); redrawn only when the light moved about one pixel.
    pub(crate) fn set_light(&mut self, toward: [f32; 3]) {
        if self.globe.is_none() {
            return;
        }
        if self
            .light
            .is_some_and(|last| !hud_minimap::light_moved(last, toward))
        {
            return;
        }
        self.light = Some(toward);
        self.refresh();
    }
    /// Uploads the picture: fog shading first, then the night side.
    fn refresh(&mut self) {
        let picture = if self.fogged {
            &self.shaded
        } else {
            &self.base
        };
        let result = match self.light {
            Some(toward) if !self.dirs.is_empty() => {
                hud_minimap::night_pixels(picture, &self.dirs, toward, &mut self.lit);
                self.texture.update_texture(&self.lit)
            }
            _ => self.texture.update_texture(picture),
        };
        if let Err(error) = result {
            eprintln!("rom-render: minimap shading: {error}");
        }
    }
    pub(crate) fn draw(
        &self,
        d: &mut impl RaylibDraw,
        layout: &HudLayout,
        entities: &[EntityView],
        camera: &Camera,
        orbit: Option<&OrbitCamera>,
        overlays: MinimapOverlays<'_>,
    ) {
        let MinimapOverlays { cities, graticule } = overlays;
        let view = layout.rect(Region::MinimapView);
        d.draw_rectangle_rec(rectangle(view), Color::new(4, 6, 12, 255));
        let area = hud_minimap::fit_map(self.size.0, self.size.1, view);
        d.draw_texture_pro(
            &self.texture,
            Rectangle::new(
                0.0,
                0.0,
                self.texture.width as f32,
                self.texture.height as f32,
            ),
            rectangle(area),
            Vector2::zero(),
            0.0,
            Color::WHITE,
        );
        let mut clip = d.begin_scissor_mode(
            area.x.ceil() as i32,
            area.y.ceil() as i32,
            area.w.floor() as i32,
            area.h.floor() as i32,
        );
        if graticule && self.globe.is_some() {
            for (kind, a, b) in crate::graticule::minimap_graticule(area) {
                let [r, g, b_, alpha] = kind.colour();
                clip.draw_line_ex(
                    Vector2::new(a.0, a.1),
                    Vector2::new(b.0, b.1),
                    layout.scale.max(1.0),
                    Color::new(r, g, b_, alpha),
                );
            }
        }
        let marks = match (orbit, &self.globe) {
            (Some(orbit), Some(globe)) => {
                let dots: Vec<_> = if entities.is_empty() {
                    self.starts
                        .iter()
                        .enumerate()
                        .map(|(i, &dir)| {
                            let rgb = crate::owner_color((i % 8) as u8 + 1)
                                .map(|c| (c * 255.0).round() as u8);
                            (dir, [rgb[0], rgb[1], rgb[2], 255], 4.0)
                        })
                        .collect()
                } else {
                    entities
                        .iter()
                        .map(|e| {
                            let dir = globe.dir_of(SurfacePos {
                                face: e.face,
                                x: (e.x * 1000.0).round() as i64,
                                y: (e.y * 1000.0).round() as i64,
                            });
                            let rgb = (if e.carry_node {
                                crate::entities::NODE_COLOR
                            } else {
                                crate::owner_color(e.owner)
                            })
                            .map(|c| (c * 255.0).round() as u8);
                            (
                                dir,
                                [rgb[0], rgb[1], rgb[2], 255],
                                if e.building { 4.0 } else { 2.0 },
                            )
                        })
                        .collect()
                };
                hud_minimap::globe_marks(orbit, &dots, layout, area)
            }
            _ => hud_minimap::marks(entities, camera, layout, self.size, area),
        };
        draw_marks(&mut clip, layout, marks);
        draw_marks(
            &mut clip,
            layout,
            hud_minimap::city_radius_marks(cities, self.size, area, self.globe.as_deref()),
        );
    }
}

fn draw_marks(d: &mut impl RaylibDraw, layout: &HudLayout, marks: Vec<MapMark>) {
    for mark in marks {
        match mark {
            MapMark::Dot(r, c) => {
                d.draw_rectangle_rec(rectangle(r), Color::new(c[0], c[1], c[2], c[3]))
            }
            MapMark::Line(a, b) => d.draw_line_ex(
                Vector2::new(a.0, a.1),
                Vector2::new(b.0, b.1),
                layout.scale.max(1.0),
                Color::WHITE,
            ),
            MapMark::Ring(centre, radius, c) => d.draw_circle_lines(
                centre.0.round() as i32,
                centre.1.round() as i32,
                radius,
                Color::new(c[0], c[1], c[2], c[3]),
            ),
            MapMark::Segment(a, b, c) => d.draw_line_ex(
                Vector2::new(a.0, a.1),
                Vector2::new(b.0, b.1),
                1.0,
                Color::new(c[0], c[1], c[2], c[3]),
            ),
        }
    }
}

/// The minimap above the planet range: the system diagram or the star map.
pub(crate) fn draw_space_minimap(
    d: &mut impl RaylibDraw,
    layout: &HudLayout,
    zoom: &ZoomCamera,
    orbit: &OrbitCamera,
) {
    let view = layout.rect(Region::MinimapView);
    d.draw_rectangle_rec(rectangle(view), Color::new(4, 6, 12, 255));
    let mut clip = d.begin_scissor_mode(
        view.x.ceil() as i32,
        view.y.ceil() as i32,
        view.w.floor() as i32,
        view.h.floor() as i32,
    );
    draw_marks(
        &mut clip,
        layout,
        hud_minimap::space_marks(zoom, orbit, layout, view),
    );
}

/// Four pips next to the minimap, one per level: the current one lit, the others dim, and the
/// levels beyond the zoom cap hollow.
pub(crate) fn draw_level_pips(
    d: &mut impl RaylibDraw,
    layout: &HudLayout,
    level: Level,
    cap: Level,
) {
    let view = layout.rect(Region::MinimapView);
    let side = 11.0 * layout.scale;
    let gap = 7.0 * layout.scale;
    let x = view.right() + 12.0 * layout.scale;
    let total = 4.0 * side + 3.0 * gap;
    let top = view.y + (view.h - total) / 2.0;
    for (i, pip) in Level::ALL.into_iter().enumerate() {
        // The farthest level on top, the closest at the bottom.
        let y = top + (3 - i) as f32 * (side + gap);
        let rect = Rectangle::new(x, y, side, side);
        let beyond = pip > cap.max(Level::Globe);
        if pip == level {
            d.draw_rectangle_rec(rect, Color::new(240, 200, 90, 255));
        } else if beyond {
            d.draw_rectangle_lines_ex(rect, 1.0, Color::new(110, 110, 120, 160));
        } else {
            d.draw_rectangle_rec(rect, Color::new(90, 100, 120, 220));
        }
        d.draw_rectangle_lines_ex(rect, 1.0, Color::new(10, 12, 18, 255));
    }
}
