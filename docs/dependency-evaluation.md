# Dependency Evaluation — Phase 0.4

Date: 2026-04-26

Scope: evaluate `egui`, `pdfium-render`, and `egui-phosphor` for the `viewkai` v0.2.1 patch release.

## Inputs reviewed

- `viewkai/Cargo.toml`
- `viewkai/crates/viewkai/Cargo.toml`
- `viewkai/Cargo.lock`
- `viewkai/docs/dependency-policy.md`
- crates.io metadata via `cargo search` and `crates.io` API

## Current vs latest

| Dependency | Workspace / crate requirement | Resolved in `Cargo.lock` | Latest on crates.io | Assessment |
|---|---:|---:|---:|---|
| `egui` | `0.34` | `0.34.1` | `0.34.1` | Already at latest patch release. |
| `eframe` | `0.34` | `0.34.1` | `0.34.1` via `egui` family cadence | Already aligned with current `egui` patch. |
| `pdfium-render` | `0.9` | `0.9.0` | `0.9.0` | Already at latest published release. |
| `egui-phosphor` | not yet used | n/a | `0.12.0` | Candidate for Phase B only; compatibility checked below. |

## API change assessment

### `egui`

- The manifest uses `egui = "0.34"`, which already resolved to `0.34.1` in `Cargo.lock`.
- The latest crates.io release is also `0.34.1`.
- Result: there is no newer compatible patch release to adopt, and no API delta to absorb for v0.2.1.

### `pdfium-render`

- The workspace uses `pdfium-render = { version = "0.9", features = ["thread_safe"] }`.
- The lockfile already resolves this to `0.9.0`.
- The latest crates.io release is still `0.9.0`.
- Result: there is no newer compatible release to evaluate for the patch train, so there is no API churn risk to take on.

## `egui-phosphor` v0.12 compatibility check

- crates.io dependency metadata for `egui-phosphor` `0.12.0` declares:
  - `egui = ^0.34`
  - `eframe = ^0.34` (dev dependency)
- That matches the current `viewkai` workspace line (`egui` / `eframe` `0.34`, resolved as `0.34.1`).
- Conclusion: `egui-phosphor` `0.12.0` is compatible with the current `egui` generation and should be viable for Phase B without first upgrading `egui`.

## Decision

### `egui`: defer manifest change

Rationale:

- No newer compatible version exists beyond the already-resolved `0.34.1` patch.
- Changing `Cargo.toml` from `0.34` to `0.34.1` would not change the resolved dependency graph in practice.
- Per patch-release policy, we should avoid no-op manifest churn when it provides no runtime or API benefit.

### `pdfium-render`: defer manifest change

Rationale:

- No newer compatible version exists beyond the already-resolved `0.9.0`.
- A patch-release dependency bump is only justified when there is a real newer compatible release with minimal API impact.
- That condition is not met here.

## Recommendation for Plan 03.25

- Do **not** modify `Cargo.toml` for `egui` or `pdfium-render` in v0.2.1.
- Record this phase as **evaluated and deferred**, because the workspace is already effectively on the latest compatible published versions.
- Proceed with later phases assuming:
  - `egui` / `eframe` remain on the `0.34.x` line
  - `pdfium-render` remains on `0.9.0`
  - `egui-phosphor` `0.12.0` is compatible with the current `egui` line for Phase B
