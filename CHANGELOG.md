# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

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
