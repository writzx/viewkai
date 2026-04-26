//! Text extraction from PDF pages using pdfium-render.

use pdfium_render::prelude::*;
use viewkai_core::{GlyphBox, LineSpan, PageIndex, PageText, PointsRect, WordSpan};

use crate::Document;
use crate::error::{EngineError, Result};

/// Convert a pdfium `PdfRect` (Y-up, bottom-left origin) to a viewkai
/// `PointsRect` (Y-down, top-left origin).
///
/// `page_height_pt` is the page height in PDF points.
fn pdf_rect_to_viewkai(rect: PdfRect, page_height_pt: f32) -> PointsRect {
    let left = rect.left().value;
    let right = rect.right().value;
    let top = rect.top().value;
    let bottom = rect.bottom().value;

    PointsRect {
        x: left,
        y: page_height_pt - top,
        width: (right - left).abs(),
        height: (top - bottom).abs(),
    }
}

const GLYPH_BBOX_PAGE_FRACTION_LIMIT: f32 = 0.5;
const GLYPH_BBOX_PAGE_TOLERANCE_PT: f32 = 1.0;

fn glyph_bbox_is_plausible(bbox: PointsRect, page_rect: PointsRect) -> bool {
    if bbox.width <= 0.0 || bbox.height <= 0.0 {
        return false;
    }

    let max_width = page_rect.width * GLYPH_BBOX_PAGE_FRACTION_LIMIT;
    let max_height = page_rect.height * GLYPH_BBOX_PAGE_FRACTION_LIMIT;
    if bbox.width > max_width || bbox.height > max_height {
        return false;
    }

    bbox.x >= page_rect.x - GLYPH_BBOX_PAGE_TOLERANCE_PT
        && bbox.y >= page_rect.y - GLYPH_BBOX_PAGE_TOLERANCE_PT
        && bbox.x + bbox.width <= page_rect.x + page_rect.width + GLYPH_BBOX_PAGE_TOLERANCE_PT
        && bbox.y + bbox.height <= page_rect.y + page_rect.height + GLYPH_BBOX_PAGE_TOLERANCE_PT
}

/// Extract all text from a single PDF page.
///
/// Returns a [`PageText`] containing per-glyph bboxes, word groups, and line
/// groups. The coordinate system is Y-down with the origin at the page's
/// top-left corner.
///
/// # Errors
///
/// Returns an error if the page index is out of bounds or if pdfium fails to
/// extract text.
// justify: pdfium-render uses signed integer types for page indices and
// character counts; these casts are bounded by the document's actual page
// and character counts.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]
pub fn extract_page_text(doc: &Document, page_idx: PageIndex) -> Result<PageText> {
    use crate::PDFIUM_OP_LOCK;

    let _guard = PDFIUM_OP_LOCK
        .lock()
        .map_err(|_| EngineError::InitLockPoisoned)?;

    let page = doc
        .pdf
        .pages()
        .get(page_idx.0 as PdfPageIndex)
        .map_err(|e| match e {
            PdfiumError::PageIndexOutOfBounds => EngineError::PageIndexOutOfBounds {
                index: page_idx.0 as u32,
                count: doc.page_count() as u32,
            },
            _ => EngineError::Pdfium {
                message: e.to_string(),
            },
        })?;

    let page_rect = PointsRect {
        x: 0.0,
        y: 0.0,
        width: page.width().value,
        height: page.height().value,
    };
    let page_height_pt = page_rect.height;
    let page_text = page.text().map_err(|e| EngineError::Pdfium {
        message: e.to_string(),
    })?;

    let mut glyphs = Vec::new();

    for char_obj in page_text.chars().iter() {
        let Some(ch) = char_obj.unicode_char() else {
            continue;
        };
        if ch.is_control() && ch != ' ' && ch != '\t' {
            continue;
        }

        let Ok(rect) = char_obj.tight_bounds() else {
            continue;
        };
        let bbox = pdf_rect_to_viewkai(rect, page_height_pt);
        if !glyph_bbox_is_plausible(bbox, page_rect) {
            continue;
        }

        glyphs.push(GlyphBox {
            char: ch,
            bbox,
            font_size_pt: char_obj.scaled_font_size().value,
            rotation_deg: char_obj.angle_degrees().unwrap_or(0.0),
        });
    }

    let (words, lines) = group_glyphs(&glyphs, page_idx);

    Ok(PageText {
        glyphs,
        words,
        lines,
    })
}

