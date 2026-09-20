//! Angular Verlet with centre-only RATTLE rods and real contact-point impulses.

use super::{contacts::System, model::Rotations, pose, sweep};
use crate::collision::{model::BoundCollider, solve::ContactStep};
use crate::mechanics::{
    constraints::{self, BoundLink},
    forces::{spring_forces, total_forces},
    integration::{StepCandidate, apply_drag},
};
use crate::{
    BodyDesc, Error, LinkDesc, Mobility, NumericStage, PhysicsSettings, SolverConfig, Vec2, numeric,
};

pub(crate) fn advance(
    before: &[BodyDesc],
    links: &[BoundLink],
    settings: PhysicsSettings,
    solver: SolverConfig,
    external: &[Vec2],
    colliders: &[BoundCollider],
    rotations: &Rotations,
) -> Result<StepCandidate, Error> {
    let dt = solver.fixed_dt_s;
    let old_springs = spring_forces(before, links, rotations)?;
    let old_forces = total_forces(before, settings, external, &old_springs.bodies)?;
    let mut bodies = before.to_vec();
    let mut angular = rotations.clone();
    let mut drag_impulses = vec![Vec2::ZERO; before.len()];
    let mut drag_loss = 0.0;
    apply_drag(
        &mut bodies,
        settings.linear_drag_per_s,
        dt * 0.5,
        &mut drag_impulses,
        &mut drag_loss,
    )?;
    kick(&mut bodies, &old_forces, dt)?;
    angular_kick(&bodies, &mut angular, &old_springs.torques, dt)?;
    for body in &mut bodies {
        if body.mobility == Mobility::Fixed {
            continue;
        }
        let drift = numeric::scaled(body.velocity_m_s, dt, NumericStage::Position)?;
        body.position_m = numeric::advance(body.position_m, drift, NumericStage::Position)?;
    }
    for (index, body) in bodies.iter().enumerate() {
        if body.mobility == Mobility::Fixed {
            continue;
        }
        if let Some(rotation) = angular.indices[index] {
            let state = &mut angular.states[rotation];
            let drift = numeric::product(
                &[state.angular_velocity_rad_s, dt],
                NumericStage::Orientation,
            )?;
            let has_offset_spring = links.iter().any(|link| {
                let (a, b) = pose::offsets(link.desc);
                (link.a == index && a != Vec2::ZERO) || (link.b == index && b != Vec2::ZERO)
            });
            if has_offset_spring && drift.abs() > 0.05 {
                return Err(Error::AngularMotionTooLarge(body.id));
            }
            state.angle_rad = advance_scalar(state.angle_rad, drift, NumericStage::Orientation)?;
        }
    }
    sweep::guard(before, rotations, &bodies, &angular, colliders, solver)?;
    let mut rods = vec![Vec2::ZERO; links.len()];
    let once = SolverConfig {
        constraint_iterations: 1,
        ..solver
    };
    if links
        .iter()
        .any(|link| matches!(link.desc, LinkDesc::Rod { .. }))
    {
        for _ in 0..solver.constraint_iterations {
            let previous = bodies.clone();
            constraints::solve_positions(before, &mut bodies, links, once, &mut rods)?;
            sweep::guard(&previous, &angular, &bodies, &angular, colliders, solver)?;
        }
    }
    let system = System {
        before,
        before_rotations: rotations,
        colliders,
        links,
        settings,
        solver,
    };
    let mut contacts = ContactStep::empty(bodies.len());
    system.positions(&mut bodies, &mut angular, &mut rods, &mut contacts)?;
    sweep::guard(before, rotations, &bodies, &angular, colliders, solver)?;
    let new_springs = spring_forces(&bodies, links, &angular)?;
    let new_forces = total_forces(&bodies, settings, external, &new_springs.bodies)?;
    kick(&mut bodies, &new_forces, dt)?;
    angular_kick(&bodies, &mut angular, &new_springs.torques, dt)?;
    // Translational drag does not damp spin. Apply its final half before the
    // coupled velocity projection, otherwise v+omega x r could close afterward.
    apply_drag(
        &mut bodies,
        settings.linear_drag_per_s,
        dt * 0.5,
        &mut drag_impulses,
        &mut drag_loss,
    )?;
    system.velocities(&mut bodies, &mut angular, &mut rods, &mut contacts)?;
    for (index, &body) in bodies.iter().enumerate() {
        body.validate()?;
        if let Some(rotation) = angular.get(index) {
            rotation.validate(body)?;
        }
    }
    constraints::validate_residuals(&bodies, links, solver, false)?;
    system.validate_gaps(&bodies, &angular)?;
    Ok(StepCandidate {
        bodies,
        old_springs,
        new_springs,
        rod_impulses: rods,
        drag_impulses,
        drag_loss,
        contacts,
        rotations: angular,
    })
}

fn kick(bodies: &mut [BodyDesc], forces: &[Vec2], dt: f64) -> Result<(), Error> {
    for (body, &force) in bodies.iter_mut().zip(forces) {
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
    Ok(())
}

fn angular_kick(
    bodies: &[BodyDesc],
    rotations: &mut Rotations,
    torques: &[f64],
    dt: f64,
) -> Result<(), Error> {
    for (body_index, body) in bodies.iter().enumerate() {
        if body.mobility == Mobility::Fixed {
            continue;
        }
        if let Some(index) = rotations.indices[body_index] {
            let state = &mut rotations.states[index];
            let kick = numeric::product(
                &[torques[body_index], 1.0 / state.inertia_kg_m2, dt * 0.5],
                NumericStage::AngularVelocity,
            )?;
            state.angular_velocity_rad_s = advance_scalar(
                state.angular_velocity_rad_s,
                kick,
                NumericStage::AngularVelocity,
            )?;
        }
    }
    Ok(())
}

fn advance_scalar(old: f64, delta: f64, stage: NumericStage) -> Result<f64, Error> {
    let new = numeric::finite(old + delta, stage)?;
    if delta != 0.0 && new == old {
        return Err(Error::PrecisionLoss(stage));
    }
    Ok(new)
}
