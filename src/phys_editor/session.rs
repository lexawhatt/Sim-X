//! Application timing and authoring conversion, never physical equations.

use sim_logic::prelude::*;
use sim_physics::{
    BodyDesc, BodyId, ColliderDesc, CollisionShape, LinkDesc, LinkId, Mobility, PhysicsSettings,
    RotationDesc, SolverConfig, Vec2,
};

use super::{
    document::{Document, LinkKind, ObjectKind, PhysicsEnvironment, Point},
    state::{EditorState, Mode},
};

pub(crate) const MAX_STEPS_PER_FRAME: usize = 32;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum PlaybackSpeed {
    Quarter,
    #[default]
    Normal,
    Fast,
}

impl PlaybackSpeed {
    pub(crate) const fn multiplier(self) -> f64 {
        match self {
            Self::Quarter => 0.25,
            Self::Normal => 1.0,
            Self::Fast => 4.0,
        }
    }
}

pub(crate) struct RunSession {
    world: sim_physics::World,
    pub(crate) paused: bool,
    pub(crate) speed: PlaybackSpeed,
    pub(crate) failure: Option<String>,
    pub(crate) dropped_simulation_s: f64,
    accumulator_s: f64,
    skip_elapsed: bool,
}

fn vector(point: Point) -> sim_physics::Vec2 {
    sim_physics::Vec2::new(point.x, point.y)
}

fn settings(environment: PhysicsEnvironment) -> PhysicsSettings {
    PhysicsSettings {
        gravity_m_s2: vector(environment.gravity_m_s2),
        linear_drag_per_s: environment.linear_drag_per_s,
    }
}

impl RunSession {
    pub(crate) fn new(document: &Document) -> Result<Self, sim_physics::Error> {
        let bodies = document
            .objects()
            .iter()
            .map(|object| {
                Ok(BodyDesc {
                    id: BodyId::new(object.id)?,
                    position_m: vector(object.position),
                    velocity_m_s: sim_physics::Vec2::ZERO,
                    mass_kg: object.mass_kg,
                    mobility: if object.fixed {
                        Mobility::Fixed
                    } else {
                        Mobility::Dynamic
                    },
                })
            })
            .collect::<Result<Vec<_>, sim_physics::Error>>()?;
        let colliders = document
            .objects()
            .iter()
            .filter_map(|object| {
                let shape = match object.kind {
                    ObjectKind::Ball => CollisionShape::Circle {
                        radius_m: object.size_m * 0.5,
                    },
                    ObjectKind::Box => CollisionShape::Box {
                        half_extents_m: Vec2::new(object.size_m * 0.5, object.height_m * 0.5),
                        // Body orientation lives in the kernel rotation state.
                        // Collider orientation is relative to that body frame.
                        angle_rad: 0.0,
                    },
                    // Attachment markers have no physical extent. A fixed
                    // obstacle is an explicitly fixed Ball or Box instead.
                    ObjectKind::Anchor => return None,
                };
                Some(BodyId::new(object.id).map(|body| ColliderDesc {
                    body,
                    shape,
                    restitution: object.restitution,
                }))
            })
            .collect::<Result<Vec<_>, sim_physics::Error>>()?;
        let rotations = colliders
            .iter()
            .map(|collider| {
                // Both lists were created from the same immutable document.
                let body = bodies
                    .iter()
                    .find(|body| body.id == collider.body)
                    .ok_or(sim_physics::Error::MissingBody(collider.body))?;
                let object = document
                    .object(collider.body.get())
                    .ok_or(sim_physics::Error::MissingBody(collider.body))?;
                RotationDesc::uniform(*body, collider.shape, object.rotation_deg.to_radians(), 0.0)
            })
            .collect::<Result<Vec<_>, sim_physics::Error>>()?;
        let links = document
            .links()
            .iter()
            .map(|link| {
                let id = LinkId::new(link.id)?;
                let a = BodyId::new(link.a)?;
                let b = BodyId::new(link.b)?;
                Ok(match link.kind {
                    LinkKind::Rod { length_m } => LinkDesc::Rod { id, a, b, length_m },
                    LinkKind::Spring {
                        rest_length_m,
                        stiffness_n_m,
                    } => LinkDesc::AttachedSpring {
                        id,
                        a,
                        b,
                        local_a_m: vector(link.a_local_m),
                        local_b_m: vector(link.b_local_m),
                        rest_length_m,
                        stiffness_n_m,
                    },
                })
            })
            .collect::<Result<Vec<_>, sim_physics::Error>>()?;
        let world = sim_physics::World::with_rigid_bodies(
            settings(document.environment()),
            SolverConfig::default(),
            &bodies,
            &links,
            &colliders,
            &rotations,
        )?;
        Ok(Self {
            world,
            paused: false,
            speed: PlaybackSpeed::Normal,
            failure: None,
            dropped_simulation_s: 0.0,
            accumulator_s: 0.0,
            skip_elapsed: true,
        })
    }

