# Sim;Engine Adapter

This is the only directory allowed to import the `sim_engine` crate.

It owns checked conversion from Sim;X units and `f64` values to finite visual
`f32` state, scene/resource construction, camera and picking conversion,
rendering-path selection, budgets, renderer error handling, and GPU recovery.
The adapter receives successful frame reports, but the current host does not
yet expose their performance metrics in application/UI diagnostics.

For the current shell it also owns all drawing, including panels, buttons,
editor canvas, hover visuals, and an adapter-local pixel font. `winit` hosts the
window and input loop but is not a second renderer.

The adapter targets Sim;Engine 0.2.0. Every redraw uses one bounded
`FrameComposer`: fixed chrome is a budgeted `ScreenScene`, while the scientific
canvas is a separate budgeted world `Scene` with its own `Camera2d` and
positioned viewport. Required scene construction is fallible and the first
rejection aborts the frame instead of silently dropping a visual.

The three project-owned social SVG files are normalized to square canvases.
`social_icons.rs` rasterizes them once into one exact 96-by-32 RGBA atlas and
uploads it as a retained `Image2d`. Hover scaling is viewport placement, so it
does not rebuild or upload icon geometry each frame.

Confirmed renderer limitations and the current bounded workarounds are tracked
only in `docs/integrations/sim_engine/SIM_ENGINE_RENDERING_GAPS.md`.

Sim;Engine visual particles, scalar fields, vectors, and meshes are snapshots
or GPU resources. They are never canonical domain entities or storage. Scene
building and rendering must not advance simulation state.
