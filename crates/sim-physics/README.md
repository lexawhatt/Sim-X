# sim-physics: first shared physical kernel

This crate supplies actual headless mechanics, not a decorative preview or a
general-purpose claim to simulate all of physics. The current model contains
2D translational masses, uniform configurable gravity, center-applied external
forces, massless bilateral rigid rods, Hooke springs, optional linear drag,
and optional frictionless finite-circle/box collisions. Explicit angular bodies
add body-local spring attachments, torque, planar inertia, and shared-point
contact impulses. There is no friction, angular damping, contact heating,
off-center rod constraint, material model, heat transport, fluid flow, circuit,
quantum model or fracture. Existing `new`/`with_colliders` constructors preserve
the original point/locked-orientation model; angular response is opt-in through
`with_rigid_bodies`. This is a bounded planar model, not all rigid-body physics.

See [ARCHITECTURE.md](ARCHITECTURE.md) for module ownership and extension rules.
The library has no dependencies beyond Rust's standard library.

```sh
cargo test -p sim-physics
cargo clippy -p sim-physics --all-targets -- -D warnings
cargo run -p sim-physics --example pendulum
```

## Composition and mutable settings

Bodies have stable IDs, physical positions and velocities in SI units, finite
positive mass, and `Dynamic` or `Fixed` mobility. A rod or spring refers to two
body IDs. The solver never sees names such as pendulum, motor or lamp. A pendulum
is just a fixed body, dynamic body, rod and shared gravitational field. Reference
recipes in `scenarios` are used by tests and the headless example.

`World::new` validates the complete submitted graph. Duplicate IDs, self-links,
missing endpoints, duplicate rods, fixed-to-fixed links, invalid initial rod
lengths or radial velocities are errors. Fixed bodies must have zero velocity;
infinite mass is never the representation of an anchor. Springs may be parallel
independent relationships; coincident spring endpoints are singular and rejected.

`set_settings` changes the scene-wide gravity vector and drag rate atomically.
Changing gravity affects every dynamic body on the next step without moving
current positions or resetting velocities. It increments `settings_revision`
and clears stale telemetry. No-op settings writes do neither. Changing a field
also changes potential energy relative to the chosen coordinate-origin datum;
that change is a parameter intervention, not unexplained solver energy loss.

Editor state is not a live physics world. The application builds a separate Run
instance and decides when to call `step`. Wall time, pause, speed selection,
rendering, history, camera transforms and user gestures never enter this crate.

`World::new` and `BodyDesc` remain the point-mass interface. `with_colliders`
additionally accepts at most one `ColliderDesc` per existing body, with a circle
radius or box half-extents/fixed angle and restitution. Bodies without a collider
do not collide; there is no implicit floor. `colliders()` and `Snapshot.colliders`
preserve these descriptions in body-ID order. Collider geometry is immutable
during a Run, like the graph topology. A rod does not silently filter collisions
between its endpoints. An anchor may simply remain a fixed point without a shape.

`with_rigid_bodies` additionally accepts up to 128 `RotationDesc` records, one
per existing body. State contains the world body angle, angular velocity and
positive planar moment of inertia. `RotationDesc::uniform` explicitly uses a
uniform disk (`I=m*r^2/2`) or rectangular lamina
(`I=m*(half_width^2+half_height^2)/3`), not a 3D sphere. An explicit inertia may
instead be supplied. A box collider's own angle is then a fixed **local offset**
added to the body's evolving angle. Without a rotation record its orientation
remains locked. Fixed bodies have zero linear and angular velocity; their
recorded loads are balanced by an external support. `rotations()`, `rotation(id)`
and `Snapshot.rotations` expose immutable, canonically ordered angular state.

`AttachedSpring` adds `local_a_m` and `local_b_m` to the spring relationship.
Any nonzero endpoint offset requires that body's explicit rotation component,
including on a fixed shape. A zero offset may attach to an ordinary point anchor.
Attachment sites are coordinates in a body frame, not arbitrary shape ports:
the kernel does not require a site to lie on the collision surface. The editor
chooses such sites according to its authoring policy. Rods remain center-only.

## Determinism and bounded work

The same binary/platform, initial graph and step input produce identical states
and observations. Body IDs and link IDs define canonical storage and solve
order. Cross-platform bit-identical transcendental arithmetic is not promised.

