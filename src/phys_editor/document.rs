//! Bounded, renderer-neutral records for authoring a Physics scene.
//!
//! These are editable object descriptions, not simulation bodies. All spatial
//! values use metres; nothing in this module advances time or applies forces.

use std::{collections::VecDeque, error::Error, fmt};

use super::attachment::{Attachment, authored_point, rescale_local, valid_local};

/// Maximum authored objects in this first editor prototype.
pub(crate) const MAX_OBJECTS: usize = 128;
/// Maximum authored relationships, independent of the body count.
pub(crate) const MAX_LINKS: usize = 256;
/// Maximum retained edits, shared by the undo and redo history.
pub(crate) const HISTORY_LIMIT: usize = 64;

const MAX_POSITION_M: f64 = 1_000_000.0;
const MIN_MASS_KG: f64 = 0.001;
const MAX_MASS_KG: f64 = 1_000_000.0;
const MIN_SIZE_M: f64 = 0.1;
const MAX_SIZE_M: f64 = 1_000.0;
const DUPLICATE_OFFSET_M: f64 = 0.5;
const MAX_GRAVITY_M_S2: f64 = 10_000.0;
const MAX_LINEAR_DRAG_PER_S: f64 = 100.0;

/// A position in the editor's world coordinate space, in metres.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Point {
    /// Horizontal coordinate in metres.
    pub x: f64,
    /// Vertical coordinate in metres.
    pub y: f64,
}

impl Point {
    /// Constructs a coordinate pair; document operations validate its bounds.
    pub(crate) const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// Authoring primitives; anchors are non-colliding attachment points.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ObjectKind {
    Ball,
    Box,
    Anchor,
}

impl ObjectKind {
    /// Stable palette order.
    pub(crate) const ALL: [Self; 3] = [Self::Ball, Self::Box, Self::Anchor];

    /// Returns the position in the stable palette order.
    pub(crate) const fn index(self) -> usize {
        match self {
            Self::Ball => 0,
            Self::Box => 1,
            Self::Anchor => 2,
        }
    }

    /// Returns the palette and inspector label.
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Ball => "Ball",
            Self::Box => "Box",
            Self::Anchor => "Anchor",
        }
    }
}

/// An authored object's values, detached from any renderer or simulation.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Object {
    /// Nonzero identifier, never reused by the originating document.
    pub id: u64,
    /// Shape selected from the editor palette.
    pub kind: ObjectKind,
    /// Object centre, in metres.
    pub position: Point,
    /// Authoring mass in kilograms; fixed and unused for anchors.
    pub mass_kg: f64,
    /// Whether this object stays stationary; anchors are always fixed.
    pub fixed: bool,
    /// Normal collision restitution in [0, 1]; unused by anchors.
    pub restitution: f64,
    /// Diameter for balls, width for boxes, marker size for anchors.
    pub size_m: f64,
    /// Physical box height in metres; unused by balls and anchor markers.
    pub height_m: f64,
    /// Authored counterclockwise orientation in [0, 360) degrees.
    pub rotation_deg: f64,
}

/// A physical relationship with body-local attachment coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Link {
    /// Nonzero identity in the document's separate relationship namespace.
    pub id: u64,
    /// First object's stable identity, never a vector index.
    pub a: u64,
    /// Second object's stable identity.
    pub b: u64,
    /// First endpoint relative to the body's unrotated centre, in metres.
    pub a_local_m: Point,
    /// Second endpoint relative to the body's unrotated centre, in metres.
    pub b_local_m: Point,
    /// Canonical physical parameters, not display geometry.
    pub kind: LinkKind,
}

/// Authored physical relationships; named assemblies are not solver types.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum LinkKind {
    /// A bidirectional rigid distance, not a tension-only rope.
    Rod { length_m: f64 },
    /// A conservative axial Hooke spring, with no built-in damping.
    Spring {
        rest_length_m: f64,
        stiffness_n_m: f64,
    },
}

/// Scene-wide physical parameters copied into each independently started run.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PhysicsEnvironment {
    /// Uniform acceleration vector in metres per second squared, with Y up.
    pub gravity_m_s2: Point,
    /// Uniform linear velocity damping rate in inverse seconds.
    pub linear_drag_per_s: f64,
}

impl Default for PhysicsEnvironment {
    fn default() -> Self {
        Self {
            gravity_m_s2: Point::new(0.0, -9.80665),
            linear_drag_per_s: 0.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Snapshot {
    objects: Vec<Object>,
    links: Vec<Link>,
    environment: PhysicsEnvironment,
}

/// A rejected edit; rejection preserves scene, history, and ID allocation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EditError {
    TooManyObjects,
    TooManyLinks,
    IdExhausted,
    LinkIdExhausted,
    MissingObject(u64),
    InvalidPosition,
    InvalidMass,
    InvalidSize,
    InvalidRotation,
    InvalidRestitution,
    HeightRequiresBox,
    AnchorMustRemainFixed,
    AnchorHasNoCollider,
    AnchorMassIsFixed,
    InvalidEnvironment,
    SelfLink,
    FixedEndpoints,
    DuplicateLink,
    InvalidLinkLength,
    InvalidAttachment,
}

impl fmt::Display for EditError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyObjects => write!(formatter, "Scene limit: {MAX_OBJECTS} objects"),
            Self::TooManyLinks => write!(formatter, "Scene limit: {MAX_LINKS} relationships"),
            Self::IdExhausted => formatter.write_str("Object identifiers are exhausted"),
            Self::LinkIdExhausted => formatter.write_str("Relationship identifiers are exhausted"),
            Self::MissingObject(id) => write!(formatter, "Object {id} no longer exists"),
            Self::InvalidPosition => formatter
                .write_str("Position must be finite and within +/-1,000,000 metres on each axis"),
            Self::InvalidMass => formatter.write_str("Mass must be between 0.001 and 1,000,000 kg"),
            Self::InvalidSize => formatter.write_str("Size must be between 0.1 and 1,000 metres"),
            Self::InvalidRotation => formatter.write_str("Rotation must be finite"),
            Self::InvalidRestitution => formatter.write_str("Restitution must be finite within 0..1"),
            Self::HeightRequiresBox => formatter.write_str("Only boxes have an independent height"),
            Self::AnchorMustRemainFixed => formatter.write_str("Anchors must remain fixed"),
            Self::AnchorHasNoCollider => formatter.write_str("Anchors have no collision surface"),
            Self::AnchorMassIsFixed => formatter.write_str("Anchor mass cannot be edited"),
            Self::InvalidEnvironment => formatter.write_str(
                "Gravity must be finite within +/-10,000 m/s^2 per axis; drag within 0..100 per second",
            ),
            Self::SelfLink => formatter.write_str("A relationship needs two different objects"),
            Self::FixedEndpoints => formatter.write_str("A relationship needs a dynamic endpoint"),
            Self::DuplicateLink => formatter.write_str("These objects already have a relationship"),
            Self::InvalidLinkLength => {
                formatter.write_str("Relationship endpoints need a finite positive separation")
            }
            Self::InvalidAttachment => formatter
                .write_str("Attachment must be finite and inside its body's physical geometry"),
        }
    }
}

impl Error for EditError {}

