//! Capital markers (task 131): the emblem, a crowned star from the HUD's icon set tinted with
//! the owner's colour, drawn on the 2D batch over the finished picture like the health bars,
//! so nothing in the world hides it. Over a capital city on the ground it keeps one size on
//! screen at every height of the camera; beside a planet in a system and beside a star on the
//! star map it marks where a known capital lies.

use crate::bars::{BarAnchor, behind_sphere};
use crate::entities::{Footing, owner_color};
use crate::hud_model::HudCapital;
use crate::pick::View;
use crate::space::CapitalView;

/// The emblem's name in the HUD atlas.
pub(crate) const EMBLEM_ICON: &str = "icons/capital";
/// Edge of the emblem over a city, in pixels at HUD scale 1.
pub(crate) const CITY_EMBLEM: f32 = 40.0;
/// Edge of the emblem on the minimap, in pixels at HUD scale 1.
pub(crate) const MINIMAP_EMBLEM: f32 = 18.0;
/// Edge of the emblem beside a planet or a star, in pixels at HUD scale 1.
pub(crate) const SPACE_EMBLEM: f32 = 28.0;
/// Room between emblems side by side, in pixels at HUD scale 1.
pub(crate) const EMBLEM_GAP: f32 = 2.0;
/// Pixels between the top of the city's model and the emblem's lower edge, at HUD scale 1:
/// room for the health bar.
const RISE: f32 = 10.0;
/// Height above the ground the emblem stands at when the city itself is not drawn, in tiles.
const FALLBACK_TOP: f32 = 2.0;
/// Opacity of a remembered capital's marks, out of 255.
pub(crate) const FADED: f32 = 160.0;
/// How far the emblem's colour goes toward white.
const LIGHTER: f32 = 0.2;
/// How far a remembered capital's colour goes toward grey.
const GREYING: f32 = 0.35;

/// An emblem placed on the picture: its centre and edge in pixels, colour and opacity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ScreenEmblem {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) size: f32,
    pub(crate) rgb: [u8; 3],
    pub(crate) alpha: u8,
}

impl ScreenEmblem {
    /// The emblem's square on the picture: left, top, width, height.
    pub(crate) fn rect(&self) -> crate::hud_layout::Rect {
        crate::hud_layout::Rect::new(
            self.x - self.size / 2.0,
            self.y - self.size / 2.0,
            self.size,
            self.size,
        )
    }
}

/// Where an emblem hangs in the world: the point above the city's top.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct EmblemAnchor {
    pub(crate) at: [f32; 3],
    /// Distance from the planet's centre to the ground under the city on a round ground: an
    /// emblem past the horizon is hidden.
    pub(crate) ground_radius: Option<f32>,
    pub(crate) owner: u8,
    pub(crate) remembered: bool,
}

/// The owner's colour as bytes, a little lighter so the emblem reads on dark ground and in
/// space, greyed for a remembered capital; and the opacity of its marks.
pub(crate) fn emblem_colour(owner: u8, remembered: bool) -> ([u8; 3], u8) {
    let rgb = owner_color(owner).map(|c| c + (1.0 - c) * LIGHTER);
    let toward = if remembered { GREYING } else { 0.0 };
    let grey = (rgb[0] + rgb[1] + rgb[2]) / 3.0;
    let rgb = rgb.map(|c| {
        ((c + (grey - c) * toward) * 255.0)
            .round()
            .clamp(0.0, 255.0) as u8
    });
    (rgb, if remembered { FADED as u8 } else { 255 })
}

/// The anchors of the capitals on the shown surface: over the city's model where the city is
/// drawn (`bars` holds where every drawn entity's bar hangs), else a little above the ground
/// at its last known place.
pub(crate) fn anchors(
    capitals: &[HudCapital],
    bars: &[BarAnchor],
    ground: &dyn Footing,
    out: &mut Vec<EmblemAnchor>,
) {
    out.clear();
    for capital in capitals {
        let drawn = bars.iter().find(|b| b.id == capital.city);
        let (at, ground_radius) = match drawn {
            Some(bar) => (bar.at, bar.ground_radius),
            None => {
                let (x, y) = (capital.x as f32 / 1000.0, capital.y as f32 / 1000.0);
                let at = ground.point(capital.face, x, y, (0.0, 0.0), FALLBACK_TOP);
                let radius = ground.is_round().then(|| {
                    let floor = ground.frame(capital.face, x, y).origin;
                    floor.iter().map(|c| c * c).sum::<f32>().sqrt()
                });
                (at, radius)
            }
        };
        out.push(EmblemAnchor {
            at,
            ground_radius,
            owner: capital.owner,
            remembered: capital.remembered,
        });
    }
}

