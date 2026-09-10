use std::{error::Error, fmt};

use crate::foundation::{EntityId, IdError, QuantityError};

use super::ForceSourceId;

/// Monotonic counter that could not be advanced atomically.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CounterKind {
    /// Successfully committed physics-step counter.
    Step,
    /// Canonical world-state revision counter.
    WorldRevision,
}

/// Exact checked arithmetic phase that produced a non-finite result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArithmeticOperation {
    /// Ordered force accumulation for one body.
    ForceReduction,
    /// Division of force by mass.
    Acceleration,
    /// Multiplication of acceleration by the time step.
    VelocityDelta,
    /// Addition of the velocity delta to current velocity.
    Velocity,
    /// Multiplication of next velocity by the time step.
    PositionDelta,
    /// Addition of the position delta to current position.
    Position,
    /// Addition of a fixed step to elapsed simulation time.
    ElapsedTime,
}

/// Structured failure returned by the Mechanics domain boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MechanicsError {
    /// A foundation quantity failed validation before entering canonical state.
    InvalidQuantity(QuantityError),
    /// A stable identity failed validation.
    InvalidIdentity(IdError),
    /// The bounded world cannot accept another body.
    EntityCapacityReached {
        /// Maximum number of bodies allowed in one Mechanics world.
        capacity: usize,
    },
    /// The monotonically increasing entity allocator cannot produce another ID.
    EntityIdExhausted,
    /// A commit counter cannot advance without overflow.
    CounterExhausted {
        /// Monotonic counter that could not be incremented.
        counter: CounterKind,
    },
    /// A contribution names an entity that does not exist.
    MissingForceTarget {
        /// Stable identity named by the rejected force contribution.
        target: EntityId,
    },
    /// An Editor setup command names a body that does not exist.
    MissingBody {
        /// Stable identity named by the rejected Editor command.
        entity: EntityId,
    },
    /// An ordinary force cannot bind to a fixed body.
    ForceTargetIsFixed {
        /// Fixed body that cannot accept an ordinary force capability.
        target: EntityId,
    },
    /// Fixed mobility requires exact zero canonical velocity.
    FixedBodyHasVelocity,
    /// Two contributions use the same semantic source for the same target.
    DuplicateForceSource {
        /// Body receiving both contributions.
        target: EntityId,
        /// Repeated semantic source identity.
        source: ForceSourceId,
    },
    /// A step input exceeds the hard bounded-work contribution limit.
    ContributionBudgetExceeded {
        /// Number of contributions supplied by the rejected call.
        submitted: usize,
        /// Hard per-step contribution limit.
        maximum: usize,
    },
    /// Checked domain arithmetic produced NaN or infinity.
    NonFiniteArithmetic {
        /// Formula stage that produced NaN or infinity.
        operation: ArithmeticOperation,
        /// Affected body, or `None` for world-level arithmetic.
        entity: Option<EntityId>,
    },
    /// A non-zero physical change was completely lost to `f64` rounding.
    PrecisionLoss {
        /// Formula stage whose non-zero progress became unrepresentable.
        operation: ArithmeticOperation,
        /// Affected body, or `None` for world-level arithmetic.
        entity: Option<EntityId>,
    },
}

impl From<QuantityError> for MechanicsError {
    fn from(error: QuantityError) -> Self {
        Self::InvalidQuantity(error)
    }
}

impl From<IdError> for MechanicsError {
    fn from(error: IdError) -> Self {
        Self::InvalidIdentity(error)
    }
}

impl fmt::Display for MechanicsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidQuantity(error) => error.fmt(formatter),
            Self::InvalidIdentity(error) => error.fmt(formatter),
            Self::EntityCapacityReached { capacity } => {
                write!(
                    formatter,
                    "mechanics body capacity of {capacity} was reached"
                )
            }
            Self::EntityIdExhausted => formatter.write_str("entity identifier space is exhausted"),
            Self::CounterExhausted { counter } => {
                write!(formatter, "{counter:?} counter is exhausted")
            }
            Self::MissingForceTarget { target } => {
                write!(formatter, "force target {} does not exist", target.get())
            }
            Self::MissingBody { entity } => {
                write!(formatter, "mechanics body {} does not exist", entity.get())
            }
            Self::ForceTargetIsFixed { target } => {
                write!(
                    formatter,
                    "fixed body {} cannot receive ordinary force",
                    target.get()
                )
            }
            Self::FixedBodyHasVelocity => {
                formatter.write_str("fixed bodies must have zero linear velocity")
            }
            Self::DuplicateForceSource { target, source } => write!(
                formatter,
                "force source {} was submitted more than once for body {}",
                source.get(),
                target.get()
            ),
            Self::ContributionBudgetExceeded { submitted, maximum } => write!(
                formatter,
                "step submitted {submitted} force contributions, exceeding the limit of {maximum}"
            ),
            Self::NonFiniteArithmetic { operation, entity } => match entity {
                Some(entity) => write!(
                    formatter,
                    "{operation:?} produced non-finite state for body {}",
                    entity.get()
                ),
                None => write!(formatter, "{operation:?} produced a non-finite value"),
            },
            Self::PrecisionLoss { operation, entity } => match entity {
                Some(entity) => write!(
                    formatter,
                    "{operation:?} lost all representable progress for body {}",
                    entity.get()
                ),
                None => write!(formatter, "{operation:?} lost all representable progress"),
            },
        }
    }
}

impl Error for MechanicsError {}
