# sim-math

Independent real-valued Euclidean mathematics for Sim;X. No UI, renderer,
Physics, system clock, file access or random generator. `unsafe` is forbidden.
Application dependencies point into this crate, never out of it.

## Responsibilities

- `expression/`: meval grammar/RPN adapter, finite real evaluation, variable
  bindings and conservative interval/rectangle/XYZ-box domain screening.
  Spatial expressions use explicit `with_coordinates` / `evaluate_xyz` APIs;
  a plane evaluator never silently substitutes zero for an expression's z.
  Its structural affine analysis recognizes linear coefficients over integral
  operands without numerical probing; unproved nonlinear cases are not shaded.
- `functions`: one function/arity/convention registry, also used by discovery UI.
  statrs supplies statistics, Gamma and the error function. Variadic statistics
  accept scalar arguments, not lists or datasets. `functions/roots` defines
  `root(n,x)` (`nthroot` alias): odd integer indices accept negative radicands,
  negative indices give reciprocal roots, and zero indices are undefined.
  Positive radicands accept any finite nonzero real index; zero radicands
  require a positive index. Negative radicands do not choose a complex branch.
- `statement`: distinguish scalar values, y=f(x), x=f(y), implicit equations,
  explicit z=f(x,y), spatial relations, definite integrals and derivatives. Application scheduling
  and cross-row dependency resolution live outside the core.
- `integral`: resumable 16/32-point adaptive Gauss-Legendre quadrature using
  gauss-quad; signed integral and integral of absolute value are distinct.
- `calculus`: scalar arithmetic and functions around non-nested definite
  integrals. Balanced calls compile into a resumable calculation; no partial
  scalar escapes if one term fails. Per-integral accuracy is not a certified
  error bound on nonlinear outer expressions.
- `calculus/visualization`: renderer-neutral integral operands and mathematically
  justified between-curve or separate-contribution plans. Equal intervals,
  reversed limits, linear weights/offsets and nonlinear outer functions are
  distinguished independently of quadrature or viewport sampling.
- `contour`: sampled marching-triangle implicit contours, caller-selected
  resolution and cancellation. Not symbolic geometry or a completeness proof.
- `surface`: sampled explicit z=f(x,y) grid strips, separately selected grid
  density and interpolation resolution, domain gaps and per-sample cancellation.
- `relation`: equality and ordered comparisons of a finite real residual,
  preserving strict versus inclusive boundary membership.
- `implicit3d`: generic XYZ relation boundaries, e.g. `x^2+y^2+z^2=9` or
  `max(abs(x),abs(y),abs(z))<=1`. Three families of axis-aligned cross-sections
  use marching triangles with independent section spacing and contour resolution.
  One supplementary pass samples discovered coordinate extrema, also inserting
  those coordinates as grid knots to expose flat-face outlines between regular
  slices. It uses sampled geometry, not shape-name recognition or residual snapping.
  Domain screening rejects pole-crossing cells; cancellation is checked at every
  vertex and cell. There is no renderer, camera, triangle mesh or filled volume.
- `geometry`: free and derived points, directed constructions, segments/circles,
  line intersection and triangle area. Derived points cannot be directly moved.

## Numerical contract

Coordinates and results are real `f64`; angles are radians. Invalid domains,
non-finite results and exhausted subdivision precision return errors rather
than fabricated answers. No artificial formula/node/object/total-work ceiling.
Callers choose visual resolution and scheduling quanta; `advance(n)` means at
most n quadrature panels during this call, not n panels over the job's lifetime.

Domain screening is conservative and incomplete. It rejects intervals crossing
known poles and discontinuities; unsupported varying-argument combinations can
be rejected even when a scalar evaluation works. Negative Gamma intervals and
general improper integrals are not certified. Interval envelopes and quadrature
error estimates are not rigorous mathematical proofs. Contours can miss small
features, tangencies, isolated points or regions on which an equation vanishes
identically. In particular, an implicit 3D inequality is visualized by its zero
boundary only; this is not a claim to display its interior, and strictness must
remain visible in the caller's UI. Planar inequalities currently return a specific
unsupported-region error. No-z equations retain their existing XY-curve meaning;
`y=z` is a spatial relation, not a falsely promoted height field. Zooming changes
visual samples, never a numerical integral result.

Indexed-root screening preserves continuous negative odd-root branches and
handles varying indices on nonnegative radicands away from index zero. Varying
indices over negative radicands are conservatively rejected: nearby noninteger
indices need not have a real value. Integer parity follows represented `f64`
values exactly, without rounding or claiming odd integers beyond its precision.
Specialized square/cube roots use Rust's `sqrt`/`cbrt`; general roots use `powf`
with overflow-safe handling of reciprocal indices, subject to floating-point
rounding and underflow. See the [Rust f64 reference](https://doc.rust-lang.org/std/primitive.f64.html#method.cbrt)
for `cbrt`/`powf` precision; `sqrt` is correctly rounded.

The derivative row is currently evaluated numerically by the application
sampling adapter. There is no symbolic differentiation, CAS, list/matrix model,
distribution inference, arbitrary named-function definition or nested calculus
evaluation. Scalar definitions use single letters other than x, y, z and e;
the application diagnoses duplicate, unresolved and cyclic definitions.

## Dependencies and verification

Pinned: meval 0.2.0 (MIT/Unlicense), gauss-quad 0.3.2 (MIT OR Apache-2.0),
statrs 0.18.0 (MIT).
CI checks this direct-dependency allowlist and the full tree for application,
Physics, Logic and GPU dependencies. meval pulls nom 1.2.4, which emits a
future-Rust compatibility warning: replace/update the parser dependency before
claiming long-term compiler compatibility. The warning is not suppressed.

Run `cargo test -p sim-math`. Integration tests use the public API: analytic
integrals, domains/poles, statistical conventions, expressions beyond previous
quotas, construction dependencies, implicit contours and cancellation.

Function discovery was compared with the official
[Desmos supported-functions reference](https://help.desmos.com/hc/en-us/articles/212235786-Supported-Functions).
That page describes several products and mixes functions with operators and
object properties; this crate does not claim full compatibility or an equivalent
function count. Evaluator references: [meval ContextProvider](https://docs.rs/meval/0.2.0/meval/trait.ContextProvider.html),
[statrs Statistics](https://docs.rs/statrs/0.18.0/statrs/statistics/trait.Statistics.html),
[Gamma](https://docs.rs/statrs/0.18.0/statrs/function/gamma/index.html),
[erf](https://docs.rs/statrs/0.18.0/statrs/function/erf/index.html).
