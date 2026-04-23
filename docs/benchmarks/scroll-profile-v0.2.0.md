# Scroll-lag profile — v0.2.0

## Setup

- Workspace: `/home/opencode/projects/valeria/viewkai`
- Viewer path under analysis: `crates/viewkai/src/lib.rs`
- Raster path under analysis: `crates/viewkai-engine/src/lib.rs`
- Supporting components: `crates/viewkai/src/viewport.rs`, `crates/viewkai/src/cache.rs`, `crates/viewkai/benches/large_doc.rs`
- Baseline benchmark artifact already in tree: `docs/benchmarks/v0.0.3.md`
- Existing bench inventory check: `cargo test --workspace -- --list 2>&1 | grep bench` reported only `0 benchmarks` lines, so this profile is based on static code analysis plus the existing v0.0.3 benchmark note rather than a fresh runtime harness.

## Methodology

- Traced the continuous-scroll hot path in `show_pages_continuous()` (`crates/viewkai/src/lib.rs:1093-1187`).
- Checked whether layout work is recomputed every frame via `compute_page_layout()` (`lib.rs:1439-1471`).
- Checked pending-scroll behavior via `handle_pending_scroll()` (`lib.rs:1854-1892`) and its call into `compute_page_viewport_rect()` / `compute_page_layout()`.
- Checked page visibility and prefetch policy via `VisibilityTracker` (`crates/viewkai/src/viewport.rs:26-99`).
- Checked render scheduling and cache-miss behavior via `render_queued_pages()` (`lib.rs:1528-1563`).
- Checked texture-cache budget and eviction behavior via `TextureCache` (`crates/viewkai/src/cache.rs:26-151`).
- Checked the raster backend in `viewkai_engine::render_page()` (`crates/viewkai-engine/src/lib.rs:334-391`) to determine whether page rasterization is synchronous.

## Timing table

Estimated per-frame / per-event costs for medium-to-large documents, using code structure plus the existing blank-page benchmark in `docs/benchmarks/v0.0.3.md` (`rasterize_page_at_150dpi ~= 0.65 ms` on a minimal page) as the only numeric anchor. Real PDFs with images, fonts, and vector content will be materially slower.

| Stage | Code path | p50 | p95 | p99 | Notes |
| --- | --- | ---: | ---: | ---: | --- |
| Layout recompute | `compute_page_layout()` | 0.10-0.30 ms | 0.40-0.90 ms | 1.0-2.0 ms | O(n) in page count every frame: one pass for `page_tops`, one pass for `page_heights`. Cost grows with document length but stays arithmetic-only. |
| Visibility + prioritization | `VisibilityTracker::compute()` + `prioritize_renders()` | 0.03-0.08 ms | 0.10-0.25 ms | 0.30-0.60 ms | Also O(n), but simple comparisons and a small sort over visible/prefetch pages. |
| Cache hit path | `cache.get()` + `paint_pages()` | 0.10-0.40 ms | 0.50-1.2 ms | 1.5-3.0 ms | Smooth-scroll baseline when already-rasterized textures exist. |
| Cache miss raster: blank/simple page | `render_queued_pages()` -> `viewkai_engine::render_page()` | 0.7-1.5 ms per page | 2-5 ms per page | 6-10 ms per page | Lower bound inferred from v0.0.3 blank-page render benchmark plus egui texture upload. |
| Cache miss raster: real content page | same | 4-12 ms per page | 12-25 ms per page | 25-50+ ms per page | Dominant hitch source once PDFium render + RGBA upload runs on the UI thread for one or more newly needed pages. |
| Cache-eviction burst | `TextureCache::insert()` with LRU evictions | ~0.1 ms | 0.5-2 ms | 2-5 ms | Extra overhead when 256 MB budget is near full; still secondary unless it triggers more misses next frame. |

Interpretation: the arithmetic work is linear in page count, but the visible hitch threshold is crossed when a scroll frame includes one or more synchronous raster misses. At 60 Hz, anything above ~16.7 ms is user-visible stutter; two content-heavy misses in one frame can easily exceed that budget.

## Bottleneck identification

**Dominant cause: synchronous PDFium rasterization and texture upload on the UI thread.**

Why this dominates:

1. `show_pages_continuous()` calls `render_queued_pages()` directly during the egui frame (`crates/viewkai/src/lib.rs:1143-1154`).
2. `render_queued_pages()` performs `viewkai_engine::render_page(...)` inline on every cache miss before returning to painting (`lib.rs:1538-1560`).
3. `viewkai_engine::render_page()` is fully synchronous: it locks PDFium state, fetches the page, configures render settings, rasterizes with `page.render_with_config(&config)`, and returns raw RGBA bytes (`crates/viewkai-engine/src/lib.rs:340-390`).
4. The same function then immediately uploads that RGBA buffer into an egui texture via `ui.ctx().load_texture(...)` in the frame loop (`lib.rs:1551-1559`).

Secondary contributors, but not the primary hitch source:

- `compute_page_layout()` is O(n) and runs every continuous-view frame (`lib.rs:1119-1121`, `1439-1471`), so long documents do pay a steady linear tax.
- `VisibilityTracker::new(2)` means only a 2-page prefetch window above and below the viewport (`lib.rs:77`, `crates/viewkai/src/viewport.rs:33-36`), which is conservative and increases the chance that a fast fling reaches uncached pages.
- `TextureCache` has a 256 MB default budget (`crates/viewkai/src/cache.rs:37-53`); for image-heavy or high-DPI pages, that can force eviction churn and create more future cache misses.

Even so, those secondary costs mostly explain *how often* misses happen. The actual hitch amplitude comes from doing rasterization synchronously in-frame.

## Micro-fix candidates

Bounded changes worth considering for Plan 03.25 D.3, without changing architecture:

1. **Increase prefetch distance from 2 to 3 pages.**
   - Scope: change `VisibilityTracker::new(2)` default in `RenderState::new()` to `3`.
   - Benefit: lowers the chance that moderate scroll velocity lands on an uncached page.
   - Risk: more eager raster work and faster cache consumption under the same 256 MB budget.

2. **Memoize `compute_page_layout()` by `(doc generation, effective_zoom bucket, rotation state)` until those inputs change.**
   - Scope: keep cached `page_tops/page_heights` inside render state and reuse across frames.
   - Benefit: removes the per-frame O(n) layout walk from normal scrolling.
   - Risk: modest state invalidation complexity; helps frame baseline, but not worst-case raster stalls.

3. **Cap in-frame raster work to one cache miss per frame.**
   - Scope: in `render_queued_pages()`, stop after first successful miss render and let later misses spill to the next repaint.
   - Benefit: bounds worst-frame latency and reduces multi-miss hitch spikes.
   - Risk: pages may briefly appear one frame later; this is a mitigation, not a full fix.

## Architecture-level recommendation

Treat this as a **Plan 04 blocker** unless the product accepts only partial mitigation.

The architectural problem is that page rasterization is synchronous and performed inside the main egui frame. As long as `render_queued_pages()` directly calls `viewkai_engine::render_page()` and uploads textures in that same frame, medium-to-large documents will continue to hitch whenever scrolling outruns the cache.

Recommended direction for the next plan:

- move raster jobs off the UI frame path into an asynchronous/background render queue compatible with native and WASM constraints;
- keep the UI thread limited to scheduling, polling completed renders, and uploading finished textures;
- make prefetch depth and cache budget policy serve that queue rather than trying to hide synchronous misses.

Conclusion: **ship Phase 0.2 as diagnosis + documentation, with at most bounded mitigations; do not claim the hitch is fixed in v0.2.0.**
