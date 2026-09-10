use crate::{
    domains::phys::electromagnetism::{
        Coulombs, ElectrostaticSnapshot, ElectrostaticWorld, PointChargeSpec, Position2,
    },
    foundation::{Meters, PhysicalConstants},
    presentation::ui::ProjectTemplate,
};

/// App-owned constants and immutable evaluation for one electrostatic project.
#[derive(Clone, Debug)]
pub(super) struct ElectrostaticSession {
    template: ProjectTemplate,
    snapshot: ElectrostaticSnapshot,
}

impl ElectrostaticSession {
    pub(super) fn new(template: ProjectTemplate) -> Result<Self, String> {
        let mut world = ElectrostaticWorld::new();
        match template {
            ProjectTemplate::Blank => {}
            ProjectTemplate::PrimaryLab => build_dipole(&mut world)?,
            ProjectTemplate::SecondaryLab => build_monopole_field_map(&mut world)?,
        }
        let constants = PhysicalConstants::codata_2022();
        let snapshot = world
            .evaluate(&constants)
            .map_err(|error| error.to_string())?;
        Ok(Self { template, snapshot })
    }

    pub(super) const fn template(&self) -> ProjectTemplate {
        self.template
    }

    pub(super) const fn snapshot(&self) -> &ElectrostaticSnapshot {
        &self.snapshot
    }

    pub(super) const fn advance(&mut self, _wall_seconds: f64) {}
}

fn build_dipole(world: &mut ElectrostaticWorld) -> Result<(), String> {
    for (x, charge) in [(-1.0, 1.0e-9), (1.0, -1.0e-9)] {
        world
            .create_charge(PointChargeSpec::new(
                position(x, 0.0)?,
                Coulombs::new(charge).map_err(|error| error.to_string())?,
            ))
            .map_err(|error| error.to_string())?;
    }
    for y_index in -3..=3 {
        for x_index in -5..=5 {
            let x = f64::from(x_index) * 0.5;
            let y = f64::from(y_index) * 0.5;
            if y == 0.0 && (x == -1.0 || x == 1.0) {
                continue;
            }
            world
                .create_probe(position(x, y)?)
                .map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

fn build_monopole_field_map(world: &mut ElectrostaticWorld) -> Result<(), String> {
    world
        .create_charge(PointChargeSpec::new(
            position(0.0, 0.0)?,
            Coulombs::new(1.0e-9).map_err(|error| error.to_string())?,
        ))
        .map_err(|error| error.to_string())?;
    for y_index in -4..=4 {
        for x_index in -6..=6 {
            if x_index == 0 && y_index == 0 {
                continue;
            }
            world
                .create_probe(position(
                    f64::from(x_index) * 0.4,
                    f64::from(y_index) * 0.4,
                )?)
                .map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

fn position(x: f64, y: f64) -> Result<Position2, String> {
    Ok(Position2::new(
        Meters::new(x).map_err(|error| error.to_string())?,
        Meters::new(y).map_err(|error| error.to_string())?,
    ))
}

#[cfg(test)]
mod tests {
    use super::ElectrostaticSession;
    use crate::presentation::ui::ProjectTemplate;

    #[test]
    fn dipole_project_builds_a_bounded_static_snapshot() {
        let session = ElectrostaticSession::new(ProjectTemplate::PrimaryLab).expect("session");
        assert_eq!(session.snapshot().charges.len(), 2);
        assert!(!session.snapshot().probes.is_empty());
    }

    #[test]
    fn field_mapping_project_builds_a_radial_probe_grid() {
        let session = ElectrostaticSession::new(ProjectTemplate::SecondaryLab).expect("session");
        assert_eq!(session.snapshot().charges.len(), 1);
        assert_eq!(session.snapshot().probes.len(), 116);
        assert!(
            session
                .snapshot()
                .probes
                .iter()
                .all(|probe| probe.magnitude.get() > 0.0)
        );
    }
}
