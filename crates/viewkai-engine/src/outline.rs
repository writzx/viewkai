//! Outline extraction support built on top of pdfium-render.

use pdfium_render::prelude::PdfDestinationViewSettings;
use viewkai_core::{Outline, OutlineNode, OutlineNodeId, PageIndex, PointsRect, outline::*};

use crate::Document;
use crate::error::{EngineError, Result};

/// Extract the document outline (PDF bookmarks) into a flat tree.
pub fn extract_outline(doc: &Document) -> Result<Outline> {
    let _pdfium_op_guard = crate::PDFIUM_OP_LOCK
        .lock()
        .map_err(|_| EngineError::InitLockPoisoned)?;

    let mut outline = Outline::default();
    let mut next_id = 0_u32;

    let mut root = doc.pdf.bookmarks().root();
    while let Some(bookmark) = root {
        let root_id = push_bookmark(doc, &bookmark, None, &mut outline, &mut next_id)?;
        outline.roots.push(root_id);
        root = bookmark.next_sibling();
    }

    Ok(outline)
}

fn push_bookmark(
    doc: &Document,
    bookmark: &pdfium_render::prelude::PdfBookmark<'_>,
    parent: Option<OutlineNodeId>,
    outline: &mut Outline,
    next_id: &mut u32,
) -> Result<OutlineNodeId> {
    let id = OutlineNodeId(*next_id);
    *next_id += 1;

    let node_index = outline.nodes.len();
    outline.nodes.push(OutlineNode {
        id,
        title: bookmark.title().unwrap_or_default(),
        destination: bookmark
            .destination()
            .map(|dest| map_destination(&dest))
            .transpose()?,
        parent,
        children: Vec::new(),
    });

    let mut child_ids = Vec::new();
    for child in bookmark.iter_direct_children() {
        child_ids.push(push_bookmark(doc, &child, Some(id), outline, next_id)?);
    }

    outline.nodes[node_index].children = child_ids;
    Ok(id)
}

fn map_destination(dest: &pdfium_render::prelude::PdfDestination<'_>) -> Result<Destination> {
    let page = PageIndex(dest.page_index().map_err(|err| EngineError::Pdfium {
        message: err.to_string(),
    })? as usize);

    let position = match dest.view_settings().map_err(|err| EngineError::Pdfium {
        message: err.to_string(),
    })? {
        PdfDestinationViewSettings::SpecificCoordinatesAndZoom(Some(x), Some(y), _) => {
            Some(DestPosition::Point {
                x_pt: x.value,
                y_pt: y.value,
            })
        }
        PdfDestinationViewSettings::FitPageToWindow => Some(DestPosition::FitPage),
        PdfDestinationViewSettings::FitPageHorizontallyToWindow(y) => {
            Some(DestPosition::FitWidth {
                y_pt: y.map(|value| value.value),
            })
        }
        PdfDestinationViewSettings::FitPageVerticallyToWindow(x) => Some(DestPosition::FitHeight {
            x_pt: x.map(|value| value.value),
        }),
        PdfDestinationViewSettings::FitPageToRectangle(rect) => Some(DestPosition::FitRect {
            bbox: PointsRect {
                x: rect.left().value,
                y: rect.bottom().value,
                width: rect.right().value - rect.left().value,
                height: rect.top().value - rect.bottom().value,
            },
        }),
        PdfDestinationViewSettings::Unknown
        | PdfDestinationViewSettings::SpecificCoordinatesAndZoom(_, _, _)
        | PdfDestinationViewSettings::FitBoundsToWindow
        | PdfDestinationViewSettings::FitBoundsHorizontallyToWindow(_)
        | PdfDestinationViewSettings::FitBoundsVerticallyToWindow(_) => None,
    };

    Ok(Destination { page, position })
}
