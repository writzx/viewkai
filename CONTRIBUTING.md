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
wasm-pack build crates/viewkai --target web
```

Use the demo crate for native app validation and keep library crates free of app-only dependencies.
