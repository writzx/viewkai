[![pre-alpha](https://img.shields.io/badge/status-pre--alpha-red)](https://github.com/writzx/viewkai)

# viewkai

viewkai is a Rust PDF viewer library targeting `wasm32-unknown-unknown` and native dev platforms, built on pdfium-render and egui.

## Stack

- Rust
- egui
- pdfium-render
- WASM / `wasm32-unknown-unknown`

## Status

- Pre-alpha foundation workspace
- Library crates keep rendering, UI, and demo concerns separated
- Demo app exists only for local native and future web validation

## Repository layout

- `crates/viewkai-core` — core types and errors
- `crates/viewkai-engine` — PDFium integration boundary
- `crates/viewkai` — embeddable viewer library surface
- `crates/viewkai-demo` — minimal demo application
