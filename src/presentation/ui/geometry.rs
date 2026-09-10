#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(in crate::presentation) struct Point {
    pub(in crate::presentation) x: f32,
    pub(in crate::presentation) y: f32,
}

impl Point {
    pub(in crate::presentation) const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(in crate::presentation) struct UiRect {
    pub(in crate::presentation) min: Point,
    pub(in crate::presentation) max: Point,
}

impl UiRect {
    pub(in crate::presentation) const fn from_min_size(
        min: Point,
        width: f32,
        height: f32,
    ) -> Self {
        Self {
            min,
            max: Point::new(min.x + width, min.y + height),
        }
    }

    pub(in crate::presentation) fn width(self) -> f32 {
        self.max.x - self.min.x
    }

    pub(in crate::presentation) fn height(self) -> f32 {
        self.max.y - self.min.y
    }

    pub(in crate::presentation) fn center(self) -> Point {
        Point::new(
            (self.min.x + self.max.x) * 0.5,
            (self.min.y + self.max.y) * 0.5,
        )
    }

    pub(in crate::presentation) fn contains(self, point: Point) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
    }

    pub(in crate::presentation) fn expand(self, amount: f32) -> Self {
        Self {
            min: Point::new(self.min.x - amount, self.min.y - amount),
            max: Point::new(self.max.x + amount, self.max.y + amount),
        }
    }
}
