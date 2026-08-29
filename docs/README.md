# Sim;X Documentation

This directory is the canonical home of project-level design, architecture,
product, and integration documentation. README files inside `src/` describe
the responsibility of their local module; they do not override these project
contracts.

## Current Scope

Only the following vertical slice is authorized for implementation:

```text
Sim;Phys
└── Phys;Mechanics
```

The documents may record future ideas, but an idea is not an implementation
commitment. Sim;Math, Sim;Chem, Sim;Biol, other Physics subdomains, and a
cross-domain composition framework remain outside the active slice unless the
roadmap explicitly promotes them.

## Mandatory Reading Order

1. [`READ_FIRST_SIM_X_BOUNDARIES_AND_CODE_STYLE.md`](READ_FIRST_SIM_X_BOUNDARIES_AND_CODE_STYLE.md)
   — repository-wide boundaries and code style.
2. [`COMPOSITION.md`](COMPOSITION.md)
   — normative entity, capability, relationship, interaction, determinism,
   and saved-composition contract for Phys;Mechanics.
3. [`Structure.md`](Structure.md)
   — module ownership and dependency direction.
4. [`ROADMAP.md`](ROADMAP.md)
   — authorized order of implementation.
5. [`Implementation.md`](Implementation.md)
   — specification-first implementation procedure.

For any work involving user-authored content, new rule packs, or executable
extensions, also read [`EXTENSIBILITY.md`](EXTENSIBILITY.md).

## Product Documents

- [`Brainstorm.md`](Brainstorm.md) — accepted product decisions and unresolved
  product questions.
- [`UI.md`](UI.md) — Physics-only editor and View-mode direction.
- [`Ideas.md`](Ideas.md) — broad concept and idea inventory; not normative.

## Architecture Policies

- [`EXTENSIBILITY.md`](EXTENSIBILITY.md) — accepted policy replacing the old
  Lua plugin direction with Custom Objects and first-party Rust rule packs.

## Technical Reference

- [`DOCUMENTATION.md`](DOCUMENTATION.md) — local Sim;Engine v0.1.0 integration
  and API reference. The online crate documentation can differ from this
  local copy, so integration work must compare both when relevant.

## Authority of Documents

When documents disagree, use this priority:

1. the currently approved user decision;
2. the mandatory boundary document;
3. the relevant normative domain contract such as `COMPOSITION.md`;
4. architecture and roadmap;
5. product brainstorm and UI direction;
6. non-normative ideas and examples.

Do not silently resolve a meaningful contradiction in code. Record the
decision in the appropriate document first.
