mod electrostatic_session;
mod fixed_step_clock;
mod mechanics_session;
mod physics_session;
mod thermal_session;
mod wave_session;

use crate::presentation::ui::{
    DesktopUiModel, PhysSubdomain, PhysicsEditorCommand, PhysicsEditorOutcome, PhysicsSnapshotRef,
    PlaybackRate, ProjectTemplate,
};

use physics_session::PhysicsSession;

/// Application-owned bridge between navigation and canonical domain worlds.
///
/// Presentation never owns or mutates scientific state. It sends commands to
/// this model and receives immutable snapshots suitable for drawing.
#[derive(Debug)]
struct DesktopAppModel {
    active_session: Option<PhysicsSession>,
    view_baseline: Option<PhysicsSession>,
    physics_error: Option<String>,
    playback_rate: PlaybackRate,
    simulation_view_active: bool,
}

impl Default for DesktopAppModel {
    fn default() -> Self {
        Self {
            active_session: None,
            view_baseline: None,
            physics_error: None,
            playback_rate: PlaybackRate::Paused,
            simulation_view_active: false,
        }
    }
}

impl DesktopUiModel for DesktopAppModel {
    fn open_physics_project(
        &mut self,
        subdomain: PhysSubdomain,
        template: ProjectTemplate,
    ) -> Result<(), String> {
        match PhysicsSession::new(subdomain, template) {
            Ok(session) => {
                self.active_session = Some(session);
                self.view_baseline = None;
                self.physics_error = None;
                self.playback_rate = PlaybackRate::Paused;
                self.simulation_view_active = false;
                Ok(())
            }
            Err(error) => {
                self.active_session = None;
                self.view_baseline = None;
                self.physics_error = Some(error.clone());
                self.playback_rate = PlaybackRate::Paused;
                self.simulation_view_active = false;
                Err(error)
            }
        }
    }

    fn reset_physics_project(&mut self) {
        if self.simulation_view_active {
            if let Some(baseline) = self.view_baseline.as_ref() {
                self.active_session = Some(baseline.clone());
                self.physics_error = None;
            }
            return;
        }
        let Some((subdomain, template)) =
            self.active_session.as_ref().map(PhysicsSession::selection)
        else {
            return;
        };
        match PhysicsSession::new(subdomain, template) {
            Ok(session) => {
                self.active_session = Some(session);
                self.view_baseline = None;
                self.physics_error = None;
            }
            Err(error) => {
                self.active_session = None;
                self.view_baseline = None;
                self.physics_error = Some(error);
                self.playback_rate = PlaybackRate::Paused;
                self.simulation_view_active = false;
            }
        }
    }

    fn edit_physics(
        &mut self,
        command: PhysicsEditorCommand,
    ) -> Result<PhysicsEditorOutcome, String> {
        if self.simulation_view_active {
            return Err("structural edits are disabled in View".to_owned());
        }
        let session = self
            .active_session
            .as_mut()
            .ok_or_else(|| "no Physics project is open".to_owned())?;
        match session.edit(command) {
            Ok(outcome) => {
                self.physics_error = None;
                Ok(outcome)
            }
            Err(error) => {
                self.physics_error = Some(error.clone());
                Err(error)
            }
        }
    }

    fn enter_physics_view(&mut self) -> Result<(), String> {
        let session = self
            .active_session
            .as_mut()
            .ok_or_else(|| "no Physics project is open".to_owned())?;
        session.reset_transient_runtime()?;
        self.view_baseline = self.active_session.clone();
        self.simulation_view_active = true;
        self.physics_error = None;
        self.playback_rate = if self.supports_playback() {
            PlaybackRate::Normal
        } else {
            PlaybackRate::Paused
        };
        Ok(())
    }

    fn apply_physics_view(&mut self) {
        if let Some(session) = self.active_session.as_mut()
            && let Err(error) = session.reset_transient_runtime()
        {
            self.physics_error = Some(error);
        } else {
            self.physics_error = None;
        }
        self.view_baseline = None;
        self.simulation_view_active = false;
        self.playback_rate = PlaybackRate::Paused;
    }

    fn discard_physics_view(&mut self) {
        if let Some(baseline) = self.view_baseline.take() {
            self.active_session = Some(baseline);
        }
        if let Some(session) = self.active_session.as_mut()
            && let Err(error) = session.reset_transient_runtime()
        {
            self.physics_error = Some(error);
        } else {
            self.physics_error = None;
        }
        self.simulation_view_active = false;
        self.playback_rate = PlaybackRate::Paused;
    }