Budgets are hard boundaries, not suggested scene sizes:

- 128 bodies and 256 relationships per world.
- 1,024 external force contributions per step.
- 1..=128 fixed sweeps for each RATTLE phase, default 32.
- At most 65,536 rod visits per step at maximum relationships and sweeps.
- Exactly one bounded body report and link report per committed body/link.
  No per-iteration event stream or retained unbounded history exists.

The rod-visit bound above describes the unchanged point-mass path. Finite-shape
worlds additionally bound 128 colliders, 8,128 unordered candidate pairs, and 256
simultaneous active contact pairs per position sweep/final report. No unbounded
contact history is stored. Active-contact discovery checks the cap before adding
a 257th contact record. A contact correction can discover another pair on the
next stable-ID scan; every final pair is checked again before commit.

Angular worlds retain those limits and allow at most two manifold points per
active pair, hence 512 points and at most 65,536 point impulse updates per maximum
128-sweep phase. Two-point faces use a constant-size unilateral 2x2 active-set
solve; nearly singular faces use sequential updates. Position cleanup rebuilds
geometry before sequential point corrections rather than using a stale gap.

For `I <= 128` sweeps, a conservative full contact-path bound is `(3*I+5)*8128`
pair visits (at most 3,161,792), `3*I*256` rod visits (98,304), and `I*256` normal
impulse updates (32,768). Pair geometry/sweeps contain only fixed-size operations;
temporary body arrays contain at most 128 items. Those are safety limits, not
a frame-rate guarantee for a maximally dense scene. No-rod worlds skip redundant
RATTLE guard scans, and an initial no-penetration contact scan skips position
cleanup. The usual point-mass path does not enter finite-shape geometry at all.

The angular branch additionally allows at most `2*I*256` manifold rebuilds
inside position scans and bounded rotational safety subdivision: 128 interval
visits per candidate pair, depth 32, and 4,096 visits shared by an entire guard
call. There are at most `2*I+2` such calls per step, hence at most 1,056,768
interval visits at maximum sweeps, not that limit multiplied by every pair.
Exhaustion rejects the candidate; no unbounded recursive stack or contact list
exists. These are conservative worst-work bounds, not realtime promises.

Input slice lengths are checked before traversal or allocation. Forces must
have existing dynamic targets, finite components, and unique `(target, source)`
pairs. Duplicate discovery precedes canonical target validation. All counters,
body state, time and reports commit together; any error leaves all unchanged.

External force components use a fixed 2,176-bit integer superaccumulator:
binary64 values are exact integer multiples of `2^-1074`, positive and negative
magnitudes are summed separately, then subtracted and rounded once to nearest,
ties-to-even. The internal reduction limit is 2,048 values; each current use is
below it. Exact zero is positive zero. Input ordering and source-ID renaming
cannot change the reduced physical force. Avoidable intermediate overflow is
not an error: `MAX + MAX - MAX` reduces to `MAX`. A final non-finite rounded
result is rejected. This accuracy promise applies to each reduction, not to
all subsequent floating-point equation evaluations as an exact real number.

## Numerical operating envelope

All canonical scalars are binary64. Public raw descriptions are validated at
the world boundary. Values outside these ranges are rejected, never clamped:

| Quantity | Supported range |
| --- | --- |
| Position component | `-1e9..=1e9` m |
| Velocity component | `-1e6..=1e6` m/s |
| Positive body mass | `1e-6..=1e12` kg |
| Gravity component | `-1e6..=1e6` m/s^2 |
| Linear drag rate | `0..=1000` 1/s |
| Rod/rest length | `1e-6..=1e8` m |
| Spring stiffness | `1e-9..=1e12` N/m |
| Fixed step | `1e-6..=1/30` s; default `1/240` s |
| Absolute position tolerance | `1e-12..=1e-4` m; default `1e-9` m |
| Absolute radial-velocity tolerance | `1e-12..=1e-4` m/s; default `1e-9` m/s |
| Relative rod-length tolerance | `1e-12..=1e-6`; default `1e-10` |
| Circle radius / box half-extent | `1e-4..=1e6` m |
| Body or local box angle | `-1e6..=1e6` radians |
| Angular velocity | `-1e6..=1e6` radians/s |
| Positive planar inertia | `1e-15..=1e24` kg*m^2 |
| Local spring attachment component | `-1e6..=1e6` m |
| Material normal restitution | `0..=1` |

