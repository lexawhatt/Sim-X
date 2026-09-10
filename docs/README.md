# Sim;X Documentation

This directory is the canonical home of project-level design, architecture,
domain contracts, integration records, and external reviews. Local README files
inside `src/` describe source-module ownership and do not override these
contracts.

## Start Here

1. [`READ_FIRST_SIM_X_BOUNDARIES_AND_CODE_STYLE.md`](READ_FIRST_SIM_X_BOUNDARIES_AND_CODE_STYLE.md)
   - mandatory repository boundaries and code style.
2. The focused contract under [`domains/`](domains/) for the domain being
   changed.
3. [`architecture/COMPOSITION.md`](architecture/COMPOSITION.md) for Mechanics
   entities, capabilities, relationships, interactions, or saved compositions.
4. [`architecture/Structure.md`](architecture/Structure.md) for ownership and
   dependency direction.
5. [`product/ROADMAP.md`](product/ROADMAP.md) for authorized implementation
   order.
6. [`product/Implementation.md`](product/Implementation.md) for the
   specification-first procedure.

For user-authored content, rule packs, or executable extension proposals, also
read [`architecture/EXTENSIBILITY.md`](architecture/EXTENSIBILITY.md).

## Current Scope

The accepted prototype contains coherent minimum slices for four of the seven
visible Sim;Phys subdomains: Mechanics, Thermodynamics, Waves and Optics, and
Electromagnetism. This is not a claim that half of scientific Physics is
modeled. Relativity, Fluid Dynamics, Phys;Sandbox, other top-level domains, and
universal cross-domain composition remain outside the completed milestone.

## Directory Map

### Architecture

- [`architecture/COMPOSITION.md`](architecture/COMPOSITION.md) - normative
  Mechanics composition model.
- [`architecture/EXTENSIBILITY.md`](architecture/EXTENSIBILITY.md) - Custom
  Objects and first-party Rust rule-pack policy.
- [`architecture/Structure.md`](architecture/Structure.md) - module ownership,
  dependency map, and repository structure.

### Product

- [`product/Brainstorm.md`](product/Brainstorm.md) - accepted product decisions
  and unresolved questions.
- [`product/UI.md`](product/UI.md) - Physics-specific Editor and View direction.
- [`product/ROADMAP.md`](product/ROADMAP.md) - staged implementation plan.
- [`product/Implementation.md`](product/Implementation.md) - implementation
  rules and specification template.
- [`product/DISTRIBUTION_AND_MONETIZATION.md`](product/DISTRIBUTION_AND_MONETIZATION.md)
  - public source, Demo/Full, itch.io, support, licensing, and asset-rights
  direction.
- [`product/Ideas.md`](product/Ideas.md) - non-normative idea inventory.

### Domain Contracts

Sim;Phys contracts live together under [`domains/phys/`](domains/phys/):

- [`MECHANICS_FMA.md`](domains/phys/MECHANICS_FMA.md);
- [`PHYSICS_EDITOR_COMMANDS.md`](domains/phys/PHYSICS_EDITOR_COMMANDS.md);
- [`THERMODYNAMICS_CONDUCTION.md`](domains/phys/THERMODYNAMICS_CONDUCTION.md);
- [`WAVES_1D.md`](domains/phys/WAVES_1D.md);
- [`ELECTROSTATICS.md`](domains/phys/ELECTROSTATICS.md).

### Sim;Engine Integration

- [`DOCUMENTATION.md`](integrations/sim_engine/DOCUMENTATION.md) - exact local
  Sim;Engine 0.2.0 reference.
- [`SIM_ENGINE_RENDERING_GAPS.md`](integrations/sim_engine/SIM_ENGINE_RENDERING_GAPS.md)
  - renderer-only historical gaps and current integration ledger.
- [`SIM_ENGINE_WISHLIST.md`](integrations/sim_engine/SIM_ENGINE_WISHLIST.md) -
  proposed renderer roadmap and acceptance gates.

### External Reviews

- [`RED_TEAM_GATE_0_2.md`](reviews/RED_TEAM_GATE_0_2.md) - original Mechanics
  Gates 0-2 review and remediation packet.
- [`RED_TEAM_PHYS_4_OF_7.md`](reviews/RED_TEAM_PHYS_4_OF_7.md) - round 1
  rejection, remediation ledger, and final accepted four-subdomain verdict.

### Historical Reference

- [`reference/ui_demo_sim_engine_0_1.txt`](reference/ui_demo_sim_engine_0_1.txt)
  - archived non-compiling Sim;Engine 0.1 UI example.

## Authority of Documents

When documents disagree, use this priority:

1. the currently approved user decision;
2. the mandatory boundary document;
3. the relevant normative domain contract;
4. architecture and roadmap;
5. product brainstorm and UI direction;
6. non-normative ideas and examples.

Do not silently resolve a meaningful contradiction in code. Record the
decision in the appropriate document first.
