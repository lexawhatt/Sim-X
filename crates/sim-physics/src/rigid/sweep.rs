//! Bounded conservative guard for linear translation with interpolated angle.
//! This rejects unsupported missed impacts; it does not implement time of impact.

use super::{model::Rotations, pose};
use crate::{
    BodyDesc, CollisionShape, Error, SolverConfig, Vec2,
    collision::{
        geometry,
        model::{self, BoundCollider},
        sweep,
    },
};

const MAX_ANGLE_PER_SEGMENT_RAD: f64 = 0.05;
const MAX_INTERVAL_DEPTH: usize = 32;
const MAX_INTERVAL_VISITS: usize = 128;
const MAX_GUARD_INTERVAL_VISITS: usize = 4096;

pub(crate) fn guard(
    before: &[BodyDesc],
    before_rot: &Rotations,
    after: &[BodyDesc],
    after_rot: &Rotations,
    base_colliders: &[BoundCollider],
    solver: SolverConfig,
) -> Result<(), Error> {
    let mut interval_visits = 0;
    for (index, &a) in base_colliders.iter().enumerate() {
        for &b in &base_colliders[index + 1..] {
            if !model::dynamic_pair(a, b, before) {
                continue;
            }
            let angle_a = before_rot.angle(a.index);
            let angle_b = before_rot.angle(b.index);
            let delta_a = shape_rotation(a, after_rot.angle(a.index) - angle_a);
            let delta_b = shape_rotation(b, after_rot.angle(b.index) - angle_b);
            let world_a = orient(a, angle_a);
            let world_b = orient(b, angle_b);
            if delta_a == 0.0 && delta_b == 0.0 {
                sweep::guard(before, after, &[world_a, world_b], solver)?;
                continue;
            }
            let start = before[b.index].position_m - before[a.index].position_m;
            let end = after[b.index].position_m - after[a.index].position_m;
            let travel = radius(a) * delta_a.abs() + radius(b) * delta_b.abs();
            let broad = [Vec2::new(1.0, 0.0), Vec2::new(0.0, 1.0)]
                .into_iter()
                .all(|axis| {
                    let extent = world_a.extent(axis) + world_b.extent(axis) + travel;
                    let x = start.dot(axis);
                    let y = end.dot(axis);
                    x.min(y) <= extent && x.max(y) >= -extent
                });
            if !broad {
                continue;
            }
            if delta_a.abs() > MAX_ANGLE_PER_SEGMENT_RAD {
                return Err(Error::AngularMotionTooLarge(a.desc.body));
            }
            if delta_b.abs() > MAX_ANGLE_PER_SEGMENT_RAD {
                return Err(Error::AngularMotionTooLarge(b.desc.body));
            }
            let motion = (end - start).length() + travel;
            if motion > 0.25 * a.feature().min(b.feature()) {
                return Err(Error::AngularMotionTooLarge(if delta_a != 0.0 {
                    a.desc.body
                } else {
                    b.desc.body
                }));
            }
            let tolerance = model::tolerance(a, b, solver);
            let old_gap = geometry::between(world_a, Vec2::ZERO, world_b, start).gap;
            let new_gap = geometry::between(
                orient(a, angle_a + delta_a),
                Vec2::ZERO,
                orient(b, angle_b + delta_b),
                end,
            )
            .gap;
            // Endpoint contact is resolved discretely. A penetration-cleanup
            // batch may also start inside; do not reclassify that as tunnelling.
            if new_gap <= tolerance || old_gap < -tolerance {
                continue;
            }
            let path = Path {
                a,
                b,
                start,
                end,
                angle_a,
                angle_b,
                delta_a,
                delta_b,
                motion,
                tolerance,
            };
            path.certify(&mut interval_visits)?;
        }
    }
    Ok(())
}

fn shape_rotation(collider: BoundCollider, angle: f64) -> f64 {
    match collider.desc.shape {
        CollisionShape::Circle { .. } => 0.0,
        CollisionShape::Box { .. } => angle,
    }
}

fn radius(collider: BoundCollider) -> f64 {
    match collider.desc.shape {
        CollisionShape::Circle { radius_m } => radius_m,
        CollisionShape::Box { half_extents_m, .. } => half_extents_m.length(),
    }
}

fn orient(collider: BoundCollider, angle: f64) -> BoundCollider {
    BoundCollider {
        axes: collider.axes.map(|axis| pose::rotate(axis, angle)),
        ..collider
    }
}

struct Path {
    a: BoundCollider,
    b: BoundCollider,
    start: Vec2,
    end: Vec2,
    angle_a: f64,
    angle_b: f64,
    delta_a: f64,
    delta_b: f64,
    motion: f64,
    tolerance: f64,
}

impl Path {
    fn gap(&self, t: f64) -> f64 {
        geometry::between(
            orient(self.a, self.angle_a + self.delta_a * t),
            Vec2::ZERO,
            orient(self.b, self.angle_b + self.delta_b * t),
            self.start + (self.end - self.start) * t,
        )
        .gap
    }

