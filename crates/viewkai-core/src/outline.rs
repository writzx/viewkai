//! Document outline (table of contents) types.

use crate::types::{PageIndex, PointsRect};

/// Stable identifier for an outline node within an [`Outline`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct OutlineNodeId(pub u32);

/// A single outline entry.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct OutlineNode {
    /// Stable node identifier.
    pub id: OutlineNodeId,
    /// User-visible bookmark title.
    pub title: String,
    /// Target destination, if this bookmark points at a page location.
    pub destination: Option<Destination>,
    /// Parent node, or `None` for top-level roots.
    pub parent: Option<OutlineNodeId>,
    /// Child node identifiers in reading order.
    pub children: Vec<OutlineNodeId>,
}

/// A bookmark destination inside a document.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Destination {
    /// Target page.
    pub page: PageIndex,
    /// Optional target position / fit mode on that page.
    pub position: Option<DestPosition>,
}

/// Positioning instructions for an outline destination.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum DestPosition {
    /// Scroll to a specific point on the page.
    Point {
        /// Horizontal position in PDF points.
        x_pt: f32,
        /// Vertical position in PDF points.
        y_pt: f32,
    },
    /// Fit the whole page in view.
    FitPage,
    /// Fit page width, optionally preserving a y coordinate.
    FitWidth {
        /// Optional vertical position in PDF points.
        y_pt: Option<f32>,
    },
    /// Fit page height, optionally preserving an x coordinate.
    FitHeight {
        /// Optional horizontal position in PDF points.
        x_pt: Option<f32>,
    },
    /// Fit a specific rectangle.
    FitRect {
        /// Bounding box to fit in PDF points.
        bbox: PointsRect,
    },
}

/// A document outline (table of contents) with flat node storage.
///
/// Nodes are stored in a flat `Vec`; parent/child relationships use IDs.
/// `roots` contains the top-level entry IDs in reading order.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, Default)]
pub struct Outline {
    /// Flat node storage.
    pub nodes: Vec<OutlineNode>,
    /// Top-level node IDs in reading order.
    pub roots: Vec<OutlineNodeId>,
}

impl Outline {
    /// Returns true if this outline has no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.roots.is_empty()
    }

    /// Look up a node by its ID.
    #[must_use]
    pub fn node(&self, id: OutlineNodeId) -> Option<&OutlineNode> {
        self.nodes.iter().find(|node| node.id == id)
    }
}
