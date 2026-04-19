[![pre-alpha](https://img.shields.io/badge/status-pre--alpha-red)](https://github.com/writzx/viewkai)

# viewkai

viewkai is a Rust PDF viewer library targeting `wasm32-unknown-unknown` and native dev platforms, built on pdfium-render and egui.

## Stack

- Rust
- egui
- pdfium-render
- WASM / `wasm32-unknown-unknown`

## Status

- First useful release: `v0.1.0`
- Plugin architecture now powers text extraction, selection, and search
- Native and web demos live in separate sibling crates

## Repository layout

- `crates/viewkai-core` — core types and errors
- `crates/viewkai-engine` — PDFium integration boundary
- `crates/viewkai-plugins` — sealed built-in plugin abstraction (`TextLayerPlugin`, `SearchPlugin`)
- `crates/viewkai` — embeddable viewer library surface
- `crates/viewkai-app` — native PDF viewer application
- `crates/viewkai-web` — WASM web demo (renamed out of the old combined demo crate)

## Architecture

`viewkai` owns a fixed-order built-in plugin registry exposed through the sibling `viewkai-plugins` crate. In `v0.1.0`, the sealed `TextLayerPlugin` and `SearchPlugin` now ship as built-in functionality for text extraction, text selection, clipboard copy, and full-text search across the viewer's three contribution surfaces: per-page overlays, toolbar UI, and viewer-level overlays.

## Screenshots

### Text Selection (v0.1.0)
![Text selection](docs/media/v0.1.0-selection.png)

### Search (v0.1.0)
![Search](docs/media/v0.1.0-search.png)
