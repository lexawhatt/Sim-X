use std::{error::Error, fmt};

use crate::foundation::EntityId;

use super::{ThermalLinkId, ThermalQuantityError};

/// Named arithmetic stage used by structured thermodynamic failures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThermalArithmetic {
    /// Multiplication of initial temperature by heat capacity.
    InitialEnergy,
    /// Division of canonical energy by heat capacity.
    Temperature,
    /// Signed temperature difference across one conductive link.
    TemperatureDifference,
    /// Multiplication of conductance by temperature difference.
    ConductivePower,
    /// Multiplication of conductive power by the fixed time step.
    TransferredHeat,
    /// Dimensionless explicit-Euler stability ratios.
    StabilityRatio,
    /// Exact reduction of one body's link-energy contributions.
    EnergyReduction,
    /// Addition of reduced energy change to canonical energy.
    EnergyUpdate,
    /// Division of observable energy change by the fixed time step.
    NetPower,
    /// Addition of the fixed time step to elapsed simulation time.
    ElapsedTime,
    /// Closed-system energy conservation accounting.
    EnergyAccounting,
}

/// Structured failure from the lumped-conduction domain.
#[derive(Clone, Debug, PartialEq)]
pub enum ThermalError {
    /// A typed thermodynamic scalar failed boundary validation.
    Quantity(ThermalQuantityError),
    /// The world already stores its maximum body count.
    BodyCapacityReached {
        /// Hard body limit for one thermal world.
        maximum: usize,
    },
    /// The world already stores its maximum conductive-link count.
    LinkCapacityReached {
        /// Hard link limit for one thermal world.
        maximum: usize,
    },
    /// The monotonically increasing body allocator has no remaining ID.
    EntityIdentityExhausted,
    /// The monotonically increasing link allocator has no remaining ID.
    LinkIdentityExhausted,
    /// A step or revision counter cannot advance without overflow.
    CounterExhausted,
    /// A command names a thermal body absent from this world.
    MissingBody {
        /// Missing stable body identity.
        entity: EntityId,
    },
    /// A conductive link cannot connect a body to itself.
    IdenticalEndpoints {
        /// Repeated endpoint identity.
        entity: EntityId,
    },
    /// The unordered endpoint pair already has a conductive link.
    DuplicateLink {
        /// Canonically ordered first endpoint.
        first: EntityId,
        /// Canonically ordered second endpoint.
        second: EntityId,
        /// Existing link for this endpoint pair.
        existing: ThermalLinkId,
    },
    /// A body's explicit-Euler stability ratio exceeds one.
    UnstableTimeStep {
        /// Body whose incident-link ratio is unstable.
        entity: EntityId,
        /// Computed dimensionless stability ratio.
        ratio: f64,
    },
    /// One isolated conductive pair would cross its shared equilibrium.
    UnstableLinkTimeStep {
        /// Conductive link whose isolated pair would cross equilibrium.
        link: ThermalLinkId,
        /// Computed dimensionless pair ratio.
        ratio: f64,
    },
    /// Checked derived arithmetic produced NaN or infinity.
    DerivedNonFinite {
        /// Formula stage that failed.
        operation: ThermalArithmetic,
        /// Affected body, or `None` for link/world-level arithmetic.
        entity: Option<EntityId>,
    },
    /// A non-zero derived result became zero or made no representable progress.
    PrecisionLoss {
        /// Formula stage that lost representability.
        operation: ThermalArithmetic,
        /// Affected body, or `None` for link/world-level arithmetic.
        entity: Option<EntityId>,
    },
    /// A candidate update would make canonical body energy non-positive.
    NonPositiveCandidateEnergy {
        /// Body rejected by the positivity invariant.
        entity: EntityId,
    },
    /// Candidate closed-system energy drift exceeds the accepted envelope.
    EnergyConservationLost {
        /// Absolute exact-reduction drift in joules.
        drift_joules: f64,
        /// Maximum accepted deterministic drift in joules.
        tolerance_joules: f64,
    },
}

impl From<ThermalQuantityError> for ThermalError {
    fn from(value: ThermalQuantityError) -> Self {
        Self::Quantity(value)
    }
}

impl fmt::Display for ThermalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Quantity(error) => error.fmt(formatter),
            Self::BodyCapacityReached { maximum } => {
                write!(formatter, "thermal body capacity {maximum} reached")
            }
            Self::LinkCapacityReached { maximum } => {
                write!(formatter, "conductive link capacity {maximum} reached")
            }
            Self::EntityIdentityExhausted => formatter.write_str("thermal entity IDs exhausted"),
            Self::LinkIdentityExhausted => formatter.write_str("thermal link IDs exhausted"),
            Self::CounterExhausted => formatter.write_str("thermal world counter exhausted"),
            Self::MissingBody { entity } => {
                write!(formatter, "thermal body {} not found", entity.get())
            }
            Self::IdenticalEndpoints { entity } => {
                write!(formatter, "thermal link repeats body {}", entity.get())
            }
            Self::DuplicateLink { first, second, .. } => write!(
                formatter,
                "thermal link {}-{} already exists",
                first.get(),
                second.get()
            ),
            Self::UnstableTimeStep { entity, ratio } => write!(
                formatter,
                "thermal stability ratio {ratio} exceeds one for body {}",
                entity.get()
            ),
            Self::UnstableLinkTimeStep { link, ratio } => write!(
                formatter,
                "thermal pair stability ratio {ratio} exceeds one for link {}",
                link.get()
            ),
            Self::DerivedNonFinite { operation, .. } => {
                write!(formatter, "non-finite thermal result during {operation:?}")
            }
            Self::PrecisionLoss { operation, .. } => {
                write!(formatter, "thermal progress lost during {operation:?}")
            }
            Self::NonPositiveCandidateEnergy { entity } => write!(
                formatter,
                "thermal body {} would have non-positive energy",
                entity.get()
            ),
            Self::EnergyConservationLost {
                drift_joules,
                tolerance_joules,
            } => write!(
                formatter,
                "thermal energy drift {drift_joules} J exceeds {tolerance_joules} J"
            ),
        }
    }
}

impl Error for ThermalError {}
