use std::ops::{Add, Mul, Neg, Sub};

/// A two-component value in the right-handed, Y-up physical plane.
/// Units are carried explicitly by the owning field's SI suffix.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec2 {
    /// Horizontal component.
    pub x: f64,
    /// Vertical component, positive upward.
    pub y: f64,
}

impl Vec2 {
    /// The exact zero vector.
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };
    /// Constructs a value; the world boundary validates its components.
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
    /// Euclidean scalar product.
    pub fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y
    }
    /// Euclidean magnitude.
    pub fn length(self) -> f64 {
        self.x.hypot(self.y)
    }
    pub(crate) fn finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

impl Add for Vec2 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}
impl Sub for Vec2 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}
impl Neg for Vec2 {
    type Output = Self;
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y)
    }
}
impl Mul<f64> for Vec2 {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self {
        Self::new(self.x * rhs, self.y * rhs)
    }
}