    fn close_physics_project(&mut self) {
        self.active_session = None;
        self.view_baseline = None;
        self.physics_error = None;
        self.playback_rate = PlaybackRate::Paused;
        self.simulation_view_active = false;
    }

    fn set_playback_rate(&mut self, rate: PlaybackRate) {
        if self.simulation_view_active && self.supports_playback() {
            self.playback_rate = rate;
        }
    }

    fn playback_rate(&self) -> PlaybackRate {
        self.playback_rate
    }

    fn supports_playback(&self) -> bool {
        self.active_session
            .as_ref()
            .is_some_and(PhysicsSession::supports_playback)
    }

    fn advance(&mut self, wall_seconds: f64) {
        if !self.simulation_view_active {
            return;
        }
        let Some(session) = self.active_session.as_mut() else {
            return;
        };
        let scaled_wall_seconds = wall_seconds * self.playback_rate.multiplier();
        if let Err(error) = session.advance(scaled_wall_seconds) {
            self.physics_error = Some(error);
            self.playback_rate = PlaybackRate::Paused;
        }
    }

    fn physics_snapshot(&self) -> Option<PhysicsSnapshotRef<'_>> {
        self.active_session.as_ref().map(PhysicsSession::snapshot)
    }

    fn physics_error(&self) -> Option<&str> {
        self.physics_error.as_deref()
    }
}

