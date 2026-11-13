//! Fitting the orthographic light camera of the shadow map to the part of the surface in view.
//! Pure maths: no window, no GPU.

use crate::camera::Camera;
use crate::orbit::OrbitCamera;
use crate::pick::View;

/// Side of the shadow map in texels.
pub const SHADOW_MAP_SIZE: u32 = 2048;

/// The light camera: an orthographic box looking along the light.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LightFit {
    /// Unit vector toward the light.
    pub toward: [f32; 3],
    /// Unit vector to the right of the light view.
    pub right: [f32; 3],
    /// Unit vector up in the light view.
    pub up: [f32; 3],
    /// Eye of the light camera.
    pub eye: [f32; 3],
    /// Point the light camera looks at.
    pub target: [f32; 3],
    /// Half the side of the square the box covers, in world units.
    pub half: f32,
    /// Near clip distance from the eye.
    pub near: f32,
    /// Far clip distance from the eye.
    pub far: f32,
}

impl LightFit {
    /// Fits the box around `points` seen along the light coming from `toward` (unit), with room
    /// for casters up to `caster_height` above the points and receivers as far below. The centre
    /// is snapped to whole texels and the size to steps of about 6 %, so small camera moves
    /// neither shimmer the shadows nor force a new map.
    pub fn around(points: &[[f32; 3]], toward: [f32; 3], caster_height: f32) -> Option<Self> {
        if points.is_empty() {
            return None;
        }
        let toward = unit(toward);
        let forward = toward.map(|c| -c);
        let hint = if forward[1].abs() > 0.99 {
            [0.0, 0.0, 1.0]
        } else {
            [0.0, 1.0, 0.0]
        };
        let right = unit(cross(forward, hint));
        let up = cross(right, forward);
        let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
        for &p in points {
            let q = [dot(p, right), dot(p, up), dot(p, toward)];
            for k in 0..3 {
                lo[k] = lo[k].min(q[k]);
                hi[k] = hi[k].max(q[k]);
            }
        }
        let extent = (hi[0] - lo[0]).max(hi[1] - lo[1]) / 2.0 + caster_height + 1.0;
        // Quantise the size, then snap the centre to the texel grid of that size.
        let step = 1.06_f32;
        let half = step.powf(extent.max(1.0).log(step).ceil());
        let texel = 2.0 * half / SHADOW_MAP_SIZE as f32;
        let snap = |v: f32| (v / texel).round() * texel;
        let (cx, cy) = (snap((lo[0] + hi[0]) / 2.0), snap((lo[1] + hi[1]) / 2.0));
        let margin = caster_height + 2.0;
        let top = hi[2] + margin;
        let bottom = lo[2] - margin;
        let centre = add(scale(right, cx), scale(up, cy));
        let eye = add(centre, scale(toward, top));
        Some(Self {
            toward,
            right,
            up,
            eye,
            target: add(centre, scale(toward, bottom)),
            half,
            near: 0.01,
            far: top - bottom,
        })
    }

    /// Normalised light-space coordinates of `p`: x and y in −1..1 across the box, z in 0..1
    /// from the near to the far plane. Inside the box when all three are in range.
    pub fn project(&self, p: [f32; 3]) -> [f32; 3] {
        let d = sub(p, self.eye);
        [
            dot(d, self.right) / self.half,
            dot(d, self.up) / self.half,
            (-dot(d, self.toward) - self.near) / (self.far - self.near),
        ]
    }

    /// Whether `p` lies inside the box.
    pub fn contains(&self, p: [f32; 3]) -> bool {
        let q = self.project(p);
        q[0].abs() <= 1.0 && q[1].abs() <= 1.0 && (0.0..=1.0).contains(&q[2])
    }

    /// Whether a sphere around `centre` of `radius` overlaps the box, across the light and
    /// between the clip planes.
    pub fn may_touch(&self, centre: [f32; 3], radius: f32) -> bool {
        let d = sub(centre, self.eye);
        let depth = -dot(d, self.toward);
        dot(d, self.right).abs() <= self.half + radius
            && dot(d, self.up).abs() <= self.half + radius
            && depth >= self.near - radius
            && depth <= self.far + radius
    }

    /// Width of one shadow-map texel in world units.
    pub fn texel(&self) -> f32 {
        2.0 * self.half / SHADOW_MAP_SIZE as f32
    }
}

