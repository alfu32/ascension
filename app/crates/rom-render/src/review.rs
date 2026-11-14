//! What the model review screens add to a flat scene: a fixed light, every model at full
//! detail, labels at points of the world, part outlines, and a sheet of several views.

use crate::camera::Camera;
use crate::overlay::Line3;

/// A text drawn centred on a point of the world.
#[derive(Clone, Debug, PartialEq)]
pub struct WorldLabel {
    /// The point `[east, up, south]` in tiles.
    pub at: [f32; 3],
    /// The text.
    pub text: String,
    /// RGBA.
    pub colour: [u8; 4],
}

/// One view of a sheet.
#[derive(Clone, Debug, PartialEq)]
pub struct SheetView {
    /// The camera of the view.
    pub camera: Camera,
    /// Title drawn in the view's top-left corner.
    pub title: String,
}

/// A review layout published to the render thread.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Review {
    /// Direction the light travels; `None` keeps the flat map's usual light.
    pub light: Option<[f32; 3]>,
    /// Draw every model with its near mesh, whatever its distance.
    pub near_only: bool,
    /// Labels drawn over the world.
    pub labels: Vec<WorldLabel>,
    /// Lines drawn in the world.
    pub lines: Vec<Line3>,
    /// Views drawn side by side in two columns instead of the camera's single view.
    pub sheet: Vec<SheetView>,
    /// Seconds of motion shown, the same for every model (no phase per entity); `None` runs
    /// the render clock.
    pub anim_time: Option<f32>,
}

/// The cell of view `index` of `count` on a canvas of `size` pixels: two columns, as many rows
/// as needed. Returns the cell's top-left corner and size.
pub fn sheet_cell(index: usize, count: usize, size: (u32, u32)) -> ((u32, u32), (u32, u32)) {
    let columns = if count > 1 { 2 } else { 1 };
    let rows = count.div_ceil(columns).max(1) as u32;
    let (w, h) = (size.0 / columns as u32, size.1 / rows);
    let (column, row) = ((index % columns) as u32, (index / columns) as u32);
    ((column * w, row * h), (w.max(1), h.max(1)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sheet_cells_tile_the_canvas() {
        assert_eq!(sheet_cell(0, 1, (800, 600)), ((0, 0), (800, 600)));
        assert_eq!(sheet_cell(0, 4, (800, 600)), ((0, 0), (400, 300)));
        assert_eq!(sheet_cell(1, 4, (800, 600)), ((400, 0), (400, 300)));
        assert_eq!(sheet_cell(3, 4, (800, 600)), ((400, 300), (400, 300)));
        assert_eq!(sheet_cell(2, 3, (800, 600)), ((0, 300), (400, 300)));
    }
}
