use crate::Vec2;

/// Scene-wide physical choices, validated on construction and atomic replacement.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhysicsSettings {
    /// Uniform acceleration in meters per second squared; negative Y is down.
    pub gravity_m_s2: Vec2,
    /// Linear velocity decay rate in reciprocal seconds. Zero means no drag.
    pub linear_drag_per_s: f64,
}

impl Default for PhysicsSettings {
    fn default() -> Self {
        Self {
            gravity_m_s2: Vec2::new(0.0, -9.80665),
            linear_drag_per_s: 0.0,
        }
    }
}

/// Fixed numerical policy. A world's policy cannot change mid-run.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SolverConfig {
    /// Fixed physical step in seconds, within `1e-6..=1/30`.
    pub fixed_dt_s: f64,
    /// Deterministic sweeps per active position and velocity solve, `1..=128`.
    /// Contact cleanup skips further sweeps when its initial scan needs no correction.
    pub constraint_iterations: u32,
    /// Absolute rod-length and contact-gap tolerance in meters, `1e-12..=1e-4`.
    pub position_tolerance_m: f64,
    /// Absolute rod-radial/contact-normal velocity tolerance, metres per second.
    pub velocity_tolerance_m_s: f64,
    /// Additional rod-length/contact-feature tolerance, `1e-12..=1e-6`.
    /// Also scales the passive contact-island kinetic-energy acceptance tolerance.
    pub relative_tolerance: f64,
}

impl Default for SolverConfig {
    fn default() -> Self {
        Self {
            fixed_dt_s: 1.0 / 240.0,
            constraint_iterations: 32,
            position_tolerance_m: 1e-9,
            velocity_tolerance_m_s: 1e-9,
            relative_tolerance: 1e-10,
        }
    }
}
