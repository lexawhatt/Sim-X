use crate::{Error, NumericStage, Vec2};

pub(crate) fn bounded_vec(value: Vec2, bound: f64) -> bool {
    value.finite() && value.x.abs() <= bound && value.y.abs() <= bound
}

pub(crate) fn finite(value: f64, stage: NumericStage) -> Result<f64, Error> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(Error::Numeric(stage))
    }
}

pub(crate) fn finite_vec(value: Vec2, stage: NumericStage) -> Result<Vec2, Error> {
    if value.finite() {
        Ok(value)
    } else {
        Err(Error::Numeric(stage))
    }
}

/// Integration, unlike a converged constraint iteration, must not absorb all
/// of a nonzero requested displacement or kick in either Cartesian component.
pub(crate) fn advance(old: Vec2, increment: Vec2, stage: NumericStage) -> Result<Vec2, Error> {
    let next = finite_vec(old + increment, stage)?;
    if (increment.x != 0.0 && next.x == old.x) || (increment.y != 0.0 && next.y == old.y) {
        return Err(Error::PrecisionLoss(stage));
    }
    Ok(next)
}

pub(crate) fn scaled(value: Vec2, factor: f64, stage: NumericStage) -> Result<Vec2, Error> {
    let next = finite_vec(value * factor, stage)?;
    if factor != 0.0 && ((value.x != 0.0 && next.x == 0.0) || (value.y != 0.0 && next.y == 0.0)) {
        return Err(Error::PrecisionLoss(stage));
    }
    Ok(next)
}

pub(crate) fn sum(values: &[f64], stage: NumericStage) -> Result<f64, Error> {
    super::exact_sum::reproducible_sum(values).map_err(|_| Error::Numeric(stage))
}

pub(crate) fn sum_vec(values: &[Vec2], stage: NumericStage) -> Result<Vec2, Error> {
    let x: Vec<_> = values.iter().map(|value| value.x).collect();
    let y: Vec<_> = values.iter().map(|value| value.y).collect();
    Ok(Vec2::new(sum(&x, stage)?, sum(&y, stage)?))
}
