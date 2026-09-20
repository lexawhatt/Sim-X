use super::validation;
use crate::collision::model::{self as collision_model, BoundCollider};
use crate::mechanics::{
    constraints::{self, BoundLink},
    forces, integration,
    telemetry::{self, ReportIdentity},
};
use crate::rigid::model::Rotations;
use crate::{
    BodyDesc, BodyId, ColliderDesc, Error, ForceInput, LinkDesc, MAX_BODIES, MAX_COLLIDERS,
    MAX_FORCES, MAX_LINKS, NumericStage, PhysicsSettings, RotationDesc, SolverConfig, StepReport,
    numeric,
};

/// Immutable owned snapshot; reconstructible solver caches are not canonical.
#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    /// Canonical body states in stable ID order.
    pub bodies: Vec<BodyDesc>,
    /// Canonical relationships in stable ID order.
    pub links: Vec<LinkDesc>,
    /// Optional finite shape descriptions in stable body order.
    pub colliders: Vec<ColliderDesc>,
    /// Optional angular body states in stable body order.
    pub rotations: Vec<RotationDesc>,
    /// Current settings.
    pub settings: PhysicsSettings,
    /// Fixed numerical policy.
    pub solver: SolverConfig,
    /// Physical elapsed seconds.
    pub elapsed_s: f64,
    /// Successful fixed-step count.
    pub step_index: u64,
    /// Incremented only when settings genuinely change.
    pub settings_revision: u64,
    /// Latest report, cleared when settings change to avoid stale observations.
    pub last_report: Option<StepReport>,
}

/// Validated physical graph and its committed state. No mutable state escapes.
#[derive(Clone, Debug)]
pub struct World {
    settings: PhysicsSettings,
    solver: SolverConfig,
    bodies: Vec<BodyDesc>,
    links: Vec<LinkDesc>,
    bound_links: Vec<BoundLink>,
    colliders: Vec<ColliderDesc>,
    bound_colliders: Vec<BoundCollider>,
    rotations: Rotations,
    elapsed_s: f64,
    step_index: u64,
    settings_revision: u64,
    last_report: Option<StepReport>,
}

impl World {
    /// Validates and canonically orders an entire composition atomically.
    /// Slice budgets are checked before allocation or traversal. Rods must begin
    /// at their prescribed length with tangent relative velocity; no hidden
    /// projection silently edits initial conditions.
    pub fn new(
        settings: PhysicsSettings,
        solver: SolverConfig,
        bodies: &[BodyDesc],
        links: &[LinkDesc],
    ) -> Result<Self, Error> {
        Self::with_colliders(settings, solver, bodies, links, &[])
    }

    /// Constructs a finite-body translation-only world, preserving the point
    /// API when `colliders` is empty. One shape per body, at most MAX_COLLIDERS;
    /// overlapping pairs with a dynamic endpoint reject beyond tolerance.
    /// Fixed-fixed shape overlap is allowed as authored stationary geometry.
    pub fn with_colliders(
        settings: PhysicsSettings,
        solver: SolverConfig,
        bodies: &[BodyDesc],
        links: &[LinkDesc],
        colliders: &[ColliderDesc],
    ) -> Result<Self, Error> {
        Self::with_rigid_bodies(settings, solver, bodies, links, colliders, &[])
    }

    /// Adds explicit planar angular states and optional body-local spring anchors.
    /// Box collider angles are local offsets added to the body's world angle.
    /// Missing rotation records preserve legacy locked orientation; nonzero spring
    /// anchors require an explicit rotation record, including on fixed shapes.
    pub fn with_rigid_bodies(
        settings: PhysicsSettings,
        solver: SolverConfig,
        bodies: &[BodyDesc],
        links: &[LinkDesc],
        colliders: &[ColliderDesc],
        rotations: &[RotationDesc],
    ) -> Result<Self, Error> {
        if bodies.len() > MAX_BODIES {
            return Err(Error::BodyBudget);
        }
        if links.len() > MAX_LINKS {
            return Err(Error::LinkBudget);
        }
        if colliders.len() > MAX_COLLIDERS {
            return Err(Error::ColliderBudget);
        }
        if rotations.len() > MAX_BODIES {
            return Err(Error::RotationBudget);
        }
        validation::settings(settings)?;
        validation::solver(solver)?;
        let mut bodies = bodies.to_vec();
        let mut links = links.to_vec();
        bodies.sort_by_key(|body| body.id);
        links.sort_by_key(|link| link.id());
        for pair in bodies.windows(2) {
            if pair[0].id == pair[1].id {
                return Err(Error::DuplicateBody(pair[0].id));
            }
        }
        for pair in links.windows(2) {
            if pair[0].id() == pair[1].id() {
                return Err(Error::DuplicateLink(pair[0].id()));
            }
        }
        for &body in &bodies {
            body.validate()?;
        }
        let rotations = Rotations::bind(&bodies, rotations)?;
        let bound_links = validation::bind_links(&bodies, &links, solver, &rotations)?;
        constraints::validate_residuals(&bodies, &bound_links, solver, true)?;
        let bound_colliders = collision_model::bind(&bodies, colliders, solver, &rotations)?;
        let colliders = bound_colliders
            .iter()
            .map(|collider| collider.desc)
            .collect();
        Ok(Self {
            settings,
            solver,
            bodies,
            links,
            bound_links,
            colliders,
            bound_colliders,
            rotations,
            elapsed_s: 0.0,
            step_index: 0,
            settings_revision: 0,
            last_report: None,
        })
    }

