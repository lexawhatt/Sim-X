use super::constraints::BoundLink;
use crate::rigid::{model::Rotations, pose};
use crate::{
    BodyDesc, Error, ForceInput, LinkDesc, Mobility, NumericStage, PhysicsSettings, Vec2, numeric,
};

pub(crate) fn external_forces(
    bodies: &[BodyDesc],
    input: &[ForceInput],
) -> Result<Vec<Vec2>, Error> {
    let mut ordered = input.to_vec();
    ordered.sort_by_key(|force| (force.target, force.source));
    for pair in ordered.windows(2) {
        if (pair[0].target, pair[0].source) == (pair[1].target, pair[1].source) {
            return Err(Error::DuplicateForce(pair[0].target, pair[0].source));
        }
    }
    let mut buckets = vec![Vec::new(); bodies.len()];
    for force in ordered {
        let index = bodies
            .binary_search_by_key(&force.target, |body| body.id)
            .map_err(|_| Error::MissingBody(force.target))?;
        if bodies[index].mobility == Mobility::Fixed {
            return Err(Error::FixedForce(force.target));
        }
        if !force.force_n.finite() {
            return Err(Error::InvalidForce(force.target));
        }
        buckets[index].push(force.force_n);
    }
    buckets
        .iter()
        .map(|values| numeric::sum_vec(values, NumericStage::ForceReduction))
        .collect()
}

pub(crate) struct SpringForces {
    pub bodies: Vec<Vec2>,
    pub links: Vec<Vec2>,
    pub torques: Vec<f64>,
}

pub(crate) fn spring_forces(
    bodies: &[BodyDesc],
    links: &[BoundLink],
    rotations: &Rotations,
) -> Result<SpringForces, Error> {
    let mut buckets = vec![Vec::new(); bodies.len()];
    let mut link_forces = vec![Vec2::ZERO; links.len()];
    let mut torques = vec![Vec::new(); bodies.len()];
    for (index, link) in links.iter().enumerate() {
        let (LinkDesc::Spring {
            rest_length_m,
            stiffness_n_m,
            id,
            ..
        }
        | LinkDesc::AttachedSpring {
            rest_length_m,
            stiffness_n_m,
            id,
            ..
        }) = link.desc
        else {
            continue;
        };
        let direction = link.attachment_direction(bodies, rotations);
        let length = direction.length();
        if !length.is_finite() || length < 1e-9 {
            return Err(Error::ConstraintFailure(id));
        }
        let force = numeric::finite_vec(
            direction * (stiffness_n_m * (length - rest_length_m) / length),
            NumericStage::ForceReduction,
        )?;
        buckets[link.a].push(force);
        buckets[link.b].push(-force);
        link_forces[index] = force;
        let (local_a, local_b) = pose::offsets(link.desc);
        torques[link.a].push(numeric::finite(
            pose::cross(pose::rotate(local_a, rotations.angle(link.a)), force),
            NumericStage::ForceReduction,
        )?);
        torques[link.b].push(numeric::finite(
            pose::cross(pose::rotate(local_b, rotations.angle(link.b)), -force),
            NumericStage::ForceReduction,
        )?);
    }
    Ok(SpringForces {
        bodies: buckets
            .iter()
            .map(|values| numeric::sum_vec(values, NumericStage::ForceReduction))
            .collect::<Result<_, _>>()?,
        links: link_forces,
        torques: torques
            .iter()
            .map(|values| numeric::sum(values, NumericStage::ForceReduction))
            .collect::<Result<_, _>>()?,
    })
}

pub(crate) fn total_forces(
    bodies: &[BodyDesc],
    settings: PhysicsSettings,
    external: &[Vec2],
    springs: &[Vec2],
) -> Result<Vec<Vec2>, Error> {
    bodies
        .iter()
        .enumerate()
        .map(|(index, body)| {
            if body.mobility == Mobility::Fixed {
                return Ok(Vec2::ZERO);
            }
            numeric::sum_vec(
                &[
                    numeric::scaled(
                        settings.gravity_m_s2,
                        body.mass_kg,
                        NumericStage::ForceReduction,
                    )?,
                    external[index],
                    springs[index],
                ],
                NumericStage::ForceReduction,
            )
        })
        .collect()
}
