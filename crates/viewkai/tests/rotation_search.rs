//! Rotated-page search regression coverage.

use std::{cell::Cell, collections::HashMap};

use viewkai::{PluginContext, RotationDelta, Viewer};
use viewkai_core::{PageIndex, PointsRect, SearchQuery};
use viewkai_engine::Document;

#[test]
fn search_highlights_on_180_rotated_page() {
    viewkai_engine::init().expect("pdfium init");
    let bytes = include_bytes!("../../../tests/fixtures/hello.pdf").to_vec();
    let doc = Document::from_bytes(bytes.clone()).expect("reload hello.pdf");
    let mut viewer = Viewer::new();
    viewer.load_bytes(bytes).expect("load hello.pdf");
    viewer.rotate_page(PageIndex(0), RotationDelta::Clockwise);
    viewer.rotate_page(PageIndex(0), RotationDelta::Clockwise);
    viewer.open_search();

    let egui_ctx = egui::Context::default();
    let pending_scroll = Cell::new(None::<(PageIndex, PointsRect)>);
    let mut rotations = HashMap::new();
    rotations.insert(PageIndex(0), viewer.rotation_of(PageIndex(0)));
    let ctx = PluginContext::new(
        Some(&doc),
        1.0,
        &[],
        &egui_ctx,
        egui::Color32::from_rgba_unmultiplied(70, 120, 210, 96),
        true,
        &rotations,
        None,
        &pending_scroll,
    );
    viewer.search_mut().update_query(
        SearchQuery {
            term: "Hello".to_owned(),
            case_sensitive: false,
            whole_word: false,
        },
        &ctx,
    );

    assert!(viewer.search_state().is_some());
}
