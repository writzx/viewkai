//! Regression coverage for oversized text-layer word bounds.

use std::sync::{Mutex, OnceLock};

use viewkai_core::PageIndex;
use viewkai_engine::Document;

fn test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[test]
fn glyph_bboxes_are_subpage_sized() {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    viewkai_engine::init().expect("pdfium init");

    let bytes = include_bytes!("../../../tests/fixtures/hello.pdf").to_vec();
    let doc = Document::from_bytes(bytes).expect("load hello.pdf");
    let page = PageIndex(0);
    let page_size = doc.page_size(page).expect("page size");
    let text = doc.page_text(page).expect("extract text");

    assert!(!text.words.is_empty(), "hello.pdf should have extracted words");
    for word in &text.words {
        assert!(
            word.bbox.width < page_size.width_pt * 0.5,
            "word bbox {:?} exceeded half-page width {}",
            word.bbox,
            page_size.width_pt * 0.5
        );
    }
}
