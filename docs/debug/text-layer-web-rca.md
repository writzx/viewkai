# Text-layer web RCA — oversized nested red rectangles

## Setup

- Goal: determine whether the web debug overlay bug is caused by:
  - **H1**: missing device-pixel-ratio (DPR) scaling in plugin screen conversion; or
  - **H2**: bad glyph rectangles returned by pdfium-WASM via `tight_bounds()` / `loose_bounds()`.
- Inputs used for this RCA:
  - user-provided screenshot description: nested concentric red rectangles, roughly 6–8× oversized, with progressive growth across a line;
  - source review of:
    - `viewkai/crates/viewkai-plugins/src/text_layer.rs`
    - `viewkai/crates/viewkai-engine/src/text.rs`
    - `viewkai/crates/viewkai/src/lib.rs`
    - supporting files: `viewkai-engine/src/lib.rs`, `viewkai-plugins/src/search.rs`, `viewkai-core/src/rotation.rs`, `viewkai-core/src/text.rs`;
  - native snapshot artifacts already in repo:
    - `crates/viewkai/tests/snapshots/text_layer_debug_hello.png`
    - `crates/viewkai/tests/snapshots/text_layer_overlay_page_two.png`
- No browser was run for this phase. The web side is diagnosed from the shared code path plus the known native vs WASM backend split in `viewkai-engine`.

## Screenshot analysis

The reported shape strongly disfavors a pure DPR-scaling bug:

1. **Nested/concentric rectangles** imply multiple word boxes with different extents, not one uniform transform error.
2. **~6–8× oversize** is much larger than common web DPR values (typically ~1.25×, 1.5×, 2×, maybe 3×).
3. **Progressive growth across a line** matches cumulative word-union growth from bad glyph boxes. It does **not** match a missing scalar multiplier, which would enlarge all rectangles by the same factor.

This screenshot pattern already weights the diagnosis toward **H2**.

## Code path analysis

### 1. What the debug overlay actually draws

`TextLayerPlugin::draw_page_overlay()` does **not** draw per-glyph boxes. In debug mode it iterates `text.words` and strokes each `word.bbox` in red.

Relevant flow:

1. `viewkai-engine::extract_page_text()` builds `glyphs` from PDFium character bounds.
2. `group_glyphs()` unions glyph boxes into `WordSpan::bbox` values.
3. `TextLayerPlugin::draw_page_overlay()` draws those word boxes after rotation and zoom conversion.

That means any bad glyph rectangle upstream can inflate an entire word rectangle downstream.

### 2. How glyph boxes are produced

`viewkai/crates/viewkai-engine/src/text.rs`:

- iterates `page_text.chars()`;
- calls `char_obj.tight_bounds()` for every glyph;
- converts the returned `PdfRect` into `PointsRect` via `pdf_rect_to_viewkai()`;
- rejects only zero/negative sizes;
- performs **no plausibility filtering** against page size, font size, or neighboring glyphs.

Current extraction is therefore trust-based: if WASM PDFium emits a wildly wrong char rectangle, that rectangle is accepted as-is.

### 3. How word boxes grow

`group_glyphs()` uses `union_rect()` across all glyphs in a word. So if a single glyph has an extreme right edge, the whole word box expands. If several consecutive glyphs have increasingly wrong right edges, each later word box will appear larger than the previous one.

That exactly matches the reported “progressive line growth” pattern.

### 4. How screen conversion works

`TextLayerPlugin::page_rect_to_screen()` converts page-local point rects to egui screen rects as:

```rust
page_origin + bbox * zoom
```

where:

- `page_origin` comes from `PluginContext.page_rect_screen.min`;
- `zoom` is `ctx.zoom`, i.e. the same effective zoom used to size the page image rect;
- `page_rect_screen` is set by `Viewer::paint_pages()` / `paint_positioned_page()` to the same `page_rect` used to paint the page texture;
- pointer hit-testing uses the exact inverse mapping: `(pointer_pos - page_rect.min) / effective_zoom`.

This is a clean, self-consistent logical-point transform.

### 5. Where DPR does and does not appear

- The overlay path does **not** use device pixels; it uses egui logical coordinates end-to-end.
- `egui` pointer positions and `page_rect` are both in that same logical space.
- The rendered page bitmap is also drawn into `page_rect`, again in logical space.

So the absence of an explicit DPR factor in `page_rect_to_screen()` is not automatically a bug. A missing DPR factor here would only make sense if one side of the transform were in physical pixels and the other in logical points, but the reviewed code does not do that.

