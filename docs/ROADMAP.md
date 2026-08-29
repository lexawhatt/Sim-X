# Sim;X Roadmap

The broader product brainstorm, including Domain Sandbox and Custom Objects, is
recorded in `Brainstorm.md`. It remains a design record and does not move ahead
of the current Mechanics vertical slice.

The Physics-only default Workspace and optional UI customization direction is
recorded in `UI.md`. Other domains do not inherit the Physics editor design.

The normative Mechanics composition contract is recorded in
`COMPOSITION.md`. It defines the gates that prevent named objects, UI state,
or incidental update order from leaking into the solver.

`EXTENSIBILITY.md` replaces the earlier Lua plugin direction. The active path
is Custom Objects plus first-party Rust rule packs; no external executable
extension runtime belongs to this roadmap yet.

## Current Product Priority

The first vertical slice is **Sim;Phys -> Phys;Mechanics: Newton's second
law**.

The slice starts with a body whose mass and applied force are controlled by the
user. The domain computes acceleration from `F = m * a`, advances motion with
an explicitly documented integrator, and exposes a bounded renderer-neutral
snapshot. A pendulum is the first editor composition built on those mechanics
contracts, not a separate UI-owned simulation.

Other domains and Phys subdomains remain visible in the product shell but are
not implemented before this slice is complete.

## Product Navigation

The navigation hierarchy is:

```text
Sim;X -> Sim;Phys -> Phys;Mechanics -> Mechanics editor
```

Sim;Phys exposes seven subdomains:

1. Mechanics
2. Thermodynamics
3. Waves & Optics
4. Electromagnetism
5. Relativity
6. Fluid Dynamics
7. Sandbox

Every top-level domain reserves its own `Sandbox` subdomain. A sandbox may
compose only compatible capabilities from its owning domain. There is no
implicit global sandbox that bypasses domain boundaries.

## Work Order

### 1. Dark Full-Screen UI Shell - Complete

- run in a borderless full-screen native window;
- use a dark palette only;
- render the complete UI through Sim;Engine scenes;
- animate hover state and title transitions at both navigation levels;
- expose all seven Sim;Phys subdomains while keeping Mechanics active;
- open a non-monolithic Mechanics editor with build palette, canvas, inspector,
  reset, and back navigation;
- keep UI policy in `src/presentation/ui` and all `sim_engine` imports in
  `src/presentation/sim_engine`.

The editor currently supports visual pendulum placement. It clearly marks the
simulation as disabled until the mechanics domain step exists.

### 2. Composition Contract - Current Architecture Gate

- keep `COMPOSITION.md` normative for the Mechanics slice;
- resolve Gate 0 types before implementing the Mechanics world;
- treat entities, canonical components, capabilities, relationships,
  interaction rules, contributions, and events as different concepts;
- reject named-object branches such as `Pendulum` in the solver;
- record every still-open numerical or persistence choice before code depends
  on it.

### 3. Foundation Contracts - Next

- specify and implement typed `Seconds`, `Kilograms`, `Newtons`,
  `MetersPerSecond`, and `MetersPerSecondSquared` values;
- add validated finite-value construction and checked arithmetic boundaries;
- define the constants registry/API version even though basic `F = m * a`
  reads no fundamental constant;
- define a validated positive finite simulation step;
- test invalid, NaN, infinite, zero, and negative inputs.

### 4. Sim;Phys;Mechanics Domain

- create body identity and canonical mechanics state;
- represent net force independently from visual arrows;
- compute acceleration as `force / mass`;
- define the integration method explicitly in the module specification;
- make each step validate first and mutate state atomically;
- test zero force, force direction, mass scaling, invalid mass, extreme finite
  values, and deterministic stepping.

### 5. Renderer-Neutral Mechanics Snapshot

- expose position, velocity, acceleration, force direction, and body shape in
  Sim;X types;
- keep domain calculations in typed units and the selected domain precision;
- bound the number and size of visual items;
- ensure snapshot production does not advance or mutate the simulation.

### 6. Connect Mechanics to the Sim;Engine View

- convert checked Sim;X values to finite renderer `f32` visual state;
- replace editor placeholders with the mechanics read model;
- render the body, force vector, trajectory, pendulum, and reference marks;
- use visual tweening only for presentation smoothing;
- preserve canonical mechanics state across skipped frames and GPU recovery;
- surface renderer status and dropped visual data in diagnostics.

The single-window composition path is already established: Sim;Engine owns all
drawing while the Sim;X adapter builds its scenes.

### 7. Mechanics Controls and Feedback

- add force and mass controls with units;
- show derived acceleration as read-only domain output;
- add play, pause, slow-motion, fast-motion, and reset actions;
- make reset restore the documented baseline state;
- display validation and clamp outcomes without crashing or hiding them;
- keep UI events as typed intents rather than direct domain mutation.

### 8. Pendulum Composition

- define gravity and constraint capabilities through mechanics contracts;
- compose a pendulum from a body, anchor, constraint, and applicable forces;
- keep editor objects as application commands and domain identities;
- ensure the renderer only consumes immutable visual snapshots.

### 9. Vertical-Slice Completion Gate

The slice is complete when a user can:

1. launch Sim;X full-screen and choose Sim;Phys;
2. open Phys;Mechanics;
3. create or select a mechanics object;
4. change mass and force;
5. observe acceleration and motion consistent with `F = m * a`;
6. pause, slow down, speed up, and reset;
7. enter invalid or extreme values without NaN, infinity, partial mutation, or
   an application crash;
8. resize or temporarily lose rendering without changing canonical results.

Required verification includes unit tests for domain behavior, boundary tests
for invalid data, architecture checks for forbidden imports, strict Clippy,
formatting, and a visual smoke test on the supported Linux path.
