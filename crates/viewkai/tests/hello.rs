//! Smoke test for loading and rendering the hello fixture PDF.

use egui_kittest::Harness;
use viewkai::Viewer;

#[test]
fn viewer_loads_hello_pdf() {
    viewkai_engine::init().expect("Failed to initialize PDFium");

    let bytes = include_bytes!("../../../tests/fixtures/hello.pdf").to_vec();
    let mut viewer = Viewer::new();
    viewer.load_bytes(bytes).expect("should open hello.pdf");

    let mut harness = Harness::new_ui_state(
        |ui, viewer| {
            viewer.show(ui);
        },
        viewer,
    );
    harness.run_ok();

    let viewer = harness.state();
    assert_eq!(viewer.page_count(), 1, "hello.pdf should expose one page");

    let size = viewer.page_size_pt(0).expect("page 0 should exist");
    assert!(
        size.x > 0.0,
        "page width should be positive, got {}",
        size.x
    );

    println!("pages.len() == {} ✓", viewer.page_count());
    println!("pages[0].size_pt.x == {} ✓", size.x);
}
