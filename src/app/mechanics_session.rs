use std::collections::BTreeMap;

use crate::{
    domains::phys::mechanics::{
        BodySpec, Force2, ForceContribution, ForceSourceId, MechanicsSnapshot, MechanicsWorld,
        Mobility, Position2, Velocity2,
    },
    foundation::{EntityId, Kilograms, Meters, Newtons, TimeStep},
    presentation::ui::ProjectTemplate,
};

use super::fixed_step_clock::FixedStepClock;

const FIXED_STEP_SECONDS: f64 = 1.0 / 120.0;
const MAX_FIXED_STEPS_PER_FRAME: usize = 8;
const MAX_WALL_DELTA_SECONDS: f64 = 0.1;
const EDITOR_FORCE_SOURCE: u64 = 1;

/// Fixed-step runtime adapter for the currently open Mechanics project.
///
/// Wall-clock time exists only here. The domain receives the same validated
/// `TimeStep` on every call, and a bounded accumulator prevents an inactive
/// window from producing unbounded catch-up work on resume.
#[derive(Clone, Debug)]
pub(super) struct MechanicsSession {
    template: ProjectTemplate,
    world: MechanicsWorld,
    snapshot: MechanicsSnapshot,
    persistent_forces: BTreeMap<EntityId, ForceContribution>,
    fixed_step: TimeStep,
    clock: FixedStepClock,
    faulted: bool,
}

impl MechanicsSession {
    pub(super) fn new(template: ProjectTemplate) -> Result<Self, String> {
        let fixed_step = TimeStep::new(FIXED_STEP_SECONDS).map_err(|error| error.to_string())?;
        let mut world = MechanicsWorld::new();

        let (mass_kg, force_newtons, position_x) = match template {
            ProjectTemplate::Blank => (1.0, None, 0.0),
            ProjectTemplate::PrimaryLab => (2.0, Some(0.8), -2.0),
            ProjectTemplate::SecondaryLab => {
                return Err("pendulum project requires the constraint solver gate".to_owned());
            }
        };
        let body = world
            .create_body(BodySpec::new(
                Position2::new(
                    Meters::new(position_x).map_err(|error| error.to_string())?,
                    Meters::ZERO,
                ),
                Velocity2::ZERO,
                Kilograms::new(mass_kg).map_err(|error| error.to_string())?,
                Mobility::Dynamic,
            ))
            .map_err(|error| error.to_string())?;
        let contribution = force_newtons
            .map(|force_x| -> Result<ForceContribution, String> {
                Ok(ForceContribution::new(
                    body,
                    ForceSourceId::new(EDITOR_FORCE_SOURCE).map_err(|error| error.to_string())?,
                    Force2::new(
                        Newtons::new(force_x).map_err(|error| error.to_string())?,
                        Newtons::ZERO,
                    ),
                ))
            })
            .transpose()?;
        let persistent_forces = contribution
            .map(|contribution| (contribution.target, contribution))
            .into_iter()
            .collect();
        let snapshot = world.snapshot();

        Ok(Self {
            template,
            world,
            snapshot,
            persistent_forces,
            fixed_step,
            clock: FixedStepClock::new(
                FIXED_STEP_SECONDS,
                MAX_FIXED_STEPS_PER_FRAME,
                MAX_WALL_DELTA_SECONDS,
            )?,
            faulted: false,
        })
    }

    pub(super) const fn template(&self) -> ProjectTemplate {
        self.template
    }

    pub(super) const fn snapshot(&self) -> &MechanicsSnapshot {
        &self.snapshot
    }

    pub(super) const fn persistent_forces(&self) -> &BTreeMap<EntityId, ForceContribution> {
        &self.persistent_forces
    }

    /// Creates a default dynamic `1 kg` body without advancing simulation time.
    pub(super) fn place_body(&mut self, position: Position2) -> Result<EntityId, String> {
        let entity = self
            .world
            .create_body(BodySpec::new(
                position,
                Velocity2::ZERO,
                Kilograms::new(1.0).map_err(|error| error.to_string())?,
                Mobility::Dynamic,
            ))
            .map_err(|error| error.to_string())?;
        self.refresh_snapshot();
        Ok(entity)
    }

    /// Moves an existing body to an Editor-authored position in meters.
    pub(super) fn move_body(
        &mut self,
        entity: EntityId,
        position: Position2,
    ) -> Result<(), String> {
        self.world
            .set_body_position(entity, position)
            .map_err(|error| error.to_string())?;
        self.refresh_snapshot();
        Ok(())
    }

