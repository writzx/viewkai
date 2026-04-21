//! Snapshot regression tests for core viewer states.

use egui_kittest::Harness;
use viewkai::{ViewMode, Viewer};
use viewkai::zoom::ZoomState;

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

#[test]
fn snapshot_single_mode_hello() {
    let mut h = make_snapshot_harness_loaded(None);
    h.state_mut().set_view_mode(ViewMode::Single);
    h.run_ok();
    h.run_ok();
    h.snapshot("single_mode_hello");
}

#[test]
fn snapshot_spread_mode_cover_separate() {
    viewkai_engine::init().expect("pdfium init");
    let bytes = include_bytes!("../../../tests/fixtures/500page.pdf").to_vec();
    let mut viewer = Viewer::new();
    viewer.load_bytes(bytes).expect("load 500page.pdf");
    viewer.set_view_mode(ViewMode::Spread {
        cover_separate: true,
    });
    viewer.scroll_to_page(1);

    let mut h = Harness::builder()
        .with_size(egui::Vec2::new(1200.0, 700.0))
        .with_os(egui::os::OperatingSystem::Nix)
        .build_ui_state(|ui, viewer| viewer.show(ui), viewer);
    h.run_ok();
    h.run_ok();
    h.snapshot("spread_mode_cover_separate");
}
