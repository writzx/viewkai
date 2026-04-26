# Changelog

All notable changes to this project will be documented in this file.

## [0.2.1] — 2026-04-26

### Fixed
- Text-layer web fix: glyph bbox plausibility filtering (H2 confirmed: bad pdfium-WASM bounds)
- 100% zoom DPR crispness via `zoom_to_dpi_bucket_with_dpr` internal helper
- Scroll coordinate fix: `content_origin` offset in `handle_pending_scroll` (fixes thumbnail click + page-number input)
- Find overlay: `<`, `>`, `x` ASCII fallback (no more tofu squares)

### Changed
- Thumbnail visual polish: removed captions, clickable styling, active-page indicator, scrollbar gutter
- Mode selector: native toolbar now uses ComboBox (like web)
- Page nav: `<` / `>` buttons and auto-updating page input
- Menu restructure: `View → Debug View` toggle; text-layer toggle moved to debug panel; toolbar checkboxes removed

### Added
- `egui-phosphor` v0.12 added as runtime dependency for icons

## [0.2.0] — 2026-04-21 — Application Shell

### Phase 0 — Layout engine refactor + baseline metrics
- Captured `v0.1.1` baseline metrics under `.sisyphus/metrics/`
- Added `ViewMode` enum (`Single`, `Continuous`, `Spread { cover_separate }`) to `viewkai-core`, re-exported as `viewkai::ViewMode`
- Refactored `Viewer::show_pages()` to dispatch by mode; `Continuous` behavior unchanged
- Extracted shared layout helpers for Phase C reuse

### Phase A — Document outline (S6.1)
- Added `OutlinePlugin` (built-in) that extracts PDF bookmarks and renders a collapsible tree sidebar
- New public types: `Outline`, `OutlineNode`, `OutlineNodeId`, `Destination`, `DestPosition` in `viewkai-core`
- `Document::outline()` with eager `OnceCell<Arc<Outline>>` caching
- `Viewer::document_arc()`, `Viewer::outline()` / `outline_mut()`, `Viewer::goto_destination()`
- `Ctrl+Shift+O` sidebar toggle in `viewkai-app` and `viewkai-web`
- `viewkai::init()` reexport added
- 10+ new tests; `bookmarks.pdf` fixture

### Phase B — Page thumbnails (S6.2)
- Added `ThumbnailPlugin` (built-in) with `ThumbnailCache` (64 MB LRU, separate from main texture cache)
- Engine helper `render_thumbnail(doc, page, 120px)` using `PdfRenderConfig::thumbnail()`
- `Viewer::thumbnails()` / `thumbnails_mut()`, `Viewer::thumbnail_texture()` shim
- `Ctrl+Shift+T` toggle; tabbed sidebar (Outline + Thumbnails tabs) in both crates
- 8+ new tests; budget enforcement tested

### Phase C — Viewing modes (S7.1)
- `ViewMode::Single`: one centered page, `PgUp`/`PgDn`/`Home`/`End`/`Space` navigation
- `ViewMode::Spread { cover_separate }`: two-up pages, `Ctrl+Left`/`Ctrl+Right` spread navigation
- Mode switcher in both `viewkai-app` and `viewkai-web`
- `Viewer::navigate_next_page()` / `navigate_prev_page()` public API
- Phase 0 placeholder fully removed
- 6+ new tests; 2 new snapshots

### Phase D — Display-time rotation (S7.2)
- `PdfPageRotation` enum (`None`, `R90`, `R180`, `R270`) in `viewkai-core`
- `Viewer::rotate_page()`, `rotate_all()`, `rotation_of()`, `reset_rotations()`
- Rotation threaded through engine rendering, texture cache, thumbnail cache
- Text-layer hit-testing and selection/search highlight rendering corrected for rotated pages
- `Ctrl+Shift+L` / `Ctrl+Shift+R` shortcuts; rotation submenu in app
- 8+ new tests; 4 rotation snapshots

### Phase E — Application shell (native ⇄ web parity)
- `File` / `View` / `Help` menu bars in both `viewkai-app` and `viewkai-web`
- `File → Open File…`: `rfd` on native, hidden `<input type="file">` on web
- `File → Open from URL…`: `ehttp` on native, existing `start_fetch` on web
- Top-bar URL input removed from `viewkai-web`; URL entry is menu-only
- Window title reflects loaded document on both platforms
- `docs/shortcuts.md` shortcut registry added
- About dialog in both crates
- Dep delta: `viewkai-app` +`ehttp`; `viewkai-web` +6 `web-sys` feature flags; library crates unchanged

