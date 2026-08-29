# Sim;Phys;Mechanics Composition Model

Status: **normative architecture contract for the active Mechanics vertical
slice**.

This document defines how independently authored pieces of a Mechanics scene
may be combined without creating a solver full of object-specific branches.
It is intentionally detailed because composition errors become data-model and
save-format errors; after projects and Custom Objects exist, those errors are
expensive to repair.

This is not a universal ontology for all sciences. It does not define how
chemistry, biology, mathematics, thermodynamics, electromagnetism, or a future
cross-domain Sandbox must work. Those systems may reuse a proven concept only
after their own vertical slices demonstrate that the concept has the same
meaning.

## 1. Why This Contract Exists

The central technical risk in Sim;X is the composition model:

> Objects built from different physical concepts must interact naturally,
> predictably, and without a new special case for every named machine.

Rendering can display a scene and a numerical method can advance one isolated
body. Neither answers these questions:

- What makes two scene elements compatible?
- What does a capability prove?
- Who may create or remove an interaction?
- How is a force connected to its target?
- How does a constraint refer to a body?
- Which relationships belong to a saved Custom Object?
- In what order do multiple effects run?
- How does the same initial state produce the same result again?

Without one explicit answer, different features invent incompatible answers.
One feature stores a pointer, another stores a string tag, another mutates a
body from an event callback, and the eventual Sandbox becomes accidental
behavior. This contract prevents that split.

## 2. Scope

### 2.1 In scope now

The active model covers enough Mechanics composition to build and verify the
first vertical slice:

- stable scene identities;
- canonical body state;
- narrowly derived Mechanics capabilities;
- persistent typed relationships;
- interaction rules and deterministic contributions;
- the `F = ma` update;
- the architectural shape of a distance constraint and pendulum;
- atomic world updates;
- post-commit phenomenon events;
- the save boundary required by a future Custom Object.

### 2.2 Explicitly outside the current implementation scope

The following must not be pre-built as supposedly generic infrastructure:

- universal capabilities shared by all four sciences;
- heat transfer, electrical contact, chemical bonds, cell behavior, or graph
  mathematics;
- a runtime plugin registry for arbitrary interaction rules;
- a universal Sandbox solver;
- full rigid-body collision and fracture;
- cross-platform bit-for-bit floating-point guarantees;
- Custom Object editing and migration UI;
- project persistence beyond what the current roadmap authorizes.

They are discussed only where the Mechanics design must leave a clean
extension boundary.

## 3. Non-Negotiable Principles

### 3.1 Named things are recipes, not solver types

`Pendulum`, `engine`, `lamp`, and `generator` are useful catalog names. They
are not fundamental physics types.

A pendulum is a composition such as:

- a fixed anchor;
- a dynamic body;
- a distance constraint between them;
- participation in the configured gravity field.

The Mechanics solver must not contain code such as
`if object.kind == Pendulum`. The catalog may know how to construct a
pendulum recipe; the solver knows only the components, capabilities,
relationships, and rules that the recipe creates.

### 3.2 Canonical data, capability, relationship, and behavior are different

- **Canonical data** stores the actual state of the world.
- **A capability** is a narrow, derived proof that some behavior may read or
  affect that data.
- **A relationship** records which participants are connected and with what
  persistent parameters.
- **An interaction rule** defines how compatible participants produce effects.

Combining these concepts into one untyped object bag is prohibited.

### 3.3 The domain owns truth

The UI, an editor tool, a project loader, and a future scripting layer may
submit an intent or command. Only the Mechanics domain may validate that
intent, create a bound interaction, and commit a physical state change.

UI compatibility previews are advisory. The domain repeats validation at the
commit boundary because the world may have changed since the preview.

### 3.4 Updates are deterministic and atomic

For the same supported build, platform, initial snapshot, fixed-step command
stream, and random seed, execution must use the same ordering and produce the
same observable result.

A step is committed in full or rejected in full. A failed calculation may not
leave half the bodies updated and half untouched.

### 3.5 Capabilities stay local to their domain

Mechanics capabilities belong to `domains::phys::mechanics`. They must not be
moved to `foundation` merely because another future domain might have a
similar word such as `position`, `energy`, or `connection`.

Shared foundation types are earned through demonstrated identical semantics,
not anticipated reuse.

