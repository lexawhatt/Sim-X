//! Renderer-neutral body-local attachment geometry and surface selection.

use super::document::{Object, ObjectKind, Point};

/// A material point on an authored body, in its local metre coordinate system.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Attachment {
    /// Stable body identity, never an index into the authoring vector.
    pub(crate) body: u64,
    /// Material offset from the unrotated body centre, in metres.
    pub(crate) local_m: Point,
}

impl Attachment {
    /// Centre attachment used by anchors and the current centre-only rods.
    pub(crate) const fn center(body: u64) -> Self {
        Self {
            body,
            local_m: Point::new(0.0, 0.0),
        }
    }
}

/// Resolves local metres through an explicit pose; does not advance time.
pub(crate) fn world_point(position: Point, rotation_deg: f64, local: Point) -> Point {
    if local == Point::default() {
        return position;
    }
    if rotation_deg == 0.0 {
        return Point::new(position.x + local.x, position.y + local.y);
    }
    let (sin, cos) = rotation_deg.to_radians().sin_cos();
    // Match the kernel's rotate-then-translate grouping. A different grouping
    // can invent one-ULP spring strain before the first simulation step.
    let rotated_x = cos * local.x - sin * local.y;
    let rotated_y = sin * local.x + cos * local.y;
    Point::new(position.x + rotated_x, position.y + rotated_y)
}

/// Resolves an attachment against the object's canonical authored pose.
pub(crate) fn authored_point(object: &Object, local: Point) -> Point {
    world_point(object.position, object.rotation_deg, local)
}

/// Validates finite local points inside the shape, with a relative roundoff
/// allowance at its boundary. Legacy centre attachments remain valid. Anchors
/// have no physical surface and accept only their exact centre.
pub(crate) fn valid_local(object: &Object, local: Point) -> bool {
    if !local.x.is_finite() || !local.y.is_finite() {
        return false;
    }
    let half = object.size_m * 0.5;
    let tolerance = object.size_m.max(object.height_m) * 1e-12;
    match object.kind {
        ObjectKind::Anchor => local == Point::default(),
        ObjectKind::Ball => local.x.hypot(local.y) <= half + tolerance,
        ObjectKind::Box => {
            local.x.abs() <= half + tolerance && local.y.abs() <= object.height_m * 0.5 + tolerance
        }
    }
}

/// Preserves a normalized material location through a physical resize.
/// Box axes scale independently; circle axes scale by its diameter together.
pub(crate) fn rescale_local(before: &Object, after: &Object, local: Point) -> Point {
    let vertical_scale = if before.kind == ObjectKind::Box {
        after.height_m / before.height_m
    } else {
        after.size_m / before.size_m
    };
    Point::new(
        local.x * (after.size_m / before.size_m),
        local.y * vertical_scale,
    )
}

