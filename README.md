# Sim;X

Sim;X is a modular 2D scientific simulation platform. The active development
scope is the first coherent 4-of-7 breadth prototype for Sim;Phys: independently
specified vertical slices for Mechanics, Thermodynamics, Waves and Optics, and
Electromagnetism. Relativity, Fluid Dynamics, and Phys;Sandbox remain honest
navigation destinations without invented scientific runtimes.

Before changing code, read:

1. [`docs/READ_FIRST_SIM_X_BOUNDARIES_AND_CODE_STYLE.md`](docs/READ_FIRST_SIM_X_BOUNDARIES_AND_CODE_STYLE.md)
2. the focused specification for the subdomain being changed
3. [`docs/architecture/COMPOSITION.md`](docs/architecture/COMPOSITION.md) for Mechanics composition work
4. [`docs/architecture/Structure.md`](docs/architecture/Structure.md)
5. [`docs/product/ROADMAP.md`](docs/product/ROADMAP.md)

The complete documentation index is in [`docs/README.md`](docs/README.md).
The extension policy is in
[`docs/architecture/EXTENSIBILITY.md`](docs/architecture/EXTENSIBILITY.md).
The provisional Demo/Full and public-source distribution direction is in
[`docs/product/DISTRIBUTION_AND_MONETIZATION.md`](docs/product/DISTRIBUTION_AND_MONETIZATION.md).

## Current Prototype

Launch with `cargo run --all-features`. The native dark full-screen flow is:

```text
Main Menu -> Domains -> Sim;Phys -> Subdomain -> Projects -> Editor -> View Simulation
```

Mechanics supports body placement/dragging, mass scaling, and persistent force
dragging without stepping in Editor. Thermodynamics, Waves and Optics, and
Electromagnetism expose curated scientific setups; their Editor pages are
truthfully read-only. View owns live fixed stepping and Pause/0.25x/1x/4x for
dynamic slices. Returning from View always asks Apply, Discard, or Cancel.

Pendulum, Spring, Relativity, Fluid Dynamics, and Phys;Sandbox remain visibly
locked until their scientific contracts exist.

## Development Commands

```text
cargo run
cargo test --lib --no-default-features
cargo test --all-targets
cargo clippy --all-targets --all-features -- -D warnings
bash tools/check_module_boundaries.sh
```

The last command requires pinned `cargo-modules` 0.27.0 and is run by CI.

The local Sim;Engine reference is stored in
[`docs/integrations/sim_engine/DOCUMENTATION.md`](docs/integrations/sim_engine/DOCUMENTATION.md).
