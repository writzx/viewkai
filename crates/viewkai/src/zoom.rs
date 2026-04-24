//! Zoom state for viewkai.

/// DPI rendering buckets used for zoom quantization.
const BUCKETS: [u32; 6] = [72, 96, 144, 216, 288, 432];

/// Zoom level for the viewer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ZoomState {
    /// A discrete zoom level (e.g., 1.0 = 100%, 2.0 = 200%).
    Discrete(f32),
    /// Fit the page width to the available viewport width.
    FitWidth,
    /// Fit the entire page within the viewport.
    FitPage,
    /// A custom continuous zoom level (e.g., from pinch-to-zoom).
    Custom(f32),
}

impl Default for ZoomState {
    fn default() -> Self {
        Self::Discrete(1.0)
    }
}

impl ZoomState {
    /// Compute the effective zoom factor given the viewport and page dimensions.
    ///
    /// - `viewport_width`: available width in pixels
    /// - `viewport_height`: available height in pixels
    /// - `page_width_pt`: page width in PDF points
    /// - `page_height_pt`: page height in PDF points
    ///
    /// Returns a scale factor where 1.0 means 1 PDF point = 1 pixel.
    #[must_use]
    pub fn effective_zoom(
        &self,
        viewport_width: f32,
        viewport_height: f32,
        page_width_pt: f32,
        page_height_pt: f32,
    ) -> f32 {
        match self {
            Self::Discrete(zoom) | Self::Custom(zoom) => *zoom,
            Self::FitWidth => {
                if page_width_pt > 0.0 {
                    viewport_width / page_width_pt
                } else {
                    1.0
                }
            }
            Self::FitPage => {
                if page_width_pt > 0.0 && page_height_pt > 0.0 {
                    let scale_w = viewport_width / page_width_pt;
                    let scale_h = viewport_height / page_height_pt;
                    scale_w.min(scale_h)
                } else {
                    1.0
                }
            }
        }
    }

    /// Map an effective zoom factor to the nearest DPI bucket.
    ///
    /// Buckets: [72, 96, 144, 216, 288, 432] DPI.
    // justify: zoom buckets are a tiny non-negative table and caller-provided
    // zoom values are clamped by viewer behavior to sensible rendering scales.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    #[must_use]
    pub fn zoom_to_dpi_bucket(zoom: f32) -> u32 {
        Self::zoom_to_dpi_bucket_with_dpr(zoom, 1.0)
    }

    /// Map an effective zoom factor plus DPR to the nearest DPI bucket.
    // justify: zoom buckets are a tiny non-negative table and caller-provided
    // zoom/DPR values are clamped by viewer behavior to sensible rendering scales.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    #[must_use]
    pub(crate) fn zoom_to_dpi_bucket_with_dpr(zoom: f32, pixels_per_point: f32) -> u32 {
        let target_dpi = (zoom * pixels_per_point * 72.0).round() as u32;

        BUCKETS
            .iter()
            .min_by_key(|&&bucket| (i64::from(bucket) - i64::from(target_dpi)).unsigned_abs())
            .copied()
            .unwrap_or(72)
    }

    /// Map a DPI value to a zoom bucket index (0-5).
    #[must_use]
    pub fn dpi_to_bucket_index(dpi: u32) -> u8 {
        BUCKETS
            .iter()
            .position(|&bucket| bucket == dpi)
            .and_then(|index| u8::try_from(index).ok())
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discrete_zoom_returns_value() {
        let z = ZoomState::Discrete(1.5);
        assert!((z.effective_zoom(800.0, 600.0, 612.0, 792.0) - 1.5).abs() < 0.001);
    }

    #[test]
    fn fit_width_scales_to_viewport() {
        let z = ZoomState::FitWidth;
        let zoom = z.effective_zoom(800.0, 600.0, 612.0, 792.0);
        assert!((zoom - 800.0 / 612.0).abs() < 0.001);
    }

    #[test]
    fn fit_page_uses_smaller_scale() {
        let z = ZoomState::FitPage;
        let zoom = z.effective_zoom(800.0, 600.0, 612.0, 792.0);
        assert!((zoom - 600.0 / 792.0).abs() < 0.001);
    }

    #[test]
    fn dpi_bucket_mapping() {
        assert_eq!(ZoomState::zoom_to_dpi_bucket(1.0), 72);
        assert_eq!(ZoomState::zoom_to_dpi_bucket(2.0), 144);
        assert_eq!(ZoomState::zoom_to_dpi_bucket(1.333), 96);
        assert_eq!(ZoomState::zoom_to_dpi_bucket_with_dpr(1.0, 2.0), 144);
        assert_eq!(ZoomState::zoom_to_dpi_bucket_with_dpr(1.0, 1.0), 72);
    }
}