/// An editable, bounded scene with snapshot-based undo and redo.
///
/// History stores only authoring records. ID allocation is deliberately not
/// undone: references to deleted or undone objects never identify new objects.
#[derive(Clone, Debug)]
pub(crate) struct Document {
    objects: Vec<Object>,
    links: Vec<Link>,
    environment: PhysicsEnvironment,
    undo: VecDeque<Snapshot>,
    redo: VecDeque<Snapshot>,
    next_id: Option<u64>,
    next_link_id: Option<u64>,
}

impl Default for Document {
    fn default() -> Self {
        Self {
            objects: Vec::new(),
            links: Vec::new(),
            environment: PhysicsEnvironment::default(),
            undo: VecDeque::new(),
            redo: VecDeque::new(),
            next_id: Some(1),
            next_link_id: Some(1),
        }
    }
}

impl Document {
    /// Returns all authored objects in insertion order.
    pub(crate) fn objects(&self) -> &[Object] {
        &self.objects
    }

    /// Returns authored relationships in insertion order, with immutable IDs.
    pub(crate) fn links(&self) -> &[Link] {
        &self.links
    }

    /// Returns the scene-wide environment; changes require validated commands.
    pub(crate) fn environment(&self) -> PhysicsEnvironment {
        self.environment
    }

    /// Changes all scene parameters as one undoable edit after validating them.
    ///
    /// Gravity components are bounded to +/-10,000 m/s^2 and linear drag to
    /// [0, 100] per second. Rejection and no-op preserve history and allocation.
    pub(crate) fn set_environment(
        &mut self,
        environment: PhysicsEnvironment,
    ) -> Result<(), EditError> {
        for component in [environment.gravity_m_s2.x, environment.gravity_m_s2.y] {
            if !component.is_finite() || component.abs() > MAX_GRAVITY_M_S2 {
                return Err(EditError::InvalidEnvironment);
            }
        }
        if !environment.linear_drag_per_s.is_finite()
            || !(0.0..=MAX_LINEAR_DRAG_PER_S).contains(&environment.linear_drag_per_s)
        {
            return Err(EditError::InvalidEnvironment);
        }
        if environment != self.environment {
            self.record_edit();
            self.environment = environment;
        }
        Ok(())
    }

    /// Looks up an object without permitting unvalidated mutation.
    pub(crate) fn object(&self, id: u64) -> Option<&Object> {
        self.objects.iter().find(|object| object.id == id)
    }

    /// Returns whether there is a retained edit to undo.
    pub(crate) fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// Returns whether there is an undone edit to redo.
    pub(crate) fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Adds a 1 kg, 1 metre object with zero rotation at a valid position.
    ///
    /// Rejects invalid positions, exhausted IDs, and the object limit atomically.
    pub(crate) fn add(&mut self, kind: ObjectKind, position: Point) -> Result<u64, EditError> {
        self.insert(Object {
            id: 0,
            kind,
            position,
            mass_kg: 1.0,
            fixed: kind == ObjectKind::Anchor,
            restitution: 0.35,
            size_m: 1.0,
            height_m: 1.0,
            rotation_deg: 0.0,
        })
    }

    /// Removes an object and its incident relationships in one undoable edit.
    ///
    /// An unknown ID rejects without mutation. No dangling endpoint survives.
    pub(crate) fn remove(&mut self, id: u64) -> Result<(), EditError> {
        let index = self.index_of(id)?;
        self.record_edit();
        self.objects.remove(index);
        self.links.retain(|link| link.a != id && link.b != id);
        Ok(())
    }

    /// Removes a bounded selection and all incident links as one history edit.
    pub(crate) fn remove_many(&mut self, ids: &[u64]) -> Result<(), EditError> {
        self.validate_selection(ids)?;
        if ids.is_empty() {
            return Ok(());
        }
        self.record_edit();
        self.objects.retain(|object| !ids.contains(&object.id));
        self.links
            .retain(|link| !ids.contains(&link.a) && !ids.contains(&link.b));
        Ok(())
    }

    /// Translates a complete selection atomically, preserving internal rods.
    /// External rods adopt authored endpoint separation; springs keep rest length.
    pub(crate) fn translate_many(&mut self, ids: &[u64], delta: Point) -> Result<(), EditError> {
        self.validate_selection(ids)?;
        validate_position(delta)?;
        if ids.is_empty() || delta == Point::default() {
            return Ok(());
        }
        let mut objects = self.objects.clone();
        for object in &mut objects {
            if ids.contains(&object.id) {
                object.position.x += delta.x;
                object.position.y += delta.y;
                validate_position(object.position)?;
            }
        }
        let mut links = self.links.clone();
        for link in &mut links {
            let a_moves = ids.contains(&link.a);
            let b_moves = ids.contains(&link.b);
            if !a_moves && !b_moves {
                continue;
            }
            let a = objects
                .iter()
                .find(|object| object.id == link.a)
                .ok_or(EditError::MissingObject(link.a))?;
            let b = objects
                .iter()
                .find(|object| object.id == link.b)
                .ok_or(EditError::MissingObject(link.b))?;
            let length = separation(
                authored_point(a, link.a_local_m),
                authored_point(b, link.b_local_m),
            )?;
            if a_moves != b_moves
                && let LinkKind::Rod { length_m } = &mut link.kind
            {
                *length_m = length;
            }
        }
        self.record_edit();
        self.objects = objects;
        self.links = links;
        Ok(())
    }

    fn validate_selection(&self, ids: &[u64]) -> Result<(), EditError> {
        if ids.len() > MAX_OBJECTS {
            return Err(EditError::TooManyObjects);
        }
        for id in ids {
            self.index_of(*id)?;
        }
        Ok(())
    }

    /// Moves an object within +/-1,000,000 metres per axis.
    ///
    /// Incident rods adopt their new authored length; springs retain their rest
    /// lengths so displacement can author initial strain. Coincident linked
    /// endpoints reject the entire move. No-op and invalid edits preserve history.
    pub(crate) fn move_object(&mut self, id: u64, position: Point) -> Result<(), EditError> {
        validate_position(position)?;
        self.update_object(id, |object| object.position = position)
    }

    /// Sets mass to a finite value in [0.001, 1,000,000] kilograms.
    ///
    /// Anchor mass is fixed. No-op and rejected edits preserve undo and redo.
    pub(crate) fn set_mass(&mut self, id: u64, mass_kg: f64) -> Result<(), EditError> {
        if !mass_kg.is_finite() || !(MIN_MASS_KG..=MAX_MASS_KG).contains(&mass_kg) {
            return Err(EditError::InvalidMass);
        }
        if self.object(id).ok_or(EditError::MissingObject(id))?.kind == ObjectKind::Anchor {
            return Err(EditError::AnchorMassIsFixed);
        }
        self.update_object(id, |object| object.mass_kg = mass_kg)
    }

    /// Sets size to a finite value in [0.1, 1,000] metres.
    /// Local attachment coordinates scale with physical geometry. A collapsed
    /// relationship rejects the entire edit, including geometry and history.
    pub(crate) fn set_size(&mut self, id: u64, size_m: f64) -> Result<(), EditError> {
        if !size_m.is_finite() || !(MIN_SIZE_M..=MAX_SIZE_M).contains(&size_m) {
            return Err(EditError::InvalidSize);
        }
        self.update_object(id, |object| object.size_m = size_m)
    }

