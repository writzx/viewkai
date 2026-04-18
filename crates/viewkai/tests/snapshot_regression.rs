use egui_kittest::Harness;
use viewkai::zoom::ZoomState;
use viewkai::Viewer;

fn make_snapshot_harness_empty() -> Harness<'static, Viewer> {
    let viewer = Viewer::new();
    Harness::builder()
        .with_size(egui::Vec2::new(800.0, 600.0))
        .with_os(egui::os::OperatingSystem::Nix)
        .build_ui_state(|ui, viewer| viewer.show(ui), viewer)
}

fn make_snapshot_harness_loaded(zoom: Option<ZoomState>) -> Harness<'static, Viewer> {
    viewkai_engine::init().expect("pdfium init");
    let bytes = include_bytes!("../../../tests/fixtures/hello.pdf").to_vec();
    let mut viewer = Viewer::new();
    viewer.load_bytes(bytes).expect("load hello.pdf");
    if let Some(z) = zoom {
        viewer.set_zoom(z);
    }
    Harness::builder()
        .with_size(egui::Vec2::new(800.0, 600.0))
        .with_os(egui::os::OperatingSystem::Nix)
        .build_ui_state(|ui, viewer| viewer.show(ui), viewer)
}

#[test]
fn snapshot_empty_state() {
    viewkai_engine::init().expect("pdfium init");
    let mut h = make_snapshot_harness_empty();
    h.run_ok();
    h.snapshot("empty_state");
}

#[test]
fn snapshot_hello_loaded() {
    let mut h = make_snapshot_harness_loaded(None);
    h.run_ok();
    h.run_ok();
    h.snapshot("hello_loaded");
}

#[test]
fn snapshot_error_state() {
    viewkai_engine::init().expect("pdfium init");
    let mut viewer = Viewer::new();
    let _ = viewer.load_bytes(b"not a pdf".to_vec());
    let mut h = Harness::builder()
        .with_size(egui::Vec2::new(800.0, 600.0))
        .with_os(egui::os::OperatingSystem::Nix)
        .build_ui_state(|ui, viewer| viewer.show(ui), viewer);
    h.run_ok();
    h.snapshot("error_state");
}

#[test]
fn snapshot_hello_fitwidth() {
    let mut h = make_snapshot_harness_loaded(Some(ZoomState::FitWidth));
    h.run_ok();
    h.run_ok();
    h.snapshot("hello_fitwidth");
}

#[test]
fn snapshot_hello_custom_2x() {
    let mut h = make_snapshot_harness_loaded(Some(ZoomState::Discrete(2.0)));
    h.run_ok();
    h.run_ok();
    h.snapshot("hello_custom_2x");
}
