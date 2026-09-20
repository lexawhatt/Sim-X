//! Independent Euclidean mathematics, with no UI, renderer or Physics dependency.
//!
//! - [`Expression`]: real expression parsing with meval, radians.
//! - [`IntegrationJob`]: resumable finite-interval quadrature with gauss-quad.
//! - [`geometry`]: editable free points and directed dependent constructions.
//! - [`statement`]: scalar, plane, height-field and spatial-relation classification.
//! - [`implicit3d`]: cancellable XYZ relation boundary cross-sections, not volumes.
//! - [`relation`]: equality and inequality semantics independent of visualization.
//!
//! Domain screening is conservative, not a general symbolic proof engine.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod calculus;
pub mod contour;
mod expression;
pub mod functions;
pub mod geometry;
pub mod implicit3d;
mod integral;
pub mod relation;
pub mod statement;
pub mod surface;

pub use expression::Expression;
pub use integral::{Integral, IntegralKind, IntegrationJob};

/// A rejected mathematical operation. Failure never supplies a numeric result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MathError {
    /// Caller cancelled a resumable or sampled operation.
    Cancelled,
    /// Invalid or unsupported expression syntax.
    Syntax,
    /// A planar inequality has no region visualization implementation yet.
    UnsupportedPlanarInequality,
    /// An input/result is not finite or outside the supported numeric range.
    NumericRange,
    /// A value is undefined in the real-number domain.
    Undefined,
    /// Continuity on the interval could not be established by this method.
    UnresolvedDomain,
    /// Floating-point coordinates can no longer distinguish a subdivision.
    PrecisionExhausted,
    /// A construction handle does not belong to this document's current nodes.
    MissingPoint,
    /// A dependent construction cannot be directly moved.
    DerivedPoint,
}

impl std::fmt::Display for MathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Cancelled => "Cancelled",
            Self::Syntax => "Invalid syntax or unsupported name",
            Self::UnsupportedPlanarInequality => "2D inequality regions are not supported yet",
            Self::NumericRange => "Outside the supported finite numeric range",
            Self::Undefined => "Undefined in the real-number domain",
            Self::UnresolvedDomain => "Interval may contain a pole or undefined region",
            Self::PrecisionExhausted => "More precision is needed to resolve this interval",
            Self::MissingPoint => "Construction point does not exist",
            Self::DerivedPoint => "Move the inputs of this derived point instead",
        })
    }
}

impl std::error::Error for MathError {}

fn finite(value: f64) -> Result<f64, MathError> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(MathError::NumericRange)
    }
}