    /// Changes box height within [0.1, 1,000] metres as one undoable edit.
    /// Other object kinds reject; invalid values and no-ops preserve history.
    pub(crate) fn set_height(&mut self, id: u64, height_m: f64) -> Result<(), EditError> {
        if !height_m.is_finite() || !(MIN_SIZE_M..=MAX_SIZE_M).contains(&height_m) {
            return Err(EditError::InvalidSize);
        }
        if self.object(id).ok_or(EditError::MissingObject(id))?.kind != ObjectKind::Box {
            return Err(EditError::HeightRequiresBox);
        }
        self.update_object(id, |object| object.height_m = height_m)
    }

    /// Sets restitution within [0, 1] for balls or boxes, without changing mass.
    /// Anchors have no collider. Rejection and no-ops preserve both histories.
    pub(crate) fn set_restitution(&mut self, id: u64, restitution: f64) -> Result<(), EditError> {
        if !restitution.is_finite() || !(0.0..=1.0).contains(&restitution) {
            return Err(EditError::InvalidRestitution);
        }
        if self.object(id).ok_or(EditError::MissingObject(id))?.kind == ObjectKind::Anchor {
            return Err(EditError::AnchorHasNoCollider);
        }
        self.update_object(id, |object| object.restitution = restitution)
    }

    /// Changes mobility without changing geometry or physical mass.
    /// Anchors cannot become dynamic, and every existing link must retain at
    /// least one dynamic endpoint. The complete graph is checked before commit.
    pub(crate) fn set_fixed(&mut self, id: u64, fixed: bool) -> Result<(), EditError> {
        let object = self.object(id).ok_or(EditError::MissingObject(id))?;
        if object.kind == ObjectKind::Anchor && !fixed {
            return Err(EditError::AnchorMustRemainFixed);
        }
        if object.fixed == fixed {
            return Ok(());
        }
        if fixed {
            for link in &self.links {
                let other = if link.a == id {
                    link.b
                } else if link.b == id {
                    link.a
                } else {
                    continue;
                };
                if self
                    .object(other)
                    .ok_or(EditError::MissingObject(other))?
                    .fixed
                {
                    return Err(EditError::FixedEndpoints);
                }
            }
        }
        self.update_object(id, |object| object.fixed = fixed)
    }

    /// Rotates by a finite delta, normalizing the result into [0, 360).
    pub(crate) fn rotate(&mut self, id: u64, delta_deg: f64) -> Result<(), EditError> {
        if !delta_deg.is_finite() {
            return Err(EditError::InvalidRotation);
        }
        self.update_object(id, |object| {
            // Reduce the delta before adding to avoid intermediate overflow.
            object.rotation_deg = (object.rotation_deg + delta_deg.rem_euclid(360.0)) % 360.0;
        })
    }

    /// Connects two body centres with a rod at their present separation.
    ///
    /// Rejects self-links, missing or coincident endpoints, fixed-fixed pairs,
    /// an existing unordered pair, exhausted IDs, and the relationship budget.
    pub(crate) fn add_rod(&mut self, a: u64, b: u64) -> Result<u64, EditError> {
        let (object_a, object_b) = self.validate_pair(a, b)?;
        let length_m = separation(object_a.position, object_b.position)?;
        self.insert_link(
            Attachment::center(a),
            Attachment::center(b),
            LinkKind::Rod { length_m },
        )
    }

    /// Connects two body centres with a 20 N/m spring, initially unstrained.
    ///
    /// Has the same endpoint and budget requirements as `add_rod`.
    pub(crate) fn add_spring(&mut self, a: u64, b: u64) -> Result<u64, EditError> {
        let (object_a, object_b) = self.validate_pair(a, b)?;
        let rest_length_m = separation(object_a.position, object_b.position)?;
        self.insert_link(
            Attachment::center(a),
            Attachment::center(b),
            LinkKind::Spring {
                rest_length_m,
                stiffness_n_m: 20.0,
            },
        )
    }

    /// Connects two finite points inside their owners' physical geometry.
    /// The editor surface picker supplies boundary points; centre attachments
    /// remain valid for existing prepared assemblies. Rest length is the real
    /// world-space endpoint separation, not the distance between body centres.
    /// Invalid local coordinates, collapsed endpoints or budgets reject atomically.
    pub(crate) fn add_spring_at(&mut self, a: Attachment, b: Attachment) -> Result<u64, EditError> {
        if a.local_m == Point::default() && b.local_m == Point::default() {
            return self.add_spring(a.body, b.body);
        }
        let (object_a, object_b) = self.validate_pair(a.body, b.body)?;
        if !valid_local(object_a, a.local_m) || !valid_local(object_b, b.local_m) {
            return Err(EditError::InvalidAttachment);
        }
        let rest_length_m = separation(
            authored_point(object_a, a.local_m),
            authored_point(object_b, b.local_m),
        )?;
        self.insert_link(
            a,
            b,
            LinkKind::Spring {
                rest_length_m,
                stiffness_n_m: 20.0,
            },
        )
    }

    /// Adds an anchor and a 1 kg bob joined by a 3 m rod, tilted 25 degrees.
    ///
    /// `origin` locates the anchor. Returns the bob ID for selection. This is a
    /// graph recipe, not a distinct object kind; the entire graph is one edit.
    pub(crate) fn add_pendulum(&mut self, origin: Point) -> Result<u64, EditError> {
        let angle = 25.0_f64.to_radians();
        self.insert_pair(
            origin,
            Point::new(origin.x + 3.0 * angle.sin(), origin.y - 3.0 * angle.cos()),
            LinkKind::Rod { length_m: 3.0 },
        )
    }

    /// Adds an anchor and a 1 kg bob 2.5 m below it, joined by a 20 N/m spring.
    ///
    /// The 2 m rest length provides initial extension. `origin` locates the
    /// anchor. Returns the bob ID. IDs, limits, and history commit together.
    pub(crate) fn add_spring_pair(&mut self, origin: Point) -> Result<u64, EditError> {
        self.insert_pair(
            origin,
            Point::new(origin.x, origin.y - 2.5),
            LinkKind::Spring {
                rest_length_m: 2.0,
                stiffness_n_m: 20.0,
            },
        )
    }

    /// Adds a fixed 8 x 0.5 m box and two 1 kg, 1 m balls as one edit.
    /// The balls start 3 m above its centre, with restitution 0.2 and 0.8.
    /// This is an ordinary three-body composition, not an implicit world floor.
    /// Validates positions, capacity and all three IDs before any mutation.
    pub(crate) fn add_bounce_lab(&mut self, origin: Point) -> Result<u64, EditError> {
        if self.objects.len() > MAX_OBJECTS - 3 {
            return Err(EditError::TooManyObjects);
        }
        let first = self.next_id.ok_or(EditError::IdExhausted)?;
        let second = first.checked_add(1).ok_or(EditError::IdExhausted)?;
        let third = second.checked_add(1).ok_or(EditError::IdExhausted)?;
        let objects = [
            Object {
                id: first,
                kind: ObjectKind::Box,
                position: Point::new(origin.x, origin.y - 3.0),
                mass_kg: 1.0,
                fixed: true,
                restitution: 0.0,
                size_m: 8.0,
                height_m: 0.5,
                rotation_deg: 0.0,
            },
            Object {
                id: second,
                kind: ObjectKind::Ball,
                position: Point::new(origin.x - 1.5, origin.y),
                mass_kg: 1.0,
                fixed: false,
                restitution: 0.2,
                size_m: 1.0,
                height_m: 1.0,
                rotation_deg: 0.0,
            },
            Object {
                id: third,
                kind: ObjectKind::Ball,
                position: Point::new(origin.x + 1.5, origin.y),
                mass_kg: 1.0,
                fixed: false,
                restitution: 0.8,
                size_m: 1.0,
                height_m: 1.0,
                rotation_deg: 0.0,
            },
        ];
        for object in &objects {
            validate_position(object.position)?;
        }
        self.record_edit();
        self.objects.extend(objects);
        self.next_id = third.checked_add(1);
        Ok(third)
    }

