# Contributing

## Branch policy

- Work from short-lived feature branches opened against `main`.
- Keep commits focused and aligned with a single plan step or milestone.

## Run tests

```bash
cargo test --workspace
```

## Testing guide

See [docs/testing.md](docs/testing.md) for the full testing guide, including:
- Test levels (L1a unit, L1b widget, L1c snapshot, L1d demo integration)
- How to update snapshot baselines
- How to add new demo tests

## Build WASM

```bash
cd crates/viewkai-web && trunk build --release
```

Use `viewkai-app` for native app validation, use `viewkai-web` for browser validation, and keep library crates free of app-only dependencies.

## Further reading

- [docs/architecture.md](docs/architecture.md) — System architecture and crate dependencies
- [docs/testing.md](docs/testing.md) — Test levels, snapshot workflow, adding new tests
- [docs/coding-style.md](docs/coding-style.md) — Naming, module organization, unsafe policy, doc comments
- [docs/dependency-policy.md](docs/dependency-policy.md) — How to add deps, crate boundaries, pinning strategy
- [docs/error-handling.md](docs/error-handling.md) — Error types, `.unwrap()` rules, pdfium error mapping