/// Chooses the nearest body surface from a nearby pointer. Rectangle corners
/// take precedence within the supplied world-space snapping distance. Ties
/// have a stable order. Zero-distance circle input chooses its local top.
/// Returns None for a remote or non-finite pointer; anchors resolve to centre.
pub(crate) fn pick_surface(object: &Object, pointer: Point, snap_m: f64) -> Option<Point> {
    if !pointer.x.is_finite() || !pointer.y.is_finite() || !snap_m.is_finite() || snap_m < 0.0 {
        return None;
    }
    let (sin, cos) = object.rotation_deg.to_radians().sin_cos();
    let dx = pointer.x - object.position.x;
    let dy = pointer.y - object.position.y;
    let local = Point::new(dx * cos + dy * sin, -dx * sin + dy * cos);
    let half = object.size_m * 0.5;
    if object.kind == ObjectKind::Anchor {
        return (local.x.abs() <= half + snap_m && local.y.abs() <= half + snap_m)
            .then_some(Point::default());
    }
    if object.kind == ObjectKind::Ball {
        let distance = local.x.hypot(local.y);
        if distance > half + snap_m {
            return None;
        }
        return Some(if distance <= f64::EPSILON * half {
            Point::new(0.0, half)
        } else {
            Point::new(local.x / distance * half, local.y / distance * half)
        });
    }
    let height = object.height_m * 0.5;
    let mut corner = None;
    let mut nearest = f64::INFINITY;
    for (x, y) in [
        (-half, -height),
        (half, -height),
        (half, height),
        (-half, height),
    ] {
        let distance = (local.x - x).hypot(local.y - y);
        if distance <= snap_m && distance < nearest {
            corner = Some(Point::new(x, y));
            nearest = distance;
        }
    }
    if corner.is_some() {
        return corner;
    }
    let mut boundary = Point::new(local.x.clamp(-half, half), local.y.clamp(-height, height));
    if (local.x - boundary.x).hypot(local.y - boundary.y) > snap_m {
        return None;
    }
    if local.x.abs() <= half && local.y.abs() <= height {
        if half - local.x.abs() <= height - local.y.abs() {
            boundary.x = if local.x >= 0.0 { half } else { -half };
        } else {
            boundary.y = if local.y >= 0.0 { height } else { -height };
        }
    }
    Some(boundary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phys_editor::document::Document;

    #[test]
    fn rotated_rectangle_corners_snap_from_just_outside_and_faces_project() {
        let mut document = Document::default();
        let id = document
            .add(ObjectKind::Box, Point::new(4.0, -2.0))
            .unwrap();
        document.set_size(id, 4.0).unwrap();
        document.set_height(id, 2.0).unwrap();
        document.rotate(id, 37.0).unwrap();
        let object = document.object(id).unwrap();
        for corner in [
            Point::new(-2.0, -1.0),
            Point::new(2.0, -1.0),
            Point::new(2.0, 1.0),
            Point::new(-2.0, 1.0),
        ] {
            let nearby = Point::new(corner.x * 1.02, corner.y * 1.02);
            assert_eq!(
                pick_surface(object, authored_point(object, nearby), 0.1),
                Some(corner)
            );
        }
        let point =
            pick_surface(object, authored_point(object, Point::new(0.5, 0.8)), 0.05).unwrap();
        assert!((point.x - 0.5).abs() < 1e-12);
        assert_eq!(point.y, 1.0);
        assert!(pick_surface(object, Point::new(100.0, 100.0), 0.1).is_none());
    }

    #[test]
    fn circles_use_real_radius_and_anchor_markers_use_only_centre() {
        let mut document = Document::default();
        let id = document.add(ObjectKind::Ball, Point::default()).unwrap();
        document.set_size(id, 4.0).unwrap();
        let ball = document.object(id).unwrap();
        assert_eq!(
            pick_surface(ball, Point::default(), 0.1),
            Some(Point::new(0.0, 2.0))
        );
        let point = pick_surface(ball, Point::new(1.0, 1.0), 0.1).unwrap();
        assert!((point.x.hypot(point.y) - 2.0).abs() < 1e-12);
        assert!(valid_local(ball, point));
        assert!(!valid_local(ball, Point::new(2.1, 0.0)));
        let id = document.add(ObjectKind::Anchor, Point::default()).unwrap();
        let anchor = document.object(id).unwrap();
        assert_eq!(
            pick_surface(anchor, Point::new(0.4, 0.4), 0.1),
            Some(Point::default())
        );
        assert!(!valid_local(anchor, Point::new(0.1, 0.0)));
        assert!(!valid_local(anchor, Point::new(f64::NAN, 0.0)));
    }

    #[test]
    fn resize_rotation_and_translation_keep_material_points_and_undo_together() {
        let mut document = Document::default();
        let anchor = document
            .add(ObjectKind::Anchor, Point::new(-8.0, 4.0))
            .unwrap();
        let body = document.add(ObjectKind::Box, Point::new(3.0, 1.0)).unwrap();
        document.set_size(body, 4.0).unwrap();
        document.set_height(body, 2.0).unwrap();
        document.rotate(body, 30.0).unwrap();
        document
            .add_spring_at(
                Attachment::center(anchor),
                Attachment {
                    body,
                    local_m: Point::new(2.0, 1.0),
                },
            )
            .unwrap();
        let original_link = document.links()[0];
        let original = document.object(body).unwrap().clone();
        document.set_size(body, 8.0).unwrap();
        assert_eq!(document.links()[0].b_local_m, Point::new(4.0, 1.0));
        assert_eq!(document.links()[0].kind, original_link.kind);
        assert!(document.undo());
        assert_eq!(document.links()[0], original_link);
        assert_eq!(document.object(body).unwrap(), &original);
        assert!(document.redo());
        document.set_height(body, 6.0).unwrap();
        assert_eq!(document.links()[0].b_local_m, Point::new(4.0, 3.0));
        document.rotate(body, 60.0).unwrap();
        let endpoint = authored_point(
            document.object(body).unwrap(),
            document.links()[0].b_local_m,
        );
        assert!((endpoint.x - 0.0).abs() < 1e-12);
        assert!((endpoint.y - 5.0).abs() < 1e-12);
        let before_move = document.links()[0];
        document
            .translate_many(&[anchor, body], Point::new(10.0, -10.0))
            .unwrap();
        assert_eq!(document.links()[0], before_move);
        let moved = authored_point(
            document.object(body).unwrap(),
            document.links()[0].b_local_m,
        );
        assert!((moved.x - endpoint.x - 10.0).abs() < 1e-12);
        assert!((moved.y - endpoint.y + 10.0).abs() < 1e-12);
    }

    #[test]
    fn circle_resizing_scales_both_local_axes_and_rotation_preserves_radius() {
        let mut document = Document::default();
        let anchor = document
            .add(ObjectKind::Anchor, Point::new(-5.0, 0.0))
            .unwrap();
        let body = document.add(ObjectKind::Ball, Point::default()).unwrap();
        document
            .add_spring_at(
                Attachment::center(anchor),
                Attachment {
                    body,
                    local_m: Point::new(0.3, 0.4),
                },
            )
            .unwrap();
        document.set_size(body, 2.0).unwrap();
        assert_eq!(document.links()[0].b_local_m, Point::new(0.6, 0.8));
        document.rotate(body, 90.0).unwrap();
        let point = authored_point(
            document.object(body).unwrap(),
            document.links()[0].b_local_m,
        );
        assert!((point.x + 0.8).abs() < 1e-12);
        assert!((point.y - 0.6).abs() < 1e-12);
        assert!((point.x.hypot(point.y) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn malformed_or_collapsed_attachments_reject_without_history_or_id_consumption() {
        use crate::phys_editor::document::EditError;
        let mut document = Document::default();
        let a = document.add(ObjectKind::Ball, Point::default()).unwrap();
        let b = document
            .add(ObjectKind::Ball, Point::new(1.0, 0.0))
            .unwrap();
        document.set_mass(a, 2.0).unwrap();
        assert!(document.undo());
        for local_m in [
            Point::new(0.5001, 0.0),
            Point::new(f64::NAN, 0.0),
            Point::new(0.0, f64::INFINITY),
        ] {
            assert_eq!(
                document.add_spring_at(Attachment { body: a, local_m }, Attachment::center(b)),
                Err(EditError::InvalidAttachment)
            );
            assert!(document.can_redo());
            assert!(document.links().is_empty());
        }
        assert_eq!(
            document.add_spring_at(
                Attachment {
                    body: a,
                    local_m: Point::new(0.5, 0.0)
                },
                Attachment {
                    body: b,
                    local_m: Point::new(-0.5, 0.0)
                },
            ),
            Err(EditError::InvalidLinkLength)
        );
        assert!(document.can_redo());
        assert_eq!(document.add_spring(a, b).unwrap(), 1);
    }

    #[test]
    fn resizing_or_rotating_into_collapsed_endpoint_rejects_entire_edit() {
        use crate::phys_editor::document::EditError;
        let mut document = Document::default();
        let anchor = document
            .add(ObjectKind::Anchor, Point::new(1.0, 0.0))
            .unwrap();
        let body = document.add(ObjectKind::Box, Point::default()).unwrap();
        document
            .add_spring_at(
                Attachment::center(anchor),
                Attachment {
                    body,
                    local_m: Point::new(0.5, 0.0),
                },
            )
            .unwrap();
        let before = document.object(body).unwrap().clone();
        let link = document.links()[0];
        document.set_mass(body, 2.0).unwrap();
        assert!(document.undo());
        assert_eq!(
            document.set_size(body, 2.0),
            Err(EditError::InvalidLinkLength)
        );
        assert_eq!(document.object(body).unwrap(), &before);
        assert_eq!(document.links()[0], link);
        assert!(document.can_redo());
        assert_eq!(
            document.move_object(body, Point::new(0.5, 0.0)),
            Err(EditError::InvalidLinkLength)
        );
        assert!(document.can_redo());

        let mut rotation_document = Document::default();
        let point = world_point(Point::default(), 90.0, Point::new(0.5, 0.0));
        let anchor = rotation_document.add(ObjectKind::Anchor, point).unwrap();
        let body = rotation_document
            .add(ObjectKind::Ball, Point::default())
            .unwrap();
        rotation_document
            .add_spring_at(
                Attachment::center(anchor),
                Attachment {
                    body,
                    local_m: Point::new(0.5, 0.0),
                },
            )
            .unwrap();
        let before = rotation_document.object(body).unwrap().clone();
        assert_eq!(
            rotation_document.rotate(body, 90.0),
            Err(EditError::InvalidLinkLength)
        );
        assert_eq!(rotation_document.object(body).unwrap(), &before);
    }

    #[test]
    fn prepared_relationships_remain_centred() {
        let mut document = Document::default();
        let bob = document.add_pendulum(Point::new(-5.0, 0.0)).unwrap();
        let rod = document.links()[0];
        document.rotate(bob, 17.0).unwrap();
        document.set_size(bob, 3.0).unwrap();
        assert_eq!(
            document.links()[0],
            rod,
            "Centre rods do not depend on body orientation or surface size"
        );
        document.add_spring_pair(Point::new(5.0, 0.0)).unwrap();
        assert!(
            document.links().iter().all(
                |link| link.a_local_m == Point::default() && link.b_local_m == Point::default()
            )
        );
    }

    #[test]
    fn either_endpoint_resizes_and_selection_translation_rejects_collapsed_points() {
        use crate::phys_editor::document::EditError;
        let mut document = Document::default();
        let a = document.add(ObjectKind::Ball, Point::default()).unwrap();
        let b = document
            .add(ObjectKind::Ball, Point::new(3.0, 0.0))
            .unwrap();
        document
            .add_spring_at(
                Attachment {
                    body: a,
                    local_m: Point::new(0.5, 0.0),
                },
                Attachment {
                    body: b,
                    local_m: Point::new(-0.5, 0.0),
                },
            )
            .unwrap();
        document.set_size(a, 2.0).unwrap();
        assert_eq!(document.links()[0].a_local_m, Point::new(1.0, 0.0));
        assert_eq!(document.links()[0].b_local_m, Point::new(-0.5, 0.0));
        let link = document.links()[0];
        let objects = document.objects().to_vec();
        assert_eq!(
            document.translate_many(&[b], Point::new(-1.5, 0.0)),
            Err(EditError::InvalidLinkLength)
        );
        assert_eq!(document.objects(), objects);
        assert_eq!(document.links()[0], link);
        document
            .translate_many(&[a, b], Point::new(2.0, 1.0))
            .unwrap();
        assert_eq!(document.links()[0], link);
    }
}
