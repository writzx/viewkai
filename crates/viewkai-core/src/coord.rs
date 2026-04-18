use serde::{Deserialize, Serialize};

/// A rectangle measured in PDF points.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PointsRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// A rectangle measured in screen pixels.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PixelRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// DPI scale factor mapping points → pixels.
///
/// `pixels = points * dpi_scale`
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DpiScale(pub f32);

impl DpiScale {
    pub const IDENTITY: Self = DpiScale(1.0);

    /// Convert a length in points to pixels using this scale.
    pub fn points_to_pixels(self, pts: f32) -> f32 {
        pts * self.0
    }
}
