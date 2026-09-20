# sim-physics ownership

This is an independent standard-library-only Rust library. It cannot depend on
Sim;X, Sim;Logic, Sim;Engine, an editor, a window, or a renderer. The crate root
forbids unsafe code. Public exports are deliberate integration contracts;
implementation helpers remain crate-private.

```text
Sim;X authoring/session adapter
          |
          v
    core::World ----> mechanics candidate pipeline ----> numeric
          |             forces -> integration
          |                         constraints <---- collision coupling
          |                                           geometry / sweep guards
          |                         rigid angular path + local spring sites
          |             telemetry <- validated candidate
          v
   immutable Snapshot / StepReport -> presentation

reference scenarios -> the same World API (no special solver paths)
```

`World` orchestrates validation, candidate calculation and atomic commit. It
does not implement the equations or format UI messages. Mechanics functions
receive slices and settings, not a mutable `World`. Internal `BoundLink` array
indices are reconstructible caches; canonical edges store stable endpoint IDs.

## Files and responsibilities

```text
src/
  lib.rs                    deliberate public reexports and boundary policy
  core/
    identity.rs             nonzero body/link/source identities
    settings.rs             scene parameters and fixed solver policy
    error.rs                structured transaction and numerical errors
    validation.rs           graph and settings compatibility
    world.rs                owned state, snapshots, lifecycle, atomic commit
    world_tests.rs          private counter/elapsed-time boundary regressions
  collision/
    model.rs                optional finite shapes, binding, pair/report limits
    geometry.rs             circles, closest box features, locked-box SAT normals
    sweep.rs                conservative motion caps and swept-miss rejection
    solve.rs                split position cleanup and coupled contact/rod impulses
    accounting.rs           local energy deltas and independent-island energy guard
  rigid/
    model.rs                optional angular state and explicit planar inertia
    pose.rs                 local attachment and collider-axis transforms
    integration.rs          angular Verlet and contact-safe translational drag order
    contacts.rs             split angular cleanup and coupled contact/rod impulses
    manifold.rs             shared circle points and clipped two-point box faces
    normal_block.rs         bounded two-point unilateral active-set solve
    sweep.rs                bounded rotational separation certification/rejection
    telemetry.rs            angular observations and fixed-support moments
  mechanics/
    model.rs                translational masses, invariants, rods, springs, forces
    forces.rs               external/Hooke forces and exact reduction
    integration.rs          bounded Verlet step and symmetric drag splitting
    constraints.rs          position and velocity RATTLE solves
    energy.rs               range-safe energy and drag-work observations
    telemetry.rs            runtime observations and energy accounting
  numeric/
    vector.rs               renderer-neutral binary64 vector arithmetic
    checked.rs              checked progress and reduction boundaries
    exact_sum.rs            bounded exact accumulator and rounding tests
    product.rs              bounded exact products, one final binary64 rounding
  scenarios.rs              small ordinary reference composition recipes
tests/
  analytic.rs               trajectories, periods, conservation, convergence
  contracts.rs              invalid input, work limits, replay and atomicity
  telemetry.rs              tiny representable energies and diagnostic atomicity
  contacts.rs               shape validation, contact budgets, loads and exclusions
  contact_audit.rs           independent analytic contacts, swept safety, coupling
  attachments.rs             angular API budgets, travel and stable face regressions
  attachments_audit.rs       independent torque, angular conservation and sweep tests
  support/mod.rs            integration-test fixture construction only
examples/
  pendulum.rs               no-window executable using the shared recipe
```

## Identity and authority

Only `World::new`/`with_colliders`/`with_rigid_bodies`, `World::step`, and `World::set_settings` establish or change
physical state. Reads return immutable borrows or owned copies. IDs are caller
supplied, nonzero, unique in a world, and support `u64::MAX`. This slice has no
body deletion or creation after construction, so ID reuse and stale-slot access
cannot occur. There is no slot arena requiring a generation counter yet.

Future dynamic graph transactions must explicitly adopt never-reused scene IDs
or checked generations; adding an index allocator without a stale-reference
policy is not permitted. A Custom Object will be a versioned graph recipe with
ID remapping, not a new solver type or shared mutable runtime instance.

`ColliderDesc` is an optional, separately validated geometry component. It does
not add dummy radius/temperature/material fields to every `BodyDesc`. A body
without a collider retains the old point model. `BoundCollider` caches body
indices and constant box axes; descriptions and stable IDs, not these caches,
are canonical snapshot data. Fixed-fixed intersections are stationary geometry.

`RotationDesc` is another explicit optional component, never a shape-name switch.
It contains angle, angular velocity and planar inertia. Missing records preserve
legacy locked orientation, while `with_rigid_bodies` activates a separate angular
pipeline and composes body angle with each collider's fixed local axes. Local
spring offsets belong to `AttachedSpring`, not an editor-only drawing record.
The `mechanics` force calculation consumes these transforms to produce both
forces and moments. Rods continue using center-only constraints. Neither shape
nor inertia implies materials, density, friction or a hidden environmental field.

The integrator calls the contact branch only when finite geometry exists. The
contact solver calls the same rod projection routines, with explicit separation
between smooth RATTLE velocity feedback and contact-only geometric cleanup.
Final contact and rod residuals must both validate before World commits.
Collision shape geometry does not receive a mutable World, UI shape or renderer.
Energy differences are computed locally in `collision/accounting`, not by
subtracting large unrelated world totals or labelling correction work as heat.

For angular bodies, `rigid/manifold` supplies at most two shared physical contact
points; `normal_block` solves the two-point unilateral active set without a
dynamic matrix. `rigid/contacts` handles the coupling while preserving separate
pose cleanup and physical impulses. It never adds pose correction divided by
time to angular velocity. The rotational sweep guard has both per-pair and
aggregate interval-work limits, rejecting an uncertified path atomically.
Body/global kinetic diagnostics include spin; per-rotation energy is a breakdown,
not an additional energy to sum a second time.

## Extension boundary

Do not add optional temperature/charge/material fields to every point mass.
When a thermal or electrical slice is authorized, create its focused sibling
module with canonical state, typed relationships, equations, limits and tests.
Then implement an explicit coupling rule with an energy/charge/mass balance.
Do not create empty future directories, string capability registries, global
event buses, fake IN/OUT ports, or switches on catalog names.

The planned heat -> steam -> shaft -> generator -> wire -> lamp experiment is
a cross-physics acceptance goal, not an implemented scenario. Each new coupling
must state its approximation level and operating range; one visual animation
must never stand in for a conservation law.
