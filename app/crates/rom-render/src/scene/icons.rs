//! Model pictures for the palette (task 91): the GPU half. The uploaded models are drawn side by
//! side into one strip, each filling a square cell, through an orthographic camera at a fixed
//! three-quarter view, in the review gallery's light and the HUD player's colour, on a
//! transparent ground; the strip is read back once and cut into pictures.

use super::*;
use raylib::prelude::{PixelFormat, RaylibRenderTexture2D, RaylibTexture2D};

/// Direction the light travels on the pictures: the review gallery's mid-morning light.
const ICON_LIGHT: [f32; 3] = [-0.55, -0.62, -0.56];
/// Where the camera stands, degrees: turned from the model's front (+z) toward its −x side, so
/// the lit front and the shaded side both show, and raised above the ground.
const ICON_YAW: f32 = -35.0;
const ICON_PITCH: f32 = 30.0;
/// Share of its cell a model's picture fills.
const ICON_FILL: f32 = 0.94;
/// The player whose colour the pictures wear: the HUD shows player one.
const ICON_OWNER: u8 = 1;

/// The camera's forward, right and up axes for the three-quarter view.
pub(super) fn icon_axes() -> [[f32; 3]; 3] {
    let (yaw, pitch) = (ICON_YAW.to_radians(), ICON_PITCH.to_radians());
    // From the model toward the camera.
    let back = [
        pitch.cos() * yaw.sin(),
        pitch.sin(),
        pitch.cos() * yaw.cos(),
    ];
    let forward = back.map(|c| -c);
    let right = sun::unit_or([-forward[2], 0.0, forward[0]]);
    let up = [
        right[1] * forward[2] - right[2] * forward[1],
        right[2] * forward[0] - right[0] * forward[2],
        right[0] * forward[1] - right[1] * forward[0],
    ];
    [forward, right, up]
}

/// Model pictures read back from the GPU.
pub(crate) struct ModelPictures {
    /// Edge of each picture, pixels.
    pub(crate) edge: u32,
    /// Each model asked for and its picture: straight-alpha RGBA rows, top first, transparent
    /// round the model; `None` for a model that is not uploaded.
    pub(crate) pictures: Vec<(u16, Option<Vec<u8>>)>,
}

/// The batch whose pixels are on their way back.
pub(super) struct PendingPictures {
    models: Vec<u16>,
    drawn: Vec<bool>,
    edge: u32,
}

/// The strip the pictures are drawn into, its size, and the read-back.
#[derive(Default)]
pub(super) struct IconStrip {
    target: Option<(RenderTexture2D, (u32, u32))>,
    /// The read without waiting; `None` until first used, or where the context lacks it.
    readback: Option<crate::readback::PictureReadback>,
    /// Whether the read without waiting has been tried.
    tried: bool,
    pending: Option<PendingPictures>,
}

/// Cuts the strip of `pending` (RGBA rows, bottom first, `width` wide) into its pictures.
fn cut(pending: PendingPictures, rgba: &[u8], width: usize) -> ModelPictures {
    let side = pending.edge as usize;
    let pictures = pending
        .models
        .iter()
        .zip(&pending.drawn)
        .enumerate()
        .map(|(cell, (&model, &drawn))| {
            let picture = drawn.then(|| {
                let mut pixels = Vec::with_capacity(side * side * 4);
                // The render target's rows run bottom to top.
                for row in (0..side).rev() {
                    let start = (row * width + cell * side) * 4;
                    pixels.extend_from_slice(&rgba[start..start + side * 4]);
                }
                pixels
            });
            (model, picture)
        })
        .collect();
    ModelPictures {
        edge: pending.edge,
        pictures,
    }
}

impl Scene {
    /// Whether model pictures are on their way back from the GPU.
    pub(crate) fn model_icons_busy(&self) -> bool {
        self.icons.pending.is_some()
    }

    /// The pictures started by [`Scene::render_model_icons`], once the GPU has copied them.
    pub(crate) fn take_model_icons(&mut self) -> Option<ModelPictures> {
        let strip = &mut self.icons;
        let (rgba, size) = strip.readback.as_mut()?.take()?;
        let pending = strip.pending.take()?;
        Some(cut(pending, &rgba, size.0 as usize))
    }

