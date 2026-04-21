//! Viewing-mode data shared across the workspace.

/// Viewer page layout mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum ViewMode {
    /// Show one page at a time.
    Single,
    /// Show pages in a single continuous vertical scroll.
    #[default]
    Continuous,
    /// Show pages in paired spreads.
    ///
    /// When `cover_separate` is `true`, page 1 is shown alone and later spreads
    /// pair as `(2, 3)`, `(4, 5)`, and so on. When `false`, spreads pair from
    /// the start as `(1, 2)`, `(3, 4)`, and so on.
    Spread {
        /// Whether the cover page stands alone before two-page pairing begins.
        cover_separate: bool,
    },
}