## [0.1.1] — 2026-04-20 — Bugfix Pass

### Fixed

- Per-page text-overlay misalignment on pages 2+ (`text_layer.rs`/`search.rs` `page_origin` computation).
- Find-highlight rectangles drawn over wrong glyphs on pages 2+ (same root cause as above).
- Clicking inside a page area but outside any glyph now clears selection (blur semantics).
- Starting a new selection without `Cmd`/`Ctrl` clears prior selection; with `Cmd`/`Ctrl` extends it.
- `viewkai-web` now compiles for `wasm32-unknown-unknown` (`use wasm_bindgen::JsCast` import + wasm32 stub `fn main()`).
- `viewkai-core` `test-support` feature no longer trips clippy `redundant_feature_names`.
- `viewkai-plugins` no longer declares unused `thiserror` dep.
- `cargo fmt --check` clean across `viewkai-web`.
- `selection_highlight_hello` snapshot regenerated from Linux/lavapipe CI.

### Added

- `console_error_panic_hook` in `viewkai-web` surfaces WASM panics to the browser console.
- `viewkai-web` auto-loads bundled `hello.pdf` on startup so the public demo opens with rendered content.
- `F3` / `Shift+F3` (and `Cmd/Ctrl+G` / `Cmd/Ctrl+Shift+G`) global keyboard shortcuts for find next/prev in both `viewkai-app` and `viewkai-web`.
- ▲▼ navigation buttons in the find overlay.

### Internal

- `PluginContext` gains `page_rect_screen: Option<egui::Rect>` for plugins that paint absolute-positioned per-page overlays.
- `PointerEvent` gains `inside_page_rect: bool` so plugins can distinguish "click in page, miss text" from "click outside page".

### Notes

- No public API breakage. The two new struct fields are non-breaking additions because the structs are constructed only by `viewkai`'s own code via the sealed plugin trait surface.

## [0.1.0] — 2026-04-20 — Text & Interaction (Plugin Architecture)

### Added

- **Plugin architecture** (`viewkai-plugins` crate): sealed `ViewerPlugin` trait with three contribution surfaces (per-page overlay, toolbar, viewer-level overlay). Built-in plugins auto-registered in `Viewer::new()`. Typed accessors `Viewer::text_layer()`, `Viewer::search()`.
- **TextLayerPlugin** (Phase A): per-character bbox extraction, word/line grouping, debug overlay (`Viewer::set_text_layer_debug`), hit-testing (`char_at_page_pos`).
- **Text selection** (Phase B): drag-select, shift-click extend, double-click word, triple-click line, Ctrl+A select-all, Ctrl+C copy, multi-page selection. `Viewer::select_all()`, `Viewer::selected_text()`, `Viewer::copy_selected_text()`.
- **SearchPlugin** (Phase C): Ctrl+F floating overlay, per-page match highlights, case-sensitive and whole-word toggles, Enter/Shift+Enter navigation, auto-scroll to current match. `Viewer::open_search()`, `Viewer::next_match()`, `Viewer::prev_match()`.
- **`viewkai-app`** crate: native PDF viewer application (split from `viewkai-demo`).
- **`viewkai-web`** crate: WASM-only web demo (renamed from `viewkai-demo`).
- Three new CI purity jobs: `purity-app`, `purity-web`, `purity-plugins`.

### Changed

- `viewkai-demo` split into `viewkai-app` (native) and `viewkai-web` (WASM). The combined `viewkai-demo` crate no longer exists.
- `Viewer::show(ui)` now also invokes viewer-level plugin overlays (`show_plugin_overlays`). Consumers who want custom placement should call `show_pages(ui)` + `show_plugin_overlays(ctx)` instead.

### Notes

- First public-useful release. viewkai now supports text selection, clipboard copy, and full-text search — all delivered through the new plugin architecture.
- The `ViewerPlugin` trait is sealed (third parties cannot author plugins) in 0.1.0; this is deliberate and may be relaxed in a future release.
- Previously combined `viewkai-demo` crate was split: native code moved to `viewkai-app` and web-only code moved to `viewkai-web`.

## v0.0.5 — 2026-04-19 (Plan 01.75, architecture pass)

