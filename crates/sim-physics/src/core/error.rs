use crate::{BodyId, ForceSourceId, LinkId};

/// Physical calculation phase associated with a numerical rejection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NumericStage {
    /// Exact force reduction.
    ForceReduction,
    /// Velocity update.
    Velocity,
    /// Angular velocity integration or impulse.
    AngularVelocity,
    /// Body angle drift or angular position correction.
    Orientation,
    /// Position update.
    Position,
    /// Physical time advancement.
    Time,
    /// Constraint solve.
    Constraint,
    /// Diagnostic energy or impulse construction.
    Telemetry,
}

/// Structured atomic rejection. Display text is not the error identity.
#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    /// Stable IDs cannot be zero.
    ZeroId,
    /// Body slice exceeded its hard budget before traversal.
    BodyBudget,
    /// Link slice exceeded its hard budget before traversal.
    LinkBudget,
    /// External force slice exceeded its hard budget before traversal.
    ForceBudget,
    /// Finite-shape input exceeds the pre-traversal limit.
    ColliderBudget,
    /// Angular descriptions exceed the pre-traversal body-count limit.
    RotationBudget,
    /// Angular state or planar inertia is invalid for this body.
    InvalidRotation(BodyId),
    /// A body has more than one angular state.
    DuplicateRotation(BodyId),
    /// A noncentral spring attachment has no explicit angular body state.
    MissingRotation(BodyId),
    /// A spring's local attachment is nonfinite or outside the supported range.
    InvalidAttachment(LinkId),
    /// Angular motion exceeds an attached-spring or potential contact step bound.
    AngularMotionTooLarge(BodyId),
    /// A rotational swept path cannot be proven safe by this discrete solver.
    RotationalSweepUnsupported(BodyId, BodyId),
    /// More than one collider targets this body.
    DuplicateCollider(BodyId),
    /// Shape dimensions, angle or restitution are unsupported.
    InvalidCollider(BodyId),
    /// Dynamic collision geometry initially overlaps beyond tolerance.
    InitialOverlap(BodyId, BodyId),
    /// Active unique contact pairs exceed the bounded solver/report budget.
    ContactBudget,
    /// Final contact gap or normal velocity failed its acceptance tolerance.
    ContactFailure(BodyId, BodyId),
    /// A potentially interacting shape translated too far for a discrete contact step.
    CollisionMotionTooLarge(BodyId),
    /// A swept impact would be missed by endpoint-only discrete resolution.
    SweptCollisionUnsupported(BodyId, BodyId),
    /// The coupled passive contact solve increased kinetic energy beyond tolerance.
    ContactEnergyIncrease,
    /// Body ID occurs more than once.
    DuplicateBody(BodyId),
    /// Link ID occurs more than once.
    DuplicateLink(LinkId),
    /// Two rods duplicate the same unordered endpoint pair.
    DuplicateRod(LinkId),
    /// Link or force references a missing body.
    MissingBody(BodyId),
    /// A body's finite values or operating ranges are invalid.
    InvalidBody(BodyId),
    /// A fixed body carries nonzero velocity.
    FixedVelocity(BodyId),
    /// Invalid relationship parameters or endpoints.
    InvalidLink(LinkId),
    /// Initial rod geometry or velocity is inconsistent with its constraint.
    InitialConstraint(LinkId),
    /// A spring frequency is too high for this world's fixed time step.
    SpringStepTooLarge(LinkId),
    /// Scene settings are outside their supported numeric ranges.
    InvalidSettings,
    /// Numerical policy is outside its supported ranges.
    InvalidSolver,
    /// External force is not finite.
    InvalidForce(BodyId),
    /// Ordinary external force targeted a fixed body.
    FixedForce(BodyId),
    /// One target/source pair was submitted twice.
    DuplicateForce(BodyId, ForceSourceId),
    /// Finite arithmetic exceeded supported ranges or became non-finite.
    Numeric(NumericStage),
    /// A nonzero physical increment lost all representable progress.
    PrecisionLoss(NumericStage),
    /// A rod failed the finite, nonsingular or final tolerance requirement.
    ConstraintFailure(LinkId),
    /// A counter would exceed its integer range.
    CounterExhausted,
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InitialOverlap(a, b) => write!(formatter, "Objects {} and {} overlap; separate them before Run.", a.get(), b.get()),
            Self::CollisionMotionTooLarge(body) => write!(formatter, "Body {} moves too far for this contact step; lower speed/force or use a smaller time step.", body.get()),
            Self::SweptCollisionUnsupported(a, b) => write!(formatter, "Objects {} and {} cross between contact samples; a smaller time step is required.", a.get(), b.get()),
            Self::ContactFailure(a, b) => write!(formatter, "Contact between {} and {} did not converge; simplify the constraint graph or reduce the time step.", a.get(), b.get()),
            Self::ContactEnergyIncrease => formatter.write_str("Coupled contacts would create kinetic energy; this simultaneous impact is unsupported at the current solver settings."),
            Self::ContactBudget => formatter.write_str("Contact limit reached: at most 256 simultaneous contact pairs are supported."),
            Self::ColliderBudget => formatter.write_str("Collider limit reached: at most 128 finite shapes are supported."),
            Self::InvalidCollider(body) => write!(formatter, "Object {} has invalid collision dimensions, orientation or restitution.", body.get()),
            Self::DuplicateCollider(body) => write!(formatter, "Object {} has more than one collider.", body.get()),
            Self::RotationBudget => formatter.write_str("Angular body limit reached: at most 128 rotation components are supported."),
            Self::InvalidRotation(body) => write!(formatter, "Object {} has invalid angle, angular speed or moment of inertia; fixed bodies must have zero spin.", body.get()),
            Self::DuplicateRotation(body) => write!(formatter, "Object {} has more than one rotation component.", body.get()),
            Self::MissingRotation(body) => write!(formatter, "Object {} needs an explicit rotation component for an off-center spring attachment.", body.get()),
            Self::InvalidAttachment(link) => write!(formatter, "Spring {} has an invalid body-local attachment point.", link.get()),
            Self::AngularMotionTooLarge(body) => write!(formatter, "Object {} rotates too far for an attached spring or contact step; lower spin/torque or use a smaller time step.", body.get()),
            Self::RotationalSweepUnsupported(a, b) => write!(formatter, "The rotating path between objects {} and {} cannot be resolved safely; use a smaller time step.", a.get(), b.get()),
            _ => write!(formatter, "physics rejected transaction: {self:?}"),
        }
    }
}
impl std::error::Error for Error {}
