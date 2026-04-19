//! Full-text search using pdfium-render's page text search API.

use pdfium_render::prelude::*;
use viewkai_core::text::CharSpan;
use viewkai_core::{PageIndex, PointsRect, SearchMatch, SearchQuery};

use crate::error::{EngineError, Result};
use crate::Document;

/// Search a single page for matches of the given query.
///
/// Returns a list of [`SearchMatch`] objects, each with page-local rects.
///
/// # Errors
///
/// Returns an error if the page index is out of bounds or pdfium fails.
// justify: pdfium-render uses signed integer types for page indices and
// character counts; these casts are bounded by the document's actual counts.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]
pub fn search_page(
    doc: &Document,
    page_idx: PageIndex,
    query: &SearchQuery,
) -> Result<Vec<SearchMatch>> {
    use crate::PDFIUM_OP_LOCK;

    if query.term.is_empty() {
        return Ok(Vec::new());
    }

    let char_spans = {
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

        let page_text = page.text().map_err(|e| EngineError::Pdfium {
            message: e.to_string(),
        })?;

        let mut options = PdfSearchOptions::new();
        if query.case_sensitive {
            options = options.match_case(true);
        }
        if query.whole_word {
            options = options.match_whole_word(true);
        }

        let search = page_text.search(&query.term, &options).map_err(|e| EngineError::Pdfium {
            message: e.to_string(),
        })?;

        search
            .iter(PdfSearchDirection::SearchForward)
            .filter_map(|segments| {
                let mut start = None;
                let mut end = None;

                for segment in segments.iter() {
                    let chars = segment.chars().ok()?;
                    let segment_start = chars.first_char_index()?;
                    let segment_end = chars.last_char_index()?.saturating_add(1);
                    start = Some(
                        start.map_or(segment_start, |current: usize| current.min(segment_start)),
                    );
                    end =
                        Some(end.map_or(segment_end, |current: usize| current.max(segment_end)));
                }

                let (start, end) = (start?, end?);

                Some(CharSpan {
                    page: page_idx,
                    start,
                    end,
                })
            })
            .collect::<Vec<_>>()
    };

    let page_text = doc.page_text(page_idx)?;

    Ok(char_spans
        .into_iter()
        .map(|char_span| SearchMatch {
            page: page_idx,
            rects: rects_for_char_span(&page_text, char_span),
            char_span,
        })
        .collect())
}

fn rects_for_char_span(page_text: &viewkai_core::PageText, char_span: CharSpan) -> Vec<PointsRect> {
    let mut rects = Vec::new();

    for line in &page_text.lines {
        let start = char_span.start.max(line.start_char);
        let end = char_span.end.min(line.end_char);
        if start >= end {
            continue;
        }

        let Some(glyphs) = page_text.glyphs.get(start..end) else {
            continue;
        };
        let Some((first, rest)) = glyphs.split_first() else {
            continue;
        };

        let mut union = first.bbox;
        for glyph in rest {
            union = union_rect(union, glyph.bbox);
        }
        rects.push(union);
    }

    if rects.is_empty()
        && let Some(glyphs) = page_text.glyphs.get(char_span.start..char_span.end)
        && let Some((first, rest)) = glyphs.split_first()
    {
        let mut union = first.bbox;
        for glyph in rest {
            union = union_rect(union, glyph.bbox);
        }
        rects.push(union);
    }

    rects
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
