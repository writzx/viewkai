# Scroll Performance Profile v0.2.0

This document establishes the baseline performance profile for the `viewkai` PDF viewer as of version 0.2.0. It focuses on the `ViewMode::Continuous` scroll path to identify bottlenecks before implementing asynchronous rasterization.

## Profiling Methodology

The analysis is based on a static code path review of the main rendering loop. We evaluated the algorithmic complexity of layout, visibility tracking, and texture management. Latency estimates are derived from the synchronous nature of the current PDFium integration.

## Code Path Analysis

### 1. Layout Computation (`compute_page_layout`)
The viewer calculates the vertical position of every page in the document during each frame in continuous mode.
- **Complexity**: O(n), where n is the total page count.
- **Impact**: For documents with thousands of pages, this adds linear overhead to every frame, even if only two pages are visible.

### 2. Visibility Tracking (`VisibilityTracker::compute`)
The tracker iterates through the pre-computed page tops to determine which pages intersect the current viewport.
- **Complexity**: O(n).
- **Impact**: Similar to layout computation, this scales linearly with document size. While the operations are simple comparisons, they occur on the main UI thread every frame.

### 3. Synchronous Rasterization (`render_queued_pages`)
This is the most significant performance bottleneck. When a page enters the visible or prefetch range and lacks a cached texture, the viewer renders it immediately.
- **Complexity**: O(v * R), where v is the number of pages to render and R is the cost of a synchronous PDFium render call.
- **Impact**: The main UI thread blocks until the PDFium engine completes the render. This causes immediate and noticeable frame drops (jank) during scrolling.

### 4. Texture Cache (`TextureCache`)
The cache uses an LRU (Least Recently Used) eviction policy.
- **Hit Path**: O(1) hash map lookup.
- **Miss Path**: Triggers synchronous rendering.
- **Eviction**: O(m), where m is the number of entries in the cache, due to a linear search for the oldest entry during eviction.

## Estimated Latencies

These estimates represent the time the main UI thread is blocked during a scroll event.

| Metric | Estimated Latency | Scenario |
|--------|-------------------|----------|
| p50    | 1 to 2 ms         | All visible pages are already in the texture cache. |
| p95    | 50 to 150 ms      | A single page cache miss requiring synchronous rendering. |
| p99    | 300 to 1000+ ms   | Multiple cache misses or complex pages during rapid scrolling. |

## Primary Bottlenecks

1. **Synchronous PDFium Calls**: The `viewkai_engine::render_page` call is the primary source of scroll lag. It prevents the UI from remaining responsive while new pages are prepared.
2. **Linear Document Processing**: Both layout and visibility tracking scale with the total number of pages rather than the number of visible pages. This will degrade performance on very large documents.

## Baseline for Future Improvements

This profile confirms that the current architecture is limited by its synchronous rendering model. The upcoming implementation of asynchronous rasterization in Plan 03.5 aims to move the `render_page` calls to background threads, which should bring the p95 and p99 latencies closer to the p50 baseline.
