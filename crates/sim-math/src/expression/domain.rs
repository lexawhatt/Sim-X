//! Scientific domain adapter over meval's public RPN, not a second parser.
use crate::{MathError, finite};
use meval::tokenizer::{Operation, Token};

pub(super) type Range = (f64, f64);

fn range(values: &[f64]) -> Result<Range, MathError> {
    let (mut low, mut high) = (f64::INFINITY, f64::NEG_INFINITY);
    for &value in values {
        finite(value)?;
        low = low.min(value);
        high = high.max(value);
    }
    Ok((low.next_down(), high.next_up()))
}

fn crosses_zero((low, high): Range) -> bool {
    low <= 0.0 && high >= 0.0
}

pub(super) fn screen(
    tokens: &[Token],
    x: [f64; 2],
    y: [f64; 2],
    z: [f64; 2],
    bindings: &std::collections::BTreeMap<String, f64>,
) -> Result<(), MathError> {
    let mut stack: Vec<Range> = Vec::new();
    for token in tokens {
        let result = match token {
            Token::Number(n) => (*n, *n),
            Token::Var(name) => match name.as_str() {
                "x" => (x[0], x[1]),
                "y" => (y[0], y[1]),
                "z" => (z[0], z[1]),
                "tau" => (std::f64::consts::TAU, std::f64::consts::TAU),
                "pi" => (std::f64::consts::PI, std::f64::consts::PI),
                "e" => (std::f64::consts::E, std::f64::consts::E),
                _ => {
                    let v = *bindings.get(name).ok_or(MathError::Syntax)?;
                    (v, v)
                }
            },
            Token::Unary(op) => {
                let (l, h) = stack.pop().ok_or(MathError::Syntax)?;
                match op {
                    Operation::Minus => (-h, -l),
                    Operation::Plus => (l, h),
                    _ => return Err(MathError::Syntax),
                }
            }
            Token::Binary(op) => {
                let r = stack.pop().ok_or(MathError::Syntax)?;
                let l = stack.pop().ok_or(MathError::Syntax)?;
                binary(*op, l, r)?
            }
            Token::Func(name, Some(count)) => {
                let start = stack.len().checked_sub(*count).ok_or(MathError::Syntax)?;
                let args = stack.split_off(start);
                if *count == 1
                    && matches!(
                        name.as_str(),
                        "sin" | "cos" | "tan" | "tg" | "sqrt" | "ln" | "exp" | "abs"
                    )
                {
                    unary(name, args[0])?
                } else {
                    super::function_ranges::screen(name, &args)?
                }
            }
            _ => return Err(MathError::Syntax),
        };
        finite(result.0)?;
        finite(result.1)?;
        stack.push(result);
    }
    if stack.len() == 1 {
        Ok(())
    } else {
        Err(MathError::Syntax)
    }
}

fn binary(op: Operation, l: Range, r: Range) -> Result<Range, MathError> {
    if l.0 == l.1 && r.0 == r.1 {
        // A fixed subexpression is one evaluated f64 value, not a varying
        // interval. Retaining that fact is essential for root(1+2,negative_x):
        // expanding the index around 3 would lose its exact odd-integer domain.
        let value = match op {
            Operation::Plus => l.0 + r.0,
            Operation::Minus => l.0 - r.0,
            Operation::Times => l.0 * r.0,
            Operation::Div if r.0 != 0.0 => l.0 / r.0,
            Operation::Pow if l.0 != 0.0 || r.0 > 0.0 => l.0.powf(r.0),
            _ => return Err(MathError::UnresolvedDomain),
        };
        return finite(value).map(|value| (value, value));
    }
    match op {
        Operation::Plus => range(&[l.0 + r.0, l.1 + r.1]),
        Operation::Minus => range(&[l.0 - r.1, l.1 - r.0]),
        Operation::Times => range(&[l.0 * r.0, l.0 * r.1, l.1 * r.0, l.1 * r.1]),
        Operation::Div if !crosses_zero(r) => range(&[l.0 / r.0, l.0 / r.1, l.1 / r.0, l.1 / r.1]),
        Operation::Pow => power(l, r),
        _ => Err(MathError::UnresolvedDomain),
    }
}

fn unary(name: &str, (low, high): Range) -> Result<Range, MathError> {
    match name {
        "sin" | "cos" => Ok((-1.0, 1.0)),
        "tan" | "tg" => {
            let half = std::f64::consts::FRAC_PI_2;
            let pi = std::f64::consts::PI;
            if high - low >= pi
                || ((low - half) / pi).ceil() <= ((high - half) / pi).floor()
                || low.cos().abs().min(high.cos().abs()) <= 1e-12
            {
                return Err(MathError::UnresolvedDomain);
            }
            range(&[low.tan(), high.tan()])
        }
        "sqrt" if low >= 0.0 => Ok((low.sqrt(), finite(high.sqrt())?)),
        "ln" if low > 0.0 => range(&[low.ln(), high.ln()]),
        "exp" => range(&[low.exp(), high.exp()]),
        "abs" => Ok((
            if crosses_zero((low, high)) {
                0.0
            } else {
                low.abs().min(high.abs())
            },
            low.abs().max(high.abs()),
        )),
        _ => Err(MathError::UnresolvedDomain),
    }
}

fn power(base: Range, exponent: Range) -> Result<Range, MathError> {
    if exponent.0 == exponent.1 && exponent.0.fract() == 0.0 {
        let n = exponent.0;
        if n <= 0.0 && crosses_zero(base) {
            return Err(MathError::UnresolvedDomain);
        }
        let mut result = range(&[base.0.powf(n), base.1.powf(n)])?;
        if n > 0.0 && n % 2.0 == 0.0 && crosses_zero(base) {
            result.0 = 0.0;
        }
        Ok(result)
    } else if base.0 > 0.0 {
        range(&[
            base.0.powf(exponent.0),
            base.0.powf(exponent.1),
            base.1.powf(exponent.0),
            base.1.powf(exponent.1),
        ])
    } else {
        Err(MathError::UnresolvedDomain)
    }
}
