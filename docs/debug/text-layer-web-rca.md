# Text-layer web RCA

## Setup

- Workspace: `/home/opencode/projects/valeria/viewkai`
- Overlay path under analysis: `crates/viewkai-plugins/src/text_layer.rs`
- Extraction path under analysis: `crates/viewkai-engine/src/text.rs`
- Core text types: `crates/viewkai-core/src/text.rs`
- Native probe used `hello.pdf` plus `cargo test -p viewkai-engine -- --nocapture 2>&1 | head -50` and a one-off local probe to print native glyph/word boxes.

## Screenshot analysis

- Reported web symptom: nested concentric red rectangles that are roughly page-sized, about `6-8x` too large, and progressively larger line-by-line.
- That shape does **not** match a device-pixel-ratio bug. Missing DPR division would multiply every overlay by the same browser factor (`~2x` or `~3x`) and preserve relative glyph/word proportions.
- The observed growth pattern **does** match word boxes being unioned from one or more already-bad glyph boxes. In `viewkai-engine/src/text.rs`, each `WordSpan.bbox` is produced by repeated `union_rect()` over glyph bboxes, so one page-scale glyph on a line makes the whole word box explode.

## Native readings

- `page_rect_to_screen()` in `crates/viewkai-plugins/src/text_layer.rs:324-329` only performs `page_origin + bbox * zoom`.
- `draw_page_overlay()` draws `text.words[*].bbox` after `forward_rotate_rect(...)` using that same conversion (`text_layer.rs:356-370`).
- Native extraction in `crates/viewkai-engine/src/text.rs:73-95` builds glyph boxes directly from `char_obj.tight_bounds()` and converts them from PDF Y-up to viewkai Y-down via `pdf_rect_to_viewkai()`.
- Native probe on `hello.pdf` returned plausible glyph sizes, for example:
  - `glyph[0] 'F' bbox=(78.86, 80.82, 4.97, 7.18)`
  - `glyph[14] 'F' bbox=(79.368, 101.076, 9.198, 12.924)`
  - `word[0] chars=0..14 bbox=(78.86, 80.82, 73.51, 9.37)`
- `hello.pdf` is asserted to be about `612 pt` wide in `crates/viewkai-engine/tests/hello_pdf.rs:22-27`, so native glyph/word boxes are tens of points wide/tall, not page-sized.

## Web readings

- The web app does not implement a separate text overlay path. It embeds the shared `viewkai::Viewer`, which calls the same plugin overlay code.
- On WASM, `viewkai-engine::init()` uses `Pdfium::bind_to_system_library()` (`crates/viewkai-engine/src/lib.rs:111-115`), i.e. the platform divergence is the PDFium binding/backend, not custom overlay math.
- `PluginContext.zoom` is the viewer's effective zoom factor in egui logical points, and `page_rect_screen` is the egui page rect already used by all overlays (`crates/viewkai-plugins/src/plugin.rs:14-39`, `crates/viewkai/src/lib.rs:1649-1654`, `1725-1730`).
- The search overlay uses the same `page_origin + rect * zoom` math in `crates/viewkai-plugins/src/search.rs:263-268`. There is no web-only DPR compensation in one path but not the other.

## Diff analysis

### H1: missing DPR scaling in `page_rect_to_screen`

- Evidence against H1:
  1. `page_rect_to_screen()` is shared code, not web-only code.
  2. Viewer layout already expresses page size in egui logical points (`display_size = page_size_pt * effective_zoom` in `crates/viewkai/src/lib.rs:998-1001`, `1588-1591`). Overlay math matches that same space.
  3. A DPR bug would create a uniform multiplier across all boxes. It cannot explain nested, progressively larger rectangles.
  4. Search overlay uses the same conversion pattern, so a missing DPR correction would be a cross-overlay coordinate bug, not a text-only per-line growth bug.

### H2: pdfium-WASM returns page-sized `tight_bounds()` per glyph

- Evidence for H2:
  1. `extract_page_text()` trusts `char_obj.tight_bounds()` unconditionally and then unions glyph boxes into words.
  2. The screenshot pattern is exactly what happens when some glyph boxes are already too large before overlay conversion: word unions become concentric and line-dependent.
  3. Native readings show `tight_bounds()` is sensible on native for the same PDF, so the bad behavior is likely backend-specific.
  4. WASM is the only meaningful runtime divergence in this path.
  5. `pdfium-render` maps `tight_bounds()` directly to `FPDFText_GetCharBox()` (`/tmp/pdfium-render/src/pdf/document/page/text/char.rs:509-535`), so any WASM/backend oddity there propagates straight into `PageText`.

## Conclusion

**H2 confirmed**

The oversized web debug rectangles do not come from `page_rect_to_screen()`. That function is a straightforward page-points-to-egui-logical-points transform and would only produce a uniform scale error if DPR handling were wrong. The screenshot instead shows bad source rectangles being unioned into words. Because native `tight_bounds()` output for `hello.pdf` is normal while the web path uses the same overlay math but a different PDFium backend, the root cause is most consistent with pdfium-WASM returning page-scale or otherwise implausible `tight_bounds()` boxes for some glyphs.

## Fix direction

Implement the fix in `crates/viewkai-engine/src/text.rs`, immediately after reading each glyph bbox from `tight_bounds()` and before pushing it into `glyphs`:

1. Keep the current `tight_bounds()` call as the fast path.
2. Add a per-glyph sanity guard for implausible boxes on the current page, for example reject/fallback when any of these hold:
   - `bbox.width > page_width_pt * 0.5`
   - `bbox.height > page_height_pt * 0.5`
   - `bbox.width > char_obj.scaled_font_size().value * 4.0`
   - `bbox.height > char_obj.scaled_font_size().value * 4.0`
3. On guard hit, do **not** feed that raw box into word unioning. Prefer a safer replacement in this order:
   - try `char_obj.loose_bounds()` and convert it with `pdf_rect_to_viewkai()`;
   - if that is also implausible, skip the glyph entirely.
4. Keep the change engine-local so native and web continue sharing the same overlay code.
5. Add/extend an engine test to assert extracted glyph boxes for `hello.pdf` stay far below page scale.

This keeps Phase D.1 narrowly scoped: add a WASM-safe glyph-bbox sanity clamp in `viewkai-engine`, not a DPR patch in `viewkai-plugins`.
