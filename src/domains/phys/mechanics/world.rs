use std::collections::{BTreeMap, BTreeSet};

use crate::foundation::{EntityId, Seconds, StepIndex, TimeStep, WorldRevision};

use super::{
    ArithmeticOperation, BodySnapshot, BodySpec, CounterKind, Force2, ForceContribution,
    ForceSourceId, MechanicsError, MechanicsEvent, MechanicsEventKind, MechanicsSnapshot, Mobility,
    StepReport,
    body::Body,
    capability::{CapabilityRejection, ForceReceiver},
    integration,
    numeric::checked_add_progress,
    summation::reduce_forces,
};

/// Maximum number of canonical bodies and body snapshots in one Mechanics world.
pub const MAX_BODIES: usize = 4_096;
/// Maximum number of force contributions accepted by one fixed step.
pub const MAX_FORCE_CONTRIBUTIONS_PER_STEP: usize = 16_384;
/// Maximum number of typed events one successful fixed step can emit.
pub const MAX_EVENTS_PER_STEP: usize = MAX_FORCE_CONTRIBUTIONS_PER_STEP + MAX_BODIES + 1;

/// Canonical owner of deterministic translational Mechanics state.
///
/// The world exposes validated commands and immutable snapshots. Canonical body
/// storage is private and never shared with presentation code.
#[derive(Clone, Debug)]
pub struct MechanicsWorld {
    bodies: BTreeMap<EntityId, Body>,
    next_entity_raw: Option<u64>,
    step: StepIndex,
    revision: WorldRevision,
    elapsed: Seconds,
}

impl MechanicsWorld {
    /// Constructs an empty world at step and revision zero.
    pub fn new() -> Self {
        Self {
            bodies: BTreeMap::new(),
            next_entity_raw: Some(1),
            step: StepIndex::ZERO,
            revision: WorldRevision::ZERO,
            elapsed: Seconds::ZERO,
        }
    }

    /// Atomically creates one body and returns its stable identity.
    ///
    /// The body limit, ID allocator, and world revision are checked before the
    /// canonical map changes.
    pub fn create_body(&mut self, spec: BodySpec) -> Result<EntityId, MechanicsError> {
        if spec.mobility == Mobility::Fixed && spec.velocity != super::Velocity2::ZERO {
            return Err(MechanicsError::FixedBodyHasVelocity);
        }
        if self.bodies.len() >= MAX_BODIES {
            return Err(MechanicsError::EntityCapacityReached {
                capacity: MAX_BODIES,
            });
        }

        let next_revision =
            self.revision
                .checked_next()
                .ok_or(MechanicsError::CounterExhausted {
                    counter: CounterKind::WorldRevision,
                })?;
        let entity_raw = self
            .next_entity_raw
            .ok_or(MechanicsError::EntityIdExhausted)?;
        let id = EntityId::new(entity_raw)?;
        let following_entity_raw = entity_raw.checked_add(1);

        self.bodies.insert(id, Body::new(id, spec));
        self.next_entity_raw = following_entity_raw;
        self.revision = next_revision;
        Ok(id)
    }

    /// Atomically changes one body's world position in meters for Editor setup.
    ///
    /// The operation preserves mass, mobility, and velocity, clears outputs
    /// produced by the previous simulation step, and increments only the world
    /// revision. A missing body or exhausted revision leaves the world unchanged.
    pub fn set_body_position(
        &mut self,
        entity: EntityId,
        position: super::Position2,
    ) -> Result<(), MechanicsError> {
        let next_revision =
            self.revision
                .checked_next()
                .ok_or(MechanicsError::CounterExhausted {
                    counter: CounterKind::WorldRevision,
                })?;
        let body = self
            .bodies
            .get_mut(&entity)
            .ok_or(MechanicsError::MissingBody { entity })?;
        body.position = position;
        body.clear_step_outputs();
        self.revision = next_revision;
        Ok(())
    }

    /// Atomically changes one body's finite positive mass for Editor setup.
    ///
    /// The operation preserves pose, mobility, and velocity, clears outputs
    /// produced by the previous simulation step, and increments only the world
    /// revision. A missing body or exhausted revision leaves the world unchanged.
    pub fn set_body_mass(
        &mut self,
        entity: EntityId,
        mass: crate::foundation::Kilograms,
    ) -> Result<(), MechanicsError> {
        let next_revision =
            self.revision
                .checked_next()
                .ok_or(MechanicsError::CounterExhausted {
                    counter: CounterKind::WorldRevision,
                })?;
        let body = self
            .bodies
            .get_mut(&entity)
            .ok_or(MechanicsError::MissingBody { entity })?;
        body.mass = mass;
        body.clear_step_outputs();
        self.revision = next_revision;
        Ok(())
    }

