//! Bounded frictionless contact geometry: shared impulse points, never a force.
//!
//! Box faces use reference/incident clipping (Catto, GDC 2006). The incident
//! segment is clipped to the reference side planes, then its surviving points
//! are paired with their projections on the reference face. Their midpoints
//! are shared by both bodies, preserving angular impulse balance.

use crate::{
    BodyDesc, CollisionShape, Vec2,
    collision::{geometry, model::BoundCollider},
};

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ManifoldPoint {
    pub point_m: Vec2,
    pub gap_m: f64,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Manifold {
    pub normal: Vec2,
    pub points: [ManifoldPoint; 2],
    pub len: usize,
    pub gap: f64,
}

pub(crate) fn contacts(
    a: BoundCollider,
    b: BoundCollider,
    bodies: &[BodyDesc],
    tolerance: f64,
) -> Manifold {
    let geometry = geometry::contact(a, b, bodies);
    let mut result = Manifold {
        normal: geometry.normal,
        points: [ManifoldPoint::default(); 2],
        len: 0,
        gap: geometry.gap,
    };
    if geometry.gap > tolerance {
        return result;
    }
    let pa = bodies[a.index].position_m;
    let pb = bodies[b.index].position_m;
    // The circle support point and its opposing surface projection lie along
    // the contact normal, including the defined circle-inside-box convention.
    let point = match (a.desc.shape, b.desc.shape) {
        (CollisionShape::Circle { radius_m }, _) => {
            Some(pa + geometry.normal * (radius_m + geometry.gap * 0.5))
        }
        (_, CollisionShape::Circle { radius_m }) => {
            Some(pb - geometry.normal * (radius_m + geometry.gap * 0.5))
        }
        _ => None,
    };
    if let Some(point_m) = point {
        result.points[0] = ManifoldPoint {
            point_m,
            gap_m: geometry.gap,
        };
        result.len = 1;
    } else {
        box_points(a, pa, b, pb, tolerance, &mut result);
    }
    result
}

fn box_points(
    a: BoundCollider,
    pa: Vec2,
    b: BoundCollider,
    pb: Vec2,
    tolerance: f64,
    result: &mut Manifold,
) {
    // Match the narrow phase's exact feature order and tie policy. Axes are
    // already in world space; desc.angle_rad is NOT the body's current angle.
    let delta = pb - pa;
    let mut best_gap = f64::NEG_INFINITY;
    let mut feature = 0;
    for (index, axis) in a.axes.into_iter().chain(b.axes).enumerate() {
        let gap = delta.dot(axis).abs() - a.extent(axis) - b.extent(axis);
        if gap > best_gap {
            best_gap = gap;
            feature = index;
        }
    }
    let (reference, center, incident, incident_center, normal) = if feature < 2 {
        (a, pa, b, pb, result.normal)
    } else {
        (b, pb, a, pa, -result.normal)
    };
    let axis_index = feature % 2;
    let tangent = reference.axes[1 - axis_index];
    let half_normal = reference.extent(normal);
    let half_side = reference.extent(tangent);
    let incident_axis = if normal.dot(incident.axes[0]).abs() >= normal.dot(incident.axes[1]).abs()
    {
        0
    } else {
        1
    };
    let face_normal = incident.axes[incident_axis]
        * if normal.dot(incident.axes[incident_axis]) < 0.0 {
            1.0
        } else {
            -1.0
        };
    // Subtract centres before constructing endpoints to avoid cancellation
    // from taking dot products of two large absolute world positions.
    let face_center = incident_center - center + face_normal * incident.extent(face_normal);
    let face_side =
        incident.axes[1 - incident_axis] * incident.extent(incident.axes[1 - incident_axis]);
    let start = face_center - face_side;
    let end = face_center + face_side;
    let Some((lower, upper)) = clip_side(start.dot(tangent), end.dot(tangent), half_side) else {
        return;
    };
    for weight in [lower, upper] {
        let relative = start + (end - start) * weight;
        let gap = relative.dot(normal) - half_normal;
        if gap > tolerance {
            continue;
        }
        let point_m = center + relative - normal * (gap * 0.5);
        if result.len > 0 && result.points[0].point_m == point_m {
            continue;
        }
        result.points[result.len] = ManifoldPoint {
            point_m,
            gap_m: gap,
        };
        result.len += 1;
    }
}

/// Intersects a segment parameter with a closed tangent slab. Exact tangency
/// retains one point; no fabricated fallback contact or heap work is needed.
fn clip_side(start: f64, end: f64, half: f64) -> Option<(f64, f64)> {
    let delta = end - start;
    if delta == 0.0 {
        return (start.abs() <= half).then_some((0.0, 1.0));
    }
    let first = (-half - start) / delta;
    let second = (half - start) / delta;
    let lower = first.min(second).max(0.0);
    let upper = first.max(second).min(1.0);
    (lower <= upper).then_some((lower, upper))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BodyId, ColliderDesc, Mobility};

    fn shape(index: usize, center: Vec2, half: Vec2, angle: f64) -> (BodyDesc, BoundCollider) {
        let id = BodyId::new(index as u64 + 1).unwrap();
        let body = BodyDesc {
            id,
            position_m: center,
            velocity_m_s: Vec2::ZERO,
            mass_kg: 1.0,
            mobility: Mobility::Dynamic,
        };
        let (sin, cos) = angle.sin_cos();
        let collider = BoundCollider {
            index,
            desc: ColliderDesc {
                body: id,
                restitution: 0.0,
                shape: CollisionShape::Box {
                    half_extents_m: half,
                    angle_rad: 0.0,
                },
            },
            axes: [Vec2::new(cos, sin), Vec2::new(-sin, cos)],
        };
        (body, collider)
    }

    #[test]
    fn flat_face_retains_two_shared_surface_midpoints() {
        let (a, ca) = shape(0, Vec2::ZERO, Vec2::new(2.0, 1.0), 0.0);
        let (b, cb) = shape(1, Vec2::new(0.5, 1.9), Vec2::new(1.0, 1.0), 0.0);
        let m = contacts(ca, cb, &[a, b], 1e-9);
        assert_eq!(m.len, 2);
        assert_eq!(m.normal, Vec2::new(0.0, 1.0));
        assert!((m.gap + 0.1).abs() < 1e-12);
        for p in m.points {
            assert!((p.point_m.y - 0.95).abs() < 1e-12);
            assert!((p.gap_m + 0.1).abs() < 1e-12);
        }
        let mut x = m.points.map(|p| p.point_m.x);
        x.sort_by(f64::total_cmp);
        assert_eq!(x, [-0.5, 1.5]);
    }

    #[test]
    fn corner_contact_has_one_point_and_swapping_bodies_preserves_it() {
        let (a, ca) = shape(0, Vec2::ZERO, Vec2::new(3.0, 1.0), 0.0);
        let (b, cb) = shape(
            1,
            Vec2::new(0.0, 1.0 + 2.0_f64.sqrt()),
            Vec2::new(1.0, 1.0),
            std::f64::consts::FRAC_PI_4,
        );
        let m = contacts(ca, cb, &[a, b], 1e-9);
        let reversed = contacts(cb, ca, &[a, b], 1e-9);
        assert_eq!(m.len, 1);
        assert_eq!(reversed.len, 1);
        assert!((m.points[0].point_m - Vec2::new(0.0, 1.0)).length() < 1e-12);
        assert!((reversed.points[0].point_m - m.points[0].point_m).length() < 1e-12);
        assert!((m.normal + reversed.normal).length() < 1e-12);
    }

    #[test]
    fn world_rotation_is_taken_from_axes_not_authored_shape_angle() {
        let theta: f64 = 0.6;
        let rotate = |p: Vec2| {
            Vec2::new(
                p.x * theta.cos() - p.y * theta.sin(),
                p.x * theta.sin() + p.y * theta.cos(),
            )
        };
        let origin = Vec2::new(100.0, -200.0);
        let (a, ca) = shape(0, origin, Vec2::new(2.0, 1.0), theta);
        let (b, cb) = shape(
            1,
            origin + rotate(Vec2::new(0.5, 2.0)),
            Vec2::new(1.0, 1.0),
            theta,
        );
        let m = contacts(ca, cb, &[a, b], 1e-9);
        assert_eq!(m.len, 2);
        assert!((m.normal - rotate(Vec2::new(0.0, 1.0))).length() < 1e-12);
        for p in m.points {
            assert!(((p.point_m - origin).dot(rotate(Vec2::new(0.0, 1.0))) - 1.0).abs() < 1e-12);
        }
    }

    #[test]
    fn circles_share_the_midpoint_between_opposing_surfaces() {
        let (a, mut ca) = shape(0, Vec2::ZERO, Vec2::new(1.0, 1.0), 0.0);
        let (b, mut cb) = shape(1, Vec2::new(2.9, 0.0), Vec2::new(1.0, 1.0), 0.0);
        ca.desc.shape = CollisionShape::Circle { radius_m: 1.0 };
        cb.desc.shape = CollisionShape::Circle { radius_m: 2.0 };
        let m = contacts(ca, cb, &[a, b], 1e-9);
        assert_eq!(m.len, 1);
        assert!((m.points[0].point_m.x - 0.95).abs() < 1e-12);
        assert_eq!(m.points[0].point_m.y, 0.0);
    }

    #[test]
    fn separated_boxes_and_exact_tangent_clipping_are_bounded() {
        let (a, ca) = shape(0, Vec2::ZERO, Vec2::new(1.0, 1.0), 0.0);
        let (b, cb) = shape(1, Vec2::new(4.0, 0.0), Vec2::new(1.0, 1.0), 0.0);
        assert_eq!(contacts(ca, cb, &[a, b], 1e-9).len, 0);
        assert_eq!(clip_side(2.0, 1.0, 1.0), Some((1.0, 1.0)));
        assert_eq!(clip_side(2.0, 2.0, 1.0), None);
        assert_eq!(clip_side(-2.0, 2.0, 1.0), Some((0.25, 0.75)));
    }
}
