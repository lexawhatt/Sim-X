use super::constraints::{self, BoundLink};
use super::energy;
use super::forces::{SpringForces, spring_forces, total_forces};
use crate::collision::{
    model::BoundCollider,
    solve::{ContactStep, ContactSystem},
    sweep,
};
use crate::rigid::model::Rotations;
use crate::{
    BodyDesc, Error, LinkDesc, Mobility, NumericStage, PhysicsSettings, SolverConfig, Vec2, numeric,
};

pub(crate) struct StepCandidate {
    pub bodies: Vec<BodyDesc>,
    pub old_springs: SpringForces,
    pub new_springs: SpringForces,
    pub rod_impulses: Vec<Vec2>,
    pub drag_impulses: Vec<Vec2>,
    pub drag_loss: f64,
    pub contacts: ContactStep,
    pub rotations: Rotations,
}

pub(crate) fn advance(
    before: &[BodyDesc],
    links: &[BoundLink],
    settings: PhysicsSettings,
    solver: SolverConfig,
    external: &[Vec2],
    colliders: &[BoundCollider],
    rotations: &Rotations,
) -> Result<StepCandidate, Error> {
    if !rotations.states.is_empty() {
        return crate::rigid::integration::advance(
            before, links, settings, solver, external, colliders, rotations,
        );
    }
    let dt = solver.fixed_dt_s;
    let old_springs = spring_forces(before, links, rotations)?;
    let old_forces = total_forces(before, settings, external, &old_springs.bodies)?;
    let mut candidate = before.to_vec();
    let mut drag_impulses = vec![Vec2::ZERO; candidate.len()];
    let mut drag_loss = 0.0;
    apply_drag(
        &mut candidate,
        settings.linear_drag_per_s,
        dt * 0.5,
        &mut drag_impulses,
        &mut drag_loss,
    )?;
    for (body, &force) in candidate.iter_mut().zip(&old_forces) {
        if body.mobility == Mobility::Fixed {
            continue;
        }
        let kick = numeric::scaled(
            force,
            body.inverse_mass() * dt * 0.5,
            NumericStage::Velocity,
        )?;
        body.velocity_m_s = numeric::advance(body.velocity_m_s, kick, NumericStage::Velocity)?;
        let drift = numeric::scaled(body.velocity_m_s, dt, NumericStage::Position)?;
        body.position_m = numeric::advance(body.position_m, drift, NumericStage::Position)?;
    }
    let mut rod_impulses = vec![Vec2::ZERO; links.len()];
    let mut contacts = ContactStep::empty(candidate.len());
    let contact_system = ContactSystem {
        before,
        colliders,
        links,
        settings,
        solver,
    };
    if colliders.is_empty() {
        constraints::solve_positions(before, &mut candidate, links, solver, &mut rod_impulses)?;
    } else {
        sweep::guard(before, &candidate, colliders, solver)?;
        let once = SolverConfig {
            constraint_iterations: 1,
            ..solver
        };
        if links
            .iter()
            .any(|link| matches!(link.desc, LinkDesc::Rod { .. }))
        {
            for _ in 0..solver.constraint_iterations {
                let previous = candidate.clone();
                constraints::solve_positions(
                    before,
                    &mut candidate,
                    links,
                    once,
                    &mut rod_impulses,
                )?;
                sweep::guard(&previous, &candidate, colliders, solver)?;
            }
        }
        contact_system.positions(&mut candidate, &mut rod_impulses, &mut contacts)?;
        sweep::guard(before, &candidate, colliders, solver)?;
    }
    let new_springs = spring_forces(&candidate, links, rotations)?;
    let new_forces = total_forces(&candidate, settings, external, &new_springs.bodies)?;
    for (body, &force) in candidate.iter_mut().zip(&new_forces) {
        if body.mobility == Mobility::Fixed {
            continue;
        }
        let kick = numeric::scaled(
            force,
            body.inverse_mass() * dt * 0.5,
            NumericStage::Velocity,
        )?;
        body.velocity_m_s = numeric::advance(body.velocity_m_s, kick, NumericStage::Velocity)?;
    }
    if colliders.is_empty() {
        constraints::solve_velocities(&mut candidate, links, solver, &mut rod_impulses)?;
    } else {
        contact_system.velocities(&mut candidate, &mut rod_impulses, &mut contacts)?;
    }
    apply_drag(
        &mut candidate,
        settings.linear_drag_per_s,
        dt * 0.5,
        &mut drag_impulses,
        &mut drag_loss,
    )?;
    for &body in &candidate {
        body.validate()?;
    }
    constraints::validate_residuals(&candidate, links, solver, false)?;
    contact_system.validate_gaps(&candidate)?;
    Ok(StepCandidate {
        bodies: candidate,
        old_springs,
        new_springs,
        rod_impulses,
        drag_impulses,
        drag_loss,
        contacts,
        rotations: rotations.clone(),
    })
}

pub(crate) fn apply_drag(
    bodies: &mut [BodyDesc],
    gamma: f64,
    duration: f64,
    impulses: &mut [Vec2],
    loss: &mut f64,
) -> Result<(), Error> {
    if gamma == 0.0 {
        return Ok(());
    }
    let factor = (-gamma * duration).exp();
    if factor == 1.0 {
        return Err(Error::PrecisionLoss(NumericStage::Velocity));
    }
    for (index, body) in bodies.iter_mut().enumerate() {
        if body.mobility == Mobility::Fixed {
            continue;
        }
        let previous = body.velocity_m_s;
        let next = numeric::scaled(previous, factor, NumericStage::Velocity)?;
        if previous != Vec2::ZERO && previous == next {
            return Err(Error::PrecisionLoss(NumericStage::Velocity));
        }
        impulses[index] = numeric::finite_vec(
            impulses[index] + (next - previous) * body.mass_kg,
            NumericStage::Telemetry,
        )?;
        *loss = numeric::sum(
            &[*loss, energy::drag_decrease(body.mass_kg, previous, next)?],
            NumericStage::Telemetry,
        )?;
        body.velocity_m_s = next;
    }
    Ok(())
}
