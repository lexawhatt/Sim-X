use super::geometry;
use crate::{BodyDesc, BodyId, Error, Mobility, SolverConfig, Vec2};

/// Hard limit on finite shapes, checked before input traversal or allocation.
pub const MAX_COLLIDERS: usize = 128;
/// Hard limit on simultaneous active pairs per solve sweep and final report.
pub const MAX_CONTACTS: usize = 256;
/// At most two physical impulse points for each retained rigid contact pair.
pub const MAX_CONTACT_POINTS: usize = MAX_CONTACTS * 2;
/// Numerical settling regularization: approaching normal speeds below this
/// value use zero effective restitution. This is not a material measurement.
pub const RESTITUTION_SPEED_THRESHOLD_M_S: f64 = 0.2;

/// Finite body-local collision geometry, with dimensions in `1e-4..=1e6` metres.
/// A [`crate::RotationDesc`] rotates it; legacy bodies retain locked orientation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CollisionShape {
    /// A disk in the physical plane.
    Circle {
        /// Positive physical radius in metres.
        radius_m: f64,
    },
    /// A rectangle oriented relative to its body's angular frame.
    Box {
        /// Positive half-width and half-height in local physical axes.
        half_extents_m: Vec2,
        /// Local counterclockwise angle in radians, within `-1e6..=1e6`.
        /// With no angular state this is also its world orientation.
        angle_rad: f64,
    },
}

/// One optional finite shape per existing body. Bodies without one remain points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColliderDesc {
    /// Stable body identity; unique among all colliders in a world.
    pub body: BodyId,
    /// Finite shape centred at the body's physical position.
    pub shape: CollisionShape,
    /// Normal restitution in `[0,1]`; a pair uses the larger value.
    pub restitution: f64,
}

/// One committed contact pair; impulses never include position cleanup.
#[derive(Clone, Debug, PartialEq)]
pub struct ContactTelemetry {
    /// First body in stable identity order.
    pub a: BodyId,
    /// Second body in stable identity order.
    pub b: BodyId,
    /// Unit normal from A toward B at the committed contact geometry.
    pub normal_a_to_b: Vec2,
    /// Signed momentum change on A, in newton-seconds; B receives its opposite.
    pub impulse_on_a_ns: Vec2,
    /// Contact impulse divided by the full fixed step; not instantaneous force.
    pub average_force_on_a_n: Vec2,
    /// Signed final gap; negative means overlap within the accepted tolerance.
    pub gap_m: f64,
    /// Mixed material restitution; an established resting contact does not bounce.
    pub restitution: f64,
    /// Maximum actual point coefficient after resting/low-speed settling.
    pub effective_restitution: f64,
    /// At most two shared impulse points; empty on the legacy translation-only path.
    pub points: Vec<ContactPointTelemetry>,
    /// Total angular impulse on A; torque need not be opposite at distinct centres.
    pub angular_impulse_on_a_nms: f64,
    /// Total angular impulse on B about its own centre.
    pub angular_impulse_on_b_nms: f64,
}

/// A real shared contact point, with both endpoint angular impulses.
#[derive(Clone, Debug, PartialEq)]
pub struct ContactPointTelemetry {
    /// Shared physical impulse position in metres.
    pub position_m: Vec2,
    /// Signed local separation at this manifold point in metres.
    pub gap_m: f64,
    /// Actual coefficient at this point after resting/low-speed regularization.
    pub effective_restitution: f64,
    /// Linear impulse on A; B receives its negative.
    pub impulse_on_a_ns: Vec2,
    /// Angular impulse about A's centre in newton-metre-seconds.
    pub angular_impulse_on_a_nms: f64,
    /// Angular impulse about B's centre in newton-metre-seconds.
    pub angular_impulse_on_b_nms: f64,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct BoundCollider {
    pub desc: ColliderDesc,
    pub index: usize,
    pub axes: [Vec2; 2],
}

impl BoundCollider {
    pub fn feature(self) -> f64 {
        match self.desc.shape {
            CollisionShape::Circle { radius_m } => radius_m,
            CollisionShape::Box { half_extents_m, .. } => half_extents_m.x.min(half_extents_m.y),
        }
    }

    pub fn extent(self, axis: Vec2) -> f64 {
        match self.desc.shape {
            CollisionShape::Circle { radius_m } => radius_m,
            CollisionShape::Box { half_extents_m, .. } => {
                half_extents_m.x * axis.dot(self.axes[0]).abs()
                    + half_extents_m.y * axis.dot(self.axes[1]).abs()
            }
        }
    }
}

pub(crate) fn tolerance(a: BoundCollider, b: BoundCollider, config: SolverConfig) -> f64 {
    config.position_tolerance_m + config.relative_tolerance * a.feature().min(b.feature())
}

pub(crate) fn dynamic_pair(a: BoundCollider, b: BoundCollider, bodies: &[BodyDesc]) -> bool {
    bodies[a.index].mobility != Mobility::Fixed || bodies[b.index].mobility != Mobility::Fixed
}

pub(crate) fn valid_shape(shape: CollisionShape) -> bool {
    let dimension = |value: f64| (1e-4..=1e6).contains(&value);
    match shape {
        CollisionShape::Circle { radius_m } => dimension(radius_m),
        CollisionShape::Box {
            half_extents_m,
            angle_rad,
        } => {
            dimension(half_extents_m.x)
                && dimension(half_extents_m.y)
                && (-1e6..=1e6).contains(&angle_rad)
        }
    }
}

pub(crate) fn bind(
    bodies: &[BodyDesc],
    descriptions: &[ColliderDesc],
    config: SolverConfig,
    rotations: &crate::rigid::model::Rotations,
) -> Result<Vec<BoundCollider>, Error> {
    let mut descriptions = descriptions.to_vec();
    descriptions.sort_by_key(|collider| collider.body);
    for pair in descriptions.windows(2) {
        if pair[0].body == pair[1].body {
            return Err(Error::DuplicateCollider(pair[0].body));
        }
    }
    let mut result = Vec::with_capacity(descriptions.len());
    for desc in descriptions {
        let valid = valid_shape(desc.shape);
        if !valid || !(0.0..=1.0).contains(&desc.restitution) {
            return Err(Error::InvalidCollider(desc.body));
        }
        let index = bodies
            .binary_search_by_key(&desc.body, |body| body.id)
            .map_err(|_| Error::MissingBody(desc.body))?;
        let angle = match desc.shape {
            CollisionShape::Circle { .. } => 0.0,
            CollisionShape::Box { angle_rad, .. } => angle_rad,
        };
        let (sin, cos) = angle.sin_cos();
        result.push(BoundCollider {
            desc,
            index,
            axes: [Vec2::new(cos, sin), Vec2::new(-sin, cos)],
        });
    }
    let mut active = 0;
    let posed = crate::rigid::pose::colliders(&result, rotations);
    for (i, &a) in posed.iter().enumerate() {
        for &b in &posed[i + 1..] {
            if !dynamic_pair(a, b, bodies) {
                continue;
            }
            let gap = geometry::contact(a, b, bodies).gap;
            let allowed = tolerance(a, b, config);
            if gap < -allowed {
                return Err(Error::InitialOverlap(a.desc.body, b.desc.body));
            }
            if gap <= allowed {
                active += 1;
                if active > MAX_CONTACTS {
                    return Err(Error::ContactBudget);
                }
            }
        }
    }
    Ok(result)
}
