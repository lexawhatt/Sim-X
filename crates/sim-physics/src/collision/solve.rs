use super::{
    accounting, geometry,
    model::{self, BoundCollider, ContactTelemetry, MAX_CONTACTS, RESTITUTION_SPEED_THRESHOLD_M_S},
    sweep,
};
use crate::mechanics::constraints::{self, BoundLink};
use crate::{BodyDesc, Error, NumericStage, PhysicsSettings, SolverConfig, Vec2, numeric};

pub(crate) struct ContactStep {
    pub reports: Vec<ContactTelemetry>,
    pub impulses: Vec<Vec2>,
    pub position_correction: Vec<Vec2>,
    pub projection_energy_change: f64,
    pub solve_kinetic_change: f64,
    pub angular_impulses: Vec<f64>,
    pub angle_correction: Vec<f64>,
}

impl ContactStep {
    pub fn empty(bodies: usize) -> Self {
        Self {
            reports: Vec::new(),
            impulses: vec![Vec2::ZERO; bodies],
            position_correction: vec![Vec2::ZERO; bodies],
            projection_energy_change: 0.0,
            solve_kinetic_change: 0.0,
            angular_impulses: vec![0.0; bodies],
            angle_correction: vec![0.0; bodies],
        }
    }
}

/// Coupling context contains immutable policy and reconstructible graph caches.
pub(crate) struct ContactSystem<'a> {
    pub before: &'a [BodyDesc],
    pub colliders: &'a [BoundCollider],
    pub links: &'a [BoundLink],
    pub settings: PhysicsSettings,
    pub solver: SolverConfig,
}

impl ContactSystem<'_> {
    pub fn positions(
        &self,
        bodies: &mut [BodyDesc],
        rods: &mut [Vec2],
        result: &mut ContactStep,
    ) -> Result<(), Error> {
        let initial = bodies.to_vec();
        let mut changed = false;
        let once = SolverConfig {
            constraint_iterations: 1,
            ..self.solver
        };
        for _ in 0..self.solver.constraint_iterations {
            let sweep_before = bodies.to_vec();
            let mut active = 0;
            for (i, &a) in self.colliders.iter().enumerate() {
                for &b in &self.colliders[i + 1..] {
                    if !model::dynamic_pair(a, b, bodies) {
                        continue;
                    }
                    let contact = geometry::contact(a, b, bodies);
                    let tolerance = model::tolerance(a, b, self.solver);
                    if contact.gap <= tolerance {
                        active += 1;
                        if active > MAX_CONTACTS {
                            return Err(Error::ContactBudget);
                        }
                    }
                    if contact.gap >= -tolerance * 0.01 {
                        continue;
                    }
                    let wa = bodies[a.index].inverse_mass();
                    let wb = bodies[b.index].inverse_mass();
                    let correction = contact.normal * (-contact.gap / (wa + wb));
                    bodies[a.index].position_m = numeric::finite_vec(
                        bodies[a.index].position_m - correction * wa,
                        NumericStage::Constraint,
                    )?;
                    bodies[b.index].position_m = numeric::finite_vec(
                        bodies[b.index].position_m + correction * wb,
                        NumericStage::Constraint,
                    )?;
                    changed = true;
                }
            }
            if changed {
                constraints::project_positions(self.before, bodies, self.links, once, rods)?;
                sweep::guard(&sweep_before, bodies, self.colliders, self.solver)?;
            } else {
                // The initial full pair scan found no penetration requiring
                // cleanup; the preceding smooth RATTLE phase already validated
                // rod geometry. Keep distant finite bodies on the smooth path.
                return Ok(());
            }
        }
        self.validate_gaps(bodies)?;
        if changed {
            for (index, body) in bodies.iter().enumerate() {
                result.position_correction[index] = body.position_m - initial[index].position_m;
            }
            result.projection_energy_change =
                accounting::potential_change(&initial, bodies, self.links, self.settings)?;
        }
        Ok(())
    }

    pub fn validate_gaps(&self, bodies: &[BodyDesc]) -> Result<(), Error> {
        for (i, &a) in self.colliders.iter().enumerate() {
            for &b in &self.colliders[i + 1..] {
                if model::dynamic_pair(a, b, bodies)
                    && geometry::contact(a, b, bodies).gap < -model::tolerance(a, b, self.solver)
                {
                    return Err(Error::ContactFailure(a.desc.body, b.desc.body));
                }
            }
        }
        Ok(())
    }

    pub fn velocities(
        &self,
        bodies: &mut [BodyDesc],
        rods: &mut [Vec2],
        result: &mut ContactStep,
    ) -> Result<(), Error> {
        let mut contacts = self.contacts(bodies)?;
        if contacts.is_empty() {
            return constraints::solve_velocities(bodies, self.links, self.solver, rods);
        }
        let before_solve = bodies.to_vec();
        let once = SolverConfig {
            constraint_iterations: 1,
            ..self.solver
        };
        for _ in 0..self.solver.constraint_iterations {
            for contact in &mut contacts {
                contact.solve(bodies)?;
            }
            constraints::solve_velocities(bodies, self.links, once, rods)?;
        }
        let pairs: Vec<_> = contacts
            .iter()
            .map(|contact| (contact.a.index, contact.b.index))
            .collect();
        result.solve_kinetic_change =
            accounting::kinetic_change(&before_solve, bodies, self.links, &pairs, self.solver)?;
        let mut contributions = vec![Vec::new(); bodies.len()];
        for contact in contacts {
            contact.validate(bodies, self.solver)?;
            let impulse =
                numeric::finite_vec(contact.normal * -contact.lambda, NumericStage::Telemetry)?;
            contributions[contact.a.index].push(impulse);
            contributions[contact.b.index].push(-impulse);
            result.reports.push(ContactTelemetry {
                a: contact.a.desc.body,
                b: contact.b.desc.body,
                normal_a_to_b: contact.normal,
                impulse_on_a_ns: impulse,
                average_force_on_a_n: numeric::finite_vec(
                    impulse * (1.0 / self.solver.fixed_dt_s),
                    NumericStage::Telemetry,
                )?,
                gap_m: geometry::contact(contact.a, contact.b, bodies).gap,
                restitution: contact.restitution,
                effective_restitution: contact.effective_restitution,
                points: Vec::new(),
                angular_impulse_on_a_nms: 0.0,
                angular_impulse_on_b_nms: 0.0,
            });
        }
        for (index, impulses) in contributions.iter().enumerate() {
            result.impulses[index] = numeric::sum_vec(impulses, NumericStage::Telemetry)?;
        }
        Ok(())
    }

    fn contacts(&self, bodies: &[BodyDesc]) -> Result<Vec<ActiveContact>, Error> {
        let mut contacts = Vec::new();
        for (i, &a) in self.colliders.iter().enumerate() {
            for &b in &self.colliders[i + 1..] {
                if !model::dynamic_pair(a, b, bodies) {
                    continue;
                }
                let geometry = geometry::contact(a, b, bodies);
                let tolerance = model::tolerance(a, b, self.solver);
                if geometry.gap > tolerance {
                    continue;
                }
                if contacts.len() == MAX_CONTACTS {
                    return Err(Error::ContactBudget);
                }
                let relative = geometry
                    .normal
                    .dot(bodies[b.index].velocity_m_s - bodies[a.index].velocity_m_s);
                let old_relative = geometry
                    .normal
                    .dot(self.before[b.index].velocity_m_s - self.before[a.index].velocity_m_s);
                let arriving = geometry::contact(a, b, self.before).gap > tolerance
                    || old_relative < -self.solver.velocity_tolerance_m_s;
                let restitution = a.desc.restitution.max(b.desc.restitution);
                let effective_restitution =
                    if arriving && -relative >= RESTITUTION_SPEED_THRESHOLD_M_S {
                        restitution
                    } else {
                        0.0
                    };
                contacts.push(ActiveContact {
                    a,
                    b,
                    normal: geometry.normal,
                    restitution,
                    effective_restitution,
                    target: -effective_restitution * relative.min(0.0),
                    lambda: 0.0,
                });
            }
        }
        Ok(contacts)
    }
}

