//! Directed Euclidean constructions. Collections grow with the document, not a
//! product quota. Dependencies always point to earlier nodes, preventing cycles.
use crate::{Expression, MathError, finite};

/// Scientific coordinates, independent of pixels and camera framing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    /// Horizontal coordinate.
    pub x: f64,
    /// Vertical coordinate.
    pub y: f64,
}
impl Point {
    /// Creates finite coordinates; there is no arbitrary coordinate ceiling.
    pub fn new(x: f64, y: f64) -> Result<Self, MathError> {
        Ok(Self {
            x: finite(x)?,
            y: finite(y)?,
        })
    }
    /// Euclidean distance, with overflow diagnosed.
    pub fn distance(self, other: Self) -> Result<f64, MathError> {
        finite((self.x - other.x).hypot(self.y - other.y))
    }
}

/// Document-local point identity. Indices are never reused by this append-only model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PointId(usize);
impl PointId {
    /// Stable index for names and linking presentation records to this document.
    pub fn index(self) -> usize {
        self.0
    }
}

/// A point's definition, not an independently editable cached location.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Construction {
    /// Directly editable coordinates.
    Free(Point),
    /// Midpoint of two existing points.
    Midpoint(PointId, PointId),
    /// Intersection of the infinite supporting lines AB and CD.
    Intersection([PointId; 4]),
    /// Point at a fixed x on the document's active function.
    OnFunction(f64),
}

/// Shape boundaries refer to points rather than copying their positions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shape {
    /// Segment with linked endpoints.
    Segment(PointId, PointId),
    /// Circle defined by its center and a point on its circumference.
    Circle(PointId, PointId),
}

/// Editable construction graph. It has no fixed object-count limit.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Geometry {
    points: Vec<Construction>,
    shapes: Vec<Shape>,
}

impl Geometry {
    /// Point definitions in dependency order.
    pub fn points(&self) -> &[Construction] {
        &self.points
    }
    /// Linked shape definitions.
    pub fn shapes(&self) -> &[Shape] {
        &self.shapes
    }
    /// Adds a finite free point, or leaves the document unchanged on invalid input.
    pub fn add_free(&mut self, position: Point) -> Result<PointId, MathError> {
        Point::new(position.x, position.y)?;
        Ok(self.push(Construction::Free(position)))
    }
    /// Adds a point linked to the active function at x.
    pub fn add_on_function(&mut self, x: f64) -> Result<PointId, MathError> {
        Ok(self.push(Construction::OnFunction(finite(x)?)))
    }
    /// Adds a dependent midpoint; both inputs must already exist.
    pub fn add_midpoint(&mut self, a: PointId, b: PointId) -> Result<PointId, MathError> {
        self.validate(&[a, b])?;
        Ok(self.push(Construction::Midpoint(a, b)))
    }
    /// Adds an intersection. Parallel or degenerate lines resolve to None,
    /// including when an edit later makes a previously valid intersection vanish.
    pub fn add_intersection(&mut self, inputs: [PointId; 4]) -> Result<PointId, MathError> {
        self.validate(&inputs)?;
        Ok(self.push(Construction::Intersection(inputs)))
    }
    /// Adds a linked segment or circle after validating all references.
    pub fn add_shape(&mut self, shape: Shape) -> Result<(), MathError> {
        let (Shape::Segment(a, b) | Shape::Circle(a, b)) = shape;
        self.validate(&[a, b])?;
        self.shapes.push(shape);
        Ok(())
    }
    /// Moves a free point. A derived point must be changed through its inputs.
    pub fn move_free(&mut self, id: PointId, position: Point) -> Result<(), MathError> {
        Point::new(position.x, position.y)?;
        match self.points.get_mut(id.0).ok_or(MathError::MissingPoint)? {
            Construction::Free(point) => {
                *point = position;
                Ok(())
            }
            _ => Err(MathError::DerivedPoint),
        }
    }
    /// Evaluates in dependency order without recursion. Unavailable constructions
    /// propagate None to dependants, rather than inventing finite coordinates.
    pub fn resolve(&self, expression: &Expression, parameter: f64) -> Vec<Option<Point>> {
        self.resolve_with(|x| expression.evaluate(x, parameter).ok())
    }
    /// Resolve geometry with an explicit function evaluator. Returning None
    /// propagates missing/undefined function values without hiding free points.
    pub fn resolve_with(&self, function: impl Fn(f64) -> Option<f64>) -> Vec<Option<Point>> {
        let mut result: Vec<Option<Point>> = Vec::with_capacity(self.points.len());
        for node in &self.points {
            let point = match *node {
                Construction::Free(p) => Some(p),
                Construction::OnFunction(x) => function(x).and_then(|y| Point::new(x, y).ok()),
                Construction::Midpoint(a, b) => result[a.0].zip(result[b.0]).and_then(|(a, b)| {
                    Point::new(a.x * 0.5 + b.x * 0.5, a.y * 0.5 + b.y * 0.5).ok()
                }),
                Construction::Intersection(ids) => {
                    let values = ids.map(|id| result[id.0]);
                    match values {
                        [Some(a), Some(b), Some(c), Some(d)] => intersection(a, b, c, d),
                        _ => None,
                    }
                }
            };
            result.push(point);
        }
        result
    }
    fn push(&mut self, construction: Construction) -> PointId {
        let id = PointId(self.points.len());
        self.points.push(construction);
        id
    }
    fn validate(&self, points: &[PointId]) -> Result<(), MathError> {
        if points.iter().all(|id| id.0 < self.points.len()) {
            Ok(())
        } else {
            Err(MathError::MissingPoint)
        }
    }
    /// Returns a handle for a present point, without exposing handle construction.
    pub fn point_id(&self, index: usize) -> Option<PointId> {
        (index < self.points.len()).then_some(PointId(index))
    }
}

/// Nonnegative triangle area; a degenerate triangle has zero area.
pub fn triangle_area(a: Point, b: Point, c: Point) -> Result<f64, MathError> {
    finite(((b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)).abs() * 0.5)
}

fn intersection(a: Point, b: Point, c: Point, d: Point) -> Option<Point> {
    let (ux, uy, vx, vy) = (b.x - a.x, b.y - a.y, d.x - c.x, d.y - c.y);
    let (un, vn) = (ux.hypot(uy), vx.hypot(vy));
    if un == 0.0 || vn == 0.0 || !un.is_finite() || !vn.is_finite() {
        return None;
    }
    let (ux, uy, vx, vy) = (ux / un, uy / un, vx / vn, vy / vn);
    let determinant = ux * vy - uy * vx;
    // Numerical conditioning policy, not an object/work quota.
    if determinant.abs() <= 32.0 * f64::EPSILON {
        return None;
    }
    let along = ((c.x - a.x) * vy - (c.y - a.y) * vx) / determinant;
    Point::new(a.x + along * ux, a.y + along * uy).ok()
}
