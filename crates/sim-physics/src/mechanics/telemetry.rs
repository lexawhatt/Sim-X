use super::constraints::BoundLink;
use super::energy;
use super::integration::StepCandidate;
use crate::rigid::{model::Rotations, pose};
use crate::{
    BodyDesc, BodyId, ContactTelemetry, Error, LinkDesc, LinkId, Mobility, NumericStage,
    PhysicsSettings, SolverConfig, Vec2, numeric,
};

/// Bounded committed per-body observations, suitable for future nerd-mode UI.
#[derive(Clone, Debug, PartialEq)]
pub struct BodyTelemetry {
    /// Stable observed body.
    pub id: BodyId,
    /// Committed position in meters.
    pub position_m: Vec2,
    /// Committed velocity in meters per second.
    pub velocity_m_s: Vec2,
    /// Actual step-average change in velocity divided by step time.
    pub acceleration_m_s2: Vec2,
    /// Uniform gravitational force, zero on fixed bodies in this model.
    pub gravity_force_n: Vec2,
    /// Exact reduction of external forces supplied for this step.
    pub external_force_n: Vec2,
    /// Average of Hooke forces evaluated at the old and new positions.
    pub spring_force_n: Vec2,
    /// Sum of applied rod impulses divided by step time; not instantaneous tension.
    pub constraint_force_n: Vec2,
    /// Drag impulse divided by step time.
    pub drag_force_n: Vec2,
    /// Contact impulse this step; position cleanup is explicitly excluded.
    pub contact_impulse_ns: Vec2,
    /// Contact impulse divided by the fixed physical step.
    pub contact_force_n: Vec2,
    /// Numerical contact/rod position cleanup, not a physical impulse.
    pub contact_position_correction_m: Vec2,
    /// Sum of modeled applied forces. Fixed supports need a balancing reaction.
    pub net_force_n: Vec2,
    /// Combined translational and rotational kinetic energy in joules.
    pub kinetic_energy_j: f64,
    /// Uniform-gravity potential relative to coordinate origin, in joules.
    pub gravitational_energy_j: f64,
}

/// Per-relationship committed observations.
#[derive(Clone, Debug, PartialEq)]
pub struct LinkTelemetry {
    /// Stable link identity.
    pub id: LinkId,
    /// Actual world position of endpoint A, including a local spring attachment.
    pub endpoint_a_m: Vec2,
    /// Actual world position of endpoint B, including a local spring attachment.
    pub endpoint_b_m: Vec2,
    /// Current endpoint distance in meters.
    pub distance_m: f64,
    /// Rod length error or spring extension, in meters.
    pub extension_m: f64,
    /// Radial relative velocity in meters per second.
    pub radial_velocity_m_s: f64,
    /// Step-average relationship force on endpoint A. B receives its negative.
    pub force_on_a_n: Vec2,
    /// Hooke elastic potential energy, zero for ideal rods.
    pub elastic_energy_j: f64,
}

/// Finite scalar energy diagnostics. These do not claim perfect conservation.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EnergyTelemetry {
    /// Combined translational and rotational kinetic energy, joules.
    pub kinetic_j: f64,
    /// Rotational contribution to kinetic_j, joules.
    pub rotational_kinetic_j: f64,
    /// Sum of dynamic gravitational potentials, joules.
    pub gravitational_j: f64,
    /// Sum of spring potentials, joules.
    pub elastic_j: f64,
    /// Kinetic plus gravitational plus elastic energy, joules.
    pub total_j: f64,
    /// Energy removed by the two exact drag substeps, joules.
    pub drag_dissipated_j: f64,
    /// Signed gravitational/elastic energy change from numerical position cleanup.
    /// This is a discretization intervention, never material heat production.
    pub contact_projection_energy_change_j: f64,
    /// Signed kinetic change during the coupled contact/rod velocity projection.
    /// Includes discrete support stabilization, not just material restitution loss.
    pub contact_solve_kinetic_change_j: f64,
}