struct ActiveContact {
    a: BoundCollider,
    b: BoundCollider,
    normal: Vec2,
    restitution: f64,
    effective_restitution: f64,
    target: f64,
    lambda: f64,
}

impl ActiveContact {
    fn speed(&self, bodies: &[BodyDesc]) -> f64 {
        self.normal
            .dot(bodies[self.b.index].velocity_m_s - bodies[self.a.index].velocity_m_s)
    }
    fn solve(&mut self, bodies: &mut [BodyDesc]) -> Result<(), Error> {
        let wa = bodies[self.a.index].inverse_mass();
        let wb = bodies[self.b.index].inverse_mass();
        let next = numeric::finite(
            (self.lambda + (self.target - self.speed(bodies)) / (wa + wb)).max(0.0),
            NumericStage::Constraint,
        )?;
        let impulse = self.normal * (next - self.lambda);
        bodies[self.a.index].velocity_m_s = numeric::finite_vec(
            bodies[self.a.index].velocity_m_s - impulse * wa,
            NumericStage::Velocity,
        )?;
        bodies[self.b.index].velocity_m_s = numeric::finite_vec(
            bodies[self.b.index].velocity_m_s + impulse * wb,
            NumericStage::Velocity,
        )?;
        self.lambda = next;
        Ok(())
    }
    fn validate(&self, bodies: &[BodyDesc], config: SolverConfig) -> Result<(), Error> {
        let residual = self.speed(bodies) - self.target;
        if !residual.is_finite()
            || residual < -config.velocity_tolerance_m_s
            || (self.lambda > 0.0 && residual.abs() > config.velocity_tolerance_m_s)
        {
            return Err(Error::ContactFailure(self.a.desc.body, self.b.desc.body));
        }
        Ok(())
    }
}
