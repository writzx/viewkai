//! Multipage search regression coverage.

use std::sync::Mutex;

use egui_kittest::Harness;
use viewkai::Viewer;

static TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn search_on_multipage_pdf_does_not_panic() {
    let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    viewkai_engine::init().expect("pdfium init");
    let bytes = include_bytes!("../../../tests/fixtures/500page.pdf").to_vec();
    let mut viewer = Viewer::new();
    viewer.load_bytes(bytes).expect("load 500page.pdf");
    viewer.open_search();
    viewer.scroll_to_page(2);

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(800.0, 600.0))
        .with_os(egui::os::OperatingSystem::Nix)
        .build_ui_state(|ui, viewer| viewer.show(ui), viewer);

    harness.run_ok();
    harness.run_ok();

    assert_eq!(
        harness
            .state()
            .search_state()
            .map_or(0, |state| state.matches.len()),
        0
    );
}
