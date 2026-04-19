//! Shared value types and errors for the viewkai workspace.

/// Canonical crate name for the shared core types crate.
pub const NAME: &str = "viewkai-core";

/// Coordinate-space value types.
pub mod coord;
/// Common error types used across the workspace.
pub mod error;
/// Page identifiers and dimensions.
pub mod page;
/// Rendering output value types.
pub mod render;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_roundtrip() {
        let page_idx = page::PageIndex(3);
        let page_size = page::PageSize {
            width_pt: 612.0,
            height_pt: 792.0,
        };
        let rect = coord::PointsRect {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 200.0,
        };
        let pixel_rect = coord::PixelRect {
            x: 0,
            y: 0,
            width: 100,
            height: 200,
        };
        let dpi = coord::DpiScale(1.5);

        let json = serde_json::to_string(&page_idx).unwrap();
        let decoded: page::PageIndex = serde_json::from_str(&json).unwrap();
        assert_eq!(page_idx, decoded);

        let json = serde_json::to_string(&page_size).unwrap();
        let decoded: page::PageSize = serde_json::from_str(&json).unwrap();
        assert_eq!(page_size, decoded);

        let json = serde_json::to_string(&rect).unwrap();
        let decoded: coord::PointsRect = serde_json::from_str(&json).unwrap();
        assert_eq!(rect, decoded);

        let json = serde_json::to_string(&pixel_rect).unwrap();
        let decoded: coord::PixelRect = serde_json::from_str(&json).unwrap();
        assert_eq!(pixel_rect, decoded);

        let json = serde_json::to_string(&dpi).unwrap();
        let decoded: coord::DpiScale = serde_json::from_str(&json).unwrap();
        assert_eq!(dpi, decoded);
    }
}
