//! Energy fields between shield emitters (task 156), built into the overlay geometry from the HUD
//! snapshot with the shield domes' glass (task 157): a translucent wall standing on the ground
//! along the line from one emitter to the next, the colour of the domes, brighter toward its
//! edges, with slow bands of light running along it and bright lines along its foot and top.
//! A field that is down is not in the snapshot and is not drawn. Nothing here allocates.

use super::{OverlayGeometry, Tri3};
use crate::HudField;
use crate::entities::Footing;
use crate::hud::HudSnapshot;
use std::f32::consts::TAU;

/// The wall's foot and top above the ground, tiles: between the emitters' light bars.
const FOOT: f32 = 0.28;
const TOP: f32 = 1.84;
/// Rows of the wall, and columns per tile along it (it follows the ground).
const ROWS: usize = 4;
const COLUMNS_PER_TILE: f32 = 2.0;
/// Most columns of one stretch.
const MAX_COLUMNS: usize = 48;
/// The domes' colour (task 157) and the edges' lighter one.
const FIELD: [u8; 3] = [110, 205, 255];
const EDGE: [u8; 3] = [175, 232, 255];

fn rgba(rgb: [u8; 3], alpha: f32) -> [u8; 4] {
    [
        rgb[0],
        rgb[1],
        rgb[2],
        (alpha.clamp(0.0, 1.0) * 255.0) as u8,
    ]
}

/// The density of the wall `row_share` of the way up (0 at its foot, 1 at its top), at `along`
/// tiles from its start, `clock` seconds into the match: denser toward its edges, with bands of
/// light running along it.
fn density(row_share: f32, along: f32, clock: f32, phase: f32) -> f32 {
    let edge = (2.0 * row_share - 1.0).abs();
    let band = 0.5 + 0.5 * (along * 1.7 - clock * 1.6 + phase).sin();
    0.10 + 0.10 * edge * edge + 0.08 * band
}

/// Draws every standing field of the frame.
pub(super) fn walls(out: &mut OverlayGeometry, ground: &dyn Footing, hud: &HudSnapshot) {
    let tps = hud.ticks_per_second.max(1) as f32;
    let clock = hud.game_ticks as f32 / tps;
    for field in &hud.fields {
        wall(out, ground, field, clock);
    }
}

fn wall(out: &mut OverlayGeometry, ground: &dyn Footing, f: &HudField, clock: f32) {
    let (face, x, y) = (f.face, f.from.0 as f32 / 1000.0, f.from.1 as f32 / 1000.0);
    let (dx, dy) = (
        (f.to.0 - f.from.0) as f32 / 1000.0,
        (f.to.1 - f.from.1) as f32 / 1000.0,
    );
    let length = (dx * dx + dy * dy).sqrt();
    if length < 0.01 {
        return;
    }
    let columns = ((length * COLUMNS_PER_TILE).ceil() as usize).clamp(1, MAX_COLUMNS);
    let phase = (f.id % 89) as f32 * 0.37;
    let point = |column: usize, row: usize| {
        let t = column as f32 / columns as f32;
        let lift = FOOT + (TOP - FOOT) * row as f32 / ROWS as f32;
        ground.point(face, x, y, (dx * t, dy * t), lift)
    };
    for column in 0..columns {
        let along = (column as f32 + 0.5) / columns as f32 * length;
        for row in 0..ROWS {
            let share = (row as f32 + 0.5) / ROWS as f32;
            let colour = rgba(FIELD, density(share, along, clock, phase));
            let (a, b, c, d) = (
                point(column, row),
                point(column + 1, row),
                point(column + 1, row + 1),
                point(column, row + 1),
            );
            out.glass.push(Tri3 {
                v: [a, b, c],
                colour,
            });
            out.glass.push(Tri3 {
                v: [a, c, d],
                colour,
            });
        }
        // Its foot and top glow; a faint line runs through its middle.
        let shimmer = 0.75 + 0.25 * (clock * 2.3 + phase + column as f32 * TAU / 7.0).sin();
        for (row, alpha) in [(0, 0.85), (ROWS / 2, 0.25), (ROWS, 0.85)] {
            out.line(
                point(column, row),
                point(column + 1, row),
                rgba(EDGE, alpha * shimmer),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_wall_is_densest_at_its_edges_and_never_opaque() {
        for clock in [0.0, 1.3, 7.9] {
            let middle = density(0.5, 1.0, clock, 0.0);
            let edge = density(0.0, 1.0, clock, 0.0);
            assert!(edge > middle - 0.08 && edge < 0.5 && middle > 0.05);
        }
    }

    #[test]
    fn a_stretch_is_drawn_without_growing_past_its_columns() {
        let height = |_: f32, _: f32| 0.0;
        let ground = crate::entities::FlatGround(&height);
        let mut out = OverlayGeometry::default();
        let field = HudField {
            id: 7,
            face: 0,
            from: (2500, 4500),
            to: (5500, 4500),
        };
        wall(&mut out, &ground, &field, 0.0);
        // Three tiles: six columns of four rows, two triangles each; three lines a column.
        assert_eq!(out.glass.len(), 6 * ROWS * 2);
        assert_eq!(out.lines.len(), 6 * 3);
        let top = out
            .glass
            .iter()
            .flat_map(|t| t.v)
            .map(|v| v[1])
            .fold(0.0, f32::max);
        assert!((top - TOP).abs() < 1e-4, "{top}");
    }
}
