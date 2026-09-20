use super::model::Rotations;
use crate::collision::model::BoundCollider;
use crate::{BodyDesc, LinkDesc, Vec2};

pub(crate) fn rotate(value: Vec2, angle: f64) -> Vec2 {
    if angle == 0.0 {
        return value;
    }
    let (sin, cos) = angle.sin_cos();
    Vec2::new(cos * value.x - sin * value.y, sin * value.x + cos * value.y)
}

pub(crate) fn cross(a: Vec2, b: Vec2) -> f64 {
    a.x * b.y - a.y * b.x
}
pub(crate) fn spin(omega: f64, radius: Vec2) -> Vec2 {
    Vec2::new(-omega * radius.y, omega * radius.x)
}

pub(crate) fn offsets(link: LinkDesc) -> (Vec2, Vec2) {
    match link {
        LinkDesc::AttachedSpring {
            local_a_m,
            local_b_m,
            ..
        } => (local_a_m, local_b_m),
        _ => (Vec2::ZERO, Vec2::ZERO),
    }
}

pub(crate) fn attachment(
    bodies: &[BodyDesc],
    rotations: &Rotations,
    index: usize,
    local: Vec2,
) -> Vec2 {
    if local == Vec2::ZERO {
        return bodies[index].position_m;
    }
    bodies[index].position_m + rotate(local, rotations.angle(index))
}

pub(crate) fn velocity(
    bodies: &[BodyDesc],
    rotations: &Rotations,
    index: usize,
    radius: Vec2,
) -> Vec2 {
    bodies[index].velocity_m_s + spin(rotations.omega(index), radius)
}

pub(crate) fn colliders(input: &[BoundCollider], rotations: &Rotations) -> Vec<BoundCollider> {
    input
        .iter()
        .map(|&value| collider(value, rotations))
        .collect()
}

pub(crate) fn collider(value: BoundCollider, rotations: &Rotations) -> BoundCollider {
    BoundCollider {
        axes: value
            .axes
            .map(|axis| rotate(axis, rotations.angle(value.index))),
        ..value
    }
}
