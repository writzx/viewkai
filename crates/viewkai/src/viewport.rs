//! Visible-page determination for viewkai.

use viewkai_core::page::PageIndex;

/// The set of pages that should be rendered in the current frame.
#[derive(Debug, Default, Clone)]
pub struct VisibleSet {
    /// Pages currently visible in the viewport.
    pub pages: Vec<PageIndex>,
    /// Pages just above the viewport (prefetch candidates).
    pub prefetch_above: Vec<PageIndex>,
    /// Pages just below the viewport (prefetch candidates).
    pub prefetch_below: Vec<PageIndex>,
}

impl VisibleSet {
    /// All pages that should be rasterized (visible + prefetch).
    pub fn all_to_render(&self) -> impl Iterator<Item = &PageIndex> {
        self.pages
            .iter()
            .chain(self.prefetch_above.iter())
            .chain(self.prefetch_below.iter())
    }
}

/// Determines which pages are visible and which should be prefetched.
pub struct VisibilityTracker {
    /// Number of pages to prefetch above and below the visible area.
    prefetch_distance: usize,
}

impl VisibilityTracker {
    /// Create a tracker with the given prefetch distance (default: 2).
    #[must_use]
    pub fn new(prefetch_distance: usize) -> Self {
        Self { prefetch_distance }
    }

    /// Compute the visible set given the current scroll state.
    ///
    /// # Parameters
    /// - `scroll_offset_y`: vertical scroll offset in pixels
    /// - `viewport_height`: height of the visible area in pixels
    /// - `page_tops`: y-coordinate of the top of each page (in pixels, cumulative)
    /// - `page_heights`: height of each page in pixels
    ///
    /// # Panics
    ///
    /// Panics if `page_tops.len() != page_heights.len()`.
    #[must_use]
    pub fn compute(
        &self,
        scroll_offset_y: f32,
        viewport_height: f32,
        page_tops: &[f32],
        page_heights: &[f32],
    ) -> VisibleSet {
        assert_eq!(page_tops.len(), page_heights.len());

        let viewport_top = scroll_offset_y;
        let viewport_bottom = scroll_offset_y + viewport_height;

        let mut visible = Vec::new();
        let mut first_visible = None;
        let mut last_visible = None;

        for (i, (&top, &height)) in page_tops.iter().zip(page_heights.iter()).enumerate() {
            let bottom = top + height;
            if bottom > viewport_top && top < viewport_bottom {
                visible.push(PageIndex(i));
                if first_visible.is_none() {
                    first_visible = Some(i);
                }
                last_visible = Some(i);
            }
        }

        let page_count = page_tops.len();

        let prefetch_above = if let Some(first) = first_visible {
            let start = first.saturating_sub(self.prefetch_distance);
            (start..first).map(PageIndex).collect()
        } else {
            Vec::new()
        };

        let prefetch_below = if let Some(last) = last_visible {
            let end = (last + 1 + self.prefetch_distance).min(page_count);
            ((last + 1)..end).map(PageIndex).collect()
        } else {
            Vec::new()
        };

        VisibleSet {
            pages: visible,
            prefetch_above,
            prefetch_below,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tracker() -> VisibilityTracker {
        VisibilityTracker::new(2)
    }

    // justify: test inputs are tiny fixture counts, so converting indices to
    // `f32` cannot lose meaningful precision here.
    #[allow(clippy::cast_precision_loss)]
    fn uniform_layout(count: usize, height: f32) -> (Vec<f32>, Vec<f32>) {
        let tops = (0..count).map(|i| i as f32 * height).collect();
        let heights = vec![height; count];
        (tops, heights)
    }

    #[test]
    fn single_visible_page_mid_doc() {
        let (tops, heights) = uniform_layout(10, 100.0);
        let tracker = tracker();
        let set = tracker.compute(500.0, 100.0, &tops, &heights);

        assert_eq!(set.pages, vec![PageIndex(5)]);
        assert_eq!(set.prefetch_above, vec![PageIndex(3), PageIndex(4)]);
        assert_eq!(set.prefetch_below, vec![PageIndex(6), PageIndex(7)]);
    }

    #[test]
    fn all_pages_fit_in_viewport() {
        let (tops, heights) = uniform_layout(3, 100.0);
        let tracker = tracker();
        let set = tracker.compute(0.0, 400.0, &tops, &heights);

        assert_eq!(set.pages.len(), 3);
        assert!(set.prefetch_above.is_empty());
        assert!(set.prefetch_below.is_empty());
    }

    #[test]
    fn scroll_near_start_no_prefetch_above() {
        let (tops, heights) = uniform_layout(10, 100.0);
        let tracker = tracker();
        let set = tracker.compute(0.0, 100.0, &tops, &heights);

        assert_eq!(set.pages, vec![PageIndex(0)]);
        assert!(set.prefetch_above.is_empty());
        assert_eq!(set.prefetch_below, vec![PageIndex(1), PageIndex(2)]);
    }

    #[test]
    fn scroll_near_end_no_prefetch_below() {
        let (tops, heights) = uniform_layout(10, 100.0);
        let tracker = tracker();
        let set = tracker.compute(900.0, 100.0, &tops, &heights);

        assert_eq!(set.pages, vec![PageIndex(9)]);
        assert_eq!(set.prefetch_above, vec![PageIndex(7), PageIndex(8)]);
        assert!(set.prefetch_below.is_empty());
    }

    #[test]
    fn variable_heights() {
        let tops = vec![0.0, 100.0, 250.0, 350.0];
        let heights = vec![100.0, 150.0, 100.0, 200.0];
        let tracker = VisibilityTracker::new(1);
        let set = tracker.compute(100.0, 150.0, &tops, &heights);

        assert_eq!(set.pages, vec![PageIndex(1)]);
        assert_eq!(set.prefetch_above, vec![PageIndex(0)]);
        assert_eq!(set.prefetch_below, vec![PageIndex(2)]);
    }
}