    /// Multiplies an existing body's mass by a finite positive scale.
    pub(super) fn scale_body_mass(&mut self, entity: EntityId, scale: f64) -> Result<(), String> {
        if !scale.is_finite() || scale <= 0.0 {
            return Err("mass scale must be finite and strictly positive".to_owned());
        }
        let current_mass = self
            .snapshot
            .bodies
            .iter()
            .find(|body| body.id == entity)
            .ok_or_else(|| format!("mechanics body {} does not exist", entity.get()))?
            .mass
            .get();
        let mass = Kilograms::new(current_mass * scale).map_err(|error| error.to_string())?;

        self.world
            .set_body_mass(entity, mass)
            .map_err(|error| error.to_string())?;
        self.refresh_snapshot();
        Ok(())
    }

    /// Replaces one body's persistent Editor force for future View steps.
    ///
    /// Exact zero clears the definition so an invisible source cannot consume
    /// the bounded per-step contribution budget. This prototype intentionally
    /// supports one semantic Editor force per body.
    pub(super) fn set_force(&mut self, entity: EntityId, force: Force2) -> Result<(), String> {
        let body = self
            .snapshot
            .bodies
            .iter()
            .find(|body| body.id == entity)
            .ok_or_else(|| format!("force target {} does not exist", entity.get()))?;
        if body.mobility != Mobility::Dynamic {
            return Err(format!(
                "fixed body {} cannot receive ordinary force",
                entity.get()
            ));
        }
        if force == Force2::ZERO {
            return self.clear_force(entity);
        }

        let source = ForceSourceId::new(EDITOR_FORCE_SOURCE).map_err(|error| error.to_string())?;
        self.persistent_forces
            .insert(entity, ForceContribution::new(entity, source, force));
        Ok(())
    }

    /// Clears one body's persistent Editor force after validating the target.
    pub(super) fn clear_force(&mut self, entity: EntityId) -> Result<(), String> {
        if !self.snapshot.bodies.iter().any(|body| body.id == entity) {
            return Err(format!("force target {} does not exist", entity.get()));
        }
        self.persistent_forces.remove(&entity);
        Ok(())
    }

    /// Clears non-canonical wall-time residue and a prior runtime fault.
    ///
    /// Project setup, canonical world state, persistent forces, and the current
    /// immutable snapshot are preserved. The app calls this at the
    /// Editor-to-View checkpoint so View always begins at a fixed-step boundary.
    pub(super) fn reset_transient_runtime(&mut self) -> Result<(), String> {
        self.clock = Self::new_clock()?;
        self.faulted = false;
        Ok(())
    }

    pub(super) fn advance(&mut self, wall_seconds: f64) -> Result<(), String> {
        if self.faulted || !wall_seconds.is_finite() || wall_seconds <= 0.0 {
            return Ok(());
        }

        let steps = self.clock.take_steps(wall_seconds);
        let contributions: Vec<_> = self.persistent_forces.values().copied().collect();
        for _ in 0..steps {
            match self.world.step(self.fixed_step, &contributions) {
                Ok(report) => self.snapshot = report.snapshot,
                Err(error) => {
                    self.faulted = true;
                    return Err(format!(
                        "mechanics session paused after domain error: {error}"
                    ));
                }
            }
        }
        Ok(())
    }

    fn refresh_snapshot(&mut self) {
        self.snapshot = self.world.snapshot();
    }

