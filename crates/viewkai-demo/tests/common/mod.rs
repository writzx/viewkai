use eframe::egui;
use egui_kittest::Harness;
use viewkai_demo::DemoApp;

pub fn demo_harness() -> Harness<'static, DemoApp> {
    viewkai_engine::init().expect("pdfium init");
    Harness::builder()
        .with_size(egui::Vec2::new(1280.0, 900.0))
        .with_os(egui::os::OperatingSystem::Nix)
        .build_eframe(|cc| DemoApp::new(cc))
}

pub fn demo_harness_with_hello() -> Harness<'static, DemoApp> {
    let mut h = demo_harness();
    let bytes = include_bytes!("../../../../tests/fixtures/hello.pdf").to_vec();
    h.state_mut().load_bytes_sync(bytes).expect("load hello");
    h.run_ok();
    h
}

pub fn demo_harness_with_500page() -> Harness<'static, DemoApp> {
    let mut h = demo_harness();
    let bytes = include_bytes!("../../../../tests/fixtures/500page.pdf").to_vec();
    h.state_mut().load_bytes_sync(bytes).expect("load 500page");
    h.run_ok();
    h
}