### Changed
- `viewkai::Viewer` internally decomposed into `Viewer { state, render, pending_scroll_to_page }`. No public API change.
- `ViewerState::Error` now wraps the structured `viewkai::LoadError` enum instead of a `String`. Existing callers that match on the error get richer context; error-as-string callers must adapt.
- `viewkai_engine::Document` now caches a live `PdfDocument` internally; subsequent `render_page` calls no longer re-parse the PDF bytes.
- `viewkai_engine::EngineError` enum replaces the previous string-typed error surface.

### Added
- Governance docs: `docs/coding-style.md`, `docs/dependency-policy.md`, `docs/error-handling.md`.
- CI jobs: `deny`, `machete`, `hack`, `msrv`, `docs`.
- `[workspace.lints]` clippy-pedantic + clippy-cargo at warn level.
- `viewkai-app/src/zoom_ui.rs` — shared zoom toolbar helper.
- `viewkai-web/src/wasm_state.rs` — consolidated WASM-only state.

### Removed
- `viewkai-core` file fragmentation: `coord.rs`, `page.rs`, `render.rs` merged into `types.rs`.

### Fixed
- `clippy.toml` MSRV was `1.81`, incompatible with `edition = "2024"`; corrected to `1.92` (actual minimum from transitive deps).
- `cache::evict_lru` dead `else`-branch removed.
- `cache::evict_page` now uses single-pass `HashMap::retain`.
- `zoom.rs` `BUCKETS` constant was duplicated in two functions; unified to one module-level const.
- `render_page` no longer re-parses PDF bytes on every frame.

### Architecture
- Three quality criteria codified: less code / better architecture, published best practices, library-documented idioms. Enforced by `[workspace.lints]` + the new CI jobs.

## [0.0.4] — 2026-04-18 — Testing Migration

### Changed
- Migrated tests from `egui::Context::default() + ctx.run()` to `egui_kittest::Harness`
- `viewkai` and `viewkai-app` dev-dependencies now include `egui_kittest = "0.34"` with `wgpu` + `snapshot` + `eframe` features

### Added
- `kittest.toml` at repo root with per-OS snapshot thresholds
- `docs/testing.md` documenting the testing approach (four layers: L1a unit, L1b library widget, L1c library snapshot, L1d demo eframe integration)
- Library-level tests covering Empty/Error/zoom/scroll/clear viewer states (`crates/viewkai/tests/states.rs`)
- Library-level baseline snapshots (visual regression safety net) in `crates/viewkai/tests/snapshots/`
- Demo-level eframe integration tests covering keyboard shortcuts (Ctrl+0/1/2/±/G), `LoadState` transitions (`crates/viewkai-app/tests/shortcuts.rs`)
- `viewkai-app` restructured as a library + binary crate. `App`, `LoadState`, and `run` are now public exports of `viewkai_app`.
- `App::load_bytes_sync(&mut self, Vec<u8>)` method — loads a PDF without going through the file dialog.
- `App::viewer(&self) -> &Viewer` read-accessor for inspection.
- `App::load_state(&self) -> &LoadState` read-accessor for test state inspection.

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
- `viewkai-app`: `LoadState` machine with native file dialog (rfd)
- `viewkai-web`: `DemoLoadState` machine with web URL fetch (ehttp) and drag-and-drop
- Integration test `crates/viewkai/tests/hello.rs` (headless egui, no eframe)

## [0.0.1] — 2026-04-18 — Toolchain PoC

### Added
- Workspace scaffold: `viewkai-core`, `viewkai-engine`, `viewkai`, `viewkai-app`, `viewkai-web` crates
- `viewkai-core`: `PageIndex`, `PageSize`, `Error`, `Result`, `PointsRect`, `PixelRect`, `DpiScale` types with serde support
- `viewkai-engine`: PDFium binding via `pdfium-render`; `init()`, `Document::from_bytes()`, `page_count()`, `page_size()`
- `viewkai-app` / `viewkai-web`: eframe apps showing "PDF loaded: N pages. Page 1 size: WxH points." on native and WASM respectively
- PDFium WASM artifacts (`public/pdfium.wasm`, `public/pdfium.js`) from paulocoutinhox/pdfium-lib 7623
- CI pipeline: fmt, clippy, test, build-wasm, purity-viewkai, purity-engine, purity-core
- GitHub Pages deploy workflow
- `docs/architecture.md` documenting the four hard architectural constraints
