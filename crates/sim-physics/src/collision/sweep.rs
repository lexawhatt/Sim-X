//! Conservative rejection of unsupported swept impacts; this is not TOI solving.

use super::{
    geometry,
    model::{self, BoundCollider, CollisionShape},
};
use crate::{BodyDesc, Error, SolverConfig, Vec2};

/// Checks piecewise linear translation of the complete candidate, including
/// solver correction batches. Far-separated fast bodies do not hit a speed cap.
pub(crate) fn guard(
    before: &[BodyDesc],
    after: &[BodyDesc],
    colliders: &[BoundCollider],
    config: SolverConfig,
) -> Result<(), Error> {
    for (index, &a) in colliders.iter().enumerate() {
        for &b in &colliders[index + 1..] {
            if !model::dynamic_pair(a, b, before) {
                continue;
            }
            let start = before[b.index].position_m - before[a.index].position_m;
            let end = after[b.index].position_m - after[a.index].position_m;
            let movement = end - start;
            let broad = [Vec2::new(1.0, 0.0), Vec2::new(0.0, 1.0)]
                .into_iter()
                .all(|axis| {
                    let radius = a.extent(axis) + b.extent(axis);
                    let x = start.dot(axis);
                    let y = end.dot(axis);
                    x.min(y) <= radius && x.max(y) >= -radius
                });
            if !broad {
                continue;
            }
            let tolerance = model::tolerance(a, b, config);
            if movement.length() > 0.25 * a.feature().min(b.feature()) {
                return Err(Error::CollisionMotionTooLarge(a.desc.body));
            }
            let old_gap = geometry::contact(a, b, before).gap;
            let new_gap = geometry::contact(a, b, after).gap;
            if old_gap >= -tolerance
                && new_gap > tolerance
                && swept_interior(a, b, start, end, tolerance)
            {
                return Err(Error::SweptCollisionUnsupported(a.desc.body, b.desc.body));
            }
        }
    }
    Ok(())
}

fn swept_interior(
    a: BoundCollider,
    b: BoundCollider,
    start: Vec2,
    end: Vec2,
    tolerance: f64,
) -> bool {
    match (a.desc.shape, b.desc.shape) {
        (CollisionShape::Circle { radius_m: ra }, CollisionShape::Circle { radius_m: rb }) => {
            point_segment(Vec2::ZERO, start, end) < ra + rb - tolerance
        }
        (CollisionShape::Box { .. }, CollisionShape::Box { .. }) => slabs(
            start,
            end,
            a.axes
                .into_iter()
                .chain(b.axes)
                .map(|axis| (axis, a.extent(axis) + b.extent(axis) - tolerance)),
        ),
        (CollisionShape::Circle { radius_m }, CollisionShape::Box { half_extents_m, .. }) => {
            let local = |point: Vec2| Vec2::new((-point).dot(b.axes[0]), (-point).dot(b.axes[1]));
            segment_box_distance(local(start), local(end), half_extents_m) < radius_m - tolerance
        }
        (CollisionShape::Box { half_extents_m, .. }, CollisionShape::Circle { radius_m }) => {
            let local = |point: Vec2| Vec2::new(point.dot(a.axes[0]), point.dot(a.axes[1]));
            segment_box_distance(local(start), local(end), half_extents_m) < radius_m - tolerance
        }
    }
}

fn slabs(start: Vec2, end: Vec2, axes: impl Iterator<Item = (Vec2, f64)>) -> bool {
    let delta = end - start;
    let mut entry: f64 = 0.0;
    let mut exit: f64 = 1.0;
    for (axis, extent) in axes {
        let position = start.dot(axis);
        let velocity = delta.dot(axis);
        if velocity == 0.0 {
            if position.abs() > extent {
                return false;
            }
        } else {
            let first = (-extent - position) / velocity;
            let last = (extent - position) / velocity;
            entry = entry.max(first.min(last));
            exit = exit.min(first.max(last));
            if entry > exit {
                return false;
            }
        }
    }
    entry <= exit
}

fn point_segment(point: Vec2, start: Vec2, end: Vec2) -> f64 {
    let delta = end - start;
    let squared = delta.dot(delta);
    if squared == 0.0 {
        return (point - start).length();
    }
    let t = ((point - start).dot(delta) / squared).clamp(0.0, 1.0);
    (point - (start + delta * t)).length()
}

fn segment_box_distance(start: Vec2, end: Vec2, half: Vec2) -> f64 {
    if slabs(
        start,
        end,
        [(Vec2::new(1.0, 0.0), half.x), (Vec2::new(0.0, 1.0), half.y)].into_iter(),
    ) {
        return 0.0;
    }
    let endpoint = |point: Vec2| {
        Vec2::new(
            (point.x.abs() - half.x).max(0.0),
            (point.y.abs() - half.y).max(0.0),
        )
        .length()
    };
    let mut best = endpoint(start).min(endpoint(end));
    for x in [-half.x, half.x] {
        for y in [-half.y, half.y] {
            best = best.min(point_segment(Vec2::new(x, y), start, end));
        }
    }
    best
}
