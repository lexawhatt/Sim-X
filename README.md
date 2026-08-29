# Sim;X

Sim;X is a modular 2D scientific simulation platform. The current development
scope is deliberately narrow: one vertical slice in
`Sim;Phys -> Phys;Mechanics`.

Before changing code, read:

1. [`docs/READ_FIRST_SIM_X_BOUNDARIES_AND_CODE_STYLE.md`](docs/READ_FIRST_SIM_X_BOUNDARIES_AND_CODE_STYLE.md)
2. [`docs/COMPOSITION.md`](docs/COMPOSITION.md) for domain or simulation work
3. [`docs/Structure.md`](docs/Structure.md)
4. [`docs/ROADMAP.md`](docs/ROADMAP.md)

The complete documentation index is in [`docs/README.md`](docs/README.md).
The extension policy is in
[`docs/EXTENSIBILITY.md`](docs/EXTENSIBILITY.md).

## Development Commands

```text
cargo run
cargo test --all-targets
cargo clippy --all-targets --all-features -- -D warnings
```

The local Sim;Engine reference is stored in
[`docs/DOCUMENTATION.md`](docs/DOCUMENTATION.md).
