# Plan 02.5 Feedback — v0.1.1

## Demo Date
2026-04-20

## Features Demonstrated
- F3 / Shift+F3 global find next/prev shortcuts (both native and web)
- Cmd/Ctrl+G / Cmd/Ctrl+Shift+G secondary find shortcuts
- ▲▼ navigation buttons in the find overlay
- Click-outside-text clears selection (blur semantics)
- Cmd/Ctrl-click extends existing selection (additive)
- Web demo auto-loads hello.pdf on startup
- WASM panics now surface to browser console

## Bugs Fixed
- Per-page text-overlay misalignment on pages 2+ (page_rect_screen fix)
- Find-highlight rectangles drawn over wrong glyphs on pages 2+ (same root cause)
- CI red on v0.1.0 ship commit (fmt, clippy, machete, build-wasm, test)
- GitHub Pages deploy cascade from wasm build failure

## Known Issues
- Non-test Rust LOC increased by +268 lines vs v0.1.0 baseline (3831 → 4099); all additions are justified by audit-confirmed bugs (page_rect_screen, inside_page_rect, F3 shortcuts, wasm fixes)
- Multipage regression tests (text_layer_multipage, search_multipage) use blank 500page.pdf fixture; they verify no crash but do not directly prove glyph alignment on page 2 — a real-content multi-page fixture would strengthen coverage
- Triple-click line selection: resolved — `response.triple_clicked()` is used in lib.rs:947 (known issue from plan-02.md is now closed)
- Large-doc search performance: still not benchmarked (carried forward from plan-02)
- F3 shortcut on macOS requires Fn key unless system settings map function keys; this is a platform constraint, not a bug
