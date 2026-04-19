//! Acceptance test covering `TextureCache` budget behavior during scrolling.
// justify: test output keeps explicit formatting and bounded byte-to-MB casts.
#![allow(clippy::cast_precision_loss, clippy::uninlined_format_args)]

use egui::Vec2;
use egui_kittest::Harness;
use viewkai::Viewer;

#[test]
fn memory_budget_acceptance() {
    viewkai_engine::init().expect("Failed to initialize PDFium");

    let bytes = include_bytes!("../../../tests/fixtures/500page.pdf").to_vec();
    let mut viewer = Viewer::new();
    viewer.load_bytes(bytes).expect("should open 500page.pdf");

    let page_count = viewer.page_count();
    assert_eq!(page_count, 500, "500page.pdf should have 500 pages");

    let budget = 256 * 1024 * 1024;
    let mut peak_bytes = 0usize;

    let mut harness = Harness::builder()
        .with_size(Vec2::new(1280.0, 900.0))
        .build_ui_state(
            |ui, viewer| {
                viewer.show(ui);
            },
            viewer,
        );

    for page_idx in 0..page_count {
        harness.state_mut().scroll_to_page(page_idx);
        harness.run_steps(2);

        let current = harness.state().cache_bytes();
        peak_bytes = peak_bytes.max(current);

        assert!(
            current <= budget,
            "cache bytes {} exceeded budget {} on page {}",
            current,
            budget,
            page_idx
        );
    }

    println!(
        "Peak cache bytes: {} ({:.1} MB)",
        peak_bytes,
        peak_bytes as f64 / 1024.0 / 1024.0
    );
    println!(
        "Budget: {} ({:.1} MB)",
        budget,
        budget as f64 / 1024.0 / 1024.0
    );
    println!("memory_budget_acceptance: PASS");
}
