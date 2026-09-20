use super::model::Rotations;
use crate::mechanics::integration::StepCandidate;
use crate::{BodyDesc, BodyId, Error, NumericStage, SolverConfig, numeric};

/// Bounded committed angular observations; fixed bodies record support loads.
#[derive(Clone, Debug, PartialEq)]
pub struct RotationTelemetry {
    /// Stable body identity.
    pub body: BodyId,
    /// Physical body angle in radians.
    pub angle_rad: f64,
    /// Physical angular velocity in radians per second.
    pub angular_velocity_rad_s: f64,
    /// Actual angular velocity change divided by step duration.
    pub angular_acceleration_rad_s2: f64,
    /// Planar moment of inertia in kilograms times square metres.
    pub inertia_kg_m2: f64,
    /// Trapezoid-average spring torque about the body centre.
    pub spring_torque_nm: f64,
    /// Contact angular impulse about the centre, newton-metre-seconds.
    pub contact_angular_impulse_nms: f64,
    /// Contact angular impulse divided by step duration.
    pub contact_torque_nm: f64,
    /// Sum of modeled applied torques; fixed bodies need a balancing reaction.
    pub net_torque_nm: f64,
    /// Rotational kinetic energy in joules.
    pub kinetic_energy_j: f64,
    /// Numerical contact angle cleanup, explicitly not a physical angular impulse.
    pub contact_angle_correction_rad: f64,
}

pub(crate) fn report(
    bodies: &[BodyDesc],
    before: &Rotations,
    step: &StepCandidate,
    solver: SolverConfig,
) -> Result<Vec<RotationTelemetry>, Error> {
    let mut reports = Vec::with_capacity(before.states.len());
    for (index, body) in bodies.iter().enumerate() {
        let Some(rotation) = step.rotations.get(index) else {
            continue;
        };
        let spring = numeric::finite(
            (step.old_springs.torques[index] + step.new_springs.torques[index]) * 0.5,
            NumericStage::Telemetry,
        )?;
        let contact = numeric::finite(
            step.contacts.angular_impulses[index] / solver.fixed_dt_s,
            NumericStage::Telemetry,
        )?;
        reports.push(RotationTelemetry {
            body: body.id,
            angle_rad: rotation.angle_rad,
            angular_velocity_rad_s: rotation.angular_velocity_rad_s,
            angular_acceleration_rad_s2: numeric::finite(
                (rotation.angular_velocity_rad_s - before.omega(index)) / solver.fixed_dt_s,
                NumericStage::Telemetry,
            )?,
            inertia_kg_m2: rotation.inertia_kg_m2,
            spring_torque_nm: spring,
            contact_angular_impulse_nms: step.contacts.angular_impulses[index],
            contact_torque_nm: contact,
            net_torque_nm: numeric::sum(&[spring, contact], NumericStage::Telemetry)?,
            kinetic_energy_j: numeric::product(
                &[
                    0.5,
                    rotation.inertia_kg_m2,
                    rotation.angular_velocity_rad_s,
                    rotation.angular_velocity_rad_s,
                ],
                NumericStage::Telemetry,
            )?,
            contact_angle_correction_rad: step.contacts.angle_correction[index],
        });
    }
    Ok(reports)
}
