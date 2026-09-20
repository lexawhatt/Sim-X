//! Conservative envelopes for the extra built-ins. Unsupported intervals are
//! explicitly rejected rather than joined across poles or discontinuities.
use super::domain::Range;
use crate::{MathError, functions};
pub(super) fn screen(name: &str, args: &[Range]) -> Result<Range, MathError> {
    let f = functions::find(name).ok_or(MathError::Syntax)?;
    if args.iter().all(|(l, h)| l == h) {
        let v = f.evaluate(&args.iter().map(|a| a.0).collect::<Vec<_>>());
        return if v.is_finite() {
            Ok((v, v))
        } else {
            Err(MathError::UnresolvedDomain)
        };
    }
    let (l, h) = args[0];
    let endpoint = || {
        let a = f.evaluate(&[l]);
        let b = f.evaluate(&[h]);
        if a.is_finite() && b.is_finite() {
            Ok((a.min(b).next_down(), a.max(b).next_up()))
        } else {
            Err(MathError::UnresolvedDomain)
        }
    };
    match f.name {
        "root" => super::root_ranges::screen(args[0], args[1]),
        "cbrt" => super::root_ranges::screen((3.0, 3.0), args[0]),
        "mod" if args[1].0 == args[1].1 && args[1].0 != 0.0 => {
            let m = args[1].0.abs();
            if (l / m).floor() != (h / m).floor() {
                return Err(MathError::UnresolvedDomain);
            }
            Ok((0.0, m.next_up()))
        }
        "atan2" => {
            let (yl, yh) = args[0];
            let (xl, xh) = args[1];
            // Exclude both the origin and the principal-angle branch cut.
            if yl <= 0.0 && yh >= 0.0 && xl <= 0.0 {
                return Err(MathError::UnresolvedDomain);
            }
            let values = [yl.atan2(xl), yl.atan2(xh), yh.atan2(xl), yh.atan2(xh)];
            Ok((
                values
                    .iter()
                    .copied()
                    .fold(f64::INFINITY, f64::min)
                    .next_down(),
                values
                    .iter()
                    .copied()
                    .fold(f64::NEG_INFINITY, f64::max)
                    .next_up(),
            ))
        }
        "var" | "varp" | "stdev" | "stdevp" => {
            let low = args.iter().map(|a| a.0).fold(f64::INFINITY, f64::min);
            let high = args.iter().map(|a| a.1).fold(f64::NEG_INFINITY, f64::max);
            let sample = matches!(f.name, "var" | "stdev");
            let factor = args.len() as f64 / (args.len() - usize::from(sample)) as f64;
            let upper = if matches!(f.name, "stdev" | "stdevp") {
                (high - low) * factor.sqrt()
            } else {
                (high - low).powi(2) * factor
            };
            if !upper.is_finite() {
                return Err(MathError::UnresolvedDomain);
            }
            Ok((0.0, upper.next_up()))
        }
        "gamma" | "lngamma" if l > 0.0 => {
            let (mut a, b) = endpoint()?;
            let critical = 1.4616321449683623;
            if l <= critical && critical <= h {
                a = a.min(f.evaluate(&[critical]).next_down());
            }
            Ok((a, b))
        }
        "logbase" if l > 0.0 && args[1].0 == args[1].1 && args[1].0 > 0.0 && args[1].0 != 1.0 => {
            let a = f.evaluate(&[l, args[1].0]);
            let b = f.evaluate(&[h, args[1].0]);
            if a.is_finite() && b.is_finite() {
                Ok((a.min(b).next_down(), a.max(b).next_up()))
            } else {
                Err(MathError::UnresolvedDomain)
            }
        }
        "asin" | "acos" if l >= -1.0 && h <= 1.0 => endpoint(),
        "atan" | "acot" | "asinh" | "sinh" | "tanh" | "erf" | "rad" | "deg" | "sigmoid"
        | "softplus" | "smoothstep" | "smootherstep" => endpoint(),
        "acosh" if l >= 1.0 => endpoint(),
        "acsch" if l > 0.0 || h < 0.0 => endpoint(),
        "asech" if l > 0.0 && h <= 1.0 => endpoint(),
        "acoth" if l > 1.0 || h < -1.0 => endpoint(),
        "atanh" if l > -1.0 && h < 1.0 => endpoint(),
        "log" if l > 0.0 => endpoint(),
        "acsc" | "asec" if l >= 1.0 || h <= -1.0 => endpoint(),
        "csch" | "coth" if l > 0.0 || h < 0.0 => endpoint(),
        "cosh" | "sech" | "gaussian" => {
            let (mut a, mut b) = endpoint()?;
            if l <= 0.0 && h >= 0.0 {
                a = a.min(1.0);
                b = b.max(1.0);
            }
            Ok((a, b))
        }
        "sinc" => Ok((-1.0, 1.0)),
        "floor" | "ceil" | "round" | "sign" if f.evaluate(&[l]) == f.evaluate(&[h]) => endpoint(),
        "sec" | "csc" | "cot" => {
            let pi = std::f64::consts::PI;
            let shift = if f.name == "sec" { pi * 0.5 } else { 0.0 };
            if ((l - shift) / pi).ceil() <= ((h - shift) / pi).floor() {
                return Err(MathError::UnresolvedDomain);
            }
            let (mut a, mut b) = endpoint()?;
            if f.name != "cot"
                && ((l - shift - pi * 0.5) / pi).ceil() <= ((h - shift - pi * 0.5) / pi).floor()
            {
                a = a.min(-1.0);
                b = b.max(1.0);
            }
            Ok((a, b))
        }
        "min" | "max" | "mean" | "median" | "total" | "count" => {
            let a = f.evaluate(&args.iter().map(|a| a.0).collect::<Vec<_>>());
            let b = f.evaluate(&args.iter().map(|a| a.1).collect::<Vec<_>>());
            if a.is_finite() && b.is_finite() {
                Ok((a.next_down(), b.next_up()))
            } else {
                Err(MathError::UnresolvedDomain)
            }
        }
        "hypot" => {
            let a = args[0].0.abs().max(args[0].1.abs());
            let b = args[1].0.abs().max(args[1].1.abs());
            let upper = a.hypot(b);
            if upper.is_finite() {
                Ok((0.0, upper.next_up()))
            } else {
                Err(MathError::UnresolvedDomain)
            }
        }
        "clamp" if args[1].0 == args[1].1 && args[2].0 == args[2].1 && args[1].0 <= args[2].0 => {
            Ok((l.clamp(args[1].0, args[2].0), h.clamp(args[1].0, args[2].0)))
        }
        "lerp" if args[0].0 == args[0].1 && args[1].0 == args[1].1 => {
            let a = f.evaluate(&[args[0].0, args[1].0, args[2].0]);
            let b = f.evaluate(&[args[0].0, args[1].0, args[2].1]);
            if a.is_finite() && b.is_finite() {
                Ok((a.min(b).next_down(), a.max(b).next_up()))
            } else {
                Err(MathError::UnresolvedDomain)
            }
        }
        // Other varying arguments and negative Gamma intervals require a
        // stronger enclosure. Scalar evaluation works; continuity is not guessed.
        _ => Err(MathError::UnresolvedDomain),
    }
}
