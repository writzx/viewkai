//! Text extraction types for viewkai.

use serde::{Deserialize, Serialize};

use crate::{PageIndex, PointsRect};

/// Zero-based character index within a page's text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CharIndex {
    /// The page this character belongs to.
    pub page: PageIndex,
    /// Zero-based character position within the page.
    pub char: usize,
}

/// A half-open range of characters spanning one or more pages: `[start, end)`.
///
/// Always normalized so `start` comes before `end` in document order
/// (i.e. `start.page < end.page`, or `start.page == end.page && start.char <= end.char`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectionRange {
    /// First character (inclusive).
    pub start: CharIndex,
    /// One past the last character (exclusive).
    pub end: CharIndex,
}

impl SelectionRange {
    /// Create a normalized selection range.
    #[must_use]
    pub fn new(a: CharIndex, b: CharIndex) -> Self {
        if (a.page.0, a.char) <= (b.page.0, b.char) {
            Self { start: a, end: b }
        } else {
            Self { start: b, end: a }
        }
    }

    /// Return true if the selection is empty (start == end).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }
}

/// A half-open range of characters on a single page: `[start, end)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharSpan {
    /// The page this span belongs to.
    pub page: PageIndex,
    /// First character index (inclusive).
    pub start: usize,
    /// One past the last character index (exclusive).
    pub end: usize,
}

/// A word: a contiguous run of non-whitespace characters on a single page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WordSpan {
    /// The page this word belongs to.
    pub page: PageIndex,
    /// First character index (inclusive).
    pub start_char: usize,
    /// One past the last character index (exclusive).
    pub end_char: usize,
    /// Bounding box of the word in page-local PDF points (Y-down, top-left origin).
    pub bbox: PointsRect,
}

/// A line: a horizontal run of characters sharing approximately the same baseline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LineSpan {
    /// The page this line belongs to.
    pub page: PageIndex,
    /// Y coordinate of the baseline in page-local PDF points (Y-down).
    pub y_baseline_pt: f32,
    /// First character index (inclusive).
    pub start_char: usize,
    /// One past the last character index (exclusive).
    pub end_char: usize,
}

/// A single glyph with its bounding box and font metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GlyphBox {
    /// The Unicode character this glyph represents.
    pub char: char,
    /// Tight bounding box in page-local PDF points (Y-down, top-left origin).
    pub bbox: PointsRect,
    /// Font size in PDF points.
    pub font_size_pt: f32,
    /// Glyph rotation in degrees (0 = upright).
    pub rotation_deg: f32,
}

/// All text data extracted from a single PDF page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PageText {
    /// All glyphs on the page, in extraction order.
    pub glyphs: Vec<GlyphBox>,
    /// Word groups derived from the glyphs.
    pub words: Vec<WordSpan>,
    /// Line groups derived from the glyphs.
    pub lines: Vec<LineSpan>,
}
