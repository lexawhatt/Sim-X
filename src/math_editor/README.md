# Math editor boundary

This module is a native Sim;Logic consumer. It never imports Physics or a direct
GPU/Engine host. Math is a live mathematical document, not Physics Editor/Run.

- `app`: world lifecycle, input registration, font resource reuse, navigation.
- `state` / `commands`: equal-status rows, history, operation activation,
  appearance and geometric authoring gestures.
- `formula`: arena-backed editable notation and caret/slot traversal. Fractions,
  powers, radicals, absolute values, integral limits/body and derivatives are
  structures, not a pretty overlay over an unrelated plain-text input.
- `formula_layout`: iterative layout and the same caret positions for picking.
  Dynamic row height plus scrolling keeps deep expressions editable.
  Serialization and layout share the formula's implicit-product classifier;
  a visible dot has width, but is not a fake editable atom.
- `formula_selection` / `formula_import`: structural selections and atomic
  plain-math import using meval's tokenizer/RPN parser, preserving numeric spelling.
- `clipboard` / `platform/clipboard`: explicit intents and asynchronous native
  IO. Native ownership stays alive in the executable, never in headless builders.
- `parameters` / `motion`: literal-definition sliders, editable ranges and one
  undo transaction per drag/play session; separate decorative easing.
- `camera_motion` / `spatial` / `spatial_view`: eased zoom/reset and 2D/3D
  transition, orbit ownership, Logic Camera3d projection and open-axis wireframe.
  Camera motion never morphs mathematical answers; surface sampling is off-thread.
- `spatial_clip` / `wireframe_view`: invisible, view-scaled world bounds and
  near/far segment clipping before projection, followed by depth-faded isolines.
  Samples outside the volume are not clamped into artificial boundary faces.
- `worker_spatial` / `surface_paths`: cancellable implicit XYZ boundary sampling
  and endpoint stitching/deduplication on the worker, not the frame thread.
- `row_interpretation`: explicit, fading XY-curve hints for ordinary rows in 3D.
- `axis_ticks` / `axis_view` / `axis_spatial`: zero-anchored 1/2/5 major/minor
  grids, font-width-aware collision avoidance and projected XYZ numeric ticks.
  Density follows screen spacing even for foreshortened 3D grids.
- `layout` / `input`: shared logical-pixel hit regions, sidebar/catalog scrolling,
  keyboard entry, graph pan/zoom, click vs hold, focus and modal ownership.
- `row_flow` / `editing` / `entry_hint`: click-to-write draft rows, cross-row
  focus, local held-key cadence, contiguous-symbol undo groups and slot hints.
- `point_labels`: screen-space placement prefers clear space, with a small
  background for dense cases. This is heuristic layout, not a guarantee that
  arbitrarily crowded labels all fit.
- `worker`: one latest-request mailbox; scalar dependency resolution, sampling,
  contour jobs and round-robin resumable integration. Cancellation invalidates
  obsolete publication; each row owns its result and diagnostic.
- `worker_calculus`: resumable result adapter and one-request scalar cache.
  Camera-only resampling preserves completed calculations; source edits clear
  the cache, and activation changes invalidate only the affected row.
- `integral_plot` / `integral_view` / `integral_motion`: worker-sampled boundary
  curves and signed bands, 2D-only fill plus projected-XY curves, and per-row reveal
  after successful calculation. Exact bounds are included even below pixel size.
- `graph_style`, `curve_display`, `contour_display`: appearance, subpixel display
  simplification and contour stitching. They do not modify mathematical samples.
- `view`, `sidebar_view`, `graph_view`, `drawing`, `assets`: retained native
  presentation and bundled font metrics, not scientific state.

## Input and result conventions

One blank row on opening. Click empty sidebar space to start an expression;
the draft affordance reuses a trailing empty row. Enter inserts immediately
after the current expression, reusing a following blank. Empty Enter does not
create more blanks. Shift+Enter always takes this path, including integrals.
`+ Expression` still explicitly appends. Backspace removes an empty row and
focuses the previous end; holding it cannot delete that previous expression.
Up/Down first navigate slots, then rows; focus scrolls into view.

`root(3,x)`, `nthroot(5,x)` and `cbrt(x)` display editable indexed radicals;
`sqrt(x)` remains a square root. The n-root key starts with an empty degree;
comma/Tab moves to its value. Odd integer degrees allow negative values;
zero degrees and even roots of negative values are undefined in this real core.
Root/function/power/fraction actions wrap selections. Removing a radical wrapper
preserves its value; Backspace in an empty exponent/denominator restores the base.
Ambiguous products get a visible dot, e.g. x^4 followed by Right, then 7x.
Digits do not automatically exit powers: x^12 must remain possible. Right or
Space exits; ordinary arithmetic after a completed exponent resumes the baseline.

