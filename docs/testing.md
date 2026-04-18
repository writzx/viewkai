# Testing Guide

This document describes viewkai's testing strategy and how to work with tests.

## Test Levels

| Level | Location | Purpose |
|---|---|---|
| **L1a — Pure-logic unit tests** | Inline `#[cfg(test)]` in `viewkai-core`, `cache.rs`, `viewport.rs`, `zoom.rs` | Coord math, cache semantics, zoom state math |
| **L1b — Library widget tests** | `crates/viewkai/tests/` via `egui_kittest::Harness::new_ui_state` | Viewer rendering states, API surface (zoom, scroll, clear) |
| **L1c — Library snapshot regression** | `crates/viewkai/tests/snapshots/` via `harness.snapshot()` | Visual output of key viewer states |
| **L1d — Demo eframe integration** | `crates/viewkai-demo/tests/` via `egui_kittest::Harness::new_eframe` | Keyboard shortcuts, zoom toolbar, page-jump, DemoLoadState transitions |
| **L1e — Benchmarks** | `crates/viewkai/benches/` via criterion | Parsing and rasterization performance |
| **L2 — Manual interactive** | `cargo run -p viewkai-demo` | Real rendering, pinch-to-zoom, native file dialog |
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

# Run only demo tests
cargo test -p viewkai-demo

# Run with output (verbose)
cargo test -p viewkai -- --nocapture
```

## Snapshot Tests

Snapshots live in `crates/viewkai/tests/snapshots/` (library) and `crates/viewkai-demo/tests/snapshots/` (demo).

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

## Testing the Demo (L1d)

The demo crate (`viewkai-demo`) is structured as a lib+bin crate:
- `src/lib.rs` — `DemoApp`, `DemoLoadState`, `run_native`, `run_web` (all public)
- `src/main.rs` — thin binary entry point

Integration tests in `crates/viewkai-demo/tests/` use `egui_kittest::Harness::new_eframe()` wrapping the real `DemoApp`.

### Adding New Demo Tests

See `crates/viewkai-demo/tests/common/mod.rs` for shared harness helpers:
- `demo_harness()` — fresh DemoApp, no PDF loaded
- `demo_harness_with_hello()` — DemoApp with hello.pdf loaded
- `demo_harness_with_500page()` — DemoApp with 500page.pdf loaded

```bash
# Run demo tests specifically
cargo test -p viewkai-demo

# Run a specific test
cargo test -p viewkai-demo -- shortcut_ctrl_0_resets_zoom
```
