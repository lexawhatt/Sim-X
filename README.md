# Sim;X

Sim;X is being rebuilt from scratch on
[Sim;Logic](https://github.com/lexawhatt/Sim-Logic). The current implementation
scope is the main menu, domain/scale selection, a Physics editor with an
independent 2D mechanics runtime, and an independent Math workspace with
2D graphs and an initial 3D wireframe view.
The earlier Physics prototype has been removed
from the active source tree; its scientific contracts and review history remain
available as reference, not as claims about this build.

The menu uses a dark theme, real bundled fonts, smooth hover feedback,
keyboard navigation, and GitHub, YouTube, and Telegram icons at the lower left.
Four animated motifs rotate through the main menu, one per five-second slot,
with a smooth crossfade. Domains opens a
centered selector for Physics, Mathematics, Chemistry, and Biology; its faded
background starts empty and previews a subject only on hover or selection.
Hovering a domain briefly pulses its motif. Settings contains a reduced-motion option, and Quit requires
confirmation. Physics opens **Phys;X** with **Micro**, **Macro** and **Astra**.
Hover or keyboard focus previews an atom, a carousel of coherent mechanisms
(pendulum, spring-mass, gears), or a centered flat Earth/Moon diagram. All use
the same thin-contour style; Macro shows one mechanism at a time.
Only Macro is available: **Sim;X/Phys/Macro -> Create new project -> Name -> Editor**.
Names are trimmed, nonempty and limited to 64 characters; native entry currently
supports physical ASCII keys, not clipboard or IME input. Projects are session-only,
not files on disk. The Physics editor remains a unified construction workspace
without mechanics/thermodynamics submenus. Its `sim-physics` library runs gravity, applied
forces, rods, springs and optional linear drag without a renderer. Persistent
projects, music and Sim;Time are not connected to this build.

## Build and Run

Rust 1.95 or newer is required. Keep a Sim;Logic checkout at `../Sim-Logic`:

```text
cargo run
```

Launch directly into the Physics editor (Cargo's `--` forwards application flags):

```text
cargo run -- --phys_editor
```

`--phys-editor` is an equivalent spelling. The editor starts blank with a
right-hand Inspector. A six-to-eight-slot quick dock provides recent/favorite
items alongside a searchable command palette, instead of a flat list of every
object. The catalog uses five function
categories: Bodies, Connections, Assemblies, Measure and Actuators. Idealized
models are badges/filter metadata, not a separate category or Physics subdomain.
See the current local UI contract for implementation and interaction status.

Implemented entries are circular Ball and rectangular Box colliders, fixed
non-colliding Anchors, Rod and Spring relationships, and prepared Pendulum,
Spring oscillator and Bounce lab assemblies.
Gravity/linear drag settings live only in Environment, not as fake tools in the
catalog. Empty categories explicitly describe
what is not implemented; they do not place fake scientific objects.

Click a primitive or prepared assembly card and then the canvas, or drag its
card onto the canvas. Connections select a tool: click the first existing body,
then the second. Click an authored object to select it and drag to move. The
entire scene is **in memory only**; closing discards it.

- `1` / `2` / `3`: Select / Build / Erase; `Delete`: remove selection.
- `4` / `5`: Rod / Spring connection tools; click two endpoints.
- MMB drag: pan; wheel: zoom around pointer; `Home`: reset camera.
- `G`: toggle 0.5 m snapping; `Ctrl+Z`: undo; `Ctrl+Y` or `Ctrl+Shift+Z`: redo.
- `Ctrl+D`: duplicate; Inspector adjusts mass, physical diameter/width/height,
  initial orientation, fixed/dynamic state and restitution.
- `/`, `Ctrl+K` or `Ctrl+F`: open the focused catalog search; arrows/Enter
  choose, Escape closes. Click a card's star to pin it in quick access.
- In Select: `Shift+drag` rectangle, `Ctrl+drag` lasso; drag a selected group
  to move it, Delete removes it. Group movement/deletion is one undoable edit.
- Build keeps its chosen placement active for repeated stamps, without Shift.
- Catalog search currently uses ASCII physical-key input; localized aliases do
  not imply native Cyrillic/IME text entry.
- `F5` / Run: start a separate simulation; Stop / Escape: return to Editor.
- Space in Run: pause/resume; playback buttons select 0.25x, 1x or 4x.
- `E` / Environment: adjust shared gravity and linear drag.
- Escape during a drag cancels it without modifying the document.

**Editor never simulates.** Run constructs a validated independent world and
advances fixed 1/240-second steps. It hides structural editing and the Inspector.
Stop discards runtime motion and runtime environment changes, leaving the
authoring scene untouched. Copying a runtime state back to Editor is not
implemented. Environment changes in Editor are undoable scene settings for new
runs; changes in Run affect that run only. Opening Environment suspends physical
stepping until it closes. The current panel has increment/decrement controls
and Earth/zero-gravity presets, not arbitrary text entry.

For a first experiment, place the **Pendulum** assembly, press Run, then change
Gravity Y in Environment and observe the different motion. Assemblies consist
of ordinary bodies and relationships. In Editor, moving a rod endpoint changes
its authored length; moving a spring endpoint preserves its rest length and
changes initial strain. Deleting a body also deletes its incident relationships
in the same undoable edit. Duplicate currently copies a body, not its connections.

For contacts, place **Bounce lab** from Assemblies (or quick access on a wide
window), then Run. Its ordinary fixed rectangle and two balls use restitution
0.2 and 0.8. Each part can be selected, resized or edited independently. Build
your own obstacles by placing a Box, changing its width/height and enabling
Fixed in the Inspector. Anchors are attachment points, not invisible walls.

Springs attach to the clicked surface, including a rectangle corner (snapping
within eight logical pixels). The highlighted endpoints are the actual physical
attachments, not shortened centre-to-centre drawings. Moving or rotating a body
keeps its attachment; resizing preserves its relative location on that body.
Stretch a spring by moving an endpoint after connecting it. Off-centre spring
forces can both translate and rotate a body. Rods still connect centres only.
The drawn spring itself is massless: it does not collide or wrap around obstacles.

The current model is **2D mechanics** with disk/rectangular-lamina inertia and
frictionless circle/rectangle contacts, not three-dimensional rigid-body physics.
The kernel owns angle and angular velocity; Run reads them for body and
attachment rendering. Pair restitution uses the larger
of the two body values, with an explicit low-speed settling policy in the
[kernel contract](crates/sim-physics/README.md). Rods can push and pull, and no
implicit ground exists. Initial intersecting colliders reject Run instead of
silently changing the authored scene. Unsupported fast/swept contact cases
pause atomically rather than silently passing through obstacles. Heat, fluids, electricity, quantum
models, custom-object saving and the full teacher-facing nerd-mode are future
work. Run shows a compact time/step/energy or overload/failure summary; the
kernel already exposes typed force and energy observations for later instruments.

The editor minimum viewport is 900x600; current authoring budgets are 128
objects, 256 relationships and 64 history entries. Catch-up work is capped at
32 physical steps per frame, with excess simulation time reported rather than
hidden by changing the physical step. The main-menu route remains unchanged
while the editor uses the direct development launch.

The independent library, numerical limitations and architecture are documented
in [`crates/sim-physics/README.md`](crates/sim-physics/README.md) and
[`ARCHITECTURE.md`](crates/sim-physics/ARCHITECTURE.md). Run its renderer-free
reference example with:

```text
cargo run -p sim-physics --example pendulum
```

The development profile optimizes the rendering/runtime libraries as well as
the application. The first rebuild after this profile change takes longer;
unoptimized geometry validation is not representative of native UI performance.

This integration was read against Sim;Logic commit
`b48191159c188d6e12e997d153ccd17cbdd40ab9`, whose manifest pins Sim;Engine to
`=0.4.1`. The local path dependency is intentional during integration; Cargo
does not pin that checkout's Git revision. Review its changes before updating
the baseline.

The desktop requests borderless fullscreen at startup. F11 switches to a
resizable 1280x800 window and back using Sim;Logic's native window controls.
Integration requests and their migration status are recorded in
[`INTEGRATION_ISSUES.md`](workflow/integrations/sim_logic/INTEGRATION_ISSUES.md).

Menu controls: mouse press and release on the same button, Tab/Shift-Tab or arrow keys to move
keyboard focus, Enter or Space to activate, and Escape to close a dialog or
return from Domains. Escape at the main menu opens Quit confirmation. The
menu's minimum usable window size is 360x560 logical pixels; smaller windows show a
resize notice.

Official links: [GitHub](https://github.com/lexawhatt/Sim-X),
[YouTube](https://www.youtube.com/@LexaWhat),
[Telegram](https://t.me/Simulation_X).

## Mathematics

Open Math in Domains, or launch directly with `cargo run -- --math_editor`
(`--math-editor` also works). The workspace is session-only; leaving asks for
confirmation. Physics is not used by mathematical computations.

The left sidebar contains equal-status expression rows rather than a fixed
function form. Type `y=x^2`, `b=-0.6`, `x^b`, or `x^2+y^2=3`. Rows have their
own color and diagnostics; scalar definitions update dependent expressions.
Click a colored icon to hide its graph; hold for 0.45 seconds to choose color,
width, opacity, or solid/dashed/dotted lines. Appearance does not change math.

- Natural typing, not LaTeX: fractions, superscripts, nested roots, absolute-value
  bars and editable integral limits. Arrow keys and Tab navigate slots; click
  to position the caret. Shift+arrows or mouse drag select formula parts; crossing
  a template boundary selects the enclosing structure. Ctrl+A selects a row.
  Ctrl+C/X/V use the system clipboard; Ctrl+Z/Y undo/redo. External paste accepts
  one plain-math expression at a time (not LaTeX), rejecting unsupported text.
  Removing an integral at its boundary preserves the integrand; inserting an
  integral wraps the selection or the expression to the right of the caret.
- Click empty sidebar space to start another expression. Enter inserts after
  the current row; Shift+Enter does this even in an integral. Empty-row Backspace
  returns to the previous expression. Held arrows/deletion repeat safely, and
  consecutive typing shares an undo transaction until a pause or navigation.
- Indexed roots: `root(3,x)`, `nthroot(5,x)`, `cbrt(x)` have editable degrees.
  The n-root key opens degree/value slots; Tab or comma moves to the value.
  Templates/functions wrap selected content. Ambiguous products display a dot;
  use Right or Space to leave a power, so `x^12` remains a valid exponent.
- `int(` inserts an integral with default limits 0 and 1. Type its integrand,
  edit the limits, and use **Calculate** (or Enter in that row). Results are
  approximate signed integrals; integrate an absolute value for geometric area.
  Activated integrals recalculate when their inputs change. Undo/redo disarms
  calculation to avoid transferring activation to a different row.
  Scalar arithmetic around multiple integrals also works: for example,
  `y=integral(0,1,x^2)-integral(0,1,x)` gives a horizontal -1/6 after Calculate.
  Without `y=`, a difference with common bounds shows both curves and smoothly
  shades their signed separation. Positive/negative contributions have distinct
  colors; reversed limits and curve crossings preserve the integral's sign.
  Different intervals show separate contributions, while nonlinear combinations
  show reference curves without pretending their area equals the result.
  Nested calculus is not yet supported.
- An optional bottom keyboard, hidden initially, inserts into the focused row. Its
  Functions button opens a scrollable catalog directly above the button, leaving
  the sidebar for expressions. ABC/123 switches letters and scientific keys.
  The catalog shares evaluation's registry: trigonometric/inverse
  and hyperbolic functions, logs/roots, scalar statistics, Gamma/erf, angles,
  and visual building blocks such as `smoothstep`, `sinc`, `sigmoid`, `softplus`.
- Literal definitions such as `g=-0.6` have a slider and Play/Pause. Unknown
  parameters offer an Add button. Click either slider endpoint to edit its range;
  Enter applies and Esc cancels. The interval controls the slider, not which
  numbers can be typed. A drag or playback session is one undoable change.
- Wheel scrolls the sidebar/catalog or zooms the graph under the pointer;
  MMB pans. No artificial expression/object/depth/total-work quotas are imposed
  by the Math document. Numerical precision and actual renderer resources remain
  finite; dense scenes can exceed the current upstream command envelope.
- **2D / 3D** tilts XY down to reveal upright Z, then raises it back on return.
  Axes, labels and controls transition together; fading controls cannot click through.
  In 3D, try `z=sin(x)+cos(y)` or `z=(x^2-y^2)/4`. Drag the canvas to orbit;
  wheel zooms around its center and Home resets smoothly. Existing plane curves
  and constructions are explicitly on XY (z=0); y=x is not automatically extruded
  into a plane. Integral shading stays in 2D, fading out near the frontal view.
  Construction editing is available in 2D; point labels prefer nearby clear space.
  The axes are open, without a bounding cube. `x`, `y`, `z` are coordinates,
  not scalar-slider names. Reduced motion makes camera changes immediate.
  A supplementary `z` key fades into the numeric keyboard in 3D. All XYZ axes
  have numeric ticks; adaptive major/minor grids and collision-aware labels
  keep zoomed-out views readable.

This is **not full Desmos compatibility**: no named function
definitions, list/matrix algebra, sum/product operators, inference/distributions,
CAS or persistence yet. Implicit contours and derivatives are numerical.
3D supports sampled explicit height fields and implicit XYZ boundaries, including
`x^2+y^2+z^2=9` and `max(|x|,|y|,|z|)<=1`. Comparisons can be typed naturally or
pasted as Unicode. Inequalities display their boundary only (strict boundaries
are dashed), not a filled volume. World/near-far clipping and depth fading make
wireframes readable without a visible bounding cube. Filled surfaces,
hidden-surface removal and solids of revolution remain unimplemented. Orthographic projection
uses Logic's public Engine camera; missing Logic mesh/subviewport APIs are logged
in the integration ledger. Sampling and plot preparation remain asynchronous.
Native typing is currently physical ASCII keys, without committed-text/IME input.
Clipboard IO is asynchronous and desktop-only; errors are shown in the footer.
Math glyph typography still needs refinement, particularly the integral sign.
General inverse functions need branch selection; `sin^-1(x)` means principal
arcsine, while `sin(x)^-1` is a reciprocal. These are different operations.
Direct Math pauses its unused fixed schedule so input cannot accumulate waiting
for a simulation tick; frame animations and calculations continue normally.

See [the Math core](crates/sim-math/README.md) and
[the editor boundary](src/math_editor/README.md) for current contracts and limits.

## Development

Before changing code, read
[`READ_FIRST_SIM_X_BOUNDARIES_AND_CODE_STYLE.md`](workflow/READ_FIRST_SIM_X_BOUNDARIES_AND_CODE_STYLE.md).
Then read the [main-menu contract](workflow/product/MAIN_MENU.md) and
[architecture](workflow/architecture/Structure.md). The [documentation index](workflow/README.md)
distinguishes current contracts from historical and future design.

`workflow/` contains local working documentation and is intentionally ignored
by Git. These local links are available in this working checkout, not promised
as files in a fresh public clone.

```text
cargo fmt --all -- --check
cargo test --locked --workspace --all-features --all-targets
cargo test --locked --workspace --no-default-features --all-targets
cargo clippy --locked --workspace --all-features --all-targets -- -D warnings
cargo clippy --locked --workspace --no-default-features --all-targets -- -D warnings
cargo test --locked -p sim-physics --all-targets
cargo tree --locked -p sim-physics
```

For a bounded native cadence diagnostic without opening external links:

```text
cargo run --example menu_probe -- --seconds 22
```

This reports actual drawn/skipped frames and end-to-end cadence, including
warmup; it is not a controlled GPU benchmark. Do not interact during the sample.

The application `--no-default-features` profile uses `headless-text`: no window
or GPU device, and no wgpu/winit compile dependency. The independent Physics
crate is std-only in every profile; CI also checks its actual Cargo dependency
metadata. Headless application tests cover editor/run separation, input and
catalog behavior; separate kernel tests cover physical equations, numerical
limits, conservation/convergence and atomic failure. Test totals and native
verification results belong to the current working ledger, not old gate reports.
The desktop feature enables GPU text separately.
IBM Plex Sans Medium supplies UI text; Space Grotesk Medium supplies the wordmark.
Their original OFL notices and provenance ship in `assets/fonts/`.

The current implementation and verification ledger is in
[`WORKFLOW.md`](workflow/WORKFLOW.md). Distribution plans remain provisional in
[`DISTRIBUTION_AND_MONETIZATION.md`](workflow/product/DISTRIBUTION_AND_MONETIZATION.md).

## Previous Implementation

The prior implementation is recoverable from Git commit
`082446c` (`PRE_REWRITE_ON_SIM_LOGIC`). A full local pre-rewrite archive,
including the then-current documentation and assets, is stored at
`/home/lexa/Desktop/Rust/Sim-X-rewrite-backup.XBDa5H/pre-rewrite.tar.gz`.
Do not restore old runtime or scientific modules into the new menu implicitly.