Typing `int(` creates an integral with editable default limits 0 and 1; bounds
and integrand are separate slots, not comma-delimited arguments in a raw box.
Calculate/Enter explicitly activates its approximate signed result. Use `abs`
or bars in the integrand for unsigned geometric area. Subsequent edits update
activated operations; undo/redo disarms them to avoid index-based misactivation.
Arithmetic/scalar functions around multiple definite integrals also use Calculate.
For example, `y=integral(0,1,x^2)-integral(0,1,x)` produces a horizontal -1/6
after calculation. Bare differences on a common interval display both scaled
curves and the signed region between them; Calculate reveals the fill over 0.8s.
Positive contributions use the row color; negative contributions use the legend's
contrasting color. Reversed bounds reverse the contribution, not the curve.
Crossing curves may have zero integral despite a nonempty geometric region.
Different intervals and linear sums show separate contributions. Scalar offsets
are disclosed; nonlinear compositions show reference curves without false area.
Linearity is checked structurally in the core, not guessed from numeric samples.
Use an absolute-value integrand for geometric area. y=integral expressions show
the completed horizontal result only; omit y= for the explanatory region view.
Nested calculus and free coordinates outside the integrals remain unsupported.
Fill is finite-resolution illustration, not the quadrature result itself or
certified intersection geometry. Known/unresolved singularities prevent a
successful result and its fill. Stale numbers are hidden during source edits.

Expressions stay in the left sidebar. The optional bottom keyboard is hidden
on startup; Show keys opens it without changing physical-key entry. It spans
the window; Functions anchors the categorized popup above its button.
Both expression list and graph stop above this dock. Hiding it expands both,
without changing the document. Popup input cannot create graph objects beneath it.

Physical ASCII typing and ABC/123 on-screen pages are supported. Ctrl+A selects
a whole field; Shift+arrows and mouse drag select structural ranges. Crossing a
slot boundary selects its containing atom, never half a fraction's ownership.
Ctrl+C/X/V use the OS clipboard. Copy without a selection copies the active row;
cut removes it only after successful copying. Paste at the caret is one undoable
operation. Cached internal transfers retain exact structure. External paste
accepts one plain expression/equation, explicit integral(a,b,body), registered
functions and implicit multiplication; it rejects LaTeX, unsupported characters
and multiple rows without changing the document. Native service uses arboard
3.6.1, default features off, optional desktop-only Wayland data-control support.
It retains clipboard ownership and never blocks the frame on a clipboard read.
Late paste/cut results cannot change a different document revision or selection.
Held letters/digits, arrows and deletion use a local 450ms delay and 25Hz target
cadence, at most one repeat per rendered frame and no stalled-frame backlog.
Enter, templates and shortcuts never auto-repeat. Single-symbol edits coalesce
until a 700ms pause, selection, navigation or structural action; clipboard and
templates remain separate undo transactions. No committed-text/IME, OS-configured
native repeat or localization claims. The fixed input queue is not involved.
`sin^-1(x)` is principal arcsine; `sin(x)^-1` is reciprocal.
The scientific core accepts explicit multiplication; the notation editor adds
it for natural `2x`, `xy`, and `2sin(x)` input.

Short click on a color toggles graph visibility. A stationary 0.45-second hold
opens color/width/opacity/line-pattern controls; release does not also toggle.
Lost focus cancels ownership. Popups block background activation. Styles are
presentation metadata and do not restart sampling or integration.

Scalar definitions resolve forward references. Invalid rows receive their own
diagnostic without suppressing other rows. While a new request is pending,
previous geometry is dimmed; once it completes invalid rows have no stale result.
On-function construction points currently follow the first explicit y=f(x)
row, not a stable named function. Changing/removing that row can change their
reference: stable per-row mathematical handles are a future milestone.

## Work and remaining limits

No artificial Math formula/object/depth/total-work quota. Parse of one large
formula is not preemptible; arena-to-source conversion and full document history
can be expensive for huge edits. Sampling checks cancellation between samples
or grid rows; integration yields between panels. Visual pools grow incrementally,
with notation prioritized before graph segments. Simplification tolerance is
0.35 logical pixels; contour endpoint stitching snaps within about 0.001 pixels.

Upstream Sim;Logic still has a 10,000-command scene envelope that its public API
does not yet make configurable (local LOGIC-009). Dense views may exhaust it.
Do not confuse unbounded documents with unlimited hardware/rendering resources.