/// Places the emblems of `anchors` on `view`: one size on screen at any distance, standing
/// above the anchor; hidden behind the camera, off the picture and past a round ground's
/// horizon. `scale` is the HUD scale.
pub(crate) fn project(
    anchors: &[EmblemAnchor],
    view: &View,
    scale: f32,
    out: &mut Vec<ScreenEmblem>,
) {
    let (w, h) = view.size();
    let eye = view.eye();
    let size = CITY_EMBLEM * scale;
    for a in anchors {
        if a.ground_radius
            .is_some_and(|r| behind_sphere(eye, a.at, [0.0; 3], r * 0.999))
        {
            continue;
        }
        let Some((x, y, _)) = view.project(a.at) else {
            continue;
        };
        let y = y - RISE * scale - size / 2.0;
        if x + size < 0.0 || x - size > w || y + size < 0.0 || y - size > h {
            continue;
        }
        let (rgb, alpha) = emblem_colour(a.owner, a.remembered);
        out.push(ScreenEmblem {
            x,
            y,
            size,
            rgb,
            alpha,
        });
    }
}

/// The owners of the capitals known in `system` at any level, each once, in the order given.
pub(crate) fn system_owners(capitals: &[CapitalView], system: u32) -> Vec<u8> {
    let mut owners: Vec<u8> = Vec::new();
    for c in capitals.iter().filter(|c| c.system == system) {
        if !owners.contains(&c.owner) {
            owners.push(c.owner);
        }
    }
    owners
}

/// The owners of the capitals known to stand on planet `body` (planet level or finer).
pub(crate) fn planet_owners(capitals: &[CapitalView], body: u32) -> Vec<u8> {
    let mut owners: Vec<u8> = Vec::new();
    for c in capitals.iter().filter(|c| c.body == Some(body)) {
        if !owners.contains(&c.owner) {
            owners.push(c.owner);
        }
    }
    owners
}

/// `owners`' emblems of edge `size` side by side, centred on `x` at height `y`, with `gap`
/// pixels between them; `opacity` from 0 to 1.
pub(crate) fn row(
    owners: &[u8],
    (x, y): (f32, f32),
    size: f32,
    gap: f32,
    opacity: f32,
) -> Vec<ScreenEmblem> {
    let n = owners.len() as f32;
    let pitch = size + gap;
    owners
        .iter()
        .enumerate()
        .map(|(i, &owner)| {
            let (rgb, _) = emblem_colour(owner, false);
            ScreenEmblem {
                x: x + (i as f32 - (n - 1.0) / 2.0) * pitch,
                y,
                size,
                rgb,
                alpha: (255.0 * opacity).clamp(0.0, 255.0) as u8,
            }
        })
        .collect()
}