    /// Advances every dynamic body by one fixed step and commits atomically.
    ///
    /// Contributions are one-step inputs. They are canonicalized by
    /// `(target, source)`, revalidated against the current world, and never
    /// retained for a later call. On any error all canonical fields remain
    /// unchanged and no events are returned.
    pub fn step(
        &mut self,
        dt: TimeStep,
        contributions: &[ForceContribution],
    ) -> Result<StepReport, MechanicsError> {
        if contributions.len() > MAX_FORCE_CONTRIBUTIONS_PER_STEP {
            return Err(MechanicsError::ContributionBudgetExceeded {
                submitted: contributions.len(),
                maximum: MAX_FORCE_CONTRIBUTIONS_PER_STEP,
            });
        }
        let next_step = self
            .step
            .checked_next()
            .ok_or(MechanicsError::CounterExhausted {
                counter: CounterKind::Step,
            })?;
        let next_revision =
            self.revision
                .checked_next()
                .ok_or(MechanicsError::CounterExhausted {
                    counter: CounterKind::WorldRevision,
                })?;
        let next_elapsed_raw = checked_add_progress(
            self.elapsed.get(),
            dt.get(),
            ArithmeticOperation::ElapsedTime,
            None,
        )?;
        let next_elapsed =
            Seconds::new(next_elapsed_raw).map_err(|_| MechanicsError::NonFiniteArithmetic {
                operation: ArithmeticOperation::ElapsedTime,
                entity: None,
            })?;

        let ordered = canonicalize_contributions(contributions)?;
        self.validate_force_targets(&ordered)?;
        let net_forces = reduce_forces(&ordered)?;

        let mut candidate_bodies = self.bodies.clone();
        let mut dynamic_bodies = 0usize;
        for (entity, body) in &mut candidate_bodies {
            match body.mobility {
                Mobility::Dynamic => {
                    let net_force = net_forces.get(entity).copied().unwrap_or(Force2::ZERO);
                    let integrated = {
                        let receiver =
                            ForceReceiver::project(body).map_err(map_capability_error)?;
                        integration::integrate(&receiver, net_force, dt)?
                    };
                    body.commit_integrated(
                        integrated.position,
                        integrated.velocity,
                        integrated.acceleration,
                        net_force,
                    );
                    dynamic_bodies += 1;
                }
                Mobility::Fixed => body.clear_step_outputs(),
            }
        }

        self.bodies = candidate_bodies;
        self.step = next_step;
        self.revision = next_revision;
        self.elapsed = next_elapsed;

        let events = self.build_events(&ordered, dynamic_bodies);
        Ok(StepReport {
            snapshot: self.snapshot(),
            events,
        })
    }

    /// Builds a bounded immutable snapshot without advancing the world.
    pub fn snapshot(&self) -> MechanicsSnapshot {
        let bodies = self.bodies.values().map(BodySnapshot::from).collect();
        MechanicsSnapshot {
            step: self.step,
            revision: self.revision,
            elapsed: self.elapsed,
            bodies,
        }
    }

    fn validate_force_targets(
        &self,
        ordered: &BTreeMap<(EntityId, ForceSourceId), Force2>,
    ) -> Result<(), MechanicsError> {
        for &(target, _) in ordered.keys() {
            let body = self
                .bodies
                .get(&target)
                .ok_or(MechanicsError::MissingForceTarget { target })?;
            ForceReceiver::project(body).map_err(map_capability_error)?;
        }
        Ok(())
    }

    fn build_events(
        &self,
        ordered: &BTreeMap<(EntityId, ForceSourceId), Force2>,
        dynamic_bodies: usize,
    ) -> Vec<MechanicsEvent> {
        let mut events = Vec::with_capacity(ordered.len() + dynamic_bodies + 1);
        let envelope = |kind| MechanicsEvent {
            step: self.step,
            revision: self.revision,
            elapsed: self.elapsed,
            kind,
        };

        for (&(target, source), &force) in ordered {
            events.push(envelope(MechanicsEventKind::ForceAccepted {
                target,
                source,
                force,
            }));
        }
        for body in self
            .bodies
            .values()
            .filter(|body| body.mobility == Mobility::Dynamic)
        {
            events.push(envelope(MechanicsEventKind::BodyIntegrated {
                entity: body.id,
                position: body.position,
                velocity: body.velocity,
                acceleration: body.last_acceleration,
                net_force: body.last_net_force,
            }));
        }
        events.push(envelope(MechanicsEventKind::StepCommitted {
            dynamic_bodies,
            accepted_forces: ordered.len(),
        }));
        events
    }
}

impl Default for MechanicsWorld {
    fn default() -> Self {
        Self::new()
    }
}

