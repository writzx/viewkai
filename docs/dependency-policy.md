# Dependency Policy

## Hard crate boundaries (from the four architectural invariants)

| Crate | May depend on |
|---|---|
| `viewkai` | `egui`, `viewkai-core`, `viewkai-engine` (never `eframe`, never pdfium types) |
| `viewkai-core` | `thiserror`, `serde` (never `egui`, `eframe`, `pdfium-render`) |
| `viewkai-engine` | `pdfium-render`, `viewkai-core` (never `egui`, `eframe`) |
| `viewkai-demo` | All of the above + `eframe`, `rfd`, `ehttp` |

Violations are caught by CI jobs `purity-viewkai`, `purity-engine`, `purity-core`.

## How to propose a new dependency

PR description must include:
1. **Alternatives considered** — at least two alternatives (including "hand-roll") with LOC estimates
2. **WASM compatibility** — confirmation the dep builds on `wasm32-unknown-unknown` or a justification for a native-only dep behind `[target.'cfg(not(target_arch = "wasm32"))'.dependencies]`
3. **License check** — license in the allow-list in `deny.toml`; advisory check via `cargo deny check`

## Pinning strategy

- Workspace-shared deps (`egui`, `pdfium-render`, `eframe`, `thiserror`, `serde`, `serde_json`, `rfd`, `ehttp`) are pinned via `[workspace.dependencies]` in the root `Cargo.toml`
- Individual crate deps reference them with `{ workspace = true }` — no version duplication
- No `*` wildcards (enforced by `[workspace.lints.clippy] cargo = "warn"`)
- Dev-only deps (e.g., `egui_kittest`, `criterion`) stay in `[dev-dependencies]` only

## Unused dependency detection

`cargo machete` runs in CI (`machete` job). If it flags a dep:
- Confirm with `rg <crate_name> src/ tests/` that it's truly unused
- If used only in integration tests, add to `[package.metadata.cargo-machete] ignored = ["<crate>"]` with a comment pointing at the test file
- If genuinely unused, remove it
