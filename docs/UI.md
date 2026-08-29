# Sim;Phys User Interface Direction

Status: product concept for the active `Sim;Phys -> Phys;Mechanics` vertical
slice. This document does not authorize a universal UI implementation.

## Scope Boundary

This document specifies the Physics editor and View experience only. The shared
Sim;X shell may own full-screen hosting, domain navigation, accessibility
basics, and visual tokens, but it does not imply that every domain shares the
same workspace, panels, tools, or interaction model.

Sim;Math may need graphs, symbolic input, geometric canvases, or retained 3D.
Sim;Chem and Sim;Biol may require other domain-specific editors. Those products
will be designed when their vertical slices begin.

Do not extract a cross-domain panel or editor framework from Physics before a
second implemented domain proves which concepts and invariants are genuinely
shared.

## Design Position

The Sim;Phys editor should borrow proven structural ideas from Blender without
copying its visual density or requiring users to design their own interface
before they can run a simulation.

The product provides polished default layouts. UI customization is optional and
progressive: a new user can ignore it, while an experienced user can adapt the
workspace to a specific laboratory or construction workflow.

The application remains dark-theme-first. A light theme is not part of the
current product direction.

## Ideas Worth Borrowing from Blender

### Task Workspaces

A Workspace is a named, saved arrangement of UI areas for a task. Blender uses
task-specific workspaces composed of areas and editors. Sim;Phys can use the
same concept with a smaller physics-oriented set.

Initial Sim;Phys workspaces:

- `Editor` for structural scene construction;
- `View` for running and interacting with a simulation;
- optional user-created workspaces after the core layouts are stable.

Editor and View are product modes with different permissions, not merely two
cosmetic layouts. Changing a workspace must not bypass their rules.

### Purpose-Specific Areas

The application window is divided into resizable areas. Each area has one clear
purpose, such as:

- Simulation Viewport;
- Object Library;
- Scene Objects;
- Inspector;
- Tool Settings;
- Constants;
- Measurements;
- Event Log;
- Causality View.

An advanced layout may show the same area type more than once, but each instance
reads the same application state through defined presentation contracts.

### Scene Objects Browser

The Scene Objects browser borrows the clarity of an Outliner: hierarchical
display, search, selection synchronization, drag-and-drop organization, and
quick camera framing.

Its folders are intentionally simpler than Blender Collections:

- folders organize the browser only;
- placing objects in one folder does not parent their transforms;
- folders do not create shared physical behavior;
- an object has one organizational location in the initial design;
- physical grouping, constraints, and Custom Objects remain explicit concepts.

### Contextual Inspector

The Inspector follows the current selection and shows domain-owned properties,
units, validation, and Scientific Warnings. It appears in Editor Mode and is not
restored in View Mode merely because a panel layout contains an Inspector area.

## Official Default Layouts

### Editor Default

```text
+------------------+---------------------------+------------------+
| Object Library   |                           | Scene Objects    |
| search/categories|    Simulation Viewport    | folders/search   |
|                  |                           +------------------+
| Custom Objects   |                           | Inspector        |
|                  |                           | properties/units |
+------------------+---------------------------+------------------+
| status, scale, warnings, autosave state, editor hints          |
+----------------------------------------------------------------+
```

The viewport remains the largest area. Object discovery lives on the left;
scene organization and selected-object properties live on the right.

### View Default

```text
+------------+-------------------------------------+--------------+
| View Tools |                                     | Measurements |
| domain-    |          Simulation Scene           | optional     |
| specific   |                                     |              |
+------------+-------------------------------------+--------------+
| play/pause, speed, time, warnings, optional log toggle         |
+----------------------------------------------------------------+
```

View Mode prioritizes the scene. The Inspector is absent. Logs, measurements,
causality, and tool settings open only when requested.

These wireframes describe ownership and emphasis, not final dimensions or
visual styling.

## Customization Model

The recommended first customization level includes:

- resize panels;
- collapse and reopen optional panels;
- move panels between supported docking positions;
- choose which supported area type occupies an optional slot;
- save a named personal Workspace;
- duplicate and rename a Workspace;
- reset any Workspace to the official default;
- lock a layout against accidental movement.

Full Blender-style arbitrary splitting and joining of every area is deferred.
It would substantially increase input routing, focus, persistence, minimum-size,
and rendering complexity before the scientific editor is mature.

## Persistence and Scope

- Official defaults are always recoverable.
- Personal Workspaces persist across application launches.
- A project may remember the last active Workspace without owning the global
  Workspace definition.
- Editor and View defaults are independent.
- Layout persistence must be versioned so a UI update can migrate or safely
  reset an incompatible layout.
- A broken layout must never prevent a project from opening.

Workspace persistence in this document applies to Sim;Phys only. Cross-domain
Workspace sharing is deliberately undecided.

## Preferences

Possible UI preferences include:

- global UI scale;
- text scale;
- panel density;
- animation and reduced-motion settings;
- contrast and accent color within the dark visual system;
- default Editor/View Workspace;
- View-state `Ask`, `Apply`, or `Discard` behavior;
- configurable shortcuts and input sensitivity.

The exact preference set is not decided. Preferences must not change scientific
state or domain results.

## Interaction Consistency

- Selection is synchronized between the viewport and Scene Objects browser.
- `F` frames the selected object from either location.
- `Ctrl+F` focuses the relevant search for the area under keyboard focus; the
  default Editor binding searches the Object Library.
- Middle mouse movement pans the active viewport.
- Editor shortcuts never execute structural edits in View Mode.
- Every panel must remain usable with keyboard focus and high-DPI scaling.

## Boundaries

- UI layout is presentation state.
- A panel reads application/domain read models and emits typed intents.
- Hiding a warning panel does not disable validation or diagnostics.
- Workspace switching does not mutate canonical simulation state.
- Sim;Phys defines physical meanings, properties, and available actions; it
  does not place pixels or own panel geometry.
- Sim;Engine renders the resulting interface but does not decide UI policy.

## Open Questions

1. Is bounded docking sufficient for the first public version, or is arbitrary
   split/join a required identity feature?
2. Which optional area should occupy the lower-right Editor slot by default
   when the Inspector is short?
3. Should View Tools use a left shelf, a radial menu, or both?
4. Which keyboard shortcuts should be fixed product conventions and which may
   be rebound?
5. May an object appear in multiple organizational folders later, or should the
   browser remain a simple single-parent tree?
