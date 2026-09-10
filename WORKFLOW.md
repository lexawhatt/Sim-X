# Sim;X Active Workflow

Purpose: short-lived operational memory for the current implementation run.
This is the only workflow tracker. Normative product and architecture decisions
remain in `docs/`; this file links to them instead of replacing them.

Last updated: 2026-09-02

## Active State

**Development resumed against published Sim;Engine `0.2.0`.** The crates.io
archive, docs.rs build, GitHub release/tag, and local source all resolve to git
commit `906e4464904bbdd871b8cdd151490443f2254e9d`; the archive checksum is
`0e0867c3ef3e22880d79d0f5bfbdc2179b3e671f7c835a0c1edb220a2931cdea`.

The local `/home/lexa/Desktop/Rust/Sim-Engine/engine` directory is only an
untracked, byte-identical copy of the three release documents. The authoritative
local source is the Sim;Engine repository root and `src/` at the commit above.

The current dirty worktree is the owned implementation baseline. Do not reset,
checkout, discard, or normalize unrelated files.

The first four-subdomain Red Review rejected the milestone. Its findings and
the complete remediation ledger are preserved in
`docs/reviews/RED_TEAM_PHYS_4_OF_7.md`. The fresh external read-only review accepts the
remediated Gate 0-2 scientific and architecture scope and the Sim;Phys 4/7
prototype with no unresolved Critical, High, or Medium finding.

## Current Objective

Preserve the accepted four-subdomain prototype while organizing project
documentation and bundled media, and record the provisional public-source plus
Demo/Full distribution direction without pretending a software license has
already been selected.

Implementation status: the defined 4/7 breadth prototype and its Gate 0-2
scientific/architecture scope are externally accepted. This is not a claim
that all of Physics, the three locked subdomains, or full authoring are done.

For this run, "50% of Sim;Phys" means **4/7 subdomains with coherent minimum
vertical slices**, not 50% of all possible physics knowledge. A subdomain counts
only if it has:

1. a written numerical/API contract;
2. renderer-neutral typed canonical state;
3. deterministic bounded and atomic stepping/evaluation;
4. headless tests, including invalid/extreme inputs;
5. an app-owned runtime adapter;
6. a reachable
   `Main Menu -> Domains -> Sim;Phys -> Subdomain -> Projects -> Editor -> View`
   UI path where Editor is no-step and View displays a live immutable snapshot;
7. no scientific dependency on presentation, Sim;Engine, or wall time.

Target counted slices:

- [x] Phys;Mechanics — `F = ma` translational slice.
- [x] Phys;Thermodynamics — lumped thermal bodies and bounded conductive heat
  exchange toward equilibrium.
- [x] Phys;Waves & Optics — deterministic 1D wave field slice with explicit
  boundary conditions and bounded samples.
- [x] Phys;Electromagnetism — electrostatic point charges and electric-field
  probe evaluation with singularity policy.

Relativity, Fluid Dynamics, and Phys;Sandbox remain visible but do not count
until they satisfy the same gate. Sandbox cannot be faked before compatible
capabilities from multiple Phys subdomains exist.

## What Is Happening Now

1. Treat the accepted Sim;Phys 4/7 prototype as the implementation baseline.
2. Keep project contracts grouped under architecture, product, domain,
   integration, review, and reference directories.
3. Keep compiled media under `assets/`, never as loose repository-root files.
4. Defer the real-font migration until the user resumes it; no font dependency
   or code change was introduced during the interrupted investigation.
5. Do not publish Demo or Full builds until the software-license choice and
   every bundled media right have been resolved.

## Immediate Next Actions

- [x] Identify and independently verify Sim;Engine `0.2.0` provenance.
- [x] Read its release notes, online documentation, and local source needed for
  migration.
- [x] Update `docs/integrations/sim_engine/DOCUMENTATION.md` to the exact integrated engine contract.
- [x] Compare every `SIM_ENGINE_RENDERING_GAPS.md` item with verified new API
  behavior.
- [x] Plan the adapter migration before changing dependencies or source.
- [x] Finish presentation migration without changing scientific ownership
  boundaries: four independently budgeted visual layers share one bounded
  `FrameComposer`, and metric canvases use a separate scientific scene/camera.
- [x] Integrate bounded typed surface-loss recovery with retained icon-atlas
  restoration and an exact one-frame retry from the same immutable snapshot.
- [x] Remediate the rejected milestone findings in
  `docs/reviews/RED_TEAM_PHYS_4_OF_7.md` with code, specifications, and regressions.
- [x] Replace misleading Editor affordances with real Mechanics commands or
  clearly locked/read-only controls before the showcase gate.
- [x] Request a fresh Red Review after migration and remediation.
- [x] Receive acceptance of Gate 0-2 and the four-subdomain prototype with no
  unresolved Critical, High, or Medium finding.
- [x] Launch the native all-feature build and visually inspect a clean
  borderless 1920x1080 full-screen Main Menu with the three social icons.
