# Sim;X Product Brainstorm

Status: product concept and brainstorming record. This document does not
authorize implementation yet.

## Current Scope Boundary

The active product and implementation scope is the vertical
`Sim;Phys -> Phys;Mechanics` slice. Physics editor, View tools, panels, and
interaction decisions in this document must not be promoted into a universal
cross-domain UI framework.

Sim;Math, Sim;Chem, and Sim;Biol may require radically different interfaces.
Their concrete editors and View interactions will be designed only when their
own vertical slices begin. Mentions of their future phenomena below record the
product vision, not current UI requirements.

The detailed entity/capability/relationship model for the current Mechanics
slice is specified in `COMPOSITION.md`. This brainstorm records the product
experience; it does not authorize object-specific physics or a universal
cross-domain capability system.

## Domain Sandbox Concept

A Sandbox is a free-form laboratory for one top-level Sim;X domain. It allows a
user to construct a scene from all compatible subdomains of that domain and
observe domain phenomena directly through the resulting simulation.

There is one Sandbox per domain:

- Phys;Sandbox mixes Mechanics, Thermodynamics, Waves and Optics,
  Electromagnetism, Relativity, and Fluid Dynamics;
- Math;Sandbox mixes compatible mathematical subdomains;
- Chem;Sandbox mixes compatible chemical subdomains;
- Biol;Sandbox mixes compatible biological subdomains.

A Sandbox is not a global boundary-free world. Mixing between top-level
domains remains an explicit future product and architecture decision.

## Core Experience

The user freely builds a scene instead of selecting only a curated experiment.
For example, a Phys;Sandbox scene could contain an internal combustion engine
that drives a pendulum, activates magnets, generates electrical power, and
lights a lamp.

Sim;X provides useful built-in objects and assemblies, including its own
internal combustion engine. Custom Objects extend the library; users are not
expected to construct every useful machine from primitives.

The simulation scene itself is the primary explanation of events. A collision,
cell division, phase transition, chemical reaction, mathematical extremum, or
other phenomenon should be visible through the behavior of the scene rather
than requiring a separate event visualization layer.

The experience may be used as a serious scientific laboratory or as an
open-ended construction game. Scientific meaning has priority, but the visual
presentation and object-building experience should be intuitive and enjoyable.

## Editor Mode

Editor Mode owns scene construction. Simulation rewind and editor undo are
different concepts and must never be conflated.

- Objects and saved assemblies are placed with drag and drop.
- A normal click selects an object and opens its Inspector.
- Shift-drag creates a multi-object selection.
- The middle mouse button pans the editor view.
- Standard editing shortcuts behave conventionally, including undo, cut, copy,
  and paste.
- A selected assembly can be saved from the `Custom Objects` section.
- `Ctrl+F` focuses object search. Search spans built-in and Custom Objects in
  the currently available palette hierarchy.
- Camera zoom behavior is still to be specified.

Editor interaction state, selection, camera state, and visual placement helpers
are not canonical domain state.

## View Mode

View Mode runs and observes the constructed simulation.

- The user can play and pause.
- The user can slow down and speed up simulation time.
- Event-by-event stepping is not currently planned.
- Rewind is explicitly excluded from the Domain Sandbox.
- Rewind and world-line behavior may become part of Sim;Time in the future.
- The object Inspector is not shown in View Mode.
- Optional logs, causal explanations, measurements, and direct value displays
  may still be available.
- View Mode may provide domain-specific intervention tools without becoming an
  editor. In Phys, examples include a Hand tool for moving an object, a Force
  tool for applying force, and magnetic or other physical tools.
- The Phys Hand tool defaults to a physical interaction whose behavior can be
  configured by the user. Its optional `Free Mode` ignores object mass and
  permits unrestricted movement.
- Only Sim;Phys View tools are being specified now. Chemistry, Biology, and
  Mathematics tools are deferred and may use entirely different interaction
  models and layouts.

Any structural editing requires Editor Mode. A View tool changes the running
experiment through a validated domain action; it does not edit the object's
definition or silently mutate editor data.

When leaving View Mode, Sim;X asks whether the final simulation state should be
applied back to the editor:

```text
Apply This Simulation State to the Editor? [Y/N]
```

Product meaning:

- `Y` applies the final View state as the new Editor state;
- `N` discards the run and restores the state from before View Mode.

Preferences can choose `Ask every time`, `Apply`, or `Discard` so the user does
not need to answer the confirmation after every run.

## Object Palette Hierarchy

