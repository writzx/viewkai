use viewkai_core::page::PageIndex;
use viewkai_engine::{Document, init};

#[test]
fn open_hello_pdf() {
    if init().is_err() {
        eprintln!("open_hello_pdf: skipped — pdfium library not available");
        return;
    }

    let bytes = include_bytes!("../../../tests/fixtures/hello.pdf").to_vec();
    let doc = Document::from_bytes(bytes).expect("should open hello.pdf");

    assert_eq!(doc.page_count(), 1, "hello.pdf has 1 page");

    let size = doc.page_size(PageIndex(0)).expect("page 0 exists");
    assert!(size.width_pt > 0.0, "page width should be positive");
    assert!(size.height_pt > 0.0, "page height should be positive");

    assert!(
        (size.width_pt - 612.0).abs() < 1.0,
        "expected ~612 pt wide, got {}",
        size.width_pt
    );
}
