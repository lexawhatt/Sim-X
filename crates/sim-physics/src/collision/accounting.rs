//! Local energy differences retain small contact observations beside large,
//! unrelated world energies. No global before/after subtraction is used.

use crate::mechanics::{constraints::BoundLink, energy};
use crate::rigid::model::Rotations;
use crate::{
    BodyDesc, Error, LinkDesc, Mobility, NumericStage, PhysicsSettings, SolverConfig, numeric,
};

pub(crate) fn potential_change(
    before: &[BodyDesc],
    after: &[BodyDesc],
    links: &[BoundLink],
    settings: PhysicsSettings,
) -> Result<f64, Error> {
    potential_change_impl(before, after, links, settings, None)
}

pub(crate) fn potential_change_rigid(
    before: &[BodyDesc],
    after: &[BodyDesc],
    links: &[BoundLink],
    settings: PhysicsSettings,
    old_rotations: &Rotations,
    new_rotations: &Rotations,
) -> Result<f64, Error> {
    potential_change_impl(
        before,
        after,
        links,
        settings,
        Some((old_rotations, new_rotations)),
    )
}

fn potential_change_impl(
    before: &[BodyDesc],
    after: &[BodyDesc],
    links: &[BoundLink],
    settings: PhysicsSettings,
    rotations: Option<(&Rotations, &Rotations)>,
) -> Result<f64, Error> {
    let mut changes = Vec::with_capacity(before.len() + links.len());
    for (old, new) in before
        .iter()
        .zip(after)
        .filter(|(body, _)| body.mobility == Mobility::Dynamic)
    {
        changes.push(energy::gravitational(
            old.mass_kg,
            settings.gravity_m_s2,
            new.position_m - old.position_m,
        )?);
    }
    for link in links {
        if let LinkDesc::Spring {
            rest_length_m,
            stiffness_n_m,
            ..
        }
        | LinkDesc::AttachedSpring {
            rest_length_m,
            stiffness_n_m,
            ..
        } = link.desc
        {
            let old = rotations
                .map_or_else(
                    || link.direction(before),
                    |(old, _)| link.attachment_direction(before, old),
                )
                .length()
                - rest_length_m;
            let new = rotations
                .map_or_else(
                    || link.direction(after),
                    |(_, new)| link.attachment_direction(after, new),
                )
                .length()
                - rest_length_m;
            changes.push(numeric::product(
                &[0.5, stiffness_n_m, new - old, new + old],
                NumericStage::Telemetry,
            )?);
        }
    }
    numeric::sum(&changes, NumericStage::Telemetry)
}

pub(crate) fn kinetic_change(
    before: &[BodyDesc],
    after: &[BodyDesc],
    links: &[BoundLink],
    pairs: &[(usize, usize)],
    solver: SolverConfig,
) -> Result<f64, Error> {
    kinetic_change_impl(before, after, links, pairs, solver, None)
}

pub(crate) fn kinetic_change_rigid(
    before: &[BodyDesc],
    after: &[BodyDesc],
    links: &[BoundLink],
    pairs: &[(usize, usize)],
    solver: SolverConfig,
    old_rotations: &Rotations,
    new_rotations: &Rotations,
) -> Result<f64, Error> {
    kinetic_change_impl(
        before,
        after,
        links,
        pairs,
        solver,
        Some((old_rotations, new_rotations)),
    )
}

fn kinetic_change_impl(
    before: &[BodyDesc],
    after: &[BodyDesc],
    links: &[BoundLink],
    pairs: &[(usize, usize)],
    solver: SolverConfig,
    rotations: Option<(&Rotations, &Rotations)>,
) -> Result<f64, Error> {
    let mut parent: Vec<_> = (0..before.len()).collect();
    for &(a, b) in pairs {
        join(&mut parent, a, b);
    }
    for link in links {
        if matches!(link.desc, LinkDesc::Rod { .. }) {
            join(&mut parent, link.a, link.b);
        }
    }
    let mut changes = vec![Vec::new(); before.len()];
    let mut energies = vec![Vec::new(); before.len()];
    for (index, (old, new)) in before.iter().zip(after).enumerate() {
        let island = root(&parent, index);
        changes[island].push(-energy::drag_decrease(
            old.mass_kg,
            old.velocity_m_s,
            new.velocity_m_s,
        )?);
        energies[island].push(energy::kinetic(old.mass_kg, old.velocity_m_s)?);
        if let Some((old_rotations, new_rotations)) = rotations
            && let Some(rotation) = old_rotations.get(index)
        {
            let old = rotation.angular_velocity_rad_s;
            let new = new_rotations.omega(index);
            changes[island].push(numeric::product(
                &[0.5, rotation.inertia_kg_m2, new - old, new + old],
                NumericStage::Telemetry,
            )?);
            energies[island].push(numeric::product(
                &[0.5, rotation.inertia_kg_m2, old, old],
                NumericStage::Telemetry,
            )?);
        }
    }
    let mut totals = Vec::with_capacity(before.len());
    for (changes, energies) in changes.iter().zip(&energies) {
        let change = numeric::sum(changes, NumericStage::Telemetry)?;
        let scale = numeric::sum(energies, NumericStage::Telemetry)?;
        if change > scale * solver.relative_tolerance + f64::EPSILON {
            return Err(Error::ContactEnergyIncrease);
        }
        totals.push(change);
    }
    numeric::sum(&totals, NumericStage::Telemetry)
}

fn root(parent: &[usize], mut index: usize) -> usize {
    while parent[index] != index {
        index = parent[index];
    }
    index
}

fn join(parent: &mut [usize], a: usize, b: usize) {
    let a = root(parent, a);
    let b = root(parent, b);
    parent[a.max(b)] = a.min(b);
}
