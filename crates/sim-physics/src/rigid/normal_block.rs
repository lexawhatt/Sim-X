//! Fixed-size unilateral two-point face solve. No global matrix allocation.

use super::{model::Rotations, pose};
use crate::{BodyDesc, Vec2, collision::model::BoundCollider};

pub(super) struct EffectiveMass {
    pub inverse_mass: [f64; 2],
    pub inverse_inertia: [f64; 2],
    pub levers: [[f64; 2]; 2],
    pub matrix: [f64; 3],
}

impl EffectiveMass {
    pub fn new(
        bodies: &[BodyDesc],
        rotations: &Rotations,
        a: BoundCollider,
        b: BoundCollider,
        normal: Vec2,
        positions: [Vec2; 2],
    ) -> Self {
        let inverse_mass = [
            bodies[a.index].inverse_mass(),
            bodies[b.index].inverse_mass(),
        ];
        let inverse_inertia = [
            rotations.inverse_inertia(a.index, bodies),
            rotations.inverse_inertia(b.index, bodies),
        ];
        let levers = [a.index, b.index].map(|index| {
            positions.map(|point| pose::cross(point - bodies[index].position_m, normal))
        });
        let entry = |i: usize, j: usize| {
            inverse_mass[0]
                + inverse_mass[1]
                + inverse_inertia[0] * levers[0][i] * levers[0][j]
                + inverse_inertia[1] * levers[1][i] * levers[1][j]
        };
        let matrix = [entry(0, 0), entry(0, 1), entry(1, 1)];
        Self {
            inverse_mass,
            inverse_inertia,
            levers,
            matrix,
        }
    }

    pub fn solve(&self, residual: [f64; 2]) -> Option<[f64; 2]> {
        solve(self.matrix[0], self.matrix[1], self.matrix[2], residual)
    }

    pub fn angular_delta(&self, impulses: [f64; 2]) -> [f64; 2] {
        [0, 1].map(|body| {
            self.inverse_inertia[body]
                * (self.levers[body][0] * impulses[0] + self.levers[body][1] * impulses[1])
        })
    }
}

/// Returns the active-set solution of K*x+b >= 0, x >= 0, x.(K*x+b)=0.
/// Nearly singular faces use the caller's sequential fallback instead.
pub(super) fn solve(k11: f64, k12: f64, k22: f64, b: [f64; 2]) -> Option<[f64; 2]> {
    let determinant = k11 * k22 - k12 * k12;
    if !determinant.is_finite() || determinant <= 32.0 * f64::EPSILON * k11 * k22 {
        return None;
    }
    let both = [
        (k12 * b[1] - k22 * b[0]) / determinant,
        (k12 * b[0] - k11 * b[1]) / determinant,
    ];
    if both
        .into_iter()
        .all(|value| value.is_finite() && value >= 0.0)
    {
        return Some(both);
    }
    let first = -b[0] / k11;
    if first >= 0.0 && k12 * first + b[1] >= 0.0 {
        return Some([first, 0.0]);
    }
    let second = -b[1] / k22;
    if second >= 0.0 && k12 * second + b[0] >= 0.0 {
        return Some([0.0, second]);
    }
    if b.into_iter().all(|value| value >= 0.0) {
        return Some([0.0; 2]);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn covers_both_one_neither_and_singular_active_sets() {
        assert_eq!(solve(2.0, 1.0, 2.0, [-3.0, -3.0]), Some([1.0, 1.0]));
        assert_eq!(solve(2.0, 1.0, 2.0, [-2.0, 0.0]), Some([1.0, 0.0]));
        assert_eq!(solve(2.0, 1.0, 2.0, [0.0, -2.0]), Some([0.0, 1.0]));
        assert_eq!(solve(2.0, 1.0, 2.0, [1.0, 1.0]), Some([0.0, 0.0]));
        assert_eq!(solve(1.0, 1.0, 1.0, [-1.0, -1.0]), None);
    }
}