    /// Copies an object with a fresh ID and a +0.5 metre offset on both axes.
    ///
    /// Rejects unknown IDs, limits, and out-of-range resulting positions before
    /// modifying any authoring state or history. This copies only the body,
    /// never its external relationships; graph copying is a separate operation.
    pub(crate) fn duplicate(&mut self, id: u64) -> Result<u64, EditError> {
        let mut object = self.object(id).ok_or(EditError::MissingObject(id))?.clone();
        object.position.x += DUPLICATE_OFFSET_M;
        object.position.y += DUPLICATE_OFFSET_M;
        self.insert(object)
    }

    /// Restores the previous snapshot; returns false if no undo remains.
    pub(crate) fn undo(&mut self) -> bool {
        let Some(previous) = self.undo.pop_back() else {
            return false;
        };
        let current = self.restore(previous);
        self.redo.push_back(current);
        true
    }

    /// Restores the next snapshot; returns false if no redo remains.
    pub(crate) fn redo(&mut self) -> bool {
        let Some(next) = self.redo.pop_back() else {
            return false;
        };
        let current = self.restore(next);
        self.undo.push_back(current);
        true
    }

    fn insert(&mut self, mut object: Object) -> Result<u64, EditError> {
        if self.objects.len() >= MAX_OBJECTS {
            return Err(EditError::TooManyObjects);
        }
        validate_position(object.position)?;
        let id = self.next_id.ok_or(EditError::IdExhausted)?;
        object.id = id;
        self.record_edit();
        self.objects.push(object);
        self.next_id = id.checked_add(1);
        Ok(id)
    }

    fn validate_pair(&self, a: u64, b: u64) -> Result<(&Object, &Object), EditError> {
        if a == b {
            return Err(EditError::SelfLink);
        }
        let object_a = self.object(a).ok_or(EditError::MissingObject(a))?;
        let object_b = self.object(b).ok_or(EditError::MissingObject(b))?;
        if object_a.fixed && object_b.fixed {
            return Err(EditError::FixedEndpoints);
        }
        if self
            .links
            .iter()
            .any(|link| (link.a == a && link.b == b) || (link.a == b && link.b == a))
        {
            return Err(EditError::DuplicateLink);
        }
        Ok((object_a, object_b))
    }

    fn insert_link(
        &mut self,
        a: Attachment,
        b: Attachment,
        kind: LinkKind,
    ) -> Result<u64, EditError> {
        if self.links.len() >= MAX_LINKS {
            return Err(EditError::TooManyLinks);
        }
        let id = self.next_link_id.ok_or(EditError::LinkIdExhausted)?;
        self.record_edit();
        self.links.push(Link {
            id,
            a: a.body,
            b: b.body,
            a_local_m: a.local_m,
            b_local_m: b.local_m,
            kind,
        });
        self.next_link_id = id.checked_add(1);
        Ok(id)
    }

    fn insert_pair(
        &mut self,
        anchor_position: Point,
        bob_position: Point,
        kind: LinkKind,
    ) -> Result<u64, EditError> {
        if self.objects.len() > MAX_OBJECTS - 2 {
            return Err(EditError::TooManyObjects);
        }
        if self.links.len() >= MAX_LINKS {
            return Err(EditError::TooManyLinks);
        }
        validate_position(anchor_position)?;
        validate_position(bob_position)?;
        separation(anchor_position, bob_position)?;
        let anchor_id = self.next_id.ok_or(EditError::IdExhausted)?;
        let bob_id = anchor_id.checked_add(1).ok_or(EditError::IdExhausted)?;
        let link_id = self.next_link_id.ok_or(EditError::LinkIdExhausted)?;
        let objects = [
            Object {
                id: anchor_id,
                kind: ObjectKind::Anchor,
                position: anchor_position,
                mass_kg: 1.0,
                fixed: true,
                restitution: 0.35,
                size_m: 1.0,
                height_m: 1.0,
                rotation_deg: 0.0,
            },
            Object {
                id: bob_id,
                kind: ObjectKind::Ball,
                position: bob_position,
                mass_kg: 1.0,
                fixed: false,
                restitution: 0.35,
                size_m: 1.0,
                height_m: 1.0,
                rotation_deg: 0.0,
            },
        ];
        self.record_edit();
        self.objects.extend(objects);
        self.links.push(Link {
            id: link_id,
            a: anchor_id,
            b: bob_id,
            a_local_m: Point::default(),
            b_local_m: Point::default(),
            kind,
        });
        self.next_id = bob_id.checked_add(1);
        self.next_link_id = link_id.checked_add(1);
        Ok(bob_id)
    }

    fn index_of(&self, id: u64) -> Result<usize, EditError> {
        self.objects
            .iter()
            .position(|object| object.id == id)
            .ok_or(EditError::MissingObject(id))
    }

    fn update_object(
        &mut self,
        id: u64,
        update: impl FnOnce(&mut Object),
    ) -> Result<(), EditError> {
        let index = self.index_of(id)?;
        let mut candidate = self.objects[index].clone();
        update(&mut candidate);
        let previous = &self.objects[index];
        if candidate == *previous {
            return Ok(());
        }
        let mut links = self.links.clone();
        if candidate.position != previous.position
            || candidate.rotation_deg != previous.rotation_deg
            || candidate.size_m != previous.size_m
            || candidate.height_m != previous.height_m
        {
            for link in &mut links {
                if link.a != id && link.b != id {
                    continue;
                }
                if link.a == id {
                    link.a_local_m = rescale_local(previous, &candidate, link.a_local_m);
                }
                if link.b == id {
                    link.b_local_m = rescale_local(previous, &candidate, link.b_local_m);
                }
                let a = if link.a == id {
                    &candidate
                } else {
                    self.object(link.a)
                        .ok_or(EditError::MissingObject(link.a))?
                };
                let b = if link.b == id {
                    &candidate
                } else {
                    self.object(link.b)
                        .ok_or(EditError::MissingObject(link.b))?
                };
                if !valid_local(a, link.a_local_m) || !valid_local(b, link.b_local_m) {
                    return Err(EditError::InvalidAttachment);
                }
                let length = separation(
                    authored_point(a, link.a_local_m),
                    authored_point(b, link.b_local_m),
                )?;
                if candidate.position != previous.position
                    && let LinkKind::Rod { length_m } = &mut link.kind
                {
                    *length_m = length;
                }
            }
        }
        self.record_edit();
        self.objects[index] = candidate;
        self.links = links;
        Ok(())
    }

