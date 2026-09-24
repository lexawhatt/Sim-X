//! Invisible view-local bounds. Clip segments, never clamp a surface's heights
//! or connect separate branches. Scientific samples remain untouched.
use crate::math_editor::view::layout::{Camera, Rect};

#[derive(Clone, Copy, Debug)]
pub(in crate::math_editor) struct Bounds {
    pub ranges: [[f64; 2]; 3],
}
impl Bounds {
    pub fn for_view(camera: Camera, rect: Rect) -> Option<Self> {
        let low = camera.unproject([rect.x, rect.y + rect.h], rect)?;
        let high = camera.unproject([rect.x + rect.w, rect.y], rect)?;
        Self::from_xy(low, high)
    }

    pub fn from_xy(
        low: sim_math::geometry::Point,
        high: sim_math::geometry::Point,
    ) -> Option<Self> {
        // A cube's projected radius is at most sqrt(3) times its half extent.
        // 0.28 of the shorter view span fits inside the canvas while orbiting.
        // This box is never drawn; zoom changes its mathematical extent.
        let half = (high.x - low.x).min(high.y - low.y) * 0.28;
        let x = low.x * 0.5 + high.x * 0.5;
        let y = low.y * 0.5 + high.y * 0.5;
        let ranges = [[x - half, x + half], [y - half, y + half], [-half, half]];
        ranges
            .iter()
            .all(|r| r[0].is_finite() && r[1].is_finite() && r[0] < r[1])
            .then_some(Self { ranges })
    }

    pub fn segment(&self, mut segment: [[f64; 3]; 2]) -> Option<[[f64; 3]; 2]> {
        if !segment.iter().flatten().all(|v| v.is_finite()) {
            return None;
        }
        for (axis, &range) in self.ranges.iter().enumerate() {
            let values = [segment[0][axis], segment[1][axis]];
            let (clipped, values) = slab(segment, values, range)?;
            segment = clipped;
            // Preserve the exact boundary coordinate, even when the parameter
            // of a very long segment is indistinguishable from 0, 1 or 0.5.
            segment[0][axis] = values[0];
            segment[1][axis] = values[1];
        }
        Some(segment)
    }

    pub fn corners(self) -> impl Iterator<Item = [f64; 3]> {
        (0..8).map(move |i| std::array::from_fn(|axis| self.ranges[axis][(i >> axis) & 1]))
    }
}

/// Clip against a scalar affine coordinate (an axis or camera depth). Scaling
/// before subtraction avoids overflow with finite, opposite-sign endpoints.
pub(in crate::math_editor) fn slab(
    mut segment: [[f64; 3]; 2],
    mut values: [f64; 2],
    range: [f64; 2],
) -> Option<([[f64; 3]; 2], [f64; 2])> {
    if !values.iter().chain(&range).all(|v| v.is_finite()) || range[0] > range[1] {
        return None;
    }
    for (side, bound) in range.into_iter().enumerate() {
        let outside = values.map(|v| if side == 0 { v < bound } else { v > bound });
        if outside == [true, true] {
            return None;
        }
        if outside[0] == outside[1] {
            continue;
        }
        let scale = values[0].abs().max(values[1].abs()).max(bound.abs());
        let a = values[0] / scale;
        let b = values[1] / scale;
        let t = ((bound / scale - a) / (b - a)).clamp(0.0, 1.0);
        if !t.is_finite() {
            return None;
        }
        let end = usize::from(outside[1]);
        segment[end] =
            std::array::from_fn(|axis| segment[0][axis] * (1.0 - t) + segment[1][axis] * t);
        values[end] = bound;
    }
    Some((segment, values))
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOX: Bounds = Bounds {
        ranges: [[-5.0, 5.0]; 3],
    };

    #[test]
    fn crossing_both_outside_ends_keeps_exact_boundary_intersections() {
        assert_eq!(
            BOX.segment([[1.0, 2.0, -100.0], [1.0, 2.0, 100.0]]),
            Some([[1.0, 2.0, -5.0], [1.0, 2.0, 5.0]])
        );
        for extent in [1e20, 1e100, f64::MAX] {
            let clipped = BOX
                .segment([[0.0, 0.0, -extent], [0.0, 0.0, extent]])
                .unwrap();
            assert_eq!(clipped, [[0.0, 0.0, -5.0], [0.0, 0.0, 5.0]]);
        }
    }

    #[test]
    fn parallel_outside_invalid_and_corner_cases_do_not_create_tails() {
        assert!(BOX.segment([[0.0, 0.0, 6.0], [1.0, 2.0, 6.0]]).is_none());
        assert!(BOX.segment([[f64::NAN, 0.0, 0.0], [0.0; 3]]).is_none());
        assert!(BOX.segment([[0.0; 3], [0.0, 0.0, f64::INFINITY]]).is_none());
        let diagonal = BOX.segment([[-10.0; 3], [10.0; 3]]).unwrap();
        assert!(diagonal[0].iter().all(|v| (v + 5.0).abs() < 1e-12));
        assert!(diagonal[1].iter().all(|v| (v - 5.0).abs() < 1e-12));
        let edge = [[-5.0, -5.0, 5.0], [5.0, -5.0, 5.0]];
        assert_eq!(BOX.segment(edge), Some(edge));
        let touch = BOX.segment([[-10.0, 0.0, 0.0], [0.0, 10.0, 0.0]]).unwrap();
        assert_eq!(touch, [[-5.0, 5.0, 0.0]; 2]);
    }

    #[test]
    fn clipping_is_contained_idempotent_and_reversible() {
        for i in 0..1000 {
            let a = [
                ((i * 7) % 43) as f64 - 21.0,
                ((i * 11) % 41) as f64 - 20.0,
                ((i * 13) % 47) as f64 - 23.0,
            ];
            let b = [
                ((i * 17) % 31) as f64 - 15.0,
                ((i * 19) % 37) as f64 - 18.0,
                ((i * 23) % 53) as f64 - 26.0,
            ];
            let forward = BOX.segment([a, b]);
            let backward = BOX.segment([b, a]);
            assert_eq!(forward.is_some(), backward.is_some());
            if let Some(c) = forward {
                assert!(
                    c.iter()
                        .flatten()
                        .all(|v| (-5.0000000001..=5.0000000001).contains(v))
                );
                let reverse = backward.unwrap();
                let again = BOX.segment(c).unwrap();
                for end in 0..2 {
                    for axis in 0..3 {
                        assert!((c[end][axis] - reverse[1 - end][axis]).abs() < 1e-12);
                        assert!((c[end][axis] - again[end][axis]).abs() < 1e-12);
                    }
                }
            }
        }
    }

    #[test]
    fn orbit_and_transition_do_not_change_volume_or_push_its_corners_offscreen() {
        use crate::math_editor::scene::Spatial;
        let rect = Rect::new(380.0, 66.0, 900.0, 620.0);
        let camera = Camera {
            x: 20.0,
            y: -8.0,
            scale: 55.0,
        };
        let bounds = Bounds::for_view(camera, rect).unwrap();
        for blend in [0.0, 0.2, 0.5, 0.9, 1.0] {
            for yaw in [-2.0, -0.5, 0.0, 1.0, 2.5] {
                for pitch in [-1.3, -0.4, 0.5, 1.3] {
                    let view = Spatial {
                        blend,
                        yaw,
                        pitch,
                        ..Default::default()
                    };
                    let projector = view.projector(camera, rect).unwrap();
                    for point in bounds.corners() {
                        assert!(rect.contains(projector.point(point).unwrap()));
                    }
                }
            }
        }
    }
}
