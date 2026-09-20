//! Small code-native catalog symbols in a bounded, normalized coordinate space.

pub(crate) const LINES_PER_ICON: usize = 8;
pub(crate) const CIRCLES_PER_ICON: usize = 2;

#[derive(Clone, Copy, Debug)]
pub(crate) enum Symbol {
    Objects,
    Ball,
    Box,
    Anchor,
    Rod,
    Spring,
    Pendulum,
    Oscillator,
    BounceLab,
    Search,
    Favorite,
    Close,
    Previous,
    Next,
}

pub(crate) struct Geometry {
    pub(crate) lines: &'static [(f32, f32, f32, f32)],
    pub(crate) circles: &'static [(f32, f32, f32)],
}

impl Symbol {
    pub(crate) fn geometry(self) -> Geometry {
        let (lines, circles): (&[_], &[_]) = match self {
            Self::Objects => (
                &[
                    (-0.7, -0.65, 0.2, -0.65),
                    (0.2, -0.65, 0.2, 0.25),
                    (0.2, 0.25, -0.7, 0.25),
                    (-0.7, 0.25, -0.7, -0.65),
                ],
                &[(0.5, 0.5, 0.4)],
            ),
            Self::Rod => (
                &[(-0.65, 0.45, 0.65, -0.45)],
                &[(-0.65, 0.45, 0.2), (0.65, -0.45, 0.2)],
            ),
            Self::Ball => (&[], &[(0.0, 0.0, 0.65)]),
            Self::Box => (
                &[
                    (-0.65, -0.65, 0.65, -0.65),
                    (0.65, -0.65, 0.65, 0.65),
                    (0.65, 0.65, -0.65, 0.65),
                    (-0.65, 0.65, -0.65, -0.65),
                ],
                &[],
            ),
            Self::Anchor => (
                &[
                    (-0.8, 0.6, 0.8, 0.6),
                    (-0.5, 0.6, 0.0, -0.35),
                    (0.0, -0.35, 0.5, 0.6),
                    (-0.6, 0.65, -0.8, 0.9),
                    (0.0, 0.65, -0.2, 0.9),
                    (0.6, 0.65, 0.4, 0.9),
                ],
                &[(0.0, -0.5, 0.18)],
            ),
            Self::Spring => (
                &[
                    (-0.95, 0.0, -0.7, 0.0),
                    (-0.7, 0.0, -0.5, -0.4),
                    (-0.5, -0.4, -0.25, 0.4),
                    (-0.25, 0.4, 0.0, -0.4),
                    (0.0, -0.4, 0.25, 0.4),
                    (0.25, 0.4, 0.5, -0.4),
                    (0.5, -0.4, 0.7, 0.0),
                    (0.7, 0.0, 0.95, 0.0),
                ],
                &[],
            ),
            Self::Pendulum => (
                &[(-0.6, -0.75, 0.6, -0.75), (0.0, -0.75, 0.5, 0.45)],
                &[(0.0, -0.75, 0.12), (0.5, 0.55, 0.3)],
            ),
            Self::Oscillator => (
                &[
                    (-0.75, -0.8, -0.75, 0.8),
                    (-0.75, 0.0, -0.55, 0.0),
                    (-0.55, 0.0, -0.35, -0.35),
                    (-0.35, -0.35, -0.1, 0.35),
                    (-0.1, 0.35, 0.15, -0.35),
                    (0.15, -0.35, 0.35, 0.0),
                    (0.35, 0.0, 0.6, 0.0),
                ],
                &[(0.65, 0.0, 0.3)],
            ),
            Self::BounceLab => (
                &[
                    (-0.9, 0.75, 0.9, 0.75),
                    (-0.9, 0.75, -0.9, 0.95),
                    (-0.9, 0.95, 0.9, 0.95),
                    (0.9, 0.95, 0.9, 0.75),
                ],
                &[(-0.45, 0.2, 0.25), (0.45, -0.5, 0.25)],
            ),
            Self::Search => (&[(0.25, 0.25, 0.85, 0.85)], &[(-0.2, -0.2, 0.5)]),
            Self::Favorite => (
                &[
                    (0.0, -0.9, 0.53, 0.73),
                    (0.53, 0.73, -0.856, -0.278),
                    (-0.856, -0.278, 0.856, -0.278),
                    (0.856, -0.278, -0.53, 0.73),
                    (-0.53, 0.73, 0.0, -0.9),
                ],
                &[],
            ),
            Self::Close => (
                &[(-0.55, -0.55, 0.55, 0.55), (-0.55, 0.55, 0.55, -0.55)],
                &[],
            ),
            Self::Previous => (&[(0.3, -0.55, -0.3, 0.0), (-0.3, 0.0, 0.3, 0.55)], &[]),
            Self::Next => (&[(-0.3, -0.55, 0.3, 0.0), (0.3, 0.0, -0.3, 0.55)], &[]),
        };
        Geometry { lines, circles }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_icons_fit_the_retained_budget_with_valid_geometry() {
        for symbol in [
            Symbol::Objects,
            Symbol::Ball,
            Symbol::Box,
            Symbol::Anchor,
            Symbol::Rod,
            Symbol::Spring,
            Symbol::Pendulum,
            Symbol::Oscillator,
            Symbol::BounceLab,
            Symbol::Search,
            Symbol::Favorite,
            Symbol::Close,
            Symbol::Previous,
            Symbol::Next,
        ] {
            let geometry = symbol.geometry();
            assert!(geometry.lines.len() <= LINES_PER_ICON, "{symbol:?}");
            assert!(geometry.circles.len() <= CIRCLES_PER_ICON, "{symbol:?}");
            for &(x1, y1, x2, y2) in geometry.lines {
                assert!(
                    [x1, y1, x2, y2]
                        .into_iter()
                        .all(|value| value.is_finite() && value.abs() <= 1.0)
                );
                assert!((x1 - x2).hypot(y1 - y2) > 0.01);
            }
            for &(x, y, radius) in geometry.circles {
                assert!(x.is_finite() && y.is_finite() && radius > 0.0);
                assert!(x.abs() + radius <= 1.0 && y.abs() + radius <= 1.0);
            }
        }
    }
}
