//! Find navigation regression coverage.

use std::{cell::Cell, sync::Mutex};
use std::collections::HashMap;

use egui_kittest::Harness;
use viewkai::{PluginContext, Viewer};
use viewkai_core::{PageIndex, PointsRect, SearchQuery};
use viewkai_engine::Document;

static TEST_LOCK: Mutex<()> = Mutex::new(());

fn query_hello(viewer: &mut Viewer, term: &str) {
    let bytes = include_bytes!("../../../tests/fixtures/hello.pdf").to_vec();
    let doc = Document::from_bytes(bytes).expect("reload hello.pdf for search context");
    let egui_ctx = egui::Context::default();
    let pending_scroll = Cell::new(None::<(PageIndex, PointsRect)>);
    let rotations = HashMap::new();
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
    viewer.open_search();
    viewer.search_mut().update_query(
        SearchQuery {
            term: term.to_owned(),
            case_sensitive: false,
            whole_word: false,
        },
        &ctx,
    );
}

fn make_harness_with_query(term: &str) -> Harness<'static, Viewer> {
    viewkai_engine::init().expect("pdfium init");
    let bytes = include_bytes!("../../../tests/fixtures/hello.pdf").to_vec();
    let mut viewer = Viewer::new();
    viewer.load_bytes(bytes).expect("load hello.pdf");
    query_hello(&mut viewer, term);

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(800.0, 600.0))
        .with_os(egui::os::OperatingSystem::Nix)
        .build_ui_state(|ui, viewer| viewer.show(ui), viewer);
    harness.run_ok();
    harness.run_ok();
    assert!(
        harness
            .state()
            .search_state()
            .is_some_and(|state| state.matches.len() >= 2),
        "query should produce multiple matches"
    );
    harness
}

#[test]
fn next_match_advances_index() {
    let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut harness = make_harness_with_query("l");

    assert_eq!(harness.state().current_match_index(), Some(0));
    assert!(harness.state_mut().next_match().is_some());
    assert_eq!(harness.state().current_match_index(), Some(1));
}

#[test]
fn prev_match_decrements_index() {
    let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut harness = make_harness_with_query("l");

    harness.state_mut().next_match();
    assert_eq!(harness.state().current_match_index(), Some(1));
    assert!(harness.state_mut().prev_match().is_some());
    assert_eq!(harness.state().current_match_index(), Some(0));
}

#[test]
fn next_match_wraps_at_end() {
    let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut harness = make_harness_with_query("l");
    let last = harness
        .state()
        .search_state()
        .expect("search state")
        .matches
        .len()
        - 1;

    for _ in 0..last {
        harness.state_mut().next_match();
    }

    assert_eq!(harness.state().current_match_index(), Some(last));
    assert!(harness.state_mut().next_match().is_some());
    assert_eq!(harness.state().current_match_index(), Some(0));
}

#[test]
fn prev_match_wraps_at_start() {
    let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut harness = make_harness_with_query("l");
    let last = harness
        .state()
        .search_state()
        .expect("search state")
        .matches
        .len()
        - 1;

    assert_eq!(harness.state().current_match_index(), Some(0));
    assert!(harness.state_mut().prev_match().is_some());
    assert_eq!(harness.state().current_match_index(), Some(last));
}
