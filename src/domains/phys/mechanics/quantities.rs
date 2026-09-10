use crate::foundation::{Meters, MetersPerSecond, MetersPerSecondSquared, Newtons};

/// A finite 2D world position in meters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Position2 {
    x: Meters,
    y: Meters,
}

impl Position2 {
    /// The world origin.
    pub const ZERO: Self = Self {
        x: Meters::ZERO,
        y: Meters::ZERO,
    };

    /// Constructs a position from typed meter components.
    pub const fn new(x: Meters, y: Meters) -> Self {
        Self { x, y }
    }

    /// Returns the horizontal world component in meters.
    pub const fn x(self) -> Meters {
        self.x
    }

    /// Returns the vertical world component in meters.
    pub const fn y(self) -> Meters {
        self.y
    }
}

/// A finite 2D linear velocity in meters per second.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Velocity2 {
    x: MetersPerSecond,
    y: MetersPerSecond,
}

impl Velocity2 {
    /// Exact zero linear velocity.
    pub const ZERO: Self = Self {
        x: MetersPerSecond::ZERO,
        y: MetersPerSecond::ZERO,
    };

    /// Constructs a velocity from typed components.
    pub const fn new(x: MetersPerSecond, y: MetersPerSecond) -> Self {
        Self { x, y }
    }

    /// Returns the horizontal velocity component.
    pub const fn x(self) -> MetersPerSecond {
        self.x
    }

    /// Returns the vertical velocity component.
    pub const fn y(self) -> MetersPerSecond {
        self.y
    }
}

/// A finite 2D linear acceleration in meters per second squared.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Acceleration2 {
    x: MetersPerSecondSquared,
    y: MetersPerSecondSquared,
}

impl Acceleration2 {
    /// Exact zero linear acceleration.
    pub const ZERO: Self = Self {
        x: MetersPerSecondSquared::ZERO,
        y: MetersPerSecondSquared::ZERO,
    };

    /// Constructs an acceleration from typed components.
    pub const fn new(x: MetersPerSecondSquared, y: MetersPerSecondSquared) -> Self {
        Self { x, y }
    }

    /// Returns the horizontal acceleration component.
    pub const fn x(self) -> MetersPerSecondSquared {
        self.x
    }

    /// Returns the vertical acceleration component.
    pub const fn y(self) -> MetersPerSecondSquared {
        self.y
    }
}

/// A finite 2D force vector in newtons.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Force2 {
    x: Newtons,
    y: Newtons,
}

impl Force2 {
    /// Exact zero force.
    pub const ZERO: Self = Self {
        x: Newtons::ZERO,
        y: Newtons::ZERO,
    };

    /// Constructs a force from typed newton components.
    pub const fn new(x: Newtons, y: Newtons) -> Self {
        Self { x, y }
    }

    /// Returns the horizontal force component.
    pub const fn x(self) -> Newtons {
        self.x
    }

    /// Returns the vertical force component.
    pub const fn y(self) -> Newtons {
        self.y
    }
}
