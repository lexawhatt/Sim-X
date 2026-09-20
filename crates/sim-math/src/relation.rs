//! Mathematical comparisons, separate from rendering a boundary or a volume.
use crate::MathError;

/// Comparison applied to the real residual left minus right.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Relation {
    /// Left equals right.
    Equal,
    /// Left is strictly less than right.
    Less,
    /// Left is less than or equal to right.
    LessOrEqual,
    /// Left is strictly greater than right.
    Greater,
    /// Left is greater than or equal to right.
    GreaterOrEqual,
}
impl Relation {
    /// Canonical ASCII spelling, suitable for plain mathematical source.
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Equal => "=",
            Self::Less => "<",
            Self::LessOrEqual => "<=",
            Self::Greater => ">",
            Self::GreaterOrEqual => ">=",
        }
    }
    /// Whether points on the zero boundary belong to this relation.
    pub const fn includes_boundary(self) -> bool {
        matches!(self, Self::Equal | Self::LessOrEqual | Self::GreaterOrEqual)
    }
    /// Test a finite left-minus-right residual; nonfinite values never satisfy
    /// a real comparison. Equality here is exact, not a numerical root tolerance.
    pub fn accepts(self, residual: f64) -> bool {
        residual.is_finite()
            && match self {
                Self::Equal => residual == 0.0,
                Self::Less => residual < 0.0,
                Self::LessOrEqual => residual <= 0.0,
                Self::Greater => residual > 0.0,
                Self::GreaterOrEqual => residual >= 0.0,
            }
    }
}

pub(crate) fn split(source: &str) -> Result<Option<(&str, Relation, &str)>, MathError> {
    let mut depth = 0_usize;
    let mut found = None;
    let mut chars = source.char_indices().peekable();
    while let Some((at, ch)) = chars.next() {
        match ch {
            '(' => depth += 1,
            ')' => depth = depth.checked_sub(1).ok_or(MathError::Syntax)?,
            '=' | '<' | '>' | '\u{2264}' | '\u{2265}' => {
                if depth != 0 || found.is_some() {
                    return Err(MathError::Syntax);
                }
                let mut end = at + ch.len_utf8();
                let relation = match ch {
                    '=' => Relation::Equal,
                    '\u{2264}' => Relation::LessOrEqual,
                    '\u{2265}' => Relation::GreaterOrEqual,
                    '<' | '>' => {
                        let equal = chars.peek().is_some_and(|(_, ch)| *ch == '=');
                        if equal {
                            let _ = chars.next();
                            end += 1;
                        }
                        match (ch, equal) {
                            ('<', false) => Relation::Less,
                            ('<', true) => Relation::LessOrEqual,
                            (_, false) => Relation::Greater,
                            (_, true) => Relation::GreaterOrEqual,
                        }
                    }
                    _ => return Err(MathError::Syntax),
                };
                found = Some((at, end, relation));
            }
            _ => {}
        }
    }
    if depth != 0 {
        return Err(MathError::Syntax);
    }
    Ok(found.map(|(at, end, relation)| (source[..at].trim(), relation, source[end..].trim())))
}