    pub(crate) fn world(&self) -> &sim_physics::World {
        &self.world
    }

    pub(crate) fn environment(&self) -> PhysicsEnvironment {
        let settings = self.world.settings();
        PhysicsEnvironment {
            gravity_m_s2: Point::new(settings.gravity_m_s2.x, settings.gravity_m_s2.y),
            linear_drag_per_s: settings.linear_drag_per_s,
        }
    }

    pub(crate) fn set_environment(
        &mut self,
        environment: PhysicsEnvironment,
    ) -> Result<(), sim_physics::Error> {
        self.world.set_settings(settings(environment))?;
        Ok(())
    }

    pub(crate) fn position(&self, id: u64) -> Option<Point> {
        let position = self.world.body(BodyId::new(id).ok()?)?.position_m;
        Some(Point::new(position.x, position.y))
    }

    /// Reads the canonical body angle; this adapter never integrates a pose.
    pub(crate) fn angle_deg(&self, id: u64) -> Option<f64> {
        Some(
            self.world
                .rotation(BodyId::new(id).ok()?)?
                .angle_rad
                .to_degrees(),
        )
    }

    pub(crate) fn pause(&mut self, paused: bool) {
        self.paused = paused || self.failure.is_some();
        self.discard_elapsed();
    }

    pub(crate) fn set_speed(&mut self, speed: PlaybackSpeed) {
        self.speed = speed;
        self.discard_elapsed();
    }

    pub(crate) fn discard_elapsed(&mut self) {
        self.accumulator_s = 0.0;
        self.skip_elapsed = true;
    }

    pub(crate) fn advance(&mut self, elapsed_s: f64) {
        if self.paused || self.failure.is_some() {
            self.discard_elapsed();
            return;
        }
        if std::mem::take(&mut self.skip_elapsed) {
            return;
        }
        if !elapsed_s.is_finite() || elapsed_s < 0.0 {
            self.failure = Some("Invalid frame duration; simulation paused.".into());
            self.pause(true);
            return;
        }
        let dt = self.world.solver().fixed_dt_s;
        // Bound arithmetic even for a pathological host duration. Report the
        // omitted time, rather than letting a slow frame change physical dt.
        let scaled = elapsed_s.min(1.0) * self.speed.multiplier();
        let dropped =
            self.dropped_simulation_s + (elapsed_s - elapsed_s.min(1.0)) * self.speed.multiplier();
        if !dropped.is_finite() {
            self.failure =
                Some("Frame duration exceeds accounting range; simulation paused.".into());
            self.pause(true);
            return;
        }
        self.dropped_simulation_s = dropped;
        self.accumulator_s += scaled;
        let available = (self.accumulator_s / dt).floor() as usize;
        let steps = available.min(MAX_STEPS_PER_FRAME);
        if available > steps {
            let dropped = (available - steps) as f64 * dt;
            self.accumulator_s -= dropped;
            self.dropped_simulation_s += dropped;
        }
        for _ in 0..steps {
            match self.world.step(&[]) {
                Ok(_) => self.accumulator_s = (self.accumulator_s - dt).max(0.0),
                Err(error) => {
                    self.failure = Some(format!("Physics paused: {error}"));
                    self.pause(true);
                    break;
                }
            }
        }
    }
}

pub(crate) fn advance(time: FrameTime, state: Option<ResMut<EditorState>>) {
    let Some(mut state) = state else {
        return;
    };
    let allowed = state.mode == Mode::Preview && !state.environment_open;
    if let Some(run) = state.run.as_mut() {
        if allowed {
            run.advance(time.seconds());
        } else {
            run.discard_elapsed();
        }
    }
}
