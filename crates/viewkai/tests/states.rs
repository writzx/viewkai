use egui_kittest::{Harness, kittest::Queryable};
use std::sync::{Mutex, OnceLock};
use viewkai::Viewer;
use viewkai::zoom::ZoomState;

fn test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[test]
fn viewer_empty_state_renders_placeholder() {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    viewkai_engine::init().expect("Failed to initialize PDFium");

    let viewer = Viewer::new();
    let mut harness = Harness::new_ui_state(
        |ui, viewer| {
            viewer.show(ui);
        },
        viewer,
    );
    harness.run_ok();

    assert_eq!(harness.state().page_count(), 0);
    let _ = harness.get_by_label("No document loaded");
}

#[test]
fn viewer_error_state_surfaces_and_retries() {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    viewkai_engine::init().expect("Failed to initialize PDFium");

    let mut viewer = Viewer::new();
    let _ = viewer.load_bytes(b"not a pdf".to_vec());
    assert_eq!(viewer.page_count(), 0, "error state has 0 pages");

    let mut harness = Harness::new_ui_state(
        |ui, viewer| {
            viewer.show(ui);
        },
        viewer,
    );
    harness.run_ok();

    harness.get_by_label("Retry").click();
    harness.run_ok();

    assert_eq!(harness.state().page_count(), 0);
    let _ = harness.get_by_label("No document loaded");
}

#[test]
fn viewer_scroll_to_page_advances_layout() {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    viewkai_engine::init().expect("Failed to initialize PDFium");

    let bytes = include_bytes!("../../../tests/fixtures/500page.pdf").to_vec();
    let mut viewer = Viewer::new();
    viewer.load_bytes(bytes).expect("should load 500page.pdf");

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(800.0, 600.0))
        .build_ui_state(
            |ui, viewer| {
                viewer.show(ui);
            },
            viewer,
        );
    harness.run_ok();

    harness.state_mut().scroll_to_page(100);
    harness.run_ok();
    assert_eq!(harness.state().page_count(), 500);

    harness.state_mut().scroll_to_page(499);
    harness.run_ok();
    assert_eq!(harness.state().page_count(), 500);
}

#[test]
fn viewer_zoom_setter_roundtrips() {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    viewkai_engine::init().expect("Failed to initialize PDFium");

    let bytes = include_bytes!("../../../tests/fixtures/hello.pdf").to_vec();
    let mut viewer = Viewer::new();

    viewer.set_zoom(ZoomState::Discrete(2.0));
    assert_eq!(viewer.zoom(), ZoomState::Discrete(2.0));

    viewer.load_bytes(bytes).expect("should load hello.pdf");

    let mut harness = Harness::new_ui_state(
        |ui, viewer| {
            viewer.show(ui);
        },
        viewer,
    );
    harness.run_ok();
    assert_eq!(harness.state().page_count(), 1);

    harness.state_mut().clear();
    harness.run_ok();

    for zoom in [
        ZoomState::FitWidth,
        ZoomState::FitPage,
        ZoomState::Custom(0.5),
        ZoomState::Custom(4.0),
        ZoomState::Discrete(1.0),
    ] {
        harness.state_mut().set_zoom(zoom);
        harness.run_ok();
        assert_eq!(harness.state().zoom(), zoom);
    }
}

#[test]
fn viewer_loaded_state_renders_pages() {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    viewkai_engine::init().expect("Failed to initialize PDFium");

    let bytes = include_bytes!("../../../tests/fixtures/hello.pdf").to_vec();
    let mut viewer = Viewer::new();
    viewer.load_bytes(bytes).expect("should load hello.pdf");

    let mut harness = Harness::new_ui_state(
        |ui, viewer| {
            viewer.show(ui);
        },
        viewer,
    );
    harness.run_ok();

    assert_eq!(harness.state().page_count(), 1);
    let size = harness
        .state()
        .page_size_pt(0)
        .expect("page 0 should exist");
    assert!(size.x > 0.0, "page width > 0");
    assert!(size.y > 0.0, "page height > 0");
}

#[test]
fn viewer_clear_resets_to_empty() {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    viewkai_engine::init().expect("Failed to initialize PDFium");

    let bytes = include_bytes!("../../../tests/fixtures/hello.pdf").to_vec();
    let mut viewer = Viewer::new();
    viewer.load_bytes(bytes).expect("should load hello.pdf");

    let mut harness = Harness::new_ui_state(
        |ui, viewer| {
            viewer.show(ui);
        },
        viewer,
    );
    harness.run_ok();

    assert_eq!(harness.state().page_count(), 1);

    harness.state_mut().clear();
    harness.run_ok();

    assert_eq!(harness.state().page_count(), 0);
}
