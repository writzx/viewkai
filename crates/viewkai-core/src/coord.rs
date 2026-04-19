use serde::{Deserialize, Serialize};

/// A rectangle measured in PDF points.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PointsRect {
    /// Left edge in PDF points.
    pub x: f32,
    /// Top edge in PDF points.
    pub y: f32,
    /// Rectangle width in PDF points.
    pub width: f32,
    /// Rectangle height in PDF points.
    pub height: f32,
}

/// A rectangle measured in screen pixels.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PixelRect {
    /// Left edge in screen pixels.
    pub x: i32,
    /// Top edge in screen pixels.
    pub y: i32,
    /// Rectangle width in screen pixels.
    pub width: u32,
    /// Rectangle height in screen pixels.
    pub height: u32,
}

/// DPI scale factor mapping points → pixels.
///
/// `pixels = points * dpi_scale`
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DpiScale(pub f32);

impl DpiScale {
    /// Identity scale where one point maps to one pixel.
    pub const IDENTITY: Self = DpiScale(1.0);

    /// Convert a length in points to pixels using this scale.
    #[must_use]
    pub fn points_to_pixels(self, pts: f32) -> f32 {
        pts * self.0
    }
}
