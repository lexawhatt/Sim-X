//! Shared scalar-function catalog. UI labels and accepted arities come from the
//! same registry as evaluation; this is not a list of promised capabilities.
pub(crate) mod roots;

use statrs::{
    function::{
        erf::erf,
        gamma::{gamma, ln_gamma},
    },
    statistics::{Data, Median, Statistics},
};

/// A real-valued built-in function. Undefined/non-finite results are rejected.
pub struct Function {
    /// Canonical input spelling.
    pub name: &'static str,
    /// Functional group for discovery.
    pub group: &'static str,
    /// Arguments shown in contextual help.
    pub signature: &'static str,
    /// Meaning and important conventions.
    pub description: &'static str,
    /// Minimum argument count.
    pub min_args: usize,
    /// Maximum arity; None denotes a variadic scalar sequence.
    pub max_args: Option<usize>,
    evaluate: fn(&[f64]) -> f64,
}
impl Function {
    /// Whether the argument count matches this function's mathematical arity.
    pub fn accepts(&self, count: usize) -> bool {
        count >= self.min_args && self.max_args.is_none_or(|n| count <= n)
    }
    /// Evaluate finite scalar arguments; NaN represents an undefined operation.
    pub fn evaluate(&self, args: &[f64]) -> f64 {
        if !self.accepts(args.len()) || !args.iter().all(|x| x.is_finite()) {
            return f64::NAN;
        }
        let value = (self.evaluate)(args);
        if value.is_finite() { value } else { f64::NAN }
    }
}
macro_rules! f {
    ($name:literal,$group:literal,$sig:literal,$help:literal,$n:expr,$eval:expr) => {
        Function {
            name: $name,
            group: $group,
            signature: $sig,
            description: $help,
            min_args: $n,
            max_args: Some($n),
            evaluate: $eval,
        }
    };
}
macro_rules! seq {
    ($name:literal,$help:literal,$n:expr,$eval:expr) => {
        Function {
            name: $name,
            group: "Statistics",
            signature: "values...",
            description: $help,
            min_args: $n,
            max_args: None,
            evaluate: $eval,
        }
    };
}
/// Built-ins available in the expression editor; aliases are not counted twice.
pub static FUNCTIONS: &[Function] = &[
    f!("sin", "Trigonometry", "x", "Sine; radians", 1, |v| v[0]
        .sin()),
    f!("cos", "Trigonometry", "x", "Cosine; radians", 1, |v| v[0]
        .cos()),
    f!(
        "tan",
        "Trigonometry",
        "x",
        "Tangent; undefined at cos(x)=0",
        1,
        |v| v[0].tan()
    ),
    f!("csc", "Trigonometry", "x", "Reciprocal sine", 1, |v| 1.0
        / v[0].sin()),
    f!("sec", "Trigonometry", "x", "Reciprocal cosine", 1, |v| 1.0
        / v[0].cos()),
    f!(
        "cot",
        "Trigonometry",
        "x",
        "Cosine divided by sine",
        1,
        |v| v[0].cos() / v[0].sin()
    ),
    f!(
        "asin",
        "Inverse trigonometry",
        "x",
        "Principal arcsine in [-pi/2,pi/2]",
        1,
        |v| v[0].asin()
    ),
    f!(
        "acos",
        "Inverse trigonometry",
        "x",
        "Principal arccosine in [0,pi]",
        1,
        |v| v[0].acos()
    ),
    f!(
        "atan",
        "Inverse trigonometry",
        "x",
        "Principal arctangent in (-pi/2,pi/2)",
        1,
        |v| v[0].atan()
    ),
    f!(
        "acsc",
        "Inverse trigonometry",
        "x",
        "asin(1/x); |x| >= 1",
        1,
        |v| (1.0 / v[0]).asin()
    ),
    f!(
        "asec",
        "Inverse trigonometry",
        "x",
        "acos(1/x); |x| >= 1",
        1,
        |v| (1.0 / v[0]).acos()
    ),
    f!(
        "acot",
        "Inverse trigonometry",
        "x",
        "Principal arccotangent in (0,pi)",
        1,
        |v| 1.0_f64.atan2(v[0])
    ),
    f!("sinh", "Hyperbolic", "x", "Hyperbolic sine", 1, |v| v[0]
        .sinh()),
    f!("cosh", "Hyperbolic", "x", "Hyperbolic cosine", 1, |v| v[0]
        .cosh()),
    f!("tanh", "Hyperbolic", "x", "Hyperbolic tangent", 1, |v| v[0]
        .tanh()),
    f!(
        "csch",
        "Hyperbolic",
        "x",
        "Reciprocal hyperbolic sine",
        1,
        |v| 1.0 / v[0].sinh()
    ),
    f!(
        "sech",
        "Hyperbolic",
        "x",
        "Reciprocal hyperbolic cosine",
        1,
        |v| 1.0 / v[0].cosh()
    ),
    f!(
        "coth",
        "Hyperbolic",
        "x",
        "Hyperbolic cotangent",
        1,
        |v| 1.0 / v[0].tanh()
    ),
    f!(
        "asinh",
        "Hyperbolic",
        "x",
        "Inverse hyperbolic sine",
        1,
        |v| v[0].asinh()
    ),
    f!(
        "acosh",
        "Hyperbolic",
        "x",
        "Inverse hyperbolic cosine; x >= 1",
        1,
        |v| v[0].acosh()
    ),
    f!(
        "atanh",
        "Hyperbolic",
        "x",
        "Inverse hyperbolic tangent; |x| < 1",
        1,
        |v| v[0].atanh()
    ),
    f!(
        "acsch",
        "Hyperbolic",
        "x",
        "Inverse hyperbolic cosecant; x != 0",
        1,
        |v| (1.0 / v[0]).asinh()
    ),
    f!(
        "asech",
        "Hyperbolic",
        "x",
        "Inverse hyperbolic secant; 0 < x <= 1",
        1,
        |v| (1.0 / v[0]).acosh()
    ),
    f!(
        "acoth",
        "Hyperbolic",
        "x",
        "Inverse hyperbolic cotangent; |x| > 1",
        1,
        |v| (1.0 / v[0]).atanh()
    ),
    f!(
        "exp",
        "Powers and logarithms",
        "x",
        "e raised to x",
        1,
        |v| v[0].exp()
    ),
    f!(
        "ln",
        "Powers and logarithms",
        "x",
        "Natural logarithm; x > 0",
        1,
        |v| v[0].ln()
    ),
    f!(
        "log",
        "Powers and logarithms",
        "x",
        "Base-10 logarithm; x > 0",
        1,
        |v| v[0].log10()
    ),
    f!(
        "logbase",
        "Powers and logarithms",
        "x,b",
        "Logarithm with b > 0 and b != 1",
        2,
        |v| if v[1] > 0.0 && v[1] != 1.0 {
            v[0].log(v[1])
        } else {
            f64::NAN
        }
    ),
    f!(
        "sqrt",
        "Powers and logarithms",
        "x",
        "Nonnegative square root",
        1,
        |v| v[0].sqrt()
    ),
    f!(
        "cbrt",
        "Powers and logarithms",
        "x",
        "Real cube root, including negative x",
        1,
        |v| v[0].cbrt()
    ),
    f!(
        "root",
        "Powers and logarithms",
        "n,x",
        "Real nth root; n != 0; negative x requires an odd integer n",
        2,
        |v| roots::evaluate(v[0], v[1])
    ),
    f!(
        "abs",
        "Rounding and numbers",
        "x",
        "Absolute value",
        1,
        |v| v[0].abs()
    ),
    f!(
        "floor",
        "Rounding and numbers",
        "x",
        "Greatest integer no larger than x",
        1,
        |v| v[0].floor()
    ),
    f!(
        "ceil",
        "Rounding and numbers",
        "x",
        "Least integer no smaller than x",
        1,
        |v| v[0].ceil()
    ),
    f!(
        "round",
        "Rounding and numbers",
        "x",
        "Nearest integer; ties away from zero",
        1,
        |v| v[0].round()
    ),
    f!(
        "sign",
        "Rounding and numbers",
        "x",
        "-1, 0 or 1; sign(0)=0",
        1,
        |v| if v[0] == 0.0 { 0.0 } else { v[0].signum() }
    ),
    f!(
        "mod",
        "Rounding and numbers",
        "x,m",
        "Euclidean remainder in [0,|m|)",
        2,
        |v| v[0].rem_euclid(v[1])
    ),
    seq!("min", "Smallest scalar argument", 1, |v| v.min()),
    seq!("max", "Largest scalar argument", 1, |v| v.max()),
    seq!("mean", "Arithmetic mean of scalar arguments", 1, |v| v
        .mean()),
    seq!("median", "Median of scalar arguments", 1, |v| Data::new(
        v.to_vec()
    )
    .median()),
    seq!("stdev", "Sample standard deviation (n-1)", 2, |v| v
        .std_dev()),
    seq!("stdevp", "Population standard deviation (n)", 1, |v| v
        .population_std_dev()),
    seq!("var", "Sample variance (n-1)", 2, |v| v.variance()),
    seq!("varp", "Population variance (n)", 1, |v| v
        .population_variance()),
    seq!("total", "Sum of scalar arguments", 1, |v| v.iter().sum()),
    seq!("count", "Number of scalar arguments", 1, |v| v.len() as f64),
    f!(
        "erf",
        "Special functions",
        "x",
        "Gaussian error function",
        1,
        |v| erf(v[0])
    ),
    f!(
        "gamma",
        "Special functions",
        "x",
        "Gamma function; excludes nonpositive integers",
        1,
        |v| if v[0] <= 0.0 && v[0].fract() == 0.0 {
            f64::NAN
        } else {
            gamma(v[0])
        }
    ),
    f!(
        "lngamma",
        "Special functions",
        "x",
        "Natural log of Gamma for x > 0",
        1,
        |v| if v[0] > 0.0 { ln_gamma(v[0]) } else { f64::NAN }
    ),
    f!(
        "hypot",
        "Geometry and angles",
        "x,y",
        "Euclidean length of a 2D vector",
        2,
        |v| v[0].hypot(v[1])
    ),
    f!(
        "atan2",
        "Geometry and angles",
        "y,x",
        "Signed angle of vector (x,y); radians",
        2,
        |v| if v[0] == 0.0 && v[1] == 0.0 {
            f64::NAN
        } else {
            v[0].atan2(v[1])
        }
    ),
    f!(
        "rad",
        "Geometry and angles",
        "degrees",
        "Degrees to radians",
        1,
        |v| v[0].to_radians()
    ),
    f!(
        "deg",
        "Geometry and angles",
        "radians",
        "Radians to degrees",
        1,
        |v| v[0].to_degrees()
    ),
    f!(
        "clamp",
        "Visual building blocks",
        "x,lo,hi",
        "Clamp x to ordered bounds lo <= hi",
        3,
        |v| if v[1] <= v[2] {
            v[0].clamp(v[1], v[2])
        } else {
            f64::NAN
        }
    ),
    f!(
        "lerp",
        "Visual building blocks",
        "a,b,t",
        "Linear interpolation (and extrapolation)",
        3,
        |v| v[0] * (1.0 - v[2]) + v[1] * v[2]
    ),
    f!(
        "smoothstep",
        "Visual building blocks",
        "x",
        "Cubic transition, clamped to [0,1]",
        1,
        |v| {
            let x = v[0].clamp(0.0, 1.0);
            x * x * (3.0 - 2.0 * x)
        }
    ),
    f!(
        "smootherstep",
        "Visual building blocks",
        "x",
        "Quintic transition with smooth acceleration",
        1,
        |v| {
            let x = v[0].clamp(0.0, 1.0);
            x * x * x * (x * (x * 6.0 - 15.0) + 10.0)
        }
    ),
    f!(
        "sinc",
        "Visual building blocks",
        "x",
        "sin(x)/x with sinc(0)=1; unnormalized",
        1,
        |v| if v[0] == 0.0 { 1.0 } else { v[0].sin() / v[0] }
    ),
    f!(
        "gaussian",
        "Visual building blocks",
        "x",
        "Unit-height Gaussian exp(-x^2/2)",
        1,
        |v| (-0.5 * v[0] * v[0]).exp()
    ),
    f!(
        "sigmoid",
        "Visual building blocks",
        "x",
        "Stable logistic transition 1/(1+exp(-x))",
        1,
        |v| if v[0] >= 0.0 {
            1.0 / (1.0 + (-v[0]).exp())
        } else {
            let e = v[0].exp();
            e / (1.0 + e)
        }
    ),
    f!(
        "softplus",
        "Visual building blocks",
        "x",
        "Smooth positive ramp log(1+exp(x))",
        1,
        |v| v[0].max(0.0) + (-v[0].abs()).exp().ln_1p()
    ),
];
/// Resolve canonical names and common keyboard aliases.
pub fn find(name: &str) -> Option<&'static Function> {
    let name = match name {
        "tg" => "tan",
        "ctg" => "cot",
        "arcsin" => "asin",
        "arccos" => "acos",
        "arctan" => "atan",
        "nthroot" => "root",
        other => other,
    };
    FUNCTIONS.iter().find(|f| f.name == name)
}