- [x] Sort project documentation into purpose-owned directories and verify all
  Markdown link targets.
- [x] Move the MP3 and SVG assets out of the repository root, update their
  compile-time paths, and pass the complete all-feature test suite.
- [x] Record the provisional public-source, itch.io Demo/Full, symbolic-price,
  and Buy Me a Coffee direction in
  `docs/product/DISTRIBUTION_AND_MONETIZATION.md`.

## Non-Negotiable Boundaries to Reconfirm

- No named-object branches in solvers; capabilities and interactions own
  scientific applicability.
- UI emits intents and reads immutable snapshots; it never owns canonical
  scientific state.
- Wall time is converted to bounded fixed steps only in `app`.
- Every externally sized collection has a hard work/memory/event budget checked
  before unbounded work.
- Finite is not sufficient: representational no-progress and unsafe arithmetic
  require structured outcomes.
- Other domains do not inherit the Physics editor UI.
- Renderer-only Sim;Engine limitations go only in
  `docs/integrations/sim_engine/SIM_ENGINE_RENDERING_GAPS.md`.
- No Lua or executable plugin runtime; Custom Objects and first-party Rust rule
  packs remain the accepted direction.
- A public repository is not a license. Do not promise compilation,
  redistribution, or commercial-use rights until an explicit root license or
  EULA is accepted.
- The current easter-egg music is development-only for release planning unless
  commercial redistribution rights are documented.

## Reread Notes

Complete on 2026-08-29. Confirmed constraints:

- the newest user decision supersedes the old Mechanics-only roadmap scope;
- every code-writing series begins by rereading
  `docs/READ_FIRST_SIM_X_BOUNDARIES_AND_CODE_STYLE.md`;
- the three new subdomains must start independent and renderer-neutral;
- no subdomain imports another subdomain; future Phys;Sandbox bridges belong
  in app until an explicit shared contract is proven;
- fundamental constants live in a versioned registry and formulas read them;
- UI policy and wall time remain outside scientific state;
- fixed collection/work/event budgets and atomic failure are required;
- Physics UI ideas may be shared inside Sim;Phys only after concrete consumers;
- the old Mechanics-only statements in the documentation must be updated before
  new scientific code is treated as authorized.

Resolved conflict: the four-subdomain milestone is now the active scope. The
Mechanics composition contract remains Mechanics-only and is not generalized.

## Verification Ledger

- Current candidate: all four counted slices have focused contracts, bounded
  cores, app-owned sessions, and reachable Editor/View paths. Seven honest
  built-in project paths are available: four primary labs plus secondary
  Thermal, Wave, and Electrostatic labs; Mechanics Pendulum and Spring remain
  locked until their constraints exist.
- 2026-09-02 frozen candidate gate: formatting and `git diff --check` passed;
  `cargo test --all-features --all-targets` passed 174 unit tests and 8
  architecture tests; `cargo test --no-default-features --all-targets` passed
  110 core tests and the same 8 architecture tests. All-feature and core-only
  strict Clippy, missing-docs rustdoc, and Rust 1.90 checks passed. The
  no-default dependency tree contains only Sim;X. The pinned exact
  resolved-module graph check also passed.
- 2026-09-02 external verdict: Gate 0-2 and the Sim;Phys 4/7 prototype are
  accepted. No unresolved Critical, High, or Medium finding was reproduced.
  The Red Team independently repeated the complete gate above. The remaining
  read-only Thermal, Wave, and Electrostatic Editors are explicit non-blocking
  scope limits.
- 2026-09-02 native smoke: `cargo run --all-features` opened the expected dark
  borderless 1920x1080 full-screen Main Menu. The title, Domains/Settings/Quit
  actions, shortcut hints, hidden-help marker, and retained GitHub/YouTube/
  Telegram icons rendered cleanly. Native input automation was unavailable
  because the running KWin compositor does not expose the virtual-keyboard
  protocol; navigation and Editor/View transitions remain covered by tests.
- 2026-09-02 organization pass: all project-level Markdown links resolve; root
  media moved to `assets/audio/` and `assets/icons/`; all-feature verification
  still passes 174 unit tests plus 8 architecture tests, including MP3 decode
  and exact SVG-atlas construction.

## Resume Rule

After interruption or context compaction:

1. read this file;
2. inspect `git status --short` without discarding user changes;
3. confirm that the new Sim;Engine version and source are actually available;
4. read the new engine release notes, online documentation, local source, and
   `docs/integrations/sim_engine/SIM_ENGINE_RENDERING_GAPS.md`;
5. read `docs/reviews/RED_TEAM_PHYS_4_OF_7.md` so scientific findings are not hidden by
   renderer migration;
6. inspect the dependency and adapter diff before making changes;
7. read `docs/product/DISTRIBUTION_AND_MONETIZATION.md` before changing
   licensing, edition boundaries, store behavior, or bundled release assets;
8. continue the first unchecked immediate action;
9. update this file after any material decision or completed gate.
