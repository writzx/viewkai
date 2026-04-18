use egui::{Pos2, RawInput, Rect, Vec2};
use viewkai::Viewer;

#[allow(deprecated)]
fn run_headless_frame(viewer: &mut Viewer) {
    let ctx = egui::Context::default();
    let raw_input = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 900.0))),
        ..Default::default()
    };

    let _ = ctx.run(raw_input, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            viewer.show(ui);
        });
    });
}

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

    for page_idx in 0..page_count {
        viewer.scroll_to_page(page_idx);
        run_headless_frame(&mut viewer);
        run_headless_frame(&mut viewer);

        let current = viewer.cache_bytes();
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
