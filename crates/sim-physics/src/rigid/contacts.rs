//! Coupled frictionless angular contact impulses and split position cleanup.

use super::{manifold, model::Rotations, normal_block::EffectiveMass, pose, sweep};
use crate::collision::{
    accounting, geometry,
    model::{self, BoundCollider},
    solve::ContactStep,
};
use crate::mechanics::constraints::{self, BoundLink};
use crate::{
    BodyDesc, ContactPointTelemetry, ContactTelemetry, Error, MAX_CONTACT_POINTS, MAX_CONTACTS,
    NumericStage, PhysicsSettings, RESTITUTION_SPEED_THRESHOLD_M_S, SolverConfig, Vec2, numeric,
};

pub(crate) struct System<'a> {
    pub before: &'a [BodyDesc],
    pub before_rotations: &'a Rotations,
    pub colliders: &'a [BoundCollider],
    pub links: &'a [BoundLink],
    pub settings: PhysicsSettings,
    pub solver: SolverConfig,
}

impl System<'_> {
    pub fn positions(
        &self,
        bodies: &mut [BodyDesc],
        rotations: &mut Rotations,
        rods: &mut [Vec2],
        result: &mut ContactStep,
    ) -> Result<(), Error> {
        let initial = bodies.to_vec();
        let initial_rotations = rotations.clone();
        let once = SolverConfig {
            constraint_iterations: 1,
            ..self.solver
        };
        let mut changed = false;
        for _ in 0..self.solver.constraint_iterations {
            let previous = bodies.to_vec();
            let previous_rotations = rotations.clone();
            let mut count = 0;
            for (index, &a) in self.colliders.iter().enumerate() {
                for &b in &self.colliders[index + 1..] {
                    if !model::dynamic_pair(a, b, bodies) {
                        continue;
                    }
                    let tolerance = model::tolerance(a, b, self.solver);
                    let contact = manifold::contacts(
                        pose::collider(a, rotations),
                        pose::collider(b, rotations),
                        bodies,
                        tolerance,
                    );
                    if contact.gap > tolerance {
                        continue;
                    }
                    if contact.len == 0 {
                        return Err(Error::ContactFailure(a.desc.body, b.desc.body));
                    }
                    count += 1;
                    if count > MAX_CONTACTS {
                        return Err(Error::ContactBudget);
                    }
                    if contact.len == 2 {
                        let mass = EffectiveMass::new(
                            bodies,
                            rotations,
                            a,
                            b,
                            contact.normal,
                            contact.points.map(|point| point.point_m),
                        );
                        let gaps = contact.points.map(|point| point.gap_m);
                        if gaps.iter().any(|&gap| gap < -tolerance * 0.01)
                            && let Some(lambda) = mass.solve(gaps)
                        {
                            let total = lambda[0] + lambda[1];
                            let angular = mass.angular_delta(lambda);
                            bodies[a.index].position_m = numeric::finite_vec(
                                bodies[a.index].position_m
                                    - contact.normal * (mass.inverse_mass[0] * total),
                                NumericStage::Position,
                            )?;
                            bodies[b.index].position_m = numeric::finite_vec(
                                bodies[b.index].position_m
                                    + contact.normal * (mass.inverse_mass[1] * total),
                                NumericStage::Position,
                            )?;
                            shift_angle(rotations, a.index, -angular[0])?;
                            shift_angle(rotations, b.index, angular[1])?;
                            changed = true;
                            continue;
                        }
                    }
                    // Each correction changes both orientation and separation.
                    // Rebuild the manifold before the next point; reusing its
                    // old penetration could separate a face twice and drop
                    // the physical velocity contact altogether.
                    for _ in 0..contact.len {
                        let contact = manifold::contacts(
                            pose::collider(a, rotations),
                            pose::collider(b, rotations),
                            bodies,
                            tolerance,
                        );
                        if contact.gap > tolerance {
                            break;
                        }
                        let point = contact.points[..contact.len]
                            .iter()
                            .min_by(|a, b| a.gap_m.total_cmp(&b.gap_m))
                            .ok_or(Error::ContactFailure(a.desc.body, b.desc.body))?;
                        if point.gap_m >= -tolerance * 0.01 {
                            continue;
                        }
                        let ra = point.point_m - bodies[a.index].position_m;
                        let rb = point.point_m - bodies[b.index].position_m;
                        let wa = bodies[a.index].inverse_mass();
                        let wb = bodies[b.index].inverse_mass();
                        let ia = rotations.inverse_inertia(a.index, bodies);
                        let ib = rotations.inverse_inertia(b.index, bodies);
                        let ca = pose::cross(ra, contact.normal);
                        let cb = pose::cross(rb, contact.normal);
                        let lambda = -point.gap_m / (wa + wb + ia * ca * ca + ib * cb * cb);
                        bodies[a.index].position_m = numeric::finite_vec(
                            bodies[a.index].position_m - contact.normal * (wa * lambda),
                            NumericStage::Position,
                        )?;
                        bodies[b.index].position_m = numeric::finite_vec(
                            bodies[b.index].position_m + contact.normal * (wb * lambda),
                            NumericStage::Position,
                        )?;
                        shift_angle(rotations, a.index, -ia * ca * lambda)?;
                        shift_angle(rotations, b.index, ib * cb * lambda)?;
                        changed = true;
                    }
                }
            }
            if !changed {
                return Ok(());
            }
            constraints::project_positions(self.before, bodies, self.links, once, rods)?;
            sweep::guard(
                &previous,
                &previous_rotations,
                bodies,
                rotations,
                self.colliders,
                self.solver,
            )?;
        }
        self.validate_gaps(bodies, rotations)?;
        for (index, body) in bodies.iter().enumerate() {
            result.position_correction[index] = body.position_m - initial[index].position_m;
            result.angle_correction[index] =
                rotations.angle(index) - initial_rotations.angle(index);
        }
        result.projection_energy_change = accounting::potential_change_rigid(
            &initial,
            bodies,
            self.links,
            self.settings,
            &initial_rotations,
            rotations,
        )?;
        Ok(())
    }

    pub fn validate_gaps(&self, bodies: &[BodyDesc], rotations: &Rotations) -> Result<(), Error> {
        let colliders = pose::colliders(self.colliders, rotations);
        for (index, &a) in colliders.iter().enumerate() {
            for &b in &colliders[index + 1..] {
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
        rotations: &mut Rotations,
        rods: &mut [Vec2],
        result: &mut ContactStep,
    ) -> Result<(), Error> {
        let mut pairs = self.collect(bodies, rotations)?;
        if pairs.is_empty() {
            return constraints::solve_velocities(bodies, self.links, self.solver, rods);
        }
        let before = bodies.to_vec();
        let old_rotations = rotations.clone();
        let once = SolverConfig {
            constraint_iterations: 1,
            ..self.solver
        };
        for _ in 0..self.solver.constraint_iterations {
            for pair in &mut pairs {
                if pair.solve_block(bodies, rotations)? {
                    continue;
                }
                for point in &mut pair.points[..pair.len] {
                    point.solve(pair.a, pair.b, pair.normal, bodies, rotations)?;
                }
            }
            constraints::solve_velocities(bodies, self.links, once, rods)?;
        }
        let edges: Vec<_> = pairs
            .iter()
            .map(|pair| (pair.a.index, pair.b.index))
            .collect();
        result.solve_kinetic_change = accounting::kinetic_change_rigid(
            &before,
            bodies,
            self.links,
            &edges,
            self.solver,
            &old_rotations,
            rotations,
        )?;
        self.report(pairs, bodies, rotations, result)
    }

    fn collect(&self, bodies: &[BodyDesc], rotations: &Rotations) -> Result<Vec<Pair>, Error> {
        let mut result = Vec::new();
        let mut point_count = 0;
        for (index, &a) in self.colliders.iter().enumerate() {
            for &b in &self.colliders[index + 1..] {
                if !model::dynamic_pair(a, b, bodies) {
                    continue;
                }
                let tolerance = model::tolerance(a, b, self.solver);
                let contact = manifold::contacts(
                    pose::collider(a, rotations),
                    pose::collider(b, rotations),
                    bodies,
                    tolerance,
                );
                if contact.gap > tolerance {
                    continue;
                }
                if contact.len == 0 {
                    return Err(Error::ContactFailure(a.desc.body, b.desc.body));
                }
                if result.len() == MAX_CONTACTS || point_count + contact.len > MAX_CONTACT_POINTS {
                    return Err(Error::ContactBudget);
                }
                point_count += contact.len;
                let old_gap = geometry::contact(
                    pose::collider(a, self.before_rotations),
                    pose::collider(b, self.before_rotations),
                    self.before,
                )
                .gap;
                let restitution = a.desc.restitution.max(b.desc.restitution);
                let mut points = [PointConstraint::default(); 2];
                for (point, geometry) in points.iter_mut().zip(&contact.points).take(contact.len) {
                    let ra = geometry.point_m - bodies[a.index].position_m;
                    let rb = geometry.point_m - bodies[b.index].position_m;
                    let vn = contact.normal.dot(
                        pose::velocity(bodies, rotations, b.index, rb)
                            - pose::velocity(bodies, rotations, a.index, ra),
                    );
                    let old_ra = pose::rotate(
                        ra,
                        self.before_rotations.angle(a.index) - rotations.angle(a.index),
                    );
                    let old_rb = pose::rotate(
                        rb,
                        self.before_rotations.angle(b.index) - rotations.angle(b.index),
                    );
                    let old_vn = contact.normal.dot(
                        pose::velocity(self.before, self.before_rotations, b.index, old_rb)
                            - pose::velocity(self.before, self.before_rotations, a.index, old_ra),
                    );
                    let arriving =
                        old_gap > tolerance || old_vn < -self.solver.velocity_tolerance_m_s;
                    let effective = if arriving && -vn >= RESTITUTION_SPEED_THRESHOLD_M_S {
                        restitution
                    } else {
                        0.0
                    };
                    *point = PointConstraint {
                        position: geometry.point_m,
                        gap: geometry.gap_m,
                        ra,
                        rb,
                        target: -effective * vn.min(0.0),
                        effective,
                        lambda: 0.0,
                    };
                }
                result.push(Pair {
                    a,
                    b,
                    normal: contact.normal,
                    restitution,
                    points,
                    len: contact.len,
                    gap: contact.gap,
                });
            }
        }
        Ok(result)
    }

    fn report(
        &self,
        pairs: Vec<Pair>,
        bodies: &[BodyDesc],
        rotations: &Rotations,
        result: &mut ContactStep,
    ) -> Result<(), Error> {
        let mut impulses = vec![Vec::new(); bodies.len()];
        let mut angular = vec![Vec::new(); bodies.len()];
        for pair in pairs {
            let mut records = Vec::with_capacity(pair.len);
            let mut a_angular = Vec::with_capacity(pair.len);
            let mut b_angular = Vec::with_capacity(pair.len);
            let mut pair_impulses = Vec::with_capacity(pair.len);
            let mut effective: f64 = 0.0;
            for point in &pair.points[..pair.len] {
                let residual =
                    point.speed(pair.a, pair.b, pair.normal, bodies, rotations) - point.target;
                if !residual.is_finite()
                    || residual < -self.solver.velocity_tolerance_m_s
                    || (point.lambda > 0.0 && residual.abs() > self.solver.velocity_tolerance_m_s)
                {
                    return Err(Error::ContactFailure(pair.a.desc.body, pair.b.desc.body));
                }
                let impulse =
                    numeric::finite_vec(pair.normal * -point.lambda, NumericStage::Telemetry)?;
                let torque_a =
                    numeric::finite(pose::cross(point.ra, impulse), NumericStage::Telemetry)?;
                let torque_b =
                    numeric::finite(pose::cross(point.rb, -impulse), NumericStage::Telemetry)?;
                impulses[pair.a.index].push(impulse);
                impulses[pair.b.index].push(-impulse);
                angular[pair.a.index].push(torque_a);
                angular[pair.b.index].push(torque_b);
                a_angular.push(torque_a);
                b_angular.push(torque_b);
                pair_impulses.push(impulse);
                effective = effective.max(point.effective);
                records.push(ContactPointTelemetry {
                    position_m: point.position,
                    gap_m: point.gap,
                    effective_restitution: point.effective,
                    impulse_on_a_ns: impulse,
                    angular_impulse_on_a_nms: torque_a,
                    angular_impulse_on_b_nms: torque_b,
                });
            }
            let impulse = numeric::sum_vec(&pair_impulses, NumericStage::Telemetry)?;
            result.reports.push(ContactTelemetry {
                a: pair.a.desc.body,
                b: pair.b.desc.body,
                normal_a_to_b: pair.normal,
                impulse_on_a_ns: impulse,
                average_force_on_a_n: numeric::finite_vec(
                    impulse * (1.0 / self.solver.fixed_dt_s),
                    NumericStage::Telemetry,
                )?,
                gap_m: pair.gap,
                restitution: pair.restitution,
                effective_restitution: effective,
                points: records,
                angular_impulse_on_a_nms: numeric::sum(&a_angular, NumericStage::Telemetry)?,
                angular_impulse_on_b_nms: numeric::sum(&b_angular, NumericStage::Telemetry)?,
            });
        }
        for index in 0..bodies.len() {
            result.impulses[index] = numeric::sum_vec(&impulses[index], NumericStage::Telemetry)?;
            result.angular_impulses[index] =
                numeric::sum(&angular[index], NumericStage::Telemetry)?;
        }
        Ok(())
    }
}

struct Pair {
    a: BoundCollider,
    b: BoundCollider,
    normal: Vec2,
    restitution: f64,
    points: [PointConstraint; 2],
    len: usize,
    gap: f64,
}

impl Pair {
    fn solve_block(
        &mut self,
        bodies: &mut [BodyDesc],
        rotations: &mut Rotations,
    ) -> Result<bool, Error> {
        if self.len != 2 {
            return Ok(false);
        }
        let mass = EffectiveMass::new(
            bodies,
            rotations,
            self.a,
            self.b,
            self.normal,
            self.points.map(|point| point.position),
        );
        let old = self.points.map(|point| point.lambda);
        let [k11, k12, k22] = mass.matrix;
        let current = self.points.map(|point| {
            point.speed(self.a, self.b, self.normal, bodies, rotations) - point.target
        });
        let residual = [
            current[0] - k11 * old[0] - k12 * old[1],
            current[1] - k12 * old[0] - k22 * old[1],
        ];
        let Some(next) = mass.solve(residual) else {
            return Ok(false);
        };
        let delta = [next[0] - old[0], next[1] - old[1]];
        let total = delta[0] + delta[1];
        let angular = mass.angular_delta(delta);
        bodies[self.a.index].velocity_m_s = numeric::finite_vec(
            bodies[self.a.index].velocity_m_s - self.normal * (mass.inverse_mass[0] * total),
            NumericStage::Velocity,
        )?;
        bodies[self.b.index].velocity_m_s = numeric::finite_vec(
            bodies[self.b.index].velocity_m_s + self.normal * (mass.inverse_mass[1] * total),
            NumericStage::Velocity,
        )?;
        shift_omega(rotations, self.a.index, -angular[0])?;
        shift_omega(rotations, self.b.index, angular[1])?;
        self.points[0].lambda = next[0];
        self.points[1].lambda = next[1];
        Ok(true)
    }
}

#[derive(Clone, Copy, Default)]
struct PointConstraint {
    position: Vec2,
    gap: f64,
    ra: Vec2,
    rb: Vec2,
    target: f64,
    effective: f64,
    lambda: f64,
}

impl PointConstraint {
    fn speed(
        self,
        a: BoundCollider,
        b: BoundCollider,
        normal: Vec2,
        bodies: &[BodyDesc],
        rotations: &Rotations,
    ) -> f64 {
        normal.dot(
            pose::velocity(bodies, rotations, b.index, self.rb)
                - pose::velocity(bodies, rotations, a.index, self.ra),
        )
    }
    fn solve(
        &mut self,
        a: BoundCollider,
        b: BoundCollider,
        normal: Vec2,
        bodies: &mut [BodyDesc],
        rotations: &mut Rotations,
    ) -> Result<(), Error> {
        let wa = bodies[a.index].inverse_mass();
        let wb = bodies[b.index].inverse_mass();
        let ia = rotations.inverse_inertia(a.index, bodies);
        let ib = rotations.inverse_inertia(b.index, bodies);
        let ca = pose::cross(self.ra, normal);
        let cb = pose::cross(self.rb, normal);
        let delta = (self.target - self.speed(a, b, normal, bodies, rotations))
            / (wa + wb + ia * ca * ca + ib * cb * cb);
        let next = numeric::finite((self.lambda + delta).max(0.0), NumericStage::Constraint)?;
        let impulse = next - self.lambda;
        bodies[a.index].velocity_m_s = numeric::finite_vec(
            bodies[a.index].velocity_m_s - normal * (wa * impulse),
            NumericStage::Velocity,
        )?;
        bodies[b.index].velocity_m_s = numeric::finite_vec(
            bodies[b.index].velocity_m_s + normal * (wb * impulse),
            NumericStage::Velocity,
        )?;
        shift_omega(rotations, a.index, -ia * ca * impulse)?;
        shift_omega(rotations, b.index, ib * cb * impulse)?;
        self.lambda = next;
        Ok(())
    }
}

fn shift_angle(rotations: &mut Rotations, body: usize, delta: f64) -> Result<(), Error> {
    if let Some(index) = rotations.indices[body] {
        rotations.states[index].angle_rad = numeric::finite(
            rotations.states[index].angle_rad + delta,
            NumericStage::Orientation,
        )?;
    }
    Ok(())
}

fn shift_omega(rotations: &mut Rotations, body: usize, delta: f64) -> Result<(), Error> {
    if let Some(index) = rotations.indices[body] {
        rotations.states[index].angular_velocity_rad_s = numeric::finite(
            rotations.states[index].angular_velocity_rad_s + delta,
            NumericStage::AngularVelocity,
        )?;
    }
    Ok(())
}
