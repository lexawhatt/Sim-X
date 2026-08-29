# Domains

Each child directory is an independent Sim;X domain with its own canonical
state, rules, commands, read models, errors, and tests.

Domains may depend on `foundation`. They must not import sibling domains,
`app`, `presentation`, or `sim_engine`. Explicit cross-domain composition
belongs in `app`.

New official scientific behavior is added as a first-party Rust rule pack
inside its owning domain. This is not a public native plugin ABI.