/// Conservative bounds of the visible sea-level sphere, including the horizon. Extrema of the
/// sphere clipped by the view planes avoid losing distant receivers at street level.
pub fn globe_footprint(orbit: &OrbitCamera, view: &View) -> Vec<[f32; 3]> {
    let radius = orbit.radius;
    let eye = view.eye();
    let mut planes = vec![(unit(eye), radius * radius / dot(eye, eye).sqrt())];
    planes.extend(view_planes(view));
    let axes = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let mut candidates = Vec::new();
    for axis in axes {
        candidates.extend([scale(axis, radius), scale(axis, -radius)]);
        for &(n, d) in &planes {
            let centre = scale(n, d);
            let r = (radius * radius - d * d).max(0.0).sqrt();
            let tangent = unit(sub(axis, scale(n, dot(axis, n))));
            candidates.extend([
                add(centre, scale(tangent, r)),
                sub(centre, scale(tangent, r)),
            ]);
        }
    }
    for (i, &(a, da)) in planes.iter().enumerate() {
        for &(b, db) in &planes[i + 1..] {
            let ab = dot(a, b);
            let denominator = 1.0 - ab * ab;
            if denominator < 1e-6 {
                continue;
            }
            let centre = add(
                scale(a, (da - ab * db) / denominator),
                scale(b, (db - ab * da) / denominator),
            );
            let remaining = radius * radius - dot(centre, centre);
            if remaining < 0.0 {
                continue;
            }
            let tangent = scale(unit(cross(a, b)), remaining.sqrt());
            candidates.extend([add(centre, tangent), sub(centre, tangent)]);
        }
    }
    candidates.retain(|&p| planes.iter().all(|&(n, d)| dot(n, p) >= d - 0.001));
    if candidates.is_empty() {
        return vec![orbit.target()];
    }
    let lo = axes.map(|a| {
        candidates
            .iter()
            .map(|&p| dot(a, p))
            .fold(f32::INFINITY, f32::min)
    });
    let hi = axes.map(|a| {
        candidates
            .iter()
            .map(|&p| dot(a, p))
            .fold(f32::NEG_INFINITY, f32::max)
    });
    // A world-aligned box encloses the clipped spherical patch in every light orientation.
    (0..8)
        .map(|i| [0, 1, 2].map(|j| if i & (1 << j) == 0 { lo[j] } else { hi[j] }))
        .collect()
}

/// Exact polygon of the flat map's sea-level plane inside the view frustum.
pub fn flat_footprint(_camera: &Camera, view: &View, size: (u32, u32)) -> Vec<[f32; 3]> {
    let (w, h) = (size.0 as f32, size.1 as f32);
    let mut points = vec![[0.0, 0.0, 0.0], [w, 0.0, 0.0], [w, 0.0, h], [0.0, 0.0, h]];
    for (n, d) in view_planes(view) {
        let mut clipped = Vec::new();
        for i in 0..points.len() {
            let (a, b) = (points[i], points[(i + 1) % points.len()]);
            let (da, db) = (dot(n, a) - d, dot(n, b) - d);
            if da >= 0.0 {
                clipped.push(a);
            }
            if (da >= 0.0) != (db >= 0.0) {
                clipped.push(add(a, scale(sub(b, a), da / (da - db))));
            }
        }
        points = clipped;
    }
    points
}

