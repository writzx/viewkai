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
