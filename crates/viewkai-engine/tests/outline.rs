//! Outline extraction tests.

use std::sync::Arc;

use viewkai_engine::{Document, init};

#[test]
fn empty_pdf_has_no_outline() {
    if init().is_err() {
        eprintln!("empty_pdf_has_no_outline: skipped — pdfium library not available");
        return;
    }

    let bytes = include_bytes!("../../../tests/fixtures/hello.pdf").to_vec();
    let doc = Document::from_bytes(bytes).expect("should open hello.pdf");
    let outline = doc.outline().expect("extract outline");

    assert!(outline.is_empty());
    assert!(outline.nodes.is_empty());
}

#[test]
fn fixture_pdf_with_bookmarks_extracts_tree() {
    if init().is_err() {
        eprintln!("fixture_pdf_with_bookmarks_extracts_tree: skipped — pdfium library not available");
        return;
    }

    let bytes = include_bytes!("../../../tests/fixtures/bookmarks.pdf").to_vec();
    let doc = Document::from_bytes(bytes).expect("should open bookmarks.pdf");
    let outline = doc.outline().expect("extract outline");

    assert_eq!(outline.roots.len(), 3);
    assert_eq!(outline.nodes.len(), 9);
    for &root in &outline.roots {
        let node = outline.node(root).expect("root node present");
        assert_eq!(node.children.len(), 2);
    }
}

#[test]
fn outline_is_cached() {
    if init().is_err() {
        eprintln!("outline_is_cached: skipped — pdfium library not available");
        return;
    }

    let bytes = include_bytes!("../../../tests/fixtures/bookmarks.pdf").to_vec();
    let doc = Document::from_bytes(bytes).expect("should open bookmarks.pdf");

    let first = doc.outline().expect("first outline");
    let second = doc.outline().expect("second outline");

    assert!(Arc::ptr_eq(&first, &second));
}