/// Starts the fullscreen Sim;X application shell.
pub fn run() -> Result<(), String> {
    crate::presentation::ui::run(DesktopAppModel::default()).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::{DesktopAppModel, DesktopUiModel};
    use crate::presentation::ui::{
        PhysSubdomain, PhysicsEditorCommand, PhysicsEditorOutcome, PhysicsSnapshotRef,
        PlaybackRate, ProjectTemplate,
    };

    #[test]
    fn composition_root_opens_all_four_scientific_snapshot_types() {
        let mut model = DesktopAppModel::default();
        for subdomain in [
            PhysSubdomain::Mechanics,
            PhysSubdomain::Thermodynamics,
            PhysSubdomain::WavesAndOptics,
            PhysSubdomain::Electromagnetism,
        ] {
            model
                .open_physics_project(subdomain, ProjectTemplate::PrimaryLab)
                .expect("built-in project");
            let matches_subdomain = matches!(
                (subdomain, model.physics_snapshot()),
                (
                    PhysSubdomain::Mechanics,
                    Some(PhysicsSnapshotRef::Mechanics(_))
                ) | (
                    PhysSubdomain::Thermodynamics,
                    Some(PhysicsSnapshotRef::Thermodynamics(_))
                ) | (
                    PhysSubdomain::WavesAndOptics,
                    Some(PhysicsSnapshotRef::WavesAndOptics(_))
                ) | (
                    PhysSubdomain::Electromagnetism,
                    Some(PhysicsSnapshotRef::Electromagnetism(_))
                )
            );
            assert!(matches_subdomain);
            assert_eq!(model.physics_error(), None);
        }
    }

    #[test]
    fn editor_never_submits_domain_steps() {
        let mut model = DesktopAppModel::default();
        model
            .open_physics_project(PhysSubdomain::Mechanics, ProjectTemplate::PrimaryLab)
            .expect("project");
        model.set_playback_rate(PlaybackRate::Normal);
        model.advance(1.0);
        let Some(PhysicsSnapshotRef::Mechanics(snapshot)) = model.physics_snapshot() else {
            panic!("mechanics snapshot expected");
        };
        assert_eq!(snapshot.world.step.get(), 0);
    }

    #[test]
    fn view_runs_and_explicit_discard_restores_the_editor_baseline() {
        let mut model = DesktopAppModel::default();
        model
            .open_physics_project(PhysSubdomain::Mechanics, ProjectTemplate::PrimaryLab)
            .expect("project");
        model.enter_physics_view().expect("view");
        model.advance(0.1);
        let Some(PhysicsSnapshotRef::Mechanics(running)) = model.physics_snapshot() else {
            panic!("mechanics snapshot expected");
        };
        assert!(running.world.step.get() > 0);

        model.discard_physics_view();
        model.advance(1.0);
        let Some(PhysicsSnapshotRef::Mechanics(editor)) = model.physics_snapshot() else {
            panic!("mechanics snapshot expected");
        };
        assert_eq!(editor.world.step.get(), 0);
    }

    #[test]
    fn explicit_apply_keeps_the_final_view_state_as_editor_state() {
        let mut model = DesktopAppModel::default();
        model
            .open_physics_project(PhysSubdomain::Mechanics, ProjectTemplate::PrimaryLab)
            .expect("project");
        model.enter_physics_view().expect("view");
        model.advance(0.1);
        let Some(PhysicsSnapshotRef::Mechanics(running)) = model.physics_snapshot() else {
            panic!("mechanics snapshot expected");
        };
        let final_step = running.world.step;
        assert!(final_step.get() > 0);

        model.apply_physics_view();
        model.advance(1.0);
        let Some(PhysicsSnapshotRef::Mechanics(editor)) = model.physics_snapshot() else {
            panic!("mechanics snapshot expected");
        };
        assert_eq!(editor.world.step, final_step);
        assert_eq!(model.playback_rate(), PlaybackRate::Paused);
    }

    #[test]
    fn failed_project_open_clears_stale_view_lifecycle_state() {
        let mut model = DesktopAppModel::default();
        model
            .open_physics_project(PhysSubdomain::Mechanics, ProjectTemplate::PrimaryLab)
            .expect("project");
        model.enter_physics_view().expect("view");
        assert_eq!(model.playback_rate(), PlaybackRate::Normal);

        assert!(
            model
                .open_physics_project(PhysSubdomain::Relativity, ProjectTemplate::PrimaryLab)
                .is_err()
        );

        assert!(model.physics_snapshot().is_none());
        assert_eq!(model.playback_rate(), PlaybackRate::Paused);
        assert!(!model.simulation_view_active);
    }

    #[test]
    fn static_project_never_acquires_hidden_playback() {
        let mut model = DesktopAppModel::default();
        model
            .open_physics_project(PhysSubdomain::Electromagnetism, ProjectTemplate::PrimaryLab)
            .expect("project");
        model.enter_physics_view().expect("view");
        model.set_playback_rate(PlaybackRate::Fast);

        assert!(!model.supports_playback());
        assert_eq!(model.playback_rate(), PlaybackRate::Paused);
    }

    #[test]
    fn typed_editor_command_mutates_setup_without_stepping() {
        let mut model = DesktopAppModel::default();
        model
            .open_physics_project(PhysSubdomain::Mechanics, ProjectTemplate::Blank)
            .expect("project");

        let outcome = model
            .edit_physics(PhysicsEditorCommand::PlaceMechanicsBody {
                x_meters: 3.0,
                y_meters: -2.0,
            })
            .expect("body placement");
        let PhysicsEditorOutcome::BodyPlaced(entity) = outcome else {
            panic!("placed body expected");
        };
        model
            .edit_physics(PhysicsEditorCommand::SetMechanicsForce {
                entity,
                x_newtons: 2.0,
                y_newtons: 0.0,
            })
            .expect("persistent force");

        let Some(PhysicsSnapshotRef::Mechanics(editor)) = model.physics_snapshot() else {
            panic!("mechanics snapshot expected");
        };
        assert_eq!(editor.world.step.get(), 0);
        assert_eq!(editor.world.bodies.len(), 2);

        model.enter_physics_view().expect("view");
        model.advance(0.02);
        let Some(PhysicsSnapshotRef::Mechanics(view)) = model.physics_snapshot() else {
            panic!("mechanics snapshot expected");
        };
        let body = view
            .world
            .bodies
            .iter()
            .find(|body| body.id == entity)
            .expect("placed body");
        assert!(body.net_force.x().get() > 0.0);
    }

    #[test]
    fn apply_and_reenter_do_not_reuse_fractional_wall_time() {
        let mut model = DesktopAppModel::default();
        model
            .open_physics_project(PhysSubdomain::Mechanics, ProjectTemplate::Blank)
            .expect("project");
        model.enter_physics_view().expect("first view");
        model.advance(1.0 / 240.0);
        model.apply_physics_view();
        model.enter_physics_view().expect("second view");
        model.advance(1.0 / 240.0);

        let Some(PhysicsSnapshotRef::Mechanics(snapshot)) = model.physics_snapshot() else {
            panic!("mechanics snapshot expected");
        };
        assert_eq!(snapshot.world.step.get(), 0);
    }
}
