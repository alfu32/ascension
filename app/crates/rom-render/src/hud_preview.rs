//! CPU rendering of the same atlas quads used by the render thread.
use crate::hud::{HudPainter, HudSnapshot, HudTexture};
use crate::hud_layout::{HudLayout, Rect, Region};
use crate::hud_minimap::{self, MapMark};
use crate::{Camera, EntityView, TerrainScene};
use image::{Rgba, RgbaImage};

fn blend(dst: &mut Rgba<u8>, src: [u8; 4]) {
    let a = u32::from(src[3]);
    let da = u32::from(dst[3]);
    let out = a * 255 + da * (255 - a);
    if out == 0 {
        return;
    }
    for i in 0..3 {
        dst[i] = ((u32::from(src[i]) * a * 255 + u32::from(dst[i]) * da * (255 - a) + out / 2)
            / out) as u8;
    }
    dst[3] = out.div_ceil(255) as u8;
}

fn blit(dest: &mut RgbaImage, pixels: &[u8], width: u32, src: Rect, target: Rect, tint: [u8; 4]) {
    for y in (target.y.floor().max(0.0) as u32)
        ..(target.bottom().ceil().min(dest.height() as f32) as u32)
    {
        for x in (target.x.floor().max(0.0) as u32)
            ..(target.right().ceil().min(dest.width() as f32) as u32)
        {
            let sx = src.x + ((x as f32 + 0.5 - target.x) / target.w) * src.w - 0.5;
            let sy = src.y + ((y as f32 + 0.5 - target.y) / target.h) * src.h - 0.5;
            let (ix, iy) = (sx.floor(), sy.floor());
            let (fx, fy) = (sx - ix, sy - iy);
            let sample = |dx: f32, dy: f32, channel: usize| {
                let px = (ix + dx).clamp(src.x, src.right() - 1.0) as u32;
                let py = (iy + dy).clamp(src.y, src.bottom() - 1.0) as u32;
                f32::from(pixels[((py * width + px) * 4) as usize + channel])
            };
            let c = std::array::from_fn(|i| {
                let top = sample(0.0, 0.0, i) * (1.0 - fx) + sample(1.0, 0.0, i) * fx;
                let bottom = sample(0.0, 1.0, i) * (1.0 - fx) + sample(1.0, 1.0, i) * fx;
                ((top * (1.0 - fy) + bottom * fy) * f32::from(tint[i]) / 255.0).round() as u8
            });
            blend(dest.get_pixel_mut(x, y), c);
        }
    }
}

/// Composes chrome, static captions, live sample data and a terrain minimap into RGBA pixels.
pub fn render(
    painter: &mut HudPainter,
    layout: &HudLayout,
    snapshot: &HudSnapshot,
    terrain: &TerrainScene,
    entities: &[EntityView],
    camera: &Camera,
) -> RgbaImage {
    let mut result = RgbaImage::from_pixel(
        layout.width as u32,
        layout.height as u32,
        Rgba([0, 0, 0, 255]),
    );
    let view = layout.rect(Region::MinimapView);
    blit(
        &mut result,
        &[4, 6, 12, 255],
        1,
        Rect::new(0.0, 0.0, 1.0, 1.0),
        view,
        [255; 4],
    );
    let (w, h, data) = hud_minimap::terrain_pixels(terrain);
    let area = hud_minimap::fit_map(
        terrain.width,
        terrain.height,
        layout.rect(Region::MinimapView),
    );
    blit(
        &mut result,
        &data,
        w,
        Rect::new(0.0, 0.0, w as f32, h as f32),
        area,
        [255; 4],
    );
    let mut put = |x: f32, y: f32, c| {
        if area.contains(x, y) && x >= 0.0 && y >= 0.0 && x < layout.width && y < layout.height {
            blend(result.get_pixel_mut(x as u32, y as u32), c);
        }
    };
    for mark in hud_minimap::marks(
        entities,
        camera,
        layout,
        (terrain.width, terrain.height),
        area,
    )
    .into_iter()
    .chain(hud_minimap::city_radius_marks(
        &snapshot.city_radii,
        (terrain.width, terrain.height),
        area,
        None,
    )) {
        match mark {
            MapMark::Dot(r, c) => {
                for y in r.y.floor() as i32..r.bottom().ceil() as i32 {
                    for x in r.x.floor() as i32..r.right().ceil() as i32 {
                        put(x as f32, y as f32, c);
                    }
                }
            }
            MapMark::Line(a, b) => {
                let steps = ((b.0 - a.0).abs().max((b.1 - a.1).abs()).ceil() as u32).max(1);
                for i in 0..=steps {
                    let t = i as f32 / steps as f32;
                    put(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t, [255; 4]);
                }
            }
            // Only the space minimap uses these.
            MapMark::Ring(..) | MapMark::Segment(..) => {}
        }
    }
    let quads = painter.compose(layout, snapshot).to_vec();
    for q in quads {
        let (data, width) = match q.texture {
            HudTexture::Artwork => (&painter.artwork.rgba[..], painter.artwork.width),
            HudTexture::Greyscale => (&painter.greyscale[..], painter.artwork.width),
            HudTexture::Glyphs => (&painter.glyphs.rgba[..], painter.glyphs.width),
            HudTexture::Solid => (&[255, 255, 255, 255][..], 1),
        };
        blit(&mut result, data, width, q.source, q.dest, q.colour);
    }
    result
}
