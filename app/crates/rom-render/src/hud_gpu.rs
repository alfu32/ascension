//! GPU resources owned exclusively by the render thread.
use crate::hud::{HudOverlay, HudPainter, HudSnapshot, HudTexture};
use crate::hud_layout::{HudLayout, Rect, Region, scale_needs_rebake};
use crate::hud_minimap::{self, MapMark};
use crate::vision::shade_pixels;
use crate::{Camera, EntityView, Globe, OrbitCamera, PlanetScene, TerrainScene};
use raylib::prelude::*;
use rom_geo::SurfacePos;
use std::sync::Arc;

pub(crate) struct HudGpu {
    painter: HudPainter,
    artwork: Texture2D,
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
        let g = &painter.glyphs;
        let glyphs = upload(rl, thread, g.width, g.height, &g.rgba)?;
        Ok(Self {
            painter,
            artwork,
            glyphs,
        })
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
        })
    }
    /// Darkens the picture by the tile shades (see [`crate::vision::VisionMap::tile_shades`]).
    pub(crate) fn apply_shades(&mut self, shades: &[[f32; 3]]) {
        if self.tiles.iter().any(|&t| t as usize >= shades.len()) {
            return;
        }
        shade_pixels(&self.base, &self.tiles, shades, &mut self.shaded);
        if let Err(error) = self.texture.update_texture(&self.shaded) {
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
    ) {
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
                            let rgb =
                                crate::owner_color(e.owner).map(|c| (c * 255.0).round() as u8);
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
        for mark in marks {
            match mark {
                MapMark::Dot(r, c) => {
                    clip.draw_rectangle_rec(rectangle(r), Color::new(c[0], c[1], c[2], c[3]))
                }
                MapMark::Line(a, b) => clip.draw_line_ex(
                    Vector2::new(a.0, a.1),
                    Vector2::new(b.0, b.1),
                    layout.scale.max(1.0),
                    Color::WHITE,
                ),
            }
        }
    }
}
