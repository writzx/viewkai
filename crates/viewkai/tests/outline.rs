//! Viewer outline integration tests.

use viewkai::{Destination, DestPosition, Viewer, ViewerPlugin};
use viewkai_core::PageIndex;

#[test]
fn viewer_outline_accessor_returns_plugin() {
    let viewer = Viewer::new();
    assert_eq!(viewer.outline().id(), "viewkai.outline");
}

#[test]
fn outline_document_returns_loaded_outline() {
    viewkai_engine::init().expect("pdfium init");
    let bytes = include_bytes!("../../../tests/fixtures/bookmarks.pdf").to_vec();
    let mut viewer = Viewer::new();
    viewer.load_bytes(bytes).expect("load bookmarks.pdf");

    let outline = viewer.outline_document().expect("loaded outline");
    assert_eq!(outline.roots.len(), 3);
}

#[test]
fn goto_destination_sets_pending_destination() {
    let mut viewer = Viewer::new();
    let dest = Destination {
        page: PageIndex(3),
        position: Some(DestPosition::Point {
            x_pt: 40.0,
            y_pt: 80.0,
        }),
    };

    viewer.goto_destination(dest.clone());

    assert_eq!(viewer.outline().pending_destination(), Some(&dest));
}
