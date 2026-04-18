use serde::{Deserialize, Serialize};

/// Zero-based page index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PageIndex(pub usize);

/// Page dimensions in PDF points (1 pt = 1/72 inch).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PageSize {
    pub width_pt: f32,
    pub height_pt: f32,
}