### 3.6 Rendering never becomes canonical state

GPU handles, pixel coordinates, draw order, hover state, widget identity, and
Sim;Engine resources are presentation data. A relationship must remain valid
when nothing is rendered, and a Mechanics test must run without a window or
GPU.

## 4. Vocabulary

The following terms have one meaning throughout Mechanics code and docs.

### 4.1 Entity

An **entity** is a stable identity for one independently addressable scene
element. `EntityId` is opaque and never carries physical meaning.

An entity owns canonical components. It is not a subclass, a string type, or a
renderer node. Deleting an entity invokes relationship-integrity rules before
the deletion is committed.

### 4.2 Component

A **component** is typed canonical data attached to one entity. Examples for
the first slice are pose, linear velocity, mass properties, and mobility.

There must be one authoritative location for each fact. If mass is stored in a
mass component, a capability and a relationship may reference or read it but
must not cache a second authoritative mass.

### 4.3 Capability

A **capability** is a domain-defined projection proving that an entity can
participate in a particular role at a particular world revision.

It is not:

- a string such as `"force_receiver"`;
- an unchecked marker bit;
- an inheritance hierarchy;
- an `Any` value requiring downcasts;
- a duplicated component bundle;
- a promise that remains valid forever.

A capability query may fail with a structured reason. A successful projection
borrows or identifies canonical state and is re-derived when binding is
validated.

### 4.4 Endpoint

An **endpoint** identifies the precise role or attachment site through which a
relationship addresses a participant. It is expressed in domain coordinates,
not pixels.

For example, a constraint endpoint can be:

- an entity plus a stable local attachment key; or
- an explicit fixed world anchor, if the relationship type permits it.

The endpoint model must be decided and versioned before persistent constraints
are serialized. Raw memory pointers and array indices are forbidden endpoint
identities.

### 4.5 Relationship

A **relationship** is a persistent, typed edge in the scene graph. It records
an intentional connection such as a distance constraint. It has stable
identity, typed parameters, typed endpoints, enabled state, and schema
version.

A relationship contains no solver cache that is required to understand the
saved scene. Ephemeral warm-start or broad-phase data may exist separately and
must be reconstructible.

### 4.6 Transient binding

A **transient binding** is a validated interaction that exists only for a step
or detection interval, such as a future collision contact. It is not silently
promoted to a saved relationship.

### 4.7 Interaction rule

An **interaction rule** is domain behavior with explicit:

- participant requirements;
- compatibility and binding validation;
- input state;
- output contributions;
- ordering phase;
- units and reference frame;
- failure modes;
- serialization implications, if persistent relationships use it.

A rule does not directly call another entity and does not commit partial
world mutations while it is being evaluated.

### 4.8 Bound interaction

A **bound interaction** is the validated association between one rule and its
specific participants for a specific revision or step. It exists only after
requirements and invariants pass.

Binding can be persistent-derived, transient-detected, or command-created. A
bound interaction is not proof that its inputs will remain valid at the next
step; it is revalidated according to its lifecycle policy.

### 4.9 Contribution

A **contribution** is a proposed typed effect produced by a rule during a
step. Force is the first example. Contributions are collected before the
canonical state changes, then ordered, reduced, solved, and integrated by the
domain schedule.

### 4.10 Event

An **event** is a post-commit observation that a physical phenomenon or domain
transition occurred. It supports the optional detailed log and presentation.

An event is not a command and may not mutate the step that emitted it.

### 4.11 Recipe or catalog entry

A **recipe** is a named constructor for a graph of entities, components, and
relationships. Built-in objects and future Custom Objects use this concept.
Names and categories are product metadata; physical meaning remains in the
graph.

## 5. The Mechanics Scene Is a Typed Graph

The canonical Mechanics scene is modeled as:

- nodes: stable entities with typed components;
- persistent edges: typed relationships;
- ephemeral edges: validated transient bindings;
- rules: deterministic transformations from a pre-step snapshot to typed
  contributions and then a candidate next state.

This graph distinction matters. A rope-like distance constraint is not merely
a visual line and is not hidden inside one body. It is an edge that refers to
both endpoints and owns its constraint parameters.

Fields may later be represented as entities, scene configuration, or
domain-owned providers. The first implementation must choose one explicit
representation for gravity and document it; it must not add an untyped global
property bag.