The slice is session-only, with approximate implicit plane curves and derivatives,
plus an orthographic 3D wireframe for explicit z=f(x,y) and implicit XYZ relations.
Examples: x^2+y^2+z^2=9 and max(abs(x),abs(y),abs(z))<=1. Type absolute-value
bars normally; <=/>= become editable comparison glyphs, also accepting pasted
Unicode comparisons. The numeric dock includes all four comparison keys.
Switch views with
the stable 2D / 3D button; drag to orbit, wheel zooms centrally, Home resets smoothly.
XY tilts down to expose upright Z; returning raises XY to its frontal view.
Scientific coordinates never change. The existing orthographic camera is
retained; there is no perspective FOV to reduce. Grids, axis labels, mode
captions and toolbar fade with the same reversible blend. Disabled fading
controls block click-through. Reduced motion is immediate.
The numeric keyboard gains a supplementary z key with the same transition fade
and a short slide. It cannot activate during the return-to-2D fade; ABC keeps
its full alphabet. Reduced motion applies the change immediately. XYZ values
share label collision avoidance; degenerate projected axes omit crowded labels.
Plane curves/constructions explicitly stay on XY (z=0); y=x is not automatically
extruded into a 3D plane. Integral fill is 2D only, fading near the frontal endpoint;
the orbit view keeps curves without projected-area strips.
Return to 2D to edit constructions; orbit
gestures cannot accidentally move their inputs. Focus loss/resize cancels orbit.
`x`, `y`, `z` are coordinates; z=constant creates a plane, never a slider.
The implicit sampler takes cross-sections in all three coordinate directions.
An extra pass through discovered extrema exposes flat-face outlines between
regular sections; it does not recognize hardcoded shapes or fabricate zeroes.
Strict inequalities use dashed excluded boundaries; non-strict comparisons include
their boundary. Only the boundary is drawn: volume/interior shading is NOT
implemented. Empty output says no resolved boundary, not that the region is empty.
Planar inequalities have a specific unsupported-region message, not a syntax error.

Surface grid intervals target 40 logical pixels before projection, with eight
samples per interval before 0.35px projected simplification. XYZ surface bounds
share the axes' view-scaled range and stay fixed during orbit/transitions; zoom
changes the sampled extent. The bounds fit in the view and are never drawn as a
cube. World segment clipping retains intersections even if both ends are outside;
camera near/far clipping precedes Logic projection. Depth fading multiplies the
user's opacity. Ordinary XY curves draw above the transparent wireframe, and the
axis labels/top hint have backgrounds. Home uses azimuth 45/elevation 30 degrees,
cancels old pan/orbit ownership and takes the nearest equivalent yaw turn.
This remains sampled visualization, not a completeness guarantee. Hidden edges
stay visible; unresolved small components and tangencies can be missed.

Sums/products, named functions, lists/matrices, arbitrary
inverse branches, filled 3D surfaces/inequality volumes, solids of revolution, non-Euclidean
views, persistence and full text input remain unimplemented. Mesh construction
and filled-scene subviewport needs are recorded as LOGIC-013/014. No direct
Engine or second GPU host was introduced. Wireframes use the existing managed
screen-vector path and the re-exported Engine camera, not filled depth rendering.

Math explicitly pauses its unused fixed schedule. A zero time scale alone left
fixed input copies retained forever on direct launch, overflowing after 128
edges. The frame UI and asynchronous mathematics continue during pause. The
input cap is unchanged; long key/click sequences regress the root cause.
Thickness steps are 0.5, with a positive 0.5 minimum. Visual-pool expansion now
schedules up to 96 entities per frame under a 128-command host envelope instead
of slowly revealing dense plots in twelve-stroke batches. This is scheduling,
not an object-count limit. GPU throughput is not inferred from headless tests.

Headless tests drive real physical-key/mouse events, including long hold,
cancelled input, expression nesting, scrolling, history and linked integration.
They do not substitute for native visual or performance verification.
Optional CPU graph preview (not native text/GPU evidence): set
`SIM_X_MATH_PREVIEW_SVG` to an output path when running the
`native_3d_switch_orbit_and_return_preserve_document` headless test.

Literal scalar definitions have sliders; Add suggestions create missing
parameters. Range endpoints are editable finite ordered numbers, not limits on
document values. Playback uses an eight-second smooth cycle with compute
backpressure; heavy plots slow playback instead of accumulating stale work.
Only one parameter plays at once. Parameters and measurements are not interpolated
between computed graphs. Hover/popup easing honors reduced motion. Explicit
parameter playback remains an intentional mathematical operation.

Delete before an integral, Backspace after it, or Backspace at the start of its
body removes only the wrapper. Its old bounds are discarded (Undo restores them).
Integral insertion wraps selected content or the remaining current row and
starts with editable 0/1 bounds.

Typography debt: the integral sign currently uses a temporary two-Bezier outline
and radicals use procedural strokes. Replace this with licensed mathematical
glyphs and appropriate sizing/limit metrics; do not claim finished math typesetting.
