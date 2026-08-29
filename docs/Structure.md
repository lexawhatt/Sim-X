# Sim;X Architecture

## Architectural Style

Sim;X is a modular monolith. All modules run in one process, but dependency
direction and ownership are explicit. The initial codebase stays in one Cargo
package; a module may become a separate crate later if compile-time isolation,
build time, or independent reuse creates a concrete need.

The architecture has four active areas:

1. application composition;
2. a small domain-neutral foundation;
3. isolated domain modules;
4. presentation adapters.

Extensibility is a policy across these boundaries, not a fifth runtime layer.
Custom Objects are persisted compositions, and new official behavior is added
as first-party Rust rule packs inside the owning domain. See
`EXTENSIBILITY.md`.

## Project Tree

Project-level contracts live in `docs/`; local `README.md` files state the
boundary of their source module. A Rust module is added only when an approved
vertical slice gives it real behavior, so inactive domains remain documented
folders instead of placeholder type systems.

```text
Sim-X/
|-- README.md
|-- docs/
|   |-- README.md
|   |-- READ_FIRST_SIM_X_BOUNDARIES_AND_CODE_STYLE.md
|   |-- COMPOSITION.md
|   |-- EXTENSIBILITY.md
|   |-- Structure.md
|   |-- ROADMAP.md
|   |-- Implementation.md
|   |-- Brainstorm.md
|   |-- UI.md
|   |-- Ideas.md
|   `-- DOCUMENTATION.md
`-- src/
    |-- main.rs
    |-- app/
    |   |-- README.md
    |   `-- mod.rs
    |-- foundation/
    |   `-- README.md
    |-- domains/
    |   |-- README.md
    |   |-- phys/
    |   |   |-- README.md
    |   |   |-- mechanics/README.md
    |   |   `-- thermodynamics/README.md
    |   |-- math/README.md
    |   |-- biol/README.md
    |   `-- chem/README.md
    |-- presentation/
    |   |-- README.md
    |   |-- mod.rs
    |   |-- audio/
    |   |   |-- README.md
    |   |   `-- mod.rs
    |   |-- sim_engine/
    |   |   |-- README.md
    |   |   |-- mod.rs
    |   |   |-- pixel_font.rs
    |   |   `-- scenes.rs
    |   `-- ui/
    |       |-- README.md
    |       |-- mod.rs
    |       |-- application.rs
    |       |-- catalog.rs
    |       |-- geometry.rs
    |       |-- layout.rs
    |       `-- state.rs
```

Product labels retain semicolons, such as Sim;phys and Sim;math. Rust modules
and directories use `phys` and `math` because they are identifiers rather than
branding.

## Dependency Map

```text
main -> app
app -> foundation
app -> domains
app -> presentation
domains -> foundation
presentation -> foundation + domain read models
presentation/sim_engine -> external sim-engine crate
```

Arrows mean "may depend on". Reverse dependencies are forbidden. The `app`
module is the composition root and may connect implementations, but it must not
absorb domain algorithms.

## 1. Application Composition

`app` owns application lifecycle and orchestration:

- selecting a domain and curated simulation;
- deciding when validated simulation steps occur;
- routing user intents to a domain API;
- requesting a presentation snapshot after state changes;
- coordinating startup, shutdown, pause, reset, and diagnostics;
- composing explicit cross-domain scenarios when a specification permits one.

`app` does not implement formulas, collisions, rendering primitives, GPU
recovery, widget drawing, or an external extension runtime.

## 2. Foundation

`foundation` is the smallest stable vocabulary shared by multiple domains or
boundary adapters. Expected concepts include:

- API version;
- typed SI units;
- constants registry and real-world preset;
- stable identifiers;
- validated simulation time values;
- numeric-safety outcomes;
- domain-neutral lifecycle and snapshot metadata.

Force, particle, reaction, cell, graph, camera, color, and widget types do not
belong here. A type moves into `foundation` only when multiple concrete owners
share the same meaning and invariants, not merely the same field layout.

## 3. Domains

Each domain owns its state, rules, errors, commands, read models, and tests. No
domain imports another domain.

### Sim;phys

`domains/phys` owns physical concepts such as mass, forces, particles, bodies,
collisions, integration, fields, momentum, and energy. Its subcategories may
share the physical foundation through explicit internal APIs.

The first vertical slice lives in `domains/phys/mechanics` and begins with
Newton's second law, canonical body state, applied force, and deterministic
integration. The editor's first composition is a pendulum assembled from
mechanics capabilities. Thermodynamics, Waves and Optics, Electromagnetism,
Relativity, Fluid Dynamics, and Phys;Sandbox are implemented only when their
specifications are ready.

`COMPOSITION.md` is the normative contract for Mechanics entities,
capabilities, relationships, interaction rules, contributions, deterministic
stepping, and saved subgraphs. This architecture document defines ownership;
it does not weaken those domain invariants.