    fn record_edit(&mut self) {
        if self.undo.len() == HISTORY_LIMIT {
            self.undo.pop_front();
        }
        self.undo.push_back(Snapshot {
            objects: self.objects.clone(),
            links: self.links.clone(),
            environment: self.environment,
        });
        self.redo.clear();
    }

    fn restore(&mut self, snapshot: Snapshot) -> Snapshot {
        Snapshot {
            objects: std::mem::replace(&mut self.objects, snapshot.objects),
            links: std::mem::replace(&mut self.links, snapshot.links),
            environment: std::mem::replace(&mut self.environment, snapshot.environment),
        }
    }
}

fn separation(a: Point, b: Point) -> Result<f64, EditError> {
    let length = (b.x - a.x).hypot(b.y - a.y);
    if !length.is_finite() || length <= 0.0 {
        return Err(EditError::InvalidLinkLength);
    }
    Ok(length)
}

fn validate_position(position: Point) -> Result<(), EditError> {
    if !position.x.is_finite()
        || !position.y.is_finite()
        || position.x.abs() > MAX_POSITION_M
        || position.y.abs() > MAX_POSITION_M
    {
        return Err(EditError::InvalidPosition);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_unchanged(before: &Document, after: &Document) {
        assert_eq!(before.objects, after.objects);
        assert_eq!(before.links, after.links);
        assert_eq!(before.environment, after.environment);
        assert_eq!(before.undo, after.undo);
        assert_eq!(before.redo, after.redo);
        assert_eq!(before.next_id, after.next_id);
        assert_eq!(before.next_link_id, after.next_link_id);
    }

    #[test]
    fn collision_properties_are_validated_undoable_and_preserve_redo_on_rejection() {
        let mut document = Document::default();
        let id = document.add(ObjectKind::Box, Point::default()).unwrap();
        assert_eq!(document.object(id).unwrap().height_m, 1.0);
        assert_eq!(document.object(id).unwrap().restitution, 0.35);
        assert!(!document.object(id).unwrap().fixed);
        document.set_height(id, 0.5).unwrap();
        document.set_restitution(id, 0.8).unwrap();
        document.set_fixed(id, true).unwrap();
        let final_object = document.object(id).unwrap().clone();
        for _ in 0..3 {
            assert!(document.undo());
        }
        let before = document.clone();
        document.set_height(id, 1.0).unwrap();
        document.set_restitution(id, 0.35).unwrap();
        document.set_fixed(id, false).unwrap();
        for value in [f64::NAN, f64::INFINITY, -0.1, 1.01] {
            assert_eq!(
                document.set_restitution(id, value),
                Err(EditError::InvalidRestitution)
            );
        }
        for value in [f64::NAN, f64::INFINITY, 0.0, 0.09, 1000.1] {
            assert_eq!(document.set_height(id, value), Err(EditError::InvalidSize));
        }
        assert_unchanged(&before, &document);
        for _ in 0..3 {
            assert!(document.redo());
        }
        assert_eq!(document.object(id), Some(&final_object));
        for restitution in [0.0, 1.0] {
            document.set_restitution(id, restitution).unwrap();
        }
        for height in [MIN_SIZE_M, MAX_SIZE_M] {
            document.set_height(id, height).unwrap();
        }
    }

    #[test]
    fn mobility_changes_preserve_dynamic_link_endpoints_and_anchor_invariants() {
        let mut document = Document::default();
        let anchor = document
            .add(ObjectKind::Anchor, Point::new(0.0, 3.0))
            .unwrap();
        let ball = document.add(ObjectKind::Ball, Point::default()).unwrap();
        let block = document.add(ObjectKind::Box, Point::new(3.0, 0.0)).unwrap();
        assert!(document.object(anchor).unwrap().fixed);
        document.add_rod(anchor, ball).unwrap();
        document.add_spring(ball, block).unwrap();
        document.set_fixed(block, true).unwrap();
        let before = document.clone();
        assert_eq!(
            document.set_fixed(ball, true),
            Err(EditError::FixedEndpoints)
        );
        assert_eq!(
            document.set_fixed(anchor, false),
            Err(EditError::AnchorMustRemainFixed)
        );
        assert_eq!(
            document.set_restitution(anchor, 0.5),
            Err(EditError::AnchorHasNoCollider)
        );
        assert_eq!(
            document.set_height(ball, 2.0),
            Err(EditError::HeightRequiresBox)
        );
        assert_eq!(
            document.add_rod(anchor, block),
            Err(EditError::FixedEndpoints)
        );
        document.set_fixed(anchor, true).unwrap();
        assert_unchanged(&before, &document);
        for result in [
            document.set_fixed(999, true),
            document.set_restitution(999, 0.5),
            document.set_height(999, 1.0),
        ] {
            assert_eq!(result, Err(EditError::MissingObject(999)));
        }
        assert_unchanged(&before, &document);
    }

    #[test]
    fn bounce_lab_is_three_public_bodies_and_one_atomic_edit() {
        let mut document = Document::default();
        let selected = document.add_bounce_lab(Point::new(10.0, 20.0)).unwrap();
        assert_eq!(selected, 3);
        assert!(document.links().is_empty());
        assert_eq!(document.undo.len(), 1);
        let floor = &document.objects()[0];
        assert_eq!(floor.kind, ObjectKind::Box);
        assert!(floor.fixed);
        assert_eq!(
            (floor.size_m, floor.height_m, floor.restitution),
            (8.0, 0.5, 0.0)
        );
        assert_eq!(floor.position, Point::new(10.0, 17.0));
        for (object, x, restitution) in [
            (&document.objects()[1], 8.5, 0.2),
            (&document.objects()[2], 11.5, 0.8),
        ] {
            assert_eq!(object.kind, ObjectKind::Ball);
            assert!(!object.fixed);
            assert_eq!(
                (object.mass_kg, object.size_m, object.restitution),
                (1.0, 1.0, restitution)
            );
            assert_eq!(object.position, Point::new(x, 20.0));
        }
        let objects = document.objects().to_vec();
        assert!(document.undo());
        assert!(document.objects().is_empty());
        let before = document.clone();
        for origin in [
            Point::new(f64::NAN, 0.0),
            Point::new(MAX_POSITION_M, 0.0),
            Point::new(0.0, -MAX_POSITION_M),
        ] {
            assert_eq!(
                document.add_bounce_lab(origin),
                Err(EditError::InvalidPosition)
            );
        }
        assert_unchanged(&before, &document);
        assert!(document.redo());
        assert_eq!(document.objects(), objects);
        document.next_id = Some(u64::MAX - 1);
        let before = document.clone();
        assert_eq!(
            document.add_bounce_lab(Point::default()),
            Err(EditError::IdExhausted)
        );
        assert_unchanged(&before, &document);
        document.next_id = Some(u64::MAX - 2);
        assert_eq!(document.add_bounce_lab(Point::default()).unwrap(), u64::MAX);
        assert_eq!(document.next_id, None);
    }

    #[test]
    fn default_is_empty_and_palette_order_is_stable() {
        let mut document = Document::default();
        assert!(document.objects().is_empty());
        assert!(!document.can_undo());
        assert!(!document.can_redo());
        assert!(!document.undo());
        assert!(!document.redo());
        assert!(document.object(0).is_none());
        for (index, kind) in ObjectKind::ALL.into_iter().enumerate() {
            assert_eq!(kind.index(), index);
            assert!(!kind.label().is_empty());
        }
    }

    #[test]
    fn edits_round_trip_through_undo_and_redo() {
        let mut document = Document::default();
        let id = document
            .add(ObjectKind::Ball, Point::new(2.0, -3.0))
            .unwrap();
        let original = document.objects().to_vec();
        document.set_mass(id, 7.5).unwrap();
        document.set_size(id, 4.0).unwrap();
        document.move_object(id, Point::new(5.0, 6.0)).unwrap();
        document.rotate(id, -45.0).unwrap();
        let changed = document.objects().to_vec();
        assert_eq!(changed[0].rotation_deg, 315.0);
        for _ in 0..4 {
            assert!(document.undo());
        }
        assert_eq!(document.objects(), original);
        for _ in 0..4 {
            assert!(document.redo());
        }
        assert_eq!(document.objects(), changed);
        assert!(!document.can_redo());
    }

    #[test]
    fn fresh_edit_clears_redo_without_reusing_identifiers() {
        let mut document = Document::default();
        let first = document.add(ObjectKind::Box, Point::default()).unwrap();
        assert!(document.undo());
        let second = document.add(ObjectKind::Box, Point::default()).unwrap();
        assert!(second > first);
        assert!(!document.can_redo());
        assert!(document.object(first).is_none());
        document.remove(second).unwrap();
        let third = document.add(ObjectKind::Ball, Point::default()).unwrap();
        assert!(third > second);
    }

    #[test]
    fn no_op_edits_preserve_history_and_redo() {
        let mut document = Document::default();
        let id = document.add(ObjectKind::Ball, Point::default()).unwrap();
        document.set_mass(id, 5.0).unwrap();
        assert!(document.undo());
        let before = document.clone();
        document.move_object(id, Point::default()).unwrap();
        document.set_mass(id, 1.0).unwrap();
        document.set_size(id, 1.0).unwrap();
        document.rotate(id, 360.0).unwrap();
        assert_unchanged(&before, &document);
        assert!(document.redo());
        assert_eq!(document.object(id).unwrap().mass_kg, 5.0);
    }

    #[test]
    fn invalid_numeric_edits_are_atomic_and_keep_redo() {
        let mut document = Document::default();
        let id = document.add(ObjectKind::Ball, Point::default()).unwrap();
        document.set_mass(id, 2.0).unwrap();
        document.undo();
        let before = document.clone();
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1_000_001.0] {
            assert_eq!(
                document.move_object(id, Point::new(value, 0.0)),
                Err(EditError::InvalidPosition)
            );
            assert_eq!(
                document.add(ObjectKind::Box, Point::new(0.0, value)),
                Err(EditError::InvalidPosition)
            );
        }
        for value in [f64::NAN, f64::INFINITY, -1.0, 0.0, 0.000_9, 1_000_001.0] {
            assert_eq!(document.set_mass(id, value), Err(EditError::InvalidMass));
        }
        for value in [f64::NAN, f64::NEG_INFINITY, 0.0, 0.09, 1_001.0] {
            assert_eq!(document.set_size(id, value), Err(EditError::InvalidSize));
        }
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(document.rotate(id, value), Err(EditError::InvalidRotation));
        }
        assert_unchanged(&before, &document);
    }

    #[test]
    fn inclusive_numeric_boundaries_and_large_rotations_are_valid() {
        let mut document = Document::default();
        let id = document
            .add(ObjectKind::Box, Point::new(-MAX_POSITION_M, MAX_POSITION_M))
            .unwrap();
        for mass in [MIN_MASS_KG, MAX_MASS_KG] {
            document.set_mass(id, mass).unwrap();
            assert_eq!(document.object(id).unwrap().mass_kg, mass);
        }
        for size in [MIN_SIZE_M, MAX_SIZE_M] {
            document.set_size(id, size).unwrap();
            assert_eq!(document.object(id).unwrap().size_m, size);
        }
        for delta in [f64::MAX, -f64::MAX, -360.0, 720.0, -f64::MIN_POSITIVE] {
            document.rotate(id, delta).unwrap();
            assert!((0.0..360.0).contains(&document.object(id).unwrap().rotation_deg));
        }
    }

    #[test]
    fn unknown_object_operations_and_anchor_mass_are_atomic() {
        let mut document = Document::default();
        let id = document.add(ObjectKind::Anchor, Point::default()).unwrap();
        let before = document.clone();
        assert_eq!(
            document.set_mass(id, 2.0),
            Err(EditError::AnchorMassIsFixed)
        );
        assert_eq!(document.remove(0), Err(EditError::MissingObject(0)));
        assert_eq!(document.duplicate(0), Err(EditError::MissingObject(0)));
        assert_eq!(document.set_mass(0, 2.0), Err(EditError::MissingObject(0)));
        assert_eq!(document.set_size(0, 2.0), Err(EditError::MissingObject(0)));
        assert_eq!(document.rotate(0, 1.0), Err(EditError::MissingObject(0)));
        assert_eq!(
            document.move_object(0, Point::default()),
            Err(EditError::MissingObject(0))
        );
        assert_unchanged(&before, &document);
    }

    #[test]
    fn duplicate_preserves_authored_values_with_fresh_id() {
        let mut document = Document::default();
        let id = document.add(ObjectKind::Box, Point::new(1.0, 2.0)).unwrap();
        document.set_mass(id, 4.0).unwrap();
        document.set_size(id, 2.0).unwrap();
        document.set_height(id, 0.5).unwrap();
        document.set_fixed(id, true).unwrap();
        document.set_restitution(id, 0.9).unwrap();
        document.rotate(id, 30.0).unwrap();
        let duplicate_id = document.duplicate(id).unwrap();
        let duplicate = document.object(duplicate_id).unwrap().clone();
        let mut expected = document.object(id).unwrap().clone();
        expected.id = duplicate_id;
        expected.position = Point::new(1.5, 2.5);
        assert_eq!(duplicate, expected);
        assert_ne!(duplicate_id, id);
        assert!(document.undo());
        assert!(document.object(duplicate_id).is_none());
        assert!(document.redo());
        assert_eq!(document.object(duplicate_id), Some(&duplicate));
    }

    #[test]
    fn duplicate_at_position_boundary_rejects_without_allocating_id() {
        let mut document = Document::default();
        let id = document
            .add(ObjectKind::Ball, Point::new(MAX_POSITION_M, 0.0))
            .unwrap();
        let before = document.clone();
        assert_eq!(document.duplicate(id), Err(EditError::InvalidPosition));
        assert_unchanged(&before, &document);
    }

    #[test]
    fn object_count_and_combined_history_are_bounded() {
        let mut document = Document::default();
        for _ in 0..MAX_OBJECTS {
            document.add(ObjectKind::Ball, Point::default()).unwrap();
        }
        assert_eq!(document.objects().len(), MAX_OBJECTS);
        assert_eq!(document.undo.len(), HISTORY_LIMIT);
        let before = document.clone();
        assert_eq!(
            document.add(ObjectKind::Box, Point::default()),
            Err(EditError::TooManyObjects)
        );
        assert_eq!(document.duplicate(1), Err(EditError::TooManyObjects));
        assert_unchanged(&before, &document);
        for _ in 0..HISTORY_LIMIT {
            assert!(document.undo());
            assert_eq!(document.undo.len() + document.redo.len(), HISTORY_LIMIT);
        }
        assert!(!document.undo());
        assert_eq!(document.objects().len(), MAX_OBJECTS - HISTORY_LIMIT);
        for _ in 0..HISTORY_LIMIT {
            assert!(document.redo());
            assert_eq!(document.undo.len() + document.redo.len(), HISTORY_LIMIT);
        }
        assert!(!document.redo());
        assert_eq!(document.objects().len(), MAX_OBJECTS);
    }

    #[test]
    fn last_nonzero_identifier_is_usable_exactly_once() {
        let mut document = Document {
            next_id: Some(u64::MAX),
            ..Document::default()
        };
        let id = document.add(ObjectKind::Ball, Point::default()).unwrap();
        assert_eq!(id, u64::MAX);
        let before = document.clone();
        assert_eq!(
            document.add(ObjectKind::Box, Point::default()),
            Err(EditError::IdExhausted)
        );
        assert_unchanged(&before, &document);
        assert!(document.undo());
        assert_eq!(
            document.add(ObjectKind::Box, Point::default()),
            Err(EditError::IdExhausted)
        );
        assert!(document.redo());
        assert_eq!(document.object(u64::MAX).unwrap().id, u64::MAX);
    }

    #[test]
    fn environment_edits_are_bounded_atomic_and_undoable() {
        let mut document = Document::default();
        let earth = document.environment();
        assert_eq!(earth.gravity_m_s2, Point::new(0.0, -9.80665));
        let custom = PhysicsEnvironment {
            gravity_m_s2: Point::new(-MAX_GRAVITY_M_S2, MAX_GRAVITY_M_S2),
            linear_drag_per_s: MAX_LINEAR_DRAG_PER_S,
        };
        document.set_environment(custom).unwrap();
        assert!(document.undo());
        let before = document.clone();
        document.set_environment(earth).unwrap();
        for invalid in [f64::NAN, f64::INFINITY, 10_000.1, -10_000.1] {
            for gravity in [Point::new(invalid, 0.0), Point::new(0.0, invalid)] {
                assert_eq!(
                    document.set_environment(PhysicsEnvironment {
                        gravity_m_s2: gravity,
                        ..earth
                    }),
                    Err(EditError::InvalidEnvironment)
                );
            }
        }
        for invalid in [f64::NAN, f64::INFINITY, -0.01, 100.01] {
            assert_eq!(
                document.set_environment(PhysicsEnvironment {
                    linear_drag_per_s: invalid,
                    ..earth
                }),
                Err(EditError::InvalidEnvironment)
            );
        }
        assert_unchanged(&before, &document);
        assert!(document.redo());
        assert_eq!(document.environment(), custom);
    }

    #[test]
    fn link_validation_rejects_invalid_graphs_without_consuming_ids() {
        let mut document = Document::default();
        let a = document.add(ObjectKind::Anchor, Point::default()).unwrap();
        let b = document
            .add(ObjectKind::Anchor, Point::new(1.0, 0.0))
            .unwrap();
        let c = document.add(ObjectKind::Ball, Point::default()).unwrap();
        let d = document.add(ObjectKind::Box, Point::new(3.0, 4.0)).unwrap();
        let before = document.clone();
        assert_eq!(document.add_rod(a, a), Err(EditError::SelfLink));
        assert_eq!(document.add_rod(a, 999), Err(EditError::MissingObject(999)));
        assert_eq!(
            document.add_spring(999, a),
            Err(EditError::MissingObject(999))
        );
        assert_eq!(document.add_rod(a, b), Err(EditError::FixedEndpoints));
        assert_eq!(document.add_spring(a, b), Err(EditError::FixedEndpoints));
        assert_eq!(document.add_rod(a, c), Err(EditError::InvalidLinkLength));
        assert_eq!(document.add_spring(c, a), Err(EditError::InvalidLinkLength));
        assert_unchanged(&before, &document);
        assert_eq!(document.add_rod(a, d).unwrap(), 1);
        assert_eq!(document.links()[0].kind, LinkKind::Rod { length_m: 5.0 });
        let before = document.clone();
        assert_eq!(document.add_rod(d, a), Err(EditError::DuplicateLink));
        assert_eq!(document.add_spring(a, d), Err(EditError::DuplicateLink));
        assert_unchanged(&before, &document);
        assert_eq!(document.add_spring(c, d).unwrap(), 2);
        assert_eq!(
            document.links()[1].kind,
            LinkKind::Spring {
                rest_length_m: 5.0,
                stiffness_n_m: 20.0
            }
        );
    }

    #[test]
    fn move_reauthors_rods_and_preserves_spring_strain_atomically() {
        let mut document = Document::default();
        let centre = document.add(ObjectKind::Ball, Point::default()).unwrap();
        let rod_end = document
            .add(ObjectKind::Anchor, Point::new(2.0, 0.0))
            .unwrap();
        let spring_end = document
            .add(ObjectKind::Anchor, Point::new(0.0, 2.0))
            .unwrap();
        document.add_rod(centre, rod_end).unwrap();
        document.add_spring(centre, spring_end).unwrap();
        document.move_object(centre, Point::new(-1.0, 0.0)).unwrap();
        assert_eq!(document.links()[0].kind, LinkKind::Rod { length_m: 3.0 });
        assert_eq!(
            document.links()[1].kind,
            LinkKind::Spring {
                rest_length_m: 2.0,
                stiffness_n_m: 20.0
            }
        );
        document.undo();
        let before = document.clone();
        assert_eq!(
            document.move_object(centre, Point::new(2.0, 0.0)),
            Err(EditError::InvalidLinkLength)
        );
        assert_eq!(
            document.move_object(centre, Point::new(0.0, 2.0)),
            Err(EditError::InvalidLinkLength)
        );
        assert_unchanged(&before, &document);
        document.redo();
        assert_eq!(document.links()[0].kind, LinkKind::Rod { length_m: 3.0 });
    }

    #[test]
    fn remove_cascades_edges_and_undo_restores_complete_graph() {
        let mut document = Document::default();
        let bob = document.add_pendulum(Point::new(2.0, 3.0)).unwrap();
        let duplicate = document.duplicate(bob).unwrap();
        assert_eq!(
            document.links().len(),
            1,
            "single-body copy excludes external links"
        );
        document.add_spring(bob, duplicate).unwrap();
        let objects = document.objects().to_vec();
        let links = document.links().to_vec();
        document.remove(bob).unwrap();
        assert!(document.links().is_empty());
        assert!(document.undo());
        assert_eq!(document.objects(), objects);
        assert_eq!(document.links(), links);
        assert!(document.redo());
        assert!(document.links().is_empty());
        assert!(document.object(bob).is_none());
    }

    #[test]
    fn recipes_create_fresh_internal_endpoints_and_one_history_transaction() {
        let mut document = Document::default();
        let first = document.add_pendulum(Point::new(1.0, 2.0)).unwrap();
        assert_eq!(document.undo.len(), 1);
        let second = document.add_pendulum(Point::new(10.0, 20.0)).unwrap();
        let third = document.add_spring_pair(Point::new(100.0, 200.0)).unwrap();
        assert_eq!(document.objects().len(), 6);
        assert_eq!(document.links().len(), 3);
        assert_eq!(document.undo.len(), 3);
        for (index, bob) in [first, second, third].into_iter().enumerate() {
            let link = document.links()[index];
            assert_eq!(link.id, (index + 1) as u64);
            assert_eq!(link.a, (2 * index + 1) as u64);
            assert_eq!(link.b, bob);
            assert_eq!(document.object(link.a).unwrap().kind, ObjectKind::Anchor);
            assert_eq!(document.object(bob).unwrap().kind, ObjectKind::Ball);
        }
        let first_link = document.links()[0];
        assert!(
            (separation(
                document.object(first_link.a).unwrap().position,
                document.object(first).unwrap().position
            )
            .unwrap()
                - 3.0)
                .abs()
                < 1e-12
        );
        assert_eq!(
            document.object(third).unwrap().position,
            Point::new(100.0, 197.5)
        );
        assert_eq!(
            document.links()[2].kind,
            LinkKind::Spring {
                rest_length_m: 2.0,
                stiffness_n_m: 20.0
            }
        );
        let final_objects = document.objects().to_vec();
        let final_links = document.links().to_vec();
        assert!(document.undo());
        assert_eq!(document.objects().len(), 4);
        assert_eq!(document.links().len(), 2);
        assert!(document.redo());
        assert_eq!(document.objects(), final_objects);
        assert_eq!(document.links(), final_links);
    }

    #[test]
    fn compound_recipe_rejections_preserve_all_state_history_and_allocators() {
        let mut document = Document::default();
        document.add_pendulum(Point::default()).unwrap();
        document.undo();
        let before = document.clone();
        for origin in [Point::new(f64::NAN, 0.0), Point::new(0.0, -MAX_POSITION_M)] {
            assert_eq!(
                document.add_pendulum(origin),
                Err(EditError::InvalidPosition)
            );
            assert_eq!(
                document.add_spring_pair(origin),
                Err(EditError::InvalidPosition)
            );
        }
        assert_unchanged(&before, &document);
        document.next_id = Some(u64::MAX);
        let before = document.clone();
        assert_eq!(
            document.add_pendulum(Point::default()),
            Err(EditError::IdExhausted)
        );
        assert_eq!(
            document.add_spring_pair(Point::default()),
            Err(EditError::IdExhausted)
        );
        assert_unchanged(&before, &document);
        document.next_id = Some(10);
        document.next_link_id = None;
        let before = document.clone();
        assert_eq!(
            document.add_pendulum(Point::default()),
            Err(EditError::LinkIdExhausted)
        );
        assert_unchanged(&before, &document);
    }

    #[test]
    fn recipes_preflight_the_combined_object_budget() {
        let mut document = Document::default();
        for _ in 0..MAX_OBJECTS - 1 {
            document.add(ObjectKind::Ball, Point::default()).unwrap();
        }
        let before = document.clone();
        assert_eq!(
            document.add_pendulum(Point::default()),
            Err(EditError::TooManyObjects)
        );
        assert_eq!(
            document.add_spring_pair(Point::default()),
            Err(EditError::TooManyObjects)
        );
        assert_eq!(
            document.add_bounce_lab(Point::default()),
            Err(EditError::TooManyObjects)
        );
        assert_unchanged(&before, &document);
        document.add(ObjectKind::Ball, Point::default()).unwrap();
        assert_eq!(document.objects().len(), MAX_OBJECTS);
    }

    #[test]
    fn link_budget_and_graph_history_are_bounded_before_further_insertion() {
        let mut document = Document::default();
        for x in 0..24 {
            document
                .add(ObjectKind::Ball, Point::new(f64::from(x), 0.0))
                .unwrap();
        }
        let mut next_pair = None;
        'pairs: for a in 1..=24 {
            for b in a + 1..=24 {
                if document.links().len() == MAX_LINKS {
                    next_pair = Some((a, b));
                    break 'pairs;
                }
                document.add_rod(a, b).unwrap();
            }
        }
        assert_eq!(document.links().len(), MAX_LINKS);
        let (a, b) = next_pair.unwrap();
        let before = document.clone();
        assert_eq!(document.add_rod(a, b), Err(EditError::TooManyLinks));
        assert_eq!(document.add_spring(a, b), Err(EditError::TooManyLinks));
        assert_eq!(
            document.add_pendulum(Point::default()),
            Err(EditError::TooManyLinks)
        );
        assert_eq!(
            document.add_spring_pair(Point::default()),
            Err(EditError::TooManyLinks)
        );
        assert_unchanged(&before, &document);
        for _ in 0..HISTORY_LIMIT {
            assert!(document.undo());
            assert_eq!(document.undo.len() + document.redo.len(), HISTORY_LIMIT);
        }
        assert!(!document.undo());
        for _ in 0..HISTORY_LIMIT {
            assert!(document.redo());
            assert_eq!(document.undo.len() + document.redo.len(), HISTORY_LIMIT);
        }
        assert_eq!(document.links().len(), MAX_LINKS);
    }

    #[test]
    fn relationship_identifiers_never_reuse_undone_or_deleted_links() {
        let mut document = Document::default();
        let a = document.add(ObjectKind::Ball, Point::default()).unwrap();
        let b = document
            .add(ObjectKind::Ball, Point::new(1.0, 0.0))
            .unwrap();
        assert_eq!(document.add_rod(a, b).unwrap(), 1);
        document.undo();
        assert_eq!(document.add_spring(b, a).unwrap(), 2);
        document.remove(b).unwrap();
        let c = document
            .add(ObjectKind::Ball, Point::new(2.0, 0.0))
            .unwrap();
        document.next_link_id = Some(u64::MAX);
        assert_eq!(document.add_rod(a, c).unwrap(), u64::MAX);
        document.undo();
        let before = document.clone();
        assert_eq!(document.add_rod(a, c), Err(EditError::LinkIdExhausted));
        assert_unchanged(&before, &document);
        document.redo();
        assert_eq!(document.links()[0].id, u64::MAX);
    }

    #[test]
    fn compound_recipe_can_consume_last_body_and_relationship_identifiers() {
        let mut document = Document {
            next_id: Some(u64::MAX - 1),
            next_link_id: Some(u64::MAX),
            ..Document::default()
        };
        assert_eq!(document.add_pendulum(Point::default()).unwrap(), u64::MAX);
        assert_eq!(document.links()[0].id, u64::MAX);
        assert_eq!(document.next_id, None);
        assert_eq!(document.next_link_id, None);
        assert!(document.undo());
        assert!(document.objects().is_empty());
        assert!(document.links().is_empty());
        let before = document.clone();
        assert_eq!(
            document.add_pendulum(Point::default()),
            Err(EditError::IdExhausted)
        );
        assert_unchanged(&before, &document);
        assert!(document.redo());
        assert_eq!(document.links()[0].b, u64::MAX);
    }
}
