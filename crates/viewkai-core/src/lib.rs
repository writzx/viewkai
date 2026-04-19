//! Core types, errors, and coordinate math for viewkai.

/// Canonical crate name for the shared core types crate.
pub const NAME: &str = "viewkai-core";

/// Common error types used across the workspace.
pub mod error;
/// Shared coordinate, page, and render value types.
pub mod types;

pub use error::*;
pub use types::*;

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

        let json = serde_json::to_string(&page_idx).unwrap();
        let decoded: PageIndex = serde_json::from_str(&json).unwrap();
        assert_eq!(page_idx, decoded);

        let json = serde_json::to_string(&page_size).unwrap();
        let decoded: PageSize = serde_json::from_str(&json).unwrap();
        assert_eq!(page_size, decoded);

        let json = serde_json::to_string(&rect).unwrap();
        let decoded: PointsRect = serde_json::from_str(&json).unwrap();
        assert_eq!(rect, decoded);

        let json = serde_json::to_string(&pixel_rect).unwrap();
        let decoded: PixelRect = serde_json::from_str(&json).unwrap();
        assert_eq!(pixel_rect, decoded);

        let json = serde_json::to_string(&dpi).unwrap();
        let decoded: DpiScale = serde_json::from_str(&json).unwrap();
        assert_eq!(dpi, decoded);
    }
}