An effect depends on required physical capabilities, not a concrete curated
simulation name. A pendulum composition may request gravity and constraint
capabilities, but its bodies only observe forces and constraints through the
public mechanics contract.

### Sim;math

`domains/math` owns functions, geometry, coordinate systems, parametric curves,
and stereometry. It may produce a 3D presentation read model for objects whose
meaning would be lost in 2D. It never stores `Mesh3d`, `Scene3d`, or other
Sim;Engine resource types as canonical state.

### Sim;biol

`domains/biol` owns cells, internal biological state, growth, division,
populations, and biological interaction rules. Spatial behavior is exposed
through explicit shared capabilities or application composition; biology does
not reach into Sim;phys internal storage.

### Sim;chem

`domains/chem` owns substances, concentrations, reaction graphs, activation
energy, environments, states, and transition rules. Chemical meaning must not
be reduced to untyped physical particles.

### Cross-Domain Policy

Every top-level domain reserves a local `Sandbox` subdomain for composition of
compatible capabilities owned by that domain. There is no automatic global
sandbox. If a scenario genuinely combines two domains, `app` coordinates
explicit commands and snapshots through their public ports. A direct
`domains::phys -> domains::chem` dependency is forbidden even when one
scenario uses both.

## 4. Presentation

Presentation owns how state is shown and how user gestures become application
intents. It may depend on domain read models; domains never depend on it.

### Sim;Engine Adapter

`presentation/sim_engine` is the only module allowed to import `sim_engine`.
It is an anti-corruption layer between scientific/domain state and visual
state. It owns:

- full-shell drawing through validated Sim;Engine `Scene` values;
- the adapter-owned pixel font and visual UI primitives;
- checked `f64` and typed-unit conversion to finite renderer `f32` values;
- scene, particle, scalar-field, dynamic-mesh, and retained-3D construction;
- cameras, visual picking conversion, visual interpolation, and styles;
- selection of the appropriate rendering path and update cadence;
- GPU resource identity, budgets, diagnostics, and recovery;
- rebuilding visual resources from the latest domain snapshot after recovery.

It does not own simulation entities, physical integration, formulas, canonical
fields, or simulation stepping. A rendered particle and a physical particle
are deliberately different types.

Sim;Engine v0.1.0 is pre-1.0, uses the `wgpu` feature by default, declares Rust
1.90 as its minimum version, and release-gates Linux with Vulkan. Version
upgrades require an adapter-level compatibility review because minor pre-1.0
versions may contain source-breaking changes.

### UI

`presentation/ui` owns menus, panels, editors, warnings, and interaction state.
It emits typed user intents. It must not directly change a constants registry,
entity store, or domain state.

Window creation remains a Sim;X host responsibility because Sim;Engine does not
create a window or event loop. The current host uses `winit` for a borderless
full-screen window and input events; Sim;Engine renders the entire interface.

## Extensibility

There is no active Lua system, native dynamic Rust plugin ABI, or generic
plugin adapter layer.

- User-authored behavior made from existing rules is stored as a Custom Object
  or recipe: a validated graph of domain state and relationships.
- New official scientific behavior is implemented as a first-party Rust rule
  pack inside the owning domain and compiled with Sim;X.
- Executable third-party extensions remain an explicit future question and do
  not influence current domain APIs.

`EXTENSIBILITY.md` is normative for this policy. `COMPOSITION.md` governs the
graph and rule contracts used by Phys;Mechanics Custom Objects and rule packs.

## State and Frame Flow

```text
input event
    -> UI intent
    -> app command
    -> domain validation and atomic state change
    -> immutable/bounded domain read model
    -> presentation adapter conversion
    -> Sim;Engine resource update and draw
    -> render status/metrics
    -> app diagnostics and UI warnings
```

Simulation cadence, visualization cadence, and surface presentation cadence are
separate concerns. Slow or skipped rendering must not silently change the
scientific result. Scene builders and renderer callbacks never advance the
simulation.

## Boundary Enforcement

At review and in future CI, verify:

- `sim_engine` imports occur only below `src/presentation/sim_engine`;
- `wgpu`, window, and UI imports do not occur in `foundation` or `domains`;
- no domain imports another domain;
- `foundation` contains no domain-specific or presentation-specific types;
- all lossy numeric and coordinate conversions are in boundary adapters;
- domain tests run without a GPU, window, UI runtime, or extension runtime;
- renderer failure/recovery tests preserve canonical domain state;
- module specifications match the template in `Implementation.md`.

## Extraction Triggers

Folders are the correct boundary for the first vertical slice. Extract a module
into a workspace crate only when at least one concrete pressure appears:

- compile-time dependency enforcement is repeatedly violated;
- the module has an independent release or reuse target;
- build times benefit materially from crate-level parallelism;
- feature flags cannot keep platform/GPU dependencies out of domain builds.

Do not split into microservices. Network and IPC boundaries would complicate a
local real-time simulation without improving the current ownership model.
