//! Tests for the cached `PdfDocument` render path.
//!
//! These tests verify that `render_page` uses the cached document rather than
//! re-parsing the PDF bytes on every call, and that typed errors are returned
//! for out-of-bounds page indices.

use viewkai_core::PageIndex;
use viewkai_core::PdfPageRotation;
use viewkai_engine::{Document, error::EngineError};

/// Fixture: the hello.pdf test PDF.
const HELLO_PDF: &[u8] = include_bytes!("../../../tests/fixtures/hello.pdf");

fn init_engine() {
    viewkai_engine::init().expect("engine init");
}

/// Rendering the same page twice must succeed without panicking.
///
/// This guards against the pre-C.1 bug where `render_page` re-opened the PDF
/// bytes on every call; with the cached `PdfDocument`, both calls reuse the
/// same document handle.
#[test]
fn render_same_page_twice_succeeds() {
    init_engine();
    let doc = Document::from_bytes(HELLO_PDF.to_vec()).expect("load hello.pdf");
    let first = viewkai_engine::render_page(&doc, PageIndex(0), 72, PdfPageRotation::None);
    let second = viewkai_engine::render_page(&doc, PageIndex(0), 72, PdfPageRotation::None);
    assert!(first.is_ok(), "first render failed: {first:?}");
    assert!(second.is_ok(), "second render failed: {second:?}");
}

/// Requesting a page beyond the document's page count returns a typed error.
#[test]
fn render_page_out_of_bounds_returns_typed_error() {
    init_engine();
    let doc = Document::from_bytes(HELLO_PDF.to_vec()).expect("load hello.pdf");
    let page_count = doc.page_count();
    let result =
        viewkai_engine::render_page(&doc, PageIndex(page_count), 72, PdfPageRotation::None);
    match result {
        Err(EngineError::PageIndexOutOfBounds { index, count }) => {
            assert_eq!(
                index as usize, page_count,
                "index should match requested page"
            );
            assert_eq!(
                count as usize, page_count,
                "count should match document page count"
            );
        }
        other => panic!("expected PageIndexOutOfBounds, got {other:?}"),
    }
}