/// `owners`' emblems of edge `size` in a row running left from `right` (the right edge of the
/// first, which stands nearest) at height `y`: beside a star whose name and ships stand to
/// its right.
pub(crate) fn row_leftward(
    owners: &[u8],
    (right, y): (f32, f32),
    size: f32,
    gap: f32,
    opacity: f32,
) -> Vec<ScreenEmblem> {
    owners
        .iter()
        .enumerate()
        .map(|(i, &owner)| {
            let (rgb, _) = emblem_colour(owner, false);
            ScreenEmblem {
                x: right - size / 2.0 - i as f32 * (size + gap),
                y,
                size,
                rgb,
                alpha: (255.0 * opacity).clamp(0.0, 255.0) as u8,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FlatGround;

    fn capital(city: u32, owner: u8, remembered: bool) -> HudCapital {
        HudCapital {
            city,
            owner,
            face: 0,
            x: 10_000,
            y: 12_000,
            ring: 17_000,
            remembered,
        }
    }

    fn view() -> View {
        View::looking(
            [10.0, 40.0, 30.0],
            [10.0, 0.0, 12.0],
            [0.0, 1.0, 0.0],
            (800.0, 600.0),
            0.0,
        )
    }

    #[test]
    fn the_emblem_stands_on_the_drawn_city_or_above_its_last_known_place() {
        let flat = |_: f32, _: f32| 0.0;
        let ground = FlatGround(&flat);
        let bars = [BarAnchor {
            id: 7,
            at: [10.0, 5.0, 12.0],
            width: 3.0,
            frac: 1.0,
            ground_radius: None,
        }];
        let mut out = Vec::new();
        anchors(
            &[capital(7, 1, false), capital(9, 2, true)],
            &bars,
            &ground,
            &mut out,
        );
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].at, [10.0, 5.0, 12.0], "on the model's top");
        assert!(
            (out[1].at[1] - FALLBACK_TOP).abs() < 1e-4,
            "{:?}",
            out[1].at
        );
        assert!(!out[0].remembered && out[1].remembered);
    }

    #[test]
    fn the_emblem_keeps_its_size_on_screen_and_a_remembered_one_is_faded() {
        let near = EmblemAnchor {
            at: [10.0, 2.0, 12.0],
            ground_radius: None,
            owner: 1,
            remembered: false,
        };
        let mut out = Vec::new();
        project(&[near], &view(), 1.0, &mut out);
        let far = View::looking(
            [10.0, 400.0, 300.0],
            [10.0, 0.0, 12.0],
            [0.0, 1.0, 0.0],
            (800.0, 600.0),
            0.0,
        );
        project(
            &[EmblemAnchor {
                remembered: true,
                ..near
            }],
            &far,
            1.0,
            &mut out,
        );
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].size, out[1].size, "one size at any height");
        assert_eq!(out[0].alpha, 255);
        assert!(out[1].alpha < 200, "faded: {}", out[1].alpha);
        // Above the anchor, never on it.
        let (_, y, _) = view().project(near.at).unwrap();
        assert!(out[0].y + out[0].size / 2.0 < y);
    }

    #[test]
    fn an_emblem_past_the_horizon_or_behind_the_camera_is_hidden() {
        let eye = View::looking(
            [0.0, 0.0, 30.0],
            [0.0; 3],
            [0.0, 1.0, 0.0],
            (800.0, 600.0),
            0.0,
        );
        let anchor = |at: [f32; 3]| EmblemAnchor {
            at,
            ground_radius: Some(10.0),
            owner: 1,
            remembered: false,
        };
        let mut out = Vec::new();
        project(
            &[
                anchor([0.0, 0.0, 10.5]),
                anchor([0.0, 0.0, -10.5]),
                anchor([0.0, 0.0, 40.0]),
            ],
            &eye,
            1.0,
            &mut out,
        );
        assert_eq!(out.len(), 1, "{out:?}");
    }

    #[test]
    fn system_and_planet_owners_take_only_what_each_level_knows() {
        let caps = [
            CapitalView {
                owner: 1,
                system: 3,
                body: Some(30),
            },
            CapitalView {
                owner: 2,
                system: 3,
                body: None,
            },
            CapitalView {
                owner: 4,
                system: 5,
                body: Some(50),
            },
        ];
        assert_eq!(system_owners(&caps, 3), vec![1, 2]);
        assert_eq!(
            planet_owners(&caps, 30),
            vec![1],
            "a system-level capital marks no planet"
        );
        assert!(planet_owners(&caps, 31).is_empty());
        assert_eq!(system_owners(&caps, 5), vec![4]);
        assert!(system_owners(&caps, 6).is_empty());
    }

    #[test]
    fn emblems_side_by_side_do_not_overlap() {
        let r = row(&[1, 2, 3], (100.0, 50.0), 20.0, 2.0, 1.0);
        assert_eq!(r.len(), 3);
        assert!((r[1].x - 100.0).abs() < 1e-4, "centred");
        for pair in r.windows(2) {
            assert!(pair[1].x - pair[0].x >= 20.0);
        }
        let left = row_leftward(&[1, 2], (100.0, 50.0), 20.0, 2.0, 1.0);
        assert!(left.iter().all(|e| e.x + e.size / 2.0 <= 100.0 + 1e-3));
        assert!(left[0].x > left[1].x, "the first owner nearest the star");
        assert_eq!(left[0].rgb, emblem_colour(1, false).0);
        assert_eq!(left[1].rgb, emblem_colour(2, false).0);
    }
}
