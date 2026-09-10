mod application;
pub(in crate::presentation) mod catalog;
pub(in crate::presentation) mod geometry;
pub(in crate::presentation) mod layout;
pub(in crate::presentation) mod state;

use std::collections::BTreeMap;

use crate::domains::phys::{
    electromagnetism::ElectrostaticSnapshot,
    mechanics::{ForceContribution, MechanicsSnapshot},
    thermodynamics::ThermalSnapshot,
    waves_optics::WaveSnapshot,
};
use crate::foundation::EntityId;

pub(crate) use application::run;
pub(crate) use catalog::{PhysSubdomain, PlaybackRate, ProjectTemplate};

pub(crate) enum PhysicsSnapshotRef<'a> {
    Mechanics(MechanicsProjectSnapshotRef<'a>),
    Thermodynamics(&'a ThermalSnapshot),
    WavesAndOptics(&'a WaveSnapshot),
    Electromagnetism(&'a ElectrostaticSnapshot),
}

/// Immutable Mechanics read model combining domain state and app-owned setup.
#[derive(Clone, Copy)]
pub(crate) struct MechanicsProjectSnapshotRef<'a> {
    pub(crate) world: &'a MechanicsSnapshot,
    pub(crate) persistent_forces: &'a BTreeMap<EntityId, ForceContribution>,
}

/// Typed Editor intent expressed in canonical SI units, never screen pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum PhysicsEditorCommand {
    PlaceMechanicsBody {
        x_meters: f64,
        y_meters: f64,
    },
    MoveMechanicsBody {
        entity: EntityId,
        x_meters: f64,
        y_meters: f64,
    },
    ScaleMechanicsBodyMass {
        entity: EntityId,
        scale: f64,
    },
    SetMechanicsForce {
        entity: EntityId,
        x_newtons: f64,
        y_newtons: f64,
    },
}

/// Confirmed app outcome used to synchronize presentation selection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PhysicsEditorOutcome {
    BodyPlaced(EntityId),
    Updated,
}

pub(crate) trait DesktopUiModel {
    fn open_physics_project(
        &mut self,
        subdomain: PhysSubdomain,
        template: ProjectTemplate,
    ) -> Result<(), String>;
    fn reset_physics_project(&mut self);
    fn edit_physics(
        &mut self,
        command: PhysicsEditorCommand,
    ) -> Result<PhysicsEditorOutcome, String>;
    fn enter_physics_view(&mut self) -> Result<(), String>;
    fn apply_physics_view(&mut self);
    fn discard_physics_view(&mut self);
    fn close_physics_project(&mut self);
    fn set_playback_rate(&mut self, rate: PlaybackRate);
    fn playback_rate(&self) -> PlaybackRate;
    fn supports_playback(&self) -> bool;
    fn advance(&mut self, wall_seconds: f64);
    fn physics_snapshot(&self) -> Option<PhysicsSnapshotRef<'_>>;
    fn physics_error(&self) -> Option<&str>;
}
