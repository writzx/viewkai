//! Rotated-page selection regression coverage.

use viewkai::{RotationDelta, Viewer};
use viewkai_core::PageIndex;

#[test]
fn select_text_on_90_rotated_page() {
    viewkai_engine::init().expect("pdfium init");
    let mut baseline = Viewer::new();
    baseline
        .load_bytes(include_bytes!("../../../tests/fixtures/hello.pdf").to_vec())
        .expect("load hello.pdf baseline");
    baseline.select_all();
    let expected = baseline.selected_text();

    let mut viewer = Viewer::new();
    viewer
        .load_bytes(include_bytes!("../../../tests/fixtures/hello.pdf").to_vec())
        .expect("load hello.pdf");

    viewer.rotate_page(PageIndex(0), RotationDelta::Clockwise);
    viewer.select_all();

    assert_eq!(viewer.selected_text(), expected);
}