/// Group glyphs into words and lines using whitespace + y-baseline clustering.
fn group_glyphs(glyphs: &[GlyphBox], page: PageIndex) -> (Vec<WordSpan>, Vec<LineSpan>) {
    if glyphs.is_empty() {
        return (Vec::new(), Vec::new());
    }

    let mut font_sizes: Vec<f32> = glyphs.iter().map(|glyph| glyph.font_size_pt).collect();
    font_sizes.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_font_size = font_sizes[font_sizes.len() / 2].max(1.0);
    let line_threshold = 0.5 * median_font_size;

    let mut lines = Vec::new();
    let mut current_line_start = 0_usize;
    let mut current_baseline = glyph_center_y(&glyphs[0]);

    for (index, glyph) in glyphs.iter().enumerate().skip(1) {
        let glyph_baseline = glyph_center_y(glyph);
        if (glyph_baseline - current_baseline).abs() > line_threshold {
            lines.push(LineSpan {
                page,
                y_baseline_pt: current_baseline,
                start_char: current_line_start,
                end_char: index,
            });
            current_line_start = index;
            current_baseline = glyph_baseline;
        }
    }

    lines.push(LineSpan {
        page,
        y_baseline_pt: current_baseline,
        start_char: current_line_start,
        end_char: glyphs.len(),
    });

    let mut words = Vec::new();
    for line in &lines {
        let mut current_word_start = None;
        let mut current_word_bbox = None;

        for glyph_index in line.start_char..line.end_char {
            let glyph = &glyphs[glyph_index];

            if glyph.char.is_whitespace() {
                push_word(
                    &mut words,
                    page,
                    &mut current_word_start,
                    glyph_index,
                    &mut current_word_bbox,
                );
                continue;
            }

            let should_split = if let Some(previous_index) = glyph_index.checked_sub(1) {
                previous_index >= line.start_char && is_large_gap(&glyphs[previous_index], glyph)
            } else {
                false
            };

            if should_split {
                push_word(
                    &mut words,
                    page,
                    &mut current_word_start,
                    glyph_index,
                    &mut current_word_bbox,
                );
            }

            if current_word_start.is_none() {
                current_word_start = Some(glyph_index);
                current_word_bbox = Some(glyph.bbox);
            } else if let Some(bbox) = &mut current_word_bbox {
                *bbox = union_rect(*bbox, glyph.bbox);
            }
        }

        push_word(
            &mut words,
            page,
            &mut current_word_start,
            line.end_char,
            &mut current_word_bbox,
        );
    }

    (words, lines)
}

fn glyph_center_y(glyph: &GlyphBox) -> f32 {
    glyph.bbox.y + glyph.bbox.height / 2.0
}

fn is_large_gap(previous: &GlyphBox, current: &GlyphBox) -> bool {
    let advance = previous.bbox.width.max(1.0);
    let gap = current.bbox.x - (previous.bbox.x + previous.bbox.width);
    gap > 0.3 * advance
}

fn union_rect(a: PointsRect, b: PointsRect) -> PointsRect {
    let left = a.x.min(b.x);
    let top = a.y.min(b.y);
    let right = (a.x + a.width).max(b.x + b.width);
    let bottom = (a.y + a.height).max(b.y + b.height);

    PointsRect {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
    }
}

fn push_word(
    words: &mut Vec<WordSpan>,
    page: PageIndex,
    current_word_start: &mut Option<usize>,
    end_char: usize,
    current_word_bbox: &mut Option<PointsRect>,
) {
    if let (Some(start_char), Some(bbox)) = (*current_word_start, *current_word_bbox)
        && start_char < end_char
    {
        words.push(WordSpan {
            page,
            start_char,
            end_char,
            bbox,
        });
    }

    *current_word_start = None;
    *current_word_bbox = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page_rect() -> PointsRect {
        PointsRect {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 200.0,
        }
    }

    #[test]
    fn rejects_implausibly_large_glyph_bbox() {
        assert!(!glyph_bbox_is_plausible(
            PointsRect {
                x: 10.0,
                y: 10.0,
                width: 60.0,
                height: 12.0,
            },
            page_rect(),
        ));
    }

    #[test]
    fn rejects_out_of_page_glyph_bbox() {
        assert!(!glyph_bbox_is_plausible(
            PointsRect {
                x: -5.0,
                y: 10.0,
                width: 8.0,
                height: 12.0,
            },
            page_rect(),
        ));
    }

    #[test]
    fn accepts_glyph_bbox_with_small_page_tolerance() {
        assert!(glyph_bbox_is_plausible(
            PointsRect {
                x: -0.5,
                y: 10.0,
                width: 8.0,
                height: 12.0,
            },
            page_rect(),
        ));
    }
}
