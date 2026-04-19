use serde::{Deserialize, Serialize};

/// Zero-based page index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PageIndex(pub usize);

/// Page dimensions in PDF points (1 pt = 1/72 inch).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PageSize {
    /// Page width in PDF points.
    pub width_pt: f32,
    /// Page height in PDF points.
    pub height_pt: f32,
}
