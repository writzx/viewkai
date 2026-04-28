use crate::{PageSize, PointsPos, PointsRect};

/// Display-time page rotation (does NOT modify the underlying PDF).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
pub enum PdfPageRotation {
    /// No rotation (0°).
    #[default]
    None,
    /// 90° clockwise.
    R90,
    /// 180°.
    R180,
    /// 270° clockwise (= 90° counter-clockwise).
    R270,
}

impl PdfPageRotation {
    /// Returns the rotation in degrees (0, 90, 180, 270).
    #[must_use]
    pub fn as_degrees(self) -> u16 {
        match self {
            Self::None => 0,
            Self::R90 => 90,
            Self::R180 => 180,
            Self::R270 => 270,
        }
    }

    /// Apply a delta step.
    #[must_use]
    pub fn apply(self, delta: RotationDelta) -> Self {
        match delta {
            RotationDelta::Clockwise => match self {
                Self::None => Self::R90,
                Self::R90 => Self::R180,
                Self::R180 => Self::R270,
                Self::R270 => Self::None,
            },
            RotationDelta::CounterClockwise => match self {
                Self::None => Self::R270,
                Self::R90 => Self::None,
                Self::R180 => Self::R90,
                Self::R270 => Self::R180,
            },
        }
    }
}

/// A single rotation step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RotationDelta {
    /// Rotate 90° clockwise.
    Clockwise,
    /// Rotate 90° counter-clockwise.
    CounterClockwise,
}

/// Returns the page size after applying a display-time rotation.
#[must_use]
pub fn rotated_page_size(page_size: PageSize, rotation: PdfPageRotation) -> PageSize {
    match rotation {
        PdfPageRotation::R90 | PdfPageRotation::R270 => PageSize {
            width_pt: page_size.height_pt,
            height_pt: page_size.width_pt,
        },
        PdfPageRotation::None | PdfPageRotation::R180 => page_size,
    }
}

/// Converts a rendered-page point back into native PDF coordinates.
#[must_use]
pub fn inverse_rotate_point(
    point: PointsPos,
    rotation: PdfPageRotation,
    page_size: PageSize,
) -> PointsPos {
    match rotation {
        PdfPageRotation::None => point,
        PdfPageRotation::R90 => PointsPos {
            x: point.y,
            y: page_size.height_pt - point.x,
        },
        PdfPageRotation::R180 => PointsPos {
            x: page_size.width_pt - point.x,
            y: page_size.height_pt - point.y,
        },
        PdfPageRotation::R270 => PointsPos {
            x: page_size.width_pt - point.y,
            y: point.x,
        },
    }
}

/// Converts a native PDF point into rendered-page coordinates.
#[must_use]
pub fn forward_rotate_point(
    point: PointsPos,
    rotation: PdfPageRotation,
    page_size: PageSize,
) -> PointsPos {
    match rotation {
        PdfPageRotation::None => point,
        PdfPageRotation::R90 => PointsPos {
            x: page_size.height_pt - point.y,
            y: point.x,
        },
        PdfPageRotation::R180 => PointsPos {
            x: page_size.width_pt - point.x,
            y: page_size.height_pt - point.y,
        },
        PdfPageRotation::R270 => PointsPos {
            x: point.y,
            y: page_size.width_pt - point.x,
        },
    }
}

/// Converts a native PDF rect into rendered-page coordinates.
#[must_use]
pub fn forward_rotate_rect(
    rect: PointsRect,
    rotation: PdfPageRotation,
    page_size: PageSize,
) -> PointsRect {
    let corners = [
        PointsPos {
            x: rect.x,
            y: rect.y,
        },
        PointsPos {
            x: rect.x + rect.width,
            y: rect.y,
        },
        PointsPos {
            x: rect.x,
            y: rect.y + rect.height,
        },
        PointsPos {
            x: rect.x + rect.width,
            y: rect.y + rect.height,
        },
    ]
    .map(|point| forward_rotate_point(point, rotation, page_size));

    let min_x = corners
        .iter()
        .map(|point| point.x)
        .fold(f32::INFINITY, f32::min);
    let max_x = corners
        .iter()
        .map(|point| point.x)
        .fold(f32::NEG_INFINITY, f32::max);
    let min_y = corners
        .iter()
        .map(|point| point.y)
        .fold(f32::INFINITY, f32::min);
    let max_y = corners
        .iter()
        .map(|point| point.y)
        .fold(f32::NEG_INFINITY, f32::max);

    PointsRect {
        x: min_x,
        y: min_y,
        width: max_x - min_x,
        height: max_y - min_y,
    }
}
