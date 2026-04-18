use viewkai::Viewer;

#[allow(deprecated)]
fn run_headless_frame(viewer: &mut Viewer) {
    let ctx = egui::Context::default();
    let raw_input = egui::RawInput::default();

    let _ = ctx.run(raw_input, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            viewer.show(ui);
        });
    });
}

#[test]
fn viewer_loads_hello_pdf() {
    viewkai_engine::init().expect("Failed to initialize PDFium");

    let bytes = include_bytes!("../../../tests/fixtures/hello.pdf").to_vec();
    let mut viewer = Viewer::new();
    viewer.load_bytes(bytes).expect("should open hello.pdf");

    run_headless_frame(&mut viewer);

    assert_eq!(viewer.page_count(), 1, "hello.pdf should expose one page");

    let size = viewer.page_size_pt(0).expect("page 0 should exist");
    assert!(size.x > 0.0, "page width should be positive, got {}", size.x);

    println!("pages.len() == {} ✓", viewer.page_count());
    println!("pages[0].size_pt.x == {} ✓", size.x);
}
