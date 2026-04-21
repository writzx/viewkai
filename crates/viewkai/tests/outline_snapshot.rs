//! Snapshot test: outline panel renders correctly with bookmarks.

use egui_kittest::Harness;
use viewkai::Viewer;

#[test]
fn snapshot_outline_bookmarks_rendered() {
    viewkai_engine::init().expect("pdfium init");
    let bytes = include_bytes!("../../../tests/fixtures/bookmarks.pdf").to_vec();
    let mut viewer = Viewer::new();
    viewer.load_bytes(bytes).expect("load bookmarks.pdf");
    viewer.outline_mut().set_visible(true);

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(800.0, 600.0))
        .with_os(egui::os::OperatingSystem::Nix)
        .build_ui_state(|ui, viewer| viewer.show(ui), viewer);

    harness.run_ok();
    harness.run_ok();
    harness.snapshot("outline_bookmarks_rendered");
}
