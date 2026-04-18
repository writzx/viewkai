# Architecture

## The `viewkai` library crate depends on `egui` only. It does NOT depend on `eframe`.

`viewkai` is intended to be embedded by host applications, not tied to an application runtime. Keeping `eframe` out of the library preserves portability for Valeria, future consumers, and headless tests.

## viewkai is I/O-free.

Library crates accept PDF bytes from callers and return library-owned results without reading files, making network requests, or touching platform services. This keeps the core portable and predictable across native and WASM targets.

## pdfium-render never leaks out of `viewkai-engine`.

PDFium integration stays isolated inside the engine crate so public APIs can be expressed entirely in viewkai-owned types. That boundary makes future engine replacement or adaptation a single-crate change instead of a workspace-wide rewrite.

## All features are WASM-compatible from day one.

WASM is the deployment target, so native convenience cannot define the architecture. Avoiding native-only feature leakage and platform-specific execution models keeps the workspace aligned with browser delivery from the start.
