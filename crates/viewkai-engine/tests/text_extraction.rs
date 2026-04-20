//! Text extraction integration tests.

use std::sync::{Mutex, OnceLock};

use viewkai_core::PageIndex;
use viewkai_engine::{Document, init};

fn test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[test]
fn hello_pdf_has_one_word() {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    init().expect("pdfium init");
    let bytes = include_bytes!("../../../tests/fixtures/hello.pdf").to_vec();
    let doc = Document::from_bytes(bytes).expect("load hello.pdf");
    let text = doc.page_text(PageIndex(0)).expect("extract text");
    assert!(
        !text.words.is_empty(),
        "hello.pdf should have at least one word"
    );
}

#[test]
fn text_cache_reuses_arc() {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    init().expect("pdfium init");
    let bytes = include_bytes!("../../../tests/fixtures/hello.pdf").to_vec();
    let doc = Document::from_bytes(bytes).expect("load hello.pdf");
    let text1 = doc.page_text(PageIndex(0)).expect("first call");
    let text2 = doc.page_text(PageIndex(0)).expect("second call");
    assert!(
        std::sync::Arc::ptr_eq(&text1, &text2),
        "second call should return cached Arc"
    );
}

#[test]
fn glyph_coord_transform_y_flip() {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    init().expect("pdfium init");
    let bytes = include_bytes!("../../../tests/fixtures/hello.pdf").to_vec();
    let doc = Document::from_bytes(bytes).expect("load hello.pdf");
    let text = doc.page_text(PageIndex(0)).expect("extract text");

    for glyph in &text.glyphs {
        assert!(glyph.bbox.x >= 0.0, "glyph x should be non-negative");
        assert!(glyph.bbox.y >= 0.0, "glyph y should be non-negative");
        assert!(glyph.bbox.width > 0.0, "glyph width should be positive");
        assert!(glyph.bbox.height > 0.0, "glyph height should be positive");
    }
}