    fn certify(&self, guard_visits: &mut usize) -> Result<(), Error> {
        // Depth-first subdivision has at most depth+1 pending intervals.
        let mut pending = [(0.0, 0.0, 0_usize); MAX_INTERVAL_DEPTH + 1];
        pending[0] = (0.0, 1.0, 0);
        let mut len = 1;
        let mut visited = 0;
        // Account conservatively for coordinate and trigonometric roundoff;
        // large/poorly conditioned poses can reject instead of falsely certify.
        let roundoff = 32.0
            * f64::EPSILON
            * (1.0
                + self.start.length()
                + self.end.length()
                + radius(self.a) * (1.0 + self.angle_a.abs())
                + radius(self.b) * (1.0 + self.angle_b.abs()));
        while len > 0 {
            if visited == MAX_INTERVAL_VISITS || *guard_visits == MAX_GUARD_INTERVAL_VISITS {
                return Err(Error::RotationalSweepUnsupported(
                    self.a.desc.body,
                    self.b.desc.body,
                ));
            }
            visited += 1;
            *guard_visits += 1;
            len -= 1;
            let (lower, upper, depth) = pending[len];
            let middle = (lower + upper) * 0.5;
            let gap = self.gap(middle);
            if gap < -self.tolerance - roundoff {
                return Err(Error::SweptCollisionUnsupported(
                    self.a.desc.body,
                    self.b.desc.body,
                ));
            }
            // Each shape moves at most R*|delta angle| plus relative translation.
            // A separating axis at the midpoint remains separating throughout
            // this interval when that distance bound cannot close its gap.
            if gap - self.motion * (upper - lower) * 0.5 - roundoff >= -self.tolerance {
                continue;
            }
            if depth == MAX_INTERVAL_DEPTH {
                return Err(Error::RotationalSweepUnsupported(
                    self.a.desc.body,
                    self.b.desc.body,
                ));
            }
            pending[len] = (middle, upper, depth + 1);
            pending[len + 1] = (lower, middle, depth + 1);
            len += 2;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BodyId, ColliderDesc, Mobility, RotationDesc};

    #[test]
    fn aggregate_interval_budget_rejects_before_more_geometry_work() {
        let (_, _, colliders) = pair(2.0, 0.1, 0.0);
        let path = Path {
            a: colliders[0],
            b: colliders[1],
            start: Vec2::new(2.0, 0.0),
            end: Vec2::new(2.0, 0.0),
            angle_a: 0.0,
            angle_b: 0.0,
            delta_a: 1e-4,
            delta_b: 0.0,
            motion: 1e-3,
            tolerance: 1e-9,
        };
        let mut work = MAX_GUARD_INTERVAL_VISITS;
        assert!(matches!(
            path.certify(&mut work),
            Err(Error::RotationalSweepUnsupported(..))
        ));
        assert_eq!(work, MAX_GUARD_INTERVAL_VISITS);
    }

    fn pair(
        distance: f64,
        circle_radius: f64,
        angle: f64,
    ) -> (Vec<BodyDesc>, Rotations, Vec<BoundCollider>) {
        let first = BodyId::new(1).unwrap();
        let second = BodyId::new(2).unwrap();
        let bodies = vec![
            BodyDesc {
                id: first,
                position_m: Vec2::ZERO,
                velocity_m_s: Vec2::ZERO,
                mass_kg: 1.0,
                mobility: Mobility::Dynamic,
            },
            BodyDesc {
                id: second,
                position_m: Vec2::new(distance / 2.0_f64.sqrt(), distance / 2.0_f64.sqrt()),
                velocity_m_s: Vec2::ZERO,
                mass_kg: 1.0,
                mobility: Mobility::Fixed,
            },
        ];
        let colliders = vec![
            BoundCollider {
                index: 0,
                axes: [Vec2::new(1.0, 0.0), Vec2::new(0.0, 1.0)],
                desc: ColliderDesc {
                    body: first,
                    shape: CollisionShape::Box {
                        half_extents_m: Vec2::new(1.0, 1.0),
                        angle_rad: 0.0,
                    },
                    restitution: 0.0,
                },
            },
            BoundCollider {
                index: 1,
                axes: [Vec2::new(1.0, 0.0), Vec2::new(0.0, 1.0)],
                desc: ColliderDesc {
                    body: second,
                    shape: CollisionShape::Circle {
                        radius_m: circle_radius,
                    },
                    restitution: 0.0,
                },
            },
        ];
        let rotation = RotationDesc {
            body: first,
            angle_rad: angle,
            angular_velocity_rad_s: 0.0,
            inertia_kg_m2: 2.0 / 3.0,
        };
        let rotations = Rotations::bind(&bodies, &[rotation]).unwrap();
        (bodies, rotations, colliders)
    }

    #[test]
    fn tiny_rotational_grazing_contact_cannot_hide_between_separated_endpoints() {
        let (bodies, before, colliders) = pair(2.0_f64.sqrt() + 0.001 - 5e-8, 0.001, -1e-5);
        let mut after = before.clone();
        after.states[0].angle_rad = 1e-5;
        for state in [&before, &after] {
            let shaped = pose::colliders(&colliders, state);
            assert!(geometry::contact(shaped[0], shaped[1], &bodies).gap > 1e-9);
        }
        assert!(matches!(
            guard(
                &bodies,
                &before,
                &bodies,
                &after,
                &colliders,
                SolverConfig::default()
            ),
            Err(Error::SweptCollisionUnsupported(..))
        ));
    }

    #[test]
    fn truly_separated_rotation_is_certified_and_distant_rotation_is_not_speed_capped() {
        let (bodies, before, colliders) = pair(2.0_f64.sqrt() + 0.001 + 1e-5, 0.001, -1e-5);
        let mut after = before.clone();
        after.states[0].angle_rad = 1e-5;
        guard(
            &bodies,
            &before,
            &bodies,
            &after,
            &colliders,
            SolverConfig::default(),
        )
        .unwrap();
        let (bodies, before, colliders) = pair(1e5, 0.001, 0.0);
        let mut after = before.clone();
        after.states[0].angle_rad = 1.0;
        guard(
            &bodies,
            &before,
            &bodies,
            &after,
            &colliders,
            SolverConfig::default(),
        )
        .unwrap();
    }
}
