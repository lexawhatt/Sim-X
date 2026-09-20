use crate::numeric::bounded_vec;
use crate::{BodyId, Error, ForceSourceId, LinkId, Vec2};

/// Maximum bodies in a validated composition.
pub const MAX_BODIES: usize = 128;
/// Maximum persistent relationships in a validated composition.
pub const MAX_LINKS: usize = 256;
/// Maximum external force contributions in one atomic step.
pub const MAX_FORCES: usize = 1024;

/// Whether a body participates in translational integration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mobility {
    /// Finite positive mass responds to forces and constraints.
    Dynamic,
    /// Position is fixed, velocity must be zero; mass is never infinite.
    Fixed,
}

/// Canonical translational mass state. An optional separate [`crate::ColliderDesc`]
/// supplies finite geometry; without one this remains a non-colliding point mass.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyDesc {
    /// Stable identity.
    pub id: BodyId,
    /// Position in meters.
    pub position_m: Vec2,
    /// Velocity in meters per second.
    pub velocity_m_s: Vec2,
    /// Positive mass in kilograms, including for fixed bodies.
    pub mass_kg: f64,
    /// Dynamic or fixed integration behavior.
    pub mobility: Mobility,
}

impl BodyDesc {
    pub(crate) fn validate(self) -> Result<(), Error> {
        if !bounded_vec(self.position_m, 1e9)
            || !bounded_vec(self.velocity_m_s, 1e6)
            || !self.mass_kg.is_finite()
            || !(1e-6..=1e12).contains(&self.mass_kg)
        {
            return Err(Error::InvalidBody(self.id));
        }
        if self.mobility == Mobility::Fixed && self.velocity_m_s != Vec2::ZERO {
            return Err(Error::FixedVelocity(self.id));
        }
        Ok(())
    }

    pub(crate) fn inverse_mass(self) -> f64 {
        match self.mobility {
            Mobility::Dynamic => 1.0 / self.mass_kg,
            Mobility::Fixed => 0.0,
        }
    }
}

/// Typed, persistent center-to-center relationship. No named machine cases exist.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LinkDesc {
    /// Ideal massless bilateral rigid rod; supports tension and compression.
    Rod {
        /// Stable link identity.
        id: LinkId,
        /// First endpoint.
        a: BodyId,
        /// Second endpoint.
        b: BodyId,
        /// Required separation in meters.
        length_m: f64,
    },
    /// Ideal massless Hooke spring, without hidden spring damping.
    Spring {
        /// Stable link identity.
        id: LinkId,
        /// First endpoint.
        a: BodyId,
        /// Second endpoint.
        b: BodyId,
        /// Zero-force length in meters.
        rest_length_m: f64,
        /// Positive stiffness in newtons per meter.
        stiffness_n_m: f64,
    },
    /// Hooke spring between two body-local attachment points, with physical torques.
    AttachedSpring {
        /// Stable relationship identity.
        id: LinkId,
        /// First endpoint body.
        a: BodyId,
        /// Second endpoint body.
        b: BodyId,
        /// First body-local offset in metres, components bounded by one million.
        local_a_m: Vec2,
        /// Second body-local offset in metres, components bounded by one million.
        local_b_m: Vec2,
        /// Positive unstretched attachment separation in metres.
        rest_length_m: f64,
        /// Axial stiffness in newtons per metre.
        stiffness_n_m: f64,
    },
}

impl LinkDesc {
    /// Stable identity independent of endpoint order.
    pub const fn id(self) -> LinkId {
        match self {
            Self::Rod { id, .. } | Self::Spring { id, .. } | Self::AttachedSpring { id, .. } => id,
        }
    }
    /// The stable physical endpoints, not storage indices.
    pub const fn endpoints(self) -> (BodyId, BodyId) {
        match self {
            Self::Rod { a, b, .. }
            | Self::Spring { a, b, .. }
            | Self::AttachedSpring { a, b, .. } => (a, b),
        }
    }
}

/// One external force, held constant for exactly the submitted step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ForceInput {
    /// Dynamic target body.
    pub target: BodyId,
    /// One unique `(target, source)` per submitted step.
    pub source: ForceSourceId,
    /// Center-applied force in newtons.
    pub force_n: Vec2,
}
