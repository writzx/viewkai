# Architecture

## The `viewkai` library crate depends on `egui` only. It does NOT depend on `eframe`.

`viewkai` is intended to be embedded by host applications, not tied to an application runtime. Keeping `eframe` out of the library preserves portability for Valeria, future consumers, and headless tests.

## viewkai is I/O-free.

Library crates accept PDF bytes from callers and return library-owned results without reading files, making network requests, or touching platform services. This keeps the core portable and predictable across native and WASM targets.

## pdfium-render never leaks out of `viewkai-engine`.

PDFium integration stays isolated inside the engine crate so public APIs can be expressed entirely in viewkai-owned types. That boundary makes future engine replacement or adaptation a single-crate change instead of a workspace-wide rewrite.

## All features are WASM-compatible from day one.

WASM is the deployment target, so native convenience cannot define the architecture. Avoiding native-only feature leakage and platform-specific execution models keeps the workspace aligned with browser delivery from the start.

## PDFium WASM Vendoring

The `public/pdfium.js` and `public/pdfium.wasm` files are vendored binary artifacts from
[paulocoutinhox/pdfium-lib](https://github.com/paulocoutinhox/pdfium-lib) release **7623**
(Chromium/PDFium branch `chromium/7623`, released 2026-01-08).

### File inventory

| File | Size (bytes) | SHA-256 |
|---|---|---|
| `public/pdfium.wasm` | 3,984,252 | `14ca2adbe23b45dea57da28ae2746e376f1cddfb8e2d0b01b71dcc5cf227734e` |
| `public/pdfium.js` | 198,672 | `f76099920c374db2e98b6b641166218ca034d132fba2bfb015851b7440c6dfb7` |

### Init pattern (factory-function, async)

`pdfium.js` exposes a global `PDFiumModule` IIFE that returns an async factory function:

```javascript
PDFiumModule().then(function(m) {
    window.Module = m;   // m has .cwrap(), .HEAPU8, .wasmExports.malloc, etc.
});
```

This is **not** an auto-init build — you must call `PDFiumModule()` explicitly.

### Heap growth

This build is compiled with `ALLOW_MEMORY_GROWTH=1` (growable heap). The WASM heap can
grow up to the browser's available memory, unlike bblanchon builds which cap at 256 MB.
`pdfium-render` benefits from this when rendering large or many-page documents.

### pdfium-render runtime binding

In WASM targets, `pdfium-render` accesses PDFium by reading the resolved module object
(`window.Module`). The `public/index.html` init ceremony sets `window.Module` before
trunk starts the eframe WASM app, ensuring `pdfium-render`'s `bind_to_pdfium_wasm_bindings()`
call finds the bindings already in place.

### Upgrade procedure

To upgrade to a new pdfium-lib release:
1. `curl -fsSL https://github.com/paulocoutinhox/pdfium-lib/releases/download/<tag>/wasm.tgz | tar xz`
2. Copy `release/node/pdfium.wasm` and `release/node/pdfium.js` to `public/`.
3. Update SHA-256 hashes and file sizes in this section.
4. Verify the init pattern hasn't changed (test `PDFiumModule()` in browser DevTools).
