//! Core types for viewkai: coordinates, page metadata, and render output.

use serde::{Deserialize, Serialize};

/// A position measured in PDF points.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PointsPos {
    /// X coordinate in PDF points.
    pub x: f32,
    /// Y coordinate in PDF points.
    pub y: f32,
}

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
pub struct DpiScale(
    /// Raw multiplicative scale from points to pixels.
    pub f32,
);

impl DpiScale {
    /// Identity scale where one point maps to one pixel.
    pub const IDENTITY: Self = DpiScale(1.0);

    /// Convert a length in points to pixels using this scale.
    #[must_use]
    pub fn points_to_pixels(self, pts: f32) -> f32 {
        pts * self.0
    }
}

/// Zero-based page index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PageIndex(
    /// Zero-based page position within the document.
    pub usize,
);

/// Page dimensions in PDF points (1 pt = 1/72 inch).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PageSize {
    /// Page width in PDF points.
    pub width_pt: f32,
    /// Page height in PDF points.
    pub height_pt: f32,
}

/// A raw RGBA image buffer produced by the rendering engine.
///
/// Pixels are stored in row-major order, 4 bytes per pixel (R, G, B, A).
#[derive(Debug, Clone)]
pub struct RawImage {
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
    /// RGBA bytes, row-major. Length == width * height * 4.
    pub pixels: Vec<u8>,
}