The range table is necessary, not sufficient: a tiny displacement at a large
absolute position can still be unrepresentable. Each nonzero physical kick and
drift must remain nonzero after multiplication and must change its accumulated
Cartesian component. A positive time step must strictly increase elapsed time.
Underflow, absorbed increments, non-finite intermediates, and counter exhaustion
reject the whole step. Converged constraint corrections below the stated residual
tolerance are not physical drift and need not alter a bit at every sweep.

The constructor enforces the conservative graph-wide spring frequency heuristic
`dt * sqrt(sum(k * (inverse_mass_a + inverse_mass_b
+ |local_a|^2*inverse_inertia_a + |local_b|^2*inverse_inertia_b))) <= 0.5`.
Zero local offsets reduce this to the previous point-mass bound. It prevents
obviously under-resolved linear spring modes, including many parallel springs.
It is not a guarantee for every compressed geometry or nonlinear trajectory.
The solver does not silently change time step, soften rods, or add damping to
rescue difficult scenes. Rod convergence failures are explicit. The three-link
chain regression uses 96 sweeps; default 32 is not a promise that every bounded
constraint graph converges at every permitted step and tolerance.

An off-center spring endpoint may rotate at most 0.05 radians during a free
drift; otherwise `AngularMotionTooLarge` rejects rather than sampling an
under-resolved hook path. Potentially interacting rotating collider pairs also
have the swept limits below. A free rotor without a noncentral spring or nearby
collision is not subject to this increment cap, only the canonical angle/rate
ranges and representability checks. Angles are never silently wrapped.

## Step schedule and equations

The following is the smooth, no-collider path; finite contacts extend it as
specified in the next section rather than replacing the existing equations.

1. Validate the force envelope and next counters/time, exactly reduce forces.
2. Evaluate gravity and Hooke forces at old positions.
3. Apply half of the exact uniform drag velocity decay.
4. Apply half a conservative velocity kick and drift candidate positions.
5. Solve rod positions using RATTLE's old-position constraint directions;
   include each position correction divided by `dt` in half-step velocity.
6. Reevaluate conservative forces at new positions and apply the second kick.
7. Project rod-relative velocities tangent to the new constraint directions.
8. Apply the second drag half-step, validate residuals and numerical state.
9. Build finite bounded observations, then atomically commit everything.

For a rod, `d0 = old_b - old_a`, `d = candidate_b - candidate_a`,
`W = inverse_mass_a + inverse_mass_b`, and target length `L`:

```text
C = (dot(d,d) - L*L) / 2
lambda = C / (W * dot(d,d0))
position_a += inverse_mass_a * lambda * d0
position_b -= inverse_mass_b * lambda * d0
half_velocity_a += inverse_mass_a * lambda * d0 / dt
half_velocity_b -= inverse_mass_b * lambda * d0 / dt

beta = dot(d, velocity_b - velocity_a) / (W * dot(d,d))
velocity_a += inverse_mass_a * beta * d
velocity_b -= inverse_mass_b * beta * d
```

The old direction stays fixed throughout position sweeps. Non-positive or
non-finite denominators reject the candidate. Final length residual must not
exceed `absolute + relative * L`; final radial relative speed must be within
the velocity tolerance. Each configured sweep executes in stable link order;
already-converged per-link corrections below 1% of final tolerance are skipped
without early termination of the fixed sweep count.

