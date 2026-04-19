//! Search types for viewkai.

use serde::{Deserialize, Serialize};

use crate::text::CharSpan;
use crate::{PageIndex, PointsRect};

/// A search query with options.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SearchQuery {
    /// The search term.
    pub term: String,
    /// Whether the search is case-sensitive.
    pub case_sensitive: bool,
    /// Whether to match whole words only.
    pub whole_word: bool,
}

/// A single search match on a page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchMatch {
    /// The page this match is on.
    pub page: PageIndex,
    /// The character span of the match.
    pub char_span: CharSpan,
    /// Per-line highlight rectangles in page-local PDF points (Y-down).
    pub rects: Vec<PointsRect>,
}

/// The current state of a search operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchState {
    /// The active query.
    pub query: SearchQuery,
    /// All matches found so far.
    pub matches: Vec<SearchMatch>,
    /// Index of the currently highlighted match.
    pub current_match: usize,
    /// Pages not yet searched.
    pub pending_pages: Vec<PageIndex>,
}
