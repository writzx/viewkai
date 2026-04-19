# Testing Guide

This document describes viewkai's testing strategy and how to work with tests.

## Test Levels

| Level | Location | Purpose |
|---|---|---|
| **L1a — Pure-logic unit tests** | Inline `#[cfg(test)]` in `viewkai-core`, `cache.rs`, `viewport.rs`, `zoom.rs` | Coord math, cache semantics, zoom state math |
| **L1b — Library widget tests** | `crates/viewkai/tests/` via `egui_kittest::Harness::new_ui_state` | Viewer rendering states, API surface (zoom, scroll, clear) |
| **L1c — Library snapshot regression** | `crates/viewkai/tests/snapshots/` via `harness.snapshot()` | Visual output of key viewer states |
| **L1d — App eframe integration** | `crates/viewkai-app/tests/` via `egui_kittest::Harness::new_eframe` | Keyboard shortcuts, zoom toolbar, page-jump, `LoadState` transitions |
| **L1e — Benchmarks** | `crates/viewkai/benches/` via criterion | Parsing and rasterization performance |
| **L2 — Manual interactive** | `cargo run -p viewkai-app` / `cd crates/viewkai-web && trunk serve` | Native rendering, pinch-to-zoom, native file dialog, browser demo |
| **L3 — Integration gates** | GH Pages browser demo + Valeria | Stack-boots-in-real-environment |

## When to Write Which Test

- **Unit test (L1a)**: Pure logic with no egui dependency — math, state machines, data structures
- **Widget test (L1b)**: Testing `Viewer` API — load, zoom, scroll, state transitions. Fast; no snapshots
- **Snapshot (L1c)**: Visual regression for rendering changes. Expensive to maintain; use sparingly (5–7 total)
- **Demo integration (L1d)**: Keyboard shortcuts, UI interactions, eframe-specific behavior

## Running Tests

```bash
# Run all tests
cargo test --workspace

# Run only viewkai library tests
cargo test -p viewkai

# Run only native app tests
cargo test -p viewkai-app

# Run with output (verbose)
cargo test -p viewkai -- --nocapture
```

## Snapshot Tests

Snapshots live in `crates/viewkai/tests/snapshots/` (library) and `crates/viewkai-app/tests/snapshots/` (native app).

### Updating Snapshots

If a legitimate rendering change alters snapshots:
```bash
# Update all failing snapshots
UPDATE_SNAPSHOTS=true cargo test -p viewkai

# Force-update ALL snapshots (use with care)
UPDATE_SNAPSHOTS=force cargo test -p viewkai
```

1. Run with `UPDATE_SNAPSHOTS=true`
2. Inspect generated PNGs visually (check for correctness)
3. `git add crates/viewkai/tests/snapshots/*.png` — baselines only, NOT `.new.png`/`.diff.png` (gitignored)
4. Run again without the env var to confirm baselines pass

### Per-OS Thresholds

Snapshot comparison thresholds are in `kittest.toml` at repo root:
- **Linux CI**: `threshold = 0.6` (strict — source of truth)
- **macOS/Windows dev**: `threshold = 2.0` (lenient — font/driver variance expected)

Do NOT commit baselines generated on macOS — use Linux CI baselines as the source of truth.
The `egui_kittest` default OS is `OperatingSystem::Nix` for deterministic rendering.

### Files NOT to commit
`.gitignore` excludes: `*.new.png`, `*.diff.png`, `*.old.png` in snapshots directories.
Only the plain `*.png` baselines are committed.

## Testing the Native App (L1d)

The native app crate (`viewkai-app`) is structured as a lib+bin crate:
- `src/lib.rs` — `App`, `LoadState`, `run` (public)
- `src/main.rs` — thin binary entry point

Integration tests in `crates/viewkai-app/tests/` use `egui_kittest::Harness::new_eframe()` wrapping the real `App`.

### Adding New Native App Tests

See `crates/viewkai-app/tests/common/mod.rs` for shared harness helpers:
- `demo_harness()` — fresh App, no PDF loaded
- `demo_harness_with_hello()` — App with hello.pdf loaded
- `demo_harness_with_500page()` — App with 500page.pdf loaded

```bash
# Run native app tests specifically
cargo test -p viewkai-app

# Run a specific test
cargo test -p viewkai-app -- shortcut_ctrl_0_resets_zoom

## Testing the Web Demo

`viewkai-web` is a WASM-only crate. Build or serve it from its crate directory:

```bash
cd crates/viewkai-web
trunk build --release
```
```
