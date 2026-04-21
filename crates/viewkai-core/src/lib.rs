//! Core types, errors, and coordinate math for viewkai.

/// Canonical crate name for the shared core types crate.
pub const NAME: &str = "viewkai-core";

/// Common error types used across the workspace.
pub mod error;
/// Document outline (table of contents) types.
pub mod outline;
/// Shared display-time page rotation types.
pub mod rotation;
/// Shared search data types.
pub mod search;
/// Shared text extraction data types.
pub mod text;
/// Shared coordinate, page, and render value types.
pub mod types;
/// Shared viewing-mode data types.
pub mod view_mode;

pub use error::*;
pub use outline::{DestPosition, Destination, Outline, OutlineNode, OutlineNodeId};
pub use rotation::{
    PdfPageRotation, RotationDelta, forward_rotate_point, forward_rotate_rect,
    inverse_rotate_point, rotated_page_size,
};
pub use search::{SearchMatch, SearchQuery, SearchState};
pub use text::{CharIndex, CharSpan, GlyphBox, LineSpan, PageText, SelectionRange, WordSpan};
pub use types::*;
pub use view_mode::ViewMode;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_roundtrip() {
        let page_idx = PageIndex(3);
        let page_size = PageSize {
            width_pt: 612.0,
            height_pt: 792.0,
        };
        let point = PointsPos { x: 10.0, y: 20.0 };
        let rect = PointsRect {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 200.0,
        };
        let pixel_rect = PixelRect {
            x: 0,
            y: 0,
            width: 100,
            height: 200,
        };
        let dpi = DpiScale(1.5);
        let glyph = GlyphBox {
            char: 'A',
            bbox: rect,
            font_size_pt: 12.0,
            rotation_deg: 0.0,
        };
        let word = WordSpan {
            page: page_idx,
            start_char: 0,
            end_char: 1,
            bbox: rect,
        };
        let line = LineSpan {
            page: page_idx,
            y_baseline_pt: 42.0,
            start_char: 0,
            end_char: 1,
        };
        let char_index = CharIndex {
            page: page_idx,
            char: 0,
        };
        let char_span = CharSpan {
            page: page_idx,
            start: 0,
            end: 1,
        };
        let page_text = PageText {
            glyphs: vec![glyph.clone()],
            words: vec![word.clone()],
            lines: vec![line.clone()],
        };

        let json = serde_json::to_string(&page_idx).unwrap();
        let decoded: PageIndex = serde_json::from_str(&json).unwrap();
        assert_eq!(page_idx, decoded);

        let json = serde_json::to_string(&page_size).unwrap();
        let decoded: PageSize = serde_json::from_str(&json).unwrap();
        assert_eq!(page_size, decoded);

        let json = serde_json::to_string(&point).unwrap();
        let decoded: PointsPos = serde_json::from_str(&json).unwrap();
        assert_eq!(point, decoded);

        let json = serde_json::to_string(&rect).unwrap();
        let decoded: PointsRect = serde_json::from_str(&json).unwrap();
        assert_eq!(rect, decoded);

        let json = serde_json::to_string(&pixel_rect).unwrap();
        let decoded: PixelRect = serde_json::from_str(&json).unwrap();
        assert_eq!(pixel_rect, decoded);

        let json = serde_json::to_string(&dpi).unwrap();
        let decoded: DpiScale = serde_json::from_str(&json).unwrap();
        assert_eq!(dpi, decoded);

        let json = serde_json::to_string(&glyph).unwrap();
        let decoded: GlyphBox = serde_json::from_str(&json).unwrap();
        assert_eq!(glyph, decoded);

        let json = serde_json::to_string(&word).unwrap();
        let decoded: WordSpan = serde_json::from_str(&json).unwrap();
        assert_eq!(word, decoded);

        let json = serde_json::to_string(&line).unwrap();
        let decoded: LineSpan = serde_json::from_str(&json).unwrap();
        assert_eq!(line, decoded);

        let json = serde_json::to_string(&char_index).unwrap();
        let decoded: CharIndex = serde_json::from_str(&json).unwrap();
        assert_eq!(char_index, decoded);

        let json = serde_json::to_string(&char_span).unwrap();
        let decoded: CharSpan = serde_json::from_str(&json).unwrap();
        assert_eq!(char_span, decoded);

        let json = serde_json::to_string(&page_text).unwrap();
        let decoded: PageText = serde_json::from_str(&json).unwrap();
        assert_eq!(page_text, decoded);
    }
}
