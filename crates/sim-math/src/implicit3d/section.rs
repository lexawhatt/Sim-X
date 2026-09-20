use crate::{Expression, MathError};

pub(super) struct Plane<'a> {
    pub(super) fixed_axis: usize,
    pub(super) fixed_value: f64,
    pub(super) u_axis: usize,
    pub(super) v_axis: usize,
    pub(super) u: &'a [f64],
    pub(super) v: &'a [f64],
}
impl Plane<'_> {
    fn point(&self, i: usize, j: usize) -> [f64; 3] {
        let mut point = [0.0; 3];
        point[self.fixed_axis] = self.fixed_value;
        point[self.u_axis] = self.u[i];
        point[self.v_axis] = self.v[j];
        point
    }
    fn row(
        &self,
        expression: &Expression,
        j: usize,
        alive: &impl Fn() -> bool,
    ) -> Result<Vec<Option<f64>>, MathError> {
        (0..self.u.len())
            .map(|i| {
                if !alive() {
                    return Err(MathError::Cancelled);
                }
                let [x, y, z] = self.point(i, j);
                Ok(expression.evaluate_xyz(x, y, z).ok())
            })
            .collect()
    }
}

pub(super) fn sample(
    expression: &Expression,
    plane: Plane<'_>,
    alive: &impl Fn() -> bool,
    output: &mut Vec<[[f64; 3]; 2]>,
) -> Result<(), MathError> {
    let mut bottom = plane.row(expression, 0, alive)?;
    for j in 0..plane.v.len() - 1 {
        let top = plane.row(expression, j + 1, alive)?;
        for i in 0..plane.u.len() - 1 {
            if !alive() {
                return Err(MathError::Cancelled);
            }
            let points = [
                plane.point(i, j),
                plane.point(i + 1, j),
                plane.point(i + 1, j + 1),
                plane.point(i, j + 1),
            ];
            let lo = points[0];
            let hi = points[2];
            if lo[plane.u_axis] >= hi[plane.u_axis] || lo[plane.v_axis] >= hi[plane.v_axis] {
                return Err(MathError::PrecisionExhausted);
            }
            let [Some(a), Some(b), Some(c), Some(d)] =
                [bottom[i], bottom[i + 1], top[i + 1], top[i]]
            else {
                continue;
            };
            if expression
                .screen_volume([lo[0], hi[0]], [lo[1], hi[1]], [lo[2], hi[2]])
                .is_err()
            {
                continue;
            }
            triangle([points[0], points[1], points[2]], [a, b, c], output);
            triangle([points[0], points[2], points[3]], [a, c, d], output);
        }
        bottom = top;
    }
    Ok(())
}

fn triangle(points: [[f64; 3]; 3], values: [f64; 3], output: &mut Vec<[[f64; 3]; 2]>) {
    // A zero plateau has no unique contour direction. Its edges must not create
    // arbitrary internal diagonals, e.g. a cube face coinciding with a slice.
    if values.iter().all(|value| *value == 0.0) {
        return;
    }
    let mut crossings = [[0.0; 3]; 3];
    let mut count = 0;
    for [a, b] in [[0, 1], [1, 2], [2, 0]] {
        let va = values[a];
        let vb = values[b];
        if va == 0.0 && vb == 0.0 {
            output.push([points[a], points[b]]);
            return;
        }
        let crossing = if va == 0.0 {
            Some(points[a])
        } else if vb == 0.0 {
            Some(points[b])
        } else if va.is_sign_negative() != vb.is_sign_negative() {
            // Scaling avoids overflow in va-vb for finite opposite residuals.
            let scale = va.abs().max(vb.abs());
            let t = (va / scale) / ((va / scale) - (vb / scale));
            Some(std::array::from_fn(|axis| {
                points[a][axis] + (points[b][axis] - points[a][axis]) * t
            }))
        } else {
            None
        };
        if let Some(point) = crossing
            && !crossings[..count].contains(&point)
        {
            crossings[count] = point;
            count += 1;
        }
    }
    if count == 2 && crossings[0] != crossings[1] {
        output.push([crossings[0], crossings[1]]);
    }
}
