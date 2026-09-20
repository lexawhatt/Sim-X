//! Bounded, renderer-independent physical compositions in SI units.
//!
//! The model supplies translational masses, uniform gravity, bilateral rods,
//! Hooke springs, body-local attachments, and optional planar angular response
//! with frictionless circle/box contacts. Legacy constructors lock orientation.
//! It has no friction, off-center rods or continuous impact resolution.
//! [`World::step`] uses Verlet/RATTLE plus bounded contact solving, atomically.
//! See the crate README for numerical limits, telemetry semantics and sources.
//!
//! Ownership: `core` validates graph transactions and owns the world lifecycle;
//! `mechanics` evaluates forces, integrates candidates, solves constraints and
//! produces observations; `collision` owns finite geometry, swept rejection,
//! coupled contact solving and local energy accounting; `rigid` owns angular
//! states, attachment transforms, face manifolds and angular coupling;
//! `numeric` owns vectors,
//! checked products and exact reduction.
//! [`scenarios`] contains reusable reference graph recipes, never solver cases.

#![forbid(unsafe_code)]

mod collision;
mod core;
mod mechanics;
mod numeric;
mod rigid;
pub mod scenarios;

pub use collision::model::{
    ColliderDesc, CollisionShape, ContactPointTelemetry, ContactTelemetry, MAX_COLLIDERS,
    MAX_CONTACT_POINTS, MAX_CONTACTS, RESTITUTION_SPEED_THRESHOLD_M_S,
};
pub use core::error::{Error, NumericStage};
pub use core::identity::{BodyId, ForceSourceId, LinkId};
pub use core::settings::{PhysicsSettings, SolverConfig};
pub use core::world::{Snapshot, World};
pub use mechanics::model::{
    BodyDesc, ForceInput, LinkDesc, MAX_BODIES, MAX_FORCES, MAX_LINKS, Mobility,
};
pub use mechanics::telemetry::{BodyTelemetry, EnergyTelemetry, LinkTelemetry, StepReport};
pub use numeric::Vec2;
pub use rigid::model::RotationDesc;
pub use rigid::telemetry::RotationTelemetry;
