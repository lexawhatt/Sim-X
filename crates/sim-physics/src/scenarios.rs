//! Small reference compositions shared by scientific tests and headless demos.
//!
//! These are ordinary body/link recipes, not privileged physics object types.
//! Public fields allow callers to adjust initial conditions before [`WorldDefinition::build`]
//! repeats authoritative validation. This is not a persistence format.

use crate::{
    BodyDesc, BodyId, Error, LinkDesc, LinkId, Mobility, PhysicsSettings, SolverConfig, Vec2, World,
};

/// An authorable physical graph, validated only when built into a [`World`].
#[derive(Clone, Debug, PartialEq)]
pub struct WorldDefinition {
    /// Scene-wide physical parameters.
    pub settings: PhysicsSettings,
    /// Fixed numerical policy.
    pub solver: SolverConfig,
    /// Untrusted body descriptions; building enforces the body budget first.
    pub bodies: Vec<BodyDesc>,
    /// Untrusted relationship descriptions; building enforces the link budget.
    pub links: Vec<LinkDesc>,
}

impl WorldDefinition {
    /// Validates and creates an independent run; this definition is unchanged.
    pub fn build(&self) -> Result<World, Error> {
        World::new(self.settings, self.solver, &self.bodies, &self.links)
    }
    fn validated(self) -> Result<Self, Error> {
        self.build()?;
        Ok(self)
    }
}

/// A one-kilogram falling point mass, with caller-selected initial conditions.
/// Uniform scene gravity determines acceleration; there is no implicit ground.
pub fn free_fall(
    height_m: f64,
    velocity_m_s: Vec2,
    settings: PhysicsSettings,
) -> Result<WorldDefinition, Error> {
    WorldDefinition {
        settings,
        solver: SolverConfig::default(),
        bodies: vec![BodyDesc {
            id: BodyId::new(1)?,
            position_m: Vec2::new(0.0, height_m),
            velocity_m_s,
            mass_kg: 1.0,
            mobility: Mobility::Dynamic,
        }],
        links: vec![],
    }
    .validated()
}

/// A fixed origin, point bob and bilateral rod. Angle is measured from downward
/// vertical in radians. This is a graph recipe, not a special integration rule.
pub fn pendulum(
    length_m: f64,
    mass_kg: f64,
    angle_rad: f64,
    settings: PhysicsSettings,
) -> Result<WorldDefinition, Error> {
    let anchor = BodyId::new(1)?;
    let bob = BodyId::new(2)?;
    WorldDefinition {
        settings,
        solver: SolverConfig::default(),
        bodies: vec![
            BodyDesc {
                id: anchor,
                position_m: Vec2::ZERO,
                velocity_m_s: Vec2::ZERO,
                mass_kg: 1.0,
                mobility: Mobility::Fixed,
            },
            BodyDesc {
                id: bob,
                position_m: Vec2::new(length_m * angle_rad.sin(), -length_m * angle_rad.cos()),
                velocity_m_s: Vec2::ZERO,
                mass_kg,
                mobility: Mobility::Dynamic,
            },
        ],
        links: vec![LinkDesc::Rod {
            id: LinkId::new(1)?,
            a: anchor,
            b: bob,
            length_m,
        }],
    }
    .validated()
}

/// A horizontal one-meter-rest-length spring attached to a fixed origin.
/// Set scene gravity to zero for an isolated horizontal oscillator reference.
pub fn spring_oscillator(
    mass_kg: f64,
    stiffness_n_m: f64,
    extension_m: f64,
    settings: PhysicsSettings,
) -> Result<WorldDefinition, Error> {
    let anchor = BodyId::new(1)?;
    let bob = BodyId::new(2)?;
    WorldDefinition {
        settings,
        solver: SolverConfig::default(),
        bodies: vec![
            BodyDesc {
                id: anchor,
                position_m: Vec2::ZERO,
                velocity_m_s: Vec2::ZERO,
                mass_kg: 1.0,
                mobility: Mobility::Fixed,
            },
            BodyDesc {
                id: bob,
                position_m: Vec2::new(1.0 + extension_m, 0.0),
                velocity_m_s: Vec2::ZERO,
                mass_kg,
                mobility: Mobility::Dynamic,
            },
        ],
        links: vec![LinkDesc::Spring {
            id: LinkId::new(1)?,
            a: anchor,
            b: bob,
            rest_length_m: 1.0,
            stiffness_n_m,
        }],
    }
    .validated()
}