    /// Read-only body states in stable ID order.
    pub fn bodies(&self) -> &[BodyDesc] {
        &self.bodies
    }
    /// Resolves a stable body identity without exposing mutable canonical data.
    pub fn body(&self, id: BodyId) -> Option<&BodyDesc> {
        self.bodies
            .binary_search_by_key(&id, |body| body.id)
            .ok()
            .map(|index| &self.bodies[index])
    }
    /// Read-only persistent relationships in stable ID order.
    pub fn links(&self) -> &[LinkDesc] {
        &self.links
    }
    /// Immutable optional finite shapes, ordered by their body identities.
    pub fn colliders(&self) -> &[ColliderDesc] {
        &self.colliders
    }
    /// Read-only angular body records, in body identity order.
    pub fn rotations(&self) -> &[RotationDesc] {
        &self.rotations.states
    }
    /// Resolves an optional angular state by stable body identity.
    pub fn rotation(&self, id: BodyId) -> Option<&RotationDesc> {
        self.rotations
            .states
            .binary_search_by_key(&id, |state| state.body)
            .ok()
            .map(|index| &self.rotations.states[index])
    }
    /// Current scene-wide physical settings.
    pub fn settings(&self) -> PhysicsSettings {
        self.settings
    }
    /// Immutable numerical policy.
    pub fn solver(&self) -> SolverConfig {
        self.solver
    }
    /// Physical time; reading it never advances simulation.
    pub fn elapsed_s(&self) -> f64 {
        self.elapsed_s
    }
    /// Number of successful committed steps.
    pub fn step_index(&self) -> u64 {
        self.step_index
    }
    /// Revision of the scene settings.
    pub fn settings_revision(&self) -> u64 {
        self.settings_revision
    }
    /// Latest observations, absent before stepping or after settings change.
    pub fn last_report(&self) -> Option<&StepReport> {
        self.last_report.as_ref()
    }
    /// An owned, bounded, renderer-independent copy of all canonical state.
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            bodies: self.bodies.clone(),
            links: self.links.clone(),
            colliders: self.colliders.clone(),
            rotations: self.rotations.states.clone(),
            settings: self.settings,
            solver: self.solver,
            elapsed_s: self.elapsed_s,
            step_index: self.step_index,
            settings_revision: self.settings_revision,
            last_report: self.last_report.clone(),
        }
    }

    /// Atomically replaces scene settings between steps. Returns false for an
    /// exact no-op; true increments the settings revision and clears telemetry.
    /// Positions, velocities, time and step count never change here.
    pub fn set_settings(&mut self, settings: PhysicsSettings) -> Result<bool, Error> {
        validation::settings(settings)?;
        if settings == self.settings {
            return Ok(false);
        }
        let next = self
            .settings_revision
            .checked_add(1)
            .ok_or(Error::CounterExhausted)?;
        self.settings = settings;
        self.settings_revision = next;
        self.last_report = None;
        Ok(true)
    }

    /// Advances one fixed step using Verlet/RATTLE and optional split contact
    /// cleanup with coupled frictionless impulses. Unsupported swept collisions
    /// reject rather than tunnelling; there is no hidden floor. Explicit angular
    /// components add torque integration and shared-point rotational contacts.
    /// All state, counters and diagnostics commit together or remain unchanged.
    /// Forces are step-local, finite and source-unique. Their hard budget is
    /// checked before any input traversal or candidate allocation.
    pub fn step(&mut self, forces: &[ForceInput]) -> Result<StepReport, Error> {
        if forces.len() > MAX_FORCES {
            return Err(Error::ForceBudget);
        }
        let dt = self.solver.fixed_dt_s;
        let next_step = self
            .step_index
            .checked_add(1)
            .ok_or(Error::CounterExhausted)?;
        let elapsed = numeric::finite(self.elapsed_s + dt, NumericStage::Time)?;
        if elapsed <= self.elapsed_s {
            return Err(Error::PrecisionLoss(NumericStage::Time));
        }
        let external = forces::external_forces(&self.bodies, forces)?;
        let candidate = integration::advance(
            &self.bodies,
            &self.bound_links,
            self.settings,
            self.solver,
            &external,
            &self.bound_colliders,
            &self.rotations,
        )?;
        let report = telemetry::report(
            &self.bodies,
            &self.bound_links,
            self.settings,
            self.solver,
            &candidate,
            &self.rotations,
            ReportIdentity {
                external: &external,
                next_step,
                elapsed,
                settings_revision: self.settings_revision,
            },
        )?;
        self.bodies = candidate.bodies;
        self.rotations = candidate.rotations;
        self.elapsed_s = elapsed;
        self.step_index = next_step;
        self.last_report = Some(report.clone());
        Ok(report)
    }
}

#[cfg(test)]
#[path = "world_tests.rs"]
mod tests;
