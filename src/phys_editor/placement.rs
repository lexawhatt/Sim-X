//! Typed placement recipes; prepared objects expand into ordinary primitives.

use super::document::{Document, EditError, ObjectKind, Point};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Placement {
    Primitive(ObjectKind),
    Pendulum,
    Oscillator,
    BounceLab,
}

impl Placement {
    pub(crate) fn commit(self, document: &mut Document, position: Point) -> Result<u64, EditError> {
        match self {
            Self::Primitive(kind) => document.add(kind, position),
            Self::Pendulum => document.add_pendulum(position),
            Self::Oscillator => document.add_spring_pair(position),
            Self::BounceLab => document.add_bounce_lab(position),
        }
    }

    pub(crate) fn marker(self) -> ObjectKind {
        match self {
            Self::Primitive(kind) => kind,
            Self::Pendulum | Self::Oscillator | Self::BounceLab => ObjectKind::Anchor,
        }
    }
}
