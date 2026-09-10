use std::{error::Error, fmt};

use crate::foundation::{ConstantsError, EntityId};

use super::{ElectrostaticQuantityError, FieldProbeId};

/// Named arithmetic stage used by structured electrostatic failures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ElectrostaticArithmetic {
    /// Scaled inverse-square field magnitude for one charge.
    FieldScale,
    /// Scaled multiplication of field magnitude by one normalized direction.
    FieldComponent,
    /// Exact reduction of all charge contributions at one probe.
    FieldReduction,
    /// Overflow-safe Euclidean norm of the reduced field vector.
    FieldMagnitude,
}

/// Structured failure from electrostatic creation or evaluation.
#[derive(Clone, Debug, PartialEq)]
pub enum ElectrostaticError {
    /// A typed electrostatic scalar failed boundary validation.
    Quantity(ElectrostaticQuantityError),
    /// The supplied physical-constants registry is invalid.
    Constants(ConstantsError),
    /// The world already stores its maximum point-charge count.
    ChargeCapacityReached {
        /// Hard charge limit for one electrostatic world.
        maximum: usize,
    },
    /// The world already stores its maximum field-probe count.
    ProbeCapacityReached {
        /// Hard probe limit for one electrostatic world.
        maximum: usize,
    },
    /// The monotonically increasing charge allocator has no remaining ID.
    ChargeIdentityExhausted,
    /// The monotonically increasing probe allocator has no remaining ID.
    ProbeIdentityExhausted,
    /// The canonical world revision cannot advance without overflow.
    CounterExhausted,
    /// A valid world exceeds the hard charge/probe work budget.
    InteractionBudgetExceeded {
        /// Charge/probe pair count requested by the evaluation.
        interactions: usize,
        /// Hard interaction limit for one evaluation.
        maximum: usize,
    },
    /// Integer multiplication of charge and probe counts overflowed.
    InteractionCountOverflow,
    /// A field probe exactly coincides with a point charge.
    SingularProbe {
        /// Probe at the singular position.
        probe: FieldProbeId,
        /// Coincident point-charge identity.
        charge: EntityId,
    },
    /// Checked derived arithmetic produced NaN or infinity.
    DerivedNonFinite {
        /// Formula stage that failed.
        operation: ElectrostaticArithmetic,
        /// Affected field probe, when evaluation had selected one.
        probe: Option<FieldProbeId>,
        /// Affected point charge, when evaluation had selected one.
        charge: Option<EntityId>,
    },
    /// A non-zero derived result became unrepresentable as binary64.
    PrecisionLoss {
        /// Formula stage that lost representability.
        operation: ElectrostaticArithmetic,
        /// Affected field probe, when evaluation had selected one.
        probe: Option<FieldProbeId>,
        /// Affected point charge, when evaluation had selected one.
        charge: Option<EntityId>,
    },
}

impl From<ElectrostaticQuantityError> for ElectrostaticError {
    fn from(value: ElectrostaticQuantityError) -> Self {
        Self::Quantity(value)
    }
}

impl From<ConstantsError> for ElectrostaticError {
    fn from(value: ConstantsError) -> Self {
        Self::Constants(value)
    }
}

impl fmt::Display for ElectrostaticError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Quantity(error) => error.fmt(formatter),
            Self::Constants(error) => error.fmt(formatter),
            Self::ChargeCapacityReached { maximum } => {
                write!(formatter, "point-charge capacity {maximum} reached")
            }
            Self::ProbeCapacityReached { maximum } => {
                write!(formatter, "field-probe capacity {maximum} reached")
            }
            Self::ChargeIdentityExhausted => formatter.write_str("point-charge IDs exhausted"),
            Self::ProbeIdentityExhausted => formatter.write_str("field-probe IDs exhausted"),
            Self::CounterExhausted => formatter.write_str("electrostatic world revision exhausted"),
            Self::InteractionBudgetExceeded {
                interactions,
                maximum,
            } => write!(
                formatter,
                "electrostatic interaction count {interactions} exceeds {maximum}"
            ),
            Self::InteractionCountOverflow => {
                formatter.write_str("electrostatic interaction count overflowed")
            }
            Self::SingularProbe { probe, charge } => write!(
                formatter,
                "probe {} coincides with charge {}",
                probe.get(),
                charge.get()
            ),
            Self::DerivedNonFinite { operation, .. } => write!(
                formatter,
                "non-finite electrostatic result during {operation:?}"
            ),
            Self::PrecisionLoss { operation, .. } => write!(
                formatter,
                "electrostatic precision lost during {operation:?}"
            ),
        }
    }
}

impl Error for ElectrostaticError {}
