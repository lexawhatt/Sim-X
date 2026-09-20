//! Bounded decorative paths. These drawings deliberately are not physical models.

use std::f32::consts::{PI, TAU};

use sim_logic::prelude::Vec2;

#[derive(Clone, Copy)]
pub(super) enum Path {
    AtomShell { atom: u8, shell: u8 },
    ReactionBond(u8),
    Wave,
    UnitCircle,
    Polygon,
    Axis(u8),
    Helix(u8),
    Rung(u8),
    Cell,
}

#[derive(Clone, Copy)]
pub(super) enum Dot {
    Electron(u8),
    Nucleus,
    AtomElectron { atom: u8, electron: u8 },
    AtomNucleus(u8),
    WaveTracer,
    CellNucleus,
    CellParticle(u8),
}

fn rotated(point: Vec2, angle: f32) -> Vec2 {
    let (sine, cosine) = angle.sin_cos();
    Vec2::new(
        point.x() * cosine - point.y() * sine,
        point.x() * sine + point.y() * cosine,
    )
}

fn polar(radius: f32, angle: f32) -> Vec2 {
    Vec2::new(radius * angle.cos(), radius * angle.sin())
}

fn orbit_point(orbit: u8, angle: f32, phase: f32) -> Vec2 {
    rotated(
        Vec2::new(0.164 * angle.cos(), 0.405 * angle.sin()),
        f32::from(orbit) * PI / 3.0 + phase * 0.1,
    )
}

fn atom_center(atom: u8, phase: f32) -> Vec2 {
    let point = match atom {
        0 => Vec2::new(-0.25, -0.16),
        1 => Vec2::new(0.22, -0.12),
        _ => Vec2::new(-0.02, 0.25),
    };
    // A decorative encounter: atoms approach, briefly form links, then separate.
    // This sequence is an illustration rather than a claimed chemical reaction.
    let approach = reaction_progress(phase);
    rotated(point * (1.0 - approach * 0.34), 0.08 * (phase * 0.4).sin())
}

fn reaction_progress(phase: f32) -> f32 {
    (1.0 - (phase * 2.0).cos()) * 0.5
}

fn helix_rotation(phase: f32) -> f32 {
    -0.24 + 0.06 * (phase * 0.6).sin()
}

fn shell_radius(atom: u8, shell: u8) -> f32 {
    let outer = if atom == 0 { 0.13 } else { 0.165 };
    outer * if shell == 0 && atom != 0 { 0.53 } else { 1.0 }
}

fn helix_point(strand: u8, t: f32, phase: f32) -> Vec2 {
    let angle = t * TAU * 1.6 + phase * 1.4 + f32::from(strand) * PI;
    let width = 0.16 * (1.0 + 0.08 * (phase * 0.6).sin());
    Vec2::new(angle.sin() * width, (t - 0.5) * 0.79)
}

impl Path {
    pub(super) fn point(self, t: f32, phase: f32) -> Vec2 {
        match self {
            Self::AtomShell { atom, shell } => {
                atom_center(atom, phase) + polar(shell_radius(atom, shell), t * TAU)
            }
            Self::ReactionBond(index) => {
                let start = atom_center(index, phase);
                let end = atom_center(index + 1, phase);
                start + (end - start) * t
            }
            Self::Wave => Vec2::new(
                (t - 0.5) * 0.83,
                (t * TAU * 1.5 - phase * 1.6).sin() * (0.16 + 0.06 * phase.cos()),
            ),
            Self::UnitCircle => polar(0.34, t * TAU),
            Self::Polygon => {
                let edge = (t * 8.0).floor().min(7.0);
                let fraction = t * 8.0 - edge;
                let start = polar(0.26, edge * TAU / 8.0 + phase * 0.4);
                let end = polar(0.26, (edge + 1.0) * TAU / 8.0 + phase * 0.4);
                start + (end - start) * fraction
            }
            Self::Axis(axis) => {
                rotated(Vec2::new((t - 0.5) * 0.92, 0.0), f32::from(axis) * PI * 0.5)
            }
            Self::Helix(strand) => rotated(helix_point(strand, t, phase), helix_rotation(phase)),
            Self::Rung(index) => {
                let height = (f32::from(index) + 1.0) / 16.0;
                let start = helix_point(0, height, phase);
                let end = helix_point(1, height, phase);
                rotated(start + (end - start) * t, helix_rotation(phase))
            }
            Self::Cell => {
                let angle = t * TAU;
                let radius = 0.375 * (1.0 + 0.035 * (phase * 1.4).sin())
                    + 0.024 * (angle * 5.0 + phase * 1.6).sin();
                Vec2::new(radius * angle.cos(), radius * 1.04 * angle.sin())
            }
        }
    }

