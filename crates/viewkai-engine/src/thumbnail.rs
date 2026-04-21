//! Thumbnail rendering helpers.

use crate::{
    Document, PDFIUM_OP_LOCK,
    error::{EngineError, Result},
};
use pdfium_render::prelude::*;
use viewkai_core::{PageIndex, PdfPageRotation, RawImage};

/// Render a page thumbnail with the requested pixel width.
///
/// Thumbnail rendering uses pdfium's lower-quality thumbnail preset but keeps
/// the output width fixed to `width_px`, scaling height proportionally.
///
/// # Errors
///
/// Returns an error if the page index is out of bounds or pdfium fails.
// justify: `pdfium-render` exposes page indices, pixel widths, and bitmap
// dimensions as signed C integer types (`PdfPageIndex` / `Pixels` / `c_int`).
// The conversions between `PageIndex(usize)`, the caller-provided `u32`
// `width_px`, and pdfium's `bitmap.width()` / `bitmap.height()` are bounded by
// the same pdfium-sized page geometry, so wraparound and sign loss cannot
// occur for any document pdfium has successfully opened.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]
pub fn render_thumbnail(
    doc: &Document,
    page: PageIndex,
    width_px: u32,
    rotation: PdfPageRotation,
) -> Result<RawImage> {
    let _pdfium_op_guard = PDFIUM_OP_LOCK
        .lock()
        .map_err(|_| EngineError::InitLockPoisoned)?;

    let page = doc
        .pdf
        .pages()
        .get(page.0 as PdfPageIndex)
        .map_err(|e| match e {
            PdfiumError::PageIndexOutOfBounds => EngineError::PageIndexOutOfBounds {
                index: page.0 as u32,
                count: doc.page_count() as u32,
            },
            _ => EngineError::Pdfium {
                message: e.to_string(),
            },
        })?;

    let mut config = PdfRenderConfig::new()
        .thumbnail(width_px as Pixels)
        .set_target_width(width_px as Pixels);
    if let Some(rotation) = match rotation {
        PdfPageRotation::None => None,
        PdfPageRotation::R90 => Some(PdfPageRenderRotation::Degrees90),
        PdfPageRotation::R180 => Some(PdfPageRenderRotation::Degrees180),
        PdfPageRotation::R270 => Some(PdfPageRenderRotation::Degrees270),
    } {
        config = config.rotate(rotation, true);
    }
    let bitmap = page
        .render_with_config(&config)
        .map_err(|e| EngineError::Pdfium {
            message: e.to_string(),
        })?;

    let actual_width = bitmap.width() as u32;
    let actual_height = bitmap.height() as u32;
    let pixels = bitmap.as_rgba_bytes();

    Ok(RawImage {
        width: actual_width,
        height: actual_height,
        pixels,
    })
}
