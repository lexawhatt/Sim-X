use crate::mechanics::constraints::BoundLink;
use crate::numeric::bounded_vec;
use crate::rigid::{model::Rotations, pose};
use crate::{BodyDesc, Error, LinkDesc, PhysicsSettings, SolverConfig};

pub(crate) fn settings(value: PhysicsSettings) -> Result<(), Error> {
    if !bounded_vec(value.gravity_m_s2, 1e6)
        || !value.linear_drag_per_s.is_finite()
        || !(0.0..=1000.0).contains(&value.linear_drag_per_s)
    {
        return Err(Error::InvalidSettings);
    }
    Ok(())
}

pub(crate) fn solver(value: SolverConfig) -> Result<(), Error> {
    let valid = value.fixed_dt_s.is_finite()
        && (1e-6..=1.0 / 30.0).contains(&value.fixed_dt_s)
        && (1..=128).contains(&value.constraint_iterations)
        && (1e-12..=1e-4).contains(&value.position_tolerance_m)
        && (1e-12..=1e-4).contains(&value.velocity_tolerance_m_s)
        && (1e-12..=1e-6).contains(&value.relative_tolerance);
    if valid {
        Ok(())
    } else {
        Err(Error::InvalidSolver)
    }
}

pub(crate) fn bind_links(
    bodies: &[BodyDesc],
    links: &[LinkDesc],
    solver: SolverConfig,
    rotations: &Rotations,
) -> Result<Vec<BoundLink>, Error> {
    let mut result = Vec::with_capacity(links.len());
    let mut rod_pairs = std::collections::BTreeSet::new();
    let mut spring_frequency_bound = 0.0;
    for &desc in links {
        let (id_a, id_b) = desc.endpoints();
        let id = desc.id();
        if id_a == id_b {
            return Err(Error::InvalidLink(id));
        }
        let a = bodies
            .binary_search_by_key(&id_a, |body| body.id)
            .map_err(|_| Error::MissingBody(id_a))?;
        let b = bodies
            .binary_search_by_key(&id_b, |body| body.id)
            .map_err(|_| Error::MissingBody(id_b))?;
        let inverse_mass = bodies[a].inverse_mass() + bodies[b].inverse_mass();
        if inverse_mass == 0.0 {
            return Err(Error::InvalidLink(id));
        }
        let length = match desc {
            LinkDesc::Rod { length_m, .. } => length_m,
            LinkDesc::Spring { rest_length_m, .. }
            | LinkDesc::AttachedSpring { rest_length_m, .. } => rest_length_m,
        };
        if !(1e-6..=1e8).contains(&length) {
            return Err(Error::InvalidLink(id));
        }
        match desc {
            LinkDesc::Rod { .. } => {
                if !rod_pairs.insert((id_a.min(id_b), id_a.max(id_b))) {
                    return Err(Error::DuplicateRod(id));
                }
            }
            LinkDesc::Spring { stiffness_n_m, .. }
            | LinkDesc::AttachedSpring { stiffness_n_m, .. } => {
                if !(1e-9..=1e12).contains(&stiffness_n_m) {
                    return Err(Error::InvalidLink(id));
                }
                let (local_a, local_b) = pose::offsets(desc);
                for (index, local) in [(a, local_a), (b, local_b)] {
                    if !bounded_vec(local, 1e6) {
                        return Err(Error::InvalidAttachment(id));
                    }
                    if local != crate::Vec2::ZERO && rotations.get(index).is_none() {
                        return Err(Error::MissingRotation(bodies[index].id));
                    }
                }
                let angular = local_a.dot(local_a) * rotations.inverse_inertia(a, bodies)
                    + local_b.dot(local_b) * rotations.inverse_inertia(b, bodies);
                spring_frequency_bound += stiffness_n_m * (inverse_mass + angular);
                if solver.fixed_dt_s * spring_frequency_bound.sqrt() > 0.5 {
                    return Err(Error::SpringStepTooLarge(id));
                }
                if (pose::attachment(bodies, rotations, b, local_b)
                    - pose::attachment(bodies, rotations, a, local_a))
                .length()
                    < 1e-9
                {
                    return Err(Error::InvalidLink(id));
                }
            }
        }
        result.push(BoundLink { desc, a, b });
    }
    Ok(result)
}
