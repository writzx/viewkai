//! Selection blur integration coverage.

use std::sync::Mutex;

use egui_kittest::Harness;
use viewkai::{PluginContext, PointerEvent, Viewer, ViewerPlugin};
use viewkai_core::{PageIndex, PointsPos, PointsRect};
use viewkai_engine::Document;

static TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn clicking_inside_page_outside_text_clears_selection() {
    let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    viewkai_engine::init().expect("pdfium init");
    let bytes = include_bytes!("../../../tests/fixtures/hello.pdf").to_vec();
    let doc = Document::from_bytes(bytes.clone()).expect("parse hello.pdf for context");
    let mut viewer = Viewer::new();
    viewer.load_bytes(bytes).expect("load hello.pdf");
    viewer.select_all();
    assert!(
        viewer.selection().is_some(),
        "select_all should set selection"
    );

    let egui_ctx = egui::Context::default();
    let pending_scroll = std::cell::Cell::new(None::<(PageIndex, PointsRect)>);
    let mut ctx = PluginContext::new(
        Some(&doc),
        1.0,
        &[],
        &egui_ctx,
        viewer.selection_color(),
        viewer.library_shortcuts_enabled(),
        None,
        &pending_scroll,
    );
    let page = PageIndex(0);
    let page_size = doc.page_size(page).expect("page size");
    let event = PointerEvent {
        pos_in_page_pt: PointsPos {
            x: page_size.width_pt - 5.0,
            y: page_size.height_pt - 5.0,
        },
        inside_page_rect: true,
        primary_down: true,
        modifiers: egui::Modifiers::NONE,
        click_count: 1,
    };

    assert!(
        viewer
            .text_layer_mut()
            .on_pointer_event(page, &event, &mut ctx),
        "miss inside page should be consumed"
    );

    assert!(
        viewer.selection().is_none(),
        "inside-page miss should clear selection"
    );
}

#[test]
fn select_all_then_deselect_works() {
    let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
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
    assert!(
        !harness.state().selected_text().is_empty(),
        "select_all should set selection"
    );

    harness.state_mut().deselect();
    harness.run_ok();
    assert!(
        harness.state().selected_text().is_empty(),
        "deselect should clear selection"
    );
}
