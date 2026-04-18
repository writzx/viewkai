# Contributing

## Branch policy

- Work from short-lived feature branches opened against `main`.
- Keep commits focused and aligned with a single plan step or milestone.

## Run tests

```bash
cargo test --workspace
```

## Build WASM

```bash
wasm-pack build crates/viewkai --target web
```

Use the demo crate for native app validation and keep library crates free of app-only dependencies.
