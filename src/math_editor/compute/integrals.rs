//! Worker-owned integral illustration samples. Viewport resolution affects the
//! picture only, never the quadrature or the algebraic interpretation.
use crate::math_editor::compute::{Request, sample};
use sim_math::{
    MathError,
    calculus::{IntegralOperand, IntegralVisualization},
    geometry::Point,
};

#[derive(Clone, Copy, Debug)]
pub(in crate::math_editor) struct Section {
    pub x: f64,
    pub first: f64,
    pub second: f64,
}
#[derive(Clone)]
pub(in crate::math_editor) struct Band {
    pub bounds: [f64; 2],
    pub sections: Vec<Option<Section>>,
    pub direction: f64,
}
#[derive(Clone)]
pub(in crate::math_editor) struct IntegralPlot {
    pub curves: Vec<Vec<Option<Point>>>,
    pub bands: Vec<Band>,
    pub note: &'static str,
}
impl IntegralPlot {
    pub fn prepare(
        plan: IntegralVisualization,
        request: &Request,
        alive: &impl Fn() -> bool,
    ) -> Result<Self, MathError> {
        let mut picture = Self {
            curves: vec![],
            bands: vec![],
            note: "",
        };
        match plan {
            IntegralVisualization::Between { positive, negative } => {
                let weighted = positive.display_scale() != 1.0 || negative.display_scale() != 1.0;
                picture.curves.push(curve(&positive, request, alive));
                picture.curves.push(curve(&negative, request, alive));
                picture
                    .bands
                    .push(band(&positive, Some(&negative), 1.0, request, alive));
                picture.note = if weighted {
                    "Signed area; weighted curves"
                } else {
                    "Signed area between curves"
                };
            }
            IntegralVisualization::Terms { operands, offset } => {
                picture.note = if offset.is_none() {
                    "Reference curves; nonlinear result"
                } else if offset.is_some_and(|v| v != 0.0) {
                    "Areas + scalar offset"
                } else {
                    "Signed integral contributions"
                };
                for operand in operands {
                    if !alive() {
                        return Err(MathError::Cancelled);
                    }
                    picture.curves.push(curve(&operand, request, alive));
                    if let Some(direction) = operand.direction()
                        && direction != 0.0
                    {
                        picture
                            .bands
                            .push(band(&operand, None, direction, request, alive));
                    }
                }
            }
        }
        if alive() {
            Ok(picture)
        } else {
            Err(MathError::Cancelled)
        }
    }
}
fn curve(
    operand: &IntegralOperand,
    request: &Request,
    alive: &impl Fn() -> bool,
) -> Vec<Option<Point>> {
    sample(&operand.expression, request, false, false, alive)
        .into_iter()
        .map(|p| p.and_then(|p| Point::new(p.x, p.y * operand.display_scale()).ok()))
        .collect()
}

fn band(
    first: &IntegralOperand,
    second: Option<&IntegralOperand>,
    direction: f64,
    request: &Request,
    alive: &impl Fn() -> bool,
) -> Band {
    let bounds = [
        first.bounds[0].min(first.bounds[1]),
        first.bounds[0].max(first.bounds[1]),
    ];
    let mut band = Band {
        bounds,
        sections: vec![],
        direction,
    };
    let low = bounds[0].max(request.lower.x);
    let high = bounds[1].min(request.upper.x);
    let span = request.upper.x - request.lower.x;
    if low >= high || !span.is_finite() || span <= 0.0 {
        return band;
    }
    let count = ((high - low) / span * f64::from(request.pixels[0]) / 1.5)
        .ceil()
        .max(2.0) as usize;
    let mut previous = None;
    for i in 0..=count {
        if !alive() {
            break;
        }
        // Preserve the exact interval boundaries, including subpixel intervals.
        let x = if i == count {
            high
        } else {
            low + (high - low) * (i as f64 / count as f64)
        };
        if let Some(last) = previous {
            let continuous = first.expression.screen_box([last, x], [0.0, 0.0]).is_ok()
                && second.is_none_or(|s| s.expression.screen_box([last, x], [0.0, 0.0]).is_ok());
            if !continuous {
                band.sections.push(None);
            }
        }
        let section = (|| {
            let a = first.expression.evaluate_at(x, 0.0).ok()? * first.display_scale();
            let b = if let Some(second) = second {
                second.expression.evaluate_at(x, 0.0).ok()? * second.display_scale()
            } else {
                0.0
            };
            (a.is_finite() && b.is_finite()).then_some(Section {
                x,
                first: a,
                second: b,
            })
        })();
        band.sections.push(section);
        previous = Some(x);
    }
    band
}