Each subdomain editor owns its own object categories. For example, Mechanics
may have categories such as `Basic Stuff`, constraints, machines, forces, and
other Mechanics-specific groups. The final category names are not decided.

A Domain Sandbox adds one hierarchy level:

```text
Phys;Mechanics editor
`-- category
    `-- object

Phys;Sandbox
`-- Mechanics
    `-- the same category
        `-- the same object
`-- Thermodynamics
    `-- thermodynamics category
        `-- thermodynamics object
`-- Electromagnetism
    `-- electromagnetism category
        `-- electromagnetism object
```

Therefore, a category from a subdomain becomes a subcategory under that
subdomain inside the Domain Sandbox. Sandbox does not duplicate a second object
catalog with different meanings.

Blank scenes are sufficient; starting scene templates are not planned. Built-in
objects and assemblies provide the convenient starting material instead.

## Connections and Natural Interfaces

The editor supports free connections, wires, constraints, contacts, and other
domain-appropriate interactions. Sim;X does not ask the user to declare a
second set of abstract Custom Object ports.

An assembly retains the meaningful interfaces of its constituent objects. An
engine already has its fuel path, electrical contacts, mechanical shaft, heat
behavior, and physical surfaces. Saving it as a Custom Object preserves those
real interfaces. They remain the places where other objects interact.

This follows the compositional idea demonstrated by Sebastian Lague's
Digital-Logic-Sim, where a saved chip description contains its inputs, outputs,
nested chips, and wires. Sim;X generalizes the idea from fixed logic pins to
typed domain capabilities and physical interaction surfaces.

## True World Scale

Conditional, normalized, and schematic object scale are removed from the
product concept. Displayed geometry represents world geometry.

A pendulum with a length of 1 km occupies 1 km in the simulated world. Sim;X
must make extreme scales usable through camera zoom, navigation, framing,
measurement, and level-of-detail rendering without visually shrinking the
object relative to its physical interactions.

The editor provides:

- `F` to frame the selected object;
- an adaptive world grid using `km -> m -> cm -> mm -> µm -> nm` as the camera
  zoom changes;
- `um` as an ASCII fallback for the micrometre symbol `µm` where required;
- a dedicated Scene Objects browser for selecting and framing objects;
- user-created folders inside the Scene Objects browser.

Scene Objects folders are organizational only. They do not create transform
parents, shared physical behavior, or simulation groups. Those relationships
must be created explicitly through the appropriate editor operation.

A minimap is not currently planned. The broader panel, workspace, and UI
customization direction is recorded in `UI.md`.

## Scientific Freedom and Safety

The Sandbox deliberately permits scientifically implausible experiments. A
user may set the speed of light to `10 m/s`, set the speed of sound to
`100 m/s`, or otherwise alter the assumed rules of the simulated world.
Fundamental and world constants apply to the complete scene. Per-object or
spatial regions with different fundamental constants are not planned.

Sim;X separates two validation levels:

1. Computational safety is mandatory. Non-finite values, undefined arithmetic,
   numeric overflow, corrupted state, and operations that cannot be evaluated
   safely are rejected or handled through an explicit safe outcome.
2. Scientific plausibility is advisory. Unusual constants, extreme scales, and
   physically implausible combinations produce a Scientific Warning but remain
   available when they can be computed safely.

Breaking conventional physics is a supported creative and educational use of
the Sandbox. A Scientific Warning informs the user; it is not a moral judgment
and normally is not a blocker.

## Breakage and Destruction

When a simulated object breaks, the scene shows the breakage. Its resulting
parts retain applicable properties and continue interacting with the running
simulation: they may collide, heat up, conduct, or otherwise participate when
their domain state permits it.

Fragments are transient simulation entities. They are not promoted to reusable
palette objects and cannot be taken back into Editor construction as newly
harvested components.

When returning to Editor Mode, transient fragments disappear. If the final View
state is applied and the source object was destroyed, the destroyed source does
not reappear as an editable object.

## Optional Causality

Displaying causal relationships is optional and user-controlled. When enabled,
it may explain chains such as an applied force changing acceleration, motion
causing a collision, or a temperature change triggering a phase transition.

The product form of this explanation is not decided. It may be an overlay, a
compact causal view, or part of the optional log. It must not obscure the scene
by default.

## Optional Detailed Log

The user may open a detailed log of domain phenomena. The log is hidden by
default and never required to understand the basic scene.

Example entries:

```text
Force N applied to object X
Object X moved 10 meters
Substance A changed from liquid to gas
Cell X divided into cells Y and Z
Function F reached a local maximum at X
```

Log entries must be derived from validated domain outcomes. The UI must not
infer scientific events from visual movement alone. Entries should use stable
object identities, explicit units, simulation timestamps, and domain-accurate
terminology.

The user chooses whether the log shows concise phenomena, detailed causal
information, diagnostics, or no log at all.

## Measurements

Sim;X supports both direct value display and scientific instruments. Instruments
may include rulers, timers, temperature sensors, oscilloscopes, voltmeters, and
domain-specific measurement objects.

The Inspector provides editable properties and values only in Editor Mode.
View Mode can expose measurements through instruments and optional scene
overlays without restoring the Editor Inspector.

## Custom Objects

A user creates a Custom Object by building an assembly, selecting it with
Shift-drag, opening the `Custom Objects` section, and saving the selection.

The user may assign a name and category. A Custom Object without an assigned
category is stored in `Uncategorized`.

Saving captures the constructed object or assembly as it currently exists. The
current design does not introduce a separate interface for exported Blueprint
parameters. A saved assembly may itself contain previously saved Custom
Objects; selecting and saving the complete result creates another Custom
Object.

A Custom Object can be opened and edited later. The saved library entry is its
definition; every copy placed into a scene is an instance of that definition.
The UI uses ordinary language such as "placed copies" instead of requiring the
user to know this terminology.

Editing a definition requires explicit confirmation. Each Custom Object has a
stable identifier and a definition revision:

- placed copies in the open scene update immediately after the definition is
  saved;
- closed project files are not rewritten in the background;
- when another project opens, it detects older referenced revisions and lazily
  updates its placed copies;
- an incompatible update must report what cannot be preserved and must never
  leave a partially migrated scene.

If reliable migration proves impractical for a specific change, Sim;X keeps the
older placed copy rather than destructively guessing.

A Custom Object preserves:

- its entities and current canonical values;
- its internal connections, constraints, and compatible effects;
- the original objects and natural interfaces through which it interacts;
- its owning domain requirements and Sim;X API version;
- its name, category, and preview.

Instantiating a Custom Object creates new scene identities and never shares
mutable canonical state with the saved source assembly.

Saving a Custom Object is explicit and immediate:

```text
Shift-drag selection -> choose name and optional category -> Saved
```

Custom Objects are expected to be reusable across scenes in their owning
domain. Cross-domain use, storage format, sharing, nested versioning, and
migration remain open design questions.

## Projects and Autosave

Projects contain editable scenes. The planned navigation hierarchy is:

```text
Main Menu -> Domain -> Subdomain -> Projects -> Editor
```

The Projects screen is always shown after subdomain selection. It allows the
user to create, open, name, organize, and remove projects for the selected
subdomain or Domain Sandbox. Sandbox follows the same path because it is a
Domain subdomain.

Projects are sorted by most recently used by default, with the latest project
first. Additional sorting options may be added without changing that default.

Projects autosave. Autosave must use atomic/recoverable storage so an
interrupted write does not destroy the last valid project state. The UI should
show save status and recovery information without interrupting ordinary
construction.

## Domain and Presentation Boundaries

- Each domain owns its phenomena, validation, calculations, and canonical
  state.
- The application layer composes compatible subdomain capabilities inside a
  Domain Sandbox.
- The UI emits construction and control intents; it does not mutate canonical
  entities directly.
- Sim;Engine displays bounded snapshots and never becomes the source of
  scientific truth.
- World geometry and displayed geometry use the same scale; camera and
  level-of-detail behavior solve extreme-view problems.
- Editor undo operates on editor commands; it does not rewind simulation time.
- Logging and Custom Objects must not bypass validation or compatibility rules.
- View tools are validated domain actions, not hidden editor mutations.
- World constants are scene-wide.

## Open Questions for the Next Brainstorm

1. When a Custom Object definition changes, which placed-copy properties are
   preserved: transform, current state, damage, and external connections?
2. Are nested Custom Objects stored by reference or captured into the new saved
   object?
3. What should happen when a Custom Object targets an older Sim;X API?
4. Are Custom Objects local only, exportable as files, or shareable through a
   future community library?
5. What exact causal view should be offered when the option is enabled?
6. How should high-frequency events be grouped so the optional log stays
   readable?
7. Which View tools and measurement tools belong to Phys;Mechanics first?
8. Which additional project sorting options are useful beyond recent-first?
9. What camera behavior is needed above kilometres and below nanometres?