    fn new_clock() -> Result<FixedStepClock, String> {
        FixedStepClock::new(
            FIXED_STEP_SECONDS,
            MAX_FIXED_STEPS_PER_FRAME,
            MAX_WALL_DELTA_SECONDS,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{FIXED_STEP_SECONDS, MechanicsSession};
    use crate::{
        domains::phys::mechanics::{Force2, Position2},
        foundation::{EntityId, Meters, Newtons},
        presentation::ui::ProjectTemplate,
    };

    fn position(x: f64, y: f64) -> Position2 {
        Position2::new(
            Meters::new(x).expect("finite x"),
            Meters::new(y).expect("finite y"),
        )
    }

    fn force(x: f64, y: f64) -> Force2 {
        Force2::new(
            Newtons::new(x).expect("finite x force"),
            Newtons::new(y).expect("finite y force"),
        )
    }

    #[test]
    fn newton_project_advances_only_by_fixed_steps() {
        let mut session =
            MechanicsSession::new(ProjectTemplate::PrimaryLab).expect("valid built-in project");

        session
            .advance(FIXED_STEP_SECONDS * 3.5)
            .expect("fixed steps");

        assert_eq!(session.snapshot().step.get(), 3);
        assert_eq!(
            session.snapshot().elapsed.get().to_bits(),
            (FIXED_STEP_SECONDS * 3.0).to_bits()
        );
        assert!(session.snapshot().bodies[0].position.x().get() > -2.0);
    }

    #[test]
    fn catch_up_work_is_bounded_per_frame() {
        let mut session =
            MechanicsSession::new(ProjectTemplate::Blank).expect("valid built-in project");

        session.advance(60.0).expect("bounded catch-up");

        assert_eq!(session.snapshot().step.get(), 8);
    }

    #[test]
    fn pendulum_is_not_silently_approximated() {
        assert!(MechanicsSession::new(ProjectTemplate::SecondaryLab).is_err());
    }

    #[test]
    fn editor_body_commands_change_setup_without_stepping() {
        let mut session =
            MechanicsSession::new(ProjectTemplate::Blank).expect("valid built-in project");
        let before = session.snapshot().clone();

        let entity = session.place_body(position(4.0, -3.0)).expect("place body");
        session
            .move_body(entity, position(7.0, 2.0))
            .expect("move body");
        session
            .scale_body_mass(entity, 2.0)
            .expect("scale body mass");

        let after = session.snapshot();
        let body = after
            .bodies
            .iter()
            .find(|body| body.id == entity)
            .expect("edited body");
        assert_eq!(after.step, before.step);
        assert_eq!(after.elapsed, before.elapsed);
        assert_eq!(after.revision.get(), before.revision.get() + 3);
        assert_eq!(body.position, position(7.0, 2.0));
        assert_eq!(body.mass.get(), 2.0);
    }

    #[test]
    fn persistent_force_drives_view_and_replacement_does_not_accumulate() {
        let mut session =
            MechanicsSession::new(ProjectTemplate::Blank).expect("valid built-in project");
        let entity = session.snapshot().bodies[0].id;
        session.set_force(entity, force(1.0, 0.0)).expect("force");
        session
            .set_force(entity, force(2.0, 0.0))
            .expect("replace force");

        session.advance(FIXED_STEP_SECONDS).expect("one view step");

        assert_eq!(session.persistent_forces.len(), 1);
        assert_eq!(session.snapshot().step.get(), 1);
        assert_eq!(session.snapshot().bodies[0].net_force, force(2.0, 0.0));
        assert!(session.snapshot().bodies[0].position.x().get() > 0.0);
    }

    #[test]
    fn zero_force_clears_the_persistent_definition() {
        let mut session =
            MechanicsSession::new(ProjectTemplate::Blank).expect("valid built-in project");
        let entity = session.snapshot().bodies[0].id;
        session.set_force(entity, force(2.0, 0.0)).expect("force");

        session
            .set_force(entity, Force2::ZERO)
            .expect("zero clears force");

        assert!(session.persistent_forces.is_empty());
        session.advance(FIXED_STEP_SECONDS).expect("unforced step");
        assert_eq!(session.snapshot().bodies[0].net_force, Force2::ZERO);
    }

    #[test]
    fn invalid_and_missing_edits_are_atomic() {
        let mut session =
            MechanicsSession::new(ProjectTemplate::Blank).expect("valid built-in project");
        let entity = session.snapshot().bodies[0].id;
        session.set_force(entity, force(3.0, 0.0)).expect("force");
        let before_snapshot = session.snapshot().clone();
        let before_forces = session.persistent_forces.clone();
        let missing = EntityId::new(u64::MAX).expect("non-zero ID");

        assert!(session.scale_body_mass(entity, f64::NAN).is_err());
        assert!(session.move_body(missing, position(1.0, 1.0)).is_err());
        assert!(session.set_force(missing, force(1.0, 0.0)).is_err());
        assert!(session.clear_force(missing).is_err());

        assert_eq!(session.snapshot(), &before_snapshot);
        assert_eq!(session.persistent_forces, before_forces);
    }

    #[test]
    fn runtime_reset_discards_sub_step_residue_and_clears_fault() {
        let mut session =
            MechanicsSession::new(ProjectTemplate::Blank).expect("valid built-in project");
        let entity = session.snapshot().bodies[0].id;

        session
            .advance(FIXED_STEP_SECONDS * 0.75)
            .expect("fractional time");
        session
            .reset_transient_runtime()
            .expect("reset fixed-step clock");
        session
            .advance(FIXED_STEP_SECONDS * 0.5)
            .expect("new fractional time");
        assert_eq!(session.snapshot().step.get(), 0);

        session
            .scale_body_mass(entity, f64::from_bits(1))
            .expect("small positive mass");
        session
            .set_force(entity, force(f64::MAX, 0.0))
            .expect("finite force");
        assert!(session.advance(FIXED_STEP_SECONDS).is_err());
        assert!(session.faulted);

        session.clear_force(entity).expect("clear bad force");
        session
            .reset_transient_runtime()
            .expect("clear runtime fault");
        session.advance(FIXED_STEP_SECONDS).expect("healthy step");
        assert!(!session.faulted);
        assert_eq!(session.snapshot().step.get(), 1);
    }
}