The distinction between position constraints and the second velocity-constraint
stage follows [LAMMPS's SHAKE/RATTLE documentation](https://docs.lammps.org/fix_shake.html)
and [GROMACS's constraint algorithm reference](https://manual.gromacs.org/current/reference-manual/algorithms/constraint-algorithms.html).
This code is a small separately implemented point-mass model, not a binding to
either package and not a claim to inherit their validated application ranges.

Hooke forces use `F_on_a = k * (distance - rest_length) * direction / distance`,
with the opposite force on B; spring energy is `k * extension^2 / 2`. Uniform
drag is explicitly `dv/dt = -gamma*v`, applied by `exp(-gamma*dt/2)` on each side
of the conservative step. It is not aerodynamic drag or a material model.

### Body-local spring and angular schedule

For an attachment `a_local`, its world lever is `r=R(theta)*a_local`, position
`q=x+r`, and velocity `v_hook=v+omega*(-r.y,r.x)`. Spring length, extension and
radial velocity use the two hook positions/velocities, not the body centers.
The force above applies at those points, giving `torque_a=cross(r_a,F_on_a)`
and `torque_b=cross(r_b,-F_on_a)`. These moments need not be opposites; isolated
orbital **plus spin** angular momentum, not spin alone, is the conserved quantity.

The angular path performs the same conservative half-kick/drift/half-kick with
`delta_omega=torque*dt/(2*I)` and `delta_theta=omega_half*dt`. Angular moments
are recomputed at the new positions and orientations. Center RATTLE rods use
the unchanged center-only solver. Numerical contact position/angular cleanup
has no `delta_pose/dt` velocity feedback. The second translational drag half
occurs **before** the coupled rod/contact velocity projection: applying it
afterward without spin damping could close a solved `v+omega cross r` contact.
Drag never secretly damps spin. Empty-rotation constructors retain the original
step order and reference trajectories.

## Finite-body contact contract

Collision geometry is centre-based: circles and oriented rectangles.
Circle/rectangle contact uses the closest rectangle feature; rectangle/rectangle
uses the four current separating face axes. Equal-depth ties use stable feature
order. The legacy path solves one translational normal per pair. Angular worlds
instead clip the incident rectangle face against the reference face and retain
up to two shared contact points; circle contacts retain one. Mass remains an
independent physical input, never inferred
from visual area or a fictitious density.

Initial pairs with a dynamic endpoint may touch, but penetration greater than
`position_tolerance_m + relative_tolerance * min(feature_a, feature_b)` rejects
the complete constructor. `feature` is a radius or the smaller half-extent.
Fixed-fixed overlap is allowed as stationary authored geometry and is not a
contact. Invalid dimensions, missing bodies and duplicate body colliders are
structured errors; no constructor silently separates shapes.

The finite-body step first evaluates the same conservative kicks and RATTLE
position phase. Penetrations then receive inverse-mass-weighted **position-only**
cleanup, interleaved with geometric rod projection. This cleanup is not divided
by `dt` or injected into velocity. Forces at corrected positions are reevaluated
before the second conservative kick. Contact normal impulses and rod velocity
constraints then alternate in stable pair/link order for the configured sweep
count. Final gap, rod-length, rod-tangency and unilateral normal-velocity
residuals must all pass. A failure rolls back positions, velocities, time,
counters, observations and collider state together.

For a normal from A to B, `vn = dot(normal, velocity_b-velocity_a)` and
`W = inverse_mass_a + inverse_mass_b`, the accumulated normal impulse is:

```text
lambda_new = max(0, lambda_old + (target_normal_speed - vn) / W)
delta_impulse = normal * (lambda_new - lambda_old)
velocity_a -= inverse_mass_a * delta_impulse
velocity_b += inverse_mass_b * delta_impulse
```

For angular contacts, `vn` includes both hook velocities at the shared point and
the effective inverse mass becomes `W + cross(r_a,n)^2/I_a + cross(r_b,n)^2/I_b`.
The impulse changes each body's spin by its lever moment divided by inertia.
Both bodies use the **same world point**, preserving total angular momentum
of the impulse pair. A two-point face solves both unilateral normal impulses
together with a bounded 2x2 active-set solve to avoid artificial spin from
sequentially supporting a symmetric flat face at only one corner. No tangential
impulse, friction, rolling resistance or hidden rotation lock is added.

It is the accumulated impulse that is clamped, not each individual update.
The rebound target is captured once before the coupled velocity iterations;
reapplying restitution on every sweep would inject energy. A pair's material
coefficient is `max(restitution_a, restitution_b)`. The actual numerical
coefficient is zero for established resting contacts and for approach speeds
below the public `RESTITUTION_SPEED_THRESHOLD_M_S = 0.2`; equality uses the
material coefficient. A touching pair already approaching in the old state is
an impact, not automatically a resting contact. An established resting pair
receiving gravity during this step is held rather than bounced repeatedly.
Both material and effective coefficients appear in contact telemetry. This
explicit low-speed settling policy is numerical regularization, not a claim
that a material physically loses its elasticity below 0.2 m/s.

The iterative contact/rod solve is accepted only if each connected contact/rod
island's kinetic energy does not increase by more than
`relative_tolerance * initial_island_kinetic_energy + f64::EPSILON` joules.
This bound cannot be relaxed by a distant, unrelated high-energy body. Multiple
simultaneous impacts remain an iterative model; incompatible restitution and
constraint targets reject instead of silently creating energy. Redundant
supports may have nonunique load sharing even with unique body motion, so their
reported rod/contact reactions are solver estimates, not uniquely identified
experimental readings without a later compliance/material model.

### Swept safety, not continuous impact resolution

No time-of-impact stepping, adaptive substepping or hidden speed clamp is
implemented. Every potentially interacting swept-AABB pair must have relative
translation at most one quarter of its smallest shape feature during the
checked segment. The full drift, rod correction batches, contact correction
batches and old-to-final displacement are checked. Fast isolated bodies and
far-separated swept bounds do not acquire an artificial speed cap.

The movement cap alone cannot prevent a glancing contact from crossing a thin
overlap region. Therefore separated endpoint pairs also receive a continuous
translation test: closest relative-segment approach for circles, swept SAT
intervals for boxes, and exact segment-to-local-box distance for circle/box.
Crossing a strict interior beyond the configured tolerance while ending
separated returns `SweptCollisionUnsupported`; excessive candidate-pair motion
returns `CollisionMotionTooLarge`. Even small grazing passes are checked;
genuine near misses outside the tolerance remain legal. These are conservative
atomic rejections, not a claim to resolve arbitrary high-speed trajectories.
Reduce time step/speed/force when a scene exceeds this supported envelope.

When a box rotates, fixed-axis translational sweep tests are insufficient.
Potentially interacting pairs additionally limit each angle increment to
0.05 radians and relative translation plus `radius*abs(delta_angle)` surface
travel to one quarter of the smallest feature. For clear final endpoints,
bounded interval subdivision certifies separation using a midpoint separating
gap and a conservative bound on surface motion over the interval. Strict
interior crossing rejects; unresolved intervals or exhausted subdivision work
return `RotationalSweepUnsupported`. Initial penetration cleanup and final
contact go through the bounded discrete solver. Circle spin does not change
circle geometry. This is conservative swept rejection, not time-of-impact
resolution; a difficult near miss may require a smaller step.

Shared contact points, angular effective mass and clipped faces follow the
mechanical formulation described in [Catto's Sequential Impulses notes](https://box2d.org/files/ErinCatto_SequentialImpulses_GDC2006.pdf).

Contact impulse accumulation, split position correction and low-speed settling
are discussed in the primary [Box2D solver article](https://box2d.org/posts/2024/02/solver2d/).
The [Box2D simulation reference](https://box2d.org/documentation/md_simulation.html)
also distinguishes fixed rotation, restitution mixing and swept collision work.
This crate implements its own narrower model; it does not embed Box2D, copy its
full feature set, or inherit its performance/validation claims.

## Telemetry for future nerd-mode

Every successful step returns typed observations regardless of UI visibility:

- Body position, velocity, actual average acceleration `(v_new-v_old)/dt`.
- Gravity and submitted external force, trapezoid-average spring force.
- Sum of rod impulses divided by `dt`, and drag impulse divided by `dt`.
- Contact impulses, their step-average forces, and separate position cleanup.
- Net modeled applied force, kinetic and origin-relative gravitational energy.
- Link distance, extension/rod error, radial speed, endpoint force and energy.
- Total kinetic, gravitational and elastic energy; drag energy removed this step.
- Physical time, successful step count and settings revision.
- Optional body angle, angular velocity, actual average angular acceleration,
  planar inertia, spring/contact torque, spin energy and numerical angle cleanup.

`StepReport.contacts` contains at most 256 stable-ID contact records. Opposite
endpoint impulses sum to zero, including loads recorded on fixed supports.
Angular records also retain each shared point and both endpoint moments; pair
effective restitution is the maximum of the reported point coefficients.
Body `kinetic_energy_j` and world `kinetic_j` include spin, with separate
`rotational_kinetic_j` and per-rotation energies available as a breakdown.
`contact_projection_energy_change_j` is signed gravitational/elastic potential
change from numerical cleanup, and `contact_solve_kinetic_change_j` is signed
kinetic change during the coupled contact/rod velocity solve. Neither is called
physical heat or material dissipation; contact stabilization can exchange a small
amount of numerical potential and kinetic energy while holding a body at rest.
These deltas use local difference-of-squares/position differences and exact
reductions. Subtracting two huge world energy totals would erase a small but
representable contact observation beside an unrelated high-energy spectator.

Rod impulses include both `lambda*d0/dt` and `beta*d` contributions. The reported
rod force is a **step average**, not instantaneous analytic tension. Fixed bodies
record the applied spring/rod/contact loads but remain still; the environment supplies
the balancing support reaction. Gravity is integrated only on dynamic masses.
These choices must stay visible to any classroom overlay or plot.

Energy products use a separate fixed 256-bit significand calculation for at most
four finite factors, with powers of two retained separately and one final
round-to-nearest, ties-to-even conversion. This prevents intermediate `v*v`
underflow from erasing a representable mass-scaled energy. Kinetic energy uses
the robust `hypot` speed before the mass-scaled product; this is not a promise
that all preceding vector arithmetic is exact. Drag energy uses the stable
difference-of-squares identity for the actual old/new velocity components, not
subtraction of prematurely underflowed squares. Gravitational and elastic
products use the same checked path. A nonzero energy product that genuinely
rounds to zero is `PrecisionLoss(Telemetry)` and rolls the step back, rather
than claiming a successful zero-energy observation. Consequently extreme tiny
scenes may be rejected on diagnostic representability even when a position
increment alone would still fit in binary64.

The numerical audit regression is a `1e12` kg point mass moving at `1e-162` m/s:
its squared velocity rounds to zero alone, but its kinetic energy is a
representable approximately `5e-313` J. With drag rate `1/s`, both its remaining
kinetic energy and approximately `4.149e-315` J first-step drag loss remain
observable and balance within binary64 subnormal rounding limits.

## Scientific checks

Headless tests compare free fall against its analytic trajectory, verify F=ma,
force lifetime, exact reduction, momentum balance, Hooke oscillation, exact drag
decay and drag-energy accounting. A pendulum's period matches the small-angle
reference, scales with gravity, and is mass-independent. A 40-second test checks
bounded energy error and its reduction when the fixed step is halved. A linked
chain checks residuals with its explicit higher iteration budget.

Period and oscillator reference relationships are described by OpenStax's
[pendulum chapter](https://openstax.org/books/university-physics-volume-1/pages/15-4-pendulums)
and [simple harmonic motion chapter](https://openstax.org/books/university-physics-volume-1/pages/15-1-simple-harmonic-motion).
Small-angle tests intentionally use a 0.02-radian initial angle; they do not
pretend the small-angle formula is exact at large amplitudes.

Contract regressions cover invalid graphs, limits before input traversal,
source renaming/permutations, stable constructor ordering, deterministic replay,
failed last-body updates, representability loss, counter exhaustion, setting
replacement and a seeded finite-input analytic property sweep. Passing these
tests validates this model slice, not future thermodynamics or circuits.

Contact regressions independently cover unequal-mass restitution and momentum,
normal reflection from rotated fixed boxes without tangential friction, locked
box impacts, resting support and stacks, simultaneous impacts, rod/wall coupling,
strict grazing swept rejection with near-miss acceptance, deterministic caller
ordering, and unchanged separated-collider RATTLE trajectories. Contract tests
also cover invalid/overlapping geometry, both collider and active-contact limits,
atomic failed steps, explicit low-speed restitution, absence of a hidden floor,
momentum/telemetry balance with drag, and the unrelated-energy cancellation cases.

Angular checks independently cover disk/lamina inertia, local/world transforms,
torque signs and inertia scaling, rotated zero-extension springs, hook velocity,
fixed-support moments, isolated linear and total angular momentum, second-order
energy convergence, analytical off-center impact, flat/tall face support,
spin-preserving frictionless circle impacts, canonical replay and atomic invalid
angular states. Rotational glancing sweeps include a 0.00002-radian corner pass
which is below the travel cap but crosses an obstacle between clear endpoints.
