use crate::rigid::{model::Rotations, pose};
use crate::{BodyDesc, Error, LinkDesc, LinkId, NumericStage, SolverConfig, Vec2, numeric};

/// Reconstructible array indices, never exposed or serialized as endpoints.
#[derive(Clone, Copy, Debug)]
pub(crate) struct BoundLink {
    pub desc: LinkDesc,
    pub a: usize,
    pub b: usize,
}

impl BoundLink {
    pub fn direction(self, bodies: &[BodyDesc]) -> Vec2 {
        bodies[self.b].position_m - bodies[self.a].position_m
    }
    pub fn attachment_points(self, bodies: &[BodyDesc], rotations: &Rotations) -> (Vec2, Vec2) {
        let (a, b) = pose::offsets(self.desc);
        (
            pose::attachment(bodies, rotations, self.a, a),
            pose::attachment(bodies, rotations, self.b, b),
        )
    }
    pub fn attachment_direction(self, bodies: &[BodyDesc], rotations: &Rotations) -> Vec2 {
        let (a, b) = self.attachment_points(bodies, rotations);
        b - a
    }
    pub fn length_tolerance(self, config: SolverConfig) -> f64 {
        match self.desc {
            LinkDesc::Rod { length_m, .. } => {
                config.position_tolerance_m + config.relative_tolerance * length_m
            }
            LinkDesc::Spring { .. } | LinkDesc::AttachedSpring { .. } => 0.0,
        }
    }
}

pub(crate) fn validate_residuals(
    bodies: &[BodyDesc],
    links: &[BoundLink],
    config: SolverConfig,
    initial: bool,
) -> Result<(), Error> {
    for link in links {
        let LinkDesc::Rod { length_m, id, .. } = link.desc else {
            continue;
        };
        let direction = link.direction(bodies);
        let distance = direction.length();
        let radial =
            direction.dot(bodies[link.b].velocity_m_s - bodies[link.a].velocity_m_s) / distance;
        if !distance.is_finite()
            || distance == 0.0
            || !radial.is_finite()
            || (distance - length_m).abs() > link.length_tolerance(config)
            || radial.abs() > config.velocity_tolerance_m_s
        {
            return Err(if initial {
                Error::InitialConstraint(id)
            } else {
                Error::ConstraintFailure(id)
            });
        }
    }
    Ok(())
}

pub(crate) fn solve_positions(
    before: &[BodyDesc],
    bodies: &mut [BodyDesc],
    links: &[BoundLink],
    config: SolverConfig,
    impulses: &mut [Vec2],
) -> Result<(), Error> {
    position_sweeps(before, bodies, links, config, impulses, true)
}

/// Geometric cleanup after contact projection is deliberately not a velocity
/// impulse. The ordinary smooth RATTLE position phase still supplies feedback.
pub(crate) fn project_positions(
    before: &[BodyDesc],
    bodies: &mut [BodyDesc],
    links: &[BoundLink],
    config: SolverConfig,
    impulses: &mut [Vec2],
) -> Result<(), Error> {
    position_sweeps(before, bodies, links, config, impulses, false)
}

fn position_sweeps(
    before: &[BodyDesc],
    bodies: &mut [BodyDesc],
    links: &[BoundLink],
    config: SolverConfig,
    impulses: &mut [Vec2],
    feedback: bool,
) -> Result<(), Error> {
    let dt = config.fixed_dt_s;
    for _ in 0..config.constraint_iterations {
        for (index, link) in links.iter().enumerate() {
            let LinkDesc::Rod { length_m, id, .. } = link.desc else {
                continue;
            };
            let old_direction = link.direction(before);
            let direction = link.direction(bodies);
            let distance = direction.length();
            // Converged corrections smaller than the accepted geometric
            // tolerance are not mistaken for integration progress failures.
            if (distance - length_m).abs() <= link.length_tolerance(config) * 0.01 {
                continue;
            }
            let inverse_a = bodies[link.a].inverse_mass();
            let inverse_b = bodies[link.b].inverse_mass();
            let denominator = (inverse_a + inverse_b) * direction.dot(old_direction);
            if !denominator.is_finite() || denominator <= 0.0 {
                return Err(Error::ConstraintFailure(id));
            }
            let multiplier = (direction.dot(direction) - length_m * length_m) / (2.0 * denominator);
            let correction =
                numeric::finite_vec(old_direction * multiplier, NumericStage::Constraint)?;
            shift_position(
                &mut bodies[link.a],
                correction * inverse_a,
                dt,
                id,
                feedback,
            )?;
            shift_position(
                &mut bodies[link.b],
                correction * (-inverse_b),
                dt,
                id,
                feedback,
            )?;
            if feedback {
                impulses[index] = numeric::finite_vec(
                    impulses[index] + correction * (1.0 / dt),
                    NumericStage::Constraint,
                )?;
            }
        }
    }
    Ok(())
}

fn shift_position(
    body: &mut BodyDesc,
    shift: Vec2,
    dt: f64,
    id: LinkId,
    feedback: bool,
) -> Result<(), Error> {
    let next_position = body.position_m + shift;
    let next_velocity = if feedback {
        body.velocity_m_s + shift * (1.0 / dt)
    } else {
        body.velocity_m_s
    };
    if !next_position.finite() || !next_velocity.finite() {
        return Err(Error::ConstraintFailure(id));
    }
    body.position_m = next_position;
    body.velocity_m_s = next_velocity;
    Ok(())
}

pub(crate) fn solve_velocities(
    bodies: &mut [BodyDesc],
    links: &[BoundLink],
    config: SolverConfig,
    impulses: &mut [Vec2],
) -> Result<(), Error> {
    for _ in 0..config.constraint_iterations {
        for (index, link) in links.iter().enumerate() {
            let LinkDesc::Rod { id, .. } = link.desc else {
                continue;
            };
            let direction = link.direction(bodies);
            let inverse_a = bodies[link.a].inverse_mass();
            let inverse_b = bodies[link.b].inverse_mass();
            let denominator = (inverse_a + inverse_b) * direction.dot(direction);
            if !denominator.is_finite() || denominator <= 0.0 {
                return Err(Error::ConstraintFailure(id));
            }
            let projection =
                direction.dot(bodies[link.b].velocity_m_s - bodies[link.a].velocity_m_s);
            if (projection / direction.length()).abs() <= config.velocity_tolerance_m_s * 0.01 {
                continue;
            }
            let impulse = numeric::finite_vec(
                direction * (projection / denominator),
                NumericStage::Constraint,
            )?;
            bodies[link.a].velocity_m_s = numeric::finite_vec(
                bodies[link.a].velocity_m_s + impulse * inverse_a,
                NumericStage::Constraint,
            )?;
            bodies[link.b].velocity_m_s = numeric::finite_vec(
                bodies[link.b].velocity_m_s - impulse * inverse_b,
                NumericStage::Constraint,
            )?;
            impulses[index] =
                numeric::finite_vec(impulses[index] + impulse, NumericStage::Constraint)?;
        }
    }
    Ok(())
}
