//! Flat, normalized line art. No perspective, solver state or drawing APIs.

use sim_logic::prelude::Vec2;
use std::f32::consts::{PI, TAU};

use crate::menu::state::PhysicsScale;

pub(super) const EARTH_RADIUS: f32 = 0.205;
pub(super) const ORBIT_RADIUS: f32 = 0.405;
pub(super) const PIVOT: Vec2 = Vec2::new(0.0, -0.29);
pub(super) const PENDULUM_LENGTH: f32 = 0.56;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Mechanism {
    Pendulum,
    Oscillator,
    Gears,
}

impl Mechanism {
    /// Five seconds fully visible, with half-second fades on either side.
    /// The outgoing mechanism disappears before the next is drawn.
    pub(super) fn current(seconds: f32) -> (Self, f32) {
        let seconds = seconds.rem_euclid(18.0);
        let local = seconds.rem_euclid(6.0);
        let opacity = (local.min(6.0 - local) * 2.0).clamp(0.0, 1.0);
        let opacity = opacity * opacity * (3.0 - 2.0 * opacity);
        let mechanism = match (seconds / 6.0) as usize {
            0 => Self::Pendulum,
            1 => Self::Oscillator,
            _ => Self::Gears,
        };
        (mechanism, opacity)
    }
}

#[derive(Clone, Copy)]
pub(super) enum Path {
    Guide,
    Axis(bool),
    Support(Mechanism),
    Rod,
    SwingArc,
    Spring,
    Mass,
    Gear(u8),
    GearSpoke(u8, u8),
    MoonOrbit,
    Continent(u8),
}

#[derive(Clone, Copy)]
pub(super) enum Dot {
    Nucleus,
    Electron(u8),
    Pivot,
    Weight,
    SpringPin,
    GearHub(u8),
    Earth,
    Moon,
}

pub(super) const PATHS: [(Path, u16); 18] = [
    (Path::Guide, 96),
    (Path::Axis(false), 1),
    (Path::Axis(true), 1),
    (Path::Support(Mechanism::Pendulum), 1),
    (Path::Rod, 1),
    (Path::SwingArc, 32),
    (Path::Support(Mechanism::Oscillator), 1),
    (Path::Spring, 26),
    (Path::Mass, 4),
    (Path::Gear(0), 96),
    (Path::Gear(1), 96),
    (Path::GearSpoke(0, 0), 1),
    (Path::GearSpoke(0, 1), 1),
    (Path::GearSpoke(1, 0), 1),
    (Path::GearSpoke(1, 1), 1),
    (Path::MoonOrbit, 96),
    (Path::Continent(0), 12),
    (Path::Continent(1), 8),
];
pub(super) const DOTS: [Dot; 11] = [
    Dot::Nucleus,
    Dot::Electron(0),
    Dot::Electron(1),
    Dot::Electron(2),
    Dot::Pivot,
    Dot::Weight,
    Dot::SpringPin,
    Dot::GearHub(0),
    Dot::GearHub(1),
    Dot::Earth,
    Dot::Moon,
];

pub(super) fn rotate(point: Vec2, angle: f32) -> Vec2 {
    let (sine, cosine) = angle.sin_cos();
    Vec2::new(
        point.x() * cosine - point.y() * sine,
        point.x() * sine + point.y() * cosine,
    )
}

fn polar(radius: f32, angle: f32) -> Vec2 {
    Vec2::new(radius * angle.cos(), radius * angle.sin())
}

pub(super) fn bob(phase: f32) -> Vec2 {
    let angle = 0.38 * phase.sin();
    PIVOT + Vec2::new(angle.sin(), angle.cos()) * PENDULUM_LENGTH
}

fn spring_mass(phase: f32) -> Vec2 {
    Vec2::new(0.0, 0.16 + 0.065 * phase.sin())
}

pub(super) fn moon(phase: f32) -> Vec2 {
    polar(ORBIT_RADIUS, phase * 0.2 - 0.6)
}

pub(super) fn electron(orbit: u8, phase: f32) -> Vec2 {
    let angle = -PI * 0.5 + f32::from(orbit) * TAU / 3.0 + phase * 1.2;
    // Exact same normalized orbit and rotation as the approved physics.svg.
    rotate(
        Vec2::new(0.164 * angle.cos(), 0.405 * angle.sin()),
        f32::from(orbit) * PI / 3.0 + phase * 0.1,
    )
}

fn gear_center(index: u8) -> Vec2 {
    Vec2::new(if index == 0 { -0.16 } else { 0.16 }, 0.0)
}

fn gear_angle(index: u8, phase: f32) -> f32 {
    if index == 0 {
        phase * 0.2
    } else {
        -phase * 0.2 + PI / 12.0
    }
}

const WESTERN_LAND: [Vec2; 12] = [
    Vec2::new(-0.60, -0.34),
    Vec2::new(-0.47, -0.63),
    Vec2::new(-0.15, -0.70),
    Vec2::new(0.02, -0.43),
    Vec2::new(-0.13, -0.20),
    Vec2::new(-0.02, 0.00),
    Vec2::new(0.16, 0.20),
    Vec2::new(-0.01, 0.60),
    Vec2::new(-0.21, 0.75),
    Vec2::new(-0.31, 0.37),
    Vec2::new(-0.23, 0.11),
    Vec2::new(-0.52, -0.05),
];
const EASTERN_LAND: [Vec2; 8] = [
    Vec2::new(0.21, -0.54),
    Vec2::new(0.51, -0.62),
    Vec2::new(0.78, -0.31),
    Vec2::new(0.56, -0.07),
    Vec2::new(0.49, 0.32),
    Vec2::new(0.29, 0.59),
    Vec2::new(0.16, 0.16),
    Vec2::new(0.27, -0.14),
];

