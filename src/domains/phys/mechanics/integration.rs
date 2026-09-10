use crate::foundation::{Meters, MetersPerSecond, MetersPerSecondSquared, TimeStep};

use super::{
    Acceleration2, ArithmeticOperation, Force2, MechanicsError, Position2, Velocity2,
    capability::ForceReceiver,
    numeric::{checked_add_progress, checked_divide, checked_multiply},
};

pub(super) struct IntegratedState {
    pub(super) position: Position2,
    pub(super) velocity: Velocity2,
    pub(super) acceleration: Acceleration2,
}

pub(super) fn integrate(
    receiver: &ForceReceiver<'_>,
    net_force: Force2,
    dt: TimeStep,
) -> Result<IntegratedState, MechanicsError> {
    let entity = receiver.entity();
    let mass = receiver.mass().get();
    let seconds = dt.get();

    let acceleration_x = checked_divide(
        net_force.x().get(),
        mass,
        ArithmeticOperation::Acceleration,
        entity,
    )?;
    let acceleration_y = checked_divide(
        net_force.y().get(),
        mass,
        ArithmeticOperation::Acceleration,
        entity,
    )?;

    let velocity_delta_x = checked_multiply(
        acceleration_x,
        seconds,
        ArithmeticOperation::VelocityDelta,
        entity,
    )?;
    let velocity_delta_y = checked_multiply(
        acceleration_y,
        seconds,
        ArithmeticOperation::VelocityDelta,
        entity,
    )?;

    let velocity_x = checked_add_progress(
        receiver.velocity().x().get(),
        velocity_delta_x,
        ArithmeticOperation::Velocity,
        Some(entity),
    )?;
    let velocity_y = checked_add_progress(
        receiver.velocity().y().get(),
        velocity_delta_y,
        ArithmeticOperation::Velocity,
        Some(entity),
    )?;

    let position_delta_x = checked_multiply(
        velocity_x,
        seconds,
        ArithmeticOperation::PositionDelta,
        entity,
    )?;
    let position_delta_y = checked_multiply(
        velocity_y,
        seconds,
        ArithmeticOperation::PositionDelta,
        entity,
    )?;

    let position_x = checked_add_progress(
        receiver.position().x().get(),
        position_delta_x,
        ArithmeticOperation::Position,
        Some(entity),
    )?;
    let position_y = checked_add_progress(
        receiver.position().y().get(),
        position_delta_y,
        ArithmeticOperation::Position,
        Some(entity),
    )?;

    Ok(IntegratedState {
        position: Position2::new(
            Meters::new(position_x).map_err(|_| MechanicsError::NonFiniteArithmetic {
                operation: ArithmeticOperation::Position,
                entity: Some(entity),
            })?,
            Meters::new(position_y).map_err(|_| MechanicsError::NonFiniteArithmetic {
                operation: ArithmeticOperation::Position,
                entity: Some(entity),
            })?,
        ),
        velocity: Velocity2::new(
            MetersPerSecond::new(velocity_x).map_err(|_| MechanicsError::NonFiniteArithmetic {
                operation: ArithmeticOperation::Velocity,
                entity: Some(entity),
            })?,
            MetersPerSecond::new(velocity_y).map_err(|_| MechanicsError::NonFiniteArithmetic {
                operation: ArithmeticOperation::Velocity,
                entity: Some(entity),
            })?,
        ),
        acceleration: Acceleration2::new(
            MetersPerSecondSquared::new(acceleration_x).map_err(|_| {
                MechanicsError::NonFiniteArithmetic {
                    operation: ArithmeticOperation::Acceleration,
                    entity: Some(entity),
                }
            })?,
            MetersPerSecondSquared::new(acceleration_y).map_err(|_| {
                MechanicsError::NonFiniteArithmetic {
                    operation: ArithmeticOperation::Acceleration,
                    entity: Some(entity),
                }
            })?,
        ),
    })
}
