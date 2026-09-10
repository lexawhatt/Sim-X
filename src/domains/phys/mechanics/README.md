# Phys;Mechanics

This directory owns the first Sim;X vertical slice: Newton's second law,
canonical body state, forces, acceleration, velocity, position, constraints,
and deterministic integration.

The initial implementation order is:

1. typed and validated mass, force, acceleration, time, velocity, and position;
2. a body with accumulated net force;
3. atomic `F = m * a` stepping with an explicitly documented integrator;
4. a bounded immutable presentation snapshot;
5. editor commands for bodies, forces, constraints, and a pendulum composition.

This module must not import `winit`, `sim_engine`, GPU types, pixel coordinates,
or UI state. A pendulum is a composition of mechanics capabilities, not a
special renderer object and not an independent source of physical truth.

## Current Implementation

Composition Gates 0–2 are implemented:

- private validated `f64` SI quantities;
- stable IDs and ordered body storage;
- dynamic/fixed body invariants;
- ephemeral `ForceReceiver` capability projection;
- canonical one-step force contributions;
- a hard 16,384-contribution work budget and 20,481-event ceiling;
- source-ID-independent exact binary64 superaccumulator force reduction with
  one ties-to-even rounding;
- checked semi-implicit Euler integration;
- explicit failure when non-zero time, velocity, or position progress is lost
  to `f64` representation;
- atomic world commit;
- bounded immutable snapshots and typed post-commit events;
- full and `--no-default-features` headless tests.

The app-owned desktop session now feeds fixed 120 Hz steps into this API and
the editor draws its immutable snapshot. That adapter is not part of the
domain. Gravity, constraints, collisions, angular mechanics, and canonical
pendulum behavior remain outside this implementation and must follow later
gates.