fn canonicalize_contributions(
    contributions: &[ForceContribution],
) -> Result<BTreeMap<(EntityId, ForceSourceId), Force2>, MechanicsError> {
    let mut ordered = BTreeMap::new();
    let mut duplicates = BTreeSet::new();

    for contribution in contributions {
        let key = (contribution.target, contribution.source);
        if ordered.insert(key, contribution.force).is_some() {
            duplicates.insert(key);
        }
    }

    if let Some(&(target, source)) = duplicates.first() {
        return Err(MechanicsError::DuplicateForceSource { target, source });
    }
    Ok(ordered)
}

fn map_capability_error(rejection: CapabilityRejection) -> MechanicsError {
    match rejection {
        CapabilityRejection::FixedBody { entity } => {
            MechanicsError::ForceTargetIsFixed { target: entity }
        }
    }
}

#[cfg(test)]
mod internal_tests {
    use crate::foundation::{EntityId, Kilograms, Meters, TimeStep, WorldRevision};

    use crate::domains::phys::mechanics::{
        BodySpec, Force2, ForceContribution, ForceSourceId, MechanicsError, MechanicsWorld,
        Mobility, Position2, Velocity2,
    };

    fn body_spec() -> BodySpec {
        BodySpec::new(
            Position2::ZERO,
            Velocity2::ZERO,
            Kilograms::new(1.0).expect("valid mass"),
            Mobility::Dynamic,
        )
    }

    #[test]
    fn exhausted_revision_rejects_creation_atomically() {
        let mut world = MechanicsWorld::new();
        world.revision = WorldRevision::new(u64::MAX);
        let before = world.snapshot();

        assert!(matches!(
            world.create_body(body_spec()),
            Err(MechanicsError::CounterExhausted { .. })
        ));
        assert_eq!(world.snapshot(), before);
        assert_eq!(world.next_entity_raw, Some(1));
    }

    #[test]
    fn allocator_uses_maximum_id_once_then_becomes_exhausted() {
        let mut world = MechanicsWorld::new();
        world.next_entity_raw = Some(u64::MAX);

        let last = world
            .create_body(body_spec())
            .expect("last valid entity ID");
        assert_eq!(last.get(), u64::MAX);
        assert_eq!(world.next_entity_raw, None);
        let before = world.snapshot();

        assert_eq!(
            world.create_body(body_spec()),
            Err(MechanicsError::EntityIdExhausted)
        );
        assert_eq!(world.snapshot(), before);
        assert_eq!(world.next_entity_raw, None);
    }

    #[test]
    fn editor_body_edits_advance_only_revision_and_clear_step_outputs() {
        let mut world = MechanicsWorld::new();
        let entity = world.create_body(body_spec()).expect("body");
        let force = ForceContribution::new(
            entity,
            ForceSourceId::new(1).expect("source"),
            Force2::new(
                crate::foundation::Newtons::new(2.0).expect("force"),
                crate::foundation::Newtons::ZERO,
            ),
        );
        world
            .step(TimeStep::new(0.5).expect("dt"), &[force])
            .expect("step");
        let stepped = world.snapshot();

        let position = Position2::new(Meters::new(9.0).expect("x"), Meters::ZERO);
        world
            .set_body_position(entity, position)
            .expect("position edit");
        world
            .set_body_mass(entity, Kilograms::new(4.0).expect("mass"))
            .expect("mass edit");
        let edited = world.snapshot();

        assert_eq!(edited.step, stepped.step);
        assert_eq!(edited.elapsed, stepped.elapsed);
        assert_eq!(edited.revision.get(), stepped.revision.get() + 2);
        assert_eq!(edited.bodies[0].position, position);
        assert_eq!(edited.bodies[0].mass.get(), 4.0);
        assert_eq!(edited.bodies[0].velocity, stepped.bodies[0].velocity);
        assert_eq!(edited.bodies[0].net_force, Force2::ZERO);
        assert_eq!(
            edited.bodies[0].acceleration,
            super::super::Acceleration2::ZERO
        );
    }

    #[test]
    fn missing_editor_body_edit_is_atomic() {
        let mut world = MechanicsWorld::new();
        world.create_body(body_spec()).expect("body");
        let before = world.snapshot();
        let missing = EntityId::new(99).expect("id");

        assert_eq!(
            world.set_body_position(missing, Position2::ZERO),
            Err(MechanicsError::MissingBody { entity: missing })
        );
        assert_eq!(
            world.set_body_mass(missing, Kilograms::new(2.0).expect("mass")),
            Err(MechanicsError::MissingBody { entity: missing })
        );
        assert_eq!(world.snapshot(), before);
    }
}