/// Bounded observations returned only after an entire step validates.
#[derive(Clone, Debug, PartialEq)]
pub struct StepReport {
    /// Successful fixed-step count.
    pub step_index: u64,
    /// Strictly advanced physical time in seconds.
    pub elapsed_s: f64,
    /// Settings revision used for this step.
    pub settings_revision: u64,
    /// At most [`crate::MAX_BODIES`] items in stable body order.
    pub bodies: Vec<BodyTelemetry>,
    /// At most [`crate::MAX_LINKS`] items in stable link order.
    pub links: Vec<LinkTelemetry>,
    /// At most [`crate::MAX_CONTACTS`] frictionless pairs in stable body order.
    pub contacts: Vec<ContactTelemetry>,
    /// At most MAX_BODIES angular state observations in stable body order.
    pub rotations: Vec<crate::RotationTelemetry>,
    /// Energy accounting for this committed step.
    pub energy: EnergyTelemetry,
}

pub(crate) struct ReportIdentity<'a> {
    pub external: &'a [Vec2],
    pub next_step: u64,
    pub elapsed: f64,
    pub settings_revision: u64,
}

pub(crate) fn report(
    before: &[BodyDesc],
    links: &[BoundLink],
    settings: PhysicsSettings,
    solver: SolverConfig,
    step: &StepCandidate,
    rotations_before: &Rotations,
    input: ReportIdentity,
) -> Result<StepReport, Error> {
    let candidate = &step.bodies;

    let dt = solver.fixed_dt_s;
    let mut constraints = vec![Vec::new(); candidate.len()];
    let mut link_reports = Vec::with_capacity(links.len());
    for (index, link) in links.iter().enumerate() {
        let (endpoint_a, endpoint_b) = link.attachment_points(candidate, &step.rotations);
        let direction = endpoint_b - endpoint_a;
        let distance = direction.length();
        let (extension, force, elastic) = match link.desc {
            LinkDesc::Rod { length_m, .. } => {
                let force = step.rod_impulses[index] * (1.0 / dt);
                constraints[link.a].push(force);
                constraints[link.b].push(-force);
                (distance - length_m, force, 0.0)
            }
            LinkDesc::Spring {
                rest_length_m,
                stiffness_n_m,
                ..
            }
            | LinkDesc::AttachedSpring {
                rest_length_m,
                stiffness_n_m,
                ..
            } => {
                let extension = distance - rest_length_m;
                (
                    extension,
                    (step.old_springs.links[index] + step.new_springs.links[index]) * 0.5,
                    energy::elastic(stiffness_n_m, extension)?,
                )
            }
        };
        let velocity_a = pose::velocity(
            candidate,
            &step.rotations,
            link.a,
            endpoint_a - candidate[link.a].position_m,
        );
        let velocity_b = pose::velocity(
            candidate,
            &step.rotations,
            link.b,
            endpoint_b - candidate[link.b].position_m,
        );
        let radial = direction.dot(velocity_b - velocity_a) / distance;
        numeric::finite_vec(force, NumericStage::Telemetry)?;
        numeric::finite(elastic, NumericStage::Telemetry)?;
        numeric::finite(radial, NumericStage::Telemetry)?;
        link_reports.push(LinkTelemetry {
            id: link.desc.id(),
            endpoint_a_m: endpoint_a,
            endpoint_b_m: endpoint_b,
            distance_m: distance,
            extension_m: extension,
            radial_velocity_m_s: radial,
            force_on_a_n: force,
            elastic_energy_j: elastic,
        });
    }
    let rotations = crate::rigid::telemetry::report(candidate, rotations_before, step, solver)?;
    let mut body_reports = Vec::with_capacity(candidate.len());
    for (index, body) in candidate.iter().enumerate() {
        let dynamic = body.mobility == Mobility::Dynamic;
        let gravity = if dynamic {
            numeric::scaled(settings.gravity_m_s2, body.mass_kg, NumericStage::Telemetry)?
        } else {
            Vec2::ZERO
        };
        let spring = (step.old_springs.bodies[index] + step.new_springs.bodies[index]) * 0.5;
        let constraint = numeric::sum_vec(&constraints[index], NumericStage::Telemetry)?;
        let drag = step.drag_impulses[index] * (1.0 / dt);
        let contact = numeric::finite_vec(
            step.contacts.impulses[index] * (1.0 / dt),
            NumericStage::Telemetry,
        )?;
        let net = numeric::sum_vec(
            &[
                gravity,
                input.external[index],
                spring,
                constraint,
                drag,
                contact,
            ],
            NumericStage::Telemetry,
        )?;
        let angular_kinetic = rotations
            .binary_search_by_key(&body.id, |rotation| rotation.body)
            .ok()
            .map_or(0.0, |index| rotations[index].kinetic_energy_j);
        let kinetic = numeric::sum(
            &[
                energy::kinetic(body.mass_kg, body.velocity_m_s)?,
                angular_kinetic,
            ],
            NumericStage::Telemetry,
        )?;
        let potential = if dynamic {
            energy::gravitational(body.mass_kg, settings.gravity_m_s2, body.position_m)?
        } else {
            0.0
        };
        let acceleration = numeric::finite_vec(
            (body.velocity_m_s - before[index].velocity_m_s) * (1.0 / dt),
            NumericStage::Telemetry,
        )?;
        numeric::finite(kinetic, NumericStage::Telemetry)?;
        numeric::finite(potential, NumericStage::Telemetry)?;
        body_reports.push(BodyTelemetry {
            id: body.id,
            position_m: body.position_m,
            velocity_m_s: body.velocity_m_s,
            acceleration_m_s2: acceleration,
            gravity_force_n: gravity,
            external_force_n: input.external[index],
            spring_force_n: spring,
            constraint_force_n: constraint,
            drag_force_n: drag,
            contact_impulse_ns: step.contacts.impulses[index],
            contact_force_n: contact,
            contact_position_correction_m: step.contacts.position_correction[index],
            net_force_n: net,
            kinetic_energy_j: kinetic,
            gravitational_energy_j: potential,
        });
    }
    let rotational_kinetic_j = numeric::sum(
        &rotations
            .iter()
            .map(|state| state.kinetic_energy_j)
            .collect::<Vec<_>>(),
        NumericStage::Telemetry,
    )?;
    let kinetic_j = numeric::sum(
        &body_reports
            .iter()
            .map(|body| body.kinetic_energy_j)
            .collect::<Vec<_>>(),
        NumericStage::Telemetry,
    )?;
    let gravitational_j = numeric::sum(
        &body_reports
            .iter()
            .map(|body| body.gravitational_energy_j)
            .collect::<Vec<_>>(),
        NumericStage::Telemetry,
    )?;
    let elastic_j = numeric::sum(
        &link_reports
            .iter()
            .map(|link| link.elastic_energy_j)
            .collect::<Vec<_>>(),
        NumericStage::Telemetry,
    )?;
    let total_j = numeric::sum(
        &[kinetic_j, gravitational_j, elastic_j],
        NumericStage::Telemetry,
    )?;
    Ok(StepReport {
        step_index: input.next_step,
        elapsed_s: input.elapsed,
        settings_revision: input.settings_revision,
        bodies: body_reports,
        links: link_reports,
        contacts: step.contacts.reports.clone(),
        rotations,
        energy: EnergyTelemetry {
            kinetic_j,
            rotational_kinetic_j,
            gravitational_j,
            elastic_j,
            total_j,
            drag_dissipated_j: step.drag_loss,
            contact_projection_energy_change_j: step.contacts.projection_energy_change,
            contact_solve_kinetic_change_j: step.contacts.solve_kinetic_change,
        },
    })
}