## 6. Canonical Mechanics State for the First Slice

Names below describe required concepts, not a command to create these exact
Rust structs before the implementation specification is approved.

### 6.1 Stable identity

Every entity and persistent relationship has an opaque stable ID. IDs:

- are unique within a scene;
- survive save and load;
- are never derived from vector position or render order;
- are remapped transactionally when a saved composition is instantiated;
- have a deterministic allocation policy within a deterministic replay.

### 6.2 Pose

The first slice needs a world-space 2D position in meters. Orientation is added
when an approved feature requires angular mechanics; it must use typed radians
and the same explicit reference-frame convention everywhere.

True physical scale is canonical. A 1 km pendulum has a 1,000 m length. Camera
zoom changes presentation only; no conditional visual-scale substitute is
stored in the Mechanics world.

### 6.3 Linear velocity

Linear velocity is stored in meters per second in the agreed 2D coordinate
frame. Renderer Y direction must not silently redefine the physical frame.
The adapter performs any screen-space conversion.

### 6.4 Mass properties

For the `F = ma` slice, dynamic bodies have finite, strictly positive mass in
kilograms.

Zero, negative, NaN, and infinite mass are rejected at the domain boundary.
Fixed bodies use explicit mobility, not infinite mass. A future exotic model
must introduce an explicit scientific rule instead of weakening this basic
invariant.

### 6.5 Mobility

Mobility distinguishes at least:

- `Dynamic`: responds according to mass and accumulated physical effects;
- `Fixed`: not integrated by ordinary forces.

The product's Free mode is a tool behavior, not negative or missing mass. A
View-mode hand tool may request a direct manipulation governed by an explicit
tool rule. It must not corrupt the physical mass model.

### 6.6 Future components

Angular state, material, shape/collider, fracture state, charge, temperature,
and other concepts are not silently folded into a generic property map. Each
is added with its own invariants, ownership, versioning, and rule needs.

## 7. Initial Capability Projections

The first vertical slice should define the smallest projections it actually
uses.

### 7.1 Force receiver

`ForceReceiver` proves that an entity currently has the canonical state needed
by ordinary translational force integration:

- valid pose;
- valid linear velocity;
- valid finite positive mass;
- dynamic mobility.

It does not own copies of those values. A fixed entity can still be a valid
constraint endpoint but is not an ordinary force receiver.

### 7.2 Constraint endpoint

`ConstraintEndpoint` proves that a relationship can resolve a stable physical
attachment from its endpoint descriptor. The projection provides the data
required by the selected constraint solver without exposing presentation
state.

Dynamic and fixed participants may both satisfy this role. Their behavior
differs through canonical mobility, not through a special `PendulumAnchor`
kind.

### 7.3 Capability design test

Before adding a capability, answer all of these:

1. Which domain owns the term?
2. Which exact rule consumes it?
3. Which canonical components prove it?
4. Can it be derived without duplicating state?
5. What structured reasons explain failure?
6. At what boundary is it revalidated?
7. Is the name describing behavior rather than a catalog object?

If no current rule consumes the capability, do not add it yet.

## 8. Compatibility Is a Structured Decision

Compatibility is not a permanent boolean property of two entity types.

A candidate interaction is compatible only when:

1. an explicit Mechanics rule exists for the proposed role;
2. every participant provides the required capability at validation time;
3. endpoint and reference-frame requirements are satisfied;
4. parameters have correct dimensions and finite values;
5. domain invariants and relationship-integrity rules pass;
6. the rule is allowed in the current mode and schedule phase.

The result must distinguish success from structured rejection. Useful rejection
reasons include:

- missing entity;
- missing capability;
- fixed body cannot receive ordinary force;
- invalid or stale endpoint;
- dimension mismatch;
- non-finite parameter;
- unsupported self-relationship;
- duplicate exclusive relationship;
- participant pending deletion;
- rule unavailable in this Mechanics version.

These reasons are domain data. The UI may translate them into a tooltip,
Scientific Warning, or disabled action, but it must not invent them.

### 8.1 Preview versus commit

Drag-and-drop may request a pure compatibility preview for smooth feedback.
That preview receives a read-only world view and has no side effects.

On drop, a command is submitted. The Mechanics domain checks the current world
revision again and either commits the complete relationship or returns a
structured rejection. Preview success never bypasses commit validation.