    /// Draws `models` into square pictures of `edge` pixels and starts reading them back:
    /// the pictures come from [`Scene::take_model_icons`] a frame or two later, or at once
    /// (returned here) where the context cannot read without waiting or nothing was drawn.
    pub(crate) fn render_model_icons(
        &mut self,
        rl: &mut RaylibHandle,
        thread: &RaylibThread,
        models: &[u16],
        edge: u32,
    ) -> Option<ModelPictures> {
        let count = models.len();
        let nothing = || ModelPictures {
            edge,
            pictures: models.iter().map(|m| (*m, None)).collect(),
        };
        let Some(gpu) = self.models.as_ref() else {
            return Some(nothing());
        };
        let [forward, right, up] = icon_axes();
        let team = crate::entities::owner_color(ICON_OWNER);
        let mut instances: Vec<(u16, bool, Matrix)> = Vec::new();
        let mut drawn = vec![false; count];
        for (cell, &model) in models.iter().enumerate() {
            // Cells are one unit wide, side by side along the camera's right.
            let offset = cell as f32 - (count as f32 - 1.0) / 2.0;
            if let Some(transform) = gpu.icon_transform(model, [right, up], offset, ICON_FILL, team)
            {
                instances.push((model, true, transform));
                drawn[cell] = true;
            }
        }
        if instances.is_empty() {
            return Some(nothing());
        }
        let strip = &mut self.icons;
        let size = (edge * count as u32, edge);
        if strip.target.as_ref().is_none_or(|(_, s)| *s != size) {
            strip.target = None;
            match rl.load_render_texture(thread, size.0, size.1) {
                Ok(target) => strip.target = Some((target, size)),
                Err(error) => {
                    eprintln!("rom-render: model icons: {error}");
                    return Some(nothing());
                }
            }
        }
        let (target, _) = strip.target.as_mut()?;
        let shader = &mut self.model_shader;
        let location = shader.get_shader_location("toSun");
        shader.set_shader_value(location, sun::unit_or(ICON_LIGHT.map(|c| -c)));
        for name in ["planet", "shadows", "starLight"] {
            let location = shader.get_shader_location(name);
            shader.set_shader_value(location, 0_i32);
        }
        let eye = forward.map(|c| -c * 10.0);
        let camera = Camera3D::orthographic(
            Vector3::new(eye[0], eye[1], eye[2]),
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
            1.0,
        );
        {
            let mut drawing = rl.begin_texture_mode(thread, target);
            drawing.clear_background(Color::new(0, 0, 0, 0));
            let mut world = drawing.begin_mode3D(camera);
            materials::bind_noise(self.noise.as_ref().map(|t| t.id));
            gpu.draw_icons(&mut world, shader, &self.model_material, &mut instances);
            materials::bind_noise(None);
        }
        let pending = PendingPictures {
            models: models.to_vec(),
            drawn,
            edge,
        };
        if !strip.tried {
            strip.tried = true;
            match crate::readback::PictureReadback::new() {
                Ok(readback) => strip.readback = Some(readback),
                Err(error) => eprintln!("rom-render: model icons read back waiting: {error}"),
            }
        }
        if let Some(readback) = strip.readback.as_mut() {
            readback.issue(target, size);
            strip.pending = Some(pending);
            return None;
        }
        let image = target.texture().load_image().ok()?;
        if image.format() != PixelFormat::PIXELFORMAT_UNCOMPRESSED_R8G8B8A8
            || image.width() != size.0 as i32
            || image.height() != size.1 as i32
        {
            return Some(nothing());
        }
        let bytes = size.0 as usize * size.1 as usize * 4;
        // SAFETY: LoadImageFromTexture returned a tightly packed RGBA8 buffer of `bytes` bytes
        // owned by `image`, which outlives this slice.
        let rgba = unsafe { std::slice::from_raw_parts(image.data().cast::<u8>(), bytes) };
        Some(cut(pending, rgba, size.0 as usize))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_icon_view_looks_down_at_the_front_and_one_side() {
        let [forward, right, up] = icon_axes();
        let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        // Looking down, toward −z (at the front), from the −x side.
        assert!(forward[1] < -0.4 && forward[2] < 0.0 && forward[0] > 0.0);
        // Right-handed, unit and level: screen right stays on the ground plane.
        assert!(dot(forward, right).abs() < 1e-5 && dot(forward, up).abs() < 1e-5);
        assert!((dot(right, right) - 1.0).abs() < 1e-5 && right[1].abs() < 1e-6);
        assert!(up[1] > 0.5);
    }
}
