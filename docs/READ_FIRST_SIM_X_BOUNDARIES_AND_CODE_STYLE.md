# Read First: Sim;X Boundaries and Code Style

This file is mandatory context before writing or changing Sim;X code. It is the
short operational contract for the repository. `Structure.md` explains the
same architecture in more detail. Any work on Phys;Mechanics entities, forces,
constraints, interaction rules, events, persistence, or simulation stepping
must also follow `COMPOSITION.md`.

## Product Boundary

Sim;X is a modular simulation application with four independent domains:

- Sim;phys - physics;
- Sim;math - mathematics;
- Sim;biol - biology;
- Sim;chem - chemistry.

Navigation keeps domain and subdomain names explicit, for example
`Sim;X -> Sim;Phys -> Phys;Mechanics`. Every top-level domain reserves its own
`Sandbox` subdomain; there is no boundary-free global sandbox.

The semicolon is a product name convention. Rust modules and filesystem names
use ASCII `snake_case`, such as `phys`, `math`, `biol`, and `chem`.

Sim;X owns:

- domain meaning, formulas, constants, units, entities, and invariants;
- simulation state and stepping;
- domain and simulation selection;
- UI policy and user-editable parameters;
- composition of compatible effects;
- Custom Object schemas, built-in recipes, and first-party rule-pack policy;
- conversion of domain state into bounded visual snapshots.

Sim;X is a modular monolith. It runs in one process, but its modules must not
collapse into one dependency graph where every subsystem can access every
other subsystem.

## Sim;Engine Boundary

Sim;X integrates `sim-engine` 0.1.x as a visualization library. The published
0.1.0 contract is pre-1.0, requires Rust 1.90, enables `wgpu` by default, and
release-gates Linux with Vulkan.

Sim;Engine owns:

- cameras, projections, picking conversions, clipping, and visual tweening;
- validated 2D scenes and visual styles;
- prepared scenes, dynamic visual meshes, particle fields, and scalar fields;
- render targets, composition, trails, diagnostics, and GPU recovery;
- retained stereometry meshes, transforms, depth, and display edges.

Sim;Engine does not own:

- physical particles, fields, bodies, forces, collisions, or integrators;
- mathematical meaning or domain units;
- simulation time, stepping, scheduling, or canonical state;
- Sim;X entities, navigation policy, UI policy, or extension policy.

Only `src/presentation/sim_engine/` may import `sim_engine`. Domain and
foundation modules must not expose `sim_engine` types in fields, signatures,
events, errors, or tests.

Only `src/presentation/audio/` may import `rodio` or own an OS audio device.
Audio is non-canonical presentation state: playback failure never blocks
navigation or changes simulation results.

The adapter must preserve these distinctions:

- simulation positions and values use Sim;X units and normally `f64`;
- the adapter performs checked conversion to renderer `f32` values;
- `sim_engine::Vec2` is a visual world value, not a physical position type;
- `ParticleInstance2d` is a visual instance, not a Sim;phys particle;
- `ScalarField` is a visual upload snapshot, not canonical domain storage;
- `Tween` changes presentation only and never advances simulation state;
- renderer recovery rebuilds visual resources without resetting domain state.

A normal frame flows in one direction:

```text
domain state -> immutable visual snapshot -> Sim;Engine adapter -> renderer
renderer status/metrics -> diagnostics -> application/UI
```

Scene building or rendering must never call a domain step or mutate canonical
simulation state.

## Module Boundaries

The allowed dependency direction is:

```text
main -> app
app -> foundation + domains + presentation
domains -> foundation
presentation -> foundation + domain read models
presentation/sim_engine -> sim-engine
```

Additional rules:

1. `foundation` contains only genuinely domain-neutral contracts. It is not a
   dumping ground.
2. A domain must not import another domain. Cross-domain composition belongs
   in `app` and must use explicit public contracts.
3. A domain must not import `app`, `presentation`, `wgpu`, a window
   library, or a UI framework.
4. `presentation` reads snapshots and emits user intents. It does not contain
   formulas or mutate domain state directly.
5. Custom Objects are validated saved compositions, not executable plugins.
   New official scientific behavior is added as first-party Rust rule packs
   compiled with Sim;X and governed by `EXTENSIBILITY.md`.
6. There is no active external executable extension runtime or public native
   Rust plugin ABI. Do not prepare domain APIs for Lua, WASM, or dynamic Rust
   plugins without an approved concrete use case and specification.
7. New shared abstractions stay in the first real consumer until a second real
   consumer proves that the concept and invariants are actually shared.
8. Do not add broad modules named `common`, `utils`, `helpers`, `types`, or
   `misc`. Name a module after one responsibility.

## Domain Rules

1. Fundamental constants come from the versioned Sim;X constants registry.
   Never embed real-world values inside formulas.
2. SI is the default calculation system. Public domain APIs use typed units or
   explicitly document every input and output unit.
3. Validate `dt` and every user, imported, or persisted value at the nearest
   boundary.
4. NaN, infinity, derived overflow, negative mass/radius, and invalid domain
   ranges must produce an explicit error or documented clamp outcome.
5. A failed update must not partially mutate canonical state.
6. Curated simulations select stable entities and effects. A domain sandbox may
   compose compatible effects inside that domain. There is no global sandbox
   that automatically mixes all four domains.
7. Sim;math may use the retained 3D stereometry path where 2D would lose the
   mathematical meaning. This exception does not make other domains 3D by
   default.

## Code Style

1. Comments explain intent, invariants, and tradeoffs, not obvious mechanics.
2. Every public API has `///` documentation describing behavior, units,
   invariants, ranges, side effects, and failure behavior where relevant.
3. Prefer full words in names. Established domain abbreviations such as `dt`
   are allowed when they are the clearest conventional term.
4. Fallible library and domain operations return `Result` or a documented
   outcome enum. Do not use `unwrap()` or `expect()` outside tests and bounded
   executable examples.
5. Keep one responsibility per module. Split a subsystem before files become a
   mixed collection of unrelated concepts.
6. Do not introduce generic frameworks for hypothetical consumers. Add the
   smallest abstraction required by current specifications.
7. Make coordinate spaces and units explicit in types, names, and docs.
8. Use `rustfmt` defaults and keep Clippy warning-free.
9. Code identifiers, comments, doc comments, errors, module names, and
   implementation-facing specifications are written in English.
10. Use ASCII punctuation in code and engineering documentation.
11. Keep domain arithmetic at the precision selected by Sim;X. Lossy renderer
    conversion is explicit and isolated in the presentation adapter.
12. Prefer immutable snapshots between simulation and presentation. If a copy
    is too expensive, introduce a bounded read model with explicit ownership
    instead of sharing mutable world state.

## Specification and Verification Gate

Before implementing a module, its specification must define:

- purpose and owner;
- public types and functions;
- dependencies and forbidden dependencies;
- units and coordinate spaces;
- constants read;
- invariants and mutation atomicity;
- errors and clamps;
- required tests;
- visual snapshot shape, if presentation consumes it.

Before finishing an implementation session:

- run formatting, tests, and strict Clippy checks appropriate to the change;
- verify that `sim_engine` imports remain inside its adapter;
- verify that no cross-domain import was introduced;
- update the relevant specification or status document with what changed,
  what was verified, what remains, and the nearest next step.
