//! Snapshot test: selection highlight renders correctly.

use egui_kittest::Harness;
use viewkai::Viewer;

#[test]
fn snapshot_selection_highlight_hello() {
    viewkai_engine::init().expect("pdfium init");
    let bytes = include_bytes!("../../../tests/fixtures/hello.pdf").to_vec();
    let mut viewer = Viewer::new();
    viewer.load_bytes(bytes).expect("load hello.pdf");
    viewer.select_all();

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(800.0, 600.0))
        .with_os(egui::os::OperatingSystem::Nix)
        .build_ui_state(|ui, viewer| viewer.show(ui), viewer);

    harness.run_ok();
    harness.run_ok();
    harness.snapshot("selection_highlight_hello");
}
