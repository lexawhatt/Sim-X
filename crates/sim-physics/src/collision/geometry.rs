use super::model::{BoundCollider, CollisionShape};
use crate::{BodyDesc, Vec2};

#[derive(Clone, Copy, Debug)]
pub(crate) struct Geometry {
    pub normal: Vec2,
    pub gap: f64,
}

pub(crate) fn contact(a: BoundCollider, b: BoundCollider, bodies: &[BodyDesc]) -> Geometry {
    between(a, bodies[a.index].position_m, b, bodies[b.index].position_m)
}

pub(crate) fn between(a: BoundCollider, pa: Vec2, b: BoundCollider, pb: Vec2) -> Geometry {
    match (a.desc.shape, b.desc.shape) {
        (CollisionShape::Circle { radius_m: ra }, CollisionShape::Circle { radius_m: rb }) => {
            let delta = pb - pa;
            let length = delta.length();
            Geometry {
                normal: if length > 0.0 {
                    delta * (1.0 / length)
                } else {
                    Vec2::new(1.0, 0.0)
                },
                gap: length - ra - rb,
            }
        }
        (CollisionShape::Circle { radius_m }, CollisionShape::Box { half_extents_m, .. }) => {
            circle_box(pa, radius_m, pb, half_extents_m, b.axes)
        }
        (CollisionShape::Box { half_extents_m, .. }, CollisionShape::Circle { radius_m }) => {
            let geometry = circle_box(pb, radius_m, pa, half_extents_m, a.axes);
            Geometry {
                normal: -geometry.normal,
                gap: geometry.gap,
            }
        }
        (CollisionShape::Box { .. }, CollisionShape::Box { .. }) => {
            let delta = pb - pa;
            let mut best = Geometry {
                normal: a.axes[0],
                gap: f64::NEG_INFINITY,
            };
            // Stable feature order also resolves equal-depth corner/face ties.
            for axis in a.axes.into_iter().chain(b.axes) {
                let projected = delta.dot(axis);
                let gap = projected.abs() - a.extent(axis) - b.extent(axis);
                if gap > best.gap {
                    best = Geometry {
                        normal: if projected < 0.0 { -axis } else { axis },
                        gap,
                    };
                }
            }
            best
        }
    }
}

fn circle_box(circle: Vec2, radius: f64, center: Vec2, half: Vec2, axes: [Vec2; 2]) -> Geometry {
    let delta = circle - center;
    let local = Vec2::new(delta.dot(axes[0]), delta.dot(axes[1]));
    let nearest = Vec2::new(
        local.x.clamp(-half.x, half.x),
        local.y.clamp(-half.y, half.y),
    );
    let outward = local - nearest;
    let distance = outward.length();
    if distance > 0.0 {
        return Geometry {
            normal: (axes[0] * outward.x + axes[1] * outward.y) * (-1.0 / distance),
            gap: distance - radius,
        };
    }
    let x = half.x - local.x.abs();
    let y = half.y - local.y.abs();
    let (axis, coordinate, face_gap) = if x <= y {
        (axes[0], local.x, x)
    } else {
        (axes[1], local.y, y)
    };
    Geometry {
        normal: axis * if coordinate < 0.0 { 1.0 } else { -1.0 },
        gap: -face_gap - radius,
    }
}
