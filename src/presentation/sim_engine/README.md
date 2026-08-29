# Sim;Engine Adapter

This is the only directory allowed to import the `sim_engine` crate.

It owns checked conversion from Sim;X units and `f64` values to finite visual
`f32` state, scene/resource construction, camera and picking conversion,
rendering-path selection, budgets, renderer diagnostics, and GPU recovery.

For the current shell it also owns all drawing, including panels, buttons,
editor canvas, hover visuals, and an adapter-local pixel font. `winit` hosts the
window and input loop but is not a second renderer.

Sim;Engine visual particles, scalar fields, vectors, and meshes are snapshots
or GPU resources. They are never canonical domain entities or storage. Scene
building and rendering must not advance simulation state.