    pub(super) fn opacity(self, phase: f32) -> f32 {
        match self {
            Self::Axis(_) => 0.065,
            Self::UnitCircle | Self::Cell => 0.18,
            Self::Rung(_) => 0.27,
            Self::Polygon => 0.30,
            Self::ReactionBond(_) => ((reaction_progress(phase) - 0.3) / 0.7).clamp(0.0, 1.0) * 0.7,
            _ => 0.42,
        }
    }

    pub(super) fn width(self) -> f32 {
        match self {
            Self::Axis(_) => 0.65,
            Self::Wave | Self::Helix(_) | Self::ReactionBond(_) => 1.3,
            _ => 1.0,
        }
    }
}

impl Dot {
    pub(super) fn point(self, phase: f32) -> Vec2 {
        match self {
            Self::Electron(orbit) => {
                let angle = -PI * 0.5 + f32::from(orbit) * TAU / 3.0 + phase * 1.2;
                orbit_point(orbit, angle, phase)
            }
            Self::Nucleus => Vec2::ZERO,
            Self::AtomNucleus(atom) => atom_center(atom, phase),
            Self::AtomElectron { atom, electron } => {
                let (shell, index, count): (u8, u8, u8) = if atom == 0 {
                    (0, 0, 1)
                } else if electron < 2 {
                    (0, electron, 2)
                } else {
                    (1, electron - 2, if atom == 1 { 4 } else { 6 })
                };
                let direction = if shell == 0 { 1.0 } else { -1.0 };
                let angle =
                    f32::from(index) * TAU / f32::from(count) + phase * direction + f32::from(atom);
                atom_center(atom, phase) + polar(shell_radius(atom, shell), angle)
            }
            Self::WaveTracer => {
                let t = 0.5 + 0.34 * (phase * 0.8).sin();
                Path::Wave.point(t, phase)
            }
            Self::CellNucleus => rotated(helix_point(0, 0.5, phase), helix_rotation(phase)),
            Self::CellParticle(index) => polar(
                0.275 + 0.02 * (phase * 1.6 + f32::from(index)).sin(),
                f32::from(index) * PI + phase * 0.7,
            ),
        }
    }

    pub(super) fn radius(self) -> f32 {
        match self {
            Self::Nucleus => 3.2,
            Self::Electron(_) => 4.2,
            Self::AtomNucleus(_) => 4.2,
            Self::AtomElectron { .. } => 2.5,
            _ => 3.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn electrons_remain_on_the_rotating_immutable_orbit_image() {
        for phase in [0.0, 0.7, 3.0, 17.0] {
            for orbit in 0..3 {
                let image_local = rotated(
                    Dot::Electron(orbit).point(phase),
                    -(f32::from(orbit) * PI / 3.0 + phase * 0.1),
                );
                // physics.svg radii 125.952 and 311.04 in its 768-unit viewBox.
                let x = image_local.x() / 0.164;
                let y = image_local.y() / 0.405;
                assert!((x * x + y * y - 1.0).abs() < 0.00001);
            }
        }
    }

    #[test]
    fn chemistry_encounter_has_approach_links_and_separation() {
        let distance = |phase| {
            let delta = atom_center(1, phase) - atom_center(0, phase);
            delta.x().hypot(delta.y())
        };
        assert!(distance(PI * 0.5) < distance(0.0) * 0.7);
        assert!((distance(PI) - distance(0.0)).abs() < 0.00001);
        assert_eq!(Path::ReactionBond(0).opacity(0.0), 0.0);
        assert!(Path::ReactionBond(0).opacity(PI * 0.5) > 0.6);
    }

    #[test]
    fn math_and_biology_move_visibly_within_a_five_second_slot() {
        for path in [Path::Wave, Path::Helix(0), Path::Cell] {
            let delta = path.point(0.23, 3.0) - path.point(0.23, 0.0);
            assert!(delta.x().hypot(delta.y()) > 0.01);
        }
    }

    #[test]
    fn all_motifs_loop_without_a_phase_wrap_jump() {
        let paths = [
            Path::AtomShell { atom: 2, shell: 1 },
            Path::ReactionBond(1),
            Path::Wave,
            Path::Polygon,
            Path::Helix(1),
            Path::Rung(7),
            Path::Cell,
        ];
        for path in paths {
            for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
                let start = path.point(t, 0.0);
                let end = path.point(t, TAU * 10.0);
                assert!((start.x() - end.x()).abs() < 0.00001);
                assert!((start.y() - end.y()).abs() < 0.00001);
            }
        }
        for dot in [
            Dot::Electron(1),
            Dot::AtomElectron {
                atom: 2,
                electron: 7,
            },
            Dot::WaveTracer,
            Dot::CellNucleus,
        ] {
            let start = dot.point(0.0);
            let end = dot.point(TAU * 10.0);
            assert!((start.x() - end.x()).abs() < 0.00001);
            assert!((start.y() - end.y()).abs() < 0.00001);
        }
    }
}
