# Office Add-in Integration Notes

Notes from the viewkai v0.0.3 integration into Valeria's Excel add-in.

---

## PDFium WASM Loading

**How PDFium WASM loaded inside Office's iframe:**

The Office Add-in runs inside an iframe served from Valeria's dev server (localhost).
PDFium WASM is loaded via `pdfium.js` (Emscripten-generated glue) served from the same origin.

Initialization sequence:
1. `pdfium.js` is loaded via a `<script>` tag in `index.html`
2. `PDFiumModule()` factory function is called → returns a Promise
3. After the Promise resolves, `initialize_pdfium_render(pdfiumModule, wasmModule, false)` is called
4. `viewkai_engine::init()` is called from Rust, which calls `Pdfium::bind_to_system_library()`
5. The eframe app starts and the PDF viewer is ready

**CSP findings:**

- Office's iframe allows loading scripts from the same origin (localhost in dev mode)
- `pdfium.wasm` must be served from the same origin as the add-in bundle
- No CSP violations observed when serving from localhost

**Texture timing observations:**

- First page render takes ~50-100ms (PDFium parsing + rasterization)
- Subsequent pages render faster (~10-20ms each)
- LRU cache prevents memory growth beyond 256 MB

---

## Excel Sideload Results

**Date**: 2026-04-18  
**Excel version**: Microsoft Excel for Mac (latest)  
**viewkai version**: v0.0.3

- PDF renders correctly inside the add-in pane ✅
- Scroll works ✅
- Zoom controls work ✅
- No Excel crashes observed ✅
- No red errors in Edge DevTools ✅

---

## Word Sideload

Word sideload is explicitly deferred to a later plan (see V.4a in the plan document).
The `bun run sideload:mac:word` command exists and the manifest is compatible.

---

## Upgrade Notes

When upgrading viewkai:
1. Update the `tag` in `crates/ui/Cargo.toml`
2. Run `cargo update -p viewkai`
3. Run `bun run build:wasm` to rebuild
4. Test the Excel sideload
