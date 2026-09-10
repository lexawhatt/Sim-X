use std::collections::BTreeMap;

use crate::foundation::{EntityId, Newtons, reproducible_sum::reproducible_sum};

use super::{ArithmeticOperation, Force2, ForceSourceId, MechanicsError};

pub(super) fn reduce_forces(
    ordered: &BTreeMap<(EntityId, ForceSourceId), Force2>,
) -> Result<BTreeMap<EntityId, Force2>, MechanicsError> {
    let mut components: BTreeMap<EntityId, (Vec<f64>, Vec<f64>)> = BTreeMap::new();
    for (&(target, _), force) in ordered {
        let (x_values, y_values) = components.entry(target).or_default();
        x_values.push(force.x().get());
        y_values.push(force.y().get());
    }

    components
        .into_iter()
        .map(|(target, (x_values, y_values))| {
            let x = reproducible_sum(&x_values).map_err(|_| arithmetic_error(target))?;
            let y = reproducible_sum(&y_values).map_err(|_| arithmetic_error(target))?;
            let force = Force2::new(
                Newtons::new(x).map_err(|_| arithmetic_error(target))?,
                Newtons::new(y).map_err(|_| arithmetic_error(target))?,
            );
            Ok((target, force))
        })
        .collect()
}

fn arithmetic_error(target: EntityId) -> MechanicsError {
    MechanicsError::NonFiniteArithmetic {
        operation: ArithmeticOperation::ForceReduction,
        entity: Some(target),
    }
}
