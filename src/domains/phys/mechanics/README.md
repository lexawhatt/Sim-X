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