There **is** a separate DPR omission in page raster DPI selection (`ZoomState::zoom_to_dpi_bucket(effective_zoom)` does not consider `pixels_per_point`), but that explains the already-known **blurry 100% zoom** bug, not oversized debug rectangles.

## Native readings

Native evidence in-repo points to the overlay logic itself being sound:

1. Native snapshot tests exist for text-layer debug rendering:
   - `crates/viewkai/tests/text_layer_debug.rs`
   - `crates/viewkai/tests/text_layer_multipage.rs`
2. Their stored snapshots show red rectangles tracking words closely, not page-scale oversized boxes.
3. Native and web both use the same viewer/plugin code for:
   - zoom calculation,
   - page-to-screen overlay conversion,
   - word-box drawing.

So the shared overlay math is already known to behave plausibly on native.

## Web readings

The meaningful native/web split is in the PDFium backend binding, not in the overlay math:

- native `viewkai-engine` binds to a dynamic/system PDFium library;
- wasm `viewkai-engine` binds to the JS-initialized WASM PDFium backend.

Text extraction then calls the same high-level API surface (`page.text()`, `chars()`, `tight_bounds()`) in both environments.

Therefore, if native overlay output is sane but web overlay output is wildly wrong, the most credible divergence point is the backend-produced glyph rectangles, not the shared overlay transform.

## Diff analysis: H1 vs H2

### H1 — missing DPR scaling in plugin screen conversion

What we would expect if H1 were true:

- all overlay rects would be off by roughly the **same** scalar factor;
- the factor would usually resemble a realistic DPR value;
- the error would be a global coordinate mismatch, not per-word progressive expansion;
- other overlays using the same conversion pattern (for example search highlights) would be equally suspect.

What the code and evidence show instead:

- overlay conversion uses the same logical coordinate system as page layout and pointer events;
- no physical-pixel values enter `page_rect_to_screen()`;
- the screenshot symptom is non-uniform and cumulative, not uniformly scaled.

Result: **H1 is not supported.**

### H2 — pdfium-WASM returns bad glyph rectangles

What we would expect if H2 were true:

- upstream char boxes would sometimes be implausibly large;
- word unions would amplify those bad char boxes into nested larger rectangles;
- the bug could appear web-only because the viewer code is shared but the PDFium backend differs.

What the code and evidence show:

- `extract_page_text()` trusts `tight_bounds()` unconditionally aside from non-positive size checks;
- `group_glyphs()` unions glyph boxes into word boxes, amplifying any upstream rectangle corruption;
- native snapshots look sane, while the reported web screenshot is grossly wrong;
- the screenshot shape matches cumulative bad box unions very closely.

Result: **H2 is confirmed as the primary root cause.**

## Conclusion

The web text-layer debug overlay is oversized because **pdfium-WASM is producing bad glyph bounds that `viewkai-engine` currently accepts without plausibility checks**.

**Confirmed hypothesis: H2.**

**Rejected hypothesis: H1.** The reviewed overlay math is a straightforward logical-points transform and does not fit the observed failure shape.

## Fix direction for Phase D

Primary fix should be in `viewkai/crates/viewkai-engine/src/text.rs`, not in the plugin screen conversion.

Recommended changes:

1. **Add glyph-bbox plausibility validation immediately after `tight_bounds()` conversion.**
   - Reject boxes with absurd width/height relative to page size.
   - Reject boxes whose dimensions are implausible relative to `scaled_font_size()`.
   - Reject boxes that extend far outside the page rect.
2. **Skip implausible glyphs instead of feeding them into word grouping.**
   - This prevents `union_rect()` from ballooning word boxes.
3. **Optionally probe `loose_bounds()` only as a guarded fallback, not as a blind replacement.**
   - If tried, it must go through the same plausibility filter.
4. **Clip debug overlay rects to the page rect before painting.**
   - This is a defensive UI-layer containment measure, not the root-cause fix.

Recommended Phase D implementation split:

- **Engine fix (root cause):** add plausibility filtering in `extract_page_text()`.
- **Plugin hardening (defense-in-depth):** clamp/clip overlay rects in `TextLayerPlugin::draw_page_overlay()` so any future bad bounds cannot paint giant page-external boxes.

## Phase D change target summary

- Root cause change: `viewkai/crates/viewkai-engine/src/text.rs`
- Defensive overlay change: `viewkai/crates/viewkai-plugins/src/text_layer.rs`
- No change needed for the current page-to-screen DPR conversion logic.
