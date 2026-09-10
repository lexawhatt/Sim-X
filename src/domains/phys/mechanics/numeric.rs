use crate::foundation::EntityId;

use super::{ArithmeticOperation, MechanicsError};

pub(super) fn checked_add_progress(
    base: f64,
    increment: f64,
    operation: ArithmeticOperation,
    entity: Option<EntityId>,
) -> Result<f64, MechanicsError> {
    let result = base + increment;
    ensure_finite(result, operation, entity)?;
    if increment != 0.0 && result == base {
        return Err(MechanicsError::PrecisionLoss { operation, entity });
    }
    Ok(result)
}

pub(super) fn checked_multiply(
    left: f64,
    right: f64,
    operation: ArithmeticOperation,
    entity: EntityId,
) -> Result<f64, MechanicsError> {
    let result = left * right;
    ensure_finite(result, operation, Some(entity))?;
    if left != 0.0 && right != 0.0 && result == 0.0 {
        return Err(MechanicsError::PrecisionLoss {
            operation,
            entity: Some(entity),
        });
    }
    Ok(result)
}

pub(super) fn checked_divide(
    numerator: f64,
    denominator: f64,
    operation: ArithmeticOperation,
    entity: EntityId,
) -> Result<f64, MechanicsError> {
    let result = numerator / denominator;
    ensure_finite(result, operation, Some(entity))?;
    if numerator != 0.0 && result == 0.0 {
        return Err(MechanicsError::PrecisionLoss {
            operation,
            entity: Some(entity),
        });
    }
    Ok(result)
}

pub(super) fn ensure_finite(
    value: f64,
    operation: ArithmeticOperation,
    entity: Option<EntityId>,
) -> Result<(), MechanicsError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(MechanicsError::NonFiniteArithmetic { operation, entity })
    }
}
