//! Physical energy observations, kept separate from unchecked vector products.

use crate::{Error, NumericStage, Vec2, numeric};

pub(crate) fn kinetic(mass_kg: f64, velocity_m_s: Vec2) -> Result<f64, Error> {
    // hypot retains a usable speed when squaring Cartesian components would
    // underflow. The final mass-scaled product then rounds only once.
    let speed = velocity_m_s.length();
    numeric::product(&[0.5, mass_kg, speed, speed], NumericStage::Telemetry)
}

pub(crate) fn gravitational(mass_kg: f64, gravity: Vec2, position: Vec2) -> Result<f64, Error> {
    let x = numeric::product(&[-mass_kg, gravity.x, position.x], NumericStage::Telemetry)?;
    let y = numeric::product(&[-mass_kg, gravity.y, position.y], NumericStage::Telemetry)?;
    numeric::sum(&[x, y], NumericStage::Telemetry)
}

pub(crate) fn elastic(stiffness_n_m: f64, extension_m: f64) -> Result<f64, Error> {
    numeric::product(
        &[0.5, stiffness_n_m, extension_m, extension_m],
        NumericStage::Telemetry,
    )
}

pub(crate) fn drag_decrease(mass_kg: f64, before: Vec2, after: Vec2) -> Result<f64, Error> {
    // Difference of squares avoids both a premature tiny square and subtraction
    // of two almost equal kinetic energies. These are actual committed velocity
    // changes, not a second evaluation of the ideal exponential decay.
    let x = numeric::product(
        &[0.5, mass_kg, before.x - after.x, before.x + after.x],
        NumericStage::Telemetry,
    )?;
    let y = numeric::product(
        &[0.5, mass_kg, before.y - after.y, before.y + after.y],
        NumericStage::Telemetry,
    )?;
    numeric::sum(&[x, y], NumericStage::Telemetry)
}
