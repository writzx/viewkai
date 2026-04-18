# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

## [0.0.4] — 2026-04-18 — Testing Migration

### Changed
- Migrated tests from `egui::Context::default() + ctx.run()` to `egui_kittest::Harness`
- `viewkai` and `viewkai-demo` dev-dependencies now include `egui_kittest = "0.34"` with `wgpu` + `snapshot` + `eframe` features

### Added
- `kittest.toml` at repo root with per-OS snapshot thresholds
- `docs/testing.md` documenting the testing approach (four layers: L1a unit, L1b library widget, L1c library snapshot, L1d demo eframe integration)
- Library-level tests covering Empty/Error/zoom/scroll/clear viewer states (`crates/viewkai/tests/states.rs`)
- Library-level baseline snapshots (visual regression safety net) in `crates/viewkai/tests/snapshots/`
- Demo-level eframe integration tests covering keyboard shortcuts (Ctrl+0/1/2/±/G), DemoLoadState transitions (`crates/viewkai-demo/tests/shortcuts.rs`)
- `viewkai-demo` restructured as a library + binary crate. `DemoApp`, `DemoLoadState`, `run_native`, `run_web` are now public exports of `viewkai_demo`.
- `DemoApp::load_bytes_sync(&mut self, Vec<u8>)` method — loads a PDF without going through the file dialog.
- `DemoApp::viewer(&self) -> &Viewer` read-accessor for inspection.
- `DemoApp::load_state(&self) -> &DemoLoadState` read-accessor for test state inspection.

### Removed
- All `#[allow(deprecated)]` attributes from test files

### Notes
- No user-facing API changes. No breaking changes.
- Public API surface of `viewkai` library is bit-for-bit identical to v0.0.3.
- Eframe is now a dev-dependency of `viewkai` via `egui_kittest["eframe"]`. It is NOT a runtime dependency; the purity invariant `cargo tree -p viewkai -e normal` still shows no eframe.
- MSRV is explicitly not touched by this release. The pre-existing mismatch between workspace `edition = "2024"` and `clippy.toml`'s `msrv = "1.81"` is inherited tech debt from Plan 01 and remains unresolved.

## [0.0.3] — 2026-04-18 — Performant Viewer

### Added
- `viewkai::cache::TextureCache`: LRU eviction, configurable byte budget (default 256 MB)
- `viewkai::viewport::VisibilityTracker`: visible + prefetch page determination
- `viewkai::zoom::ZoomState`: Discrete, FitWidth, FitPage, Custom zoom levels
- Zoom-bucket rasterization: DPI buckets [72, 96, 144, 216, 288, 432]
- Zoom controls in demo: toolbar, dropdown, Ctrl+wheel, Ctrl+0/1/2 shortcuts
- Pinch-to-zoom via `egui::InputState::zoom_delta()`
- Page-jump control: bottom bar with Enter-to-jump, Ctrl+G focus
- `Viewer::scroll_to_page()`, `Viewer::set_zoom()`, `Viewer::zoom()`, `Viewer::cache_bytes()`
- Benchmark harness: `parse_500_page_doc`, `rasterize_page_at_150dpi` (criterion 0.5)
- Memory budget acceptance test: 500-page PDF, assert cache ≤ 256 MB
- `tests/fixtures/500page.pdf`: synthetic 500-page blank PDF fixture
- `docs/benchmarks/v0.0.3.md`: benchmark results

## [0.0.2] — 2026-04-18 — Hello Viewer

### Added
- `viewkai::Viewer` widget: `new()`, `load_bytes()`, `clear()`, `show()` with `PageState`
- Vertical-stack layout with 16px gaps and horizontal centering
- Lazy rasterization: pages rendered on first visibility at 100 DPI
- Empty state ("No document loaded"), error state (error + Retry button)
- `viewkai-engine::render_page()` function returning `RawImage` (RGBA)
- `viewkai-core::RawImage` type
- `viewkai-demo`: `DemoLoadState` machine with native file dialog (rfd), web URL fetch (ehttp), drag-and-drop
- Integration test `crates/viewkai/tests/hello.rs` (headless egui, no eframe)

## [0.0.1] — 2026-04-18 — Toolchain PoC

### Added
- Workspace scaffold: `viewkai-core`, `viewkai-engine`, `viewkai`, `viewkai-demo` crates
- `viewkai-core`: `PageIndex`, `PageSize`, `Error`, `Result`, `PointsRect`, `PixelRect`, `DpiScale` types with serde support
- `viewkai-engine`: PDFium binding via `pdfium-render`; `init()`, `Document::from_bytes()`, `page_count()`, `page_size()`
- `viewkai-demo`: eframe app showing "PDF loaded: N pages. Page 1 size: WxH points." (native + WASM)
- PDFium WASM artifacts (`public/pdfium.wasm`, `public/pdfium.js`) from paulocoutinhox/pdfium-lib 7623
- CI pipeline: fmt, clippy, test, build-wasm, purity-viewkai, purity-engine, purity-core
- GitHub Pages deploy workflow
- `docs/architecture.md` documenting the four hard architectural constraints
