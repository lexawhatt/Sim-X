use crate::presentation::ui::{
    MechanicsProjectSnapshotRef, PhysSubdomain, PhysicsSnapshotRef, ProjectTemplate,
};
use crate::{
    domains::phys::mechanics::{Force2, Position2},
    foundation::{Meters, Newtons},
    presentation::ui::{PhysicsEditorCommand, PhysicsEditorOutcome},
};

use super::{
    electrostatic_session::ElectrostaticSession, mechanics_session::MechanicsSession,
    thermal_session::ThermalSession, wave_session::WaveSession,
};

/// App-owned active project runtime without a universal scientific state type.
#[derive(Clone, Debug)]
pub(super) enum PhysicsSession {
    Mechanics(MechanicsSession),
    Thermodynamics(ThermalSession),
    WavesAndOptics(WaveSession),
    Electromagnetism(ElectrostaticSession),
}

impl PhysicsSession {
    pub(super) fn new(subdomain: PhysSubdomain, template: ProjectTemplate) -> Result<Self, String> {
        match subdomain {
            PhysSubdomain::Mechanics => MechanicsSession::new(template).map(Self::Mechanics),
            PhysSubdomain::Thermodynamics => {
                ThermalSession::new(template).map(Self::Thermodynamics)
            }
            PhysSubdomain::WavesAndOptics => WaveSession::new(template).map(Self::WavesAndOptics),
            PhysSubdomain::Electromagnetism => {
                ElectrostaticSession::new(template).map(Self::Electromagnetism)
            }
            PhysSubdomain::Relativity | PhysSubdomain::FluidDynamics | PhysSubdomain::Sandbox => {
                Err(format!(
                    "{} has no scientific project runtime yet",
                    subdomain.title()
                ))
            }
        }
    }

    pub(super) fn selection(&self) -> (PhysSubdomain, ProjectTemplate) {
        match self {
            Self::Mechanics(session) => (PhysSubdomain::Mechanics, session.template()),
            Self::Thermodynamics(session) => (PhysSubdomain::Thermodynamics, session.template()),
            Self::WavesAndOptics(session) => (PhysSubdomain::WavesAndOptics, session.template()),
            Self::Electromagnetism(session) => {
                (PhysSubdomain::Electromagnetism, session.template())
            }
        }
    }

    pub(super) fn advance(&mut self, wall_seconds: f64) -> Result<(), String> {
        match self {
            Self::Mechanics(session) => session.advance(wall_seconds),
            Self::Thermodynamics(session) => session.advance(wall_seconds),
            Self::WavesAndOptics(session) => session.advance(wall_seconds),
            Self::Electromagnetism(session) => {
                session.advance(wall_seconds);
                Ok(())
            }
        }
    }

    pub(super) fn edit(
        &mut self,
        command: PhysicsEditorCommand,
    ) -> Result<PhysicsEditorOutcome, String> {
        let Self::Mechanics(session) = self else {
            return Err("this curated lab has no construction commands yet".to_owned());
        };
        match command {
            PhysicsEditorCommand::PlaceMechanicsBody { x_meters, y_meters } => session
                .place_body(mechanics_position(x_meters, y_meters)?)
                .map(PhysicsEditorOutcome::BodyPlaced),
            PhysicsEditorCommand::MoveMechanicsBody {
                entity,
                x_meters,
                y_meters,
            } => session
                .move_body(entity, mechanics_position(x_meters, y_meters)?)
                .map(|()| PhysicsEditorOutcome::Updated),
            PhysicsEditorCommand::ScaleMechanicsBodyMass { entity, scale } => session
                .scale_body_mass(entity, scale)
                .map(|()| PhysicsEditorOutcome::Updated),
            PhysicsEditorCommand::SetMechanicsForce {
                entity,
                x_newtons,
                y_newtons,
            } => session
                .set_force(entity, mechanics_force(x_newtons, y_newtons)?)
                .map(|()| PhysicsEditorOutcome::Updated),
        }
    }

    pub(super) fn reset_transient_runtime(&mut self) -> Result<(), String> {
        match self {
            Self::Mechanics(session) => session.reset_transient_runtime(),
            Self::Thermodynamics(session) => session.reset_transient_runtime(),
            Self::WavesAndOptics(session) => session.reset_transient_runtime(),
            Self::Electromagnetism(_) => Ok(()),
        }
    }

    pub(super) const fn supports_playback(&self) -> bool {
        !matches!(self, Self::Electromagnetism(_))
    }

    pub(super) fn snapshot(&self) -> PhysicsSnapshotRef<'_> {
        match self {
            Self::Mechanics(session) => {
                PhysicsSnapshotRef::Mechanics(MechanicsProjectSnapshotRef {
                    world: session.snapshot(),
                    persistent_forces: session.persistent_forces(),
                })
            }
            Self::Thermodynamics(session) => PhysicsSnapshotRef::Thermodynamics(session.snapshot()),
            Self::WavesAndOptics(session) => PhysicsSnapshotRef::WavesAndOptics(session.snapshot()),
            Self::Electromagnetism(session) => {
                PhysicsSnapshotRef::Electromagnetism(session.snapshot())
            }
        }
    }
}

fn mechanics_position(x_meters: f64, y_meters: f64) -> Result<Position2, String> {
    Ok(Position2::new(
        Meters::new(x_meters).map_err(|error| error.to_string())?,
        Meters::new(y_meters).map_err(|error| error.to_string())?,
    ))
}

fn mechanics_force(x_newtons: f64, y_newtons: f64) -> Result<Force2, String> {
    Ok(Force2::new(
        Newtons::new(x_newtons).map_err(|error| error.to_string())?,
        Newtons::new(y_newtons).map_err(|error| error.to_string())?,
    ))
}