## 9. Who May Create an Interaction

Interaction origins are explicit:

- an editor or View-mode tool submits a command;
- a project loader proposes persisted relationships;
- a built-in or custom recipe proposes a graph transaction;
- a domain detector creates transient candidates;
- a configured field provider offers a typed contribution source.

Only `MechanicsWorld` through the approved domain schedule may bind these to
canonical participants and change state.

The following are forbidden:

- UI code inserting directly into component storage;
- presentation callbacks mutating velocity;
- one entity calling methods on another entity;
- event subscribers changing the already-running step;
- a loader accepting an invalid relationship because it once existed in a
  valid older file.

## 10. Persistent Relationship Contract

Each persistent relationship must contain or resolve:

- `RelationshipId`;
- a typed Mechanics relationship kind or rule ID;
- schema version;
- typed endpoint descriptors;
- typed parameters in canonical units;
- enabled/disabled state if the product needs it;
- stable source metadata only when required for editing or migration.

It must not contain:

- pointers or Rust references;
- collection indices;
- GPU or widget handles;
- closures;
- rule execution order inferred from creation order;
- hidden unit conventions;
- redundant canonical body state.

### 10.1 Lifecycle integrity

Relationship commands are transactions:

- create validates all endpoints and parameters before insertion;
- modify validates the complete proposed replacement;
- disable preserves the relationship but removes its physical contribution;
- delete removes the edge and any reconstructible solver cache;
- entity deletion either removes dependent edges in the same explicit
  transaction or rejects with dependency details.

No dangling relationship may appear in a committed world snapshot.

### 10.2 Distance constraint shape

The first persistent relationship is expected to be a distance constraint. Its
minimum semantic data is:

- endpoint A;
- endpoint B;
- target distance in meters;
- solver-specific parameters only after their scientific meaning is defined;
- enabled state.

Whether it is rigid, compliant, breakable, or rope-like is not a renderer
choice. Those are different domain semantics and must not be represented by
one ambiguous stiffness number without a written model.

## 11. Force as the First Contribution

Ordinary force is a typed per-step contribution, not necessarily an entity.
The first form needs:

- target `EntityId`;
- source identity or deterministic source key for diagnostics and ordering;
- force vector in newtons;
- step identity;
- application point only when angular mechanics is supported.

For the translational `F = ma` slice, all approved forces act through the
center of mass. Off-center force and torque must not be simulated incorrectly;
they remain unavailable until angular state and integration are specified.

Rules emit force contributions into a step-local buffer. They do not mutate
velocity directly. Before reduction, each contribution is checked for:

- existing target;
- current `ForceReceiver` capability;
- finite vector components;
- correct dimensional type;
- valid phase and step identity.

Contributions are ordered by an explicit stable key and reduced per target in
that order. Iteration order of a hash map is never a physical rule.

## 12. Deterministic Mechanics Schedule

Each fixed Mechanics step follows a documented phase sequence. The initial
contract is:

1. Take commands assigned to this `StepIndex` in deterministic command order.
2. Validate graph transactions against the current committed world.
3. Apply valid structural transactions to a candidate pre-step graph.
4. Freeze the read-only pre-step snapshot used by rules.
5. Resolve and validate persistent relationship bindings.
6. Detect any authorized transient interaction candidates.
7. Evaluate fields, tools, and interaction rules into typed contributions.
8. Stable-sort and reduce contributions.
9. Solve approved constraints and contacts in explicit stable order.
10. Integrate a complete candidate next state using fixed `dt`.
11. Validate all resulting domain and computational invariants.
12. Atomically commit the candidate world or reject the whole step.
13. Produce an immutable snapshot and post-commit phenomenon events.

If later evidence requires a phase change, the schedule version changes and
the decision is documented. A feature may not insert an invisible callback
between phases.

### 12.1 Time rules

- Simulation advances by fixed `dt` steps.
- Pause produces no physics steps.
- Slow motion and fast motion change how fixed steps map to wall time; they do
  not silently change the integration formula.
- Wall-clock time is never a rule input.
- Rewind is outside Mechanics scope.
- Large frame delays are handled by an explicit catch-up policy in the app,
  not by one unbounded physics step.

### 12.2 Ordering rules

Deterministic order comes from semantic stable keys, not incidental storage:

- command sequence assigned by the app boundary;
- rule phase and stable rule key;
- relationship ID;
- source key;
- target entity ID;
- explicit solver iteration number.

Parallel evaluation is allowed only when merging results yields the identical
specified order. A performance optimization cannot change physical results.

### 12.3 Randomness

The Mechanics slice does not need randomness. If a future approved rule does,
it receives an explicit seeded deterministic stream whose consumption policy
is part of that rule. It never calls ambient or wall-clock-seeded randomness.

### 12.4 Floating-point guarantee

The initial reproducibility target is identical results for the same supported
binary, platform, initial state, command stream, and seed. Stable algorithms,
finite checks, fixed iteration counts, and deterministic reduction are still
mandatory.

Cross-platform bit-for-bit equality is not promised until numeric backends,
compiler flags, transcendental functions, and serialization are audited. The
absence of that promise is not permission to use nondeterministic ordering.

## 13. `F = ma` Integration Contract

For one dynamic body and the net force accumulated during a fixed step:

```text
acceleration = net_force / mass
next_velocity = velocity + acceleration * dt
next_position = position + next_velocity * dt
```

This is semi-implicit (symplectic) Euler for the first slice. The choice is
explicit so code, tests, events, and documentation agree.

Required properties:

- canonical calculation uses `f64`;
- units are typed at the domain/foundation boundary;
- `dt` is finite and strictly positive;
- mass is finite and strictly positive;
- force, acceleration, velocity, and position remain finite;
- no silent clamp converts invalid state into plausible state;
- failure rejects the candidate step atomically.

Fixed bodies are not passed through the dynamic formula. Free-mode direct
manipulation uses a separately named command/tool rule and cannot masquerade
as `F = ma`.

## 14. Constraint Solver Boundary

A pendulum requires a constraint solver, but the exact algorithm is not chosen
by this document. Before implementation, a focused specification must define:

- position-based, velocity-based, or impulse formulation;
- how fixed and dynamic endpoint inverse masses participate;
- correction and tolerance semantics in SI units;
- a fixed deterministic iteration count;
- stable ordering of multiple constraints;
- handling of zero-length or coincident endpoints;
- failure behavior for non-finite candidate state;
- whether compliance or breakage exists in v0.

Early exit based on platform-sensitive convergence may make replay diverge.
The v0 solver should prefer a fixed iteration count unless a deterministic
alternative is proven.

## 15. Atomicity, Errors, and Scientific Warnings

Sim;X deliberately allows scientifically unusual scenes, such as changing a
scene-wide physical constant. It must still distinguish unusual science from
invalid computation.

### 15.1 Scientific Warning

A Scientific Warning reports a permitted model choice that violates the
normal real-world expectation. The operation may proceed after the product's
configured confirmation behavior.

Examples might include nonstandard gravity or a deliberately fictional global
constant after that model exists.

### 15.2 Domain rejection

A domain rejection means the requested relationship or command has no valid
meaning in the current model. Missing endpoints and unsupported roles are
examples. The committed world remains unchanged.

### 15.3 Computational failure

NaN, infinity, overflow into non-finite state, invalid `dt`, and violated
solver invariants are computational failures. They are never reclassified as
creative scientific freedom.

### 15.4 Transaction rule

Structural edits, recipe instantiation, relationship replacement, definition
migration, and physics steps build and validate a candidate state. A failure
commits none of that candidate state and returns structured diagnostics.

## 16. Events and the Detailed Log

Events are emitted only after a successful commit. A useful Mechanics event
contains:

- event kind;
- `StepIndex` and simulation time;
- involved entity and relationship IDs;
- stable rule/source identity;
- typed payload in canonical units;
- severity or classification when meaningful.

Possible first-slice events include a force being accepted, a body state being
integrated, a relationship being created or removed, and a constraint becoming
invalid. Event volume must be controllable; a hidden log panel must not force
expensive human-readable string construction every step.

The domain emits typed facts. Presentation localizes and formats lines such as
`Force N applied to object X`. Logs are derived output and are not replay
input, canonical physics, or an event-driven mutation channel.

## 17. Built-In Objects Are Graph Recipes

The product may ship convenient built-ins. A built-in is trusted content, not
a privileged solver branch.

### 17.1 Pendulum worked example

A pendulum recipe can propose this transaction:

1. Create a fixed anchor entity with a valid constraint endpoint.
2. Create a dynamic bob entity with pose, velocity, mass, and endpoint state.
3. Create a distance relationship with target length in meters.
4. Place both in a scene whose gravity model is enabled.
5. Validate the entire graph transaction.
6. Commit all three scene elements or none.

At runtime:

- gravity emits a force contribution for the dynamic bob;
- force reduction and integration propose motion;
- the distance constraint rule enforces the approved relationship;
- events describe the resulting phenomena;
- the renderer consumes only the committed snapshot.

Removing the catalog label `Pendulum` must not change the physics.

### 17.2 Future machine example

An internal-combustion engine driving a generator and lamp is a useful future
composition test, not a current Mechanics design target. Eventually it may be
a graph of conversion behaviors and natural physical contacts. No solver
should ask whether an entity is an `Engine` or `Lamp`.

This example does not authorize generic `InputPort` and `OutputPort` objects.
Per the product decision, natural properties and relationships of the composed
objects provide their interactions. Any future interface concept must be
derived from real domain semantics, not imposed only to make a block diagram.

## 18. Custom Objects Save a Subgraph

A future Custom Object is a saved composition, not a new fundamental physics
class. Its definition must be able to preserve:

- definition ID and revision;
- format and domain schema versions;
- local entity IDs;
- typed components and their schema versions;
- persistent relationships and endpoint descriptors;
- nested composition information according to the chosen storage policy;
- name, category, tags, preview, and authoring metadata;
- explicit migration metadata when a schema requires it.

It must not serialize:

- GPU resources;
- UI selection or hover state;
- raw pointers;
- transient contact bindings;
- reconstructible solver caches;
- unversioned opaque Rust memory;
- human-readable log strings as physical state.

### 18.1 Instantiation

Instantiation is one graph transaction:

1. Parse and version-check the definition.
2. Validate every component and relationship schema.
3. Allocate a deterministic new scene ID for each local entity and
   relationship.
4. Remap all internal endpoint references through that complete map.
5. Apply the requested scene transform in physical coordinates.
6. Validate the full candidate subgraph against the current Mechanics world.
7. Commit all entities and relationships or commit none.

Instances do not share mutable component state merely because they came from
the same definition.

### 18.2 Natural interfaces, not manually declared ports

If a saved object contains a fuel-consuming device, an electrically
conductive part, or a rotating shaft in future domain models, its usable
interfaces come from its internal canonical properties, exposed geometry, and
relationships. The creator should not have to draw arbitrary IN/OUT ports
that duplicate facts the object already owns.

Mechanics v0 does not invent abstractions for those future domains. It only
ensures saved endpoint identities are strong enough that real relationships
can be preserved.

### 18.3 Editing a used definition

The accepted product direction allows a definition to be opened and edited
only after explicit confirmation that existing uses may be replaced.

The architecture must support a transactional migration:

- validate the new definition first;
- update instances in the currently open scene immediately if the user
  confirms;
- detect an updated definition when another project is later opened;
- preserve instance placement and external relationships only where stable
  endpoint identity proves that preservation is valid;
- reject or explain connections whose endpoints disappeared;
- replace all affected instances in a transaction or keep the old revision;
- never partially migrate half of an instance graph.

Runtime state such as instantaneous velocity should default to reset on
definition replacement unless a later, explicit migration rule says it is
safe to preserve. Canonical authoring properties and runtime state must be
distinguishable for this reason.

### 18.4 Unresolved storage policy

Nested Custom Objects require one deliberate choice:

- **expanded snapshot:** copy the complete resolved subgraph into the new
  definition; simple and stable, but loses live ancestry; or
- **versioned references:** retain references to nested definitions; supports
  propagation but makes migration and missing dependencies harder.

The v0 recommendation is an expanded, self-contained snapshot plus provenance
metadata, because the user saves the selected result as a new object. This is
not final until the Custom Object persistence slice begins.

## 19. Rule Registration and Evolution

Mechanics v0 uses an explicit compile-time schedule and typed rules. It does
not need a universal runtime registry.

New official rules are first-party Rust rule-pack code compiled with Sim;X;
they are not dynamically loaded native plugins. `EXTENSIBILITY.md` defines
that repository-wide policy.

Adding an interaction rule requires a written mini-contract containing:

1. scientific meaning and governing equation;
2. required participant capabilities;
3. compatibility checks and rejection reasons;
4. persistent versus transient lifecycle;
5. canonical units and coordinate frame;
6. schedule phase and stable ordering key;
7. exact contribution or candidate-state output;
8. numerical invariants and failure behavior;
9. events and diagnostics;
10. serialization/version impact;
11. deterministic tests.

If the rule cannot answer these, it is not ready to enter the solver.

Other Physics subdomains must initially own their own components and rules.
Only when a concrete Phys Sandbox slice exists should explicit bridge rules be
designed. A bridge is still a typed rule with two known sides, not a global
event bus or string-based capability marketplace.

## 20. Module Ownership

The intended ownership is:

```text
foundation
  stable general-purpose IDs, typed SI quantities, common error primitives

domains/phys/mechanics
  canonical world, Mechanics components, capability projections,
  relationships, rule schedule, integration, events, snapshots

app
  lifecycle, command timing, project/editor coordination, fixed-step driving

presentation/ui
  user intent, compatibility previews, selection, inspector, warnings

presentation/sim_engine
  conversion of immutable presentation snapshots into Sim;Engine visuals

future persistence boundary
  versioned scene/subgraph records using domain-owned codecs and migrations
```

Mechanics must not import presentation or audio. Presentation must not contain
physical equations. The app may coordinate a step but must not implement
`F = ma` or decide relationship compatibility.

Do not create empty abstractions or placeholder modules solely to match this
diagram. Add a module when an approved slice gives it real responsibility.

## 21. Prohibited Designs

The following patterns are rejected unless this contract is deliberately
revised with evidence:

- `ObjectKind::Pendulum`, `ObjectKind::Engine`, or catalog-name branches in a
  physical solver;
- capabilities stored as strings or unchecked flags;
- an `Any`/downcast component bag as the main domain model;
- a universal base object with unrelated optional fields;
- relationship state stored only in UI connectors;
- renderer transforms used as physical position;
- components directly mutating components of other entities;
- same-step mutation from an event subscriber;
- update order inherited from hash iteration or draw order;
- hidden global mutable registries of interaction callbacks;
- silent NaN replacement, silent clamping, or partial step commits;
- using infinite mass to mean fixed;
- treating a Custom Object as a flattened sprite with lost relationships;
- promoting Mechanics vocabulary to all sciences before a second real use;
- designing every future domain before the Mechanics slice works.

## 22. Verification Matrix

Composition is accepted through headless domain tests, not visual impression.

### 22.1 Capability and compatibility tests

- A valid dynamic body projects `ForceReceiver`.
- A fixed body rejects that projection with the expected structured reason.
- Missing or invalid mass rejects it without mutation.
- Valid fixed and dynamic bodies can project appropriate constraint endpoints.
- A stale preview is rejected when commit-time state no longer matches.

### 22.2 `F = ma` tests

- Zero net force preserves velocity and advances position consistently.
- Equal force on double mass produces half acceleration.
- Multiple force contributions reduce in the specified stable order.
- Invalid `dt`, mass, or non-finite force rejects the full step.
- A fixed body does not enter ordinary force integration.

### 22.3 Relationship tests

- Creation with two valid endpoints commits exactly one relationship.
- Missing endpoint rejects the entire transaction.
- Deleting an entity follows the documented dependency policy and leaves no
  dangling edge.
- Disabled relationships emit no physical contribution.
- Save/load round-trip retains stable semantic data and canonical units.

### 22.4 Determinism tests

- Replaying identical initial snapshots and commands yields identical world
  snapshots and events on the supported target.
- Permuting internal insertion order does not change semantic results.
- Hash storage order cannot change contribution reduction.
- A fixed solver iteration count and relationship order are observable in
  golden tests.
- A deterministic world digest can identify replay divergence without making
  the digest itself canonical state.

### 22.5 Atomicity tests

- Failure on the last entity leaves every entity at the previous committed
  state.
- Recipe instantiation with one invalid relationship creates nothing.
- Definition migration failure preserves the complete old revision.

### 22.6 Custom Object contract tests, when authorized

- Internal IDs remap without collisions.
- Every internal relationship points to the new instance IDs.
- Two instances share no mutable physical state.
- Unsupported schema versions produce a structured error.
- Canonical serialized ordering makes equivalent graphs stable to compare.
- Removed external endpoints produce explicit migration diagnostics.

