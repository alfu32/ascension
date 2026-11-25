//! Model pictures for the palette (task 91) and the selection's portraits (task 145): the GPU
//! half. The uploaded models are drawn side by side into one strip, each filling a square cell,
//! through an orthographic camera at a fixed three-quarter view, in the review gallery's light
//! and each picture's owner colour, on a transparent ground; the strip is read back once and cut
//! into pictures. Two render targets are kept, one for the palette's strip and one for a
//! portrait, each made once per picture size and reused.

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
/// The player whose colour the palette's pictures wear: the HUD shows player one.
pub(crate) const ICON_OWNER: u8 = 1;
/// Cells of the palette's strip: most pictures drawn in one batch, a full palette page.
pub(crate) const STRIP_CELLS: usize = 15;

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

/// What a batch of pictures is for, and so which render target it is drawn in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PictureKind {
    /// The palette's, the queue's and the selection tiles' pictures.
    Icons,
    /// The single selection's portrait.
    Portrait,
}

/// A model and the owner whose colour its picture wears.
pub(crate) type Pictured = (u16, u8);

/// Model pictures read back from the GPU.
pub(crate) struct ModelPictures {
    /// What they were drawn for.
    pub(crate) kind: PictureKind,
    /// Edge of each picture, pixels.
    pub(crate) edge: u32,
    /// Each model and owner asked for and its picture: straight-alpha RGBA rows, top first,
    /// transparent round the model; `None` for a model that is not uploaded.
    pub(crate) pictures: Vec<(Pictured, Option<Vec<u8>>)>,
}

/// The batch whose pixels are on their way back.
pub(super) struct PendingPictures {
    kind: PictureKind,
    models: Vec<Pictured>,
    drawn: Vec<bool>,
    edge: u32,
}

/// The targets the pictures are drawn into, their sizes, and the read-back.
#[derive(Default)]
pub(super) struct IconStrip {
    /// The palette's strip of [`STRIP_CELLS`] cells.
    target: Option<(RenderTexture2D, (u32, u32))>,
    /// One portrait.
    portrait: Option<(RenderTexture2D, (u32, u32))>,
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
        kind: pending.kind,
        edge: pending.edge,
        pictures,
    }
}

/// Size of the target a batch of `count` pictures of `kind` is drawn in, at `edge` pixels a
/// picture: the whole palette strip whatever the batch, so it is made once per size.
fn target_size(kind: PictureKind, count: usize, edge: u32) -> (u32, u32) {
    let cells = match kind {
        PictureKind::Icons => count.max(STRIP_CELLS),
        PictureKind::Portrait => count.max(1),
    };
    (edge * cells as u32, edge)
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

    /// Draws `models` (each with the owner whose colour it wears) into square pictures of
    /// `edge` pixels for `kind` and starts reading them back: the pictures come from
    /// [`Scene::take_model_icons`] a frame or two later, or at once (returned here) where the
    /// context cannot read without waiting or nothing was drawn.
    pub(crate) fn render_model_icons(
        &mut self,
        rl: &mut RaylibHandle,
        thread: &RaylibThread,
        models: &[Pictured],
        edge: u32,
        kind: PictureKind,
    ) -> Option<ModelPictures> {
        let count = models.len();
        let nothing = || ModelPictures {
            kind,
            edge,
            pictures: models.iter().map(|m| (*m, None)).collect(),
        };
        let Some(gpu) = self.models.as_ref() else {
            return Some(nothing());
        };
        let size = target_size(kind, count, edge);
        let cells = (size.0 / edge.max(1)) as f32;
        let [forward, right, up] = icon_axes();
        let mut instances: Vec<(u16, bool, Matrix)> = Vec::new();
        let mut drawn = vec![false; count];
        for (cell, &(model, owner)) in models.iter().enumerate() {
            // Cells are one unit wide, side by side along the camera's right, from the left
            // edge of the target.
            let offset = cell as f32 - (cells - 1.0) / 2.0;
            let team = crate::entities::owner_color(owner);
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
        let slot = match kind {
            PictureKind::Icons => &mut strip.target,
            PictureKind::Portrait => &mut strip.portrait,
        };
        if slot.as_ref().is_none_or(|(_, s)| *s != size) {
            // Made again only when the HUD scale (and so the picture size) changes; the old
            // target is unloaded as it drops.
            *slot = None;
            match rl.load_render_texture(thread, size.0, size.1) {
                Ok(target) => *slot = Some((target, size)),
                Err(error) => {
                    eprintln!("rom-render: model icons: {error}");
                    return Some(nothing());
                }
            }
        }
        let (target, _) = slot.as_mut()?;
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
            kind,
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
            // Only the cells drawn are read.
            readback.issue(target, (edge * count as u32, edge));
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

    /// The palette's strip keeps one size whatever the batch, so its target is made once per
    /// HUD scale (task 145, after task 121); a portrait has its own square target.
    #[test]
    fn picture_targets_keep_their_size_across_batches() {
        let full = target_size(PictureKind::Icons, STRIP_CELLS, 40);
        for count in 1..=STRIP_CELLS {
            assert_eq!(target_size(PictureKind::Icons, count, 40), full);
        }
        assert_eq!(full, (40 * STRIP_CELLS as u32, 40));
        assert_eq!(target_size(PictureKind::Portrait, 1, 90), (90, 90));
    }

    /// Cutting a strip read back narrower than the target keeps every picture's own pixels and
    /// turns the rows top first.
    #[test]
    fn cutting_returns_each_cell_top_row_first() {
        let side = 2_usize;
        let width = side * 2;
        // Two cells, two rows, bottom row first: the value is 10 × row + column.
        let rgba: Vec<u8> = (0..side)
            .flat_map(|row| (0..width).flat_map(move |x| [(10 * row + x) as u8; 4]))
            .collect();
        let pending = PendingPictures {
            kind: PictureKind::Portrait,
            models: vec![(3, 1), (4, 2)],
            drawn: vec![true, false],
            edge: side as u32,
        };
        let done = cut(pending, &rgba, width);
        assert_eq!(done.kind, PictureKind::Portrait);
        let first = done.pictures[0].1.as_ref().expect("drawn");
        assert_eq!(first[0], 10, "top row first");
        assert_eq!(first[4], 11);
        assert_eq!(first[8], 0);
        assert_eq!(done.pictures[1], ((4, 2), None));
    }
}
