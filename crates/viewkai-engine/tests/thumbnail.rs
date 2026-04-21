//! Thumbnail rendering tests.

use viewkai_core::{PageIndex, PdfPageRotation};
use viewkai_engine::{Document, init, render_thumbnail};

#[test]
fn render_hello_thumbnail_dimensions() {
    if init().is_err() {
        eprintln!("render_hello_thumbnail_dimensions: skipped — pdfium library not available");
        return;
    }

    let bytes = include_bytes!("../../../tests/fixtures/hello.pdf").to_vec();
    let doc = Document::from_bytes(bytes).expect("load hello.pdf");

    let image = render_thumbnail(&doc, PageIndex(0), 120, PdfPageRotation::None)
        .expect("render thumbnail");

    assert_eq!(image.width, 120);
    assert!(image.height > 0);
    assert_eq!(image.pixels.len(), (image.width * image.height * 4) as usize);
}