impl Path {
    pub(super) fn scale(self) -> PhysicsScale {
        match self {
            Self::MoonOrbit | Self::Continent(_) => PhysicsScale::Astra,
            _ => PhysicsScale::Macro,
        }
    }

    pub(super) fn mechanism(self) -> Option<Mechanism> {
        match self {
            Self::Support(mechanism) => Some(mechanism),
            Self::Rod | Self::SwingArc => Some(Mechanism::Pendulum),
            Self::Spring | Self::Mass => Some(Mechanism::Oscillator),
            Self::Gear(_) | Self::GearSpoke(..) => Some(Mechanism::Gears),
            _ => None,
        }
    }

    pub(super) fn point(self, t: f32, phase: f32) -> Vec2 {
        match self {
            Self::Guide | Self::MoonOrbit => polar(ORBIT_RADIUS, t * TAU),
            Self::Axis(vertical) => {
                let distance = (t - 0.5) * 0.88;
                if vertical {
                    Vec2::new(0.0, distance)
                } else {
                    Vec2::new(distance, 0.0)
                }
            }
            Self::Support(_) => PIVOT + Vec2::new((t - 0.5) * 0.22, 0.0),
            Self::Rod => PIVOT + (bob(phase) - PIVOT) * t,
            Self::SwingArc => {
                let angle = (t - 0.5) * 0.90;
                PIVOT + Vec2::new(angle.sin(), angle.cos()) * PENDULUM_LENGTH
            }
            Self::Spring => {
                let end = spring_mass(phase) - Vec2::new(0.0, 0.058);
                // Two straight leads surround a connected zigzag, not a free waveform.
                let vertex = (t * 26.0).round() as usize;
                let x = if vertex <= 1 || vertex >= 25 {
                    0.0
                } else if vertex.is_multiple_of(2) {
                    -0.028
                } else {
                    0.028
                };
                Vec2::new(x, PIVOT.y() + (end.y() - PIVOT.y()) * t)
            }
            Self::Mass => {
                let corners = [
                    Vec2::new(-0.058, -0.058),
                    Vec2::new(0.058, -0.058),
                    Vec2::new(0.058, 0.058),
                    Vec2::new(-0.058, 0.058),
                ];
                let along = t * 4.0;
                let edge = (along.floor() as usize).min(3);
                spring_mass(phase)
                    + corners[edge]
                    + (corners[(edge + 1) % 4] - corners[edge]) * (along - edge as f32)
            }
            Self::Gear(index) => {
                let vertex = (t * 96.0).round() as usize;
                let radius = if matches!(vertex % 8, 2..=5) {
                    0.174
                } else {
                    0.146
                };
                gear_center(index) + polar(radius, t * TAU + gear_angle(index, phase))
            }
            Self::GearSpoke(index, spoke) => {
                gear_center(index)
                    + rotate(
                        Vec2::new((t - 0.5) * 0.25, 0.0),
                        gear_angle(index, phase) + f32::from(spoke) * PI * 0.5,
                    )
            }
            Self::Continent(index) => {
                let points: &[Vec2] = if index == 0 {
                    &WESTERN_LAND
                } else {
                    &EASTERN_LAND
                };
                let along = t * points.len() as f32;
                let edge = (along.floor() as usize).min(points.len() - 1);
                (points[edge]
                    + (points[(edge + 1) % points.len()] - points[edge]) * (along - edge as f32))
                    * EARTH_RADIUS
            }
        }
    }

    pub(super) fn opacity(self) -> f32 {
        match self {
            Self::Guide | Self::Axis(_) => 0.065,
            Self::SwingArc => 0.13,
            Self::MoonOrbit => 0.22,
            Self::Continent(_) => 0.28,
            _ => 0.42,
        }
    }
}

impl Dot {
    pub(super) fn scale(self) -> PhysicsScale {
        match self {
            Self::Nucleus | Self::Electron(_) => PhysicsScale::Micro,
            Self::Earth | Self::Moon => PhysicsScale::Astra,
            _ => PhysicsScale::Macro,
        }
    }
    pub(super) fn mechanism(self) -> Option<Mechanism> {
        match self {
            Self::Pivot | Self::Weight => Some(Mechanism::Pendulum),
            Self::SpringPin => Some(Mechanism::Oscillator),
            Self::GearHub(_) => Some(Mechanism::Gears),
            _ => None,
        }
    }
    pub(super) fn geometry(self, phase: f32) -> (Vec2, f32) {
        match self {
            Self::Nucleus => (Vec2::ZERO, 0.007),
            Self::Electron(orbit) => (electron(orbit, phase), 0.007),
            Self::Pivot | Self::SpringPin => (PIVOT, 0.006),
            Self::Weight => (bob(phase), 0.040),
            Self::GearHub(index) => (gear_center(index), 0.007),
            Self::Earth => (Vec2::ZERO, EARTH_RADIUS),
            Self::Moon => (moon(phase), 0.025),
        }
    }
    pub(super) fn is_outline(self) -> bool {
        matches!(self, Self::Weight | Self::Earth | Self::Moon)
    }
}