/// Inward-facing side planes of the camera pyramid, in world coordinates.
fn view_planes(view: &View) -> Vec<([f32; 3], f32)> {
    let (w, h) = view.size();
    let rays = [
        view.ray(0.0, 0.0),
        view.ray(w, 0.0),
        view.ray(w, h),
        view.ray(0.0, h),
    ];
    let centre = view.ray(w / 2.0, h / 2.0);
    (0..4)
        .map(|i| {
            let mut n = unit(cross(rays[i], rays[(i + 1) % 4]));
            if dot(n, centre) < 0.0 {
                n = scale(n, -1.0);
            }
            (n, dot(n, view.eye()))
        })
        .collect()
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale(a: [f32; 3], s: f32) -> [f32; 3] {
    a.map(|c| c * s)
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn unit(v: [f32; 3]) -> [f32; 3] {
    let length = dot(v, v).sqrt().max(1e-9);
    v.map(|c| c / length)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orbit::dir_from_lon_lat;

    const R: f32 = 122.0;

    fn views() -> Vec<(OrbitCamera, View)> {
        [
            (0.0, 10.0, 12.0, 30.0),
            (40.0, -20.0, 48.0, 0.0),
            (-120.0, 50.0, 90.0, 200.0),
            (179.0, 89.0, 8.0, 270.0),
            (-179.0, -89.0, 20.0, 90.0),
            (83.0, 0.0, 97.0, 270.0),
        ]
        .into_iter()
        .map(|(lon, lat, distance, heading)| {
            let orbit = OrbitCamera::at(lon, lat, distance, heading, R, 3.0 * R);
            let view = View::from_orbit(&orbit, (1280.0, 800.0), 0.2);
            (orbit, view)
        })
        .collect()
    }

    #[test]
    fn light_camera_covers_the_visible_surface() {
        for (orbit, view) in views() {
            let footprint = globe_footprint(&orbit, &view);
            assert_eq!(footprint.len(), 8);
            for lon in [-70.0_f32, 0.0, 33.0, 160.0] {
                let toward = dir_from_lon_lat(lon.to_radians(), 0.2);
                let fit = LightFit::around(&footprint, toward, 6.0).expect("a fit");
                // The projected view footprint, and casters above it, lie inside the box.
                for &p in &footprint {
                    assert!(fit.contains(p), "{p:?} outside {fit:?}");
                    let raised = add(p, scale(unit(p), 5.0));
                    assert!(fit.contains(raised));
                }
                // Every surface point on screen is part of the footprint's hull: sample the
                // canvas more finely and check that what the camera sees is covered.
                for j in 0..=20 {
                    for i in 0..=20 {
                        let ray = view.ray(64.0 * i as f32, 40.0 * j as f32);
                        let eye = view.eye();
                        let along = -dot(eye, ray);
                        let closest = add(eye, scale(ray, along));
                        let miss = dot(closest, closest) - R * R;
                        if along <= 0.0 || miss > 0.0 {
                            continue;
                        }
                        let hit = add(eye, scale(ray, along - (-miss).sqrt()));
                        assert!(
                            fit.contains(hit),
                            "visible receiver {hit:?} outside {fit:?}"
                        );
                    }
                }
                // Not the whole planet: the box is much smaller than the globe at street level.
                if orbit.distance < 20.0 {
                    assert!(fit.half < R * 0.5, "{}", fit.half);
                }
            }
        }
    }

    #[test]
    fn small_moves_keep_the_fit() {
        let (orbit, view) = &views()[1];
        let toward = dir_from_lon_lat(0.3, 0.1);
        let fit = LightFit::around(&globe_footprint(orbit, view), toward, 6.0).unwrap();
        let points: Vec<_> = globe_footprint(orbit, view)
            .iter()
            .map(|p| add(*p, scale(fit.right, fit.texel() * 0.2)))
            .collect();
        let moved = LightFit::around(&points, toward, 6.0).unwrap();
        assert_eq!(fit.half, moved.half);
        assert!(fit.texel() > 0.0 && fit.near < fit.far);
    }

    #[test]
    fn flat_footprint_stays_on_the_map() {
        let camera = Camera {
            target: [100.0, 0.0, 120.0],
            ..Camera::default()
        };
        let view = View::new(&camera, (1280.0, 800.0), 0.0);
        let points = flat_footprint(&camera, &view, (256, 256));
        for p in &points {
            assert!(p[0] >= 0.0 && p[0] <= 256.0 && p[2] >= 0.0 && p[2] <= 256.0);
        }
        let fit = LightFit::around(&points, unit([0.45, 1.0, 0.3]), 6.0).unwrap();
        assert!(points.iter().all(|&p| fit.contains(p)));
        for y in (0..=256).step_by(4) {
            for x in (0..=256).step_by(4) {
                let p = [x as f32, 0.0, y as f32];
                if let Some((px, py, _)) = view.project(p)
                    && (0.0..=1280.0).contains(&px)
                    && (0.0..=800.0).contains(&py)
                {
                    assert!(fit.contains(p), "visible flat receiver {p:?}");
                }
            }
        }
    }
}