### 22.7 Architecture tests

- Mechanics compiles and tests without Sim;Engine, winit, rodio, or a GPU.
- Presentation contains no canonical physical state.
- Only the approved adapter imports `sim_engine`.
- No rule depends on UI strings, object category, or draw order.

## 23. Implementation Gates

Composition work proceeds only through these gates.

### Gate 0: foundational numerical contract

Approve stable IDs, typed SI quantities needed by the slice, fixed-step time,
finite-value validation, and structured domain errors.

### Gate 1: minimal Mechanics world

Specify canonical pose, velocity, mass, mobility, snapshots, commands, and
capability projection. No pendulum or generalized registry yet.

### Gate 2: one-body `F = ma`

Implement one or more typed force contributions, deterministic reduction,
semi-implicit Euler, atomic commit, typed events, and headless tests.

### Gate 3: first persistent relationship

Choose endpoint identity and the distance-constraint numerical model. Add
relationship transactions and integrity tests.

### Gate 4: pendulum recipe

Construct a pendulum only from the approved generic Mechanics pieces. Prove
that changing or removing its catalog name does not affect simulation.

### Gate 5: editor integration

Map drag-and-drop, inspector changes, tools, and View controls to commands and
read-only previews. Render immutable snapshots through Sim;Engine.

### Gate 6: saved composition

Only when the graph and relationship model is stable, specify the versioned
Custom Object format, ID remapping, definition editing, and migrations.

Passing a later gate may not be used to avoid an unresolved earlier contract.

## 24. Decisions That Must Remain Explicitly Open

The following are not to be guessed during implementation:

1. Exact distance-constraint solver and its scientific parameters.
2. Stable endpoint-key representation and endpoint migration rules.
3. Whether orientation/angular mechanics enters before or with the pendulum.
4. Representation and ownership of the first gravity model.
5. Collision scope and ordering relative to constraints.
6. Cross-platform determinism tier.
7. Nested Custom Object snapshot versus referenced-definition storage.
8. Canonical persistence encoding and schema migration mechanism.
9. Policy for removing an entity with dependent relationships: explicit
   cascade command or rejection requiring the caller to include the edges.
10. Exact event-volume and log-retention policy.

When one of these becomes necessary, update this document or a linked focused
specification before writing the implementation.

## 25. Confirmed Decision Record

The following decisions are already accepted and should not repeatedly reopen
without new evidence:

- Current implementation is only `Sim;Phys -> Phys;Mechanics`.
- The composition model is a primary architectural risk.
- Named machines and experiments are compositions, not solver types.
- Compatibility is determined by typed rules and capabilities.
- Capabilities derive from canonical domain state and are not string tags.
- Relationships are first-class typed graph edges.
- The domain, not the UI, owns validation and state mutation.
- Updates are deterministic in explicit fixed-step phases and commit atomically.
- Events and detailed logs observe committed state; they do not drive the same
  step.
- A Custom Object saves a composition graph with relationships.
- Custom Object interfaces arise from real properties and relationships; the
  product does not require arbitrary diagram-style IN/OUT ports.
- True scale is canonical: a 1 km pendulum is physically 1 km.
- Fixed bodies use explicit mobility; ordinary dynamic mass stays finite and
  positive.
- Rewind is not part of Mechanics.
- Unusual scientific choices may be allowed with warnings, but invalid
  computation is rejected.
- Future domains may have radically different UI and domain models; Mechanics
  abstractions do not automatically become universal Sim;X abstractions.

## 26. Short Review Checklist

Before approving any new Mechanics feature, ask:

- Is this a new physical primitive, or only a named recipe?
- Where is the single canonical fact stored?
- Which capability proves participation, and which rule consumes it?
- Is the connection a persistent relationship or a transient binding?
- Who validates it at commit time?
- What contribution does the rule produce, in which units and phase?
- Is ordering explicit and deterministic?
- Can failure leave partial state?
- Which typed event describes the result?
- Can the graph survive save/load and ID remapping?
- Does any UI, renderer, string tag, or catalog name leak into the physics?
- Is this really required by the current Mechanics vertical slice?

If an answer is unclear, the feature is not ready to be encoded as Rust.
