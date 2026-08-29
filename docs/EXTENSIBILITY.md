# Sim;X Extensibility Policy

Status: **accepted architecture decision**.

Sim;X does not currently have, promise, or design around a Lua plugin system.
It also does not expose native dynamically loaded Rust plugins. Extensibility
is split by the kind of change being made instead of putting every change
behind one generic plugin API.

## 1. Decision

The supported direction has three levels:

1. **Custom Objects and recipes** for user-authored compositions that use
   existing domain behavior.
2. **First-party Rust rule packs** for new scientific behavior maintained and
   compiled as part of Sim;X.
3. **A possible sandboxed external extension format later**, only if concrete
   user requirements cannot be expressed through the first two levels.

The third level has no selected language or runtime. WASM may be investigated
later, but it is not an architectural commitment.

## 2. Why Lua Is Not the Current Direction

A Lua runtime would make execution portable and embeddable, but it would also
force Sim;X to stabilize a second, dynamically typed scientific API before the
native Mechanics model is proven. Every exposed value would require bindings,
unit validation, lifecycle rules, versioning, deterministic scheduling,
resource limits, diagnostics, and migrations.

That cost is not justified by the current vertical slice. Keeping an unused
Lua boundary in every domain API would distort the native design around a
hypothetical consumer.

Therefore:

- no Lua runtime is selected;
- no Lua bindings or manifests are planned;
- no domain port is generalized merely for future Lua access;
- no Lua-specific directory remains in the source architecture;
- references to Lua in the upstream Sim;Engine documentation do not create a
  Sim;X requirement.

## 3. Why Native Dynamic Rust Plugins Are Also Rejected

First-party code is written in Rust, but that does not imply a native plugin
ABI.

Dynamically loading third-party Rust libraries would introduce:

- compiler and dependency-version coupling;
- no stable Rust ABI for ordinary types and traits;
- platform-specific loading and packaging;
- in-process access without a strong security boundary;
- crash and memory-safety consequences for the complete application;
- difficult deterministic replay and compatibility guarantees;
- migrations tied to internal implementation details.

Sim;X does not expose internal Rust traits, ECS storage, `MechanicsWorld`, GPU
objects, or application services as a third-party binary interface.

## 4. Level One: Custom Objects and Recipes

This is the primary user extension mechanism.

A Custom Object stores a validated graph of domain entities, canonical
components, relationships, and product metadata. A recipe constructs such a
graph from existing rules. Neither mechanism introduces executable code or a
new physical primitive.

Examples:

- a pendulum assembled from an anchor, body, constraint, and gravity;
- a reusable machine assembled from existing physical behaviors;
- a categorized object saved from a Shift-drag selection;
- a built-in object shipped by Sim;X using the same composition model.

Custom Objects must obey `COMPOSITION.md`. They cannot bypass compatibility,
units, numeric validation, determinism, or atomic graph transactions.

This level should be developed before external code execution because it can
cover many apparent plugin use cases with a safer, inspectable, serializable
format.

## 5. Level Two: First-Party Rust Rule Packs

New scientific behavior is initially added as reviewed Rust source compiled
with Sim;X. “Rule pack” describes ownership and organization; it is not a
runtime-loaded binary format.

A first-party rule pack may add domain-owned:

- components and their invariants;
- capability projections;
- relationships and endpoint semantics;
- interaction rules and deterministic schedule phases;
- commands, events, snapshots, and diagnostics;
- persistence schemas and migrations;
- built-in recipes and catalog metadata;
- headless scientific and determinism tests.

It must not receive unrestricted authority merely because it is written in
Rust. It follows the same domain boundaries as the rest of the repository.

For Phys;Mechanics, every new interaction rule must satisfy the rule contract
and implementation gates in `COMPOSITION.md`. Other domains define their own
contracts when their vertical slices begin.

### 5.1 Packaging

The default remains one modular Cargo package. A rule pack becomes a separate
workspace crate only when a real extraction trigger exists, such as build-time
isolation or independent first-party reuse. Crate extraction does not make it
a public plugin ABI.

### 5.2 Release behavior

First-party rule packs ship with a Sim;X release. This allows one tested set of
domain schemas, numerical rules, UI adapters, and migrations. Adding behavior
currently requires rebuilding and releasing Sim;X; that is an accepted tradeoff
until external programmability proves necessary.

## 6. Level Three: Possible Sandboxed External Extensions

External executable extensions are deferred, not secretly implemented through
another interface.

Investigation begins only when there are concrete examples of user-authored
behavior that:

- cannot be represented as a Custom Object or recipe;
- should not reasonably become a first-party rule pack;
- have a clear scientific execution model;
- justify a long-lived public compatibility contract.

A future design must begin from a narrow deterministic boundary resembling:

```text
validated immutable inputs
+ fixed simulation context
+ explicit deterministic state
-> typed contributions
+ typed events
+ structured errors
```

It must not begin from “give user code access to the world.”

### 6.1 Minimum requirements for any future runtime

Before selecting a language or runtime, the proposal must define:

- which domain owns each exposed concept;
- typed units and schema versioning;
- deterministic scheduling and ordering;
- explicit state and seeded randomness;
- CPU, memory, and output budgets;
- filesystem, network, clock, thread, and process policy;
- failure isolation and atomic rollback;
- serialization and migration rules;
- capability/permission declaration;
- supported platforms and distribution;
- diagnostics understandable without inspecting the runtime;
- compatibility tests across supported Sim;X versions.

Direct renderer ownership, raw pointers, unrestricted filesystem/network
access, ambient randomness, wall-clock-dependent physics, and direct mutable
domain storage are prohibited.

### 6.2 WASM status

WASM is only a possible research candidate because it may provide a portable
sandbox and a versionable component boundary. It is not selected, scheduled,
or reflected in current domain APIs. Sim;X will not design speculative WASM
interfaces before a real extension use case exists.

## 7. API and Schema Versioning Still Matter

Removing plugins does not remove versioning. Sim;X still needs versions for:

- project and scene formats;
- Custom Object definitions;
- domain component and relationship schemas;
- built-in recipe revisions;
- migrations between supported releases;
- renderer-adapter compatibility where relevant.

These versions describe persisted or explicit public data. They are not a
promise that internal Rust types form a stable third-party ABI.

## 8. Boundary Summary

```text
User composition
    -> Custom Object / recipe
    -> validated domain graph transaction

New official scientific behavior
    -> first-party Rust rule pack
    -> reviewed domain contracts and tests
    -> compiled with Sim;X

Future external behavior
    -> no current implementation
    -> requires a separate approved specification
```

## 9. Reconsideration Triggers

This decision may be revisited when at least one concrete, valuable scenario
requires executable third-party behavior and cannot be served by composition
or an official rule pack. A reconsideration must compare the real scenario
against implementation cost, scientific validation, determinism, security,
distribution, and long-term migration burden.

“Plugins would be nice someday” is not sufficient evidence to introduce a
runtime or public ABI.
