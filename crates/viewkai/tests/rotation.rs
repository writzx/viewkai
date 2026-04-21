//! Rotation state API tests.

use viewkai::{PdfPageRotation, RotationDelta, Viewer};
use viewkai_core::PageIndex;

fn viewer_with_hello() -> Viewer {
    viewkai_engine::init().expect("pdfium init");
    let mut viewer = Viewer::new();
    viewer
        .load_bytes(include_bytes!("../../../tests/fixtures/hello.pdf").to_vec())
        .expect("load hello.pdf");
    viewer
}

#[test]
fn rotate_page_cycle() {
    let mut viewer = viewer_with_hello();
    let page = PageIndex(0);

    viewer.rotate_page(page, RotationDelta::Clockwise);
    assert_eq!(viewer.rotation_of(page), PdfPageRotation::R90);
    viewer.rotate_page(page, RotationDelta::Clockwise);
    assert_eq!(viewer.rotation_of(page), PdfPageRotation::R180);
    viewer.rotate_page(page, RotationDelta::Clockwise);
    assert_eq!(viewer.rotation_of(page), PdfPageRotation::R270);
    viewer.rotate_page(page, RotationDelta::Clockwise);
    assert_eq!(viewer.rotation_of(page), PdfPageRotation::None);
}

#[test]
fn rotate_all_applies_to_every_page() {
    viewkai_engine::init().expect("pdfium init");
    let mut viewer = Viewer::new();
    viewer
        .load_bytes(include_bytes!("../../../tests/fixtures/500page.pdf").to_vec())
        .expect("load 500page.pdf");

    viewer.rotate_all(RotationDelta::CounterClockwise);

    assert_eq!(viewer.rotation_of(PageIndex(0)), PdfPageRotation::R270);
    assert_eq!(viewer.rotation_of(PageIndex(1)), PdfPageRotation::R270);
    assert_eq!(
        viewer.rotation_of(PageIndex(viewer.page_count() - 1)),
        PdfPageRotation::R270
    );
}

#[test]
fn reset_rotations_clears() {
    let mut viewer = viewer_with_hello();
    let page = PageIndex(0);
    viewer.rotate_page(page, RotationDelta::Clockwise);
    assert_eq!(viewer.rotation_of(page), PdfPageRotation::R90);

    viewer.reset_rotations();

    assert_eq!(viewer.rotation_of(page), PdfPageRotation::None);
}
