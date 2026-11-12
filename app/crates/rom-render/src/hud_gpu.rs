//! GPU resources owned exclusively by the render thread.
use crate::hud::{HudPainter, HudSnapshot, HudTexture};
use crate::hud_layout::{HudLayout, Rect, Region, scale_needs_rebake};
use crate::hud_minimap::{self, MapMark};
use crate::{Camera, EntityView, TerrainScene};
use raylib::prelude::*;

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
    ) {
        for q in self.painter.compose(layout, snapshot) {
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
        })
    }
    pub(crate) fn draw(
        &self,
        d: &mut impl RaylibDraw,
        layout: &HudLayout,
        entities: &[EntityView],
        camera: &Camera,
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
        for mark in hud_minimap::marks(entities, camera, layout, self.size, area) {
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
